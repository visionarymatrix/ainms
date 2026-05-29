use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::error::{MlError, MlResult};
use crate::provider::LlmProvider;
use crate::types::{
    AgentResponse, ChatGenerateOptions, ChatMessage, ImageInput, ToolDefinition,
};

/// A function that can be invoked by the agent when the LLM requests a tool call.
pub type ToolHandlerFn = Arc<dyn Fn(serde_json::Value) -> MlResult<String> + Send + Sync>;

/// A registered tool with its handler function.
pub struct ToolHandler {
    pub definition: ToolDefinition,
    pub handler: ToolHandlerFn,
}

pub struct Agent<P: LlmProvider + ?Sized> {
    provider: Arc<P>,
    history: Arc<Mutex<Vec<ChatMessage>>>,
    tools: Arc<Mutex<Vec<ToolHandler>>>,
    system_prompt: Option<String>,
    max_iterations: usize,
    options: ChatGenerateOptions,
}

impl<P: LlmProvider + ?Sized + 'static> Agent<P> {
    pub fn new(provider: Arc<P>) -> Self {
        Self {
            provider,
            history: Arc::new(Mutex::new(Vec::new())),
            tools: Arc::new(Mutex::new(Vec::new())),
            system_prompt: None,
            max_iterations: 10,
            options: ChatGenerateOptions::default(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }

    pub fn with_options(mut self, options: ChatGenerateOptions) -> Self {
        self.options = options;
        self
    }

    pub async fn register_tool(
        &self,
        definition: ToolDefinition,
        handler: impl Fn(serde_json::Value) -> MlResult<String> + Send + Sync + 'static,
    ) {
        self.tools.lock().await.push(ToolHandler {
            definition,
            handler: Arc::new(handler),
        });
    }

    pub async fn register_simple_tool(
        &self,
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
        handler: impl Fn(serde_json::Value) -> MlResult<String> + Send + Sync + 'static,
    ) {
        let definition = ToolDefinition::new(name, description, parameters);
        self.register_tool(definition, handler).await;
    }

    pub async fn history(&self) -> Vec<ChatMessage> {
        self.history.lock().await.clone()
    }

    pub async fn clear_history(&self) {
        self.history.lock().await.clear();
    }

    /// Send a text-only user message and run the agent loop to completion.
    pub async fn run(&self, user_message: impl Into<String>) -> MlResult<String> {
        self.run_with_content(ChatMessage::user(user_message)).await
    }

    /// Send a multimodal user message (text + images) and run the agent loop.
    pub async fn run_multimodal(
        &self,
        text: impl Into<String>,
        images: Vec<ImageInput>,
    ) -> MlResult<String> {
        self.run_with_content(ChatMessage::user_multimodal(text, images))
            .await
    }

    /// Core agent loop: send a message, handle tool calls, repeat until
    /// the model responds with plain text or we hit the iteration limit.
    pub async fn run_with_content(&self, user_message: ChatMessage) -> MlResult<String> {
        let mut messages = Vec::new();

        if let Some(ref sys) = self.system_prompt {
            messages.push(ChatMessage::system(sys));
        }

        {
            let history = self.history.lock().await;
            messages.extend(history.clone());
        }

        messages.push(user_message.clone());

        self.history.lock().await.push(user_message);

        let mut iteration = 0;

        loop {
            if iteration >= self.max_iterations {
                warn!(iteration, "Agent hit max iteration limit");
                return Err(MlError::MaxIterationsReached);
            }
            iteration += 1;

            let tool_defs: Vec<ToolDefinition> = self
                .tools
                .lock()
                .await
                .iter()
                .map(|t| t.definition.clone())
                .collect();

            let mut opts = self.options.clone();
            opts.tools = tool_defs;

            debug!(iteration, "Calling chat_generate");

            let response = self.provider.chat_generate(&messages, &opts).await?;

            match response {
                AgentResponse::Text(text) => {
                    info!(iteration, text_len = text.len(), "Agent produced text response");
                    self.history.lock().await.push(ChatMessage::assistant(&text));
                    return Ok(text);
                }
                AgentResponse::ToolCalls(calls) => {
                    info!(iteration, n_calls = calls.len(), "Agent requests tool calls");

                    let assistant_content = calls
                        .iter()
                        .map(|c| format!("call {}({})", c.name, c.arguments))
                        .collect::<Vec<_>>()
                        .join("; ");

                    self.history
                        .lock()
                        .await
                        .push(ChatMessage::assistant(&assistant_content));

                    let tool_handlers = self.tools.lock().await;
                    let handler_map: HashMap<&str, &ToolHandlerFn> = tool_handlers
                        .iter()
                        .map(|t| (t.definition.name.as_str(), &t.handler))
                        .collect();

                    for call in &calls {
                        match handler_map.get(call.name.as_str()) {
                            Some(handler) => {
                                debug!(tool = call.name, "Executing tool");
                                match handler(call.arguments.clone()) {
                                    Ok(result) => {
                                        debug!(tool = call.name, result_len = result.len(), "Tool succeeded");
                                        let msg =
                                            ChatMessage::tool_result(&call.id, &result);
                                        messages.push(msg.clone());
                                        self.history.lock().await.push(msg);
                                    }
                                    Err(e) => {
                                        warn!(tool = call.name, error = %e, "Tool execution failed");
                                        let error_msg = format!("Error: {}", e);
                                        let msg = ChatMessage::tool_result(&call.id, &error_msg);
                                        messages.push(msg.clone());
                                        self.history.lock().await.push(msg);
                                    }
                                }
                            }
                            None => {
                                warn!(tool = call.name, "Tool not found");
                                let msg = ChatMessage::tool_result(
                                    &call.id,
                                    &format!("Error: tool '{}' not found", call.name),
                                );
                                messages.push(msg.clone());
                                self.history.lock().await.push(msg);
                            }
                        }
                    }
                }
            }
        }
    }
}