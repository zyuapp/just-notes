use super::*;

struct FixedTranscriber;

impl Transcriber for FixedTranscriber {
    fn transcribe_segments(
        &self,
        _samples_16k: &[f32],
        _prompt: &str,
        source: &str,
        speaker: &str,
    ) -> Result<Vec<TranscriptSegment>, String> {
        Ok(vec![TranscriptSegment {
            speaker: speaker.to_string(),
            source: source.to_string(),
            start_ms: 300,
            end_ms: 360,
            text: "Yeah.".to_string(),
        }])
    }
}

#[test]
fn keeps_short_voiced_backchannel_despite_quiet_utterance_padding() {
    const RATE: u32 = 16_000;
    let mut samples = vec![0.006; RATE as usize * 300 / 1_000];
    samples.extend(vec![0.04; RATE as usize * 60 / 1_000]);
    samples.extend(vec![0.006; RATE as usize * 300 / 1_000]);
    let utterance = Utterance {
        start_index: RATE as u64,
        sample_rate: RATE,
        samples,
    };
    let padded_rms = crate::transcription::rms(&utterance.samples);
    assert!(padded_rms < crate::transcription::faint_fillers::CONFIDENT_SPEECH_RMS);
    assert!(crate::transcription::source_bleed::mic_audio_is_system_dominated(padded_rms, 0.06));
    let channel = LiveChannel::new("mic", "You", true, RATE);

    let segments = channel.transcribe(vec![utterance], &FixedTranscriber, 2_000);

    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].text, "Yeah.");
    assert_eq!(segments[0].start_ms, 3_300);
    assert_eq!(segments[0].end_ms, 3_360);
}
