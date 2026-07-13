//! Transcript-quality harness: runs both transcription pipelines — the live
//! path (authoritative transcript) and the finalize/reprocess path — on local
//! fixture recordings with known reference transcripts and holds WER,
//! missed-segment, and hallucination metrics to a committed baseline.
//!
//! Fixtures live in `~/.just-notes/quality-fixtures` (see
//! `scripts/setup-quality-fixtures.sh`). The model is read from the sandbox
//! app-data directory, or `JUST_NOTES_QUALITY_DATA_DIR` when set. The baseline is
//! `src-tauri/quality/baseline.json`. Run via `bun run test:quality`, and
//! rerun with `UPDATE_QUALITY_BASELINE=1` to accept improved numbers.
//!
//! The suppressor chain is guarded from both directions: fixtures 6, 9, and 12
//! measure phantom segments that must stay removed, while fixtures 10 and 11
//! measure genuine quiet mic speech that must stay kept. A change to gating or
//! suppression has to improve one side without regressing the other.
//!
//! Accepted residuals, recorded as regression floors rather than endorsed
//! numbers:
//! - `7-quiet-with-room-tone@live` (~50% WER): whisper-level speech ~6 dB
//!   over a noise floor; the finalize pass (5.9%) is the recovery path.
//! - `10-quiet-backchannels` / `11-quiet-replies-over-bleed`: voiced-quiet
//!   interjections and faint substantive replies spoken over correlated
//!   bleed are partly lost — they ride inside bleed-dominated utterances
//!   that the energy suppressor removes wholesale. Known recall gap;
//!   improving it must not resurrect the phantom segments of fixture 9.

use std::{collections::BTreeSet, sync::atomic::AtomicBool};

use crate::app::AppPaths;
use crate::threads::TranscriptSegment;

use super::{
    finalize_audio::transcribe_wav_channel, load_transcriber,
    models::finalization_transcription_selection, suppress_cross_channel_bleed,
    suppress_isolated_faint_fillers, suppress_system_dominated_mic_segments, ChannelRole,
    SegmenterConfig, Transcriber,
};

mod fixtures;
mod metrics;
mod paths;
#[cfg(test)]
mod tests;

use fixtures::{discover_fixtures, load_baseline, write_baseline, Baseline, Fixture};
use metrics::{evaluate, hallucinated_segments, FixtureMetrics};

const WER_TOLERANCE: f32 = 0.01;
pub(super) const SUPPORTED_MODE_NAMES: [&str; 2] = ["live", "finalize"];

