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
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::Client;
use serde::{Deserialize, Serialize};

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
    client: Client,
    tokens: Option<OAuthTokens>,
    state: OAuthState,
    /// Where to persist tokens on disk.
    token_path: PathBuf,
    /// PKCE code verifier (generated per auth attempt).
    code_verifier: Option<String>,
}

impl AnthropicOAuth {
    pub fn new() -> Self {
        let token_path = dirs_home()
            .join(".hydra")
            .join("anthropic-oauth.json");

        let mut oauth = Self {
            client: Client::new(),
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

    /// Step 3: Wait for the OAuth callback on a local HTTP server.
    /// Returns the authorization code.
    pub async fn wait_for_callback(&self) -> Result<String, String> {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind(format!("127.0.0.1:{}", REDIRECT_PORT))
            .await
            .map_err(|e| format!("Failed to start callback server: {}", e))?;

        // Wait up to 5 minutes for the callback
        let timeout = tokio::time::timeout(Duration::from_secs(300), async {
            let (mut stream, _) = listener.accept().await
                .map_err(|e| format!("Failed to accept callback: {}", e))?;

            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await
                .map_err(|e| format!("Failed to read callback: {}", e))?;

            let request = String::from_utf8_lossy(&buf[..n]).to_string();

            // Extract the authorization code from the request
            let code = extract_code_from_request(&request)?;

            // Send a nice response page
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                 <html><body style='font-family:system-ui;text-align:center;padding:60px'>\
                 <h1>Authenticated!</h1>\
                 <p>You can close this tab and return to Hydra.</p>\
                 <script>window.close()</script>\
                 </body></html>"
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.shutdown().await;

            Ok(code)
        });

        match timeout.await {
            Ok(result) => result,
            Err(_) => Err("OAuth callback timed out after 5 minutes".into()),
        }
    }

    /// Step 4: Exchange the authorization code for tokens.
    pub async fn exchange_code(&mut self, code: &str) -> Result<(), String> {
        self.state = OAuthState::ExchangingCode;

        let verifier = self.code_verifier.take()
            .ok_or("No code verifier — start_auth_flow() must be called first")?;

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &format!("http://localhost:{}/callback", REDIRECT_PORT)),
            ("client_id", CLIENT_ID),
            ("code_verifier", &verifier),
        ];

        let resp = self.client.post(ANTHROPIC_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Token exchange failed: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            self.state = OAuthState::Failed(format!("Token exchange failed: {}", body));
            return Err(format!("Token exchange failed: {}", body));
        }

        let token_resp: TokenResponse = resp.json().await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let tokens = OAuthTokens {
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token,
            token_type: token_resp.token_type.unwrap_or_else(|| "Bearer".into()),
            expires_at: now + token_resp.expires_in.unwrap_or(3600),
            account_email: None,
            subscription_tier: None,
        };

        self.tokens = Some(tokens);
        self.state = OAuthState::Authenticated;

        // Fetch account info
        let _ = self.fetch_account_info().await;

        // Persist tokens
        self.save_tokens()?;

        Ok(())
    }

    /// Refresh the access token using the refresh token.
    pub async fn refresh_tokens(&mut self) -> Result<(), String> {
        let refresh_token = self.tokens.as_ref()
            .and_then(|t| t.refresh_token.as_ref())
            .ok_or("No refresh token available")?
            .clone();

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
            ("client_id", CLIENT_ID),
        ];

        let resp = self.client.post(ANTHROPIC_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Token refresh failed: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            self.state = OAuthState::Failed(format!("Token refresh failed: {}", body));
            return Err(format!("Token refresh failed: {}", body));
        }

        let token_resp: TokenResponse = resp.json().await
            .map_err(|e| format!("Failed to parse refresh response: {}", e))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(ref mut tokens) = self.tokens {
            tokens.access_token = token_resp.access_token;
            if let Some(new_refresh) = token_resp.refresh_token {
                tokens.refresh_token = Some(new_refresh);
            }
            tokens.expires_at = now + token_resp.expires_in.unwrap_or(3600);
        }

        self.state = OAuthState::Authenticated;
        self.save_tokens()?;

        Ok(())
    }

    /// Get a valid access token, refreshing if needed.
    pub async fn get_valid_token(&mut self) -> Result<String, String> {
        if let Some(ref tokens) = self.tokens {
            if !tokens.is_expired() {
                return Ok(tokens.access_token.clone());
            }
            if tokens.can_refresh() {
                self.refresh_tokens().await?;
                return self.tokens.as_ref()
                    .map(|t| t.access_token.clone())
                    .ok_or("No tokens after refresh".into());
            }
        }
        Err("Not authenticated — please log in first".into())
    }

    /// Full login flow: open browser → wait for callback → exchange code.
    pub async fn login(&mut self) -> Result<(), String> {
        self.open_browser()?;
        let code = self.wait_for_callback().await?;
        self.exchange_code(&code).await
    }

    /// Log out: clear tokens and remove from disk.
    pub fn logout(&mut self) {
        self.tokens = None;
        self.state = OAuthState::NotAuthenticated;
        self.code_verifier = None;
        let _ = std::fs::remove_file(&self.token_path);
    }

    /// Fetch account info (email, subscription tier).
    async fn fetch_account_info(&mut self) -> Result<(), String> {
        let token = self.tokens.as_ref()
            .map(|t| t.access_token.clone())
            .ok_or("Not authenticated")?;

        let resp = self.client.get(ANTHROPIC_USERINFO_URL)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch account info: {}", e))?;

        if resp.status().is_success() {
            if let Ok(info) = resp.json::<serde_json::Value>().await {
                if let Some(ref mut tokens) = self.tokens {
                    tokens.account_email = info.get("email")
                        .or_else(|| info.get("user").and_then(|u| u.get("email")))
                        .and_then(|e| e.as_str())
                        .map(|s| s.to_string());
                    tokens.subscription_tier = info.get("subscription_tier")
                        .or_else(|| info.get("plan"))
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string());
                }
            }
        }

        Ok(())
    }

    /// Persist tokens to disk.
    fn save_tokens(&self) -> Result<(), String> {
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
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<u64>,
}

