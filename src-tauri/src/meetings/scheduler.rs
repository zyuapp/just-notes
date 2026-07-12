use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    thread,
    time::Duration,
};

mod prompts;
#[cfg(test)]
use prompts::start_prompt_is_due;
use prompts::{reconcile_start_prompts, start_action_is_timely};

use tauri::{AppHandle, Manager};

use super::{model::Meeting, notifications, state::MeetingSchedulerState};
use crate::{
    app::now_ms,
    platform::calendar,
    recording::RecorderState,
    settings::{AppSettings, SettingsState},
};

const POLL_INTERVAL: Duration = Duration::from_secs(15);
const LOOK_BACK_MS: u64 = 15 * 60 * 1000;
const LOOK_AHEAD_MS: u64 = 12 * 60 * 60 * 1000;

pub(crate) fn spawn(app: AppHandle) {
    thread::spawn(move || loop {
        tick(&app);
        thread::sleep(POLL_INTERVAL);
    });
}

fn tick(app: &AppHandle) {
    let settings = app.state::<SettingsState>().snapshot();
    let scheduler = app.state::<MeetingSchedulerState>().inner().clone();
    if !settings.meeting_reminders_enabled {
        notifications::remove(&scheduler.reset());
        return;
    }

    let active_session_id = reconcile_active_recording(app, &scheduler);

    let Ok(now) = now_ms() else {
        return;
    };
    if let Ok(events) = calendar::upcoming_events(
        &settings.meeting_calendar_ids,
        now.saturating_sub(LOOK_BACK_MS),
        now.saturating_add(LOOK_AHEAD_MS),
    ) {
        reconcile_start_prompts(&scheduler, &settings, events, now);
    }

    if settings.meeting_end_reminders {
        if let Some((request_id, meeting)) =
            active_session_id.and_then(|session_id| scheduler.due_end_prompt(now, session_id))
        {
            notifications::show_end_prompt(&request_id, &meeting.title);
        }
    }
}

fn reconcile_active_recording(app: &AppHandle, scheduler: &MeetingSchedulerState) -> Option<u64> {
    let active_session_id = app.state::<RecorderState>().inner().active_session_id();
    let stale_request = match active_session_id {
        Some(session_id) => scheduler.clear_if_session_differs(session_id),
        None => scheduler.clear_active(),
    };
    if let Some(request_id) = stale_request {
        notifications::remove(&[request_id]);
    }
    active_session_id
}

pub(super) fn meeting_is_current(settings: &AppSettings, meeting: &Meeting) -> bool {
    if !settings.meeting_reminders_enabled
        || !settings.meeting_calendar_ids.contains(&meeting.calendar_id)
    {
        return false;
    }
    let Ok(now) = now_ms() else {
        return false;
    };
    if !start_action_is_timely(meeting, now) {
        return false;
    }
    calendar::upcoming_events(
        std::slice::from_ref(&meeting.calendar_id),
        meeting.start_at_ms.saturating_sub(1),
        meeting.end_at_ms.saturating_add(1),
    )
    .map(|events| {
        events
            .into_iter()
            .filter_map(|event| Meeting::try_from(event).ok())
            .any(|current| current.id == meeting.id)
    })
    .unwrap_or(false)
}

pub(super) fn notification_id(kind: &str, meeting_id: &str) -> String {
    let mut hasher = DefaultHasher::new();
    meeting_id.hash(&mut hasher);
    format!("just-notes-{kind}-{:016x}", hasher.finish())
}

impl TryFrom<calendar::CalendarEvent> for Meeting {
    type Error = ();

    fn try_from(event: calendar::CalendarEvent) -> Result<Self, Self::Error> {
        if event.all_day || event.canceled || event.free || event.current_user_declined {
            return Err(());
        }
        Ok(Self {
            id: event.id,
            calendar_id: event.calendar_id,
            title: if event.title.trim().is_empty() {
                "Calendar meeting".to_string()
            } else {
                event.title
            },
            start_at_ms: event.start_at_ms,
            end_at_ms: event.end_at_ms,
        })
    }
}

#[cfg(test)]
mod tests {
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
}
