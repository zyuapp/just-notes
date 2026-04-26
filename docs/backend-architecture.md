# Backend Architecture

The Rust backend is split around domain responsibilities rather than technical layers alone. `src-tauri/src/lib.rs` should stay a thin Tauri adapter: it wires commands, app state, setup, and command-to-domain calls. Business behavior should live in the domain modules below.

## Bounded Contexts

### App

`app` owns application paths and filesystem locations. It answers questions like where the data directory, threads directory, fixtures, and transcription model files live. Other contexts can depend on `AppPaths`, but path discovery should stay here.

### IPC

`ipc` owns payloads that cross the frontend/backend boundary. These structs are serialized to Tauri events or command responses and exported to TypeScript through `ts-rs`. If the frontend needs a shape change, start here and regenerate/check bindings.

### Threads

`threads` owns the note-thread domain: thread metadata, summaries, details, transcript segments, transcript JSONL storage, markdown rendering, and stale recording cleanup. It does not know how audio is captured or transcribed; it only persists and presents thread data.

### Capture

`capture` owns audio input. It prepares microphone and system loopback capture, handles macOS microphone permission, manages fixture audio for QA builds, converts input streams into mono samples, and keeps rolling buffers. It should not write transcript files or decide recording lifecycle.

### Transcription

`transcription` owns local speech-to-text behavior. It knows model status, Whisper runtime setup, audio math needed by live decoding, text cleanup, duplicate suppression, and live transcription workers. It reads capture buffers and appends committed transcript segments through the thread storage boundary.

### Recording

`recording` owns recording-session orchestration. It starts and stops capture, starts live transcription, emits meter updates, selects or creates a thread, and finalizes thread status/markdown when recording stops. It coordinates contexts, but it should avoid owning low-level capture, transcription, or thread persistence details.

## Dependency Direction

The intended direction is:

`lib.rs` -> `recording`, `threads`, `transcription`, `ipc`, `app`

`recording` -> `capture`, `threads`, `transcription`, `ipc`, `app`

`transcription` -> `capture` buffers, `threads` transcript storage, `ipc` event payloads

`capture` -> platform/audio libraries and transcription audio utility only for fixture timing

`threads` -> `app` paths and local filesystem

`ipc` -> domain DTO types only

Avoid dependencies that point back into `lib.rs`. If a domain module needs something from `lib.rs`, move that behavior into the appropriate context first.

## Rules Of Thumb

- Tauri commands belong in `lib.rs`; command behavior belongs in domain modules.
- `recording` coordinates workflows, but should not contain CoreAudio, Whisper, JSONL parsing, or text dedupe logic.
- `capture` produces sample buffers and levels, not transcript segments.
- `transcription` turns audio windows into committed transcript segments.
- `threads` persists and loads thread state; it should stay usable without audio devices.
- `ipc` structs are public contracts with the frontend, so changes should be deliberate and binding-checked.

