use super::{ramp_envelope, scaled, system_envelope};
use crate::transcription::source_bleed::{
    profile::{max_envelope_correlation, ChannelProfile},
    ECHO_CORRELATION_THRESHOLD, ECHO_MAX_LAG_MS,
};

#[test]
fn correlation_is_high_for_scaled_echo() {
    let system_env = system_envelope();
    let mic = ChannelProfile::from_envelope(&scaled(&system_env, 0.3));
    let system = ChannelProfile::from_envelope(&system_env);

    let correlation = max_envelope_correlation(&mic, &system, 0, 1_000, ECHO_MAX_LAG_MS);

    assert!(
        correlation >= ECHO_CORRELATION_THRESHOLD,
        "got {correlation}"
    );
}

#[test]
fn correlation_is_low_for_distinct_speech() {
    let mic = ChannelProfile::from_envelope(&ramp_envelope());
    let system = ChannelProfile::from_envelope(&system_envelope());

    let correlation = max_envelope_correlation(&mic, &system, 0, 1_000, ECHO_MAX_LAG_MS);

    assert!(
        correlation < ECHO_CORRELATION_THRESHOLD,
        "got {correlation}"
    );
}

#[test]
fn correlation_finds_delayed_echo() {
    let system_env = system_envelope();
    let mut mic_env = vec![0.0f32; 3];
    mic_env.extend(scaled(&system_env, 0.3));
    let mic = ChannelProfile::from_envelope(&mic_env);
    let system = ChannelProfile::from_envelope(&system_env);

    let correlation = max_envelope_correlation(&mic, &system, 0, 1_000, ECHO_MAX_LAG_MS);

    assert!(
        correlation >= ECHO_CORRELATION_THRESHOLD,
        "got {correlation}"
    );
}
