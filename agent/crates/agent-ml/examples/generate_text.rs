//! Example: Load a GGUF model and generate text using LlamaCppProvider.
//!
//! Usage (CPU only):
//!   cargo run -p agent-ml --features llama-cpp --example generate_text -- <path-to-model.gguf>
//!
//! Usage (CUDA GPU):
//!   cargo run -p agent-ml --features llama-cpp,cuda --example generate_text -- <path-to-model.gguf>
//!
//! You can download small GGUF models from https://huggingface.co/models?search=gguf
//! Recommended small model: TinyLlama-1.1B-Chat-v1.0-GGUF (Q4_K_M quantisation ~660MB)

use std::path::PathBuf;

use agent_ml::llama_cpp::LlamaCppProvider;
use agent_ml::provider::LlmProvider;
use agent_ml::types::GenerateOptions;

#[tokio::main]
async fn main() {
    // Initialise tracing so we see model loading logs.
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let model_path = args.get(1).expect("Usage: generate_text <path/to/model.gguf>");

    println!("=== agent-ml LlamaCpp text generation example ===\n");

    let config = agent_ml::llama_cpp::LlamaCppConfig {
        model_path: PathBuf::from(model_path),
        mmproj_path: None,
        n_ctx: 2048,
        n_threads: 4,
        n_gpu_layers: 99, // Offload all layers to GPU (set to 0 for CPU-only)
    };

    let provider = LlamaCppProvider::new(config);

    // Load the model
    println!("Loading model from: {}", args.get(1).unwrap());
    match provider.load_model(model_path).await {
        Ok(()) => println!("Model loaded successfully!\n"),
        Err(e) => {
            eprintln!("Failed to load model: {}", e);
            std::process::exit(1);
        }
    }

    // Define prompts to test
    let prompts = vec![
        "The capital of France is",
        "Write a haiku about programming:",
        "In one word, what is the opposite of 'hot'?",
    ];

    let options = GenerateOptions {
        max_tokens: 64,
        temperature: 0.7,
        top_k: 40,
        top_p: 0.95,
        repeat_penalty: 1.1,
        seed: Some(42),
        stop_strings: vec!["\n\n".to_string()],
    };

    for prompt in &prompts {
        println!("---");
        println!("Prompt: {}", prompt);
        match provider.generate(prompt, &options).await {
            Ok(response) => println!("Response: {}", response.trim()),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    println!("\n=== Generation complete ===");
}