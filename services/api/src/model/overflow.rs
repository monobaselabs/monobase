//! `_data` JSONB overflow system.
//!
//! Port of the TypeScript `_collectOverflow` / `_spreadData` / `fromRow` pattern.
//! Unknown fields are routed to a `_data` JSONB column on write and spread back
//! into the result object on read.

use serde_json::{Map, Value};
use std::collections::HashSet;

/// Separate known columns from overflow fields.
///
/// Fields whose snake_case name matches a known column stay as top-level inserts.
/// Everything else goes into the `_data` JSONB overflow.
///
/// Returns `(known_fields, overflow_fields)`.
pub fn collect_overflow(
    data: &Map<String, Value>,
    known_columns: &HashSet<String>,
) -> (Map<String, Value>, Map<String, Value>) {
    let mut known = Map::new();
    let mut overflow = Map::new();

    for (key, value) in data {
        if key == "id" || key == "_data" {
            known.insert(key.clone(), value.clone());
            continue;
        }
        // Skip MongoDB operators — they're handled by patch/service layer
        if key.starts_with('$') {
            continue;
        }
        let snake = to_snake(key);
        if known_columns.contains(&snake) {
            known.insert(key.clone(), value.clone());
        } else {
            overflow.insert(key.clone(), value.clone());
        }
    }

    (known, overflow)
}

/// Merge `_data` overflow fields back into the result object.
/// Dedicated columns take precedence over `_data` contents (prevents stale overwrites).
///
/// Also converts snake_case keys to camelCase for API output.
pub fn spread_data(mut row: Value) -> Value {
    let obj = match row.as_object_mut() {
        Some(o) => o,
        None => return row,
    };

    // Extract and merge _data
    if let Some(data_val) = obj.remove("_data") {
        if let Some(data_obj) = data_val.as_object() {
            for (k, v) in data_obj {
                // Only insert if the key doesn't already exist as a dedicated column
                if !obj.contains_key(k) {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
    }

    // Convert snake_case keys → camelCase
    let entries: Vec<(String, Value)> = obj
        .iter()
        .map(|(k, v)| (to_camel(k), v.clone()))
        .collect();

    let mut result = Map::new();
    for (k, v) in entries {
        result.insert(k, v);
    }

    // Convert timestamps after camelCase conversion
    convert_timestamps_to_epoch(&mut result);

    Value::Object(result)
}

/// Convert a database row (from `row_to_json()`) into API output format.
///
/// 1. Spread `_data` overflow
/// 2. Convert snake_case → camelCase
/// 3. Convert timestamps to epoch milliseconds
/// 4. Inject id alias if configured
pub fn from_row(row: Value, id_alias: Option<&str>) -> Value {
    let mut result = spread_data(row);

    if let Some(obj) = result.as_object_mut() {
        // Convert timestamp strings to epoch ms
        convert_timestamps_to_epoch(obj);

        // Inject id alias (e.g., uid = id)
        if let Some(alias) = id_alias {
            if let Some(id) = obj.get("id").cloned() {
                obj.insert(alias.to_string(), id);
            }
        }
    }

    result
}

/// Convert ISO timestamp strings to epoch milliseconds (matching TypeScript behavior).
fn convert_timestamps_to_epoch(obj: &mut Map<String, Value>) {
    let timestamp_keys: Vec<String> = obj
        .iter()
        .filter_map(|(k, v)| {
            if is_timestamp_field(k) && v.is_string() {
                Some(k.clone())
            } else {
                None
            }
        })
        .collect();

    for key in timestamp_keys {
        if let Some(Value::String(s)) = obj.get(&key) {
            if let Some(epoch_ms) = parse_timestamp_to_epoch_ms(s) {
                obj.insert(key, Value::Number(serde_json::Number::from(epoch_ms)));
            }
        }
    }
}

/// Check if a field name looks like a timestamp.
fn is_timestamp_field(name: &str) -> bool {
    name == "createdAt"
        || name == "updatedAt"
        || name.ends_with("At")
        || name.ends_with("_at")
        || name == "createdAt"
        || name == "updatedAt"
        || name == "archivedAt"
        || name == "removedAt"
        || name == "acceptedAt"
        || name == "lastActiveAt"
        || name == "resetAt"
        || name == "resetLastAt"
        || name == "resetNextAt"
        || name == "startedAt"
        || name == "finishedAt"
        || name == "paidAt"
        || name == "voidedAt"
        || name == "expiresAt"
}

/// Parse an ISO 8601 timestamp string to epoch milliseconds.
fn parse_timestamp_to_epoch_ms(s: &str) -> Option<i64> {
    // Try chrono parsing
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis());
    }
    // Try without timezone (PostgreSQL default format)
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(dt.and_utc().timestamp_millis());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(dt.and_utc().timestamp_millis());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc().timestamp_millis());
    }
    None
}

