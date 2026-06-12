# Backend Module Map

This document explains what each backend module owns after the DDD split.

## `src-tauri/src/lib.rs`

Thin Tauri shell. It registers commands, manages app state, runs startup cleanup, wires the tray, and delegates work to domain modules. New business logic should usually not be added here.

## `src-tauri/src/commands`

- `mod.rs`: shared effective-path resolution from base paths plus settings.
- `threads.rs`: thread library commands (list, create, get, rename, delete, speaker rename, segment edit, search, markdown export).
- `recording.rs`: start/stop/fixture recording commands and finalization cancel.
- `indicator.rs`: indicator-window width sync and the open-main-window action used by the pill.
- `settings.rs`: settings read/update and the native folder picker.
- `system.rs`: app info, transcription status, permission status, Finder reveal, clipboard, and privacy-settings deep links.

Commands here stay thin: they resolve state handles and delegate to the owning domain. This module exists so `lib.rs` stays a small adapter.

## `src-tauri/src/app`

- `paths.rs`: discovers and stores app filesystem paths.
- `time.rs`: shared wall-clock helpers.
- `mod.rs`: exports the app context API.

Use this when changing where Just Notes stores data, threads, fixtures, or shared app-level utilities.

## `src-tauri/src/settings`

- `store.rs`: persisted user settings (`settings.json`): transcripts folder override, raw-audio toggle, markdown-copy toggle; settings state handle and effective-path resolution.
- `mod.rs`: exports the settings API.

Use this when adding a user preference or changing how the transcripts folder override works.

## `src-tauri/src/platform`

- `mod.rs`: macOS shell helpers — reveal in Finder, native folder chooser, clipboard copy, and System Settings privacy-pane links.

Keep this free of domain knowledge; it only shells out to the OS.

## `src-tauri/src/tray`

- `mod.rs`: menu bar tray icon with recording status, elapsed-time title, stop-recording action, open window, and quit.

The tray is a thin adapter: `lib.rs` injects the stop handler, and `recording` updates the status/title.

## `src-tauri/src/indicator`

- `mod.rs`: floating recording pill — an always-on-top window pinned to the right screen edge, shown while recording. The window is created wider than the pill and slid so only the pill stays on screen; hover reveals (`set_indicator_visible_width`) move the window instead of resizing it.

The indicator is a thin adapter like the tray: `recording` toggles it, and the pill webview (`indicator.html` + `src/indicator/main.ts`) syncs its width through the indicator commands.

## `src-tauri/src/ipc`

- `dto.rs`: command/event payloads shared with the frontend and exported through `ts-rs`.
- `mod.rs`: exports IPC DTOs.

Use this when frontend/backend payload shape changes are needed.

## `src-tauri/src/threads`

- `model.rs`: thread metadata (including duration and speaker labels), thread summaries/details, transcript segment model, and thread status (idle/recording/transcribing).
- `repository.rs`: thread creation, listing, loading, status/duration updates, work directory setup, markdown rendering with speaker labels, and stale-status cleanup.
- `edits.rs`: user-initiated mutations — rename thread, delete thread, rename speakers, edit segment text, and search across titles and transcript text.
- `transcript_store.rs`: transcript JSONL append/read/count/replace behavior, snippet extraction, and atomic text writes.
- `mod.rs`: exports the thread domain API.

Use this when changing thread persistence, transcript ordering, markdown output, or thread metadata behavior.

## `src-tauri/src/capture`

- `model.rs`: capture session models, prepared input, shared rolling buffers with absolute sample indexing, channel state, and capture source.
- `device.rs`: real microphone and system-loopback device preparation.
- `system_loopback/mod.rs`: system-loopback facade and CPAL device discovery.
- `system_loopback/tap.rs`: CoreAudio process tap lifecycle and permission-aware error messages.
- `system_loopback/aggregate.rs`: CoreAudio aggregate device description, attachment, and property helpers.
- `permission.rs`: microphone authorization request and non-blocking status query.
- `samples.rs`: CPAL input stream construction and sample conversion into mono rolling buffers.
- `fixture.rs`: QA fixture WAV loading and simulated audio workers.
- `mod.rs`: capture facade used by recording.

Use this when changing how audio is acquired, buffered, leveled, or fixture-driven.

## `src-tauri/src/transcription`

- `models.rs`: transcription status payloads, model path selection, and available model discovery.
- `status.rs`: current transcription readiness/status assembly.
- `runtime.rs`: Whisper model loading, initial-prompt conditioning, and raw Whisper segment transcription.
- `audio.rs`: sample/time conversion, RMS, audible-start detection, and resampling.
- `finalize.rs`: post-recording finalization — chunked re-transcription of saved WAVs, transcript replacement, cancellation registry, and status events.
- `live/worker.rs`: live transcription thread lifecycle and status/error event emission.
- `live/channel.rs`: per-source live channel state, hypothesis agreement, segment emission, and transcript append.
- `live/sink.rs`: committed live segment persistence, cross-channel bleed gating, and frontend event emission.
- `live/window.rs`: decode-window selection and silence gating.
- `text/cleanup.rs`: transcript cleanup, partial sentence handling, prefix agreement, and end-time estimation.
- `text/dedupe.rs`: duplicate suppression policy across recent transcript text, including cross-channel duplicate checks.
- `text/bleed.rs`: mic-channel suppression of speech that duplicates overlapping system audio.
- `text/dedupe/spans.rs`: repeated word span matching.
- `text/dedupe/trimming.rs`: duplicate prefix, suffix, and middle-span trimming.
- `text/words.rs`: shared word normalization, n-gram, and sentence splitting helpers.
- `mod.rs`: transcription facade used by recording and tests.

Use this when changing model status, Whisper behavior, live transcription timing, finalization, cleanup, dedupe, or audio math.

## `src-tauri/src/recording`

- `mod.rs`: recording facade and public API exports.
- `state.rs`: recorder state, active session storage, startup guard, and selected-thread reuse predicate.
- `workflow.rs`: start orchestration — thread selection, capture startup, audio sink startup, live transcription startup, and tray updates.
- `stop.rs`: stop orchestration — worker shutdown, duration persistence, markdown rendering, finalization kickoff, and the stopped event.
- `audio_sink.rs`: streams captured samples to `mic.wav`/`system.wav` during recording via a cursor over the rolling buffers.
- `meter.rs`: live meter event worker and tray elapsed-time updates.

Use this when changing the lifecycle of a recording session or how capture/transcription/thread persistence are coordinated.

## Common Change Guide

- Add a new Tauri command: declare it in the matching `commands/*.rs` file, register it in `lib.rs`, then delegate to the owning context.
- Add a frontend-visible field: update `ipc/dto.rs` or the relevant exported domain model, then run binding checks.
- Add a user preference: start in `settings`, then thread it through the commands that need it.
- Change audio capture: start in `capture`.
- Change transcript quality: start in `transcription/text` or `transcription/live`.
- Change the post-recording polish pass: start in `transcription/finalize.rs`.
- Change thread files or markdown: start in `threads`.
- Change recording start/stop behavior: start in `recording`.
