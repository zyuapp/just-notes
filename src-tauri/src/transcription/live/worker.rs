use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use tauri::{AppHandle, Emitter};

use super::{
    channel::{process_live_channel, LiveChannelState},
    sink::LiveChannelContext,
    LIVE_TRANSCRIPTION_POLL_MS, LIVE_TRANSCRIPTION_STABILITY_DELAY_MS,
    LIVE_TRANSCRIPTION_WINDOW_MS,
};
use crate::{
    capture::SharedBuffers,
    ipc::LiveTranscriptStatusPayload,
    transcription::{TranscriptionPaths, WhisperRuntime},
};

pub(crate) struct LiveTranscriptionThreadConfig {
    pub(crate) app: AppHandle,
    pub(crate) paths: TranscriptionPaths,
    pub(crate) buffers: Arc<Mutex<SharedBuffers>>,
    pub(crate) should_stop: Arc<AtomicBool>,
    pub(crate) thread_id: String,
    pub(crate) thread_dir: PathBuf,
    pub(crate) mic_sample_rate: u32,
    pub(crate) system_sample_rate: u32,
}

pub(crate) fn spawn_live_transcription_thread(
    config: LiveTranscriptionThreadConfig,
) -> Option<JoinHandle<()>> {
    let LiveTranscriptionThreadConfig {
        app,
        paths,
        buffers,
        should_stop,
        thread_id,
        thread_dir,
        mic_sample_rate,
        system_sample_rate,
    } = config;

    if !paths.model_path.is_file() {
        emit_live_status(
            &app,
            &thread_id,
            false,
            "Live transcription is disabled until the local model is installed",
        );
        return None;
    }

    Some(thread::spawn(move || {
        let config = LiveTranscriptionThreadConfig {
            app: app.clone(),
            paths,
            buffers,
            should_stop,
            thread_id: thread_id.clone(),
            thread_dir,
            mic_sample_rate,
            system_sample_rate,
        };
        if let Err(err) = run_live_transcription_loop(config) {
            let _ = app.emit(
                "live-transcript-error",
                LiveTranscriptStatusPayload {
                    thread_id,
                    active: false,
                    message: err,
                    chunk_ms: LIVE_TRANSCRIPTION_WINDOW_MS,
                    overlap_ms: LIVE_TRANSCRIPTION_STABILITY_DELAY_MS,
                },
            );
        }
    }))
}

fn run_live_transcription_loop(config: LiveTranscriptionThreadConfig) -> Result<(), String> {
    let LiveTranscriptionThreadConfig {
        app,
        paths,
        buffers,
        should_stop,
        thread_id,
        thread_dir,
        mic_sample_rate,
        system_sample_rate,
    } = config;
    let whisper = WhisperRuntime::load(&paths.model_path)?;
    emit_live_status(
        &app,
        &thread_id,
        true,
        format!("Live transcription is listening with {}", paths.model_name),
    );

    let jsonl_path = thread_dir.join("transcript.jsonl");
    let mut mic = LiveChannelState::new("mic", "You", mic_sample_rate);
    let mut system = LiveChannelState::new("system", "Others", system_sample_rate);
    let mut emitted_count = 0usize;

    let make_context = |cross_channel_tail| LiveChannelContext {
        app: &app,
        whisper: &whisper,
        buffers: &buffers,
        jsonl_path: &jsonl_path,
        thread_dir: &thread_dir,
        thread_id: &thread_id,
        cross_channel_tail,
    };

    loop {
        let stopping = should_stop.load(Ordering::Relaxed);
        let mic_context = make_context(Some(system.emitted_text_tail.clone()));
        emitted_count += process_live_channel(&mic_context, &mut mic, stopping)?;
        let system_context = make_context(None);
        emitted_count += process_live_channel(&system_context, &mut system, stopping)?;

        if stopping {
            break;
        }

        thread::sleep(Duration::from_millis(LIVE_TRANSCRIPTION_POLL_MS));
    }

    emit_live_status(
        &app,
        &thread_id,
        false,
        format!("Live transcription stopped after {emitted_count} committed segments"),
    );
    Ok(())
}

fn emit_live_status(app: &AppHandle, thread_id: &str, active: bool, message: impl Into<String>) {
    let _ = app.emit(
        "live-transcript-status",
        LiveTranscriptStatusPayload {
            thread_id: thread_id.to_string(),
            active,
            message: message.into(),
            chunk_ms: LIVE_TRANSCRIPTION_WINDOW_MS,
            overlap_ms: LIVE_TRANSCRIPTION_STABILITY_DELAY_MS,
        },
    );
}
