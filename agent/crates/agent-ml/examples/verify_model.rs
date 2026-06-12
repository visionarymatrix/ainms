use std::path::PathBuf;

use agent_ml::llama_cpp::{LlamaCppConfig, LlamaCppProvider};
use agent_ml::provider::LlmProvider;
use agent_ml::types::GenerateOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let model_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "crates/agent-ml/models/Qwen3.5-0.8B-Q4_K_S.gguf".to_string());

    if !PathBuf::from(&model_path).exists() {
        eprintln!("Model not found at: {}", model_path);
        eprintln!("Usage: cargo run --example verify_model --features llama-cpp -- <path-to-model.gguf>");
        std::process::exit(1);
    }

    println!("=== Loading model: {} ===", model_path);
    let config = LlamaCppConfig {
        model_path: PathBuf::from(&model_path),
        mmproj_path: None,
        n_ctx: 2048,
        n_threads: 4,
        n_gpu_layers: 0,
    };

    let provider = LlamaCppProvider::new(config);
    provider.load_model(&model_path).await?;
    println!("Model loaded successfully!\n");

    // Test 1: Simple text generation
    println!("=== Test 1: Simple text generation ===");
    let prompt = "What is the capital of France? Answer in one sentence.";
    println!("Prompt: {}", prompt);

    let options = GenerateOptions {
        max_tokens: 100,
        temperature: 0.7,
        ..Default::default()
    };

    let response = provider.generate(prompt, &options).await?;
    println!("Response: {}\n", response);

    // Test 2: Code generation
    println!("=== Test 2: Code generation ===");
    let code_prompt = "Write a Rust function to calculate factorial.";
    println!("Prompt: {}", code_prompt);

    let code_options = GenerateOptions {
        max_tokens: 200,
        temperature: 0.3,
        ..Default::default()
    };

    let code_response = provider.generate(code_prompt, &code_options).await?;
    println!("Response:\n{}\n", code_response);

    // Test 3: Chat-style prompt
    println!("=== Test 3: Chat-style prompt ===");
    let chat_prompt = "User: Hello! How are you?\nAssistant:";
    println!("Prompt: {}", chat_prompt);

    let chat_options = GenerateOptions {
        max_tokens: 50,
        temperature: 0.8,
        ..Default::default()
    };

    let chat_response = provider.generate(chat_prompt, &chat_options).await?;
    println!("Response: {}\n", chat_response);

    println!("=== All tests passed! Model is working correctly. ===");

    Ok(())
}