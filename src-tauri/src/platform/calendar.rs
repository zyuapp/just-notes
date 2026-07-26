mod store;
mod worker;

use std::{ptr::NonNull, sync::mpsc, time::Duration};

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_event_kit::{
    EKAuthorizationStatus, EKEntityType, EKEventStore, EKEventStoreChangedNotification,
};
use objc2_foundation::{NSError, NSNotification, NSNotificationCenter};
use tauri::AppHandle;

use super::permission_request_result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CalendarInfo {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) account: String,
    pub(crate) color: String,
}

/// One invitee as EventKit reports it. Both identity forms are surfaced so the
/// caller can decide which to prefer and who to keep.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CalendarParticipant {
    pub(crate) name: Option<String>,
    pub(crate) email: Option<String>,
    pub(crate) is_current_user: bool,
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
    pub(crate) participants: Vec<CalendarParticipant>,
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
    let store_until_completion = store.clone();
    let completion = RcBlock::new(move |_granted: Bool, error: *mut NSError| {
        let _keep_store_alive = &store_until_completion;
        let error = unsafe { error.as_ref() }.map(|error| error.localizedDescription().to_string());
        let result = permission_request_result("Calendar", error);
        let _ = sender.send(result);
    });
    unsafe {
        store.requestFullAccessToEventsWithCompletion(&*completion as *const _ as *mut _);
    }
}

pub(crate) fn observe_changes(on_change: impl Fn() + Send + Sync + 'static) {
    let center = NSNotificationCenter::defaultCenter();
    let callback = RcBlock::new(move |_notification: NonNull<NSNotification>| on_change());
    let observer = unsafe {
        center.addObserverForName_object_queue_usingBlock(
            Some(EKEventStoreChangedNotification),
            None,
            None,
            &callback,
        )
    };
    // Calendar observation is process-lifetime infrastructure. NotificationCenter
    // owns the callback registration; retain its opaque token until app exit.
    std::mem::forget(observer);
}

pub(crate) fn list_calendars() -> Result<Vec<CalendarInfo>, String> {
    ensure_authorized()?;
    let mut calendars = worker::list_calendars()?;
    calendars.sort_by(|left, right| {
        left.account
            .cmp(&right.account)
            .then_with(|| left.title.cmp(&right.title))
    });
    Ok(calendars)
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
    let mut events = worker::upcoming_events(calendar_ids, from_ms, to_ms)?;
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
