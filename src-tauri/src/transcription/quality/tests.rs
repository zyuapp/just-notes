use super::fixtures::{validate_modes, BaselineEntry};
use super::{compare, missing_baseline_cases, Baseline, FixtureMetrics};

fn metrics() -> FixtureMetrics {
    FixtureMetrics {
        wer: 0.0,
        reference_words: 0,
        missed_segments: 0,
        reference_segments: 0,
        hallucinated_segments: 0,
    }
}

fn baseline_entry() -> BaselineEntry {
    BaselineEntry {
        wer: 0.0,
        missed_segments: 0,
        hallucinated_segments: 0,
    }
}

#[test]
fn compare_rejects_baseline_cases_missing_from_local_fixtures() {
    let results = vec![("1-clean-single@live".to_string(), metrics())];
    let baseline = Baseline::from([
        ("1-clean-single@live".to_string(), baseline_entry()),
        ("12-transient-mic-spike@live".to_string(), baseline_entry()),
    ]);

    assert_eq!(
        compare(&results, &baseline),
        vec![
            "12-transient-mic-spike@live: baseline case was not discovered; rerun \
             `bun run test:quality:setup`"
                .to_string()
        ]
    );
}

#[test]
fn baseline_update_validation_rejects_missing_existing_cases() {
    let results = vec![("1-clean-single@live".to_string(), metrics())];
    let baseline = Baseline::from([
        ("1-clean-single@live".to_string(), baseline_entry()),
        ("12-transient-mic-spike@live".to_string(), baseline_entry()),
    ]);

    assert_eq!(missing_baseline_cases(&results, &baseline).len(), 1);
}

#[test]
fn fixture_modes_reject_empty_and_unknown_lists() {
    assert_eq!(
        validate_modes(Some(Vec::new())).unwrap_err(),
        "fixture modes must not be empty"
    );
    assert_eq!(
        validate_modes(Some(vec!["lvie".to_string()])).unwrap_err(),
        "unsupported fixture mode \"lvie\""
    );
}

#[test]
fn fixture_modes_accept_supported_or_unspecified_lists() {
    assert_eq!(validate_modes(None).unwrap(), None);
    assert_eq!(
        validate_modes(Some(vec!["live".to_string()])).unwrap(),
        Some(vec!["live".to_string()])
    );
}
