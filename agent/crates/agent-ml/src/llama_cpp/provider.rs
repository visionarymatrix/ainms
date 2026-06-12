//! LlamaCpp provider — concrete [`LlmProvider`] backed by `llama-cpp-2`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use tracing::{debug, info, warn};

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;

use crate::error::{MlError, MlResult};
use crate::provider::LlmProvider;
use crate::types::{AgentResponse, ChatGenerateOptions, ChatMessage, GenerateOptions, ImageInput, ToolCall};

use base64::Engine;

#[cfg(feature = "mtmd")]
use llama_cpp_2::mtmd::{MtmdBitmap, MtmdContext, MtmdContextParams, MtmdInputText};

// ── LlamaBackend singleton ─────────────────────────────────────────────

static LLAMA_BACKEND: OnceLock<Result<LlamaBackend, llama_cpp_2::LlamaCppError>> = OnceLock::new();

fn get_backend() -> MlResult<&'static LlamaBackend> {
    LLAMA_BACKEND
        .get_or_init(|| {
            info!("Initialising llama.cpp backend");
            LlamaBackend::init()
        })
        .as_ref()
        .map_err(|e| MlError::Provider(format!("LlamaBackend init failed: {}", e)))
}

// ── LlamaCppConfig ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LlamaCppConfig {
    pub model_path: PathBuf,
    pub mmproj_path: Option<PathBuf>,
    pub n_ctx: u32,
    pub n_threads: u32,
    pub n_gpu_layers: u32,
}

impl Default for LlamaCppConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("model.gguf"),
            mmproj_path: None,
            n_ctx: 8192,
            n_threads: 4,
            n_gpu_layers: 99,
        }
    }
}

// ── Session — a loaded model ready for inference ────────────────────────

struct Session {
    model: LlamaModel,
}

// ── LlamaCppProvider ────────────────────────────────────────────────────

pub struct LlamaCppProvider {
    config: LlamaCppConfig,
    session: Arc<Mutex<Option<Session>>>,
    loaded: AtomicBool,
    #[cfg(feature = "mtmd")]
    mtmd_ctx: Arc<Mutex<Option<MtmdContext>>>,
}

impl LlamaCppProvider {
    pub fn new(config: LlamaCppConfig) -> Self {
        Self {
            config,
            session: Arc::new(Mutex::new(None)),
            loaded: AtomicBool::new(false),
            #[cfg(feature = "mtmd")]
            mtmd_ctx: Arc::new(Mutex::new(None)),
        }
    }

    #[cfg(feature = "mtmd")]
    fn resolve_bitmaps(
        &self,
        images: &[ImageInput],
    ) -> MlResult<Vec<MtmdBitmap>> {
        let mtmd_guard = self.mtmd_ctx.lock().unwrap();
        let mtmd = mtmd_guard
            .as_ref()
            .ok_or(MlError::Multimodal("MTMD context not loaded".into()))?;

        let mut bitmaps = Vec::with_capacity(images.len());
        for img in images {
            let bitmap = match img {
                ImageInput::Path(path) => {
                    MtmdBitmap::from_file(mtmd, path)
                        .map_err(|e| MlError::ImageProcessing(format!("{:?}", e)))?
                }
                ImageInput::Base64(b64) => {
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(b64)
                        .map_err(|e| MlError::Base64Decode(e.to_string()))?;
                    MtmdBitmap::from_buffer(mtmd, &bytes)
                        .map_err(|e| MlError::ImageProcessing(format!("{:?}", e)))?
                }
                ImageInput::Raw { nx, ny, data } => {
                    MtmdBitmap::from_image_data(*nx, *ny, data)
                        .map_err(|e| MlError::ImageProcessing(format!("{:?}", e)))?
                }
            };
            bitmaps.push(bitmap);
        }
        Ok(bitmaps)
    }
}

#[async_trait]
impl LlmProvider for LlamaCppProvider {
    fn provider_name(&self) -> &str {
        "llama-cpp"
    }

