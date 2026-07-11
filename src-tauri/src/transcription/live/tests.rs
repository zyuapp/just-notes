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
    // Trailing silence is trimmed back to the speech run plus the kept tail.
    assert_eq!(
        utterances[0].samples.len(),
        samples_for_ms(RATE, 500 + TAIL_KEEP_MS)
    );
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

// Speech at 0.01 RMS is well above typical room tone; soft speakers must
// reach the recognizer because the live transcript is authoritative.
#[test]
fn keeps_quiet_speech_above_the_noise_floor() {
    let mut stream = samples(500, 0.01);
    stream.extend(samples(700, 0.0));

    assert_eq!(segment(&stream).len(), 1);
}

// Unvoiced word onsets (/h/, /f/, /s/) sit below the gate; without pre-roll
// the recognizer never hears the first phonemes of an utterance.
#[test]
fn utterance_includes_audio_shortly_before_the_gate_opens() {
    let mut stream = samples(200, 0.004);
    stream.extend(samples(500, 0.1));
    stream.extend(samples(700, 0.0));

    let utterances = segment(&stream);

    assert_eq!(utterances.len(), 1);
    // The loud onset is at 200 ms; at least 100 ms of pre-roll should survive.
    assert!(
        utterances[0].start_index as usize <= samples_for_ms(RATE, 100),
        "utterance starts at sample {}",
        utterances[0].start_index
    );
}

// The duration cap should cut where a word is least likely to straddle the
// boundary, not at whatever sample the cap lands on.
#[test]
fn force_cut_lands_in_a_low_energy_dip() {
    let mut stream = samples(23_000, 0.1);
    stream.extend(samples(200, 0.002));
    stream.extend(samples(3_000, 0.1));
    stream.extend(samples(700, 0.0));

    let utterances = segment(&stream);

    assert!(utterances.len() >= 2);
    let boundary = utterances[0].start_index as usize + utterances[0].samples.len();
    let dip = samples_for_ms(RATE, 23_000)..=samples_for_ms(RATE, 23_200);
    assert!(
        dip.contains(&boundary),
        "cut at sample {boundary}, low-energy dip spans {dip:?}"
    );
}

// Clipped one-word replies ("yes", "no") can run under 250 ms of gated audio.
#[test]
fn keeps_a_short_single_word_reply() {
    let mut stream = samples(180, 0.1);
    stream.extend(samples(700, 0.0));

    assert_eq!(segment(&stream).len(), 1);
}

// In a noisy room the adaptive gate must rise above the room tone: steady
// noise never opens an utterance, while clearly louder speech still does.
#[test]
fn gate_rises_above_steady_room_tone() {
    let mut stream = samples(3_000, 0.004);
    stream.extend(samples(500, 0.05));
    stream.extend(samples(700, 0.0));

    let utterances = segment(&stream);

    assert_eq!(utterances.len(), 1);
    // The utterance anchors at the speech onset (minus pre-roll), not inside
    // the room tone.
    assert!(
        utterances[0].start_index as usize >= samples_for_ms(RATE, 3_000 - PRE_ROLL_MS),
        "utterance starts at sample {} inside the room tone",
        utterances[0].start_index
    );
}

#[test]
fn start_index_tracks_silence_skipped_before_speech() {
    let mut stream = samples(1000, 0.0);
    stream.extend(samples(500, 0.1));
    stream.extend(samples(700, 0.0));

    let utterances = segment(&stream);

    assert_eq!(utterances.len(), 1);
    // Skipped silence is not counted as speech, minus the kept pre-roll.
    assert_eq!(
        utterances[0].start_index as usize,
        samples_for_ms(RATE, 1000 - PRE_ROLL_MS)
    );
}
