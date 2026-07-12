use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
    thread,
    time::Duration,
};

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
const LATE_START_GRACE_MS: u64 = 10 * 60 * 1000;

pub(crate) fn spawn(app: AppHandle, sync_prompt: impl Fn(&AppHandle) + Send + 'static) {
    thread::spawn(move || loop {
        tick(&app);
        sync_prompt(&app);
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

    let recorder = app.state::<RecorderState>().inner().clone();
    let active_session_id = recorder.active_session_id();
    if active_session_id.is_none() {
        if let Some(request_id) = scheduler.clear_active() {
            notifications::remove(&[request_id]);
        }
    } else if let Some(request_id) =
        active_session_id.and_then(|session_id| scheduler.clear_if_session_differs(session_id))
    {
        notifications::remove(&[request_id]);
    }

    let Ok(now) = now_ms() else {
        return;
    };
    refresh_start_prompts(&settings, &scheduler, now);

    if settings.meeting_end_reminders {
        if let Some((request_id, meeting)) =
            active_session_id.and_then(|session_id| scheduler.due_end_prompt(now, session_id))
        {
            notifications::show_end_prompt(&request_id, &meeting.title);
        }
    }
}

fn refresh_start_prompts(settings: &AppSettings, scheduler: &MeetingSchedulerState, now: u64) {
    let Ok(events) = calendar::upcoming_events(
        &settings.meeting_calendar_ids,
        now.saturating_sub(LOOK_BACK_MS),
        now.saturating_add(LOOK_AHEAD_MS),
    ) else {
        return;
    };
    let meetings = events
        .into_iter()
        .filter_map(|event| Meeting::try_from(event).ok())
        .collect::<Vec<_>>();
    let valid_ids = meetings
        .iter()
        .filter(|meeting| start_action_is_timely(meeting, now))
        .map(|meeting| meeting.id.clone())
        .collect::<HashSet<_>>();
    notifications::remove(&scheduler.reconcile_start_prompts(&valid_ids));

    for meeting in meetings {
        if !start_prompt_is_due(&meeting, now, settings.meeting_reminder_minutes) {
            continue;
        }
        let request_id = notification_id("start", &meeting.id);
        if scheduler.register_start_prompt(request_id.clone(), meeting.clone()) {
            let lead = settings.meeting_reminder_minutes;
            let timing = if now >= meeting.start_at_ms {
                "now".to_string()
            } else {
                format!("in {lead} minutes")
            };
            notifications::show_start_prompt(&request_id, &meeting.title, &timing);
        }
    }
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

fn start_action_is_timely(meeting: &Meeting, now_ms: u64) -> bool {
    now_ms < meeting.start_at_ms.saturating_add(LATE_START_GRACE_MS)
}

fn start_prompt_is_due(meeting: &Meeting, now_ms: u64, lead_minutes: u16) -> bool {
    let lead_ms = u64::from(lead_minutes) * 60 * 1000;
    now_ms >= meeting.start_at_ms.saturating_sub(lead_ms)
        && now_ms < meeting.start_at_ms.saturating_add(LATE_START_GRACE_MS)
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
mod tests;
