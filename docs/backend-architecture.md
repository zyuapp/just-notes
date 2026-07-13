# Backend Architecture

The Rust backend is split around domain responsibilities rather than technical layers alone. `src-tauri/src/lib.rs` should stay a thin Tauri adapter: it wires state, setup, the tray, and command registration. Command declarations live in the `commands` module (also part of the adapter layer), and business behavior lives in the domain modules below.

`meeting_surfaces` is a small adapter that projects the meetings domain's current prompt into frontend events and the menu bar without making the domain depend on either surface.

`legacy_import` is an application workflow service. It owns the temporary folder grants and preview session, coordinates import against recording/finalization activity, and delegates recording validation and installation to `threads`.

## Bounded Contexts

### App

`app` owns application paths, filesystem locations, storage-operation coordination, and discovery of the legacy unsandboxed data layout. It builds the internal layout from Tauri's platform-provided app data directory so Mac App Store builds remain inside their sandbox container. Other contexts can depend on `AppPaths`, but thread merge policy belongs to `threads`.

### Settings

`settings` owns persisted user preferences. Transcript storage is intentionally fixed to the app container and is not a user preference. It may depend on `app` only.

### Platform

`platform` owns public macOS integration: Finder reveal, the AppKit folder chooser and security-scoped URL lifetime, clipboard copy, System Settings deep links, EventKit access, and native actionable notifications. It must not depend on any domain module.

### Meetings

`meetings` owns calendar-driven recording reminders. It filters eligible events, deduplicates start prompts, exposes the current prompt to user-facing adapters, associates a meeting-started recording with its event, schedules end prompts, and handles prompt actions. It coordinates `platform`, `recording`, and `settings` without putting meeting policy into those contexts.

### Tray

`tray` owns the menu bar item: contextual and quick recording actions, status text, elapsed-time title, and the open/quit menu. It is a thin adapter over Tauri's tray API; `lib.rs` injects behavior and meeting prompt data, while `recording` pushes status updates into it.

### IPC

`ipc` owns payloads that cross the frontend/backend boundary. These structs are serialized to Tauri events or command responses and exported to TypeScript through `ts-rs`. If the frontend needs a shape change, start here and regenerate/check bindings.

### Threads

`threads` owns the note-thread domain: thread metadata, summaries, details, transcript segments, transcript JSONL storage, markdown rendering, user edits (rename, delete, segment text, search), stale-status cleanup, and merge import of legacy recordings. The importer skips exact duplicates, preserves ID conflicts as separate copies, and rolls back newly installed items on failure. It does not know how audio is captured or transcribed; it only persists and presents thread data.

### Capture

`capture` owns audio input. It prepares microphone and system loopback capture, handles macOS microphone permission, manages fixture audio for QA builds, converts input streams into mono samples, and keeps rolling buffers with absolute sample indexing. It should not write transcript files or decide recording lifecycle.

### Transcription

`transcription` owns local speech-to-text behavior. It knows model status, the Parakeet (sherpa-onnx) runtime setup, model download/installation, audio math, the speech segmenter, text cleanup, duplicate and cross-channel bleed suppression, the live transcription worker that streams segments during recording, and the on-demand finalization pass that re-transcribes saved audio. It reads capture buffers and writes transcript segments through the thread storage boundary.

### Recording

`recording` owns recording-session orchestration. It starts and stops capture, streams raw audio to disk, emits meter updates, selects or creates a thread, persists duration, starts and stops live transcription, kicks off on-demand re-transcription, and updates the tray. It coordinates contexts, but it should avoid owning low-level capture, transcription, or thread persistence details.

## Dependency Direction

The intended direction is:

`lib.rs` -> `commands`, `legacy_import`, `meetings`, `recording`, `threads`, `transcription`, `settings`, `tray`, `ipc`, `app`

`commands` -> any domain it adapts, but no business logic of its own

`legacy_import` -> `app`, `platform`, `recording`, `threads`, `transcription`

`recording` -> `capture`, `threads`, `transcription`, `settings`, `tray`, `ipc`, `app`

`meetings` -> `app`, `platform`, `recording`, `settings`

`transcription` -> `capture` buffers, `threads` transcript storage, `ipc` event payloads

`capture` -> platform/audio libraries and transcription audio utility only for fixture timing

`threads` -> `app` paths and local filesystem

`settings` -> `app` paths only

`platform`, `tray` -> no domain modules

`ipc` -> domain DTO types only

Avoid dependencies that point back into `lib.rs`. If a domain module needs something from `lib.rs`, move that behavior into the appropriate context first.

The `bun run verify` command runs `scripts/check-rust-domain-boundaries.sh` to guard the most important dependency rules. If the script blocks a change, update the domain design first rather than bypassing the guard.

## Rules Of Thumb

- Tauri command declarations belong in the `commands` adapter module and get registered in `lib.rs`; command behavior belongs in domain modules.
- `recording` coordinates workflows, but should not contain CoreAudio, model decoding, JSONL parsing, or text dedupe logic.
- `capture` produces sample buffers and levels, not transcript segments.
- `transcription` turns audio windows into committed transcript segments.
- `threads` persists and loads thread state; it should stay usable without audio devices.
- `settings` is the only writer of the current `settings.json`; app-container paths come from `AppPaths`.
- `ipc` structs are public contracts with the frontend, so changes should be deliberate and binding-checked.
