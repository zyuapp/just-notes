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

    assert_eq!(result.unwrap_err().message, "Microphone unavailable");
    assert_eq!(
        scheduler.take_start_prompt("start-1"),
        Some(meeting.clone())
    );
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
