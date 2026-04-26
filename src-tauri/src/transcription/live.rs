mod channel;
mod window;
mod worker;

#[cfg(test)]
pub(crate) use channel::LiveChannelState;
pub(crate) use worker::{spawn_live_transcription_thread, LiveTranscriptionThreadConfig};

const LIVE_TRANSCRIPTION_STEP_MS: u64 = 2_000;
const LIVE_TRANSCRIPTION_WINDOW_MS: u64 = 12_000;
const LIVE_TRANSCRIPTION_MAX_AGREEMENT_BUFFER_MS: u64 = 30_000;
const LIVE_TRANSCRIPTION_STABILITY_DELAY_MS: u64 = 2_000;
const LIVE_TRANSCRIPTION_POLL_MS: u64 = 250;
const LIVE_SILENCE_RMS_THRESHOLD: f32 = 0.005;
