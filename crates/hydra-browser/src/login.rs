//! LoginManager — automated login with vault credentials, 2FA, and session persistence.

use crate::action::BrowserAction;
use crate::constants::LOGIN_MAX_RETRIES;
use crate::engine::BrowserEngine;
use crate::errors::BrowserError;
use crate::page::{FormType, PageAnalyzer};
use crate::vision::VisionProvider;

use std::collections::HashMap;
use std::path::Path;

/// Manages login flows for any website.
pub struct LoginManager;

impl LoginManager {
    /// Detect if the current page is a login page.
    pub fn detect_login_page(html: &str) -> bool {
        let forms = PageAnalyzer::detect_forms(html);
        forms.iter().any(|f| f.form_type == FormType::Login)
    }

    /// Find credentials for a domain from the vault.
    pub fn find_credentials(domain: &str) -> Result<LoginCredentials, BrowserError> {
        let vault_path = Path::new("vault").join(format!("{domain}.toml"));
        if !vault_path.exists() {
            // Try base domain (e.g., twitter.com from login.twitter.com)
            let base = Self::base_domain(domain);
            let alt_path = Path::new("vault").join(format!("{base}.toml"));
            if !alt_path.exists() {
                return Err(BrowserError::NoCredentials(domain.into()));
            }
            return Self::parse_vault(&alt_path, domain);
        }
        Self::parse_vault(&vault_path, domain)
    }

    /// Execute a full login flow: detect form → fill credentials → submit → verify.
    pub async fn login(
        engine: &mut BrowserEngine,
        domain: &str,
        _vision: Option<&dyn VisionProvider>,
    ) -> Result<LoginResult, BrowserError> {
        // Check for existing valid session
        if engine.session_manager_mut().has_valid_session(domain) {
            eprintln!("hydra-browser: reusing existing session for {domain}");
            return Ok(LoginResult::SessionReused);
        }

        let creds = Self::find_credentials(domain)?;

        for attempt in 0..LOGIN_MAX_RETRIES {
            eprintln!("hydra-browser: login attempt {}/{} for {domain}", attempt + 1, LOGIN_MAX_RETRIES);

            // Get current page HTML
            let html = engine.html().await?;

            if !Self::detect_login_page(&html) {
                // Maybe already logged in?
                return Ok(LoginResult::AlreadyLoggedIn);
            }

            // Find and fill the form
            let forms = PageAnalyzer::detect_forms(&html);
            let login_form = forms.iter().find(|f| f.form_type == FormType::Login);

            if let Some(form) = login_form {
                // Fill username/email
                if let Some(user_field) = form.fields.iter().find(|f| f.field_type != "password") {
                    engine
                        .execute(&BrowserAction::Type {
                            selector: user_field.selector.clone(),
                            text: creds.username.clone(),
                        })
                        .await;
                }

                // Fill password
                if let Some(pass_field) = form.fields.iter().find(|f| f.field_type == "password") {
                    engine
                        .execute(&BrowserAction::Type {
                            selector: pass_field.selector.clone(),
                            text: creds.password.clone(),
                        })
                        .await;
                }

                // Click submit
                if let Some(submit) = &form.submit_selector {
                    engine
                        .execute(&BrowserAction::Click {
                            selector: submit.clone(),
                        })
                        .await;
                }

                // Wait for navigation
                engine
                    .execute(&BrowserAction::Wait { ms: 2000 })
                    .await;

                // Verify login success
                let post_html = engine.html().await?;
                if !Self::detect_login_page(&post_html) {
                    // Login succeeded — save session
                    engine
                        .session_manager_mut()
                        .save_cookies(domain, vec![])?; // TODO: extract actual cookies from CDP
                    eprintln!("hydra-browser: login succeeded for {domain}");
                    return Ok(LoginResult::Success);
                }

                // Check for 2FA
                if Self::detect_2fa(&post_html) {
                    if let Some(totp_secret) = &creds.totp_secret {
                        let code = Self::generate_totp(totp_secret)?;
                        eprintln!("hydra-browser: entering 2FA code for {domain}");
                        // Find 2FA input and enter code
                        engine
                            .execute(&BrowserAction::Type {
                                selector: "input[name*=\"code\"], input[name*=\"otp\"], input[name*=\"2fa\"], input[type=\"tel\"]".into(),
                                text: code,
                            })
                            .await;
                        engine
                            .execute(&BrowserAction::Click {
                                selector: "button[type=\"submit\"], button".into(),
                            })
                            .await;
                        engine.execute(&BrowserAction::Wait { ms: 2000 }).await;

                        let final_html = engine.html().await?;
                        if !Self::detect_login_page(&final_html) {
                            engine.session_manager_mut().save_cookies(domain, vec![])?;
                            return Ok(LoginResult::SuccessWith2FA);
                        }
                    }
                }
            }
        }

        Err(BrowserError::LoginFailed {
            domain: domain.into(),
            reason: format!("Failed after {} attempts", LOGIN_MAX_RETRIES),
        })
    }

