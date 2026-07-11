//! Transcript-quality harness: runs the finalize transcription pipeline on
//! local fixture recordings with known reference transcripts and holds WER,
//! missed-segment, and hallucination metrics to a committed baseline.
//!
//! Fixtures live in `~/.just-notes/quality-fixtures` (see
//! `scripts/setup-quality-fixtures.sh`); the baseline is
//! `src-tauri/quality/baseline.json`. Run via `bun run test:quality`, and
//! rerun with `UPDATE_QUALITY_BASELINE=1` to accept improved numbers.

use std::sync::atomic::AtomicBool;

use crate::app::AppPaths;
use crate::threads::TranscriptSegment;

use super::{
    finalize_audio::transcribe_wav_channel, load_transcriber,
    models::finalization_transcription_selection, suppress_cross_channel_bleed,
    suppress_system_dominated_mic_segments, Transcriber,
};

mod fixtures;
mod metrics;

use fixtures::{discover_fixtures, load_baseline, write_baseline, Baseline, Fixture};
use metrics::{evaluate, FixtureMetrics};

const WER_TOLERANCE: f32 = 0.01;

#[test]
#[ignore = "needs the installed Parakeet model and local fixtures; run via `bun run test:quality`"]
fn quality_finalize_pipeline_meets_baseline() {
    let fixtures = discover_fixtures();
    assert!(
        !fixtures.is_empty(),
        "no quality fixtures found; run `bun run test:quality:setup` first"
    );
    let transcriber = load_quality_transcriber();

    let mut results = Vec::new();
    for fixture in &fixtures {
        let segments = transcribe_fixture(&*transcriber, fixture)
            .unwrap_or_else(|err| panic!("fixture {}: {err}", fixture.name));
        results.push((
            fixture.name.clone(),
            evaluate(&fixture.reference, &segments),
        ));
    }
    print_table(&results);

    if std::env::var_os("UPDATE_QUALITY_BASELINE").is_some() {
        write_baseline(&results).expect("write quality baseline");
        println!("Baseline updated.");
        return;
    }
    let baseline = load_baseline().expect(
        "no committed baseline; rerun with UPDATE_QUALITY_BASELINE=1 to record the current numbers",
    );
    let regressions = compare(&results, &baseline);
    assert!(
        regressions.is_empty(),
        "quality regressions:\n{}",
        regressions.join("\n")
    );
}

fn load_quality_transcriber() -> Box<dyn Transcriber> {
    let paths = AppPaths::discover().expect("resolve ~/.just-notes");
    let selection = finalization_transcription_selection(&paths);
    assert!(
        selection.is_installed(),
        "Parakeet model missing at {}; download it in the app first",
        selection.model_path.display()
    );
    load_transcriber(&selection).expect("load Parakeet transcriber")
}

/// Mirrors `finalize::run_finalization`: both channels through the segmenter
/// and recognizer, sorted, then both bleed suppressors.
fn transcribe_fixture(
    transcriber: &dyn Transcriber,
    fixture: &Fixture,
) -> Result<Vec<TranscriptSegment>, String> {
    let cancel = AtomicBool::new(false);
    let mut segments =
        transcribe_wav_channel(transcriber, &fixture.mic_path, "mic", "You", &cancel)?;
    segments.extend(transcribe_wav_channel(
        transcriber,
        &fixture.system_path,
        "system",
        "Others",
        &cancel,
    )?);
    segments.sort_by(|left, right| {
        left.start_ms
            .cmp(&right.start_ms)
            .then_with(|| left.end_ms.cmp(&right.end_ms))
            .then_with(|| left.source.cmp(&right.source))
    });
    let segments = suppress_cross_channel_bleed(segments);
    suppress_system_dominated_mic_segments(segments, &fixture.mic_path, &fixture.system_path)
}

fn print_table(results: &[(String, FixtureMetrics)]) {
    println!(
        "\n{:<24} {:>7} {:>10} {:>12} {:>12}",
        "fixture", "WER", "ref words", "missed segs", "hallucinated"
    );
    for (name, metrics) in results {
        println!(
            "{:<24} {:>6.1}% {:>10} {:>9}/{:<2} {:>12}",
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
    regressions
}
