# Parakeet Model Download Implementation Plan

**Goal**
Implement explicit in-app download for the Parakeet model and prevent normal recording when the selected transcription model is missing.

**Implementation Constraints**
- Do not add model-download logic to `src-tauri/src/lib.rs`; keep it in `transcription` and expose it through thin commands.
- Download is explicit. No auto-download on launch.
- Downloading Parakeet selects Parakeet after install.
- Cancellation is required. Resume is not required.
- Use one pinned Parakeet archive URL/checksum. No custom URLs.
- Failed/cancelled downloads must never make the model look installed.

**Backend Plan**
1. Add model artifact metadata in `src-tauri/src/transcription/artifacts.rs`.
   - Define Parakeet model id: `sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8`.
   - Define archive URL, archive byte size, display size, SHA-256 checksum, final model directory, temp download path, temp extract path, and required files.
   - Reuse the required-file list for both installed checks and post-extract validation.

2. Add download state in `src-tauri/src/transcription/download.rs`.
   - Add a Tauri-managed `ModelDownloadState`.
   - Track provider, state, progress bytes, total bytes, current error, and cancellation flag.
   - Supported states: `idle`, `downloading`, `installing`, `failed`, `cancelled`.

3. Implement download/install flow.
   - Stream archive to `data_dir/models/.downloads/<model-id>.tar.bz2.part`.
   - Update progress while streaming.
   - Check cancellation between chunks.
   - Verify SHA-256 before extraction.
   - Extract into `data_dir/models/.downloads/<model-id>.extracting`.
   - Validate `encoder.int8.onnx`, `decoder.int8.onnx`, `joiner.int8.onnx`, and `tokens.txt`.
   - Atomically move/copy validated files into `data_dir/models/parakeet/<model-id>`.
   - Clean temp files on success, cancel, and failure where safe.

4. Add commands in `src-tauri/src/commands/transcription.rs`.
   - `get_transcription_status`
   - `start_transcription_model_download(provider)`
   - `cancel_transcription_model_download(provider)`
   - Register these in both command lists in `src-tauri/src/lib.rs`.
   - Move or wrap the current `commands::system::get_transcription_status` so there is one transcription command surface.

5. Extend frontend-visible payloads.
   - Extend `TranscriptionModelStatus` with:
     - `downloadable`
     - `downloadState`
     - `progressBytes`
     - `totalBytes`
     - `displaySize`
     - `canDownload`
     - `canCancel`
     - `errorMessage`
   - Add exported download-state type if needed.
   - Run binding generation/checks.

6. Enforce backend recording guard in `src-tauri/src/recording/workflow.rs`.
   - Before thread creation, work-dir prep, or `prepare_audio_input`, resolve selected transcription status.
   - If selected model is missing, return `"{Provider} model is required before recording"`.
   - Apply to normal and fixture recording paths.

7. Persist provider after successful Parakeet install.
   - On successful Parakeet download, update `SettingsState` and `settings.json` to `transcriptionProvider: "parakeet"`.
   - Refresh transcription status after settings update.

**Frontend Plan**
1. Extend `src/api/transcription.ts`.
   - Add `startModelDownload(provider)`.
   - Add `cancelModelDownload(provider)`.
   - Keep `getStatus()`.

2. Add app/controller wiring.
   - Add actions in `src/features/app/state.ts` and reducer handling in `src/features/app/reducer.ts` if download status is event-driven.
   - Add controller methods for start download, cancel download, refresh transcription status, and switch to Whisper.
   - If using backend events, add listener in `src/features/app/useAppEvents.ts`.
   - If not using events, poll `getStatus()` while `downloadState === "downloading"` or `installing`.

3. Update main recording controls.
   - In `src/components/CaptureBar.tsx`, show `Download Parakeet` instead of `Record` when the selected model is missing and downloadable.
   - Show progress text and `Cancel` while downloading/installing.
   - Keep normal recording unavailable until `transcriptionStatus.ready`.
   - In `src/components/TranscriptSurface.tsx`, apply the same missing-model CTA for the empty-state `Start recording` button.

4. Update missing-model notice copy in `src/App.tsx`.
   - Remove copy saying recordings will capture audio without a transcript.
   - Point users to download/setup instead.

5. Update settings model list.
   - In `src/components/TranscriptionSettingsSection.tsx`, add row actions for Parakeet:
     - `Download`
     - progress
     - `Cancel`
     - retry on failure
     - installed/selected state
   - Show compact model size.
   - Show model folder/path only in Settings.
   - If Whisper is installed and Parakeet is missing, expose `Use Whisper for now` as a secondary action.

**Dependencies**
- Add Rust crates only if needed:
  - HTTP streaming: likely `reqwest`
  - hashing: `sha2`
  - archive extraction: `tar`, `bzip2`
- Keep all network/archive behavior isolated to `transcription::download`.

**Tests**
- Rust:
  - artifact metadata returns expected Parakeet paths/files
  - installed status is false for partial Parakeet directory
  - checksum failure does not install
  - cancellation does not install
  - successful extraction installs required files
  - recording start fails before capture when selected model is missing
- Frontend:
  - reducer/status update handles download states
  - missing selected model changes primary CTA to download
  - downloading state shows progress/cancel
  - failed state shows retry
  - Whisper fallback action switches provider

**Verification**
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun run bindings:rs:check`
- `bun run verify`
- Manual QA:
  - Parakeet missing: normal recording is blocked.
  - Download starts from main panel and Settings reflects the same state.
  - Cancel returns to missing state.
  - Retry works after cancel/failure.
  - Successful install selects Parakeet and enables recording.
  - Whisper fallback works when a Whisper model is installed.
