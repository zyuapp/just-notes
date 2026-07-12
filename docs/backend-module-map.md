# Backend Module Map

This document explains what each backend module owns after the DDD split.

## `src-tauri/src/lib.rs`

Thin Tauri shell. It registers commands, manages app state, runs startup cleanup, wires the tray, and delegates work to domain modules. New business logic should usually not be added here.

## `src-tauri/src/commands`

- `mod.rs`: shared effective-path resolution from base paths plus settings.
- `threads.rs`: thread library commands (list, create, get, rename, archive, restore, delete, segment edit, search, markdown export).
- `recording.rs`: start/stop/fixture recording commands and finalization cancel.
- `transcription.rs`: model status plus model download start/cancel and local-model deletion.
- `settings.rs`: settings read/update plus coordination of native folder authorization, legacy import, and default storage selection.
- `system.rs`: app info, permission status, Finder reveal, clipboard, and privacy-settings deep links through public AppKit APIs.
- `meetings.rs`: calendar/notification permission status, access requests, and current meeting-prompt actions.

Commands here stay thin: they resolve state handles and delegate to the owning domain. This module exists so `lib.rs` stays a small adapter.

## `src-tauri/src/meeting_surfaces.rs`

Thin adapter that synchronizes the current meeting prompt to frontend events and the menu-bar item, and translates native meeting-start outcomes into recording events or failure notifications. Meeting eligibility and prompt lifecycle remain in the `meetings` context.

## `src-tauri/src/recording_payload.rs`

Neutral adapter that converts the recording domain's start result into the IPC payload shared by commands, tray actions, and meeting surfaces. Keeping this conversion outside those sibling adapters prevents them from depending on one another.

## `src-tauri/src/app`

- `paths.rs`: constructs the container-relative app filesystem layout, including active and archived thread directories.
- `migration.rs`: non-overwriting import from the legacy `~/.just-notes` layout after explicit folder authorization.
- `storage_gate.rs`: neutral coordination gate that serializes storage-root changes with recording and reprocessing path capture.
- `time.rs`: shared wall-clock helpers.
- `mod.rs`: exports the app context API.

Use this when changing where Just Notes stores data, threads, fixtures, or shared app-level utilities.

## `src-tauri/src/settings`

- `store.rs`: persisted user settings (`settings.json`): transcripts folder override and private security-scoped bookmark, raw-audio toggle, markdown-copy toggle; settings state handle and effective-path resolution.
- `mod.rs`: exports the settings API.

Use this when adding a user preference or changing how the transcripts folder override works.

## `src-tauri/src/platform`

- `mod.rs`: public AppKit/Foundation helpers — Finder reveal, native folder chooser and persistent security-scoped access, clipboard copy, and System Settings privacy-pane links.
- `calendar.rs`: EventKit authorization, database-change observation, calendar listing, and eligible event retrieval; `calendar/worker.rs` serializes synchronous EventKit reads and replaces timed-out workers.
- `notifications.rs`: UserNotifications permission, categories, delivery, and action callback adapter.

Keep this free of domain knowledge; it only shells out to the OS.

## `src-tauri/src/meetings`

- `model.rs`: calendar-access payloads and the internal meeting model.
- `state.rs`: prompt deduplication, current-prompt selection, and the active meeting-recording association.
- `scheduler.rs`: EventKit-change-driven and periodic calendar refresh, start/end prompt timing, refresh-health logging, and prompt-surface synchronization.
- `actions.rs`: meeting start/stop orchestration into the recording domain for notification, tray, and frontend adapters.
- `mod.rs`: meeting-context facade used by commands and app setup.

Use this when changing which calendar events qualify, when meeting reminders appear, or how their actions start and stop recordings.

## `src-tauri/src/tray`

- `mod.rs`: menu bar tray icon with contextual meeting and quick-record actions, recording status, elapsed-time title, open window, and quit.

The tray is a thin adapter: `lib.rs` injects the stop handler, and `recording` updates the status/title.

## `src-tauri/src/ipc`

- `dto.rs`: command/event payloads shared with the frontend and exported through `ts-rs`.
- `mod.rs`: exports IPC DTOs.

Use this when frontend/backend payload shape changes are needed.

## `src-tauri/src/threads`

- `model.rs`: thread metadata (including duration), thread summaries/details (including the `has_audio` flag), transcript segment model, and thread status (idle/recording/transcribing).
- `repository.rs`: active/archived listing, loading, status/duration updates, work directory setup, markdown rendering and `transcript.md` export, and stale-status cleanup.
- `create.rs`: collision-safe thread directory creation plus internal and external title handling.
- `edits.rs`: user-initiated mutations — rename, archive, restore, delete, edit segment text, and search across titles and transcript text.
- `edits/fs_move.rs`: filesystem move of a thread directory between the active and archive locations.
- `artifacts.rs`: `RecordingAudioPaths` — on-disk locations of a thread's raw `mic.wav`/`system.wav`, plus existence checks and removal.
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