    /// Generate a TOTP code from a secret.
    pub fn generate_totp(secret: &str) -> Result<String, BrowserError> {
        use totp_rs::{Algorithm, Secret, TOTP};
        let decoded = Secret::Raw(secret.as_bytes().to_vec())
            .to_bytes()
            .map_err(|e| BrowserError::LoginFailed {
                domain: "(totp)".into(),
                reason: format!("Invalid TOTP secret: {e}"),
            })?;
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            decoded,
            None,
            "hydra".to_string(),
        )
        .map_err(|e| BrowserError::LoginFailed {
            domain: "(totp)".into(),
            reason: format!("TOTP creation failed: {e}"),
        })?;
        totp.generate_current().map_err(|e| BrowserError::LoginFailed {
            domain: "(totp)".into(),
            reason: format!("TOTP generation failed: {e}"),
        })
    }

    fn detect_2fa(html: &str) -> bool {
        let lower = html.to_lowercase();
        lower.contains("two-factor")
            || lower.contains("2fa")
            || lower.contains("verification code")
            || lower.contains("authenticator")
            || lower.contains("one-time")
    }

    fn base_domain(domain: &str) -> &str {
        let parts: Vec<&str> = domain.split('.').collect();
        if parts.len() >= 2 {
            let start = parts.len() - 2;
            // Return last two parts (e.g., "twitter.com")
            &domain[domain.len() - parts[start..].join(".").len()..]
        } else {
            domain
        }
    }

    fn parse_vault(
        path: &Path,
        domain: &str,
    ) -> Result<LoginCredentials, BrowserError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            BrowserError::NoCredentials(format!("{domain}: {e}"))
        })?;

        #[derive(serde::Deserialize)]
        struct VaultLogin {
            #[serde(default)]
            credentials: HashMap<String, String>,
        }

        let vault: VaultLogin =
            toml::from_str(&content).map_err(|e| BrowserError::NoCredentials(format!("{e}")))?;

        Ok(LoginCredentials {
            username: vault
                .credentials
                .get("username")
                .or_else(|| vault.credentials.get("email"))
                .cloned()
                .unwrap_or_default(),
            password: vault
                .credentials
                .get("password")
                .cloned()
                .unwrap_or_default(),
            totp_secret: vault.credentials.get("totp_secret").cloned(),
        })
    }
}

/// Credentials for a login.
#[derive(Debug, Clone)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
    pub totp_secret: Option<String>,
}

/// Login attempt result.
#[derive(Debug, Clone, PartialEq)]
pub enum LoginResult {
    Success,
    SuccessWith2FA,
    SessionReused,
    AlreadyLoggedIn,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_login_page_positive() {
        let html = r#"<form><input type="password"><button>Sign In</button></form>"#;
        assert!(LoginManager::detect_login_page(html));
    }

    #[test]
    fn detect_login_page_negative() {
        let html = r#"<html><body><h1>Welcome</h1><p>Content</p></body></html>"#;
        assert!(!LoginManager::detect_login_page(html));
    }

    #[test]
    fn detect_2fa_page() {
        let html = r#"<html><body><h1>Two-Factor Authentication</h1><input name="code"></body></html>"#;
        assert!(LoginManager::detect_2fa(html));
    }

    #[test]
    fn base_domain_extraction() {
        assert_eq!(LoginManager::base_domain("login.twitter.com"), "twitter.com");
        assert_eq!(LoginManager::base_domain("example.com"), "example.com");
    }

    #[test]
    fn missing_credentials_returns_error() {
        let result = LoginManager::find_credentials("nonexistent-domain-xyz.test");
        assert!(result.is_err());
    }
}
