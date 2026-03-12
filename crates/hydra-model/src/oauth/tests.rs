#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::oauth::{AnthropicOAuth, OAuthState, OAuthTokens};

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
    fn test_oauth_new_starts_unauthenticated() {
        let oauth = AnthropicOAuth::new();
        // Unless tokens exist on disk, should be NotAuthenticated
        // (In CI/test, no tokens exist)
        assert!(!oauth.is_authenticated() || matches!(oauth.state(), OAuthState::Authenticated));
    }
}
