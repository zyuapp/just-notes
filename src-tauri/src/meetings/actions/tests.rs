use super::*;
use crate::meetings::model::Meeting;

fn meeting() -> Meeting {
    Meeting {
        id: "meeting-1".to_string(),
        calendar_id: "calendar-1".to_string(),
        title: "Design review".to_string(),
        start_at_ms: 1_000,
        end_at_ms: 2_000,
    }
}

#[test]
fn retryable_start_restores_the_prompt_after_failure() {
    let scheduler = MeetingSchedulerState::default();
    let meeting = meeting();

    let result = retryable_start::<()>(&scheduler, "start-1", &meeting, || {
        Err("Microphone unavailable".to_string())
    });

    let error = result.unwrap_err();
    assert_eq!(error.message, "Microphone unavailable");
    assert!(error.retryable);
    assert_eq!(
        scheduler.take_start_prompt("start-1"),
        Some(meeting.clone())
    );
}

#[test]
fn orchestration_rejects_a_stale_prompt_without_restoring_it() {
    let scheduler = MeetingSchedulerState::default();
    let meeting = meeting();
    assert!(scheduler.register_start_prompt("start-1".to_string(), meeting));

    let error = orchestrate_start::<()>(
        &scheduler,
        "start-1",
        |_| false,
        |_| panic!("stale meeting must not start"),
        |_, _| panic!("stale meeting must not complete"),
    )
    .unwrap_err();

    assert!(!error.retryable);
    assert_eq!(scheduler.take_start_prompt("start-1"), None);
}

#[test]
fn orchestration_restores_recorder_failures_and_skips_success_work() {
    let scheduler = MeetingSchedulerState::default();
    let meeting = meeting();
    assert!(scheduler.register_start_prompt("start-1".to_string(), meeting.clone()));
    let mut completed = false;

    let error = orchestrate_start::<()>(
        &scheduler,
        "start-1",
        |_| true,
        |_| Err("Microphone unavailable".to_string()),
        |_, _| completed = true,
    )
    .unwrap_err();

    assert!(error.retryable);
    assert!(!completed);
    assert_eq!(scheduler.take_start_prompt("start-1"), Some(meeting));
}

#[test]
fn orchestration_runs_success_work_after_consuming_the_prompt() {
    let scheduler = MeetingSchedulerState::default();
    let meeting = meeting();
    assert!(scheduler.register_start_prompt("start-1".to_string(), meeting));
    let mut completed_title = None;

    let result = orchestrate_start(
        &scheduler,
        "start-1",
        |_| true,
        |_| Ok("started"),
        |meeting, _| completed_title = Some(meeting.title.clone()),
    );

    assert_eq!(result, Ok("started"));
    assert_eq!(completed_title.as_deref(), Some("Design review"));
    assert_eq!(scheduler.take_start_prompt("start-1"), None);
}

#[test]
fn retryable_start_keeps_success_consumed() {
    let scheduler = MeetingSchedulerState::default();
    let meeting = meeting();

    assert_eq!(
        retryable_start(&scheduler, "start-1", &meeting, || Ok("started")),
        Ok("started")
    );
    assert_eq!(scheduler.take_start_prompt("start-1"), None);
}
