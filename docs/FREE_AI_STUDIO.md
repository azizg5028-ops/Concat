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
- A Q4_K_M GGUF build is approximately 1.28 GB; the exact file and SHA-256 must be pinned before shipping.
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

1. Add llama.cpp as an optional Android arm64 native runtime and verify its license/notices and reproducible pinned source revision.
2. Add a model manager with size display, checksum verification, resumable/cancellable download, storage check, remove, and model-ready state.
3. Add a local inference interface with cancellation and bounded context/output sizes so a phone does not exhaust memory.
4. Add the four role prompts and typed JSON schemas; validate every model response before any edit reaches the Concat API.
5. Add a production preview/review step. The Producer must never silently overwrite a project or export without the user's confirmation.
6. Run the existing Android CI build and emulator smoke tests, then test inference on a physical arm64 phone before calling the feature complete.

## Known limits

- "Free" means no provider bill or subscription. The user still needs internet once to download the model and enough device storage/RAM to run it.
- A 1.7B on-device model is a compact assistant, not a cloud frontier model. Complex research, long scripts, and dialect-specific output may be weaker or slower.
- Offline Researcher can only verify material it has been given; it cannot claim to have searched the live web.
- The APK must not contain paid-provider credentials or require the user to create an API key.
