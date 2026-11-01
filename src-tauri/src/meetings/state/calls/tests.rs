use crate::meetings::{model::Meeting, state::MeetingSchedulerState};

fn meeting() -> Meeting {
    Meeting {
        id: "meeting-1".to_string(),
        calendar_id: "calendar-1".to_string(),
        title: "Design review".to_string(),
        start_at_ms: 100_000,
        end_at_ms: 200_000,
        attendees: vec!["Ada".to_string()],
    }
}

#[test]
fn a_skipped_meeting_owns_its_call_until_it_ends() {
    let state = MeetingSchedulerState::default();
    state.upsert_start_prompt("start-1".to_string(), meeting(), false);
    assert!(!state.calendar_owns_call(&meeting(), 100_000));
    assert!(state.decline_start_prompt("start-1").is_some());
    assert!(state.calendar_owns_call(&meeting(), 150_000));
    assert!(!state.calendar_owns_call(&meeting(), 200_000));
}

#[test]
fn an_armed_automatic_start_owns_its_call_until_it_is_late() {
    let state = MeetingSchedulerState::default();
    state.upsert_start_prompt("start-1".to_string(), meeting(), true);
    assert!(!state.calendar_owns_call(&meeting(), 90_000));
    assert!(state.arm_auto_start("start-1", &meeting(), 100_000));
    assert!(state.calendar_owns_call(&meeting(), 90_000));
    assert!(!state.calendar_owns_call(&meeting(), 160_000));
}

#[test]
fn claiming_a_meeting_withdraws_its_prompt_and_blocks_new_ones() {
    let state = MeetingSchedulerState::default();
    state.upsert_start_prompt("start-1".to_string(), meeting(), false);
    assert_eq!(state.claim_meeting(&meeting()), ["start-1"]);
    assert!(state.current_start_prompt().is_none());
    assert!(!state.upsert_start_prompt("start-1".to_string(), meeting(), false));
}
