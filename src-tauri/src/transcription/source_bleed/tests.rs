use super::profile::{max_envelope_correlation, ChannelProfile};
use super::{
    suppress_system_dominated_mic_segments_with_profiles, ECHO_CORRELATION_THRESHOLD,
    ECHO_MAX_LAG_MS, EXTREME_SYSTEM_DOMINANCE_MIC_MAX_RATIO,
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

fn profile_with_ratio(
    system: &ChannelProfile,
    start_ms: u64,
    end_ms: u64,
    ratio: f32,
    frames: usize,
) -> ChannelProfile {
    let level = system.rms(start_ms, end_ms) * ratio;
    ChannelProfile::from_envelope(&vec![level; frames])
}

#[test]
fn drops_quiet_mic_segment_that_echoes_system_audio() {
    let system_env = system_envelope();
    let mic = ChannelProfile::from_envelope(&scaled(&system_env, 0.3));
    let system = ChannelProfile::from_envelope(&system_env);
    let segments = vec![segment("system", 0, 1_000), segment("mic", 0, 1_000)];

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system, 0);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].source, "system");
}

#[test]
fn resumed_session_preserves_history_and_localizes_current_audio() {
    let system_env = system_envelope();
    let mic = ChannelProfile::from_envelope(&scaled(&system_env, 0.3));
    let system = ChannelProfile::from_envelope(&system_env);
    let segments = vec![
        segment("system", 0, 1_000),
        segment("mic", 0, 1_000),
        segment("system", 60_000, 61_000),
        segment("mic", 60_000, 61_000),
    ];

    let kept =
        suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system, 60_000);

    assert_eq!(kept.len(), 3);
    assert!(kept
        .iter()
        .any(|segment| segment.source == "mic" && segment.start_ms == 0));
    assert!(!kept
        .iter()
        .any(|segment| segment.source == "mic" && segment.start_ms == 60_000));
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

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system, 0);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].source, "system");
}

#[test]
fn keeps_quiet_mic_speech_that_does_not_track_system_audio() {
    let mic = ChannelProfile::from_envelope(&ramp_envelope());
    let system = ChannelProfile::from_envelope(&system_envelope());
    let segments = vec![segment("system", 0, 1_000), segment("mic", 0, 1_000)];

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system, 0);

    assert_eq!(kept.len(), 2);
}

#[test]
fn keeps_mic_segment_when_energy_is_competitive() {
    let system_env = system_envelope();
    let mic = ChannelProfile::from_envelope(&scaled(&system_env, 0.9));
    let system = ChannelProfile::from_envelope(&system_env);
    let segments = vec![segment("system", 0, 1_000), segment("mic", 0, 1_000)];

    assert_eq!(
        suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system, 0).len(),
        2
    );
}

#[test]
fn keeps_mic_segment_without_nearby_system_transcript() {
    let mic = ChannelProfile::from_envelope(&[0.01; 40]);
    let system = ChannelProfile::from_envelope(&system_envelope());
    let segments = vec![segment("mic", 0, 1_000)];

    assert_eq!(
        suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system, 0).len(),
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

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system, 0);

    assert_eq!(kept.len(), 2);
}

// Models the three false "You" spans at 6.1-6.9% RMS and low correlation.
#[test]
fn drops_low_correlation_segments_under_extreme_system_dominance() {
    const FRAMES: usize = 1_920;
    let system_env = (0..FRAMES)
        .map(|frame| 0.07 + 0.002 * (frame % 11) as f32)
        .collect::<Vec<_>>();
    let system = ChannelProfile::from_envelope(&system_env);
    let spans = [
        (0, 10_000, 0.061),
        (10_000, 24_000, 0.069),
        (39_000, 47_500, 0.062),
    ];
    let mut mic_env = vec![0.0; FRAMES];
    for (start_ms, end_ms, ratio) in spans {
        let level = system.rms(start_ms, end_ms) * ratio;
        mic_env[(start_ms / 25) as usize..(end_ms / 25) as usize].fill(level);
    }
    let mic = ChannelProfile::from_envelope(&mic_env);
    let segments = vec![
        segment("system", 0, 8_000),
        segment("system", 8_600, 24_000),
        segment("system", 39_000, 47_500),
        segment("mic", 0, 10_000),
        segment("mic", 10_000, 24_000),
        segment("mic", 39_000, 47_500),
    ];

    for (start_ms, end_ms, expected_ratio) in spans {
        let ratio = mic.rms(start_ms, end_ms) / system.rms(start_ms, end_ms);
        assert!((ratio - expected_ratio).abs() < 0.001, "got {ratio}");
        let correlation =
            max_envelope_correlation(&mic, &system, start_ms, end_ms, ECHO_MAX_LAG_MS);
        assert!(
            correlation < ECHO_CORRELATION_THRESHOLD,
            "got {correlation}"
        );
    }

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system, 0);

    assert_eq!(kept.len(), 3);
    assert!(kept.iter().all(|segment| segment.source == "system"));
}

#[test]
fn keeps_extremely_faint_mic_speech_without_sustained_system_speech() {
    const FRAMES: usize = 400;
    let system = ChannelProfile::from_envelope(&vec![0.08; FRAMES]);
    let mic = profile_with_ratio(
        &system,
        0,
        10_000,
        EXTREME_SYSTEM_DOMINANCE_MIC_MAX_RATIO - 0.01,
        FRAMES,
    );
    let segments = vec![segment("system", 0, 7_000), segment("mic", 0, 10_000)];

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system, 0);

    assert_eq!(kept.len(), 2);
}

// Preserve the quiet substantive-speech fixture's 0.16 mic/system ratio even
// with full playback overlap and no correlation evidence either way.
#[test]
fn keeps_quiet_mic_speech_above_extreme_dominance_cutoff() {
    const FRAMES: usize = 160;
    let system = ChannelProfile::from_envelope(&vec![0.08; FRAMES]);
    let mic = profile_with_ratio(&system, 0, 4_000, 0.16, FRAMES);
    let segments = vec![segment("system", 0, 4_000), segment("mic", 0, 4_000)];

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system, 0);

    assert_eq!(kept.len(), 2);
}

mod correlation;
