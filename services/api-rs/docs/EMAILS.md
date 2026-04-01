# Email System Documentation

## Overview

The Monobase Rust API implements a multi-provider email system that handles both authentication emails and notification-driven communications. The provider is selected at startup from configuration and the same `EmailService` is used throughout the application lifetime.

## Architecture

### Core Components

- **EmailService** (`src/service/email.rs`): Single service supporting SMTP, Postmark, and OneSignal providers
- **NotificationService** (`src/service/notifs.rs`): Handles push notifications with email delegated to `EmailService`
- **Email Providers**: SMTP via `lettre`, Postmark and OneSignal via `reqwest`

### Email Delivery Paths

1. **Direct EmailService calls**: Authentication and transactional emails sent by calling `EmailService::send` directly from handlers
2. **Notification-triggered emails**: Healthcare notifications with email channel delegate to `EmailService`

## EmailService

Location: `src/service/email.rs`

```rust
pub struct EmailService {
    provider: String,
    from_name: String,
    from_email: String,
    smtp_transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    postmark_api_key: Option<String>,
    onesignal_app_id: Option<String>,
    onesignal_api_key: Option<String>,
    http: reqwest::Client,
}
```

The SMTP transport is built eagerly at startup (when `EMAIL_PROVIDER=smtp`) so connection errors surface immediately rather than at first send.

### Send Interface

```rust
pub async fn send(
    &self,
    to_email: &str,
    to_name: Option<&str>,
    subject: &str,
    html_body: &str,
    text_body: Option<&str>,
) -> Result<String, String>   // returns provider message ID on success
```

When `text_body` is `Some`, the message is sent as `multipart/alternative` with both HTML and plain-text parts. When `None`, a single HTML part is used.

## Email Providers

### SMTP Provider (via `lettre`)

- **Crate**: `lettre 0.11` with `tokio1-native-tls` feature
- **Use Case**: Development (Mailpit / MailHog) and simple deployments
- **Configuration**: `EMAIL_PROVIDER=smtp`

STARTTLS is used when `SMTP_SECURE=true` (port 587); plain relay (no encryption) is used otherwise — handy for local Mailpit/MailHog servers on port 1025.

```rust
// Build SMTP transport from config (called at startup)
fn build_smtp_transport(config: &Config) -> AsyncSmtpTransport<Tokio1Executor> {
    // STARTTLS path (smtp_secure = true)
    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
        .port(port)
        .tls(Tls::Required(tls_params))
        .authentication(vec![Mechanism::Plain, Mechanism::Login])
        .credentials(Credentials::new(user, pass))
        .build()

    // Plain relay path (smtp_secure = false, for Mailpit/MailHog)
    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
        .port(port)
        .tls(Tls::None)
        .build()
}
```

### Postmark Provider (via `reqwest`)

- **Crate**: `reqwest 0.12` (shared HTTP client)
- **Use Case**: Production — advanced analytics, bounce handling
- **Configuration**: `EMAIL_PROVIDER=postmark`

```rust
// POST https://api.postmarkapp.com/email
// Header: X-Postmark-Server-Token: {POSTMARK_API_KEY}
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PostmarkPayload<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    #[serde(rename = "HtmlBody")]
    html_body: &'a str,
    #[serde(rename = "TextBody", skip_serializing_if = "Option::is_none")]
    text_body: Option<&'a str>,
    message_stream: &'a str,   // "outbound"
}
```

### OneSignal Provider (via `reqwest`)

- **Crate**: `reqwest 0.12` (shared HTTP client)
- **Use Case**: Unified multi-channel messaging (email + push + SMS from one platform)
- **Configuration**: `EMAIL_PROVIDER=onesignal`

```rust
// POST https://onesignal.com/api/v1/notifications
// Header: Authorization: Basic {ONESIGNAL_API_KEY}
#[derive(Serialize)]
struct OsPayload<'a> {
    app_id: &'a str,
    email_subject: &'a str,
    email_body: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    email_preheader: Option<&'a str>,
    include_email_tokens: Vec<OsEmailAddress<'a>>,
}
```

## Authentication Email Templates

The Rust service sends emails for these authentication events by calling `EmailService::send` directly from auth handlers:

### Email Verification (`auth.email-verify`)

**Trigger**: User registration or email change

**Variables**:
```
userName, userEmail, verificationUrl, verificationToken
```

### Password Reset (`auth.password-reset`)

**Trigger**: User requests password reset

**Variables**:
```
userName, userEmail, resetUrl, resetToken
```

### Two-Factor Authentication (`auth.2fa`)

**Trigger**: 2FA verification required

**Variables**:
```
userEmail, otpCode, otpType
```

## Notification-Triggered Emails

Healthcare notifications that include email as a delivery channel are delivered via the `NotificationService::deliver` method, which delegates email to `EmailService`:

```rust
pub async fn deliver(
    &self,
    channel: &str,
    recipient_id: &str,
    title: &str,
    message: &str,
    data: Option<serde_json::Value>,
) -> Result<(), String> {
    match channel {
        "push"   => { self.send_push(recipient_id, title, message, data).await?; }
        "email"  => { /* delegate to EmailService */ }
        "in-app" => { /* stored in DB, delivered via WebSocket */ }
        _        => return Err(format!("Unknown channel: {}", channel)),
    }
    Ok(())
}
```

