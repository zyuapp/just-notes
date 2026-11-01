use std::time::{Duration, Instant};

use super::{CallEvent, CallTracker};
use crate::{
    meetings::detection::call_app::{classify, CallApp},
    platform::microphone_activity::MicrophoneClient,
};

fn app(bundle_id: &str) -> CallApp {
    classify(
        &MicrophoneClient {
            pid: 1,
            audio_bundle_id: Some(bundle_id.to_string()),
            app: None,
        },
        |_| false,
    )
    .unwrap()
}

fn zoom() -> CallApp {
    app("us.zoom.xos")
}

fn chrome() -> CallApp {
    app("com.google.Chrome")
}

fn at(base: Instant, ms: u64) -> Instant {
    base + Duration::from_millis(ms)
}

#[test]
fn an_app_is_detected_once_after_two_stable_seconds() {
    let base = Instant::now();
    let mut tracker = CallTracker::default();
    assert!(tracker.observe(base, &[zoom()], false).is_empty());
    assert_eq!(tracker.next_deadline(false), Some(at(base, 2_000)));
    assert!(tracker
        .observe(at(base, 1_999), &[zoom()], false)
        .is_empty());
    assert_eq!(
        tracker.observe(at(base, 2_000), &[zoom()], false),
        vec![CallEvent::Detected(zoom())]
    );
    assert!(tracker
        .observe(at(base, 9_000), &[zoom()], false)
        .is_empty());
    assert_eq!(tracker.next_deadline(false), None);
}

#[test]
fn a_short_browser_voice_note_is_not_a_call() {
    let base = Instant::now();
    let mut tracker = CallTracker::default();
    tracker.observe(base, &[chrome()], false);
    assert!(tracker
        .observe(at(base, 6_000), &[chrome()], false)
        .is_empty());
    assert!(tracker.observe(at(base, 7_000), &[], false).is_empty());
    tracker.observe(at(base, 8_000), &[chrome()], false);
    assert!(tracker
        .observe(at(base, 27_999), &[chrome()], false)
        .is_empty());
    assert_eq!(
        tracker.observe(at(base, 28_000), &[chrome()], false),
        vec![CallEvent::Detected(chrome())]
    );
}

#[test]
fn a_brief_release_does_not_offer_the_same_call_again() {
    let base = Instant::now();
    let mut tracker = CallTracker::default();
    tracker.observe(base, &[zoom()], false);
    tracker.observe(at(base, 2_000), &[zoom()], false);
    tracker.observe(at(base, 3_000), &[], false);
    assert_eq!(
        tracker.observe(at(base, 6_000), &[], false),
        vec![CallEvent::Ended {
            app: zoom(),
            recorded: false
        }]
    );
    assert!(tracker
        .observe(at(base, 30_000), &[zoom()], false)
        .is_empty());
    assert!(tracker
        .observe(at(base, 40_000), &[zoom()], false)
        .is_empty());
}

#[test]
fn a_call_absent_for_a_minute_is_offered_again() {
    let base = Instant::now();
    let mut tracker = CallTracker::default();
    tracker.observe(base, &[zoom()], false);
    tracker.observe(at(base, 2_000), &[zoom()], false);
    tracker.observe(at(base, 3_000), &[], false);
    tracker.observe(at(base, 63_000), &[], false);
    tracker.observe(at(base, 64_000), &[zoom()], false);
    assert_eq!(
        tracker.observe(at(base, 66_000), &[zoom()], false),
        vec![CallEvent::Detected(zoom())]
    );
}

#[test]
fn a_call_during_a_recording_is_not_offered_and_reports_its_end() {
    let base = Instant::now();
    let mut tracker = CallTracker::default();
    assert!(tracker.observe(base, &[zoom()], true).is_empty());
    assert!(tracker.observe(at(base, 5_000), &[zoom()], true).is_empty());
    assert_eq!(tracker.next_deadline(true), None);
    tracker.observe(at(base, 6_000), &[], true);
    assert_eq!(tracker.next_deadline(true), Some(at(base, 9_000)));
    assert_eq!(
        tracker.observe(at(base, 9_000), &[], true),
        vec![CallEvent::Ended {
            app: zoom(),
            recorded: true
        }]
    );
    assert!(tracker.observe(at(base, 10_000), &[], true).is_empty());
}

#[test]
fn a_stopped_recording_does_not_reoffer_or_report_a_recorded_end() {
    let base = Instant::now();
    let mut tracker = CallTracker::default();
    tracker.observe(base, &[zoom()], true);
    tracker.observe(at(base, 2_000), &[zoom()], true);
    assert!(tracker
        .observe(at(base, 10_000), &[zoom()], false)
        .is_empty());
    tracker.observe(at(base, 11_000), &[], false);
    assert_eq!(
        tracker.observe(at(base, 14_000), &[], false),
        vec![CallEvent::Ended {
            app: zoom(),
            recorded: false
        }]
    );
}

#[test]
fn a_brief_microphone_use_during_a_recording_is_not_a_recorded_call() {
    let base = Instant::now();
    let mut tracker = CallTracker::default();
    tracker.observe(base, &[chrome()], true);
    assert_eq!(tracker.next_deadline(true), Some(at(base, 20_000)));
    tracker.observe(at(base, 1_000), &[], true);
    assert!(tracker.observe(at(base, 5_000), &[], true).is_empty());
}
