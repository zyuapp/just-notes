#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AudibleSampleSpan {
    pub(crate) decode_start_index: usize,
    pub(crate) decode_end_index: usize,
    pub(crate) audible_start_index: usize,
    pub(crate) audible_end_index: usize,
}

pub(crate) fn samples_to_ms(samples: u64, sample_rate: u32) -> u64 {
    ((samples as f64 * 1000.0) / sample_rate as f64).floor() as u64
}

pub(crate) fn ms_to_samples(ms: u64, sample_rate: u32) -> usize {
    ((ms as f64 * sample_rate as f64) / 1000.0).round() as usize
}

pub(crate) fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let square_sum = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (square_sum / samples.len() as f32).sqrt()
}

pub(crate) fn audible_sample_span(
    samples: &[f32],
    sample_rate: u32,
    silence_threshold: f32,
    pad_ms: u64,
) -> Option<AudibleSampleSpan> {
    if samples.is_empty() {
        return None;
    }

    let chunk_size = ((sample_rate as u64 * 100) / 1000).max(1) as usize;
    let mut audible_chunks = samples
        .chunks(chunk_size)
        .enumerate()
        .filter(|(_, chunk)| rms(chunk) >= silence_threshold)
        .map(|(chunk_index, _)| chunk_index);
    let first_chunk = audible_chunks.next()?;
    let last_chunk = audible_chunks.next_back().unwrap_or(first_chunk);

    let pad_samples = ms_to_samples(pad_ms, sample_rate);
    let audible_start_index = first_chunk * chunk_size;
    let audible_end_index = ((last_chunk + 1) * chunk_size).min(samples.len());
    let decode_start_index = audible_start_index.saturating_sub(pad_samples);
    let decode_end_index = audible_end_index
        .saturating_add(pad_samples)
        .min(samples.len());

    (decode_end_index > decode_start_index).then_some(AudibleSampleSpan {
        decode_start_index,
        decode_end_index,
        audible_start_index,
        audible_end_index,
    })
}

pub(crate) fn resample_to_rate(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if samples.is_empty() || source_rate == target_rate {
        return samples.to_vec();
    }

    let output_len =
        ((samples.len() as f64) * (target_rate as f64) / (source_rate as f64)).ceil() as usize;
    let ratio = source_rate as f64 / target_rate as f64;
    let mut output = Vec::with_capacity(output_len);

    for out_index in 0..output_len {
        let source_pos = out_index as f64 * ratio;
        let left = source_pos.floor() as usize;
        let right = (left + 1).min(samples.len() - 1);
        let frac = (source_pos - left as f64) as f32;
        let sample = samples[left] * (1.0 - frac) + samples[right] * frac;
        output.push(sample);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{audible_sample_span, AudibleSampleSpan};

    const LIVE_SILENCE_RMS_THRESHOLD: f32 = 0.005;

    #[test]
    fn audible_sample_span_trims_silence_with_padding() {
        let mut samples = vec![0.0; 16_000 * 2];
        samples.extend(vec![0.04; 16_000]);
        samples.extend(vec![0.0; 16_000 * 2]);

        assert_eq!(
            audible_sample_span(&samples, 16_000, LIVE_SILENCE_RMS_THRESHOLD, 250),
            Some(AudibleSampleSpan {
                decode_start_index: 28_000,
                decode_end_index: 52_000,
                audible_start_index: 32_000,
                audible_end_index: 48_000,
            })
        );
    }

    #[test]
    fn audible_sample_span_returns_none_for_silence() {
        assert_eq!(
            audible_sample_span(&vec![0.0; 16_000], 16_000, LIVE_SILENCE_RMS_THRESHOLD, 250),
            None
        );
    }

    #[test]
    fn audible_sample_span_detects_brief_speech_in_long_silence() {
        let mut samples = vec![0.0; 16_000 * 20];
        samples.extend(vec![0.04; 1_600]);
        samples.extend(vec![0.0; 16_000 * 10]);

        let span = audible_sample_span(&samples, 16_000, LIVE_SILENCE_RMS_THRESHOLD, 250).unwrap();

        assert_eq!(span.audible_start_index, 320_000);
        assert_eq!(span.audible_end_index, 321_600);
        assert_eq!(span.decode_start_index, 316_000);
        assert_eq!(span.decode_end_index, 325_600);
    }
}
