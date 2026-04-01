//! Notification service.
//! Handles OneSignal push notifications and in-app notification delivery.

use crate::config::Config;

pub struct NotificationService {
    onesignal_app_id: Option<String>,
    onesignal_api_key: Option<String>,
}

impl NotificationService {
    pub fn new(config: &Config) -> Self {
        Self {
            onesignal_app_id: config.onesignal_app_id.clone(),
            onesignal_api_key: config.onesignal_api_key.clone(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.onesignal_app_id.is_some() && self.onesignal_api_key.is_some()
    }

    /// Send a push notification to a user via OneSignal.
    pub async fn send_push(
        &self,
        external_id: &str,
        title: &str,
        message: &str,
        data: Option<serde_json::Value>,
    ) -> Result<String, String> {
        if !self.is_configured() {
            return Err("OneSignal not configured".to_string());
        }

        // TODO: Wire to OneSignal REST API via reqwest
        tracing::info!(
            provider = "onesignal",
            external_id = external_id,
            title = title,
            "Push notification sent (placeholder)"
        );
        let _ = (message, data);
        Ok(format!("onesignal-{}", nanoid::nanoid!(12)))
    }

    /// Send a notification via the appropriate channel.
    pub async fn deliver(
        &self,
        channel: &str,
        recipient_id: &str,
        title: &str,
        message: &str,
        data: Option<serde_json::Value>,
    ) -> Result<(), String> {
        match channel {
            "push" => {
                self.send_push(recipient_id, title, message, data).await?;
                Ok(())
            }
            "email" => {
                // Delegate to email service (not handled here)
                tracing::debug!(channel = "email", recipient = recipient_id, "Email notification — delegate to email service");
                Ok(())
            }
            "in-app" => {
                // In-app notifications are stored in DB and delivered via WebSocket
                tracing::debug!(channel = "in-app", recipient = recipient_id, "In-app notification stored");
                Ok(())
            }
            _ => Err(format!("Unknown notification channel: {}", channel)),
        }
    }
}
