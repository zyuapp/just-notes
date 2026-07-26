use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use super::model::{Meeting, MeetingPrompt};

mod start;

const AUTO_STOP_WARNING_MS: u64 = 60 * 1000;
const END_REMINDER_DELAY_MS: u64 = 10 * 60 * 1000;

#[derive(Clone, Default)]
pub(crate) struct MeetingSchedulerState(Arc<Mutex<SchedulerData>>);

#[derive(Default)]
struct SchedulerData {
    prompted_meetings: HashSet<String>,
    auto_start_attempted: HashSet<String>,
    auto_start_revision: u64,
    start_prompts: HashMap<String, MeetingPrompt>,
    active_meeting: Option<ActiveMeeting>,
}

struct ActiveMeeting {
    meeting: Meeting,
    recording_session_id: u64,
    end_request_id: String,
    next_prompt_at_ms: u64,
    prompted: bool,
    auto_stop_at_ms: Option<u64>,
    auto_stop_claimed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct DueEndPrompt {
    pub(super) request_id: String,
    pub(super) meeting: Meeting,
    pub(super) auto_stop: bool,
}

impl MeetingSchedulerState {
    pub(super) fn clear_active(&self) -> Option<String> {
        self.0
            .lock()
            .ok()?
            .active_meeting
            .take()
            .map(|active| active.end_request_id)
    }

    pub(super) fn clear_if_session_differs(&self, active_session_id: u64) -> Option<String> {
        let mut data = self.0.lock().ok()?;
        if data
            .active_meeting
            .as_ref()
            .is_some_and(|active| active.recording_session_id != active_session_id)
        {
            return data
                .active_meeting
                .take()
                .map(|active| active.end_request_id);
        }
        None
    }

    pub(super) fn update_active_end_reminders(
        &self,
        end_reminders_enabled: bool,
    ) -> Option<String> {
        let mut data = self.0.lock().ok()?;
        let active = data.active_meeting.as_mut()?;
        if active.auto_stop_claimed {
            return None;
        }
        if active.auto_stop_at_ms.is_some() {
            return None;
        }
        if end_reminders_enabled {
            return None;
        }
        data.active_meeting
            .take()
            .map(|active| active.end_request_id)
    }

    pub(super) fn set_active(
        &self,
        meeting: Meeting,
        recording_session_id: u64,
        end_request_id: String,
        auto_stop: bool,
    ) {
        if let Ok(mut data) = self.0.lock() {
            let next_prompt_at_ms = if auto_stop {
                meeting.end_at_ms.saturating_sub(AUTO_STOP_WARNING_MS)
            } else {
                meeting.end_at_ms
            };
            let auto_stop_at_ms = auto_stop.then_some(meeting.end_at_ms);
            data.active_meeting = Some(ActiveMeeting {
                meeting,
                recording_session_id,
                end_request_id,
                next_prompt_at_ms,
                prompted: false,
                auto_stop_at_ms,
                auto_stop_claimed: false,
            });
        }
    }

    pub(super) fn due_end_prompt(
        &self,
        now_ms: u64,
        active_session_id: u64,
    ) -> Option<DueEndPrompt> {
        let mut data = self.0.lock().ok()?;
        if data
            .active_meeting
            .as_ref()
            .is_some_and(|active| active.recording_session_id != active_session_id)
        {
            return None;
        }
        let active = data.active_meeting.as_mut()?;
        if active.prompted || now_ms < active.next_prompt_at_ms {
            return None;
        }
        active.prompted = true;
        if let Some(stop_at_ms) = active.auto_stop_at_ms.as_mut() {
            *stop_at_ms = (*stop_at_ms).max(now_ms.saturating_add(AUTO_STOP_WARNING_MS));
        }
        Some(DueEndPrompt {
            request_id: active.end_request_id.clone(),
            meeting: active.meeting.clone(),
            auto_stop: active.auto_stop_at_ms.is_some(),
        })
    }

    pub(super) fn defer_end_prompt(&self, request_id: &str, next_prompt_at_ms: u64) {
        let Ok(mut data) = self.0.lock() else {
            return;
        };
        let Some(active) = data.active_meeting.as_mut() else {
            return;
        };
        if active.end_request_id == request_id && !active.auto_stop_claimed {
            active.next_prompt_at_ms = next_prompt_at_ms;
            active.prompted = false;
            active.auto_stop_at_ms = None;
        }
    }

    pub(super) fn defer_end_prompt_for_later(&self, request_id: &str, now_ms: u64) {
        self.defer_end_prompt(request_id, now_ms.saturating_add(END_REMINDER_DELAY_MS));
    }

    pub(super) fn claim_due_auto_stop(
        &self,
        now_ms: u64,
        active_session_id: u64,
    ) -> Option<String> {
        let mut data = self.0.lock().ok()?;
        let active = data.active_meeting.as_mut()?;
        if active.recording_session_id != active_session_id
            || active.auto_stop_claimed
            || active
                .auto_stop_at_ms
                .is_none_or(|stop_at| now_ms < stop_at)
        {
            return None;
        }
        active.auto_stop_claimed = true;
        Some(active.end_request_id.clone())
    }

    pub(super) fn retry_auto_stop(&self, request_id: &str) {
        let Ok(mut data) = self.0.lock() else {
            return;
        };
        let Some(active) = data.active_meeting.as_mut() else {
            return;
        };
        if active.end_request_id == request_id {
            active.auto_stop_claimed = false;
        }
    }

    pub(super) fn retry_end_prompt(&self, request_id: &str) {
        let Ok(mut data) = self.0.lock() else {
            return;
        };
        let Some(active) = data.active_meeting.as_mut() else {
            return;
        };
        if active.end_request_id == request_id {
            active.prompted = false;
        }
    }

    pub(super) fn matches_active_end_prompt(
        &self,
        request_id: &str,
        active_session_id: u64,
    ) -> bool {
        self.0
            .lock()
            .ok()
            .and_then(|data| {
                data.active_meeting.as_ref().map(|active| {
                    active.end_request_id == request_id
                        && active.recording_session_id == active_session_id
                })
            })
            .unwrap_or(false)
    }

    pub(super) fn reset(&self) -> Vec<String> {
        if let Ok(mut data) = self.0.lock() {
            let mut request_ids = data.start_prompts.keys().cloned().collect::<Vec<_>>();
            if let Some(active) = &data.active_meeting {
                request_ids.push(active.end_request_id.clone());
            }
            let next_revision = data.auto_start_revision.wrapping_add(1);
            *data = SchedulerData {
                auto_start_revision: next_revision,
                ..SchedulerData::default()
            };
            return request_ids;
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests;
