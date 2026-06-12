//! Tool Search module — BM25-based dynamic tool discovery for AI agents.
//!
//! Instead of loading every tool definition into the LLM context upfront,
//! the agent receives a single `search_tools` meta-tool that searches a
//! ToolRegistry to find relevant tools on-demand. Only discovered tools
//! are injected into the next LLM call, saving context tokens and
//! improving selection accuracy.
//!
//! # Architecture
//!
//! 1. **ToolRegistry** — stores all registered tools with rich metadata
//! 2. **BM25 search** — ranks tools by relevance to a natural-language query
//! 3. **Tool Search Tool** — meta-tool that the LLM calls to discover tools
//! 4. **Agent integration** — Agent loop dynamically expands visible tools
//!
//! # Example
//!
//! ```ignore
//! use agent_ml::tool_search::ToolRegistry;
//! use agent_ml::agent::Agent;
//! use agent_ml::llama_cpp::{LlamaCppConfig, LlamaCppProvider};
//!
//! let registry = ToolRegistry::new();
//! registry.register("get_weather", "Get current weather for a city", json!({...}));
//! registry.register("search_files", "Search files by name pattern", json!({...}));
//!
//! let mut agent = Agent::new(provider)
//!     .with_tool_search(registry, 5)  // max 5 tools per search
//!     .with_system_prompt("You can search for tools using the search_tools function.");
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::types::ToolDefinition;

// ── Tool metadata ─────────────────────────────────────────────────────────

/// Rich metadata about a registered tool, used for search and discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEntry {
    /// Unique tool name (e.g., "get_weather").
    pub name: String,
    /// Human-readable description of what the tool does.
    pub description: String,
    /// JSON Schema of the tool's parameters (OpenAI format).
    pub parameters: serde_json::Value,
    /// Optional category for grouping (e.g., "weather", "files", "network").
    #[serde(default)]
    pub category: Option<String>,
    /// Optional keywords for improved search matching.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Whether this tool is always visible (never deferred behind search).
    #[serde(default)]
    pub always_visible: bool,
}

impl ToolEntry {
    /// Create a new tool entry.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
            category: None,
            keywords: Vec::new(),
            always_visible: false,
        }
    }

    /// Set the tool category.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set keywords for improved search.
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    /// Mark this tool as always visible (not deferred).
    pub fn always_visible(mut self) -> Self {
        self.always_visible = true;
        self
    }

    /// Convert to a ToolDefinition for use in LLM calls.
    pub fn to_tool_definition(&self) -> ToolDefinition {
        ToolDefinition::new(&self.name, &self.description, self.parameters.clone())
    }

    /// Full searchable text: name + description + category + keywords.
    fn searchable_text(&self) -> String {
        let mut parts = vec![self.name.clone(), self.description.clone()];
        if let Some(ref cat) = self.category {
            parts.push(cat.clone());
        }
        parts.extend(self.keywords.iter().cloned());
        parts.join(" ")
    }
}

// ── Search result ──────────────────────────────────────────────────────────

/// A tool search result with relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchResult {
    /// The matched tool entry.
    pub tool: ToolEntry,
    /// BM25 relevance score (higher = more relevant).
    pub score: f64,
    /// Query terms that matched this tool.
    pub matched_terms: Vec<String>,
}

// ── ToolRegistry ───────────────────────────────────────────────────────────

/// Central registry for tool discovery via BM25 search.
///
/// Tools are indexed when registered. When the LLM calls `search_tools`,
/// the registry performs BM25 ranking to find the most relevant tools.
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    entries: Vec<ToolEntry>,
    bm25: BM25Index,
    /// Maximum number of tools to return per search query.
    max_results: usize,
    /// Whether the index needs rebuilding.
    dirty: bool,
}