/// Both pipelines share the recognizer and suppressors; they differ in the
/// segmenter gate. "live" mirrors the live-to-stop flow that produces the
/// transcript users keep; "finalize" mirrors manual reprocessing.
fn modes() -> [(&'static str, SegmenterConfig); 2] {
    [
        (SUPPORTED_MODE_NAMES[0], SegmenterConfig::live()),
        (SUPPORTED_MODE_NAMES[1], SegmenterConfig::finalize()),
    ]
}

#[test]
#[ignore = "needs the installed Parakeet model and local fixtures; run via `bun run test:quality`"]
fn quality_pipelines_meet_baseline() {
    let fixtures = discover_fixtures();
    assert!(
        !fixtures.is_empty(),
        "no quality fixtures found; run `bun run test:quality:setup` first"
    );
    let transcriber = load_quality_transcriber();

    let mut results = Vec::new();
    for fixture in &fixtures {
        for (mode, config) in modes() {
            if !fixture.runs_mode(mode) {
                continue;
            }
            let segments = transcribe_fixture(&*transcriber, fixture, config)
                .unwrap_or_else(|err| panic!("fixture {} ({mode}): {err}", fixture.name));
            let name = format!("{}@{mode}", fixture.name);
            print_hallucinations(&name, fixture, &segments);
            results.push((name, evaluate(&fixture.reference, &segments)));
        }
    }
    print_table(&results);

    let baseline = load_baseline();
    if std::env::var_os("UPDATE_QUALITY_BASELINE").is_some() {
        if let Some(existing) = &baseline {
            let missing = missing_baseline_cases(&results, existing);
            assert!(
                missing.is_empty(),
                "refusing to update from incomplete fixtures:\n{}",
                missing.join("\n")
            );
        }
        write_baseline(&results).expect("write quality baseline");
        println!("Baseline updated.");
        return;
    }
    let baseline = baseline.expect(
        "no committed baseline; rerun with UPDATE_QUALITY_BASELINE=1 to record the current numbers",
    );
    let regressions = compare(&results, &baseline);
    assert!(
        regressions.is_empty(),
        "quality regressions:\n{}",
        regressions.join("\n")
    );
}

fn print_hallucinations(name: &str, fixture: &Fixture, segments: &[TranscriptSegment]) {
    for segment in hallucinated_segments(&fixture.reference, segments) {
        println!(
            "hallucination {name}: {} {}-{} ms {:?}",
            segment.source, segment.start_ms, segment.end_ms, segment.text
        );
    }
}

fn load_quality_transcriber() -> Box<dyn Transcriber> {
    let paths = AppPaths::from_data_dir(paths::quality_app_data_dir_from_env());
    let selection = finalization_transcription_selection(&paths);
    assert!(
        selection.is_installed(),
        "Parakeet model missing at {}; download it in the app first",
        selection.model_path.display()
    );
    load_transcriber(&selection).expect("load Parakeet transcriber")
}

/// Both channels through the segmenter and recognizer, sorted, then both
/// bleed suppressors — the shape of `finalize::run_finalization` and of the
/// live worker followed by the stop-time polish.
fn transcribe_fixture(
    transcriber: &dyn Transcriber,
    fixture: &Fixture,
    config: SegmenterConfig,
) -> Result<Vec<TranscriptSegment>, String> {
    let cancel = AtomicBool::new(false);
    let mut segments = transcribe_wav_channel(
        transcriber,
        &fixture.mic_path,
        ChannelRole {
            source: "mic",
            speaker: "You",
        },
        &cancel,
        config,
    )?;
    segments.extend(transcribe_wav_channel(
        transcriber,
        &fixture.system_path,
        ChannelRole {
            source: "system",
            speaker: "Others",
        },
        &cancel,
        config,
    )?);
    segments.sort_by(|left, right| {
        left.start_ms
            .cmp(&right.start_ms)
            .then_with(|| left.end_ms.cmp(&right.end_ms))
            .then_with(|| left.source.cmp(&right.source))
    });
    let segments = suppress_cross_channel_bleed(segments);
    let segments =
        suppress_system_dominated_mic_segments(segments, &fixture.mic_path, &fixture.system_path)?;
    suppress_isolated_faint_fillers(segments, &fixture.mic_path, &fixture.system_path)
}

fn print_table(results: &[(String, FixtureMetrics)]) {
    println!(
        "\n{:<32} {:>7} {:>10} {:>12} {:>12}",
        "fixture", "WER", "ref words", "missed segs", "hallucinated"
    );
    for (name, metrics) in results {
        println!(
            "{:<32} {:>6.1}% {:>10} {:>9}/{:<2} {:>12}",
            name,
            metrics.wer * 100.0,
            metrics.reference_words,
            metrics.missed_segments,
            metrics.reference_segments,
            metrics.hallucinated_segments
        );
    }
    println!();
}

fn compare(results: &[(String, FixtureMetrics)], baseline: &Baseline) -> Vec<String> {
    let mut regressions = Vec::new();
    for (name, metrics) in results {
        let Some(entry) = baseline.get(name) else {
            regressions.push(format!(
                "{name}: not in baseline; rerun with UPDATE_QUALITY_BASELINE=1"
            ));
            continue;
        };
        if metrics.wer > entry.wer + WER_TOLERANCE {
            regressions.push(format!(
                "{name}: WER {:.1}% exceeds baseline {:.1}%",
                metrics.wer * 100.0,
                entry.wer * 100.0
            ));
        }
        if metrics.missed_segments > entry.missed_segments {
            regressions.push(format!(
                "{name}: {} missed segments exceeds baseline {}",
                metrics.missed_segments, entry.missed_segments
            ));
        }
        if metrics.hallucinated_segments > entry.hallucinated_segments {
            regressions.push(format!(
                "{name}: {} hallucinated segments exceeds baseline {}",
                metrics.hallucinated_segments, entry.hallucinated_segments
            ));
        }
    }
    regressions.extend(missing_baseline_cases(results, baseline));
    regressions
}

fn missing_baseline_cases(
    results: &[(String, FixtureMetrics)],
    baseline: &Baseline,
) -> Vec<String> {
    let result_names = results
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<BTreeSet<_>>();
    baseline
        .keys()
        .filter(|name| !result_names.contains(name.as_str()))
        .map(|name| {
            format!("{name}: baseline case was not discovered; rerun `bun run test:quality:setup`")
        })
        .collect()
}
