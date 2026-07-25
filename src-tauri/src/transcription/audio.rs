use std::path::Path;

use hound::WavReader;

pub(crate) fn samples_to_ms(samples: u64, sample_rate: u32) -> u64 {
    ((samples as f64 * 1000.0) / sample_rate as f64).floor() as u64
}

pub(crate) fn samples_for_ms(sample_rate: u32, ms: u64) -> usize {
    ((u64::from(sample_rate) * ms) / 1000) as usize
}

/// Length of a recorded WAV in milliseconds from its frame count. Returns 0 for
/// a missing file so a never-captured channel contributes no duration.
pub(crate) fn wav_duration_ms(path: &Path) -> Result<u64, String> {
    if !path.is_file() {
        return Ok(0);
    }
    let reader =
        WavReader::open(path).map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let spec = reader.spec();
    let frames = reader.len() as u64 / u64::from(spec.channels.max(1));
    Ok(samples_to_ms(frames, spec.sample_rate))
}

pub(crate) fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let square_sum = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (square_sum / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests;

pub(crate) fn resample_to_rate(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if samples.is_empty() || source_rate == target_rate {
        return samples.to_vec();
    }
    if target_rate < source_rate {
        // Decimation folds everything above the target Nyquist limit back into
        // band, so the signal must be low-passed before interpolation.
        return interpolate(
            &low_pass(samples, source_rate, target_rate),
            source_rate,
            target_rate,
        );
    }
    interpolate(samples, source_rate, target_rate)
}

fn interpolate(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
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

fn low_pass(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    let taps = low_pass_taps(source_rate, target_rate);
    let mid = taps.len() / 2;
    (0..samples.len())
        .map(|index| {
            taps.iter()
                .enumerate()
                .map(|(tap_index, tap)| {
                    let source = index as isize + tap_index as isize - mid as isize;
                    match usize::try_from(source).ok().and_then(|at| samples.get(at)) {
                        Some(sample) => sample * tap,
                        None => 0.0,
                    }
                })
                .sum()
        })
        .collect()
}

/// Blackman-windowed sinc taps cutting off just under the target Nyquist
/// limit. An odd tap count keeps the filter linear-phase with no sample
/// delay, so utterance timing is unaffected.
fn low_pass_taps(source_rate: u32, target_rate: u32) -> Vec<f32> {
    const TAP_COUNT: usize = 101;
    let cutoff = 0.45 * target_rate as f32 / source_rate as f32;
    let mid = (TAP_COUNT / 2) as isize;
    let mut taps: Vec<f32> = (0..TAP_COUNT as isize)
        .map(|tap_index| {
            let offset = (tap_index - mid) as f32;
            let sinc = if offset == 0.0 {
                2.0 * cutoff
            } else {
                (2.0 * std::f32::consts::PI * cutoff * offset).sin()
                    / (std::f32::consts::PI * offset)
            };
            let phase = tap_index as f32 / (TAP_COUNT - 1) as f32;
            let window = 0.42 - 0.5 * (2.0 * std::f32::consts::PI * phase).cos()
                + 0.08 * (4.0 * std::f32::consts::PI * phase).cos();
            sinc * window
        })
        .collect();
    let gain: f32 = taps.iter().sum();
    for tap in &mut taps {
        *tap /= gain;
    }
    taps
}
