use super::*;

fn meeting(start_at_ms: u64) -> Meeting {
    Meeting {
        id: "meeting-1".to_string(),
        calendar_id: "calendar-1".to_string(),
        title: "Design review".to_string(),
        start_at_ms,
        end_at_ms: start_at_ms + 30 * 60 * 1000,
    }
}

fn calendar_event() -> calendar::CalendarEvent {
    calendar::CalendarEvent {
        id: "event-1".to_string(),
        calendar_id: "calendar-1".to_string(),
        title: "Design review".to_string(),
        start_at_ms: 1_000,
        end_at_ms: 2_000,
        all_day: false,
        canceled: false,
        free: false,
        current_user_declined: false,
    }
}

#[test]
fn prompt_becomes_due_at_the_configured_lead_time() {
    let meeting = meeting(1_000_000);
    assert!(!start_prompt_is_due(&meeting, 699_999, 5));
    assert!(start_prompt_is_due(&meeting, 700_000, 5));
}

#[test]
fn prompt_stays_available_for_a_short_late_start_window() {
    let meeting = meeting(1_000_000);
    assert!(start_prompt_is_due(&meeting, 1_599_999, 5));
    assert!(!start_prompt_is_due(&meeting, 1_600_000, 5));
}

#[test]
fn start_actions_expire_with_the_late_start_window() {
    let meeting = meeting(1_000_000);
    assert!(start_action_is_timely(&meeting, 1_599_999));
    assert!(!start_action_is_timely(&meeting, 1_600_000));
}

#[test]
fn notification_ids_are_stable_and_kind_specific() {
    assert_eq!(
        notification_id("start", "meeting-1"),
        notification_id("start", "meeting-1")
    );
    assert_ne!(
        notification_id("start", "meeting-1"),
        notification_id("end", "meeting-1")
    );
}

#[test]
fn meeting_policy_excludes_non_meeting_calendar_events() {
    assert!(Meeting::try_from(calendar_event()).is_ok());
    let mut all_day = calendar_event();
    all_day.all_day = true;
    assert!(Meeting::try_from(all_day).is_err());
    let mut canceled = calendar_event();
    canceled.canceled = true;
    assert!(Meeting::try_from(canceled).is_err());
    let mut free = calendar_event();
    free.free = true;
    assert!(Meeting::try_from(free).is_err());
    let mut declined = calendar_event();
    declined.current_user_declined = true;
    assert!(Meeting::try_from(declined).is_err());
}

#[test]
fn blank_calendar_titles_get_a_safe_fallback() {
    let mut event = calendar_event();
    event.title = "  ".to_string();
    assert_eq!(Meeting::try_from(event).unwrap().title, "Calendar meeting");
}
