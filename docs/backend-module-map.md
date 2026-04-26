# Backend Module Map

This document explains what each backend module owns after the DDD split.

## `src-tauri/src/lib.rs`

Thin Tauri shell. It registers commands, manages app state, runs startup cleanup, and delegates work to domain modules. New business logic should usually not be added here.

## `src-tauri/src/app`

- `paths.rs`: discovers and stores app filesystem paths.
- `time.rs`: shared wall-clock helpers.
- `mod.rs`: exports the app context API.

Use this when changing where Just Notes stores data, threads, fixtures, or shared app-level utilities.

## `src-tauri/src/ipc`

- `dto.rs`: command/event payloads shared with the frontend and exported through `ts-rs`.
- `mod.rs`: exports IPC DTOs.

Use this when frontend/backend payload shape changes are needed.

## `src-tauri/src/threads`

- `model.rs`: thread metadata, thread summaries/details, transcript segment model, and thread status.
- `repository.rs`: thread creation, listing, loading, status updates, work directory setup, markdown rendering, and stale-recording cleanup.
- `transcript_store.rs`: transcript JSONL append/read/count behavior and atomic text writes.
- `mod.rs`: exports the thread domain API.

Use this when changing thread persistence, transcript ordering, markdown output, or thread metadata behavior.

## `src-tauri/src/capture`

- `model.rs`: capture session models, prepared input, shared rolling buffers, channel state, and capture source.
- `device.rs`: real microphone and system-loopback device preparation.
- `system_loopback/mod.rs`: system-loopback facade and CPAL device discovery.
- `system_loopback/tap.rs`: CoreAudio process tap lifecycle.
- `system_loopback/aggregate.rs`: CoreAudio aggregate device description, attachment, and property helpers.
- `permission.rs`: microphone authorization.
- `samples.rs`: CPAL input stream construction and sample conversion into mono rolling buffers.
- `fixture.rs`: QA fixture WAV loading and simulated audio workers.
- `mod.rs`: capture facade used by recording.

Use this when changing how audio is acquired, buffered, leveled, or fixture-driven.

## `src-tauri/src/transcription`

- `models.rs`: transcription status payloads, model path selection, and available model discovery.
- `status.rs`: current transcription readiness/status assembly.
- `runtime.rs`: Whisper model loading and raw Whisper segment transcription.
- `audio.rs`: sample/time conversion, RMS, audible-start detection, and resampling.
- `live/worker.rs`: live transcription thread lifecycle and status/error event emission.
- `live/channel.rs`: per-source live channel state, hypothesis agreement, segment emission, and transcript append.
- `live/window.rs`: decode-window selection and silence gating.
- `text/cleanup.rs`: transcript cleanup, partial sentence handling, prefix agreement, and end-time estimation.
- `text/dedupe.rs`: duplicate suppression policy across recent transcript text.
- `text/dedupe/spans.rs`: repeated word span matching.
- `text/dedupe/trimming.rs`: duplicate prefix, suffix, and middle-span trimming.
- `text/words.rs`: shared word normalization and sentence splitting helpers.
- `mod.rs`: transcription facade used by recording and tests.

Use this when changing model status, Whisper behavior, live transcription timing, cleanup, dedupe, or audio math.

## `src-tauri/src/recording`

- `mod.rs`: recording facade and public API exports.
- `state.rs`: recorder state, active session storage, startup guard, and selected-thread reuse predicate.
- `workflow.rs`: start/stop orchestration, thread selection, capture startup, live transcription startup, and finalization.
- `meter.rs`: live meter event worker.

Use this when changing the lifecycle of a recording session or how capture/transcription/thread persistence are coordinated.

## Common Change Guide

- Add a new Tauri command: wire it in `lib.rs`, then delegate to the owning context.
- Add a frontend-visible field: update `ipc/dto.rs` or the relevant exported domain model, then run binding checks.
- Change audio capture: start in `capture`.
- Change transcript quality: start in `transcription/text` or `transcription/live`.
- Change thread files or markdown: start in `threads`.
- Change recording start/stop behavior: start in `recording`.
