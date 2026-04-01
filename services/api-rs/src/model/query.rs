/// Query parameter extraction for list endpoints.
///
/// Supports: offset, limit, sort, search (q)

/// Parsed pagination + filter parameters from query string.
#[derive(Debug, Clone)]
pub struct ListParams {
    pub offset: u64,
    pub limit: u64,
    pub sort_field: Option<String>,
    pub sort_direction: SortDirection,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl Default for ListParams {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 20,
            sort_field: None,
            sort_direction: SortDirection::Desc,
            search: None,
        }
    }
}

impl ListParams {
    /// Parse from a serde_json::Value (query params).
    pub fn from_query(query: &serde_json::Value) -> Self {
        let mut params = Self::default();

        if let Some(offset) = query.get("offset").and_then(|v| v.as_u64()) {
            params.offset = offset;
        }
        if let Some(limit) = query.get("limit").and_then(|v| v.as_u64()) {
            params.limit = limit.min(100); // cap at 100
        }
        if let Some(sort) = query.get("sort").and_then(|v| v.as_str()) {
            if let Some((field, dir)) = sort.split_once(':') {
                params.sort_field = Some(field.to_string());
                params.sort_direction = if dir.eq_ignore_ascii_case("asc") {
                    SortDirection::Asc
                } else {
                    SortDirection::Desc
                };
            } else {
                params.sort_field = Some(sort.to_string());
            }
        }
        if let Some(q) = query.get("q").and_then(|v| v.as_str()) {
            if !q.is_empty() {
                params.search = Some(q.to_string());
            }
        }

        params
    }

    /// Generate SQL ORDER BY clause.
    pub fn order_by_sql(&self) -> String {
        let field = self.sort_field.as_deref().unwrap_or("created_at");
        let dir = match self.sort_direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };
        format!("ORDER BY \"{}\" {}", field, dir)
    }

    /// Generate SQL LIMIT + OFFSET clause.
    pub fn limit_offset_sql(&self) -> String {
        format!("LIMIT {} OFFSET {}", self.limit, self.offset)
    }
}
