use std::collections::HashSet;

use objc2::{msg_send, rc::Retained};
use objc2_app_kit::NSColorSpace;
use objc2_event_kit::{
    EKCalendar, EKEntityType, EKEvent, EKEventAvailability, EKEventStatus, EKEventStore,
    EKParticipant, EKParticipantStatus,
};
use objc2_foundation::{NSArray, NSDate, NSURL};

use super::{CalendarEvent, CalendarInfo, CalendarParticipant};

const UNKNOWN_ACCOUNT: &str = "Other";
const FALLBACK_COLOR: &str = "#8a929a";

pub(super) fn read_calendars(store: &EKEventStore) -> Vec<CalendarInfo> {
    unsafe { store.calendarsForEntityType(EKEntityType::Event) }
        .iter()
        .map(|calendar| CalendarInfo {
            id: unsafe { calendar.calendarIdentifier() }.to_string(),
            title: unsafe { calendar.title() }.to_string(),
            account: read_account(&calendar),
            color: read_color(&calendar),
        })
        .collect()
}

fn read_account(calendar: &EKCalendar) -> String {
    unsafe { calendar.source() }
        .map(|source| unsafe { source.title() }.to_string())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| UNKNOWN_ACCOUNT.to_string())
}

/// EventKit hands back an `NSColor` in an arbitrary colour space; only sRGB
/// components can be read directly, so unconvertible colours fall back.
fn read_color(calendar: &EKCalendar) -> String {
    let color = unsafe { calendar.color() };
    let Some(srgb) = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace()) else {
        return FALLBACK_COLOR.to_string();
    };
    let channel = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(srgb.redComponent()),
        channel(srgb.greenComponent()),
        channel(srgb.blueComponent()),
    )
}

pub(super) fn read_upcoming_events(
    store: &EKEventStore,
    calendar_ids: &[String],
    from_ms: u64,
    to_ms: u64,
) -> Vec<CalendarEvent> {
    let selected_ids = calendar_ids.iter().collect::<HashSet<_>>();
    let selected = unsafe { store.calendarsForEntityType(EKEntityType::Event) }
        .iter()
        .filter(|calendar| {
            selected_ids.contains(&unsafe { calendar.calendarIdentifier() }.to_string())
        })
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Vec::new();
    }

    let calendars = NSArray::from_retained_slice(&selected);
    let start = NSDate::dateWithTimeIntervalSince1970(from_ms as f64 / 1000.0);
    let end = NSDate::dateWithTimeIntervalSince1970(to_ms as f64 / 1000.0);
    let predicate = unsafe {
        store.predicateForEventsWithStartDate_endDate_calendars(&start, &end, Some(&calendars))
    };
    unsafe { store.eventsMatchingPredicate(&predicate) }
        .iter()
        .filter_map(|event| {
            let calendar = unsafe { event.calendar() }?;
            let event_id = unsafe { event.eventIdentifier() }?;
            let start_date = unsafe { event.startDate() };
            let end_date = unsafe { event.endDate() };
            let start_at_ms = date_ms(&start_date);
            let end_at_ms = date_ms(&end_date);
            if end_at_ms <= start_at_ms {
                return None;
            }
            Some(CalendarEvent {
                id: format!("{event_id}:{start_at_ms}"),
                calendar_id: unsafe { calendar.calendarIdentifier() }.to_string(),
                title: unsafe { event.title() }.to_string(),
                start_at_ms,
                end_at_ms,
                all_day: unsafe { event.isAllDay() },
                canceled: unsafe { event.status() } == EKEventStatus::Canceled,
                free: unsafe { event.availability() } == EKEventAvailability::Free,
                current_user_declined: current_user_declined(&event),
                participants: read_participants(&event),
            })
        })
        .collect()
}

fn current_user_declined(event: &EKEvent) -> bool {
    unsafe { event.attendees() }.is_some_and(|attendees| {
        attendees.iter().any(|participant| {
            (unsafe { participant.isCurrentUser() })
                && (unsafe { participant.participantStatus() } == EKParticipantStatus::Declined)
        })
    })
}

/// Invitees in invite order, as reported. Which of them matter, and which
/// identity to show, is the calling domain's decision.
fn read_participants(event: &EKEvent) -> Vec<CalendarParticipant> {
    let Some(attendees) = (unsafe { event.attendees() }) else {
        return Vec::new();
    };
    attendees
        .iter()
        .map(|participant| CalendarParticipant {
            name: unsafe { participant.name() }.map(|name| name.to_string()),
            email: participant_email(&participant),
            is_current_user: unsafe { participant.isCurrentUser() },
        })
        .collect()
}

/// The invite address, parsed out of the participant's `mailto:` URL. EventKit
/// declares `URL` non-null but returns nil for attendees without an address, so
/// the result is read as optional.
fn participant_email(participant: &EKParticipant) -> Option<String> {
    let url: Option<Retained<NSURL>> = unsafe { msg_send![participant, URL] };
    let url = url?.absoluteString()?.to_string();
    Some(url.strip_prefix("mailto:").unwrap_or(&url).to_string())
}

fn date_ms(date: &NSDate) -> u64 {
    (date.timeIntervalSince1970().max(0.0) * 1000.0) as u64
}