    async fn load_model(&self, model_path: &str) -> MlResult<()> {
        let path = PathBuf::from(model_path);
        let n_gpu_layers = self.config.n_gpu_layers;

        info!(?path, "Loading GGUF model");

        let session = tokio::task::spawn_blocking(move || -> MlResult<Session> {
            let backend = get_backend()?;

            let model_params = LlamaModelParams::default()
                .with_n_gpu_layers(n_gpu_layers);

            let model = LlamaModel::load_from_file(backend, &path, &model_params)
                .map_err(|e| MlError::Provider(format!("Model load failed: {:?}", e)))?;

            info!(
                n_params = model.n_params(),
                n_layers = model.n_layer(),
                n_vocab = model.n_vocab(),
                n_ctx_train = model.n_ctx_train(),
                "Model loaded"
            );

            Ok(Session { model })
        })
        .await
        .map_err(|e| MlError::Provider(format!("Task join error: {}", e)))??;

        *self.session.lock().unwrap() = Some(session);
        self.loaded.store(true, Ordering::Release);

        info!("Model ready for inference");
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        self.loaded.load(Ordering::Acquire)
    }

    async fn generate(&self, prompt: &str, options: &GenerateOptions) -> MlResult<String> {
        if !self.is_loaded() {
            return Err(MlError::NotLoaded);
        }

        let prompt_owned = prompt.to_string();
        let max_tokens = options.max_tokens;
        let temperature = options.temperature;
        let top_k = options.top_k;
        let top_p = options.top_p;
        let repeat_penalty = options.repeat_penalty;
        let seed = options.seed.unwrap_or(42);
        let n_ctx = self.config.n_ctx;
        let n_threads = self.config.n_threads;
        let stop_strings = options.stop_strings.clone();
        let session_arc = self.session.clone();

        let result = tokio::task::spawn_blocking(move || -> MlResult<String> {
            let backend = get_backend()?;

            let session_guard = session_arc.lock().unwrap();
            let session = session_guard
                .as_ref()
                .ok_or(MlError::NotLoaded)?;

            let tokens = session
                .model
                .str_to_token(&prompt_owned, AddBos::Always)
                .map_err(|e| MlError::Tokenization(format!("{:?}", e)))?;

            debug!(n_tokens = tokens.len(), "Prompt tokenised");

            let ctx_params = LlamaContextParams::default()
                .with_n_ctx(std::num::NonZeroU32::new(n_ctx))
                .with_n_threads(n_threads as i32)
                .with_n_threads_batch(n_threads as i32);

            let mut ctx = session
                .model
                .new_context(backend, ctx_params)
                .map_err(|e| MlError::Provider(format!("Context creation failed: {:?}", e)))?;

            let batch_capacity = tokens.len().max(max_tokens) + 1;
            let mut batch = LlamaBatch::new(batch_capacity, 1);
            let last_idx = (tokens.len().saturating_sub(1)) as i32;

            for (i, token) in tokens.iter().enumerate() {
                let needs_logits = i as i32 == last_idx;
                batch
                    .add(*token, i as i32, &[0], needs_logits)
                    .map_err(|e| MlError::Generation(format!("Batch add failed: {:?}", e)))?;
            }

            ctx.decode(&mut batch)
                .map_err(|e| MlError::Generation(format!("Prompt decode failed: {:?}", e)))?;

            let mut sampler = LlamaSampler::chain_simple([
                LlamaSampler::temp(temperature),
                LlamaSampler::top_k(top_k as i32),
                LlamaSampler::top_p(top_p, 1),
                LlamaSampler::penalties(64, repeat_penalty, 0.0, 0.0),
                LlamaSampler::dist(seed as u32),
            ]);

            let mut n_cur = batch.n_tokens() as i32;
            let mut generated_text = String::new();
            let mut decoder = encoding_rs::UTF_8.new_decoder();

            for _ in 0..max_tokens {
                let token = sampler.sample(&ctx, batch.n_tokens() - 1);

                if session.model.is_eog_token(token) {
                    break;
                }

                sampler.accept(token);

                let piece = session
                    .model
                    .token_to_piece(token, &mut decoder, true, None)
                    .map_err(|e| MlError::Generation(format!("token_to_piece failed: {:?}", e)))?;

                generated_text.push_str(&piece);

                if stop_strings.iter().any(|s| generated_text.contains(s)) {
                    break;
                }

                batch.clear();
                batch
                    .add(token, n_cur, &[0], true)
                    .map_err(|e| MlError::Generation(format!("Batch add failed: {:?}", e)))?;
                n_cur += 1;

                ctx.decode(&mut batch)
                    .map_err(|e| MlError::Generation(format!("Decode failed: {:?}", e)))?;
            }

            debug!(generated_len = generated_text.len(), "Generation complete");
            Ok(generated_text)
        })
        .await
        .map_err(|e| MlError::Provider(format!("Task join error: {}", e)))??;

        Ok(result)
    }