- `models.rs`: transcription status payloads, Parakeet model path/catalog/selection, and local-model deletion.
- `status.rs`: current transcription readiness/status assembly, including live download state.
- `artifacts.rs`: `ModelArtifact` descriptor — Parakeet archive URL, checksum, byte size, and model directory layout.
- `download.rs`: model download lifecycle and `ModelDownloadState` (progress snapshots, cancellation, guarded local-model delete); `download/install.rs` fetches, verifies, extracts, and atomically installs; `download/snapshot.rs` holds the progress snapshot.
- `runtime.rs`: transcriber loading and the `Transcriber` trait; `runtime/parakeet.rs` holds the sherpa-onnx Parakeet model loading and segment transcription; `runtime/parakeet/segments.rs` converts model output into transcript segments.
- `audio.rs`: sample/time conversion, RMS, audible-start detection, and resampling.
- `live.rs`: the shared segmenter used by both the live and finalize paths — splits a mono stream into bounded speech utterances (energy VAD, redemption, duration cap) plus `transcribe_live_utterance`; `live/tests.rs` covers it.
- `live_worker.rs`: the live transcription worker — tails capture buffers per channel during recording, transcribes each closed utterance, emits the `transcript-update` event, and appends segments to the thread transcript.
- `finalize.rs`: on-demand re-transcription of saved WAVs through the shared segmenter — transcript replacement, cancellation registry, and status events.
- `finalize_audio.rs`: saved-WAV decoding, measured-duration, and the raw-audio retention policy a finalization pass consumes.
- `source_bleed.rs`: finalization-time, audio-level suppression of mic segments dominated by overlapping system audio; `source_bleed/profile.rs` builds per-channel RMS/envelope profiles from the WAVs.
- `text/cleanup.rs`: transcript cleanup, partial sentence handling, prefix agreement, and end-time estimation.
- `text/bleed.rs`: text-level suppression of mic speech that duplicates overlapping system audio.
- `text/words.rs`: shared word normalization, n-gram, and sentence splitting helpers.
- `mod.rs`: transcription facade used by recording and tests.

Use this when changing model status, Parakeet behavior, model download/install, the segmenter, live transcription, finalization, cleanup, cross-channel bleed, or audio math. Transcription owns both the live path (`live_worker.rs` streams segments during recording, the authoritative transcript) and the on-demand finalization pass that re-transcribes saved WAVs for cross-channel cleanup; `recording` only starts and stops them.

## `src-tauri/src/recording`

- `mod.rs`: recording facade and public API exports.
- `model.rs`: recording-owned start result before adapters translate it into an IPC payload.
- `state.rs`: recorder state, active session storage, startup guard, and selected-thread reuse predicate.
- `workflow.rs`: start orchestration — model-readiness gate, thread selection, capture startup, audio sink startup, live transcription startup, and tray updates.
- `stop.rs`: stop orchestration — worker shutdown (including live transcription), duration persistence, markdown rendering, raw-audio retention, and the stopped event. The transcript is already on disk, so stop does not re-transcribe.
- `reprocess.rs`: on-demand re-transcription entry point — checks eligibility (idle, has saved audio, not resumed) then kicks off the transcription finalization pass.
- `audio_sink.rs`: streams captured samples to `mic.wav`/`system.wav` during recording via a cursor over the rolling buffers.
- `meter.rs`: live meter event worker and tray elapsed-time updates.

Use this when changing the lifecycle of a recording session or how capture/transcription/thread persistence are coordinated.

## Common Change Guide

- Add a new Tauri command: declare it in the matching `commands/*.rs` file, register it in `lib.rs`, then delegate to the owning context.
- Add a frontend-visible field: update `ipc/dto.rs` or the relevant exported domain model, then run binding checks.
- Add a user preference: start in `settings`, then thread it through the commands that need it.
- Change meeting reminder timing or eligibility: start in `meetings`.
- Change audio capture: start in `capture`.
- Change transcript quality: start in `transcription/text` or `transcription/source_bleed`.
- Change model download/install: start in `transcription/download`.
- Change live transcription during recording: start in `transcription/live.rs` (segmenter) or `transcription/live_worker.rs` (worker).
- Change the on-demand re-transcription / polish pass: start in `transcription/finalize.rs` (kicked off by `recording/reprocess.rs`).
- Change thread files or markdown: start in `threads`.
- Change recording start/stop behavior: start in `recording`.
