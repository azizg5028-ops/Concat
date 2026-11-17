// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Concat contributors

//! Optional, fully local text assistance for Concat.
//! Model weights are downloaded on demand, checked against a pinned SHA-256,
//! and kept in app-private data. Android inference is backed by llama.cpp.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use concat_host::{AppDirs, models};

/// Stable identifier shown to the UI.
pub const MODEL_ID: &str = "qwen3-1.7b-q4_k_m";
/// Exact GGUF artifact filename.
pub const MODEL_FILE: &str = "Qwen3-1.7B-Q4_K_M.gguf";
/// Approximate size used only while a server omits Content-Length.
pub const MODEL_BYTES_ESTIMATE: u64 = 1_282_439_264;
/// Expected SHA-256 of the upstream GGUF artifact.
pub const MODEL_SHA256: &str = "d2387ca2dbfee2ffabce7120d3770dadca0b293052bc2f0e138fdc940d9bc7b5";
/// Immutable upstream commit for the selected quantization.
pub const MODEL_REVISION: &str = "daeb8e2d528a760970442092f6bf1e55c3b659eb";
/// Direct download URL for the pinned artifact.
pub const MODEL_UPSTREAM: &str = "https://huggingface.co/ggml-org/Qwen3-1.7B-GGUF/resolve/daeb8e2d528a760970442092f6bf1e55c3b659eb/Qwen3-1.7B-Q4_K_M.gguf";

/// Four prompt presets over one installed local model, not four separate services.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Writer,
    Researcher,
    AudioEngineer,
    Producer,
}

