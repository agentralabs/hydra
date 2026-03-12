use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{
    AnthropicOAuth, OAuthState, OAuthTokens, TokenResponse,
    ANTHROPIC_TOKEN_URL, ANTHROPIC_USERINFO_URL, CLIENT_ID, REDIRECT_PORT,
};
use super::helpers::extract_code_from_request;

impl AnthropicOAuth {
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

    /// Fetch account info (email, subscription tier).
    pub(super) async fn fetch_account_info(&mut self) -> Result<(), String> {
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
}
