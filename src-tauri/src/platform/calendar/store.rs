use std::collections::HashSet;

use objc2_event_kit::{
    EKEntityType, EKEventAvailability, EKEventStatus, EKEventStore, EKParticipantStatus,
};
use objc2_foundation::{NSArray, NSDate};

use super::{CalendarEvent, CalendarInfo};

pub(super) fn read_calendars(store: &EKEventStore) -> Vec<CalendarInfo> {
    unsafe { store.calendarsForEntityType(EKEntityType::Event) }
        .iter()
        .map(|calendar| CalendarInfo {
            id: unsafe { calendar.calendarIdentifier() }.to_string(),
            title: unsafe { calendar.title() }.to_string(),
        })
        .collect()
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
                id: format!("{}:{start_at_ms}", event_id),
                calendar_id: unsafe { calendar.calendarIdentifier() }.to_string(),
                title: unsafe { event.title() }.to_string(),
                start_at_ms,
                end_at_ms,
                all_day: unsafe { event.isAllDay() },
                canceled: unsafe { event.status() } == EKEventStatus::Canceled,
                free: unsafe { event.availability() } == EKEventAvailability::Free,
                current_user_declined: current_user_declined(&event),
            })
        })
        .collect()
}

fn current_user_declined(event: &objc2_event_kit::EKEvent) -> bool {
    unsafe { event.attendees() }.is_some_and(|attendees| {
        attendees.iter().any(|participant| {
            (unsafe { participant.isCurrentUser() })
                && (unsafe { participant.participantStatus() } == EKParticipantStatus::Declined)
        })
    })
}

fn date_ms(date: &NSDate) -> u64 {
    (date.timeIntervalSince1970().max(0.0) * 1000.0) as u64
}