    async fn chat_generate(
        &self,
        messages: &[ChatMessage],
        options: &ChatGenerateOptions,
    ) -> MlResult<AgentResponse> {
        if !self.is_loaded() {
            return Err(MlError::NotLoaded);
        }

        let session_arc = self.session.clone();
        #[cfg(feature = "mtmd")]
        let mtmd_arc = self.mtmd_ctx.clone();
        let n_ctx = self.config.n_ctx;
        let n_threads = self.config.n_threads;
        let max_tokens = options.max_tokens;
        let temperature = options.temperature;
        let top_k = options.top_k;
        let top_p = options.top_p;
        let repeat_penalty = options.repeat_penalty;
        let seed = options.seed.unwrap_or(42);
        let tools = options.tools.clone();
        let parallel_tool_calls = options.parallel_tool_calls;
        let tool_choice = options.tool_choice.clone();

        // Collect images from multimodal messages
        let multimodal_images: Vec<ImageInput> = messages
            .iter()
            .flat_map(|m| m.content.images().iter().cloned())
            .collect();

        #[cfg(feature = "mtmd")]
        let bitmaps = if !multimodal_images.is_empty() {
            Some(self.resolve_bitmaps(&multimodal_images)?)
        } else {
            None
        };

        #[cfg(not(feature = "mtmd"))]
        if !multimodal_images.is_empty() {
            warn!("Multimodal images provided but mtmd feature is disabled — images will be ignored");
        }

        // Build OpenAI-compatible messages JSON for chat template application
        let messages_json = build_openai_messages_json(messages);
        let tools_json = if tools.is_empty() {
            None
        } else {
            Some(
                serde_json::Value::Array(
                    tools.iter().map(|t| t.to_openai_tool_json()).collect(),
                )
                .to_string(),
            )
        };

        let messages_owned = messages.to_vec();

        let result = tokio::task::spawn_blocking(move || -> MlResult<AgentResponse> {
            let backend = get_backend()?;

            let session_guard = session_arc.lock().unwrap();
            let session = session_guard
                .as_ref()
                .ok_or(MlError::NotLoaded)?;

            // Apply chat template with optional tools
            let chat_template = session
                .model
                .chat_template(None)
                .map_err(|e| MlError::ChatTemplate(format!("{:?}", e)))?;

            #[cfg(feature = "mtmd")]
            let media_marker = llama_cpp_2::mtmd::mtmd_default_marker().to_string();

            let formatted_prompt = if !tools.is_empty() {
                let openai_params = llama_cpp_2::openai::OpenAIChatTemplateParams {
                    messages_json: &messages_json.to_string(),
                    tools_json: tools_json.as_deref(),
                    tool_choice: Some(&tool_choice),
                    json_schema: None,
                    grammar: None,
                    reasoning_format: None,
                    chat_template_kwargs: Some("{}"),
                    add_generation_prompt: true,
                    use_jinja: true,
                    parallel_tool_calls,
                    enable_thinking: false,
                    add_bos: false,
                    add_eos: false,
                    parse_tool_calls: true,
                };

                let result = session
                    .model
                    .apply_chat_template_oaicompat(&chat_template, &openai_params)
                    .map_err(|e| MlError::ChatTemplate(format!("{:?}", e)))?;
                result.prompt
            } else {
                let llama_messages: Vec<llama_cpp_2::model::LlamaChatMessage> = messages_owned
                    .iter()
                    .map(|m| {
                        llama_cpp_2::model::LlamaChatMessage::new(
                            m.role.as_str().to_string(),
                            m.content.text().to_string(),
                        )
                        .unwrap_or_else(|_| {
                            llama_cpp_2::model::LlamaChatMessage::new(
                                "user".to_string(),
                                String::new(),
                            )
                            .unwrap()
                        })
                    })
                    .collect();

                session
                    .model
                    .apply_chat_template(&chat_template, &llama_messages, true)
                    .map_err(|e| MlError::ChatTemplate(format!("{:?}", e)))?
            };

            // ── Multimodal tokenization ──────────────────────────────────
            #[cfg(feature = "mtmd")]
            let (tokens, n_past_offset) = if let Some(ref bitmaps) = bitmaps {
                let mtmd_guard = mtmd_arc.lock().unwrap();
                let mtmd = mtmd_guard
                    .as_ref()
                    .ok_or(MlError::Multimodal("MTMD context not loaded".into()))?;

                // Insert media markers into the text
                let mut prompt_with_markers = formatted_prompt;
                let marker = &media_marker;
                if !prompt_with_markers.contains(marker) && !bitmaps.is_empty() {
                    prompt_with_markers.push_str(marker);
                }

                let input_text = MtmdInputText {
                    text: prompt_with_markers,
                    add_special: true,
                    parse_special: true,
                };

                let bitmap_refs: Vec<&MtmdBitmap> = bitmaps.iter().collect();
                let chunks = mtmd
                    .tokenize(input_text, &bitmap_refs)
                    .map_err(|e| MlError::Multimodal(format!("{:?}", e)))?;

                let ctx_params = LlamaContextParams::default()
                    .with_n_ctx(std::num::NonZeroU32::new(n_ctx))
                    .with_n_threads(n_threads as i32)
                    .with_n_threads_batch(n_threads as i32);

                let mut ctx = session
                    .model
                    .new_context(backend, ctx_params)
                    .map_err(|e| MlError::Provider(format!("Context creation failed: {:?}", e)))?;

                let mut batch = LlamaBatch::new(n_ctx as usize, 1);
                let n_past = chunks
                    .eval_chunks(mtmd, &mut ctx, 0, 0, 1, true)
                    .map_err(|e| MlError::Multimodal(format!("eval_chunks failed: {:?}", e)))?;

                // Generate response after multimodal prefill
                let generated = generate_from_context(
                    &session.model,
                    &mut ctx,
                    &mut batch,
                    n_past,
                    max_tokens,
                    temperature,
                    top_k,
                    top_p,
                    repeat_penalty,
                    seed,
                )?;

                // If the model has tools, try to parse tool calls from the response
                if !tools.is_empty() {
                    return parse_agent_response(&generated, &tools);
                }
                return Ok(AgentResponse::Text(generated));
            } else {
                // Text-only path
                let tokens = session
                    .model
                    .str_to_token(&formatted_prompt, AddBos::Always)
                    .map_err(|e| MlError::Tokenization(format!("{:?}", e)))?;
                (tokens, 0i32)
            };

            #[cfg(not(feature = "mtmd"))]
            let (tokens, n_past_offset) = {
                let tokens = session
                    .model
                    .str_to_token(&formatted_prompt, AddBos::Always)
                    .map_err(|e| MlError::Tokenization(format!("{:?}", e)))?;
                (tokens, 0i32)
            };

            // ── Standard text-only generation ─────────────────────────────
            let ctx_params = LlamaContextParams::default()
                .with_n_ctx(std::num::NonZeroU32::new(n_ctx))
                .with_n_threads(n_threads as i32)
                .with_n_threads_batch(n_threads as i32);

            let mut ctx = session
                .model
                .new_context(backend, ctx_params)
                .map_err(|e| MlError::Provider(format!("Context creation failed: {:?}", e)))?;

            let batch_capacity = tokens.len().max(max_tokens) + 1;
            let mut batch = LlamaBatch::new(batch_capacity, 1);
            let last_idx = (tokens.len().saturating_sub(1)) as i32;

            for (i, token) in tokens.iter().enumerate() {
                let needs_logits = i as i32 == last_idx;
                batch
                    .add(*token, i as i32, &[0], needs_logits)
                    .map_err(|e| MlError::Generation(format!("Batch add failed: {:?}", e)))?;
            }

            ctx.decode(&mut batch)
                .map_err(|e| MlError::Generation(format!("Prompt decode failed: {:?}", e)))?;

            let n_past_after_decode = n_past_offset + batch.n_tokens() as i32;

            let generated = generate_from_context(
                &session.model,
                &mut ctx,
                &mut batch,
                n_past_after_decode,
                max_tokens,
                temperature,
                top_k,
                top_p,
                repeat_penalty,
                seed,
            )?;

            if !tools.is_empty() {
                parse_agent_response(&generated, &tools)
            } else {
                Ok(AgentResponse::Text(generated))
            }
        })
        .await
        .map_err(|e| MlError::Provider(format!("Task join error: {}", e)))??;

        Ok(result)
    }

