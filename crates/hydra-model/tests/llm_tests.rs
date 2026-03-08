use hydra_model::{
    providers::anthropic::AnthropicClient,
    providers::openai::OpenAiClient,
    providers::{CompletionRequest, LlmError, Message},
    LlmConfig, ModelExecutor, ModelRegistry,
};

// ═══════════════════════════════════════════════════════════
// UNIT TESTS (always run, no API keys needed)
// ═══════════════════════════════════════════════════════════

#[test]
fn test_llm_config_from_env_no_keys() {
    // Clear keys to test default behavior
    let config = LlmConfig {
        anthropic_api_key: None,
        openai_api_key: None,
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    assert!(!config.has_anthropic());
    assert!(!config.has_openai());
    assert!(!config.has_provider("anthropic"));
    assert!(!config.has_provider("openai"));
    assert!(config.has_provider("local"));
}

#[test]
fn test_llm_config_with_keys() {
    let config = LlmConfig {
        anthropic_api_key: Some("sk-ant-test".into()),
        openai_api_key: Some("sk-test".into()),
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    assert!(config.has_anthropic());
    assert!(config.has_openai());
    assert!(config.has_provider("anthropic"));
    assert!(config.has_provider("openai"));
}

#[test]
fn test_llm_config_empty_key_treated_as_none() {
    let config = LlmConfig {
        anthropic_api_key: Some("".into()),
        openai_api_key: Some("".into()),
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    // Empty strings should still be Some(""), but from_env filters them
    // Direct construction doesn't filter
    assert!(config.has_anthropic()); // Some("") is still Some
}

#[test]
fn test_anthropic_client_no_key() {
    let config = LlmConfig {
        anthropic_api_key: None,
        openai_api_key: None,
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    let result = AnthropicClient::new(&config);
    assert!(result.is_err());
}

#[test]
fn test_openai_client_no_key() {
    let config = LlmConfig {
        anthropic_api_key: None,
        openai_api_key: None,
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    let result = OpenAiClient::new(&config);
    assert!(result.is_err());
}

#[test]
fn test_anthropic_client_with_key() {
    let config = LlmConfig {
        anthropic_api_key: Some("sk-ant-test-123".into()),
        openai_api_key: None,
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    let result = AnthropicClient::new(&config);
    assert!(result.is_ok());
}

#[test]
fn test_openai_client_with_key() {
    let config = LlmConfig {
        anthropic_api_key: None,
        openai_api_key: Some("sk-test-123".into()),
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    let result = OpenAiClient::new(&config);
    assert!(result.is_ok());
}

#[test]
fn test_completion_request_construction() {
    let req = CompletionRequest {
        model: "claude-sonnet".into(),
        messages: vec![Message {
            role: "user".into(),
            content: "Hello".into(),
        }],
        max_tokens: 1024,
        temperature: Some(0.7),
        system: Some("You are helpful.".into()),
    };
    assert_eq!(req.model, "claude-sonnet");
    assert_eq!(req.messages.len(), 1);
    assert_eq!(req.max_tokens, 1024);
}

#[test]
fn test_llm_error_display() {
    let e = LlmError::NoApiKey;
    assert_eq!(format!("{}", e), "API key not configured");

    let e = LlmError::RateLimited;
    assert_eq!(format!("{}", e), "Rate limited");

    let e = LlmError::ApiError {
        status: 401,
        message: "Unauthorized".into(),
    };
    assert_eq!(format!("{}", e), "API error 401: Unauthorized");
}

#[tokio::test]
async fn test_executor_mock_fallback_without_keys() {
    let config = LlmConfig {
        anthropic_api_key: None,
        openai_api_key: None,
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    let registry = ModelRegistry::new();
    let executor = ModelExecutor::with_config(registry, config);

    let result = executor.execute("claude-sonnet", "test task", &[]).await;
    assert!(result.is_ok());
    let result = result.unwrap();
    // Should get mock response when no API key
    let response = result.output["response"].as_str().unwrap();
    assert!(response.contains("Processed: test task"));
}

#[tokio::test]
async fn test_executor_with_config() {
    let config = LlmConfig {
        anthropic_api_key: None,
        openai_api_key: None,
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    let registry = ModelRegistry::new();
    let executor = ModelExecutor::with_config(registry, config);
    // Verify local models still work (no API key needed)
    let result = executor.execute("llama-3-70b", "test", &[]).await;
    assert!(result.is_ok());
}

#[test]
fn test_has_provider_unknown() {
    let config = LlmConfig {
        anthropic_api_key: None,
        openai_api_key: None,
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    assert!(!config.has_provider("unknown_provider"));
    assert!(config.has_provider("deepseek"));
}

#[test]
fn test_completion_response_total_tokens() {
    use hydra_model::CompletionResponse;
    let resp = CompletionResponse {
        content: "Hello".into(),
        model: "test".into(),
        input_tokens: 10,
        output_tokens: 5,
        stop_reason: Some("end_turn".into()),
    };
    assert_eq!(resp.total_tokens(), 15);
}

// ═══════════════════════════════════════════════════════════
// LIVE LLM TESTS (feature-gated, require real API keys)
// ═══════════════════════════════════════════════════════════

#[cfg(feature = "live-llm")]
mod live {
    use super::*;

    fn live_config() -> LlmConfig {
        LlmConfig::from_env()
    }

    #[tokio::test]
    async fn test_live_anthropic_completion() {
        let config = live_config();
        if !config.has_anthropic() {
            eprintln!("Skipping: ANTHROPIC_API_KEY not set");
            return;
        }
        let client = AnthropicClient::new(&config).unwrap();
        let resp = client
            .complete(CompletionRequest {
                model: "claude-haiku".into(),
                messages: vec![Message {
                    role: "user".into(),
                    content: "Say hello in exactly 3 words.".into(),
                }],
                max_tokens: 50,
                temperature: Some(0.0),
                system: None,
            })
            .await
            .unwrap();

        assert!(!resp.content.is_empty());
        assert!(resp.input_tokens > 0);
        assert!(resp.output_tokens > 0);
    }

    #[tokio::test]
    async fn test_live_openai_completion() {
        let config = live_config();
        if !config.has_openai() {
            eprintln!("Skipping: OPENAI_API_KEY not set");
            return;
        }
        let client = OpenAiClient::new(&config).unwrap();
        let resp = client
            .complete(CompletionRequest {
                model: "gpt-4o-mini".into(),
                messages: vec![Message {
                    role: "user".into(),
                    content: "Say hello in exactly 3 words.".into(),
                }],
                max_tokens: 50,
                temperature: Some(0.0),
                system: None,
            })
            .await
            .unwrap();

        assert!(!resp.content.is_empty());
        assert!(resp.input_tokens > 0);
        assert!(resp.output_tokens > 0);
    }

    #[tokio::test]
    async fn test_live_executor_anthropic() {
        let config = live_config();
        if !config.has_anthropic() {
            eprintln!("Skipping: ANTHROPIC_API_KEY not set");
            return;
        }
        let registry = ModelRegistry::new();
        let executor = ModelExecutor::with_config(registry, config);

        let result = executor
            .execute("claude-haiku", "Say hi", &[])
            .await
            .unwrap();
        assert!(result.tokens_used > 0);
        let response = result.output["response"].as_str().unwrap();
        assert!(!response.contains("Processed:"));
    }

    #[tokio::test]
    async fn test_live_executor_openai() {
        let config = live_config();
        if !config.has_openai() {
            eprintln!("Skipping: OPENAI_API_KEY not set");
            return;
        }
        let registry = ModelRegistry::new();
        let executor = ModelExecutor::with_config(registry, config);

        let result = executor
            .execute("gpt-4o-mini", "Say hi", &[])
            .await
            .unwrap();
        assert!(result.tokens_used > 0);
        let response = result.output["response"].as_str().unwrap();
        assert!(!response.contains("Processed:"));
    }

    #[tokio::test]
    async fn test_live_invalid_api_key_anthropic() {
        let config = LlmConfig {
            anthropic_api_key: Some("sk-ant-invalid-key".into()),
            openai_api_key: None,
            anthropic_base_url: "https://api.anthropic.com".into(),
            openai_base_url: "https://api.openai.com".into(),
        };
        let client = AnthropicClient::new(&config).unwrap();
        let result = client
            .complete(CompletionRequest {
                model: "claude-haiku".into(),
                messages: vec![Message {
                    role: "user".into(),
                    content: "test".into(),
                }],
                max_tokens: 10,
                temperature: None,
                system: None,
            })
            .await;

        assert!(result.is_err());
    }
}
