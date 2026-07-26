use std::collections::HashSet;

use crate::{app::now_ms, platform::calendar, settings::AppSettings};

use super::notification_id;
use crate::meetings::{model::Meeting, notifications, state::MeetingSchedulerState};

const LATE_START_GRACE_MS: u64 = 10 * 60 * 1000;
const AUTO_START_SKIP_WINDOW_MS: u64 = 15 * 1000;

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
    let auto_start = automatic_start_for_prompt(scheduler, &request_id, settings, &meeting, now);
    if scheduler.upsert_start_prompt(request_id.clone(), meeting.clone(), auto_start) {
        let timing = start_timing(&meeting, now, settings.meeting_reminder_minutes);
        if auto_start {
            let scheduler = scheduler.clone();
            let completion_request_id = request_id.clone();
            let expected_meeting = meeting.clone();
            let meeting_title = meeting.title.clone();
            notifications::show_automatic_start_prompt(
                &request_id,
                &meeting.title,
                &timing,
                move |result| match result {
                    Ok(()) => {
                        if let Ok(confirmed_at_ms) = now_ms() {
                            let start_after_ms = expected_meeting
                                .start_at_ms
                                .max(confirmed_at_ms.saturating_add(AUTO_START_SKIP_WINDOW_MS));
                            scheduler.arm_auto_start(
                                &completion_request_id,
                                &expected_meeting,
                                start_after_ms,
                            );
                        } else {
                            scheduler.fallback_to_manual_start(
                                &completion_request_id,
                                &expected_meeting,
                            );
                        }
                    }
                    Err(error) => {
                        eprintln!(
                            "automatic recording for {meeting_title} was not armed because its \
                             notification could not be delivered: {error}"
                        );
                        scheduler
                            .fallback_to_manual_start(&completion_request_id, &expected_meeting);
                    }
                },
            );
        } else {
            notifications::show_start_prompt(&request_id, &meeting.title, &timing);
        }
    }
}

fn start_timing(meeting: &Meeting, now_ms: u64, lead_minutes: u16) -> String {
    if now_ms >= meeting.start_at_ms {
        "now".to_string()
    } else {
        format!("in {lead_minutes} minutes")
    }
}

pub(super) fn automatic_start_for_prompt(
    scheduler: &MeetingSchedulerState,
    request_id: &str,
    settings: &AppSettings,
    meeting: &Meeting,
    now_ms: u64,
) -> bool {
    auto_record_eligible(settings, meeting)
        && (now_ms < meeting.start_at_ms || scheduler.start_prompt_is_automatic(request_id))
}

pub(in crate::meetings) fn auto_record_eligible(settings: &AppSettings, meeting: &Meeting) -> bool {
    settings.meeting_auto_record_enabled && !meeting.attendees.is_empty()
}

pub(super) fn start_action_is_timely(meeting: &Meeting, now_ms: u64) -> bool {
    now_ms < meeting.start_at_ms.saturating_add(LATE_START_GRACE_MS)
}

pub(super) fn start_prompt_is_due(meeting: &Meeting, now_ms: u64, lead_minutes: u16) -> bool {
    let lead_ms = u64::from(lead_minutes) * 60 * 1000;
    now_ms >= meeting.start_at_ms.saturating_sub(lead_ms)
        && now_ms < meeting.start_at_ms.saturating_add(LATE_START_GRACE_MS)
}
