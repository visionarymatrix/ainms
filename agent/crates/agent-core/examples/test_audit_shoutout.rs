//! Standalone test: Run local AI audit with "shoutout for VS Code" prompt.
//!
//! This test exercises the same code path as `run_local_ai_audit()` but without
//! requiring backend enrollment. It:
//! 1. Captures a screenshot
//! 2. Loads LFM2.5-VL + mmproj
//! 3. Runs the audit prompt (with VS Code shoutout rule)
//! 4. Shows the alert dialog if the VLM calls show_alert
//!
//! Usage: cargo run --package agent-core --example test_audit_shoutout --features llama-cpp,mtmd

use std::path::PathBuf;
use std::sync::Arc;

use agent_ml::llama_cpp::{LlamaCppConfig, LlamaCppProvider};
use agent_ml::provider::LlmProvider;
use agent_ml::agent::Agent as MlAgent;
use agent_ml::types::{ImageInput, ChatContent, ChatGenerateOptions, ChatMessage, ChatRole};
use base64::Engine;

const SHOUTOUT_SYSTEM_PROMPT: &str = r#"You are a workplace compliance auditor AI. You analyze desktop screenshots or system activity to determine if an employee's current activity violates their assigned role.

## YOUR TOOLS
You have two tools:
1. report_violation — Call this when you determine the user IS violating work policy.
2. show_alert — Call this when you detect unproductive activity to warn the user via a desktop dialog, OR when you want to give positive reinforcement.

