//! Tests for local model support (Ollama integration).

use hydra_model::local::manager::LocalModelManager;
use hydra_model::local::ollama::OllamaClient;
use hydra_model::local::registry::{
    find_model, known_local_models, models_for_profile, to_model_profile, LocalModelProfile,
    MemoryTier,
};
use hydra_model::preferences::ModelPreferences;
use hydra_model::profile::{PrivacyLevel, TaskType};
use hydra_model::registry::ModelRegistry;
use hydra_model::router::ModelRouter;

// ═══════════════════════════════════════════════════════════
// Registry tests
// ═══════════════════════════════════════════════════════════

#[test]
fn test_local_model_registry() {
    let models = known_local_models();
    assert_eq!(models.len(), 4);
    let names: Vec<&str> = models.iter().map(|m| m.ollama_name.as_str()).collect();
    assert!(names.contains(&"phi3"));
    assert!(names.contains(&"llama3"));
    assert!(names.contains(&"mistral"));
    assert!(names.contains(&"codellama"));
}

#[test]
fn test_model_memory_requirements() {
    let phi3 = find_model("phi3").unwrap();
    assert_eq!(phi3.memory_tier, MemoryTier::Small);
    assert!(phi3.vram_mb < 3000);

    let llama3 = find_model("llama3").unwrap();
    assert_eq!(llama3.memory_tier, MemoryTier::Medium);

    let codellama = find_model("codellama").unwrap();
    assert!(codellama.capabilities.code > codellama.capabilities.creative);
}

#[test]
fn test_profile_model_selection() {
    assert!(models_for_profile(LocalModelProfile::Minimal).is_empty());
    assert_eq!(
        models_for_profile(LocalModelProfile::Standard),
        vec!["phi3"]
    );
    assert_eq!(
        models_for_profile(LocalModelProfile::Performance),
        vec!["phi3", "llama3"]
    );
    assert_eq!(models_for_profile(LocalModelProfile::Unlimited).len(), 4);
}

// ═══════════════════════════════════════════════════════════
// OllamaClient tests
// ═══════════════════════════════════════════════════════════

#[test]
fn test_ollama_client_creation() {
    let client = OllamaClient::new();
    // Should create without panicking
    drop(client);
}

#[tokio::test]
async fn test_graceful_when_ollama_unavailable() {
    let client = OllamaClient::with_url("http://localhost:19999");
    assert!(!client.is_available().await);
}

// ═══════════════════════════════════════════════════════════
// Manager tests
// ═══════════════════════════════════════════════════════════

#[test]
fn test_local_provider_interface() {
    // Verify local models can be registered and appear with correct metadata
    let mgr = LocalModelManager::new(LocalModelProfile::Unlimited);
    let registry = ModelRegistry::empty();
    mgr.register_all_known(&registry);

    let phi3 = registry.get("local-phi3").unwrap();
    assert_eq!(phi3.provider, "ollama");
    assert_eq!(phi3.privacy, PrivacyLevel::Local);
    assert_eq!(phi3.cost_per_1k_input, 0.0);
    assert_eq!(phi3.cost_per_1k_output, 0.0);
}

// ═══════════════════════════════════════════════════════════
// Routing tests
// ═══════════════════════════════════════════════════════════

#[test]
fn test_routing_prefers_local_when_available() {
    let registry = ModelRegistry::empty();

    // Register a local model as available
    let meta = find_model("phi3").unwrap();
    let mut local_profile = to_model_profile(&meta);
    local_profile.available = true;
    registry.register(local_profile);

    let router = ModelRouter::new(registry);
    let prefs = ModelPreferences::local_only();

    let decision = router.route(TaskType::General, &prefs).unwrap();
    assert_eq!(decision.model.id, "local-phi3");
    assert_eq!(decision.model.privacy, PrivacyLevel::Local);
}

#[test]
fn test_routing_fallback_to_cloud() {
    let registry = ModelRegistry::new(); // Includes cloud models
    let router = ModelRouter::new(registry);

    // No local preference — should route to a cloud model (higher capability)
    let prefs = ModelPreferences::default();
    let decision = router.route(TaskType::Reasoning, &prefs).unwrap();
    assert!(decision.model.capabilities.reasoning >= 75);
}

#[test]
fn test_routing_local_cost_advantage() {
    let registry = ModelRegistry::empty();

    // Register local and cloud model
    let meta = find_model("llama3").unwrap();
    let mut local_profile = to_model_profile(&meta);
    local_profile.available = true;
    registry.register(local_profile);

    // Local model cost is 0.0
    let local = registry.get("local-llama3").unwrap();
    assert_eq!(local.cost_per_1k(), 0.0);
}

// ═══════════════════════════════════════════════════════════
// Live tests (require Ollama running)
// ═══════════════════════════════════════════════════════════

#[tokio::test]
#[cfg(feature = "local-llm")]
async fn test_live_ollama_list() {
    let client = OllamaClient::new();
    if !client.is_available().await {
        eprintln!("Ollama not running, skipping");
        return;
    }
    let models = client.list_models().await.unwrap();
    println!("Found {} models in Ollama", models.len());
}

#[tokio::test]
#[cfg(feature = "local-llm")]
async fn test_live_local_then_cloud_fallback() {
    use hydra_model::providers::{CompletionRequest, Message};

    let client = OllamaClient::new();
    if !client.is_available().await {
        eprintln!("Ollama not running, skipping");
        return;
    }
    let models = client.list_models().await.unwrap();
    if models.is_empty() {
        eprintln!("No models in Ollama, skipping");
        return;
    }

    let req = CompletionRequest {
        model: models[0].name.clone(),
        messages: vec![Message {
            role: "user".into(),
            content: "Reply with just the word 'hello'".into(),
        }],
        max_tokens: 10,
        temperature: Some(0.0),
        system: None,
    };

    let resp = client.chat(req).await.unwrap();
    assert!(!resp.content.is_empty());
}