impl Role {
    pub fn system_prompt(self) -> &'static str {
        match self {
            Role::Writer => "You are Concat Writer, a professional short-form video scriptwriter. Reply in the user's language; when the user writes Arabic, use clear natural Arabic and Iraqi dialect when appropriate. Create a strong opening hook, concise voiceover, scene-by-scene visuals, on-screen text, and a practical shot list. Do not invent factual claims. Clearly mark assumptions.",
            Role::Researcher => "You are Concat Researcher. Work only from sources, notes, files, and facts the user provides or imports into the project. Do not claim to browse the live web or verify facts you cannot access. Separate supported facts from assumptions and identify which supplied source supports each factual point. If no sources are supplied, say that offline research needs source material.",
            Role::AudioEngineer => "You are Concat Audio Engineer. Prepare clean narration text, pronunciation notes, pauses, emphasis, sound-effect cues, and caption timing suggestions. Prefer clear voiceover and natural sound effects; do not add background music by default. Never claim to have generated or placed an audio file unless a tool confirms it.",
            Role::Producer => "You are Concat Producer. Turn the approved brief into a structured, reviewable video edit plan: aspect ratio, duration, ordered scenes, source media needed, clip timing, titles, captions, narration, transitions, and export settings. Do not claim that you changed the timeline, edited media, or exported a video. Any future edit operation must be previewed and explicitly approved by the user first.",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelStatus {
    pub installed: bool,
    pub bytes_on_disk: u64,
    pub path: PathBuf,
}

pub fn model_path(dirs: &AppDirs) -> PathBuf {
    dirs.data.join("ai-models").join(MODEL_FILE)
}

/// Does not hash the large file on every UI refresh. Download and runtime verify it.
pub fn status(dirs: &AppDirs) -> ModelStatus {
    let path = model_path(dirs);
    let bytes_on_disk = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
    ModelStatus { installed: bytes_on_disk > 0, bytes_on_disk, path }
}

#[cfg(target_os = "android")]
fn available_bytes(path: &std::path::Path) -> Option<u64> {
    use std::ffi::CString;
    let path = CString::new(path.to_string_lossy().as_bytes()).ok()?;
    // SAFETY: statvfs writes to the output struct and the C path remains alive
    // for the duration of the call.
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::statvfs(path.as_ptr(), &mut stats) };
    (result == 0).then(|| (stats.f_bavail as u64).saturating_mul(stats.f_frsize as u64))
}

/// Downloads on a worker thread using Concat's resumable, cancellable,
/// SHA-256-verifying downloader. The final filename appears only after verification.
pub fn download_model(
    dirs: &AppDirs,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<PathBuf, String> {
    let target = model_path(dirs);
    let folder = target.parent().ok_or_else(|| "the AI model folder has no parent".to_owned())?;
    std::fs::create_dir_all(folder).map_err(|error| format!("could not create {}: {error}", folder.display()))?;

    #[cfg(target_os = "android")]
    {
        let needed = MODEL_BYTES_ESTIMATE.saturating_add(64 * 1024 * 1024);
        match available_bytes(folder) {
            Some(free) if free >= needed => {}
            Some(free) => return Err(format!(
                "not enough free storage: the model needs about {:.0} MB plus temporary space, but only {:.0} MB is available",
                needed as f64 / 1_000_000.0,
                free as f64 / 1_000_000.0
            )),
            None => return Err("could not check free storage before downloading the AI model".to_owned()),
        }
    }

    if target.is_file() {
        if models::verify(&target, MODEL_SHA256).is_ok() {
            return Ok(target);
        }
        std::fs::remove_file(&target).map_err(|error| format!("could not remove the invalid model: {error}"))?;
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("download cancelled".to_owned());
    }

    let partial = target.with_extension("gguf.part");
    models::fetch(
        MODEL_FILE, MODEL_UPSTREAM, &partial, MODEL_BYTES_ESTIMATE, MODEL_SHA256,
        cancel, "download cancelled", progress,
    )?;
    std::fs::rename(&partial, &target)
        .map_err(|error| format!("could not install the verified AI model: {error}"))?;
    Ok(target)
}

pub fn remove_model(dirs: &AppDirs) -> Result<(), String> {
    let path = model_path(dirs);
    let partial = path.with_extension("gguf.part");
    for candidate in [path, partial] {
        if candidate.exists() {
            std::fs::remove_file(&candidate)
                .map_err(|error| format!("could not remove {}: {error}", candidate.display()))?;
        }
    }
    Ok(())
}

pub const fn runtime_available() -> bool {
    cfg!(target_os = "android")
}

#[cfg(target_os = "android")]
pub mod runtime {
    //! Android's local llama.cpp runtime. Load and generate on a worker thread.

    use std::num::NonZeroU32;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use encoding_rs::UTF_8;
    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaModel};
    use llama_cpp_2::sampling::LlamaSampler;
    use crate::Role;

    use std::sync::mpsc::{self, Sender};
    use std::sync::Arc;
    use std::thread;

    enum WorkerCommand {
        Generate { role: Role, user_text: String, max_tokens: usize },
    }

    /// Events from the persistent model worker. Forward them to the UI thread.
    #[derive(Clone, Debug)]
    pub enum WorkerEvent {
        Ready(Result<(), String>),
        Chunk(String),
        Finished(Result<String, String>),
    }

    /// A background worker that loads the model once and reuses it.
    /// Dropping it cancels the active request and releases model memory.
    pub struct AssistantWorker {
        commands: Sender<WorkerCommand>,
        cancel: Arc<AtomicBool>,
        busy: Arc<AtomicBool>,
    }

    impl AssistantWorker {
        /// Starts model loading in the background. The callback runs on the
        /// worker thread and must dispatch UI changes to the UI thread.
        pub fn start(
            path: std::path::PathBuf,
            mut events: impl FnMut(WorkerEvent) + Send + 'static,
        ) -> Result<Self, String> {
            let (commands, receiver) = mpsc::channel::<WorkerCommand>();
            let cancel = Arc::new(AtomicBool::new(false));
            let worker_cancel = Arc::clone(&cancel);
            let busy = Arc::new(AtomicBool::new(false));
            let worker_busy = Arc::clone(&busy);
            thread::Builder::new()
                .name("concat-local-ai".to_owned())
                .spawn(move || {
                    let assistant = match LocalAssistant::load(&path) {
                        Ok(assistant) => {
                            events(WorkerEvent::Ready(Ok(())));
                            assistant
                        }
                        Err(error) => {
                            events(WorkerEvent::Ready(Err(error)));
                            return;
                        }
                    };

                    while let Ok(command) = receiver.recv() {
                        match command {
                            WorkerCommand::Generate { role, user_text, max_tokens } => {
                                let result = assistant.generate(
                                    role,
                                    &user_text,
                                    max_tokens,
                                    &worker_cancel,
                                    |piece| events(WorkerEvent::Chunk(piece.to_owned())),
                                );
                                worker_busy.store(false, Ordering::Relaxed);
                                events(WorkerEvent::Finished(result));
                            }
                        }
                    }
                })
                .map_err(|error| format!("could not start the local AI worker: {error}"))?;
            Ok(Self { commands, cancel, busy })
        }

        /// Queues a prompt on the already-loaded model.
        pub fn generate(
            &self,
            role: Role,
            user_text: String,
            max_tokens: usize,
        ) -> Result<(), String> {
            if self.busy.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed).is_err() {
                return Err("the local AI assistant is already generating".to_owned());
            }
            self.cancel.store(false, Ordering::Relaxed);
            if self.commands.send(WorkerCommand::Generate { role, user_text, max_tokens }).is_err() {
                self.busy.store(false, Ordering::Relaxed);
                return Err("the local AI worker is no longer running".to_owned());
            }
            Ok(())
        }

        /// Stops generation at the next token boundary.
        pub fn cancel(&self) {
            self.cancel.store(true, Ordering::Relaxed);
        }
    }

