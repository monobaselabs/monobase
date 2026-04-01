use clap::Parser;

/// Monobase API server configuration.
/// All values can be set via environment variables or CLI args.
#[derive(Parser, Debug, Clone)]
#[command(name = "monobase-api", about = "Monobase Application Platform API")]
pub struct Config {
    // -- Server --
    #[arg(long, env = "SERVER_HOST", default_value = "0.0.0.0")]
    pub server_host: String,

    #[arg(long, env = "SERVER_PORT", default_value_t = 7213)]
    pub server_port: u16,

    #[arg(long, env = "SERVER_PUBLIC_URL")]
    pub server_public_url: Option<String>,

    // -- Database --
    #[arg(long, env = "DATABASE_URL", default_value = "postgres://postgres:password@localhost:5432/monobase")]
    pub database_url: String,

    #[arg(long, env = "DB_POOL_MIN", default_value_t = 2)]
    pub db_pool_min: u32,

    #[arg(long, env = "DB_POOL_MAX", default_value_t = 20)]
    pub db_pool_max: u32,

    #[arg(long, env = "DB_SSL", default_value_t = false)]
    pub db_ssl: bool,

    #[arg(long, env = "DB_LOGGING", default_value_t = false)]
    pub db_logging: bool,

    // -- CORS --
    #[arg(long, env = "CORS_ORIGINS", default_value = "*", value_delimiter = ',')]
    pub cors_origins: Vec<String>,

    #[arg(long, env = "CORS_CREDENTIALS", default_value_t = true)]
    pub cors_credentials: bool,

    // -- Logging --
    #[arg(long, env = "LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    #[arg(long, env = "LOG_PRETTY", default_value_t = true)]
    pub log_pretty: bool,

    // -- Auth --
    #[arg(long, env = "AUTH_SECRET", default_value = "change-me-in-production")]
    pub auth_secret: String,

    #[arg(long, env = "AUTH_SESSION_EXPIRES_IN", default_value_t = 604800)]
    pub auth_session_expires_secs: u64,

    #[arg(long, env = "AUTH_ADMIN_EMAILS", default_value = "", value_delimiter = ',')]
    pub auth_admin_emails: Vec<String>,

    // -- Storage --
    #[arg(long, env = "STORAGE_PROVIDER", default_value = "minio")]
    pub storage_provider: String,

    #[arg(long, env = "STORAGE_ENDPOINT", default_value = "http://localhost:9000")]
    pub storage_endpoint: String,

    #[arg(long, env = "STORAGE_PUBLIC_ENDPOINT", default_value = "http://localhost:9000")]
    pub storage_public_endpoint: String,

    #[arg(long, env = "STORAGE_BUCKET", default_value = "monobase-files")]
    pub storage_bucket: String,

    #[arg(long, env = "STORAGE_REGION", default_value = "us-east-1")]
    pub storage_region: String,

    #[arg(long, env = "STORAGE_ACCESS_KEY_ID", default_value = "minioadmin")]
    pub storage_access_key_id: String,

    #[arg(long, env = "STORAGE_SECRET_ACCESS_KEY", default_value = "minioadmin")]
    pub storage_secret_access_key: String,

    #[arg(long, env = "STORAGE_UPLOAD_URL_EXPIRY", default_value_t = 300)]
    pub storage_upload_url_expiry: u64,

    #[arg(long, env = "STORAGE_DOWNLOAD_URL_EXPIRY", default_value_t = 900)]
    pub storage_download_url_expiry: u64,

    // -- Email --
    #[arg(long, env = "EMAIL_PROVIDER", default_value = "smtp")]
    pub email_provider: String,

    #[arg(long, env = "EMAIL_FROM_NAME", default_value = "Monobase")]
    pub email_from_name: String,

    #[arg(long, env = "EMAIL_FROM_EMAIL", default_value = "noreply@monobase.com")]
    pub email_from_email: String,

    #[arg(long, env = "SMTP_HOST", default_value = "127.0.0.1")]
    pub smtp_host: String,

    #[arg(long, env = "SMTP_PORT", default_value_t = 1025)]
    pub smtp_port: u16,

    #[arg(long, env = "SMTP_SECURE", default_value_t = false)]
    pub smtp_secure: bool,

    #[arg(long, env = "SMTP_USER", default_value = "")]
    pub smtp_user: String,

    #[arg(long, env = "SMTP_PASS", default_value = "")]
    pub smtp_pass: String,

    #[arg(long, env = "POSTMARK_API_KEY")]
    pub postmark_api_key: Option<String>,

    // -- Notifications --
    #[arg(long, env = "ONESIGNAL_APP_ID")]
    pub onesignal_app_id: Option<String>,

    #[arg(long, env = "ONESIGNAL_API_KEY")]
    pub onesignal_api_key: Option<String>,

    // -- Billing --
    #[arg(long, env = "STRIPE_SECRET_KEY")]
    pub stripe_secret_key: Option<String>,

    #[arg(long, env = "STRIPE_WEBHOOK_SECRET")]
    pub stripe_webhook_secret: Option<String>,

    #[arg(long, env = "STRIPE_URL")]
    pub stripe_url: Option<String>,

    // -- WebRTC --
    #[arg(long, env = "WEBRTC_ICE_SERVERS", default_value = "stun:stun.l.google.com:19302,stun:stun1.l.google.com:19302", value_delimiter = ',')]
    pub webrtc_ice_servers: Vec<String>,
}

impl Config {
    /// Derive the public base URL for auth callbacks, etc.
    pub fn public_url(&self) -> String {
        self.server_public_url
            .clone()
            .unwrap_or_else(|| format!("http://{}:{}", self.server_host, self.server_port))
    }
}
