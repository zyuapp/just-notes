use super::*;

const START_MS: u64 = 10 * 60 * 1000;
const END_MS: u64 = START_MS + 30 * 60 * 1000;
const URL: &str = "https://meet.google.com/abc-defg-hij";

fn meeting() -> Meeting {
    Meeting {
        id: "meeting-1".to_string(),
        calendar_id: "calendar-1".to_string(),
        title: "Design review".to_string(),
        start_at_ms: START_MS,
        end_at_ms: END_MS,
        attendees: Vec::new(),
    }
}

fn candidates() -> Vec<(Meeting, String)> {
    vec![(meeting(), URL.to_string())]
}

#[test]
fn reminder_is_due_one_minute_before_start_until_the_late_grace() {
    let meeting = meeting();
    assert!(!join_reminder_is_due(&meeting, START_MS - JOIN_LEAD_MS - 1));
    assert!(join_reminder_is_due(&meeting, START_MS - JOIN_LEAD_MS));
    assert!(join_reminder_is_due(
        &meeting,
        START_MS + JOIN_LATE_GRACE_MS - 1
    ));
    assert!(!join_reminder_is_due(
        &meeting,
        START_MS + JOIN_LATE_GRACE_MS
    ));
}

#[test]
fn reminder_is_not_due_after_a_short_meeting_ends() {
    let meeting = Meeting {
        end_at_ms: START_MS + 1_000,
        ..meeting()
    };
    assert!(!join_reminder_is_due(&meeting, START_MS + 1_000));
}

#[test]
fn due_meeting_is_claimed_once_and_its_link_is_kept() {
    let state = JoinReminderState::default();
    assert_eq!(
        state.reconcile(candidates(), START_MS - 2 * JOIN_LEAD_MS),
        JoinReconciliation::default()
    );

    let outcome = state.reconcile(candidates(), START_MS - JOIN_LEAD_MS);
    let request_id = notification_id("join", "meeting-1");
    assert_eq!(
        outcome.due,
        [DueJoinReminder {
            request_id: request_id.clone(),
            meeting_title: "Design review".to_string(),
            started: false,
        }]
    );
    assert_eq!(state.join_url(&request_id).as_deref(), Some(URL));

    assert_eq!(
        state.reconcile(candidates(), START_MS),
        JoinReconciliation::default()
    );
}

#[test]
fn late_claim_reports_the_meeting_as_started() {
    let state = JoinReminderState::default();
    let outcome = state.reconcile(candidates(), START_MS + 1);
    assert!(outcome.due[0].started);
}

#[test]
fn delivered_reminder_follows_link_changes() {
    let state = JoinReminderState::default();
    state.reconcile(candidates(), START_MS);
    let changed = "https://zoom.us/j/1".to_string();
    state.reconcile(vec![(meeting(), changed.clone())], START_MS + 1);
    assert_eq!(
        state.join_url(&notification_id("join", "meeting-1")),
        Some(changed)
    );
}

#[test]
fn reminders_are_removed_when_the_meeting_ends_or_disappears() {
    let request_id = notification_id("join", "meeting-1");

    let state = JoinReminderState::default();
    state.reconcile(candidates(), START_MS);
    let outcome = state.reconcile(candidates(), END_MS);
    assert_eq!(outcome.removed, std::slice::from_ref(&request_id));
    assert!(outcome.due.is_empty());
    assert_eq!(state.join_url(&request_id), None);

    state.reconcile(candidates(), START_MS);
    assert_eq!(
        state.reconcile(Vec::new(), START_MS + 1).removed,
        [request_id]
    );
}

#[test]
fn released_reminder_is_claimed_again_on_the_next_refresh() {
    let state = JoinReminderState::default();
    let request_id = notification_id("join", "meeting-1");
    state.reconcile(candidates(), START_MS);
    state.release(&request_id);
    assert_eq!(state.join_url(&request_id), None);
    assert_eq!(state.reconcile(candidates(), START_MS + 1).due.len(), 1);
}

#[test]
fn clear_returns_delivered_request_ids() {
    let state = JoinReminderState::default();
    state.reconcile(candidates(), START_MS);
    assert_eq!(state.clear(), [notification_id("join", "meeting-1")]);
    assert!(state.clear().is_empty());
}
