use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

const RATE: u32 = 16_000;

fn samples(ms: u64, amplitude: f32) -> Vec<f32> {
    vec![amplitude; samples_for_ms(RATE, ms)]
}

#[test]
fn silence_and_faint_noise_are_not_confident_speech() {
    assert!(!segment_has_confident_speech(
        &samples(500, 0.0),
        RATE,
        0,
        500
    ));
    assert!(!segment_has_confident_speech(
        &samples(500, 0.019),
        RATE,
        0,
        500
    ));
}

#[test]
fn exactly_three_confident_frames_are_enough() {
    assert!(segment_has_confident_speech(
        &samples(MIN_CONFIDENT_RUN_MS, CONFIDENT_SPEECH_RMS),
        RATE,
        0,
        MIN_CONFIDENT_RUN_MS
    ));
}

#[test]
fn two_confident_frames_are_too_short() {
    assert!(!segment_has_confident_speech(
        &samples(MIN_CONFIDENT_RUN_MS - FRAME_MS, 0.1),
        RATE,
        0,
        MIN_CONFIDENT_RUN_MS - FRAME_MS
    ));
}

#[test]
fn separated_loud_transients_do_not_accumulate() {
    let mut audio = Vec::new();
    audio.extend(samples(40, 0.1));
    audio.extend(samples(20, 0.0));
    audio.extend(samples(40, 0.1));
    assert!(!segment_has_confident_speech(&audio, RATE, 0, 100));
}

#[test]
fn loud_audio_outside_the_segment_window_does_not_rescue_it() {
    let mut audio = samples(500, 0.005);
    audio.extend(samples(300, 0.1));
    assert!(!segment_has_confident_speech(&audio, RATE, 100, 300));
}

#[test]
fn timestamp_context_recovers_a_slightly_early_word_onset() {
    let mut audio = samples(60, 0.1);
    audio.extend(samples(300, 0.0));
    assert!(segment_has_confident_speech(&audio, RATE, 40, 200));
}

#[test]
fn works_at_capture_sample_rates() {
    let rate = 48_000;
    let audio = vec![0.04; samples_for_ms(rate, 100)];
    assert!(segment_has_confident_speech(&audio, rate, 0, 100));
}

#[test]
fn invalid_rate_or_timestamps_fail_closed() {
    assert!(!segment_has_confident_speech(&[0.1; 100], 0, 0, 100));
    assert!(!segment_has_confident_speech(&[0.1; 100], RATE, 100, 100));
    assert!(!segment_has_confident_speech(&[0.1; 100], RATE, 200, 100));
}

#[test]
fn profile_windows_are_bounded_by_recorded_audio() {
    let profile = SpeechProfile::from_frame_levels(&[0.04; 4]);
    assert_eq!(
        profile.confident_speech_evidence(u64::MAX - 100, u64::MAX),
        None
    );
}

#[test]
fn profile_confirmation_includes_the_exact_threshold() {
    let profile = SpeechProfile::from_frame_levels(&[CONFIDENT_SPEECH_RMS; 3]);
    assert_eq!(profile.confident_speech_evidence(0, 60), Some(true));
}

#[test]
fn empty_or_truncated_profiles_report_missing_evidence() {
    let empty = SpeechProfile::from_frame_levels(&[]);
    assert_eq!(empty.confident_speech_evidence(0, 60), None);

    let truncated = SpeechProfile::from_frame_levels(&[0.04; 5]);
    assert_eq!(truncated.confident_speech_evidence(60, 140), None);
    assert_eq!(truncated.confident_speech_evidence(200, 260), None);
}

#[test]
fn live_samples_and_saved_wav_agree_at_the_sixty_ms_boundary() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for (confident_ms, amplitude, expected) in [
        (40, 0.04, false),
        (60, 0.04, true),
        (60, CONFIDENT_SPEECH_RMS, true),
    ] {
        let mut waveform = samples(100, 0.005);
        waveform.extend(samples(confident_ms, amplitude));
        waveform.extend(samples(100, 0.005));
        let start_ms = 100;
        let end_ms = start_ms + confident_ms;
        let live = segment_has_confident_speech(&waveform, RATE, start_ms, end_ms);

        let path = std::env::temp_dir().join(format!(
            "just-notes-filler-profile-{}-{}.wav",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("create parity wav");
        for sample in &waveform {
            writer
                .write_sample((sample * f32::from(i16::MAX)).round() as i16)
                .expect("write parity sample");
        }
        writer.finalize().expect("finalize parity wav");
        let saved = SpeechProfile::from_wav(&path)
            .expect("read parity wav")
            .confident_speech_evidence(start_ms, end_ms)
            .expect("saved segment is fully covered");

        assert_eq!(live, expected, "live result at {confident_ms} ms");
        assert_eq!(saved, live, "saved result at {confident_ms} ms");
        let _ = std::fs::remove_file(path);
    }
}