impl ToolRegistry {
    /// Create a new empty registry with default max_results=5.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            bm25: BM25Index::new(),
            max_results: 5,
            dirty: true,
        }
    }

    /// Create a registry with a custom max_results.
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Register a tool entry in the registry.
    pub fn register(&mut self, entry: ToolEntry) {
        debug!(tool = %entry.name, "Registered tool in registry");
        self.entries.push(entry);
        self.dirty = true;
    }

    /// Convenience: register a tool with just name, description, and parameters.
    pub fn register_simple(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) {
        self.register(ToolEntry::new(name, description, parameters));
    }

    /// Register a tool with full metadata.
    pub fn register_with_metadata(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
        category: Option<String>,
        keywords: Vec<String>,
        always_visible: bool,
    ) {
        let mut entry = ToolEntry::new(name, description, parameters);
        entry.category = category;
        entry.keywords = keywords;
        entry.always_visible = always_visible;
        self.register(entry);
    }

    /// Get all registered tool entries.
    pub fn entries(&self) -> &[ToolEntry] {
        &self.entries
    }

    /// Get a tool entry by name.
    pub fn get(&self, name: &str) -> Option<&ToolEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Get all always-visible tools (not deferred behind search).
    pub fn always_visible_tools(&self) -> Vec<&ToolEntry> {
        self.entries.iter().filter(|e| e.always_visible).collect()
    }

    /// Search for tools matching a natural language query using BM25.
    pub fn search(&mut self, query: &str) -> Vec<ToolSearchResult> {
        if self.dirty {
            self.rebuild_index();
        }
        let results = self.bm25.search(query, &self.entries, self.max_results);
        if !results.is_empty() {
            info!(query = %query, n_results = results.len(), "Tool search completed");
        }
        results
    }

    /// Generate a lightweight catalog summary for the LLM system prompt.
    ///
    /// Lists all tools with brief descriptions so the model knows what
    /// capabilities exist without having full schemas in context.
    pub fn catalog_summary(&self, max_chars: usize) -> String {
        let mut lines = vec![format!("Available tool categories ({} tools total):", self.entries.len())];
        let mut total_len = lines[0].len();

        // Group by category
        let mut groups: HashMap<String, Vec<&ToolEntry>> = HashMap::new();
        for entry in &self.entries {
            let cat = entry.category.as_deref().unwrap_or("general");
            groups.entry(cat.to_string()).or_default().push(entry);
        }

        for (cat, entries) in groups.iter() {
            let header = format!("\n[{}]", cat.to_uppercase());
            let mut header_len = header.len();
            let mut block_lines = vec![header];

            for entry in entries {
                let desc = if entry.description.len() > 80 {
                    format!("{}...", &entry.description[..77])
                } else {
                    entry.description.clone()
                };
                let line = format!("  - {}: {}", entry.name, desc);
                header_len += line.len();
                block_lines.push(line);
            }

            if total_len + header_len < max_chars {
                lines.extend(block_lines);
                total_len += header_len;
            }
        }

        lines.join("\n")
    }

    /// Rebuild the BM25 index from current entries.
    fn rebuild_index(&mut self) {
        self.bm25 = BM25Index::build(&self.entries);
        self.dirty = false;
        debug!(n_tools = self.entries.len(), "BM25 tool index rebuilt");
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── BM25 Search Engine ─────────────────────────────────────────────────────

/// Pure Rust BM25 index for searching tool entries.
///
/// BM25 (Best Matching 25) is a TF-IDF variant that accounts for document
/// length normalization. It's the standard ranking function used by search
/// engines and requires zero external dependencies.
#[derive(Debug, Clone)]
struct BM25Index {
    /// Per-document term frequencies: doc_index -> term -> count.
    doc_tf: Vec<HashMap<String, usize>>,
    /// Per-document length (in tokens).
    doc_len: Vec<usize>,
    /// Document frequency: term -> number of docs containing it.
    df: HashMap<String, usize>,
    /// Average document length.
    avg_dl: f64,
    /// Number of documents.
    n_docs: usize,
    /// BM25 parameters.
    k1: f64,
    b: f64,
}

impl BM25Index {
    const K1: f64 = 1.5;
    const B: f64 = 0.75;

    fn new() -> Self {
        Self {
            doc_tf: Vec::new(),
            doc_len: Vec::new(),
            df: HashMap::new(),
            avg_dl: 0.0,
            n_docs: 0,
            k1: Self::K1,
            b: Self::B,
        }
    }

    /// Build the index from tool entries.
    fn build(entries: &[ToolEntry]) -> Self {
        let n_docs = entries.len();
        let mut doc_tf = Vec::with_capacity(n_docs);
        let mut doc_len = Vec::with_capacity(n_docs);
        let mut df: HashMap<String, usize> = HashMap::new();

        for entry in entries {
            let text = entry.searchable_text();
            let tokens = tokenize(&text);
            doc_len.push(tokens.len());

            let mut tf: HashMap<String, usize> = HashMap::new();
            let mut seen_terms = std::collections::HashSet::new();
            for token in &tokens {
                *tf.entry(token.clone()).or_insert(0) += 1;
                seen_terms.insert(token.clone());
            }
            for term in seen_terms {
                *df.entry(term).or_insert(0) += 1;
            }
            doc_tf.push(tf);
        }

        let total_len: usize = doc_len.iter().sum();
        let avg_dl = if n_docs > 0 {
            total_len as f64 / n_docs as f64
        } else {
            1.0
        };

        Self {
            doc_tf,
            doc_len,
            df,
            avg_dl,
            n_docs,
            k1: Self::K1,
            b: Self::B,
        }
    }

    /// Search for tools matching a query, returning top-k results.
    fn search(&self, query: &str, entries: &[ToolEntry], top_k: usize) -> Vec<ToolSearchResult> {
        let query_terms = tokenize(query);
        if query_terms.is_empty() || self.n_docs == 0 {
            return Vec::new();
        }

        let mut scored: Vec<(usize, f64, Vec<String>)> = Vec::new();

        for (idx, tf_map) in self.doc_tf.iter().enumerate() {
            let dl = self.doc_len[idx] as f64;
            let mut score = 0.0_f64;
            let mut matched = Vec::new();

            for term in &query_terms {
                let freq = *tf_map.get(term).unwrap_or(&0);
                if freq == 0 {
                    continue;
                }
                matched.push(term.clone());

                let df_val = *self.df.get(term).unwrap_or(&0) as f64;
                let idf = ((self.n_docs as f64 - df_val + 0.5) / (df_val + 0.5) + 1.0).ln();

                let tf_norm = (freq as f64 * (self.k1 + 1.0))
                    / (freq as f64
                        + self.k1 * (1.0 - self.b + self.b * (dl / self.avg_dl)));

                score += idf * tf_norm;
            }

            if score > 0.0 {
                scored.push((idx, score, matched));
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(top_k)
            .map(|(idx, score, matched)| ToolSearchResult {
                tool: entries[idx].clone(),
                score,
                matched_terms: matched,
            })
            .collect()
    }
}

/// Tokenize text for BM25: lowercase, split on whitespace/underscores/hyphens,
/// handle camelCase, filter short tokens.
fn tokenize(text: &str) -> Vec<String> {
    // Insert spaces before uppercase letters (camelCase splitting)
    let mut normalized = String::new();
    for (i, ch) in text.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            normalized.push(' ');
        }
        normalized.push(ch);
    }

    // Replace underscores, hyphens, slashes with spaces
    let normalized = normalized
        .replace('_', " ")
        .replace('-', " ")
        .replace('/', " ")
        .replace('\\', " ");

    normalized
        .to_lowercase()
        .split_whitespace()
        .filter(|t| t.len() > 1) // Filter single-character tokens
        .map(|t| t.to_string())
        .collect()
}

// ── Search Tool Definition ─────────────────────────────────────────────────

/// Build the `search_tools` tool definition for the LLM.
///
/// This is the meta-tool that the LLM calls to discover other tools.
pub fn search_tools_definition() -> ToolDefinition {
    ToolDefinition::new(
        "search_tools",
        "Search for available tools by describing what you need. \
         Returns a list of tool names and descriptions that match your query. \
         Use this when you need a capability you don't currently have access to.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "A natural language description of what you need the tool to do. \
                                     For example: 'get weather information', 'search for files', \
                                     'check system processes'"
                },
                "category": {
                    "type": "string",
                    "description": "Optional category to narrow the search: 'weather', 'system', \
                                     'network', 'file', 'compliance', etc."
                }
            },
            "required": ["query"]
        }),
    )
}

