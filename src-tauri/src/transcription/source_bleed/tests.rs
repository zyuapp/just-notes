use super::profile::{max_envelope_correlation, ChannelProfile};
use super::{
    suppress_system_dominated_mic_segments_with_profiles, ECHO_CORRELATION_THRESHOLD,
    ECHO_MAX_LAG_MS,
};
use crate::threads::TranscriptSegment;

fn segment(source: &str, start_ms: u64, end_ms: u64) -> TranscriptSegment {
    TranscriptSegment {
        speaker: if source == "mic" { "You" } else { "Others" }.to_string(),
        source: source.to_string(),
        start_ms,
        end_ms,
        text: "spoken words".to_string(),
    }
}

// Forty 25 ms frames (one second) of speech-like bursts separated by gaps.
fn system_envelope() -> Vec<f32> {
    (0..40)
        .map(|frame| {
            let phase = frame % 10;
            if phase < 5 {
                0.04 + 0.02 * phase as f32
            } else {
                0.0
            }
        })
        .collect()
}

fn scaled(envelope: &[f32], factor: f32) -> Vec<f32> {
    envelope.iter().map(|value| value * factor).collect()
}

fn ramp_envelope() -> Vec<f32> {
    (0..40).map(|frame| 0.01 + 0.0004 * frame as f32).collect()
}

#[test]
fn drops_quiet_mic_segment_that_echoes_system_audio() {
    let system_env = system_envelope();
    let mic = ChannelProfile::from_envelope(&scaled(&system_env, 0.3));
    let system = ChannelProfile::from_envelope(&system_env);
    let segments = vec![segment("system", 0, 1_000), segment("mic", 0, 1_000)];

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].source, "system");
}

// The signature of speaker bleed decoding as a phantom backchannel: a short
// quiet "mm-hmm"-length mic segment whose surrounding audio tracks the
// system channel. The correlation window widens around short segments, so
// the drop still rests on a full evidence window.
#[test]
fn drops_short_quiet_echo_using_surrounding_audio() {
    let system_env = system_envelope();
    let mic = ChannelProfile::from_envelope(&scaled(&system_env, 0.3));
    let system = ChannelProfile::from_envelope(&system_env);
    let segments = vec![segment("system", 0, 1_000), segment("mic", 300, 700)];

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].source, "system");
}

#[test]
fn keeps_quiet_mic_speech_that_does_not_track_system_audio() {
    let mic = ChannelProfile::from_envelope(&ramp_envelope());
    let system = ChannelProfile::from_envelope(&system_envelope());
    let segments = vec![segment("system", 0, 1_000), segment("mic", 0, 1_000)];

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system);

    assert_eq!(kept.len(), 2);
}

#[test]
fn keeps_mic_segment_when_energy_is_competitive() {
    let system_env = system_envelope();
    let mic = ChannelProfile::from_envelope(&scaled(&system_env, 0.9));
    let system = ChannelProfile::from_envelope(&system_env);
    let segments = vec![segment("system", 0, 1_000), segment("mic", 0, 1_000)];

    assert_eq!(
        suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system).len(),
        2
    );
}

#[test]
fn keeps_mic_segment_without_nearby_system_transcript() {
    let mic = ChannelProfile::from_envelope(&[0.01; 40]);
    let system = ChannelProfile::from_envelope(&system_envelope());
    let segments = vec![segment("mic", 0, 1_000)];

    assert_eq!(
        suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system).len(),
        1
    );
}

// Six 25 ms frames give the lag search 41 chances to hit a spurious match;
// short quiet interjections during loud playback need stronger evidence than
// a correlation measured on so few frames.
#[test]
fn keeps_short_quiet_mic_segment_despite_chance_envelope_match() {
    let mic_env = [0.01, 0.02, 0.03, 0.03, 0.02, 0.01];
    let mic = ChannelProfile::from_envelope(&mic_env);
    let system = ChannelProfile::from_envelope(&system_envelope());
    let segments = vec![segment("system", 0, 1_000), segment("mic", 0, 150)];

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system);

    assert_eq!(kept.len(), 2);
}

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
