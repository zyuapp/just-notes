use super::{suppress_unconfirmed_mic_fillers, suppress_with_profile, SpeechProfile};
use crate::threads::TranscriptSegment;

fn segment(source: &str, start_ms: u64, end_ms: u64, text: &str) -> TranscriptSegment {
    TranscriptSegment {
        speaker: if source == "mic" { "You" } else { "Others" }.to_string(),
        source: source.to_string(),
        start_ms,
        end_ms,
        text: text.to_string(),
    }
}

/// A profile of `frames` 20 ms envelope bins at a constant level.
fn profile(level: f32, frames: usize) -> SpeechProfile {
    SpeechProfile::from_frame_levels(&vec![level; frames])
}

#[test]
fn drops_isolated_faint_filler() {
    let mic = profile(0.005, 80);
    let kept = suppress_with_profile(vec![segment("mic", 500, 900, "Okay.")], Some(&mic), 0);
    assert!(kept.is_empty());
}

#[test]
fn drops_faint_filler_even_when_real_speech_is_nearby() {
    let mic = profile(0.005, 800);
    let segments = vec![
        segment("system", 1_000, 4_000, "Let me walk you through the plan."),
        segment("mic", 6_000, 6_400, "Okay."),
    ];
    let kept = suppress_with_profile(segments, Some(&mic), 0);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].source, "system");
}

#[test]
fn drops_clusters_that_resemble_repeated_backchannels() {
    let mic = profile(0.005, 800);
    let segments = vec![
        segment("mic", 1_000, 1_400, "Okay."),
        segment("mic", 6_000, 6_400, "Mm-hmm."),
        segment("mic", 8_000, 8_240, "Yeah."),
    ];
    assert!(suppress_with_profile(segments, Some(&mic), 0).is_empty());
}

#[test]
fn keeps_loud_fillers_and_faint_substantive_speech() {
    let loud_mic = profile(0.05, 80);
    let loud = suppress_with_profile(vec![segment("mic", 500, 900, "Okay.")], Some(&loud_mic), 0);
    assert_eq!(loud.len(), 1);

    let faint_mic = profile(0.005, 80);
    let substantive = suppress_with_profile(
        vec![segment("mic", 500, 1_500, "Remember to check the logs.")],
        Some(&faint_mic),
        0,
    );
    assert_eq!(substantive.len(), 1);
}

#[test]
fn keeps_a_brief_confident_run_despite_quiet_padding() {
    let mut envelope = vec![0.005; 40];
    envelope[12..15].fill(0.04);
    let mic = SpeechProfile::from_frame_levels(&envelope);
    let kept = suppress_with_profile(vec![segment("mic", 240, 300, "Yeah.")], Some(&mic), 0);
    assert_eq!(kept.len(), 1);
}

#[test]
fn separated_confident_transients_do_not_support_a_filler() {
    let mut envelope = vec![0.005; 40];
    envelope[10..12].fill(0.04);
    envelope[13..15].fill(0.04);
    let mic = SpeechProfile::from_frame_levels(&envelope);
    let kept = suppress_with_profile(vec![segment("mic", 250, 375, "Okay.")], Some(&mic), 0);
    assert!(kept.is_empty());
}

#[test]
fn stop_filter_only_targets_mic_fillers() {
    let mic = profile(0.005, 80);
    let kept = suppress_with_profile(
        vec![
            segment("mic", 500, 900, "Okay."),
            segment("system", 500, 900, "Yeah."),
        ],
        Some(&mic),
        0,
    );
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].source, "system");
}

#[test]
fn keeps_filler_when_mic_audio_is_unreadable() {
    let kept = suppress_with_profile(vec![segment("mic", 500, 900, "Okay.")], None, 0);
    assert_eq!(kept.len(), 1);
}

#[test]
fn keeps_fillers_when_saved_audio_does_not_cover_their_windows() {
    let empty = SpeechProfile::from_frame_levels(&[]);
    let kept = suppress_with_profile(vec![segment("mic", 0, 60, "Yeah.")], Some(&empty), 0);
    assert_eq!(kept.len(), 1);

    let truncated = SpeechProfile::from_frame_levels(&[0.005; 5]);
    let kept = suppress_with_profile(
        vec![
            segment("mic", 60, 140, "Okay."),
            segment("mic", 200, 260, "Mm-hmm."),
        ],
        Some(&truncated),
        0,
    );
    assert_eq!(kept.len(), 2);
}

#[test]
fn keeps_historical_fillers_before_a_resumed_session_offset() {
    let faint_current_session = profile(0.005, 80);
    let kept = suppress_with_profile(
        vec![segment("mic", 5_000, 5_400, "Okay.")],
        Some(&faint_current_session),
        60_000,
    );
    assert_eq!(kept.len(), 1);
}

#[test]
fn translates_resumed_session_timestamps_into_local_audio_time() {
    let mut envelope = vec![0.005; 80];
    envelope[25..28].fill(0.04);
    let mic = SpeechProfile::from_frame_levels(&envelope);
    let kept = suppress_with_profile(
        vec![
            segment("mic", 60_500, 60_600, "Yeah."),
            segment("mic", 61_000, 61_400, "Okay."),
        ],
        Some(&mic),
        60_000,
    );
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].text, "Yeah.");
}

#[test]
fn decode_filter_only_targets_the_mic_channel() {
    let faint = vec![0.005; 16_000];
    let mic = suppress_unconfirmed_mic_fillers(
        vec![segment("mic", 100, 400, "Okay.")],
        &faint,
        16_000,
        "mic",
    );
    let system = suppress_unconfirmed_mic_fillers(
        vec![segment("system", 100, 400, "Okay.")],
        &faint,
        16_000,
        "system",
    );
    assert!(mic.is_empty());
    assert_eq!(system.len(), 1);
}
