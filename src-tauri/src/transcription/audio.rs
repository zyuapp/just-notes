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

pub(crate) fn first_audible_ms(
    samples: &[f32],
    sample_rate: u32,
    silence_threshold: f32,
) -> Option<u64> {
    let chunk_size = ((sample_rate as u64 * 100) / 1000).max(1) as usize;
    samples
        .chunks(chunk_size)
        .position(|chunk| rms(chunk) >= silence_threshold)
        .map(|chunk_index| samples_to_ms((chunk_index * chunk_size) as u64, sample_rate))
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
    use super::first_audible_ms;

    const LIVE_SILENCE_RMS_THRESHOLD: f32 = 0.005;

    #[test]
    fn first_audible_ms_skips_leading_silence() {
        let mut samples = vec![0.0; 16_000 * 3];
        samples.extend(vec![0.04; 16_000]);

        assert_eq!(
            first_audible_ms(&samples, 16_000, LIVE_SILENCE_RMS_THRESHOLD),
            Some(3_000)
        );
    }
}