## RULES
- ALWAYS call report_violation with your final verdict (is_violating=true OR is_violating=false).
- If is_violating=true, ALSO call show_alert to notify the user on their desktop.
- If you see VS Code, Visual Studio, IntelliJ, PyCharm, or ANY code editor / IDE: call show_alert with title='Shoutout!' message='Great work! We see you coding — keep it up!' alert_type='notify' to give the user positive reinforcement.
- NEVER narrate your reasoning without calling a tool. ALWAYS use the tools.
- Productive = coding, terminal, IDE, technical docs, spreadsheets, email, project management.
- Unproductive = YouTube, Netflix, games, social media, shopping, personal messaging.
- Be objective: if you see a code editor or terminal, they are likely compliant.
- If you are unsure, mark is_violating=false with reason='Uncertain activity, needs review'."#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // ── 1. Capture a screenshot ─────────────────────────────────────────────
    println!("=== Step 1: Capturing screenshot ===");
    let commander = agent_screenshot::ScreenshotCommander::new();
    let screenshot_data = match commander.capture().await {
        Ok(data) => {
            println!("Screenshot captured: {} bytes", data.len());
            data
        }
        Err(e) => {
            eprintln!("Screenshot capture failed: {}", e);
            std::process::exit(1);
        }
    };

    // ── 2. Find model + mmproj ────────────────────────────────────────────
    let model_paths = [
        "crates/agent-ml/models/LFM2.5-VL-450M-Q4_0.gguf",
        "../agent-ml/models/LFM2.5-VL-450M-Q4_0.gguf",
        "models/LFM2.5-VL-450M-Q4_0.gguf",
    ];
    let model_path = model_paths.iter().find(|p| PathBuf::from(p).exists()).cloned();
    let model_path = match model_path {
        Some(p) => p.to_string(),
        None => {
            eprintln!("No model found. Checked: {:?}", model_paths);
            std::process::exit(1);
        }
    };

    let mmproj_paths = [
        "crates/agent-ml/models/mmproj-LFM2.5-VL-450m-Q8_0.gguf",
        "../agent-ml/models/mmproj-LFM2.5-VL-450m-Q8_0.gguf",
        "models/mmproj-LFM2.5-VL-450m-Q8_0.gguf",
    ];
    let mmproj_path = mmproj_paths.iter().find(|p| PathBuf::from(p).exists()).cloned();

    println!("\n=== Step 2: Loading model ===");
    println!("Model: {}", model_path);
    println!("mmproj: {:?}", mmproj_path);

    let config = LlamaCppConfig {
        model_path: PathBuf::from(&model_path),
        mmproj_path: mmproj_path.as_ref().map(PathBuf::from),
        n_ctx: 4096,
        n_threads: 4,
        n_gpu_layers: 0,
    };

    let provider = LlamaCppProvider::new(config);
    provider.load_model(&model_path).await?;
    println!("Model loaded!");

    let mut is_multimodal = false;
    if let Some(ref proj_path) = mmproj_path {
        if provider.load_mmproj(proj_path).await.is_ok() {
            is_multimodal = true;
            println!("mmproj loaded — vision is ENABLED");
        } else {
            println!("mmproj FAILED — falling back to text-only");
        }
    }

    // ── 3. Build the agent with shoutout prompt ─────────────────────────────
    println!("\n=== Step 3: Building agent with shoutout prompt ===");
    let agent = MlAgent::new(Arc::new(provider))
        .with_system_prompt(SHOUTOUT_SYSTEM_PROMPT)
        .with_max_iterations(5);

    // Register show_alert tool that actually shows a Windows dialog
    let audit_flag = Arc::new(tokio::sync::Mutex::new((false, String::new())));
    let audit_flag_clone = Arc::clone(&audit_flag);

    agent.register_simple_tool(
        "report_violation",
        "Report whether the user's current activity violates their assigned role.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "is_violating": { "type": "boolean", "description": "True if non-work, False if compliant" },
                "reason": { "type": "string", "description": "Detailed explanation" }
            },
            "required": ["is_violating", "reason"]
        }),
        move |args| {
            let is_violating = args.get("is_violating").and_then(|v| v.as_bool()).unwrap_or(false);
            let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let mut flag = audit_flag_clone.blocking_lock();
            *flag = (is_violating, reason.clone());
            Ok(format!("Violation report: violating={}, reason='{}'", is_violating, reason))
        }
    ).await;

    agent.register_simple_tool(
        "show_alert",
        "Show a desktop alert dialog to the user. Use for warnings OR positive reinforcement (shoutouts).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "message": { "type": "string" },
                "alert_type": { "type": "string", "enum": ["notify", "ask", "prompt"] }
            },
            "required": ["title", "message", "alert_type"]
        }),
        |args| {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("AINMS Alert");
            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("No details");
            let alert_type = args.get("alert_type").and_then(|v| v.as_str()).unwrap_or("notify");

            match alert_type {
                "ask" => {
                    match agent_core::dialog::ask(title, message) {
                        Ok(agent_core::dialog::DialogAnswer::Yes) => Ok(format!("User answered YES to '{}'", title)),
                        Ok(agent_core::dialog::DialogAnswer::No) => Ok(format!("User answered NO to '{}'", title)),
                        Err(e) => Ok(format!("Dialog error: {}", e)),
                    }
                }
                "prompt" => {
                    match agent_core::dialog::prompt(title, message) {
                        Ok(result) => match result.text {
                            Some(ref t) if !t.trim().is_empty() => Ok(format!("User responded: {}", t)),
                            _ => Ok(format!("User dismissed prompt '{}'", title)),
                        },
                        Err(e) => Ok(format!("Dialog error: {}", e)),
                    }
                }
                _ => {
                    match agent_core::dialog::notify(title, message) {
                        Ok(()) => Ok(format!("Alert shown: '{}' — {}", title, message)),
                        Err(e) => Ok(format!("Dialog error: {}", e)),
                    }
                }
            }
        }
    ).await;

    // ── 4. Run the audit ────────────────────────────────────────────────────
    println!("\n=== Step 4: Running audit ===");
    println!("(Please open VS Code if you want the shoutout)");

    let response = if is_multimodal {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&screenshot_data);
        let prompt = format!(
            "AUDIT this screenshot:\n\
             - Employee Role: Developer\n\
             - Role Work Description: Software development and coding\n\
             INSTRUCTIONS:\n\
             1. Look at the screenshot.\n\
             2. If you see VS Code, Visual Studio, IntelliJ, PyCharm, or ANY code editor / IDE, call show_alert with title='Shoutout!' message='Great work! We see you coding — keep it up!' alert_type='notify' FIRST, then call report_violation with is_violating=false.\n\
             3. If you see unproductive activity (YouTube, Netflix, games, social media), call report_violation with is_violating=true and show_alert to warn the user.\n\
             4. If unsure, call report_violation with is_violating=false."
        );
        agent.run_multimodal(prompt, vec![ImageInput::Base64(b64)]).await?
    } else {
        let active = agent_collectors::get_active_window();
        let active_str = match active {
            Some(w) => format!("Active: '{}' (Process: {})", w.title, w.process_name),
            None => "No active window".to_string(),
        };
        let prompt = format!(
            "AUDIT:\n\
             - Employee Role: Developer\n\
             - Role Work Description: Software development and coding\n\
             SYSTEM STATE: {}\n\
             INSTRUCTIONS:\n\
             1. If the active app is VS Code, Visual Studio, IntelliJ, PyCharm, or ANY code editor / IDE, call show_alert with title='Shoutout!' message='Great work! We see you coding — keep it up!' alert_type='notify' FIRST, then call report_violation with is_violating=false.\n\
             2. If the active app is unproductive, call report_violation with is_violating=true and show_alert to warn.\n\
             3. If unsure, call report_violation with is_violating=false.",
            active_str
        );
        agent.run(prompt).await?
    };

    println!("\n=== Agent response ===");
    println!("{}", response);

    let (is_violating, reason) = {
        let flag = audit_flag.lock().await;
        flag.clone()
    };

    println!("\n=== Audit result ===");
    println!("is_violating = {}", is_violating);
    println!("reason = {}", reason);

    if is_violating {
        println!("\nUser was flagged for violation.");
    } else {
        println!("\nUser is compliant (or shoutout given).");
    }

    Ok(())
}
