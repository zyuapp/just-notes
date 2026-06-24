use super::*;

const RATE: u32 = 16_000;

fn samples(ms: u64, amplitude: f32) -> Vec<f32> {
    vec![amplitude; samples_for_ms(RATE, ms)]
}

fn segment(stream: &[f32]) -> Vec<Utterance> {
    let mut segmenter = LiveSegmenter::new(RATE, SegmenterConfig::live());
    let mut out = Vec::new();
    segmenter.push(0, stream, &mut out);
    segmenter.flush(&mut out);
    out
}

#[test]
fn emits_one_utterance_and_trims_trailing_silence() {
    let mut stream = samples(500, 0.1);
    stream.extend(samples(700, 0.0));

    let utterances = segment(&stream);

    assert_eq!(utterances.len(), 1);
    assert_eq!(utterances[0].start_index, 0);
    // Trailing silence is trimmed back to the speech run (500 ms).
    assert_eq!(utterances[0].samples.len(), samples_for_ms(RATE, 500));
}

#[test]
fn drops_speech_shorter_than_min_utterance() {
    let mut stream = samples(100, 0.1);
    stream.extend(samples(700, 0.0));

    assert!(segment(&stream).is_empty());
}

#[test]
fn bridges_short_pause_into_a_single_utterance() {
    let mut stream = samples(300, 0.1);
    stream.extend(samples(300, 0.0)); // gap below the redemption window
    stream.extend(samples(300, 0.1));
    stream.extend(samples(700, 0.0));

    let utterances = segment(&stream);

    assert_eq!(utterances.len(), 1);
    assert_eq!(utterances[0].start_index, 0);
}

#[test]
fn force_cuts_continuous_speech_at_the_duration_cap() {
    let stream = samples(MAX_UTTERANCE_MS + 1000, 0.1);

    let utterances = segment(&stream);

    assert_eq!(utterances.len(), 2);
    assert_eq!(utterances[0].start_index, 0);
    assert_eq!(
        utterances[0].samples.len(),
        samples_for_ms(RATE, MAX_UTTERANCE_MS)
    );
    assert_eq!(
        utterances[1].start_index as usize,
        samples_for_ms(RATE, MAX_UTTERANCE_MS)
    );
}

#[test]
fn realigns_after_dropped_samples() {
    let mut segmenter = LiveSegmenter::new(RATE, SegmenterConfig::live());
    let mut out = Vec::new();
    // Open speech with no closing silence leaves an utterance in progress.
    segmenter.push(0, &samples(500, 0.1), &mut out);
    assert!(out.is_empty());

    // A jump past the next expected index marks dropped audio: the open
    // utterance is abandoned and the next one anchors at the new index.
    let gap_start = samples_for_ms(RATE, 60_000) as u64;
    let mut resumed = samples(500, 0.1);
    resumed.extend(samples(700, 0.0));
    segmenter.push(gap_start, &resumed, &mut out);
    segmenter.flush(&mut out);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].start_index, gap_start);
}

#[test]
fn start_index_tracks_silence_skipped_before_speech() {
    let mut stream = samples(1000, 0.0);
    stream.extend(samples(500, 0.1));
    stream.extend(samples(700, 0.0));

    let utterances = segment(&stream);

    assert_eq!(utterances.len(), 1);
    assert_eq!(
        utterances[0].start_index as usize,
        samples_for_ms(RATE, 1000)
    );
}