/// Build the `list_available_tools` tool definition for the LLM.
///
/// This is a simpler meta-tool that just lists all available tool names
/// and brief descriptions, without full parameter schemas.
pub fn list_tools_definition() -> ToolDefinition {
    ToolDefinition::new(
        "list_available_tools",
        "List all available tools with their names and brief descriptions. \
         Use this when you want to see what tools exist before searching for specific ones.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "description": "Optional category filter: 'weather', 'system', 'network', \
                                     'file', 'compliance', etc."
                }
            },
            "required": []
        }),
    )
}

/// Process a `search_tools` tool call from the LLM.
///
/// This function:
/// 1. Parses the search query from the tool call arguments
/// 2. Searches the registry for matching tools
/// 3. Returns a formatted string with the matching tool descriptions
/// 4. Returns the list of matching `ToolEntry`s for dynamic injection
pub fn handle_search_tools(
    registry: &mut ToolRegistry,
    arguments: &serde_json::Value,
) -> (String, Vec<ToolEntry>) {
    let query = arguments
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let category = arguments
        .get("category")
        .and_then(|v| v.as_str());

    if query.is_empty() {
        return ("Error: search query is required".to_string(), Vec::new());
    }

    let mut results = registry.search(query);

    // Filter by category if specified
    if let Some(cat) = category {
        results.retain(|r| {
            r.tool.category.as_deref() == Some(cat)
                || r.tool.keywords.iter().any(|k| k.eq_ignore_ascii_case(cat))
        });
    }

    if results.is_empty() {
        let msg = format!("No tools found matching '{}'. Try a different query or use list_available_tools to see all tools.", query);
        return (msg, Vec::new());
    }

    let mut descriptions = vec![format!("Found {} tool(s) matching '{}':\n", results.len(), query)];
    for (i, result) in results.iter().enumerate() {
        descriptions.push(format!(
            "{}. **{}** (score: {:.2}): {}",
            i + 1,
            result.tool.name,
            result.score,
            result.tool.description
        ));
        if let Some(ref cat) = result.tool.category {
            descriptions.push(format!("   Category: {}", cat));
        }
        if !result.matched_terms.is_empty() {
            descriptions.push(format!("   Matched: {}", result.matched_terms.join(", ")));
        }
    }
    descriptions.push(String::from(
        "\nYou can now use any of these tools. The tool definitions will be available in your next response.",
    ));

    let tool_entries: Vec<ToolEntry> = results.iter().map(|r| r.tool.clone()).collect();
    let response = descriptions.join("\n");

    (response, tool_entries)
}