    #[cfg(feature = "mtmd")]
    async fn load_mmproj(&self, mmproj_path: &str) -> MlResult<()> {
        if !self.is_loaded() {
            return Err(MlError::NotLoaded);
        }

        let path_owned = mmproj_path.to_string();
        let n_threads = self.config.n_threads;
        let session_arc = self.session.clone();
        let mtmd_arc = self.mtmd_ctx.clone();

        tokio::task::spawn_blocking(move || -> MlResult<()> {
            let session_guard = session_arc.lock().unwrap();
            let session = session_guard
                .as_ref()
                .ok_or(MlError::NotLoaded)?;

            let use_gpu = true;
            let mtmd_params = MtmdContextParams {
                use_gpu,
                print_timings: true,
                n_threads: n_threads as i32,
                media_marker: std::ffi::CString::new(llama_cpp_2::mtmd::mtmd_default_marker())
                    .map_err(|e| MlError::Multimodal(e.to_string()))?,
            };

            let mtmd_ctx = MtmdContext::init_from_file(&path_owned, &session.model, &mtmd_params)
                .map_err(|e| MlError::Multimodal(format!("{:?}", e)))?;

            info!("MTMD context loaded from {}", path_owned);

            if mtmd_ctx.support_vision() {
                info!("Multimodal model supports vision input");
            }
            if mtmd_ctx.support_audio() {
                info!("Multimodal model supports audio input");
            }

            *mtmd_arc.lock().unwrap() = Some(mtmd_ctx);
            Ok(())
        })
        .await
        .map_err(|e| MlError::Provider(format!("Task join error: {}", e)))??;

        Ok(())
    }
}

