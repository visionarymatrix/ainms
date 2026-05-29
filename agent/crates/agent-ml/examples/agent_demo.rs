use std::sync::Arc;

use agent_ml::agent::Agent;
use agent_ml::provider::LlmProvider;
use agent_ml::types::{ChatGenerateOptions, ImageInput};

#[cfg(feature = "llama-cpp")]
use agent_ml::llama_cpp::{LlamaCppConfig, LlamaCppProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let _model_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "model.gguf".to_string());

    let _mmproj_path = std::env::args().nth(2);

    #[cfg(feature = "llama-cpp")]
    let provider: Arc<dyn LlmProvider> = {
        let config = LlamaCppConfig {
            model_path: std::path::PathBuf::from(&model_path),
            n_ctx: 4096,
            n_threads: 8,
            n_gpu_layers: 99,
        };
        let provider = LlamaCppProvider::new(config);
        provider.load_model(&model_path).await?;

        if let Some(ref mmproj) = mmproj_path {
            provider.load_mmproj(mmproj).await?;
        }

        Arc::new(provider)
    };

    #[cfg(not(feature = "llama-cpp"))]
    let provider: Arc<dyn LlmProvider> = {
        let provider = agent_ml::mock::MockProvider::new()
            .with_chat_response(agent_ml::types::AgentResponse::Text(
                "I can see a screenshot of a terminal window with code.".to_string(),
            ))
            .with_chat_response(agent_ml::types::AgentResponse::ToolCalls(vec![
                agent_ml::types::ToolCall {
                    id: "call_1".to_string(),
                    name: "screenshot_analyze".to_string(),
                    arguments: serde_json::json!({
                        "analysis": "The user is writing Rust code in a terminal"
                    }),
                },
            ]))
            .with_chat_response(agent_ml::types::AgentResponse::Text(
                "I've analyzed the screenshot. You appear to be writing Rust code.".to_string(),
            ));
        Arc::new(provider)
    };

    let agent = Agent::new(provider)
        .with_system_prompt("You are a helpful AI assistant that can analyze images and use tools.")
        .with_max_iterations(5)
        .with_options(ChatGenerateOptions {
            max_tokens: 512,
            temperature: 0.7,
            ..Default::default()
        });

    agent
        .register_simple_tool(
            "screenshot_analyze",
            "Analyze a screenshot and describe what you see",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "analysis": {
                        "type": "string",
                        "description": "Detailed analysis of the screenshot"
                    }
                },
                "required": ["analysis"]
            }),
            |args| {
                let analysis = args
                    .get("analysis")
                    .and_then(|v| v.as_str())
                    .unwrap_or("no analysis provided");
                println!("[Tool] screenshot_analyze: {}", analysis);
                Ok(format!("Tool executed: {}", analysis))
            },
        )
        .await;

    agent
        .register_simple_tool(
            "read_file",
            "Read the contents of a file",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                },
                "required": ["path"]
            }),
            |args| {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                println!("[Tool] read_file: {}", path);
                Ok(format!("Contents of {}:", path))
            },
        )
        .await;

    println!("=== Text-only conversation ===\n");
    let response = agent.run("What tools do you have available?").await?;
    println!("Agent: {}\n", response);

    println!("=== Multimodal with tool calling ===\n");
    let images = vec![ImageInput::Path("screenshot.png".to_string())];
    let response = agent
        .run_multimodal("Analyze this screenshot", images)
        .await?;
    println!("Agent: {}\n", response);

    println!("=== Conversation history ===");
    for msg in agent.history().await {
        println!("  [{}] {}", msg.role.as_str(), msg.content.text());
    }

    Ok(())
}