    impl Drop for AssistantWorker {
        fn drop(&mut self) {
            self.cancel.store(true, Ordering::Relaxed);
        }
    }

    pub struct LocalAssistant {
        backend: LlamaBackend,
        model: LlamaModel,
    }

    impl LocalAssistant {
        pub fn load(path: &Path) -> Result<Self, String> {
            if !path.is_file() {
                return Err("the local AI model is not installed".to_owned());
            }
            super::models::verify(path, super::MODEL_SHA256)?;
            let backend = LlamaBackend::init()
                .map_err(|error| format!("could not start the local AI runtime: {error}"))?;
            let model = LlamaModel::load_from_file(&backend, path, &LlamaModelParams::default())
                .map_err(|error| format!("could not load the local AI model: {error}"))?;
            Ok(Self { backend, model })
        }

        /// Generates at most 512 tokens. on_piece receives UTF-8-safe fragments.
        pub fn generate(
            &self,
            role: Role,
            user_text: &str,
            max_tokens: usize,
            cancel: &AtomicBool,
            mut on_piece: impl FnMut(&str),
        ) -> Result<String, String> {
            const CONTEXT: u32 = 2048;
            const MAX_INPUT_CHARS: usize = 12_000;
            if user_text.trim().is_empty() {
                return Err("write a brief before asking the AI to generate".to_owned());
            }
            if user_text.chars().count() > MAX_INPUT_CHARS {
                return Err("the brief is too long for the phone's local model".to_owned());
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("generation cancelled".to_owned());
            }

            let template = self.model.chat_template(None)
                .map_err(|error| format!("could not read the model's chat template: {error}"))?;
            let messages = [
                LlamaChatMessage::new("system".to_owned(), role.system_prompt().to_owned())
                    .map_err(|error| error.to_string())?,
                LlamaChatMessage::new("user".to_owned(), user_text.to_owned())
                    .map_err(|error| error.to_string())?,
            ];
            let prompt = self.model.apply_chat_template(&template, &messages, true)
                .map_err(|error| format!("could not prepare the AI prompt: {error}"))?;
            let tokens = self.model.str_to_token(&prompt, AddBos::Always)
                .map_err(|error| format!("could not tokenize the AI prompt: {error}"))?;
            let limit = max_tokens.clamp(1, 512);
            if tokens.is_empty() || tokens.len() + limit > CONTEXT as usize {
                return Err("the brief is too long for the current context window".to_owned());
            }

            let context_params = LlamaContextParams::default()
                .with_n_ctx(Some(NonZeroU32::new(CONTEXT).expect("non-zero context")))
                .with_n_threads(4)
                .with_n_threads_batch(4);
            let mut context = self.model.new_context(&self.backend, context_params)
                .map_err(|error| format!("could not create the local AI context: {error}"))?;

            let mut batch = LlamaBatch::new(tokens.len().max(1), 1);
            let last = tokens.len() - 1;
            for (index, token) in tokens.into_iter().enumerate() {
                batch.add(token, index as i32, &[0], index == last)
                    .map_err(|error| format!("could not prepare the AI input: {error}"))?;
            }
            context.decode(&mut batch)
                .map_err(|error| format!("the local AI model could not read the prompt: {error}"))?;

            let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);
            let mut decoder = UTF_8.new_decoder();
            let mut output = String::new();
            let mut position = batch.n_tokens();

            for _ in 0..limit {
                if cancel.load(Ordering::Relaxed) {
                    return Err("generation cancelled".to_owned());
                }
                let token = sampler.sample(&context, batch.n_tokens() - 1);
                sampler.accept(token);
                if self.model.is_eog_token(token) {
                    break;
                }
                let piece = self.model
                    .token_to_piece(token, &mut decoder, false, None)
                    .map_err(|error| format!("could not decode an AI response token: {error}"))?;
                output.push_str(&piece);
                if !piece.is_empty() {
                    on_piece(&piece);
                }
                batch.clear();
                batch.add(token, position, &[0], true)
                    .map_err(|error| format!("could not prepare the next AI token: {error}"))?;
                position += 1;
                context.decode(&mut batch)
                    .map_err(|error| format!("the local AI model stopped while generating: {error}"))?;
            }
            Ok(output)
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_artifact_is_pinned_and_checkable() {
        assert_eq!(MODEL_BYTES_ESTIMATE, 1_282_439_264);
        assert_eq!(MODEL_SHA256.len(), 64);
        assert!(MODEL_UPSTREAM.contains(MODEL_REVISION));
        assert!(MODEL_UPSTREAM.ends_with(MODEL_FILE));
    }

    #[test]
    fn every_production_role_has_a_prompt() {
        for role in [Role::Writer, Role::Researcher, Role::AudioEngineer, Role::Producer] {
            assert!(!role.system_prompt().trim().is_empty());
        }
    }
}
