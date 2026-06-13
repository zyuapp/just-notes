use std::path::Path;

use hound::{SampleFormat, WavReader};

use crate::threads::TranscriptSegment;

const BLEED_TIME_PAD_MS: u64 = 2_000;
const RMS_PROFILE_BIN_MS: u64 = 100;
const SYSTEM_DOMINATED_MIN_SYSTEM_RMS: f32 = 0.005;
const SYSTEM_DOMINATED_MIC_MAX_RATIO: f32 = 0.65;

#[derive(Debug, Clone, Copy)]
struct RmsBin {
    square_sum: f64,
    samples: u64,
}

#[derive(Debug)]
struct RmsProfile {
    bins: Vec<RmsBin>,
}

pub(crate) fn mic_audio_is_system_dominated(mic_rms: f32, system_rms: f32) -> bool {
    system_rms >= SYSTEM_DOMINATED_MIN_SYSTEM_RMS
        && mic_rms < system_rms * SYSTEM_DOMINATED_MIC_MAX_RATIO
}

pub(crate) fn suppress_system_dominated_mic_segments(
    segments: Vec<TranscriptSegment>,
    mic_path: &Path,
    system_path: &Path,
) -> Result<Vec<TranscriptSegment>, String> {
    if !mic_path.is_file() || !system_path.is_file() {
        return Ok(segments);
    }

    let mic_profile = RmsProfile::from_wav(mic_path)?;
    let system_profile = RmsProfile::from_wav(system_path)?;
    Ok(suppress_system_dominated_mic_segments_with_profiles(
        segments,
        &mic_profile,
        &system_profile,
    ))
}

fn suppress_system_dominated_mic_segments_with_profiles(
    segments: Vec<TranscriptSegment>,
    mic_profile: &RmsProfile,
    system_profile: &RmsProfile,
) -> Vec<TranscriptSegment> {
    let system_spans = segments
        .iter()
        .filter(|segment| segment.source == "system")
        .map(|segment| (segment.start_ms, segment.end_ms))
        .collect::<Vec<_>>();

    segments
        .into_iter()
        .filter(|segment| {
            segment.source != "mic"
                || !overlaps_any_system_segment(segment, &system_spans)
                || !mic_segment_is_system_dominated(segment, mic_profile, system_profile)
        })
        .collect()
}

fn overlaps_any_system_segment(segment: &TranscriptSegment, system_spans: &[(u64, u64)]) -> bool {
    system_spans.iter().any(|(start_ms, end_ms)| {
        segment.start_ms <= end_ms.saturating_add(BLEED_TIME_PAD_MS)
            && *start_ms <= segment.end_ms.saturating_add(BLEED_TIME_PAD_MS)
    })
}

fn mic_segment_is_system_dominated(
    segment: &TranscriptSegment,
    mic_profile: &RmsProfile,
    system_profile: &RmsProfile,
) -> bool {
    let mic_rms = mic_profile.rms(segment.start_ms, segment.end_ms);
    let system_rms = system_profile.rms(segment.start_ms, segment.end_ms);
    mic_audio_is_system_dominated(mic_rms, system_rms)
}

impl RmsProfile {
    fn from_wav(path: &Path) -> Result<Self, String> {
        let mut reader = WavReader::open(path)
            .map_err(|err| format!("Failed to read {} for bleed profile: {err}", path.display()))?;
        let spec = reader.spec();
        let frames_per_bin = frames_per_bin(spec.sample_rate);
        let channels = spec.channels.max(1) as usize;
        let mut builder = RmsProfileBuilder::new(frames_per_bin);

        match spec.sample_format {
            SampleFormat::Float => {
                let samples = reader.samples::<f32>();
                add_samples_to_profile(samples, channels, &mut builder)?;
            }
            SampleFormat::Int => {
                let scale = (1i64 << (spec.bits_per_sample.saturating_sub(1))) as f32;
                let samples = reader
                    .samples::<i32>()
                    .map(|sample| sample.map(|value| value as f32 / scale));
                add_samples_to_profile(samples, channels, &mut builder)?;
            }
        }

        Ok(builder.finish())
    }

    fn rms(&self, start_ms: u64, end_ms: u64) -> f32 {
        if end_ms <= start_ms {
            return 0.0;
        }

        let start_bin = (start_ms / RMS_PROFILE_BIN_MS) as usize;
        let end_bin = end_ms.div_ceil(RMS_PROFILE_BIN_MS) as usize;
        let mut square_sum = 0.0;
        let mut samples = 0u64;

        for bin in self.bins.get(start_bin..end_bin).unwrap_or_default() {
            square_sum += bin.square_sum;
            samples += bin.samples;
        }

        if samples == 0 {
            0.0
        } else {
            (square_sum / samples as f64).sqrt() as f32
        }
    }

    #[cfg(test)]
    fn from_rms_bins(values: &[f32]) -> Self {
        Self {
            bins: values
                .iter()
                .map(|value| RmsBin {
                    square_sum: (*value as f64 * *value as f64) * 1_000.0,
                    samples: 1_000,
                })
                .collect(),
        }
    }
}

struct RmsProfileBuilder {
    bins: Vec<RmsBin>,
    frames_per_bin: u64,
    current_frame: u64,
    square_sum: f64,
    samples: u64,
}

impl RmsProfileBuilder {
    fn new(frames_per_bin: u64) -> Self {
        Self {
            bins: Vec::new(),
            frames_per_bin,
            current_frame: 0,
            square_sum: 0.0,
            samples: 0,
        }
    }

    fn add_frame(&mut self, sample: f32) {
        self.square_sum += sample as f64 * sample as f64;
        self.samples += 1;
        self.current_frame += 1;
        if self.current_frame >= self.frames_per_bin {
            self.flush();
        }
    }

    fn finish(mut self) -> RmsProfile {
        self.flush();
        RmsProfile { bins: self.bins }
    }

    fn flush(&mut self) {
        if self.samples == 0 {
            return;
        }
        self.bins.push(RmsBin {
            square_sum: self.square_sum,
            samples: self.samples,
        });
        self.current_frame = 0;
        self.square_sum = 0.0;
        self.samples = 0;
    }
}

fn frames_per_bin(sample_rate: u32) -> u64 {
    ((sample_rate as u64 * RMS_PROFILE_BIN_MS) / 1000).max(1)
}

fn add_samples_to_profile<T, E>(
    samples: impl Iterator<Item = Result<T, E>>,
    channels: usize,
    builder: &mut RmsProfileBuilder,
) -> Result<(), String>
where
    T: Copy + Into<f32>,
    E: std::fmt::Display,
{
    let mut frame = Vec::with_capacity(channels);
    for sample in samples {
        frame.push(sample.map_err(|err| format!("Failed to decode bleed profile audio: {err}"))?);
        if frame.len() == channels {
            let mono = frame.iter().map(|value| (*value).into()).sum::<f32>() / channels as f32;
            builder.add_frame(mono);
            frame.clear();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        suppress_system_dominated_mic_segments_with_profiles, RmsProfile, RMS_PROFILE_BIN_MS,
    };
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
}
