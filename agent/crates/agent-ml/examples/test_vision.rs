//! Test: Does LFM2.5-VL actually accept image input?
//!
//! This test loads the model, loads the separate mmproj, and tries to
//! process an image to verify vision capability.

use std::path::PathBuf;
use std::sync::Arc;

use agent_ml::llama_cpp::{LlamaCppConfig, LlamaCppProvider};
use agent_ml::provider::LlmProvider;
use agent_ml::types::{ChatContent, ChatGenerateOptions, ChatMessage, ChatRole, ImageInput};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let model_path = "crates/agent-ml/models/LFM2.5-VL-450M-Q4_0.gguf";
    let mmproj_path = "crates/agent-ml/models/mmproj-LFM2.5-VL-450m-Q8_0.gguf";
    let screenshot_path = "screenshot_test.png";

    // Verify files exist
    for path in &[model_path, mmproj_path, screenshot_path] {
        if !PathBuf::from(path).exists() {
            eprintln!("File not found: {}", path);
            std::process::exit(1);
        }
    }

    println!("=== Step 1: Loading LFM2.5-VL model ===");
    let config = LlamaCppConfig {
        model_path: PathBuf::from(model_path),
        mmproj_path: Some(PathBuf::from(mmproj_path)),
        n_ctx: 4096,
        n_threads: 4,
        n_gpu_layers: 0,
    };

    let provider = LlamaCppProvider::new(config);
    provider.load_model(model_path).await?;
    println!("Model loaded successfully!\n");

    println!("=== Step 2: Loading mmproj (separate file) ===");
    match provider.load_mmproj(mmproj_path).await {
        Ok(()) => println!("mmproj loaded successfully! Vision is ENABLED.\n"),
        Err(e) => {
            eprintln!("FAILED to load mmproj: {:?}", e);
            eprintln!("Vision input will NOT work. Falling back to text-only.\n");
        }
    }

    println!("=== Step 3: Testing vision input with screenshot ===");
    let messages = vec![
        ChatMessage {
            role: ChatRole::System,
            content: ChatContent::Text("You are a helpful assistant that describes screenshots.".to_string()),
        },
        ChatMessage {
            role: ChatRole::User,
            content: ChatContent::Multimodal {
                text: "Describe what you see in this screenshot in 1-2 sentences.".to_string(),
                images: vec![ImageInput::Path(screenshot_path.to_string())],
            },
        },
    ];

    let options = ChatGenerateOptions {
        max_tokens: 256,
        temperature: 0.7,
        ..Default::default()
    };

    match provider.chat_generate(&messages, &options).await {
        Ok(response) => {
            println!("Vision response: {:?}\n", response);
            println!("SUCCESS: Vision input works with LFM2.5-VL!");
        }
        Err(e) => {
            eprintln!("Vision test FAILED: {:?}", e);
            eprintln!("\nFalling back to text-only test...");

            // Try text-only as fallback
            let text_messages = vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: ChatContent::Text("You are a helpful assistant.".to_string()),
                },
                ChatMessage {
                    role: ChatRole::User,
                    content: ChatContent::Text("What is 2+2?".to_string()),
                },
            ];
            let text_response = provider.chat_generate(&text_messages, &options).await?;
            println!("Text-only response: {:?}", text_response);
        }
    }

    println!("\n=== Step 4: Testing with model path as mmproj (the current code approach) ===");
    // This is what activity_buffer.rs currently does - pass model path as mmproj
    let provider2 = LlamaCppProvider::new(LlamaCppConfig {
        model_path: PathBuf::from(model_path),
        mmproj_path: None,
        n_ctx: 4096,
        n_threads: 4,
        n_gpu_layers: 0,
    });
    provider2.load_model(model_path).await?;
    match provider2.load_mmproj(model_path).await {
        Ok(()) => println!("Surprisingly, model-path-as-mmproj WORKED. The model GGUF may contain embedded mmproj."),
        Err(e) => println!("As expected, model-path-as-mmproj FAILED: {:?}", e),
    }

    Ok(())
}