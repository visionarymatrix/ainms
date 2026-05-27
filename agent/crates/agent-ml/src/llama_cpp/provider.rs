//! LlamaCpp provider — concrete [`LlmProvider`] backed by `llama-cpp-2`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use tracing::{debug, info};

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;

use crate::error::{MlError, MlResult};
use crate::provider::LlmProvider;
use crate::types::GenerateOptions;

// ── LlamaBackend singleton ─────────────────────────────────────────────
// Must be initialised exactly once per process.  A second call returns
// `BackendAlreadyInitialized`, which we surface as an error.

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

/// Configuration for the `llama-cpp-2` provider.
#[derive(Debug, Clone)]
pub struct LlamaCppConfig {
    /// Path to the GGUF model file on disk.
    pub model_path: PathBuf,
    /// Size of the context window (in tokens).
    pub n_ctx: u32,
    /// Number of CPU threads for inference.
    pub n_threads: u32,
    /// Number of layers to offload to GPU (0 = CPU only, 99 = all layers).
    pub n_gpu_layers: u32,
}

impl Default for LlamaCppConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("model.gguf"),
            n_ctx: 2048,
            n_threads: 4,
            n_gpu_layers: 99,
        }
    }
}

// ── Session — a loaded model ready for inference ────────────────────────

/// Holds a loaded `LlamaModel`.  Contexts are created per-generate call
/// (they're cheap enough and this avoids lifetime issues with borrowing
/// the model).
struct Session {
    model: LlamaModel,
}

// ── LlamaCppProvider ────────────────────────────────────────────────────

/// LLM provider backed by `llama-cpp-2` (llama.cpp C++ bindings).
///
/// # Thread safety
///
/// The inner session is wrapped in an `Arc<Mutex<Option<Session>>>`.
/// All heavyweight operations (`load_model`, `generate`) are dispatched
/// to `spawn_blocking` to avoid blocking the tokio runtime.  The `Arc`
/// handle is cloned and moved into the blocking closure.
///
/// # Lifecycle
///
/// 1. Create with [`LlamaCppProvider::new`].
/// 2. Call [`load_model`](LlmProvider::load_model) to load a GGUF file.
/// 3. Call [`generate`](LlmProvider::generate) as many times as needed.
pub struct LlamaCppProvider {
    config: LlamaCppConfig,
    /// The loaded model, if any. Behind Arc<Mutex> so the Arc handle
    /// can be cloned and moved into `spawn_blocking` closures.
    session: Arc<Mutex<Option<Session>>>,
    loaded: AtomicBool,
}

impl LlamaCppProvider {
    pub fn new(config: LlamaCppConfig) -> Self {
        Self {
            config,
            session: Arc::new(Mutex::new(None)),
            loaded: AtomicBool::new(false),
        }
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

            // ── 1. Tokenise prompt ────────────────────────────────────
            let tokens = session
                .model
                .str_to_token(&prompt_owned, AddBos::Always)
                .map_err(|e| MlError::Tokenization(format!("{:?}", e)))?;

            debug!(n_tokens = tokens.len(), "Prompt tokenised");

            // ── 2. Create context ─────────────────────────────────────
            let ctx_params = LlamaContextParams::default()
                .with_n_ctx(std::num::NonZeroU32::new(n_ctx))
                .with_n_threads(n_threads as i32)
                .with_n_threads_batch(n_threads as i32);

            let mut ctx = session
                .model
                .new_context(backend, ctx_params)
                .map_err(|e| MlError::Provider(format!("Context creation failed: {:?}", e)))?;

            // ── 3. Prepare batch and encode prompt ─────────────────────
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

            // ── 4. Build sampler chain ─────────────────────────────────
            let mut sampler = LlamaSampler::chain_simple([
                LlamaSampler::temp(temperature),
                LlamaSampler::top_k(top_k as i32),
                LlamaSampler::top_p(top_p, 1),
                LlamaSampler::penalties(64, repeat_penalty, 0.0, 0.0),
                LlamaSampler::dist(seed as u32),
            ]);

            // ── 5. Generate tokens ─────────────────────────────────────
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

                // Check stop strings
                if stop_strings.iter().any(|s| generated_text.contains(s)) {
                    break;
                }

                // Prepare next batch with just this token
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
}