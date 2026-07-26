use super::*;
use crate::meetings::model::MeetingPrompt;

fn meeting() -> Meeting {
    Meeting {
        id: "meeting-1".to_string(),
        calendar_id: "calendar-1".to_string(),
        title: "Design review".to_string(),
        start_at_ms: 1_000,
        end_at_ms: 2_000,
        attendees: Vec::new(),
    }
}

fn prompt(auto_start: bool) -> MeetingPrompt {
    MeetingPrompt {
        request_id: "start-1".to_string(),
        meeting: meeting(),
        auto_start,
        auto_start_after_ms: None,
        auto_start_revision: 0,
    }
}

fn end_prompt(meeting: Meeting, auto_stop: bool) -> DueEndPrompt {
    DueEndPrompt {
        request_id: "end-1".to_string(),
        meeting,
        auto_stop,
    }
}

#[test]
fn start_prompts_are_deduplicated_and_consumed_once() {
    let state = MeetingSchedulerState::default();
    assert!(state.upsert_start_prompt("start-1".to_string(), meeting(), false));
    assert!(!state.upsert_start_prompt("start-2".to_string(), meeting(), false));
    assert_eq!(state.take_start_prompt("start-1"), Some(prompt(false)));
    assert_eq!(state.take_start_prompt("start-1"), None);
}
#[test]
fn failed_start_can_restore_the_same_action() {
    let state = MeetingSchedulerState::default();
    assert!(state.upsert_start_prompt("start-1".to_string(), meeting(), true));
    let prompt = state.take_start_prompt("start-1").unwrap();
    state.restore_start_prompt(prompt.clone());
    assert_eq!(state.take_start_prompt("start-1"), Some(prompt));
}

#[test]
fn calendar_edits_refresh_an_unconsumed_prompt_and_its_automation_policy() {
    let state = MeetingSchedulerState::default();
    assert!(state.upsert_start_prompt("start-1".to_string(), meeting(), true));
    let mut updated = meeting();
    updated.end_at_ms = 3_000;
    updated.attendees.push("Alice".to_string());
    assert!(state.upsert_start_prompt("start-1".to_string(), updated.clone(), false));
    let prompt = state.current_start_prompt().unwrap();
    assert_eq!(prompt.meeting, updated);
    assert!(!prompt.auto_start);
    assert_eq!(prompt.auto_start_after_ms, None);
    assert_eq!(state.take_due_auto_start(1_000), None);
}

#[test]
fn automatic_start_is_due_once_at_the_scheduled_time() {
    let state = MeetingSchedulerState::default();
    let original = meeting();
    assert!(state.upsert_start_prompt("start-1".to_string(), original.clone(), true));
    let mut updated = original.clone();
    updated.end_at_ms = 3_000;
    assert!(state.upsert_start_prompt("start-1".to_string(), updated.clone(), true));
    assert_eq!(state.take_due_auto_start(999), None);
    assert_eq!(state.take_due_auto_start(1_000), None);
    assert!(!state.arm_auto_start("start-1", &original, 1_000));
    assert!(state.arm_auto_start("start-1", &updated, 1_015));
    assert_eq!(state.take_due_auto_start(1_014), None);
    let request_id = state.take_due_auto_start(1_015);
    assert_eq!(request_id, Some("start-1".to_string()));
    assert_eq!(state.take_due_auto_start(1_016), None);
}

#[test]
fn manual_prompts_are_never_selected_for_automatic_start() {
    let state = MeetingSchedulerState::default();
    assert!(state.upsert_start_prompt("start-1".to_string(), meeting(), true));
    state.fallback_to_manual_start("start-1", &meeting());
    assert!(!state.arm_auto_start("start-1", &meeting(), 1_000));
    assert!(!state.current_start_prompt().unwrap().auto_start);
    assert_eq!(state.take_due_auto_start(1_000), None);
}

#[test]
fn late_meetings_are_not_selected_for_automatic_start() {
    let state = MeetingSchedulerState::default();
    let mut meeting = meeting();
    meeting.end_at_ms = 100_000;
    assert!(state.upsert_start_prompt("start-1".to_string(), meeting.clone(), true));
    assert!(state.arm_auto_start("start-1", &meeting, 1_000));
    assert_eq!(state.take_due_auto_start(61_000), None);
}

#[test]
fn current_prompt_is_the_earliest_and_can_be_dismissed() {
    let state = MeetingSchedulerState::default();
    let mut later = meeting();
    later.id = "meeting-2".to_string();
    later.start_at_ms = 1_500;
    assert!(state.upsert_start_prompt("start-2".to_string(), later, false));
    assert!(state.upsert_start_prompt("start-1".to_string(), meeting(), true));
    let current = state.current_start_prompt().unwrap();
    assert_eq!(current.request_id, "start-1");
    assert!(current.auto_start);
    assert!(state.dismiss_start_prompt("start-1"));
    assert_eq!(state.current_start_prompt().unwrap().request_id, "start-2");
    assert!(!state.dismiss_start_prompt("start-1"));
}

