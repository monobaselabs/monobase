/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub auth_secret: String,
}

impl Config {
    pub fn new(port: u16, database_url: Option<String>) -> Self {
        Self {
            port,
            database_url: database_url.unwrap_or_else(|| {
                std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgresql://postgres:postgres@127.0.0.1:5432/postgres".to_string())
            }),
            auth_secret: std::env::var("AUTH_SECRET")
                .unwrap_or_else(|_| "dev-secret-key".to_string()),
        }
    }
}
