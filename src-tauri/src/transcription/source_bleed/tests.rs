use super::{suppress_system_dominated_mic_segments_with_profiles, RmsProfile, RMS_PROFILE_BIN_MS};
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

#[test]
fn drops_mic_segment_when_system_audio_dominates_same_timespan() {
    let segments = vec![
        segment("system", 1_000, 2_000, "shared system speech"),
        segment("mic", 1_100, 1_900, "misheard system speech"),
    ];
    let mic = RmsProfile::from_rms_bins(&[0.02; 30]);
    let system = RmsProfile::from_rms_bins(&[0.05; 30]);

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].source, "system");
}

#[test]
fn keeps_mic_segment_when_mic_energy_is_competitive() {
    let segments = vec![
        segment("system", 1_000, 2_000, "system speech"),
        segment("mic", 1_100, 1_900, "actual mic speech"),
    ];
    let mic = RmsProfile::from_rms_bins(&[0.04; 30]);
    let system = RmsProfile::from_rms_bins(&[0.05; 30]);

    assert_eq!(
        suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system).len(),
        2
    );
}

#[test]
fn keeps_mic_segment_without_nearby_system_transcript() {
    let segments = vec![segment(
        "mic",
        RMS_PROFILE_BIN_MS,
        RMS_PROFILE_BIN_MS * 2,
        "actual mic speech",
    )];
    let mic = RmsProfile::from_rms_bins(&[0.02; 30]);
    let system = RmsProfile::from_rms_bins(&[0.05; 30]);

    assert_eq!(
        suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system).len(),
        1
    );
}

#[test]
fn drops_system_bleed_rows_matching_real_mislabel_pattern() {
    let segments = vec![
        segment(
            "system",
            0,
            17_000,
            "today have two earner households and someone stops overtime",
        ),
        segment(
            "mic",
            18_000,
            29_000,
            "basket of goods that people can afford",
        ),
        segment(
            "system",
            21_000,
            33_000,
            "the basket of goods that people can afford",
        ),
        segment(
            "mic",
            33_000,
            43_000,
            "these inventions drive fundamental progress",
        ),
        segment(
            "system",
            34_000,
            44_000,
            "these inventions drive fundamental progress",
        ),
        segment("mic", 63_000, 76_000, "live transcription is the same way"),
        segment(
            "system",
            66_000,
            76_000,
            "what really creates jobs is invention",
        ),
    ];
    let mic = RmsProfile::from_rms_bins(&[0.02; 800]);
    let system = RmsProfile::from_rms_bins(&[0.045; 800]);

    let kept = suppress_system_dominated_mic_segments_with_profiles(segments, &mic, &system);

    assert!(kept.iter().all(|segment| segment.source == "system"));
}