// ── Helper: generate tokens from the current context state ──────────────

fn generate_from_context(
    model: &LlamaModel,
    ctx: &mut llama_cpp_2::context::LlamaContext,
    mut batch: &mut LlamaBatch,
    n_past: i32,
    max_tokens: usize,
    temperature: f32,
    top_k: i32,
    top_p: f32,
    repeat_penalty: f32,
    seed: u64,
) -> MlResult<String> {
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::temp(temperature),
        LlamaSampler::top_k(top_k),
        LlamaSampler::top_p(top_p, 1),
        LlamaSampler::penalties(64, repeat_penalty, 0.0, 0.0),
        LlamaSampler::dist(seed as u32),
    ]);

    let mut n_cur = n_past;
    let mut generated_text = String::new();
    let mut decoder = encoding_rs::UTF_8.new_decoder();

    for _ in 0..max_tokens {
        let token = sampler.sample(ctx, batch.n_tokens() - 1);

        if model.is_eog_token(token) {
            break;
        }

        sampler.accept(token);

        let piece = model
            .token_to_piece(token, &mut decoder, true, None)
            .map_err(|e| MlError::Generation(format!("token_to_piece failed: {:?}", e)))?;

        generated_text.push_str(&piece);

        batch.clear();
        batch
            .add(token, n_cur, &[0], true)
            .map_err(|e| MlError::Generation(format!("Batch add failed: {:?}", e)))?;
        n_cur += 1;

        ctx.decode(&mut batch)
            .map_err(|e| MlError::Generation(format!("Decode failed: {:?}", e)))?;
    }

    debug!(generated_len = generated_text.len(), "Generation complete");
    Ok(generated_text)
}

// ── Helper: build OpenAI-compatible messages JSON ───────────────────────

fn build_openai_messages_json(messages: &[ChatMessage]) -> serde_json::Value {
    let json_msgs: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            let role = m.role.as_str();
            match &m.content {
                crate::types::ChatContent::Text(t) => serde_json::json!({
                    "role": role,
                    "content": t,
                }),
                crate::types::ChatContent::Multimodal { text, images } => {
                    let content_parts: Vec<serde_json::Value> = std::iter::once(
                        serde_json::json!({"type": "text", "text": text}),
                    )
                    .chain(images.iter().map(|_| {
                        serde_json::json!({"type": "image_url", "image_url": {"url": "embedded"}})
                    }))
                    .collect();
                    serde_json::json!({
                        "role": role,
                        "content": content_parts,
                    })
                }
                crate::types::ChatContent::ToolResult { tool_call_id, result } => {
                    serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tool_call_id,
                        "content": result,
                    })
                }
            }
        })
        .collect();
    serde_json::Value::Array(json_msgs)
}

// ── Helper: parse model output for tool calls ───────────────────────────

