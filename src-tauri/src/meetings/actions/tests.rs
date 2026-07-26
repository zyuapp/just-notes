use super::*;
use crate::meetings::model::Meeting;

fn meeting() -> Meeting {
    Meeting {
        id: "meeting-1".to_string(),
        calendar_id: "calendar-1".to_string(),
        title: "Design review".to_string(),
        start_at_ms: 1_000,
        end_at_ms: 2_000,
        attendees: vec!["Alice".to_string()],
    }
}

fn prompt(auto_start: bool) -> super::super::model::MeetingPrompt {
    super::super::model::MeetingPrompt {
        request_id: "start-1".to_string(),
        meeting: meeting(),
        auto_start,
        auto_start_after_ms: auto_start.then_some(1_000),
        auto_start_revision: 0,
    }
}

#[test]
fn a_meeting_hands_the_recording_context_its_calendar_facts() {
    let scheduled = scheduled_meeting(&meeting());

    assert_eq!(scheduled.title, "Design review");
    assert_eq!(scheduled.event_id, "meeting-1");
    assert_eq!(scheduled.calendar_id, "calendar-1");
    assert_eq!(scheduled.attendees, vec!["Alice"]);
    assert_eq!(scheduled.start_at_ms, 1_000);
    assert_eq!(scheduled.end_at_ms, 2_000);
}

#[test]
fn retryable_start_restores_the_prompt_after_failure() {
    let scheduler = MeetingSchedulerState::default();
    let prompt = prompt(true);

    let result = retryable_start::<()>(&scheduler, &prompt, || {
        Err("Microphone unavailable".to_string())
    });

    let error = result.unwrap_err();
    assert_eq!(error.message, "Microphone unavailable");
    assert!(error.retryable);
    assert_eq!(scheduler.take_start_prompt("start-1"), Some(prompt));
}

#[test]
fn orchestration_rejects_a_stale_prompt_without_restoring_it() {
    let scheduler = MeetingSchedulerState::default();
    let meeting = meeting();
    assert!(scheduler.upsert_start_prompt("start-1".to_string(), meeting, false));

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
    assert!(scheduler.upsert_start_prompt("start-1".to_string(), meeting.clone(), false));
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
    assert_eq!(scheduler.take_start_prompt("start-1"), Some(prompt(false)));
}

#[test]
fn orchestration_runs_success_work_after_consuming_the_prompt() {
    let scheduler = MeetingSchedulerState::default();
    let meeting = meeting();
    assert!(scheduler.upsert_start_prompt("start-1".to_string(), meeting, true));
    let mut completed_title = None;

    let result = orchestrate_start(
        &scheduler,
        "start-1",
        |_| true,
        |_| Ok("started"),
        |prompt, _| completed_title = Some(prompt.meeting.title.clone()),
    );

    assert_eq!(result, Ok("started"));
    assert_eq!(completed_title.as_deref(), Some("Design review"));
    assert_eq!(scheduler.take_start_prompt("start-1"), None);
}

#[test]
fn retryable_start_keeps_success_consumed() {
    let scheduler = MeetingSchedulerState::default();
    let prompt = prompt(false);

    assert_eq!(
        retryable_start(&scheduler, &prompt, || Ok("started")),
        Ok("started")
    );
    assert_eq!(scheduler.take_start_prompt("start-1"), None);
}

#[test]
fn settings_changes_cancel_consumed_automatic_prompts() {
    let scheduler = MeetingSchedulerState::default();
    assert!(scheduler.upsert_start_prompt("start-1".to_string(), meeting(), true));
    let prompt = scheduler.take_start_prompt("start-1").unwrap();
    scheduler.clear_start_prompts();
    let result = scheduler.with_auto_start_consent(prompt.auto_start_revision, || Ok("started"));
    assert_eq!(result, Err("Automatic recording was disabled".to_string()));
    scheduler.restore_start_prompt(prompt);
    assert_eq!(scheduler.take_start_prompt("start-1"), None);
}
