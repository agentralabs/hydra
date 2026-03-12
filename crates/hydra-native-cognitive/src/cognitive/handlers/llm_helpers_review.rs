//! LLM review utilities — adaptive tokens, self-review, clarification questions.

use hydra_native_state::utils::safe_truncate;

/// Phase 2, X3: Adaptive max_tokens based on task type and complexity.
/// Greeting → 150, simple Q&A → 500, code → 4000, architecture → 8000, full project → 16000.
pub(crate) fn adaptive_max_tokens(
    intent: &crate::cognitive::intent_router::ClassifiedIntent,
    complexity: &str,
    is_action: bool,
) -> u32 {
    use crate::cognitive::intent_router::IntentCategory;
    match intent.category {
        IntentCategory::Greeting | IntentCategory::Farewell | IntentCategory::Thanks => 150,
        IntentCategory::MemoryRecall | IntentCategory::MemoryStore => 300,
        IntentCategory::Settings | IntentCategory::AppControl => 500,
        IntentCategory::Question if complexity == "simple" => 1_000,
        IntentCategory::Question => 4_000,
        IntentCategory::CodeExplain => 2_000,
        IntentCategory::CodeFix | IntentCategory::CodeBuild if complexity == "complex" => 8_000,
        IntentCategory::CodeFix | IntentCategory::CodeBuild => 4_000,
        IntentCategory::PlanningQuery => 2_000,
        IntentCategory::SisterDiagnose | IntentCategory::SisterRepair => 1_000,
        IntentCategory::SelfRepair | IntentCategory::SelfScan | IntentCategory::SelfImplement => 4_000,
        IntentCategory::WebBrowse => 2_000,
        IntentCategory::Deploy => 4_000,
        IntentCategory::FileOperation => 2_000,
        IntentCategory::SystemControl => 500,
        _ if is_action && complexity == "complex" => 8_000,
        _ if is_action => 4_000,
        _ if complexity == "complex" => 8_000,
        _ => 4_000,
    }
}

/// Phase 2, X2: Self-review a response before delivery.
/// Returns Some(issue_description) if a problem is found, None if response is good.
/// Uses micro-LLM with 5-second timeout. Returns None on any failure (non-blocking).
pub(crate) async fn self_review_response(
    user_input: &str,
    response: &str,
    llm_config: &hydra_model::LlmConfig,
) -> Option<String> {
    let review_model = if llm_config.anthropic_api_key.is_some() {
        "claude-haiku-4-5-20251001"
    } else if llm_config.openai_api_key.is_some() {
        "gpt-4o-mini"
    } else {
        return None;
    };

    let request = hydra_model::CompletionRequest {
        model: review_model.to_string(),
        messages: vec![hydra_model::providers::Message {
            role: "user".into(),
            content: format!(
                "User asked: \"{}\"\n\nResponse to review:\n\"{}\"\n\nIs this response correct, complete, and helpful? Reply with ONLY one of:\n- YES\n- ISSUE: <one-sentence description of the problem>",
                safe_truncate(user_input, 200),
                safe_truncate(response, 800),
            ),
        }],
        max_tokens: 100,
        temperature: Some(0.2),
        system: Some("You are a response quality reviewer. Check if the response actually answers the user's question. Be concise.".to_string()),
    };

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        async {
            if llm_config.anthropic_api_key.is_some() {
                match hydra_model::providers::anthropic::AnthropicClient::new(llm_config) {
                    Ok(client) => client.complete(request).await.ok(),
                    Err(_) => None,
                }
            } else if llm_config.openai_api_key.is_some() {
                match hydra_model::providers::openai::OpenAiClient::new(llm_config) {
                    Ok(client) => client.complete(request).await.ok(),
                    Err(_) => None,
                }
            } else {
                None
            }
        }
    ).await;

    match result {
        Ok(Some(resp)) => {
            let content = resp.content.trim();
            if content.starts_with("ISSUE:") || content.starts_with("Issue:") {
                Some(content[6..].trim().to_string())
            } else {
                None // YES or anything else = pass
            }
        }
        _ => None, // Timeout or error = pass (non-blocking)
    }
}

/// Phase 2, X1: Generate a clarifying question when Hydra is uncertain.
/// Uses micro-LLM (Haiku) with tight token budget and timeout.
pub(crate) async fn generate_clarification_question(
    user_input: &str,
    llm_config: &hydra_model::LlmConfig,
    _active_model: &str,
) -> String {
    let fallback = format!("I'm not sure I understand what you mean by \"{}\". Could you rephrase or give me more context?", safe_truncate(user_input, 80));

    let clarify_model = if llm_config.anthropic_api_key.is_some() {
        "claude-haiku-4-5-20251001"
    } else if llm_config.openai_api_key.is_some() {
        "gpt-4o-mini"
    } else {
        return fallback;
    };

    let request = hydra_model::CompletionRequest {
        model: clarify_model.to_string(),
        messages: vec![hydra_model::providers::Message {
            role: "user".into(),
            content: format!("The user said: \"{}\"\n\nAsk ONE specific clarifying question to understand what they need. Be concise and friendly.", user_input),
        }],
        max_tokens: 100,
        temperature: Some(0.7),
        system: Some("You are Hydra, an AI assistant. Ask exactly ONE short clarifying question. Do not guess or answer — just ask what you need to know.".to_string()),
    };

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        async {
            if llm_config.anthropic_api_key.is_some() {
                match hydra_model::providers::anthropic::AnthropicClient::new(llm_config) {
                    Ok(client) => client.complete(request).await.ok(),
                    Err(_) => None,
                }
            } else if llm_config.openai_api_key.is_some() {
                match hydra_model::providers::openai::OpenAiClient::new(llm_config) {
                    Ok(client) => client.complete(request).await.ok(),
                    Err(_) => None,
                }
            } else {
                None
            }
        }
    ).await;

    match result {
        Ok(Some(resp)) => resp.content.trim().to_string(),
        _ => fallback,
    }
}