### Email-Enabled Notification Types

#### High Priority (in-app, email, SMS)
- `appointment_cancelled_by_other` — Appointment cancelled by other party
- `appointment_rejected` — Appointment rejected by provider
- `appointment_auto_rejected` — Auto-rejected due to timeout
- `payment_failed` — Payment processing failed
- `charge_failed` — Payment charge failed

#### Normal Priority (in-app, email)
- `payment_authorized` — Payment authorized and held
- `payment_captured` — Payment successfully processed
- `payment_received` — Provider payment notification
- `appointment_expired` — Provider missed confirmation deadline

## Service Configuration

### EmailService Construction

```rust
impl EmailService {
    pub fn new(config: &Config) -> Self {
        let smtp_transport = if config.email_provider == "smtp" {
            Some(build_smtp_transport(config))
        } else {
            None
        };

        Self {
            provider: config.email_provider.clone(),
            from_name: config.email_from_name.clone(),
            from_email: config.email_from_email.clone(),
            smtp_transport,
            postmark_api_key: config.postmark_api_key.clone(),
            onesignal_app_id: config.onesignal_app_id.clone(),
            onesignal_api_key: config.onesignal_api_key.clone(),
            http: reqwest::Client::new(),
        }
    }
}
```

### Environment Variables

```bash
# Provider selection
EMAIL_PROVIDER=smtp           # smtp | postmark | onesignal

# Sender identity
EMAIL_FROM_NAME=Monobase
EMAIL_FROM_EMAIL=noreply@monobase.com

# SMTP (when EMAIL_PROVIDER=smtp)
SMTP_HOST=127.0.0.1
SMTP_PORT=1025
SMTP_SECURE=false
SMTP_USER=
SMTP_PASS=

# Postmark (when EMAIL_PROVIDER=postmark)
POSTMARK_API_KEY=your-server-token

# OneSignal (when EMAIL_PROVIDER=onesignal)
ONESIGNAL_APP_ID=your-app-id
ONESIGNAL_API_KEY=your-rest-api-key
```

## Local Development with Mailpit

**Mailpit** is the recommended email capture tool for local development. It runs an SMTP server on port 1025 and provides a web UI on port 8025.

### Setup

Mailpit is included in the development Docker Compose dependencies:

```bash
# Start all development dependencies including Mailpit
docker compose up -d mailpit

# Mailpit is now running:
# - SMTP server: localhost:1025
# - Web UI: http://localhost:8025
```

### Default Configuration

The API defaults to Mailpit-compatible settings (no variables needed for local dev):

```bash
EMAIL_PROVIDER=smtp
SMTP_HOST=127.0.0.1
SMTP_PORT=1025
SMTP_SECURE=false
SMTP_USER=
SMTP_PASS=
```

### Running the API Against Mailpit

```bash
# Run the API (reads .env via dotenvy)
cargo run

# Trigger emails: register a user, reset password, etc.
# View captured emails at http://localhost:8025
```

### Testing with Mailpit API

Mailpit provides an HTTP API for automated testing:

```bash
# Fetch all captured emails
curl http://localhost:8025/api/v1/messages

# Search by subject
curl "http://localhost:8025/api/v1/messages?query=Verify+your+email"
```

## Monitoring and Logging

All email sends are logged via `tracing` with provider, recipient, subject, and message ID:

```
INFO Email sent via SMTP    provider=smtp    to=user@example.com subject="Verify your email" message_id=...
INFO Email sent via Postmark provider=postmark to=user@example.com subject="..." message_id=pm-...
INFO Email sent via OneSignal provider=onesignal to=user@example.com subject="..." notification_id=os-...
```

Enable debug logging to see full request/response details:

```bash
RUST_LOG=debug cargo run
```

## Security Considerations

### Email Security

- **TLS**: STARTTLS enforced when `SMTP_SECURE=true`; use `wss://` in production
- **Credentials**: API keys stored only in environment variables / secrets manager
- **Audit logging**: All email events logged via `tracing` with structured fields

### Healthcare Compliance

- **HIPAA**: No PHI in email bodies — use identifiers and generic language
- **Consent management**: Respect user email preferences before sending
- **Encryption**: TLS for all email communication in production

## Troubleshooting

### Common Issues

1. **SMTP connection refused**
   - Verify Mailpit or relay server is running
   - Check `SMTP_HOST` and `SMTP_PORT` values

2. **Postmark 401 / 422 errors**
   - Verify `POSTMARK_API_KEY` is a server token (not account token)
   - Check the `message_stream` value matches your Postmark stream

3. **OneSignal 400 errors**
   - Verify `ONESIGNAL_APP_ID` and `ONESIGNAL_API_KEY` are set
   - Confirm the app has email capability enabled in OneSignal dashboard

### Email Queue Monitoring (Database)

```sql
-- Check email queue depth (if using job-based processing)
SELECT status, COUNT(*) FROM email_queue GROUP BY status;

-- Review failed sends
SELECT * FROM email_queue WHERE status = 'failed' ORDER BY last_attempt_at DESC LIMIT 10;
```

## Future Enhancements

1. **Template System**: Database-backed Handlebars/Tera templates
2. **Email Queue**: pg-boss job integration for async delivery and retries
3. **Advanced Analytics**: Open rate tracking, bounce handling
4. **Multi-language Support**: Internationalization for global deployment
