//! Email sending service.
//! Supports SMTP (via lettre), Postmark, and OneSignal providers.

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{MultiPart, SinglePart, header::ContentType},
    transport::smtp::{
        authentication::{Credentials, Mechanism},
        client::{Tls, TlsParameters},
    },
};
use serde::Serialize;

use crate::config::Config;

pub struct EmailService {
    provider: String,
    from_name: String,
    from_email: String,
    /// Pre-built lettre transport (only Some when provider == "smtp").
    smtp_transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    /// Postmark server API token (only Some when provider == "postmark").
    postmark_api_key: Option<String>,
    /// OneSignal app ID + REST API key (only Some when provider == "onesignal").
    onesignal_app_id: Option<String>,
    onesignal_api_key: Option<String>,
    /// Shared reqwest client for HTTP-based providers.
    http: reqwest::Client,
}

impl EmailService {
    /// Build the service from configuration.
    ///
    /// The SMTP transport is constructed eagerly so connection errors surface
    /// at startup rather than at first send.
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

    /// Send an email. Returns a provider-specific message ID string.
    pub async fn send(
        &self,
        to_email: &str,
        to_name: Option<&str>,
        subject: &str,
        html_body: &str,
        text_body: Option<&str>,
    ) -> Result<String, String> {
        match self.provider.as_str() {
            "smtp" => {
                self.send_smtp(to_email, to_name, subject, html_body, text_body)
                    .await
            }
            "postmark" => {
                self.send_postmark(to_email, to_name, subject, html_body, text_body)
                    .await
            }
            "onesignal" => {
                self.send_onesignal(to_email, to_name, subject, html_body, text_body)
                    .await
            }
            other => Err(format!("Unknown email provider: {}", other)),
        }
    }

    // -------------------------------------------------------------------------
    // SMTP via lettre
    // -------------------------------------------------------------------------

    async fn send_smtp(
        &self,
        to_email: &str,
        to_name: Option<&str>,
        subject: &str,
        html_body: &str,
        text_body: Option<&str>,
    ) -> Result<String, String> {
        let transport = self
            .smtp_transport
            .as_ref()
            .ok_or("SMTP transport not configured")?;

        let from_mailbox = format!("{} <{}>", self.from_name, self.from_email)
            .parse()
            .map_err(|e| format!("Invalid from address: {e}"))?;

        let to_addr = match to_name {
            Some(name) => format!("{name} <{to_email}>"),
            None => to_email.to_string(),
        };
        let to_mailbox = to_addr
            .parse()
            .map_err(|e| format!("Invalid to address: {e}"))?;

        let body = build_multipart_body(html_body, text_body)?;

        let message = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .multipart(body)
            .map_err(|e| format!("Failed to build email message: {e}"))?;

        let response = transport
            .send(message)
            .await
            .map_err(|e| format!("SMTP send failed: {e}"))?;

        let message_id = response
            .message()
            .next()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("smtp-{}", nanoid::nanoid!(12)));

        tracing::info!(
            provider = "smtp",
            to = to_email,
            subject = subject,
            message_id = %message_id,
            "Email sent via SMTP"
        );

