# Free, on-device AI production studio

## Non-negotiable product rules

- No paid APIs, subscriptions, API keys, or accounts.
- Text generation runs locally on the Android device; no video, audio, prompt, or project is uploaded.
- Model weights are downloaded only after the user chooses to install them. Show the exact download size, available storage, progress, and a cancel action.
- The studio must remain useful without the language model installed: existing editor, captions, phone TTS, audio effects, and export continue to work.
- Never show a control as ready when its engine is missing. Report unavailable features honestly.

## Proposed local language model

Use the Qwen3 1.7B instruct model in GGUF format, quantized for phone memory, through llama.cpp.

- Upstream model: https://huggingface.co/Qwen/Qwen3-1.7B
- Android runtime/build guidance: https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md
- Selected artifact: `ggml-org/Qwen3-1.7B-GGUF` / `Qwen3-1.7B-Q4_K_M.gguf`, 1,282,439,264 bytes. Pinned revision: `daeb8e2d528a760970442092f6bf1e55c3b659eb`. SHA-256: `d2387ca2dbfee2ffabce7120d3770dadca0b293052bc2f0e138fdc940d9bc7b5`.
- The upstream Qwen3 model is Apache-2.0 and documents multilingual support across 100+ languages. Iraqi Arabic quality must still be tested on real prompts; do not promise perfect dialect output.
- Android target is arm64-v8a. Build the runtime with the Android NDK, portable CPU settings, and no OpenMP dependency unless the build is separately validated.

The model is not bundled in the APK. It is downloaded on demand to app-private storage and can be removed from Settings. Keep inference on-device after download.

## Four production roles

All four roles use the same installed local model with different system instructions and structured outputs. They are workflow roles, not four separately billed services.

1. **Writer** — turns the user's brief into a script, hook, scene-by-scene narration, on-screen text, and shot list. The user can edit the script before production.
2. **Researcher** — extracts claims, facts, and citations from sources the user imports or supplies. Offline mode must not pretend to perform live web research. A future online mode may be added only if it remains optional and free.
3. **Audio Engineer** — prepares narration text and audio instructions, then delegates transcription/captions and speech synthesis to Concat's existing local speech facilities. On Android, use the phone's system TTS; do not depend on a paid voice provider.
4. **Producer** — converts the approved storyboard into a Concat project plan, then calls the existing Concat editing API for media, clips, titles, captions, effects, timeline operations, and export. Require confirmation before destructive changes or final export.

## Existing Concat capabilities to reuse

- Whisper.cpp local transcription for captions.
- Android system text-to-speech, with no separate voice-model download.
- Existing audio processing/effects, timeline, render, and export engine.
- Existing JSON-RPC/MCP API as the controlled bridge for project operations.

Do not duplicate these engines or replace them with cloud services.

## Android delivery and validation

### Implemented in the current development branch

- Added `concat-ai`, an Android-only llama.cpp inference dependency, with no cloud endpoint or API key.
- Added the pinned Q4_K_M model metadata and an on-demand downloader using Concat's existing resumable/cancellable download path and SHA-256 verification.
- Added four role prompt presets. The Researcher is explicitly source-limited; the Producer only drafts plans and does not claim to edit the project.
- Added bounded generation (2,048-token context, at most 512 generated tokens) and token-level cancellation.
- Added an AI Studio page to Settings with model download progress, role selection, a brief field, streaming output, and generation cancellation.
- Connected the Settings UI to the model manager and Android worker. The model is still optional and is not bundled in the APK.

### Still required before release

1. Add a free-storage check before downloading the 1.28 GB model and a remove-model action to reclaim storage.
2. Add typed JSON output schemas and validate every model response before any edit reaches the Concat API.
3. Connect the approved Producer plan to the Concat editing API, with preview and explicit confirmation before project changes or export.
4. Run Android CI and emulator smoke tests, then test inference on a physical arm64 phone. The current branch has not yet produced a verified APK.

## Known limits

- "Free" means no provider bill or subscription. The user still needs internet once to download the model and enough device storage/RAM to run it.
- A 1.7B on-device model is a compact assistant, not a cloud frontier model. Complex research, long scripts, and dialect-specific output may be weaker or slower.
- Offline Researcher can only verify material it has been given; it cannot claim to have searched the live web.
- The APK must not contain paid-provider credentials or require the user to create an API key.
