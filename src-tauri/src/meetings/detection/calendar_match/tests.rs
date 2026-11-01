use super::matching_meeting;
use crate::{
    meetings::detection::call_app::{classify, CallApp},
    platform::{
        calendar::{CalendarEvent, CalendarParticipant},
        microphone_activity::MicrophoneClient,
    },
};

const MINUTE: u64 = 60 * 1000;
const NOW: u64 = 1_000 * MINUTE;

fn app(bundle_id: &str) -> CallApp {
    let client = MicrophoneClient {
        pid: 1,
        audio_bundle_id: Some(bundle_id.to_string()),
        app: None,
    };
    classify(&client, |_| false).unwrap()
}

fn event(title: &str, start_offset_min: i64, link: Option<&str>, attendees: bool) -> CalendarEvent {
    let start = NOW.saturating_add_signed(start_offset_min * MINUTE as i64);
    CalendarEvent {
        id: format!("{title}:{start}"),
        calendar_id: "work".to_string(),
        title: title.to_string(),
        start_at_ms: start,
        end_at_ms: start + 30 * MINUTE,
        all_day: false,
        canceled: false,
        free: false,
        current_user_declined: false,
        participants: if attendees {
            vec![CalendarParticipant {
                name: Some("Ada".to_string()),
                email: None,
                is_current_user: false,
            }]
        } else {
            Vec::new()
        },
        url: None,
        location: link.map(str::to_string),
        notes: None,
    }
}

fn title(app: &CallApp, events: Vec<CalendarEvent>) -> Option<String> {
    matching_meeting(app, events, NOW).map(|meeting| meeting.title)
}

#[test]
fn a_meeting_with_this_apps_link_wins_over_one_without() {
    let events = vec![
        event("Standup", -2, None, true),
        event("Design sync", -1, Some("https://us02web.zoom.us/j/1"), true),
    ];
    assert_eq!(
        title(&app("us.zoom.xos"), events).as_deref(),
        Some("Design sync")
    );
}

#[test]
fn a_link_to_another_service_never_matches() {
    let events = vec![event(
        "Teams review",
        0,
        Some("https://teams.microsoft.com/l/1"),
        true,
    )];
    assert_eq!(title(&app("us.zoom.xos"), events), None);
}

#[test]
fn a_browser_call_matches_a_google_meet_meeting() {
    let events = vec![event(
        "1:1",
        0,
        Some("https://meet.google.com/abc-defg-hij"),
        true,
    )];
    assert_eq!(
        title(&app("com.google.Chrome"), events).as_deref(),
        Some("1:1")
    );
}

#[test]
fn solo_blocks_without_links_and_distant_meetings_are_ignored() {
    let events = vec![
        event("Focus time", -5, None, false),
        event("Later", 15, Some("https://zoom.us/j/2"), true),
        event("Earlier", -45, Some("https://zoom.us/j/3"), true),
    ];
    assert_eq!(title(&app("us.zoom.xos"), events), None);
}

#[test]
fn joining_early_matches_the_upcoming_meeting() {
    let events = vec![event("Kickoff", 8, Some("https://zoom.us/j/4"), true)];
    assert_eq!(
        title(&app("us.zoom.xos"), events).as_deref(),
        Some("Kickoff")
    );
}

#[test]
fn the_meeting_starting_closest_to_now_wins_a_tie() {
    let events = vec![
        event("Overrun", -25, Some("https://zoom.us/j/5"), true),
        event("Next", 1, Some("https://zoom.us/j/6"), true),
    ];
    assert_eq!(title(&app("us.zoom.xos"), events).as_deref(), Some("Next"));
}
