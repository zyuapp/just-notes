use super::*;

fn meeting(start_at_ms: u64) -> Meeting {
    Meeting {
        id: "meeting-1".to_string(),
        calendar_id: "calendar-1".to_string(),
        title: "Design review".to_string(),
        start_at_ms,
        end_at_ms: start_at_ms + 30 * 60 * 1000,
        attendees: Vec::new(),
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
        participants: vec![named_participant("Alice"), named_participant("Bob")],
    }
}

fn named_participant(name: &str) -> calendar::CalendarParticipant {
    calendar::CalendarParticipant {
        name: Some(name.to_string()),
        email: Some(format!("{}@example.com", name.to_lowercase())),
        is_current_user: false,
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
fn eligible_meetings_keep_their_attendees() {
    let meeting = Meeting::try_from(calendar_event()).unwrap();
    assert_eq!(meeting.attendees, vec!["Alice", "Bob"]);
}

#[test]
fn attendee_names_drop_the_current_user_and_keep_invite_order() {
    let mut event = calendar_event();
    event.participants.insert(
        1,
        calendar::CalendarParticipant {
            name: Some("Me".to_string()),
            email: Some("me@example.com".to_string()),
            is_current_user: true,
        },
    );

    let meeting = Meeting::try_from(event).unwrap();
    assert_eq!(meeting.attendees, vec!["Alice", "Bob"]);
}

#[test]
fn attendee_names_fall_back_to_the_invite_address_and_deduplicate() {
    let mut event = calendar_event();
    event.participants = vec![
        calendar::CalendarParticipant {
            name: None,
            email: Some("carol@example.com".to_string()),
            is_current_user: false,
        },
        calendar::CalendarParticipant {
            name: Some("   ".to_string()),
            email: None,
            is_current_user: false,
        },
        named_participant("Alice"),
        named_participant("Alice"),
    ];

    let meeting = Meeting::try_from(event).unwrap();
    assert_eq!(meeting.attendees, vec!["carol@example.com", "Alice"]);
}

#[test]
fn blank_calendar_titles_get_a_safe_fallback() {
    let mut event = calendar_event();
    event.title = "  ".to_string();
    assert_eq!(Meeting::try_from(event).unwrap().title, "Calendar meeting");
}

#[test]
fn calendar_read_status_reports_changes_without_repeating_the_same_error() {
    let mut status = CalendarReadStatus::default();
    let failure = Err("Calendar worker is busy".to_string());

    assert_eq!(
        status.update(&failure),
        Some(CalendarReadTransition::Failed(
            "Calendar worker is busy".to_string()
        ))
    );
    assert_eq!(status.update(&failure), None);
    assert_eq!(
        status.update(&Err("Calendar access is denied".to_string())),
        Some(CalendarReadTransition::Failed(
            "Calendar access is denied".to_string()
        ))
    );
    assert_eq!(
        status.update(&Ok(())),
        Some(CalendarReadTransition::Recovered)
    );
    assert_eq!(status.update(&Ok(())), None);
}

#[test]
fn poll_cadence_accounts_for_calendar_query_time() {
    assert_eq!(
        next_poll_delay(Duration::from_secs(2)),
        Duration::from_secs(13)
    );
    assert_eq!(next_poll_delay(Duration::from_secs(15)), Duration::ZERO);
    assert_eq!(next_poll_delay(Duration::from_secs(30)), Duration::ZERO);
}

#[test]
fn calendar_failure_does_not_skip_the_end_reminder_check() {
    let checked_end_reminder = std::cell::Cell::new(false);

    let result = run_calendar_cycle(
        || checked_end_reminder.set(true),
        || {
            assert!(checked_end_reminder.get());
            Err("EventKit unavailable".to_string())
        },
    );

    assert_eq!(result, Err("EventKit unavailable".to_string()));
    assert!(checked_end_reminder.get());
}