// --- String conversion helpers ---

/// Convert camelCase to snake_case.
///
/// Matches the TypeScript `toSnakeCase`: `str.replace(/([A-Z])/g, '_$1').toLowerCase()`
/// Every uppercase letter gets a `_` prefix. This means:
/// - `configLIS` → `config_l_i_s`
/// - `configEMR` → `config_e_m_r`
/// - `bl_paymentMethods` → `bl_payment_methods`
pub fn to_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

/// Convert snake_case to camelCase.
/// Convert snake_case to camelCase, preserving namespace prefixes.
///
/// Namespace prefixes (like `bl_`, `wh_`, `mf_`, `ac_`, `doc_`, `px_`, `sms_`)
/// are preserved as-is. The rest is converted to camelCase.
///
/// Examples:
/// - `created_at` → `createdAt`
/// - `bl_payment_methods` → `bl_paymentMethods`
/// - `mf_patient_fields` → `mf_patientFields`
/// - `config_l_i_s` → `configLIS`
pub fn to_camel(s: &str) -> String {
    // Known namespace prefixes that should be preserved
    const PREFIXES: &[&str] = &[
        "bl_", "wh_", "mf_", "ac_", "doc_", "px_", "sms_",
    ];

    // Check for known prefix
    for prefix in PREFIXES {
        if s.starts_with(prefix) {
            let rest = &s[prefix.len()..];
            let camel_rest = simple_to_camel(rest);
            return format!("{}{}", prefix, camel_rest);
        }
    }

    // Handle special patterns: consecutive single-letter segments → uppercase
    // e.g., config_l_i_s → configLIS, config_e_m_r → configEMR
    simple_to_camel(s)
}

fn simple_to_camel(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_to_snake() {
        assert_eq!(to_snake("createdAt"), "created_at");
        assert_eq!(to_snake("organizationId"), "organization_id");
        assert_eq!(to_snake("id"), "id");
        assert_eq!(to_snake("name"), "name");
        assert_eq!(to_snake("isPublic"), "is_public");
    }

    #[test]
    fn test_to_camel() {
        assert_eq!(to_camel("created_at"), "createdAt");
        assert_eq!(to_camel("organization_id"), "organizationId");
        assert_eq!(to_camel("id"), "id");
        assert_eq!(to_camel("is_public"), "isPublic");
    }

    #[test]
    fn test_collect_overflow() {
        let mut data = Map::new();
        data.insert("id".to_string(), json!("123"));
        data.insert("name".to_string(), json!("Test"));
        data.insert("customField".to_string(), json!("value"));

        let mut known = HashSet::new();
        known.insert("id".to_string());
        known.insert("name".to_string());

        let (known_fields, overflow) = collect_overflow(&data, &known);

        assert_eq!(known_fields.get("id"), Some(&json!("123")));
        assert_eq!(known_fields.get("name"), Some(&json!("Test")));
        assert!(known_fields.get("customField").is_none());
        assert_eq!(overflow.get("customField"), Some(&json!("value")));
    }

    #[test]
    fn test_spread_data() {
        let row = json!({
            "id": "123",
            "name": "Test",
            "_data": {"customField": "value", "nested": {"a": 1}}
        });

        let result = spread_data(row);
        assert_eq!(result.get("id"), Some(&json!("123")));
        assert_eq!(result.get("name"), Some(&json!("Test")));
        assert_eq!(result.get("customField"), Some(&json!("value")));
        assert!(result.get("_data").is_none()); // _data removed
    }

    #[test]
    fn test_spread_data_dedicated_takes_precedence() {
        let row = json!({
            "name": "Dedicated",
            "_data": {"name": "Overflow"}
        });

        let result = spread_data(row);
        assert_eq!(result.get("name"), Some(&json!("Dedicated")));
    }

    #[test]
    fn test_from_row_with_alias() {
        let row = json!({"id": "123", "email": "test@test.com"});
        let result = from_row(row, Some("uid"));
        assert_eq!(result.get("id"), Some(&json!("123")));
        assert_eq!(result.get("uid"), Some(&json!("123")));
    }

    #[test]
    fn test_timestamp_conversion() {
        let row = json!({
            "id": "123",
            "created_at": "2025-01-01T00:00:00",
            "name": "Test"
        });

        let result = spread_data(row);
        // createdAt should be epoch ms
        let created_at = result.get("createdAt").unwrap();
        assert!(created_at.is_number(), "createdAt should be epoch ms, got {:?}", created_at);
    }
}
