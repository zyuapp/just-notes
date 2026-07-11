use super::*;

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
fn start_prompts_are_deduplicated_and_consumed_once() {
    let state = MeetingSchedulerState::default();
    assert!(state.register_start_prompt("start-1".to_string(), meeting()));
    assert!(!state.register_start_prompt("start-2".to_string(), meeting()));
    assert_eq!(state.take_start_prompt("start-1"), Some(meeting()));
    assert_eq!(state.take_start_prompt("start-1"), None);
}

#[test]
fn failed_start_can_restore_the_same_action() {
    let state = MeetingSchedulerState::default();
    assert!(state.register_start_prompt("start-1".to_string(), meeting()));
    let prompt = state.take_start_prompt("start-1").unwrap();
    state.restore_start_prompt("start-1".to_string(), prompt);
    assert_eq!(state.take_start_prompt("start-1"), Some(meeting()));
}

#[test]
fn end_prompt_is_once_per_due_time_and_keep_rearms_it() {
    let state = MeetingSchedulerState::default();
    state.set_active(meeting(), 1, "end-1".to_string());
    assert_eq!(state.due_end_prompt(1_999, 1), None);
    assert_eq!(
        state.due_end_prompt(2_000, 1),
        Some(("end-1".to_string(), meeting()))
    );
    assert_eq!(state.due_end_prompt(2_001, 1), None);
    state.defer_end_prompt("wrong-id", 3_000);
    assert_eq!(state.due_end_prompt(3_000, 1), None);
    state.defer_end_prompt("end-1", 3_000);
    assert_eq!(
        state.due_end_prompt(3_000, 1),
        Some(("end-1".to_string(), meeting()))
    );
}

#[test]
fn end_actions_cannot_target_a_different_recording() {
    let state = MeetingSchedulerState::default();
    state.set_active(meeting(), 1, "end-1".to_string());
    assert!(!state.matches_active_end_prompt("end-1", 2));
    assert_eq!(state.clear_if_session_differs(2), Some("end-1".to_string()));
    assert!(!state.matches_active_end_prompt("end-1", 1));
}

#[test]
fn reconciliation_removes_stale_prompt_actions() {
    let state = MeetingSchedulerState::default();
    assert!(state.register_start_prompt("start-1".to_string(), meeting()));
    let removed = state.reconcile_start_prompts(&HashSet::new());
    assert_eq!(removed, ["start-1"]);
    assert_eq!(state.take_start_prompt("start-1"), None);
}
