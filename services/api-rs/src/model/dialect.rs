/// Supported database dialects.
/// Encapsulates all PostgreSQL vs SQLite SQL differences so the model layer
/// never emits dialect-specific SQL directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Postgres,
    Sqlite,
}

impl Dialect {
    /// Detect dialect from a database URL.
    pub fn from_url(url: &str) -> Self {
        if url.starts_with("sqlite") {
            Dialect::Sqlite
        } else {
            Dialect::Postgres
        }
    }

    /// SQL expression for "current timestamp".
    pub fn now_expr(&self) -> &str {
        match self {
            Dialect::Postgres => "NOW()",
            Dialect::Sqlite => "datetime('now')",
        }
    }

    /// Cast suffix for inserting JSON data.
    /// PostgreSQL needs `::jsonb`; SQLite stores JSON as TEXT.
    pub fn json_cast(&self) -> &str {
        match self {
            Dialect::Postgres => "::jsonb",
            Dialect::Sqlite => "",
        }
    }

    /// Regex match operator.
    pub fn regex_op(&self, case_insensitive: bool) -> &str {
        match self {
            Dialect::Postgres => {
                if case_insensitive {
                    "~*"
                } else {
                    "~"
                }
            }
            Dialect::Sqlite => "LIKE",
        }
    }

    /// Format a regex pattern for the dialect.
    pub fn regex_pattern(&self, pattern: &str) -> String {
        match self {
            Dialect::Postgres => pattern.to_string(),
            Dialect::Sqlite => format!("%{}%", pattern),
        }
    }

    /// JSONB "contains" operator for array-of-objects columns.
    ///
    /// PostgreSQL: `column @> '[{"key":"value"}]'::jsonb`
    /// SQLite: `EXISTS (SELECT 1 FROM json_each(column) WHERE json_extract(value, '$.key') = value)`
    pub fn json_array_contains(&self, column: &str, key: &str, value: &str) -> String {
        match self {
            Dialect::Postgres => {
                format!(
                    "\"{}\" @> '[{{\"{}\": {}}}]'::jsonb",
                    column, key, value
                )
            }
            Dialect::Sqlite => {
                format!(
                    "EXISTS (SELECT 1 FROM json_each(\"{}\") WHERE json_extract(value, '$.{}') = {})",
                    column, key, value
                )
            }
        }
    }

    /// JSONB overlap operator for scalar-array columns.
    ///
    /// PostgreSQL: `column ?| array['a','b']::text[]`
    /// SQLite: `EXISTS (SELECT 1 FROM json_each(column) WHERE value IN ('a','b'))`
    pub fn json_scalar_overlap(&self, column: &str, values: &[String]) -> String {
        let quoted: Vec<String> = values
            .iter()
            .map(|v| format!("'{}'", v.replace('\'', "''")))
            .collect();
        match self {
            Dialect::Postgres => {
                format!("\"{}\" ?| array[{}]::text[]", column, quoted.join(", "))
            }
            Dialect::Sqlite => {
                format!(
                    "EXISTS (SELECT 1 FROM json_each(\"{}\") WHERE value IN ({}))",
                    column,
                    quoted.join(", ")
                )
            }
        }
    }

    /// Extract a text value from a JSONB column at a given path.
    ///
    /// PostgreSQL: `column->>'key'`
    /// SQLite: `json_extract(column, '$.key')`
    pub fn json_extract_text(&self, column: &str, path: &[&str]) -> String {
        match self {
            Dialect::Postgres => {
                if path.len() == 1 {
                    format!("\"{}\"->>'{}' ", column, path[0])
                } else {
                    let mut expr = format!("\"{}\"", column);
                    for (i, key) in path.iter().enumerate() {
                        if i == path.len() - 1 {
                            expr.push_str(&format!("->>'{}' ", key));
                        } else {
                            expr.push_str(&format!("->'{}' ", key));
                        }
                    }
                    expr
                }
            }
            Dialect::Sqlite => {
                let json_path = format!("$.{}", path.join("."));
                format!("json_extract(\"{}\", '{}')", column, json_path)
            }
        }
    }

    /// Whether RETURNING clause is supported.
    pub fn supports_returning(&self) -> bool {
        true // Both PostgreSQL and SQLite 3.35+ support RETURNING
    }

    /// The SeaORM `DatabaseBackend` enum value.
    pub fn sea_orm_backend(&self) -> sea_orm::DatabaseBackend {
        match self {
            Dialect::Postgres => sea_orm::DatabaseBackend::Postgres,
            Dialect::Sqlite => sea_orm::DatabaseBackend::Sqlite,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialect_from_url() {
        assert_eq!(
            Dialect::from_url("postgresql://localhost/db"),
            Dialect::Postgres
        );
        assert_eq!(
            Dialect::from_url("postgres://localhost/db"),
            Dialect::Postgres
        );
        assert_eq!(Dialect::from_url("sqlite:./test.db"), Dialect::Sqlite);
        assert_eq!(Dialect::from_url("sqlite::memory:"), Dialect::Sqlite);
    }

    #[test]
    fn test_now_expr() {
        assert_eq!(Dialect::Postgres.now_expr(), "NOW()");
        assert_eq!(Dialect::Sqlite.now_expr(), "datetime('now')");
    }

    #[test]
    fn test_json_cast() {
        assert_eq!(Dialect::Postgres.json_cast(), "::jsonb");
        assert_eq!(Dialect::Sqlite.json_cast(), "");
    }

    #[test]
    fn test_json_extract() {
        assert_eq!(
            Dialect::Postgres.json_extract_text("_data", &["email"]),
            "\"_data\"->>'email' "
        );
        assert_eq!(
            Dialect::Sqlite.json_extract_text("_data", &["email"]),
            "json_extract(\"_data\", '$.email')"
        );
    }
}