/// Process a `list_available_tools` tool call from the LLM.
///
/// Returns a formatted list of all available tools and their descriptions,
/// without full parameter schemas.
pub fn handle_list_tools(
    registry: &ToolRegistry,
    arguments: &serde_json::Value,
) -> String {
    let category = arguments
        .get("category")
        .and_then(|v| v.as_str());

    let entries: Vec<&ToolEntry> = if let Some(cat) = category {
        registry.entries().iter().filter(|e| {
            e.category.as_deref() == Some(cat)
                || e.keywords.iter().any(|k| k.eq_ignore_ascii_case(cat))
        }).collect()
    } else {
        registry.entries().iter().collect()
    };

    if entries.is_empty() {
        return "No tools available.".to_string();
    }

    let mut lines = vec![format!("Available tools ({} total):", entries.len())];
    for entry in &entries {
        let cat_str = entry.category.as_deref().unwrap_or("general");
        lines.push(format!("  - {} [{}]: {}", entry.name, cat_str, entry.description));
    }
    lines.push(String::from(
        "\nUse search_tools(query='...') to find specific tools and get their parameter details.",
    ));

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("get_weather Forecast");
        assert!(tokens.contains(&"get".to_string()));
        assert!(tokens.contains(&"weather".to_string()));
        assert!(tokens.contains(&"forecast".to_string()));
    }

    #[test]
    fn test_tokenize_camel_case() {
        let tokens = tokenize("getWeatherForecast");
        assert!(tokens.contains(&"get".to_string()));
        assert!(tokens.contains(&"weather".to_string()));
        assert!(tokens.contains(&"forecast".to_string()));
    }

    #[test]
    fn test_tokenize_snake_case() {
        let tokens = tokenize("search_files_by_pattern");
        assert!(tokens.contains(&"search".to_string()));
        assert!(tokens.contains(&"files".to_string()));
        assert!(tokens.contains(&"pattern".to_string()));
    }

    #[test]
    fn test_registry_search_basic() {
        let mut registry = ToolRegistry::new();
        registry.register(ToolEntry::new(
            "get_weather",
            "Get current weather conditions for a city",
            serde_json::json!({"type": "object", "properties": {"city": {"type": "string"}}}),
        ).with_category("weather"));
        registry.register(ToolEntry::new(
            "search_files",
            "Search for files by name pattern",
            serde_json::json!({"type": "object", "properties": {"pattern": {"type": "string"}}}),
        ).with_category("file"));
        registry.register(ToolEntry::new(
            "get_time",
            "Get the current time in a timezone",
            serde_json::json!({"type": "object", "properties": {"timezone": {"type": "string"}}}),
        ).with_category("system"));

        let results = registry.search("weather forecast");
        assert!(!results.is_empty());
        assert_eq!(results[0].tool.name, "get_weather");
    }

    #[test]
    fn test_registry_search_category_filter() {
        let mut registry = ToolRegistry::new();
        registry.register(ToolEntry::new(
            "get_weather",
            "Get weather for a city",
            serde_json::json!({}),
        ).with_category("weather"));
        registry.register(ToolEntry::new(
            "search_files",
            "Search files by pattern",
            serde_json::json!({}),
        ).with_category("file"));

        let _results = registry.search("search");
        let (_response, tools) = handle_search_tools(&mut registry, &serde_json::json!({
            "query": "search",
            "category": "file"
        }));
        assert!(tools.len() >= 1);
        assert_eq!(tools[0].name, "search_files");
    }

    #[test]
    fn test_registry_catalog_summary() {
        let mut registry = ToolRegistry::new();
        registry.register(ToolEntry::new(
            "get_weather",
            "Get weather for a city",
            serde_json::json!({}),
        ).with_category("weather"));
        registry.register(ToolEntry::new(
            "search_files",
            "Search files by name",
            serde_json::json!({}),
        ).with_category("file"));

        let summary = registry.catalog_summary(4000);
        assert!(summary.contains("weather"));
        assert!(summary.contains("file"));
        assert!(summary.contains("2 tools"));
    }

    #[test]
    fn test_handle_search_tools_empty_query() {
        let mut registry = ToolRegistry::new();
        let (response, tools) = handle_search_tools(&mut registry, &serde_json::json!({"query": ""}));
        assert!(response.contains("required"));
        assert!(tools.is_empty());
    }

    #[test]
    fn test_handle_search_tools_no_results() {
        let mut registry = ToolRegistry::new();
        registry.register(ToolEntry::new("get_weather", "Get weather", serde_json::json!({})));
        let (response, tools) = handle_search_tools(&mut registry, &serde_json::json!({"query": "quantum physics"}));
        assert!(response.contains("No tools found"));
        assert!(tools.is_empty());
    }

    #[test]
    fn test_always_visible_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(ToolEntry::new("search_tools", "Search for tools", serde_json::json!({})).always_visible());
        registry.register(ToolEntry::new("get_weather", "Get weather", serde_json::json!({})));

        let always_visible = registry.always_visible_tools();
        assert_eq!(always_visible.len(), 1);
        assert_eq!(always_visible[0].name, "search_tools");
    }

    #[test]
    fn test_bm25_ranking_relevance() {
        let mut registry = ToolRegistry::new();
        registry.register(ToolEntry::new(
            "get_weather_forecast",
            "Get detailed weather forecast including temperature rain wind humidity",
            serde_json::json!({}),
        ).with_keywords(vec!["weather".into(), "forecast".into(), "temperature".into()]));
        registry.register(ToolEntry::new(
            "get_current_time",
            "Get the current local time",
            serde_json::json!({}),
        ).with_keywords(vec!["time".into(), "clock".into()]));
        registry.register(ToolEntry::new(
            "read_weather_alerts",
            "Read severe weather alerts and warnings for your area",
            serde_json::json!({}),
        ).with_category("weather"));

        let results = registry.search("weather conditions temperature");
        assert!(!results.is_empty());
        // get_weather_forecast should rank higher than read_weather_alerts
        // because it has more matching terms
        assert!(results[0].tool.name.contains("weather"));
    }

    #[test]
    fn full_integration_with_output() {
        use crate::agent::Agent;
        use crate::mock::MockProvider;
        use std::sync::Arc;

        println!("\n{}", "=".repeat(70));
        println!("  TOOL SEARCH - FULL INTEGRATION TEST");
        println!("{}", "=".repeat(70));

        // Step 1: Build Registry
        println!("\n--- Step 1: Building Tool Registry ---\n");
        let mut registry = ToolRegistry::new();

        registry.register(ToolEntry::new(
            "get_weather",
            "Get current weather conditions including temperature, humidity, and wind speed for a city",
            serde_json::json!({"type": "object", "properties": {"city": {"type": "string"}, "unit": {"type": "string"}}, "required": ["city"]}),
        ).with_category("weather").with_keywords(vec!["weather".into(), "forecast".into(), "temperature".into(), "rain".into()]));

        registry.register(ToolEntry::new(
            "get_weather_forecast",
            "Get a multi-day weather forecast with daily high and low temperatures",
            serde_json::json!({"type": "object", "properties": {"city": {"type": "string"}, "days": {"type": "integer"}}, "required": ["city"]}),
        ).with_category("weather").with_keywords(vec!["forecast".into(), "prediction".into(), "future".into(), "weekly".into()]));

        registry.register(ToolEntry::new(
            "search_files",
            "Search for files by name pattern or content in the workspace",
            serde_json::json!({"type": "object", "properties": {"pattern": {"type": "string"}, "path": {"type": "string"}}, "required": ["pattern"]}),
        ).with_category("file").with_keywords(vec!["file".into(), "search".into(), "find".into(), "glob".into()]));

        registry.register(ToolEntry::new(
            "read_file",
            "Read the contents of a file at a given path",
            serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}),
        ).with_category("file").with_keywords(vec!["read".into(), "file".into(), "contents".into()]));

        registry.register(ToolEntry::new(
            "get_running_processes",
            "Get a list of currently running processes with their names, PIDs, and CPU usage",
            serde_json::json!({"type": "object", "properties": {"filter": {"type": "string"}}, "required": []}),
        ).with_category("system").with_keywords(vec!["process".into(), "running".into(), "cpu".into(), "application".into()]));

        registry.register(ToolEntry::new(
            "get_network_connections",
            "Get active network connections with remote hostnames, protocols, and ports",
            serde_json::json!({"type": "object", "properties": {"protocol": {"type": "string"}}, "required": []}),
        ).with_category("network").with_keywords(vec!["network".into(), "connection".into(), "hostname".into(), "port".into(), "dns".into()]));

        registry.register(ToolEntry::new(
            "report_violation",
            "Report if the user is engaged in activities unrelated to their role",
            serde_json::json!({"type": "object", "properties": {"is_violating": {"type": "boolean"}, "reason": {"type": "string"}}, "required": ["is_violating", "reason"]}),
        ).with_category("compliance").with_keywords(vec!["violation".into(), "audit".into(), "compliance".into()]));

        println!("Registered {} tools:", registry.entries().len());
        for entry in registry.entries() {
            println!("  [{}] {} - {}", entry.category.as_deref().unwrap_or("general"), entry.name, entry.description.chars().take(55).collect::<String>());
        }

        // Step 2: BM25 Search
        println!("\n--- Step 2: Testing BM25 Search ---\n");
        let test_queries = vec![
            "weather forecast temperature",
            "find files in my project",
            "network connections DNS",
            "what is the user doing right now",
            "check system activity processes",
            "compliance audit violation",
        ];

        for query in &test_queries {
            let results = registry.search(query);
            println!("  Query: \"{}\"", query);
            for result in &results {
                println!("    {} (score: {:.2}, matched: {}) [{}]",
                    result.tool.name, result.score,
                    result.matched_terms.join(", "),
                    result.tool.category.as_deref().unwrap_or("general"));
            }
            assert!(!results.is_empty() || query.contains("right now"),
                "BM25 should find results for '{}'", query);
        }

        // Step 3: handle_search_tools
        println!("\n--- Step 3: Testing handle_search_tools ---\n");
        let (response, tools) = handle_search_tools(&mut registry, &serde_json::json!({"query": "weather conditions"}));
        println!("Search response:\n{}", response);
        println!("\nDiscovered {} tool(s):", tools.len());
        for t in &tools {
            println!("  - {} [{}]", t.name, t.category.as_deref().unwrap_or("general"));
        }
        assert!(!tools.is_empty(), "Should find weather tools");
        assert!(tools.iter().any(|t| t.name == "get_weather"), "Should find get_weather");
        assert!(tools.iter().any(|t| t.name == "get_weather_forecast"), "Should find get_weather_forecast");

        // Step 4: Category filter
        println!("\n--- Step 4: Testing category filter ---\n");
        let (cat_response, cat_tools) = handle_search_tools(&mut registry, &serde_json::json!({"query": "search find files", "category": "file"}));
        println!("File-category search:\n{}", cat_response);
        assert!(cat_tools.iter().all(|t| t.category.as_deref() == Some("file")),
            "All results should be in 'file' category");

        // Step 5: list_available_tools
        println!("\n--- Step 5: Testing list_available_tools ---\n");
        let list_response = handle_list_tools(&registry, &serde_json::json!({}));
        println!("{}", list_response);
        assert!(list_response.contains("7 total"), "Should list all 7 tools");

        // Step 6: catalog_summary
        println!("\n--- Step 6: Testing catalog_summary ---\n");
        let summary = registry.catalog_summary(2000);
        println!("{}", summary);
        assert!(summary.contains("weather"), "Summary should mention weather category");
        assert!(summary.contains("file"), "Summary should mention file category");

        // Step 7: search_tools_definition
        println!("\n--- Step 7: Testing search_tools tool definition ---\n");
        let search_def = search_tools_definition();
        println!("search_tools definition:");
        println!("  Name: {}", search_def.name);
        println!("  Description: {}...", search_def.description.chars().take(80).collect::<String>());
        println!("  Parameters: {}", serde_json::to_string_pretty(&search_def.parameters).unwrap());
        assert_eq!(search_def.name, "search_tools");
        assert!(search_def.parameters.is_object());

        let list_def = list_tools_definition();
        println!("\nlist_available_tools definition:");
        println!("  Name: {}", list_def.name);
        assert_eq!(list_def.name, "list_available_tools");

        // Step 8: Agent Search Mode with Mock
        println!("\n--- Step 8: Testing Agent with ToolSelectionMode::Search ---\n");
        let mut search_registry_2 = ToolRegistry::new();
        search_registry_2.register(ToolEntry::new(
            "mock_tool",
            "A mock tool for testing search mode agent integration",
            serde_json::json!({"type": "object", "properties": {"input": {"type": "string"}}, "required": ["input"]}),
        ).with_category("test"));

        let _search_agent = Agent::new(Arc::new(MockProvider::new()))
            .with_system_prompt("You are a helpful assistant with access to tool search.")
            .with_max_iterations(5)
            .with_tool_search(search_registry_2, 5);

        println!("Search-mode agent created with ToolSelectionMode::Search {{ max_results: 5 }}");
        println!("Meta-tools (search_tools + list_available_tools) will be auto-injected.");
        println!("OK: Agent creation succeeded");

        println!("\n{}", "=".repeat(70));
        println!("  ALL INTEGRATION TESTS PASSED");
        println!("{}", "=".repeat(70));
    }
}