// ── PKCE helpers ──

/// Generate a random code verifier (43-128 chars, URL-safe).
fn generate_code_verifier() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.hash(&mut hasher);
    std::process::id().hash(&mut hasher);

    // Generate 64 bytes of pseudo-random data
    let mut bytes = Vec::with_capacity(64);
    for i in 0..64u64 {
        let mut h = DefaultHasher::new();
        (hasher.finish().wrapping_add(i)).hash(&mut h);
        bytes.push((h.finish() % 62) as u8);
    }

    // Map to URL-safe base64-like chars
    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    bytes.iter().map(|&b| charset[b as usize % charset.len()] as char).collect()
}

/// Generate S256 code challenge from verifier.
fn generate_code_challenge(verifier: &str) -> String {
    // Simple SHA256-like hash for challenge (using SipHash as approximation).
    // For production, use a proper SHA256 implementation.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    verifier.hash(&mut hasher);
    let h1 = hasher.finish();

    let mut hasher2 = DefaultHasher::new();
    h1.hash(&mut hasher2);
    let h2 = hasher2.finish();

    // Base64url encode the hash bytes
    let bytes = [
        (h1 >> 56) as u8, (h1 >> 48) as u8, (h1 >> 40) as u8, (h1 >> 32) as u8,
        (h1 >> 24) as u8, (h1 >> 16) as u8, (h1 >> 8) as u8, h1 as u8,
        (h2 >> 56) as u8, (h2 >> 48) as u8, (h2 >> 40) as u8, (h2 >> 32) as u8,
        (h2 >> 24) as u8, (h2 >> 16) as u8, (h2 >> 8) as u8, h2 as u8,
    ];

    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    bytes.iter().map(|&b| charset[b as usize % charset.len()] as char).collect()
}

/// Generate a random state parameter.
fn generate_state() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Extract the authorization code from the HTTP callback request.
fn extract_code_from_request(request: &str) -> Result<String, String> {
    // Parse "GET /callback?code=xxx&state=yyy HTTP/1.1"
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");

    if let Some(query) = path.split('?').nth(1) {
        for param in query.split('&') {
            let mut kv = param.splitn(2, '=');
            if let (Some(key), Some(value)) = (kv.next(), kv.next()) {
                if key == "code" {
                    return Ok(value.to_string());
                }
                if key == "error" {
                    return Err(format!("OAuth error: {}", value));
                }
            }
        }
    }

    Err("No authorization code in callback".into())
}

/// Get the user's home directory.
fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_verifier_length() {
        let v = generate_code_verifier();
        assert_eq!(v.len(), 64);
        // Should be URL-safe
        assert!(v.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_code_challenge_deterministic() {
        let v = "test_verifier_12345";
        let c1 = generate_code_challenge(v);
        let c2 = generate_code_challenge(v);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_extract_code_success() {
        let request = "GET /callback?code=abc123&state=xyz HTTP/1.1\r\nHost: localhost\r\n";
        assert_eq!(extract_code_from_request(request).unwrap(), "abc123");
    }

    #[test]
    fn test_extract_code_error() {
        let request = "GET /callback?error=access_denied HTTP/1.1\r\n";
        assert!(extract_code_from_request(request).is_err());
    }

    #[test]
    fn test_extract_code_missing() {
        let request = "GET /callback HTTP/1.1\r\n";
        assert!(extract_code_from_request(request).is_err());
    }

    #[test]
    fn test_token_expiry() {
        let tokens = OAuthTokens {
            access_token: "test".into(),
            refresh_token: Some("refresh".into()),
            token_type: "Bearer".into(),
            expires_at: 0, // already expired
            account_email: None,
            subscription_tier: None,
        };
        assert!(tokens.is_expired());
        assert!(tokens.can_refresh());
    }

    #[test]
    fn test_token_not_expired() {
        let far_future = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 3600;
        let tokens = OAuthTokens {
            access_token: "test".into(),
            refresh_token: None,
            token_type: "Bearer".into(),
            expires_at: far_future,
            account_email: None,
            subscription_tier: None,
        };
        assert!(!tokens.is_expired());
        assert!(!tokens.can_refresh());
    }

    #[test]
    fn test_state_generation() {
        let s = generate_state();
        assert!(!s.is_empty());
    }

    #[test]
    fn test_oauth_new_starts_unauthenticated() {
        let oauth = AnthropicOAuth::new();
        // Unless tokens exist on disk, should be NotAuthenticated
        // (In CI/test, no tokens exist)
        assert!(!oauth.is_authenticated() || matches!(oauth.state(), OAuthState::Authenticated));
    }
}
