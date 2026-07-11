use super::resample_to_rate;

fn sine(frequency: f32, amplitude: f32, sample_rate: u32, ms: u64) -> Vec<f32> {
    let len = ((u64::from(sample_rate) * ms) / 1000) as usize;
    (0..len)
        .map(|index| {
            let phase = 2.0 * std::f32::consts::PI * frequency * index as f32 / sample_rate as f32;
            amplitude * phase.sin()
        })
        .collect()
}

/// Amplitude of the `frequency` component, measured with the Goertzel
/// algorithm so tests need no FFT dependency.
fn tone_amplitude(samples: &[f32], sample_rate: u32, frequency: f32) -> f32 {
    let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate as f32;
    let coefficient = 2.0 * omega.cos();
    let (mut prev, mut prev2) = (0.0f32, 0.0f32);
    for &sample in samples {
        let current = sample + coefficient * prev - prev2;
        prev2 = prev;
        prev = current;
    }
    let power = (prev * prev + prev2 * prev2 - coefficient * prev * prev2).max(0.0);
    2.0 * power.sqrt() / samples.len() as f32
}

#[test]
fn resampling_preserves_in_band_tones() {
    for source_rate in [48_000, 44_100] {
        let tone = sine(1_000.0, 0.5, source_rate, 1_000);
        let resampled = resample_to_rate(&tone, source_rate, 16_000);
        let amplitude = tone_amplitude(&resampled, 16_000, 1_000.0);
        assert!(
            (0.4..=0.6).contains(&amplitude),
            "1 kHz tone from {source_rate} Hz came through at amplitude {amplitude}"
        );
    }
}

// A 10 kHz tone sits above the 8 kHz Nyquist limit of the model's 16 kHz
// input. A correct decimator removes it; without a low-pass stage it folds to
// 6 kHz inside the speech band and corrupts what the recognizer hears.
#[test]
#[ignore = "resample_to_rate has no anti-alias filter; quality-harness red test"]
fn resampling_suppresses_tones_above_target_nyquist() {
    for source_rate in [48_000, 44_100] {
        let tone = sine(10_000.0, 0.5, source_rate, 1_000);
        let resampled = resample_to_rate(&tone, source_rate, 16_000);
        let alias = tone_amplitude(&resampled, 16_000, 6_000.0);
        assert!(
            alias < 0.05,
            "10 kHz tone from {source_rate} Hz aliased into 6 kHz at amplitude {alias}"
        );
    }
}
