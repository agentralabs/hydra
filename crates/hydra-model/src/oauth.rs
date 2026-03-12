//! Anthropic OAuth — browser-based authentication using Claude subscription credits.
//!
//! This implements the OAuth2 PKCE flow that allows users to authenticate via
//! console.anthropic.com and use their Claude Pro/Max subscription credits
//! directly from Hydra, without needing a separate API key.
//!
//! Flow:
//! 1. Generate PKCE code verifier + challenge
//! 2. Open browser to Anthropic's authorization URL
//! 3. Start a local HTTP server to catch the redirect callback
//! 4. Exchange the authorization code for access + refresh tokens
//! 5. Store tokens securely and use them for API calls
//! 6. Auto-refresh tokens before expiry

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;
use serde::{Deserialize, Serialize};

mod helpers;
mod flow;
mod tests;

use helpers::{
    dirs_home, generate_code_challenge, generate_code_verifier,
    generate_state,
};

// ── Anthropic OAuth constants ──

const ANTHROPIC_AUTH_URL: &str = "https://console.anthropic.com/oauth/authorize";
const ANTHROPIC_TOKEN_URL: &str = "https://console.anthropic.com/oauth/token";
const ANTHROPIC_USERINFO_URL: &str = "https://console.anthropic.com/api/auth/session";
const REDIRECT_PORT: u16 = 19285;
const CLIENT_ID: &str = "hydra-desktop";

/// OAuth token pair with expiry tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    /// Unix timestamp when the access token expires.
    pub expires_at: u64,
    /// The user's email/account from Anthropic.
    pub account_email: Option<String>,
    /// Subscription tier (e.g. "pro", "max", "team").
    pub subscription_tier: Option<String>,
}

impl OAuthTokens {
    /// Whether the access token has expired (with 60s buffer).
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now + 60 >= self.expires_at
    }

    /// Whether we can refresh (have a refresh token).
    pub fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
    }
}

/// OAuth flow state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OAuthState {
    /// Not authenticated.
    NotAuthenticated,
    /// Browser opened, waiting for callback.
    AwaitingCallback,
    /// Exchanging code for tokens.
    ExchangingCode,
    /// Successfully authenticated.
    Authenticated,
    /// Authentication failed.
    Failed(String),
}

/// Manages the Anthropic OAuth lifecycle.
pub struct AnthropicOAuth {
    pub(crate) client: Client,
    pub(crate) tokens: Option<OAuthTokens>,
    pub(crate) state: OAuthState,
    /// Where to persist tokens on disk.
    pub(crate) token_path: PathBuf,
    /// PKCE code verifier (generated per auth attempt).
    pub(crate) code_verifier: Option<String>,
}

impl AnthropicOAuth {
    pub fn new() -> Self {
        let token_path = dirs_home()
            .join(".hydra")
            .join("anthropic-oauth.json");

        let mut oauth = Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
            tokens: None,
            state: OAuthState::NotAuthenticated,
            token_path,
            code_verifier: None,
        };

        // Try to load existing tokens
        if let Ok(data) = std::fs::read_to_string(&oauth.token_path) {
            if let Ok(tokens) = serde_json::from_str::<OAuthTokens>(&data) {
                if !tokens.is_expired() || tokens.can_refresh() {
                    oauth.state = OAuthState::Authenticated;
                    oauth.tokens = Some(tokens);
                }
            }
        }

        oauth
    }

    /// Current authentication state.
    pub fn state(&self) -> &OAuthState {
        &self.state
    }

    /// Whether we're authenticated with valid tokens.
    pub fn is_authenticated(&self) -> bool {
        self.state == OAuthState::Authenticated && self.tokens.is_some()
    }

    /// Get the current access token (if authenticated).
    pub fn access_token(&self) -> Option<&str> {
        self.tokens.as_ref().map(|t| t.access_token.as_str())
    }

    /// Get account info.
    pub fn account_email(&self) -> Option<&str> {
        self.tokens.as_ref().and_then(|t| t.account_email.as_deref())
    }

    pub fn subscription_tier(&self) -> Option<&str> {
        self.tokens.as_ref().and_then(|t| t.subscription_tier.as_deref())
    }

    /// Step 1: Generate the authorization URL and open it in the browser.
    /// Returns the URL that should be opened.
    pub fn start_auth_flow(&mut self) -> String {
        let verifier = generate_code_verifier();
        let challenge = generate_code_challenge(&verifier);
        let state = generate_state();

        self.code_verifier = Some(verifier);
        self.state = OAuthState::AwaitingCallback;

        format!(
            "{}?response_type=code&client_id={}&redirect_uri=http://localhost:{}/callback&code_challenge={}&code_challenge_method=S256&state={}&scope=user:inference",
            ANTHROPIC_AUTH_URL, CLIENT_ID, REDIRECT_PORT, challenge, state
        )
    }

    /// Step 2: Open the browser to the authorization URL.
    pub fn open_browser(&mut self) -> Result<(), String> {
        let url = self.start_auth_flow();

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&url)
                .spawn()
                .map_err(|e| format!("Failed to open browser: {}", e))?;
        }

        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(&url)
                .spawn()
                .map_err(|e| format!("Failed to open browser: {}", e))?;
        }

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", &url])
                .spawn()
                .map_err(|e| format!("Failed to open browser: {}", e))?;
        }

        Ok(())
    }

    /// Log out: clear tokens and remove from disk.
    pub fn logout(&mut self) {
        self.tokens = None;
        self.state = OAuthState::NotAuthenticated;
        self.code_verifier = None;
        let _ = std::fs::remove_file(&self.token_path);
    }

    /// Persist tokens to disk.
    pub(crate) fn save_tokens(&self) -> Result<(), String> {
        if let Some(ref tokens) = self.tokens {
            // Ensure directory exists
            if let Some(parent) = self.token_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create token directory: {}", e))?;
            }
            let json = serde_json::to_string_pretty(tokens)
                .map_err(|e| format!("Failed to serialize tokens: {}", e))?;
            std::fs::write(&self.token_path, json)
                .map_err(|e| format!("Failed to save tokens: {}", e))?;

            // Restrict file permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(
                    &self.token_path,
                    std::fs::Permissions::from_mode(0o600),
                );
            }
        }
        Ok(())
    }
}

// ── Token response from Anthropic ──

#[derive(Deserialize)]
pub(crate) struct TokenResponse {
    pub(crate) access_token: String,
    pub(crate) refresh_token: Option<String>,
    pub(crate) token_type: Option<String>,
    pub(crate) expires_in: Option<u64>,
}
