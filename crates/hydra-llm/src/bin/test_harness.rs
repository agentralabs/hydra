use hydra_llm::adapters;
use hydra_llm::config::LlmConfig;
use hydra_llm::{Message, Role};

#[tokio::main]
async fn main() {
    println!("=== hydra-llm test harness ===\n");

    let messages = vec![Message {
        role: Role::User,
        content: "Say hello in exactly 5 words.".into(),
    }];

    // Test Anthropic
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        println!("--- Anthropic ---");
        let config = LlmConfig::anthropic(key);
        let adapter = adapters::from_config(config);
        println!("  provider: {}", adapter.provider_name());
        println!("  model:    {}", adapter.model_name());
        match adapter.complete(&messages).await {
            Ok(resp) => {
                println!("  response: {:?}", resp.content);
                println!(
                    "  tokens:   in={:?} out={:?}",
                    resp.input_tokens, resp.output_tokens
                );
                println!("  STATUS:   OK\n");
            }
            Err(e) => println!("  STATUS:   FAILED — {e}\n"),
        }
    } else {
        println!("--- Anthropic: SKIPPED (no ANTHROPIC_API_KEY) ---\n");
    }

    // Test OpenAI
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        println!("--- OpenAI ---");
        let config = LlmConfig::openai(key);
        let adapter = adapters::from_config(config);
        println!("  provider: {}", adapter.provider_name());
        println!("  model:    {}", adapter.model_name());
        match adapter.complete(&messages).await {
            Ok(resp) => {
                println!("  response: {:?}", resp.content);
                println!(
                    "  tokens:   in={:?} out={:?}",
                    resp.input_tokens, resp.output_tokens
                );
                println!("  STATUS:   OK\n");
            }
            Err(e) => println!("  STATUS:   FAILED — {e}\n"),
        }
    } else {
        println!("--- OpenAI: SKIPPED (no OPENAI_API_KEY) ---\n");
    }

    // Test Ollama
    println!("--- Ollama ---");
    let mut config = LlmConfig::ollama();
    config.model = "llama3.2:1b".into();
    let adapter = adapters::from_config(config);
    println!("  provider: {}", adapter.provider_name());
    println!("  model:    {}", adapter.model_name());
    match adapter.complete(&messages).await {
        Ok(resp) => {
            println!("  response: {:?}", resp.content);
            println!(
                "  tokens:   in={:?} out={:?}",
                resp.input_tokens, resp.output_tokens
            );
            println!("  STATUS:   OK\n");
        }
        Err(e) => println!("  STATUS:   FAILED — {e}\n"),
    }

    println!("=== harness complete ===");
}
