use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
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

pub(crate) fn spawn(app: AppHandle, sync_prompt: impl Fn(&AppHandle) + Send + 'static) {
    let (wake_sender, wake_receiver) = mpsc::sync_channel(1);
    calendar::observe_changes(move || {
        let _ = wake_sender.try_send(());
    });
    thread::spawn(move || {
        let mut read_status = CalendarReadStatus::default();
        loop {
            let started_at = Instant::now();
            report_calendar_transition(read_status.update(&tick(&app)));
            sync_prompt(&app);
            let remaining = next_poll_delay(started_at.elapsed());
            if matches!(
                wake_receiver.recv_timeout(remaining),
                Err(mpsc::RecvTimeoutError::Disconnected)
            ) {
                thread::sleep(remaining);
            }
        }
    });
}

fn next_poll_delay(elapsed: Duration) -> Duration {
    POLL_INTERVAL.saturating_sub(elapsed)
}

fn tick(app: &AppHandle) -> Result<(), String> {
    let settings = app.state::<SettingsState>().snapshot();
    let scheduler = app.state::<MeetingSchedulerState>().inner().clone();
    if !settings.meeting_reminders_enabled {
        notifications::remove(&scheduler.reset());
        return Ok(());
    }

    let active_session_id = reconcile_active_recording(app, &scheduler);

    let Ok(now) = now_ms() else {
        return Ok(());
    };
    run_calendar_cycle(
        || {
            // End reminders depend only on the active recording and local
            // scheduler state, not on the EventKit refresh result.
            if settings.meeting_end_reminders {
                if let Some((request_id, meeting)) = active_session_id
                    .and_then(|session_id| scheduler.due_end_prompt(now, session_id))
                {
                    let retry_scheduler = scheduler.clone();
                    let retry_request_id = request_id.clone();
                    notifications::show_end_prompt(&request_id, &meeting.title, move |error| {
                        eprintln!(
                            "end reminder {retry_request_id} could not be delivered: {error}"
                        );
                        retry_scheduler.retry_end_prompt(&retry_request_id);
                    });
                }
            }
        },
        || refresh_start_prompts(&settings, &scheduler, now),
    )
}

fn run_calendar_cycle(
    check_end_reminder: impl FnOnce(),
    refresh_start_prompts: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    check_end_reminder();
    refresh_start_prompts()
}

fn refresh_start_prompts(
    settings: &AppSettings,
    scheduler: &MeetingSchedulerState,
    now: u64,
) -> Result<(), String> {
    let events = calendar::upcoming_events(
        &settings.meeting_calendar_ids,
        now.saturating_sub(LOOK_BACK_MS),
        now.saturating_add(LOOK_AHEAD_MS),
    )?;
    reconcile_start_prompts(scheduler, settings, events, now);
    Ok(())
}

#[derive(Default)]
struct CalendarReadStatus {
    error: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum CalendarReadTransition {
    Failed(String),
    Recovered,
}

impl CalendarReadStatus {
    fn update(&mut self, result: &Result<(), String>) -> Option<CalendarReadTransition> {
        match result {
            Ok(()) if self.error.take().is_some() => Some(CalendarReadTransition::Recovered),
            Ok(()) => None,
            Err(error) if self.error.as_deref() == Some(error) => None,
            Err(error) => {
                self.error = Some(error.clone());
                Some(CalendarReadTransition::Failed(error.clone()))
            }
        }
    }
}

fn report_calendar_transition(transition: Option<CalendarReadTransition>) {
    match transition {
        Some(CalendarReadTransition::Failed(error)) => {
            eprintln!("calendar refresh failed: {error}");
        }
        Some(CalendarReadTransition::Recovered) => eprintln!("calendar refresh recovered"),
        None => {}
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
mod tests;
