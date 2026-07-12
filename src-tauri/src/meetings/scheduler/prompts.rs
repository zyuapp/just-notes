use std::collections::HashSet;

use crate::{platform::calendar, settings::AppSettings};

use super::notification_id;
use crate::meetings::{model::Meeting, notifications, state::MeetingSchedulerState};

const LATE_START_GRACE_MS: u64 = 10 * 60 * 1000;

pub(super) fn reconcile_start_prompts(
    scheduler: &MeetingSchedulerState,
    settings: &AppSettings,
    events: Vec<calendar::CalendarEvent>,
    now: u64,
) {
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
        show_start_prompt_if_due(scheduler, settings, meeting, now);
    }
}

fn show_start_prompt_if_due(
    scheduler: &MeetingSchedulerState,
    settings: &AppSettings,
    meeting: Meeting,
    now: u64,
) {
    if !start_prompt_is_due(&meeting, now, settings.meeting_reminder_minutes) {
        return;
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

pub(super) fn start_action_is_timely(meeting: &Meeting, now_ms: u64) -> bool {
    now_ms < meeting.start_at_ms.saturating_add(LATE_START_GRACE_MS)
}

pub(super) fn start_prompt_is_due(meeting: &Meeting, now_ms: u64, lead_minutes: u16) -> bool {
    let lead_ms = u64::from(lead_minutes) * 60 * 1000;
    now_ms >= meeting.start_at_ms.saturating_sub(lead_ms)
        && now_ms < meeting.start_at_ms.saturating_add(LATE_START_GRACE_MS)
}