#[test]
fn end_prompt_is_once_per_due_time_and_keep_rearms_it() {
    let state = MeetingSchedulerState::default();
    state.set_active(meeting(), 1, "end-1".to_string(), false);
    assert_eq!(state.due_end_prompt(1_999, 1), None);
    let due = state.due_end_prompt(2_000, 1);
    assert_eq!(due, Some(end_prompt(meeting(), false)));
    assert_eq!(state.due_end_prompt(2_001, 1), None);
    state.defer_end_prompt("wrong-id", 3_000);
    assert_eq!(state.due_end_prompt(3_000, 1), None);
    state.defer_end_prompt("end-1", 3_000);
    let due = state.due_end_prompt(3_000, 1);
    assert_eq!(due, Some(end_prompt(meeting(), false)));
}

#[test]
fn automatic_stop_warns_then_becomes_due_at_the_scheduled_end() {
    let state = MeetingSchedulerState::default();
    let mut scheduled = meeting();
    scheduled.end_at_ms = 120_000;
    state.set_active(scheduled.clone(), 1, "end-1".to_string(), true);
    assert_eq!(state.due_end_prompt(59_999, 1), None);
    assert_eq!(
        state.due_end_prompt(60_000, 1),
        Some(end_prompt(scheduled, true))
    );
    assert_eq!(state.claim_due_auto_stop(119_999, 1), None);
    assert_eq!(state.claim_due_auto_stop(120_000, 1), Some("end-1".into()));
    state.defer_end_prompt("end-1", 180_000);
    state.retry_auto_stop("end-1");
    assert_eq!(state.claim_due_auto_stop(120_000, 1), Some("end-1".into()));
}

#[test]
fn keeping_an_automatic_recording_cancels_automatic_stop() {
    let state = MeetingSchedulerState::default();
    let mut scheduled = meeting();
    scheduled.end_at_ms = 120_000;
    state.set_active(scheduled, 1, "end-1".to_string(), true);
    assert!(state.due_end_prompt(60_000, 1).is_some());
    state.defer_end_prompt("end-1", 180_000);
    assert_eq!(state.claim_due_auto_stop(120_000, 1), None);
    assert!(state.due_end_prompt(180_000, 1).is_some());
}

#[test]
fn end_reminder_changes_do_not_cancel_an_enabled_automatic_stop() {
    let state = MeetingSchedulerState::default();
    state.set_active(meeting(), 1, "end-1".to_string(), true);
    assert_eq!(state.update_active_end_reminders(false), None);
    assert_eq!(state.claim_due_auto_stop(2_000, 1), Some("end-1".into()));
}

#[test]
fn a_late_warning_defers_automatic_stop_for_one_minute() {
    let state = MeetingSchedulerState::default();
    let mut scheduled = meeting();
    scheduled.end_at_ms = 120_000;
    state.set_active(scheduled.clone(), 1, "end-1".to_string(), true);
    assert_eq!(
        state.due_end_prompt(130_000, 1),
        Some(end_prompt(scheduled, true))
    );
    assert_eq!(state.claim_due_auto_stop(189_999, 1), None);
    assert_eq!(state.claim_due_auto_stop(190_000, 1), Some("end-1".into()));
}

#[test]
fn a_short_meeting_still_gets_one_minute_after_its_warning() {
    let state = MeetingSchedulerState::default();
    let mut scheduled = meeting();
    scheduled.end_at_ms = 30_000;
    state.set_active(scheduled.clone(), 1, "end-1".to_string(), true);
    assert_eq!(
        state.due_end_prompt(15_000, 1),
        Some(end_prompt(scheduled, true))
    );
    assert_eq!(state.claim_due_auto_stop(30_000, 1), None);
    assert_eq!(state.claim_due_auto_stop(75_000, 1), Some("end-1".into()));
}

#[test]
fn disabling_end_reminders_clears_a_manually_started_meeting() {
    let state = MeetingSchedulerState::default();
    state.set_active(meeting(), 1, "end-1".to_string(), false);
    let removed = state.update_active_end_reminders(false);
    assert_eq!(removed, Some("end-1".to_string()));
    assert_eq!(state.due_end_prompt(2_000, 1), None);
}

#[test]
fn failed_end_notification_rearms_the_same_prompt() {
    let state = MeetingSchedulerState::default();
    let meeting = meeting();
    state.set_active(meeting.clone(), 7, "end-1".to_string(), false);
    let due = state.due_end_prompt(2_000, 7);
    assert_eq!(due, Some(end_prompt(meeting.clone(), false)));
    assert_eq!(state.due_end_prompt(2_000, 7), None);
    state.retry_end_prompt("end-1");
    let due = state.due_end_prompt(2_000, 7);
    assert_eq!(due, Some(end_prompt(meeting, false)));
}

#[test]
fn end_actions_cannot_target_a_different_recording() {
    let state = MeetingSchedulerState::default();
    state.set_active(meeting(), 1, "end-1".to_string(), false);
    assert!(!state.matches_active_end_prompt("end-1", 2));
    assert_eq!(state.clear_if_session_differs(2), Some("end-1".to_string()));
    assert!(!state.matches_active_end_prompt("end-1", 1));
}

#[test]
fn reconciliation_removes_stale_prompt_actions() {
    let state = MeetingSchedulerState::default();
    assert!(state.upsert_start_prompt("start-1".to_string(), meeting(), false));
    let removed = state.reconcile_start_prompts(&HashSet::new());
    assert_eq!(removed, ["start-1"]);
    assert_eq!(state.take_start_prompt("start-1"), None);
}
