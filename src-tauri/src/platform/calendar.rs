use std::{collections::HashSet, sync::mpsc, time::Duration};

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_event_kit::{
    EKAuthorizationStatus, EKEntityType, EKEventAvailability, EKEventStatus, EKEventStore,
    EKParticipantStatus,
};
use objc2_foundation::{NSArray, NSDate, NSError};
use tauri::AppHandle;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CalendarInfo {
    pub(crate) id: String,
    pub(crate) title: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CalendarEvent {
    pub(crate) id: String,
    pub(crate) calendar_id: String,
    pub(crate) title: String,
    pub(crate) start_at_ms: u64,
    pub(crate) end_at_ms: u64,
    pub(crate) all_day: bool,
    pub(crate) canceled: bool,
    pub(crate) free: bool,
    pub(crate) current_user_declined: bool,
}

pub(crate) fn authorization_status() -> String {
    let status = unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) };
    match status {
        EKAuthorizationStatus::FullAccess => "authorized",
        EKAuthorizationStatus::Denied => "denied",
        EKAuthorizationStatus::Restricted => "restricted",
        EKAuthorizationStatus::NotDetermined => "notDetermined",
        EKAuthorizationStatus::WriteOnly => "writeOnly",
        _ => "unknown",
    }
    .to_string()
}

pub(crate) fn request_access(app: &AppHandle) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();
    app.run_on_main_thread(move || request_access_on_main(sender))
        .map_err(|err| format!("Failed to request Calendar access on the main thread: {err}"))?;
    receiver
        .recv_timeout(Duration::from_secs(120))
        .map_err(|_| "Timed out waiting for Calendar permission".to_string())?
}

fn request_access_on_main(sender: mpsc::Sender<Result<(), String>>) {
    let store = unsafe { EKEventStore::new() };
    let completion = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
        let result = if granted.as_bool() {
            Ok(())
        } else {
            Err("Calendar access was not granted".to_string())
        };
        let _ = sender.send(result);
    });
    unsafe {
        store.requestFullAccessToEventsWithCompletion(&*completion as *const _ as *mut _);
    }
    std::mem::forget(completion);
}

pub(crate) fn list_calendars() -> Result<Vec<CalendarInfo>, String> {
    ensure_authorized()?;
    let store = unsafe { EKEventStore::new() };
    let calendars = unsafe { store.calendarsForEntityType(EKEntityType::Event) };
    let mut result = calendars
        .iter()
        .map(|calendar| CalendarInfo {
            id: unsafe { calendar.calendarIdentifier() }.to_string(),
            title: unsafe { calendar.title() }.to_string(),
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| left.title.cmp(&right.title));
    Ok(result)
}

pub(crate) fn upcoming_events(
    calendar_ids: &[String],
    from_ms: u64,
    to_ms: u64,
) -> Result<Vec<CalendarEvent>, String> {
    ensure_authorized()?;
    if calendar_ids.is_empty() || from_ms >= to_ms {
        return Ok(Vec::new());
    }

    let selected_ids = calendar_ids.iter().collect::<HashSet<_>>();
    let store = unsafe { EKEventStore::new() };
    let selected = unsafe { store.calendarsForEntityType(EKEntityType::Event) }
        .iter()
        .filter(|calendar| {
            selected_ids.contains(&unsafe { calendar.calendarIdentifier() }.to_string())
        })
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Ok(Vec::new());
    }

    let calendars = NSArray::from_retained_slice(&selected);
    let start = NSDate::dateWithTimeIntervalSince1970(from_ms as f64 / 1000.0);
    let end = NSDate::dateWithTimeIntervalSince1970(to_ms as f64 / 1000.0);
    let predicate = unsafe {
        store.predicateForEventsWithStartDate_endDate_calendars(&start, &end, Some(&calendars))
    };
    let events = unsafe { store.eventsMatchingPredicate(&predicate) };

    let mut events = events
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
        .collect::<Vec<_>>();
    events.sort_by_key(|event| event.start_at_ms);
    Ok(events)
}

fn ensure_authorized() -> Result<(), String> {
    match authorization_status().as_str() {
        "authorized" => Ok(()),
        "denied" => Err("Calendar access is denied in System Settings".to_string()),
        "restricted" => Err("Calendar access is restricted by macOS policy".to_string()),
        _ => Err("Calendar access has not been granted".to_string()),
    }
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
