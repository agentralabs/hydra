use async_trait::async_trait;
use tracing::debug;

use super::push::{PushError, PushMessage, PushProvider};

// ═══════════════════════════════════════════════════════════
// NTFY PROVIDER
// ═══════════════════════════════════════════════════════════

/// Push provider using ntfy.sh (or self-hosted ntfy instance)
pub struct NtfyProvider {
    pub topic_id: String,
    pub server_url: String,
    client: reqwest::Client,
}

impl NtfyProvider {
    pub fn new(topic_id: String) -> Self {
        Self {
            topic_id,
            server_url: "https://ntfy.sh".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_server(topic_id: String, server_url: String) -> Self {
        Self {
            topic_id,
            server_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PushProvider for NtfyProvider {
    async fn send(&self, message: &PushMessage) -> Result<(), PushError> {
        let url = format!("{}/{}", self.server_url, self.topic_id);
        debug!("Sending ntfy notification to {}", url);

        let mut request = self
            .client
            .post(&url)
            .header("Title", &message.title)
            .header("Priority", urgency_to_ntfy_priority(&message.urgency));

        if let Some(ref action_url) = message.action_url {
            request = request.header("Click", action_url);
        }

        let response = request
            .body(message.body.clone())
            .send()
            .await
            .map_err(|e| PushError::NetworkError(e.to_string()))?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else if status.as_u16() == 401 || status.as_u16() == 403 {
            Err(PushError::AuthError(format!(
                "ntfy returned {}",
                status.as_u16()
            )))
        } else if status.as_u16() == 429 {
            Err(PushError::RateLimited)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(PushError::ProviderError(format!(
                "ntfy returned {}: {}",
                status.as_u16(),
                body
            )))
        }
    }

    fn provider_name(&self) -> &str {
        "ntfy"
    }
}

pub fn urgency_to_ntfy_priority(urgency: &str) -> &'static str {
    match urgency {
        "high" | "urgent" => "urgent",
        "low" => "low",
        _ => "default",
    }
}

// ═══════════════════════════════════════════════════════════
// WEB PUSH PROVIDER
// ═══════════════════════════════════════════════════════════

/// Web Push provider using VAPID keys
///
/// Web Push requires VAPID key generation and encryption of the payload
/// using the push subscription's public key. This implementation provides
/// the structure; full encryption would use the `web-push` crate.
pub struct WebPushProvider {
    pub vapid_key: String,
    pub subscription_endpoint: String,
    client: reqwest::Client,
}

impl WebPushProvider {
    pub fn new(vapid_key: String, subscription_endpoint: String) -> Self {
        Self {
            vapid_key,
            subscription_endpoint,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PushProvider for WebPushProvider {
    async fn send(&self, message: &PushMessage) -> Result<(), PushError> {
        // Web Push requires VAPID JWT signing and payload encryption using
        // the subscription's p256dh and auth keys. A production implementation
        // would use the `web-push` crate. For now we POST the JSON payload
        // to the subscription endpoint as a structural placeholder.
        debug!(
            "Sending web push to endpoint: {}",
            self.subscription_endpoint
        );

        let payload = serde_json::json!({
            "title": message.title,
            "body": message.body,
            "urgency": message.urgency,
            "action_url": message.action_url,
        });

        let response = self
            .client
            .post(&self.subscription_endpoint)
            .header("TTL", "86400")
            .header("Urgency", &message.urgency)
            .json(&payload)
            .send()
            .await
            .map_err(|e| PushError::NetworkError(e.to_string()))?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 201 {
            Ok(())
        } else if status.as_u16() == 404 || status.as_u16() == 410 {
            Err(PushError::DeviceNotRegistered)
        } else if status.as_u16() == 429 {
            Err(PushError::RateLimited)
        } else {
            Err(PushError::ProviderError(format!(
                "web push returned {}",
                status.as_u16()
            )))
        }
    }

    fn provider_name(&self) -> &str {
        "web_push"
    }
}

// ═══════════════════════════════════════════════════════════
// TELEGRAM PROVIDER
// ═══════════════════════════════════════════════════════════

/// Push provider using the Telegram Bot API
pub struct TelegramProvider {
    pub bot_token: String,
    pub chat_id: String,
    client: reqwest::Client,
}

impl TelegramProvider {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            bot_token,
            chat_id,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PushProvider for TelegramProvider {
    async fn send(&self, message: &PushMessage) -> Result<(), PushError> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );
        debug!("Sending Telegram notification to chat {}", self.chat_id);

        let text = if let Some(ref action_url) = message.action_url {
            format!(
                "<b>{}</b>\n\n{}\n\n<a href=\"{}\">Open</a>",
                message.title, message.body, action_url
            )
        } else {
            format!("<b>{}</b>\n\n{}", message.title, message.body)
        };

        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "disable_web_page_preview": true,
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| PushError::NetworkError(e.to_string()))?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else if status.as_u16() == 401 {
            Err(PushError::AuthError(
                "invalid Telegram bot token".to_string(),
            ))
        } else if status.as_u16() == 429 {
            Err(PushError::RateLimited)
        } else {
            let resp_body = response.text().await.unwrap_or_default();
            Err(PushError::ProviderError(format!(
                "Telegram API returned {}: {}",
                status.as_u16(),
                resp_body
            )))
        }
    }

    fn provider_name(&self) -> &str {
        "telegram"
    }
}

// ═══════════════════════════════════════════════════════════
// EMAIL PROVIDER
// ═══════════════════════════════════════════════════════════

/// Push provider using email (SMTP) via the `lettre` crate.
///
/// Sends real emails through any SMTP server (Gmail, Outlook, custom).
/// For Gmail: use smtp.gmail.com with an App Password.
pub struct EmailProvider {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub to_address: String,
}

impl EmailProvider {
    pub fn new(smtp_host: String, from_address: String, to_address: String) -> Self {
        Self {
            smtp_host,
            smtp_port: 587,
            username: from_address.clone(),
            password: String::new(),
            from_address,
            to_address,
        }
    }

    pub fn with_credentials(
        smtp_host: String,
        smtp_port: u16,
        username: String,
        password: String,
        from_address: String,
        to_address: String,
    ) -> Self {
        Self { smtp_host, smtp_port, username, password, from_address, to_address }
    }
}

#[async_trait]
impl PushProvider for EmailProvider {
    async fn send(&self, message: &PushMessage) -> Result<(), PushError> {
        use lettre::{
            AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
            message::header::ContentType,
            transport::smtp::authentication::Credentials,
        };

        debug!(
            "Sending email via SMTP: from={} to={} via={}:{} subject='{}'",
            self.from_address, self.to_address, self.smtp_host, self.smtp_port, message.title
        );

        let email = lettre::Message::builder()
            .from(self.from_address.parse().map_err(|e| {
                PushError::ProviderError(format!("Invalid from address: {}", e))
            })?)
            .to(self.to_address.parse().map_err(|e| {
                PushError::ProviderError(format!("Invalid to address: {}", e))
            })?)
            .subject(&message.title)
            .header(ContentType::TEXT_PLAIN)
            .body(message.body.clone())
            .map_err(|e| PushError::ProviderError(format!("Failed to build email: {}", e)))?;

        let creds = Credentials::new(self.username.clone(), self.password.clone());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_host)
            .map_err(|e| PushError::ProviderError(format!("SMTP relay error: {}", e)))?
            .port(self.smtp_port)
            .credentials(creds)
            .build();

        mailer.send(email).await.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("535") || msg.contains("authentication") || msg.contains("credential") {
                PushError::AuthError(format!("SMTP auth failed: {}", msg))
            } else {
                PushError::NetworkError(format!("SMTP send failed: {}", msg))
            }
        })?;

        debug!("Email sent successfully to {}", self.to_address);
        Ok(())
    }

    fn provider_name(&self) -> &str {
        "email"
    }
}
