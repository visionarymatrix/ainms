use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::error::{MlError, MlResult};
use crate::provider::LlmProvider;
use crate::tool_search::{self, ToolEntry, ToolRegistry};
use crate::types::{
    AgentResponse, ChatGenerateOptions, ChatMessage, ImageInput, ToolDefinition, ToolSelectionMode,
};

pub type ToolHandlerFn = Arc<dyn Fn(serde_json::Value) -> MlResult<String> + Send + Sync>;

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
    tool_selection: ToolSelectionMode,
    tool_registry: Arc<Mutex<ToolRegistry>>,
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
            tool_selection: ToolSelectionMode::All,
            tool_registry: Arc::new(Mutex::new(ToolRegistry::new())),
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

    /// Enable Tool Search mode with a maximum number of results per query.
    ///
    /// In this mode, the agent starts with only `search_tools` (and optionally
    /// `list_available_tools`) plus any always-visible tools. When the LLM
    /// calls `search_tools`, matching tool definitions are dynamically injected
    /// into the next LLM call.
    pub fn with_tool_search(mut self, registry: ToolRegistry, max_results: usize) -> Self {
        self.tool_selection = ToolSelectionMode::Search { max_results };
        self.tool_registry = Arc::new(Mutex::new(registry));
        self
    }

    /// Set the tool selection mode directly.
    pub fn with_tool_selection_mode(mut self, mode: ToolSelectionMode) -> Self {
        self.tool_selection = mode;
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

    /// Register a tool in both the handler list AND the search registry.
    ///
    /// Use this when in Search mode so the tool is both executable and
    /// discoverable via `search_tools`.
    pub async fn register_searchable_tool(
        &self,
        entry: ToolEntry,
        handler: impl Fn(serde_json::Value) -> MlResult<String> + Send + Sync + 'static,
    ) {
        let definition = entry.to_tool_definition();
        self.tools.lock().await.push(ToolHandler {
            definition: definition.clone(),
            handler: Arc::new(handler),
        });
        self.tool_registry.lock().await.register(entry);
    }

    pub async fn history(&self) -> Vec<ChatMessage> {
        self.history.lock().await.clone()
    }

    pub async fn clear_history(&self) {
        self.history.lock().await.clear();
    }

    pub async fn run(&self, user_message: impl Into<String>) -> MlResult<String> {
        self.run_with_content(ChatMessage::user(user_message)).await
    }

    pub async fn run_multimodal(
        &self,
        text: impl Into<String>,
        images: Vec<ImageInput>,
    ) -> MlResult<String> {
        self.run_with_content(ChatMessage::user_multimodal(text, images))
            .await
    }

    pub async fn run_with_content(&self, user_message: ChatMessage) -> MlResult<String> {
        match &self.tool_selection {
            ToolSelectionMode::All => self.run_all_tools(user_message).await,
            ToolSelectionMode::Search { max_results } => {
                self.run_search_tools(user_message, *max_results).await
            }
        }
    }

    async fn run_all_tools(&self, user_message: ChatMessage) -> MlResult<String> {
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
        let mut tools_executed_count: usize = 0;

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

            if !opts.tools.is_empty() && tools_executed_count == 0 {
                opts.tool_choice = "required".to_string();
                opts.temperature = opts.temperature.min(0.4);
            }

            debug!(iteration, "Calling chat_generate (all tools mode)");

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
                        tools_executed_count += 1;
                        match handler_map.get(call.name.as_str()) {
                            Some(handler) => {
                                debug!(tool = call.name, "Executing tool");
                                match handler(call.arguments.clone()) {
                                    Ok(result) => {
                                        debug!(tool = call.name, result_len = result.len(), "Tool succeeded");
                                        let msg = ChatMessage::tool_result(&call.id, &result);
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

    /// Run the agent in Tool Search mode.
    ///
    /// In this mode:
    /// 1. The agent starts with only `search_tools` (+ `list_available_tools`)
    ///    and any always-visible tools in the LLM context.
    /// 2. When the LLM calls `search_tools`, matching tool definitions are
    ///    found and injected into subsequent calls.
    /// 3. Discovered tools remain active for the rest of the conversation.
    async fn run_search_tools(
        &self,
        user_message: ChatMessage,
        max_results: usize,
    ) -> MlResult<String> {
        let mut messages = Vec::new();

        let mut registry = self.tool_registry.lock().await;
        let _ = max_results;

        let catalog = registry.catalog_summary(4000);

        let tool_forcing = "\
CRITICAL RULES - YOU MUST CALL TOOLS. NEVER describe what you would do. NEVER narrate. \
NEVER guess or fabricate tool results. When you need information, CALL the tool function. \
When you need a tool you don't see, call search_tools first. \
After search_tools returns results, you MUST call the discovered tool immediately — do not describe it, call it.\n\n\
Example - CORRECT: Call search_tools({\"query\": \"idle time\"}), then call get_idle_time({})\n\
Example - WRONG: \"I will use the get_idle_time tool to check...\" — this is narration, NOT a tool call.\n\
Example - WRONG: \"The idle time is 39 seconds\" — this is fabrication, NOT a tool result.";

        let search_system_prompt = if let Some(ref sys) = self.system_prompt {
            format!(
                "{}\n\n{}\n\n## Tool Catalog\n{}\n",
                sys, tool_forcing, catalog
            )
        } else {
            format!(
                "You are an AI assistant with tool access.\n\n{}\n\n## Tool Catalog\n{}\n",
                tool_forcing, catalog
            )
        };

        messages.push(ChatMessage::system(&search_system_prompt));
        {
            let history = self.history.lock().await;
            messages.extend(history.clone());
        }

        messages.push(user_message.clone());
        self.history.lock().await.push(user_message);

        let mut discovered_tools: Vec<ToolDefinition> = Vec::new();
        let mut discovered_handlers: HashMap<String, ToolHandlerFn> = HashMap::new();
        let mut tools_executed_count: usize = 0;

        let always_visible_tools: Vec<ToolDefinition> = {
            registry.always_visible_tools()
                .iter()
                .map(|e| e.to_tool_definition())
                .collect()
        };

        let search_tool_def = tool_search::search_tools_definition();
        let list_tool_def = tool_search::list_tools_definition();

        let mut iteration = 0;

        loop {
            if iteration >= self.max_iterations {
                warn!(iteration, "Agent hit max iteration limit (search mode)");
                return Err(MlError::MaxIterationsReached);
            }
            iteration += 1;

            let mut active_tools = Vec::new();
            active_tools.push(search_tool_def.clone());
            active_tools.push(list_tool_def.clone());
            active_tools.extend(always_visible_tools.clone());
            active_tools.extend(discovered_tools.clone());

            let mut opts = self.options.clone();
            opts.tools = active_tools;

            // Force tool calling for one turn right after tools are discovered.
            // After tools have been executed, switch back to "auto" so the
            // model can produce a text summary instead of being forced to
            // call more tools indefinitely (which causes context overflow).
            if !discovered_tools.is_empty() && tools_executed_count == 0 {
                opts.tool_choice = "required".to_string();
                opts.temperature = opts.temperature.min(0.4);
            }

            debug!(iteration, n_tools = opts.tools.len(), "Calling chat_generate (search mode)");

            let response = self.provider.chat_generate(&messages, &opts).await?;

            match response {
                AgentResponse::Text(text) => {
                    // Small models often narrate instead of calling tools.
                    // If tools are discovered but not yet called, re-prompt
                    // the model to actually invoke them.
                    let has_uncalled_tools = !discovered_tools.is_empty();
                    let looks_like_narration = text.len() < 500 && (
                        text.contains("I will use") ||
                        text.contains("I need to") ||
                        text.contains("Let me") ||
                        text.contains("I'll use") ||
                        text.contains("I should call") ||
                        text.contains("I can use") ||
                        text.contains("I would call") ||
                        text.contains("Let me call") ||
                        text.contains("I will call") ||
                        text.contains("Please provide me") ||
                        text.contains("Here are the steps") ||
                        text.contains("Here is what I will do")
                    );

                    if has_uncalled_tools && looks_like_narration && iteration < self.max_iterations {
                        info!(iteration, "Detected narration instead of tool call, re-prompting");
                        self.history.lock().await.push(ChatMessage::assistant(&text));
                        messages.push(ChatMessage::assistant(&text));
                        let callable_names: Vec<&str> = discovered_tools.iter()
                            .map(|t| t.name.as_str())
                            .collect();
                        let force_msg = format!(
                            "You narrated instead of calling a tool. You MUST call one of these tools now: {}. \
                             Do NOT describe what you would do. Call the tool.",
                            callable_names.join(", ")
                        );
                        messages.push(ChatMessage::user(&force_msg));
                        self.history.lock().await.push(ChatMessage::user(&force_msg));
                        continue;
                    }

                    info!(iteration, text_len = text.len(), "Agent produced text response");
                    self.history.lock().await.push(ChatMessage::assistant(&text));
                    return Ok(text);
                }
                AgentResponse::ToolCalls(calls) => {
                    info!(iteration, n_calls = calls.len(), "Agent requests tool calls (search mode)");

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

                    for call in &calls {
                        match call.name.as_str() {
                            "search_tools" => {
                                debug!("Processing search_tools call");
                                let (result_text, matched_entries) =
                                    tool_search::handle_search_tools(&mut registry, &call.arguments);

                                info!(n_discovered = matched_entries.len(), "Tool search discovered tools");

                                for entry in &matched_entries {
                                    let def = entry.to_tool_definition();
                                    if !discovered_tools.iter().any(|t| t.name == def.name) {
                                        if let Some(handler) = tool_handlers
                                            .iter()
                                            .find(|t| t.definition.name == def.name)
                                        {
                                            discovered_handlers.insert(
                                                def.name.clone(),
                                                handler.handler.clone(),
                                            );
                                        }
                                        discovered_tools.push(def);
                                    }
                                }

                                let msg = ChatMessage::tool_result(&call.id, &result_text);
                                messages.push(msg.clone());
                                self.history.lock().await.push(msg);
                            }
                            "list_available_tools" => {
                                debug!("Processing list_available_tools call");
                                let result_text =
                                    tool_search::handle_list_tools(&registry, &call.arguments);
                                let msg = ChatMessage::tool_result(&call.id, &result_text);
                                messages.push(msg.clone());
                                self.history.lock().await.push(msg);
                            }
                            tool_name => {
                                let handler = discovered_handlers.get(tool_name)
                                    .or_else(|| {
                                        tool_handlers.iter()
                                            .find(|t| t.definition.name == tool_name)
                                            .map(|t| &t.handler)
                                    });

                                tools_executed_count += 1;

                                match handler {
                                    Some(handler) => {
                                        debug!(tool = call.name, "Executing discovered/direct tool");
                                        match handler(call.arguments.clone()) {
                                            Ok(result) => {
                                                debug!(tool = call.name, result_len = result.len(), "Tool succeeded");
                                                let msg = ChatMessage::tool_result(&call.id, &result);
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
                                        warn!(tool = call.name, "Tool not found (search mode)");
                                        let msg = ChatMessage::tool_result(
                                            &call.id,
                                            &format!(
                                                "Error: tool '{}' not found. Try calling search_tools to discover available tools.",
                                                call.name
                                            ),
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
    }
}