        Ok(message_id)
    }

    // -------------------------------------------------------------------------
    // Postmark — HTTP API
    // -------------------------------------------------------------------------

    async fn send_postmark(
        &self,
        to_email: &str,
        to_name: Option<&str>,
        subject: &str,
        html_body: &str,
        text_body: Option<&str>,
    ) -> Result<String, String> {
        let api_key = self
            .postmark_api_key
            .as_deref()
            .ok_or("POSTMARK_API_KEY not configured")?;

        let to = match to_name {
            Some(name) => format!("{name} <{to_email}>"),
            None => to_email.to_string(),
        };
        let from = format!("{} <{}>", self.from_name, self.from_email);

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
            message_stream: &'a str,
        }

        let payload = PostmarkPayload {
            from: &from,
            to: &to,
            subject,
            html_body,
            text_body,
            message_stream: "outbound",
        };

        let resp = self
            .http
            .post("https://api.postmarkapp.com/email")
            .header("X-Postmark-Server-Token", api_key)
            .header("Accept", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Postmark HTTP request failed: {e}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Postmark response parse failed: {e}"))?;

        if !status.is_success() {
            let err_msg = body
                .get("Message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(format!("Postmark API error {status}: {err_msg}"));
        }

        let message_id = body
            .get("MessageID")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("pm-{}", nanoid::nanoid!(12)));

        tracing::info!(
            provider = "postmark",
            to = to_email,
            subject = subject,
            message_id = %message_id,
            "Email sent via Postmark"
        );

        Ok(message_id)
    }

    // -------------------------------------------------------------------------
    // OneSignal — Email API
    // -------------------------------------------------------------------------

    async fn send_onesignal(
        &self,
        to_email: &str,
        to_name: Option<&str>,
        subject: &str,
        html_body: &str,
        text_body: Option<&str>,
    ) -> Result<String, String> {
        let app_id = self
            .onesignal_app_id
            .as_deref()
            .ok_or("ONESIGNAL_APP_ID not configured")?;
        let api_key = self
            .onesignal_api_key
            .as_deref()
            .ok_or("ONESIGNAL_API_KEY not configured")?;

        #[derive(Serialize)]
        struct OsEmailAddress<'a> {
            email: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            name: Option<&'a str>,
        }

        #[derive(Serialize)]
        struct OsPayload<'a> {
            app_id: &'a str,
            email_subject: &'a str,
            email_body: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            email_preheader: Option<&'a str>,
            include_email_tokens: Vec<OsEmailAddress<'a>>,
        }

        let payload = OsPayload {
            app_id,
            email_subject: subject,
            email_body: html_body,
            email_preheader: text_body,
            include_email_tokens: vec![OsEmailAddress {
                email: to_email,
                name: to_name,
            }],
        };

        let resp = self
            .http
            .post("https://onesignal.com/api/v1/notifications")
            .header("Authorization", format!("Basic {api_key}"))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("OneSignal HTTP request failed: {e}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("OneSignal response parse failed: {e}"))?;

        if !status.is_success() {
            let errors = body
                .get("errors")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown error".to_string());
            return Err(format!("OneSignal API error {status}: {errors}"));
        }

        let notification_id = body
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("os-{}", nanoid::nanoid!(12)));

        tracing::info!(
            provider = "onesignal",
            to = to_email,
            subject = subject,
            notification_id = %notification_id,
            "Email sent via OneSignal"
        );

        Ok(notification_id)
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    pub fn from_name(&self) -> &str {
        &self.from_name
    }

    pub fn from_email(&self) -> &str {
        &self.from_email
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `lettre` SMTP transport from the config.
///
/// Chooses STARTTLS (port 587) when `smtp_secure` is true, otherwise falls
/// back to an unencrypted relay on the configured port (handy for local
/// MailHog / Mailpit dev servers).
fn build_smtp_transport(config: &Config) -> AsyncSmtpTransport<Tokio1Executor> {
    let host = config.smtp_host.as_str();
    let port = config.smtp_port;
    let has_creds = !config.smtp_user.is_empty();

    if config.smtp_secure {
        // STARTTLS — upgrade the plaintext connection.
        let tls_params = TlsParameters::new(host.to_string())
            .expect("failed to build TLS parameters for SMTP");

        let mut builder =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .port(port)
                .tls(Tls::Required(tls_params))
                .authentication(vec![Mechanism::Plain, Mechanism::Login]);

        if has_creds {
            builder = builder.credentials(Credentials::new(
                config.smtp_user.clone(),
                config.smtp_pass.clone(),
            ));
        }

        builder.build()
    } else {
        // Plain / local relay (MailHog, Mailpit, etc.)
        let mut builder =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .port(port)
                .tls(Tls::None);

        if has_creds {
            builder = builder
                .credentials(Credentials::new(
                    config.smtp_user.clone(),
                    config.smtp_pass.clone(),
                ))
                .authentication(vec![Mechanism::Plain, Mechanism::Login]);
        }

        builder.build()
    }
}

/// Build a multipart MIME body: `multipart/alternative` when a plain-text
/// fallback is provided, otherwise a single HTML part.
fn build_multipart_body(
    html_body: &str,
    text_body: Option<&str>,
) -> Result<MultiPart, String> {
    if let Some(text) = text_body {
        MultiPart::alternative()
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_PLAIN)
                    .body(text.to_string()),
            )
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(html_body.to_string()),
            )
            .pipe(Ok)
    } else {
        MultiPart::alternative()
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(html_body.to_string()),
            )
            .pipe(Ok)
    }
}

/// A tiny extension trait so we can write `expr.pipe(Ok)` instead of a
/// temporary binding when returning `Result<MultiPart, String>`.
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl Pipe for MultiPart {}