fn parse_agent_response(
    raw: &str,
    _tools: &[crate::types::ToolDefinition],
) -> MlResult<AgentResponse> {
    let trimmed = raw.trim();

    // Try to find JSON tool calls in the output
    if let Some(calls) = try_parse_tool_calls(trimmed) {
        if !calls.is_empty() {
            return Ok(AgentResponse::ToolCalls(calls));
        }
    }

    // No tool calls found — return as plain text
    Ok(AgentResponse::Text(raw.to_string()))
}

fn try_parse_tool_calls(text: &str) -> Option<Vec<ToolCall>> {
    // Pattern 1: The model outputs a JSON array of tool calls
    // e.g. [{"name": "get_weather", "arguments": {"location": "Paris"}}]
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(arr) = val.as_array() {
            let calls: Vec<ToolCall> = arr
                .iter()
                .filter_map(|v| json_to_tool_call(v))
                .collect();
            if !calls.is_empty() {
                return Some(calls);
            }
        }
        // Single object tool call
        if let Some(call) = json_to_tool_call(&val) {
            return Some(vec![call]);
        }
    }

    // Pattern 2: Look for JSON embedded in text
    let json_start = text.find('[');
    let json_end = text.rfind(']');
    if let (Some(s), Some(e)) = (json_start, json_end) {
        if e > s {
            let snippet = &text[s..=e];
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(snippet) {
                if let Some(arr) = val.as_array() {
                    let calls: Vec<ToolCall> = arr
                        .iter()
                        .filter_map(|v| json_to_tool_call(v))
                        .collect();
                    if !calls.is_empty() {
                        return Some(calls);
                    }
                }
            }
        }
    }

    // Pattern 3: Look for XML-style tool calls (commonly generated by Qwen models)
    if let Some(calls) = parse_xml_tool_calls(text) {
        return Some(calls);
    }

    None
}

fn parse_xml_tool_calls(text: &str) -> Option<Vec<ToolCall>> {
    let mut calls = Vec::new();
    let tool_call_blocks: Vec<&str> = text.split("<tool_call>").collect();
    for block in tool_call_blocks.into_iter().skip(1) {
        let block_content = match block.split("</tool_call>").next() {
            Some(c) => c,
            None => continue,
        };
        
        // Extract function name
        let func_start = match block_content.find("<function=") {
            Some(idx) => idx,
            None => continue,
        };
        let func_end = match block_content[func_start..].find('>') {
            Some(idx) => func_start + idx,
            None => continue,
        };
        let function_name = block_content[func_start + 10..func_end].trim().to_string();
        
        // Extract parameters
        let mut arguments = serde_json::Map::new();
        let param_split: Vec<&str> = block_content.split("<parameter=").collect();
        for param_chunk in param_split.into_iter().skip(1) {
            let p_end = match param_chunk.find('>') {
                Some(idx) => idx,
                None => continue,
            };
            let param_name = param_chunk[..p_end].trim().to_string();
            let p_val_start = p_end + 1;
            let p_val_end = match param_chunk[p_val_start..].find("</parameter>") {
                Some(idx) => p_val_start + idx,
                None => continue,
            };
            let raw_val = param_chunk[p_val_start..p_val_end].trim();
            
            // Convert value to JSON
            let json_val = if raw_val.eq_ignore_ascii_case("true") {
                serde_json::Value::Bool(true)
            } else if raw_val.eq_ignore_ascii_case("false") {
                serde_json::Value::Bool(false)
            } else if let Ok(num) = raw_val.parse::<i64>() {
                serde_json::Value::Number(num.into())
            } else if let Ok(f) = raw_val.parse::<f64>() {
                if let Some(num) = serde_json::Number::from_f64(f) {
                    serde_json::Value::Number(num)
                } else {
                    serde_json::Value::String(raw_val.to_string())
                }
            } else {
                serde_json::Value::String(raw_val.to_string())
            };
            
            arguments.insert(param_name, json_val);
        }
        
        calls.push(ToolCall {
            id: format!("call_{}", calls.len()),
            name: function_name,
            arguments: serde_json::Value::Object(arguments),
        });
    }
    
    if calls.is_empty() {
        None
    } else {
        Some(calls)
    }
}

fn json_to_tool_call(v: &serde_json::Value) -> Option<ToolCall> {
    let obj = v.as_object()?;
    let name = obj.get("name")?.as_str()?.to_string();
    let arguments = obj
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("call_0")
        .to_string();
    Some(ToolCall {
        id,
        name,
        arguments,
    })
}