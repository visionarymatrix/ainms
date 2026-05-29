use serde::{Deserialize, Serialize};

/// Options for text generation.
#[derive(Debug, Clone)]
pub struct GenerateOptions {
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: i32,
    pub repeat_penalty: f32,
    pub seed: Option<u64>,
    pub stop_strings: Vec<String>,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            max_tokens: 256,
            temperature: 0.8,
            top_p: 0.95,
            top_k: 40,
            repeat_penalty: 1.1,
            seed: Some(42),
            stop_strings: vec![],
        }
    }
}

impl GenerateOptions {
    pub fn deterministic() -> Self {
        Self {
            temperature: 0.1,
            top_p: 0.9,
            top_k: 10,
            seed: Some(42),
            ..Default::default()
        }
    }
}

// ── Chat message types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: ChatContent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

impl ChatRole {
    pub fn as_str(&self) -> &str {
        match self {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
            ChatRole::Tool => "tool",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatContent {
    Text(String),
    Multimodal { text: String, images: Vec<ImageInput> },
    ToolResult { tool_call_id: String, result: String },
}

impl ChatContent {
    pub fn text(&self) -> &str {
        match self {
            ChatContent::Text(t) => t,
            ChatContent::Multimodal { text, .. } => text,
            ChatContent::ToolResult { .. } => "",
        }
    }

    pub fn images(&self) -> &[ImageInput] {
        match self {
            ChatContent::Text(_) => &[],
            ChatContent::Multimodal { images, .. } => images,
            ChatContent::ToolResult { .. } => &[],
        }
    }

    pub fn is_multimodal(&self) -> bool {
        matches!(self, ChatContent::Multimodal { images, .. } if !images.is_empty())
    }

    pub fn tool_result(&self) -> Option<(&str, &str)> {
        match self {
            ChatContent::ToolResult { tool_call_id, result } => Some((tool_call_id, result)),
            _ => None,
        }
    }
}

impl From<String> for ChatContent {
    fn from(s: String) -> Self {
        ChatContent::Text(s)
    }
}

impl From<&str> for ChatContent {
    fn from(s: &str) -> Self {
        ChatContent::Text(s.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImageInput {
    Raw { nx: u32, ny: u32, data: Vec<u8> },
    Path(String),
    Base64(String),
}

impl ChatMessage {
    pub fn system(text: impl Into<String>) -> Self {
        Self { role: ChatRole::System, content: ChatContent::Text(text.into()) }
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self { role: ChatRole::User, content: ChatContent::Text(text.into()) }
    }

    pub fn user_multimodal(text: impl Into<String>, images: Vec<ImageInput>) -> Self {
        Self {
            role: ChatRole::User,
            content: ChatContent::Multimodal { text: text.into(), images },
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self { role: ChatRole::Assistant, content: ChatContent::Text(text.into()) }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, result: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Tool,
            content: ChatContent::ToolResult {
                tool_call_id: tool_call_id.into(),
                result: result.into(),
            },
        }
    }
}

// ── Tool definitions ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self { name: name.into(), description: description.into(), parameters }
    }

    pub fn to_openai_tool_json(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
}

// ── Agent response ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AgentResponse {
    Text(String),
    ToolCalls(Vec<ToolCall>),
}

// ── Chat generation options ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ChatGenerateOptions {
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: i32,
    pub repeat_penalty: f32,
    pub seed: Option<u64>,
    pub tools: Vec<ToolDefinition>,
    pub parallel_tool_calls: bool,
}

impl Default for ChatGenerateOptions {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            temperature: 0.7,
            top_p: 0.95,
            top_k: 40,
            repeat_penalty: 1.1,
            seed: Some(42),
            tools: vec![],
            parallel_tool_calls: false,
        }
    }
}