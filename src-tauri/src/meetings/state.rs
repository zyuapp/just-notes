use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use super::model::Meeting;

#[derive(Clone, Default)]
pub(crate) struct MeetingSchedulerState(Arc<Mutex<SchedulerData>>);

#[derive(Default)]
struct SchedulerData {
    prompted_meetings: HashSet<String>,
    start_prompts: HashMap<String, Meeting>,
    active_meeting: Option<ActiveMeeting>,
}

struct ActiveMeeting {
    meeting: Meeting,
    recording_session_id: u64,
    end_request_id: String,
    next_prompt_at_ms: u64,
    prompted: bool,
}

impl MeetingSchedulerState {
    pub(super) fn register_start_prompt(&self, request_id: String, meeting: Meeting) -> bool {
        let Ok(mut data) = self.0.lock() else {
            return false;
        };
        if !data.prompted_meetings.insert(meeting.id.clone()) {
            return false;
        }
        data.start_prompts.insert(request_id, meeting);
        true
    }

    pub(super) fn take_start_prompt(&self, request_id: &str) -> Option<Meeting> {
        self.0.lock().ok()?.start_prompts.remove(request_id)
    }

    pub(super) fn restore_start_prompt(&self, request_id: String, meeting: Meeting) {
        if let Ok(mut data) = self.0.lock() {
            data.start_prompts.insert(request_id, meeting);
        }
    }

    pub(super) fn reconcile_start_prompts(
        &self,
        valid_meeting_ids: &HashSet<String>,
    ) -> Vec<String> {
        let Ok(mut data) = self.0.lock() else {
            return Vec::new();
        };
        let removed = data
            .start_prompts
            .iter()
            .filter(|(_, meeting)| !valid_meeting_ids.contains(&meeting.id))
            .map(|(request_id, _)| request_id.clone())
            .collect::<Vec<_>>();
        data.start_prompts
            .retain(|_, meeting| valid_meeting_ids.contains(&meeting.id));
        data.prompted_meetings
            .retain(|meeting_id| valid_meeting_ids.contains(meeting_id));
        removed
    }

    pub(super) fn clear_start_prompts(&self) -> Vec<String> {
        let Ok(mut data) = self.0.lock() else {
            return Vec::new();
        };
        let request_ids = data.start_prompts.keys().cloned().collect::<Vec<_>>();
        data.start_prompts.clear();
        data.prompted_meetings.clear();
        request_ids
    }

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

    pub(super) fn set_active(
        &self,
        meeting: Meeting,
        recording_session_id: u64,
        end_request_id: String,
    ) {
        if let Ok(mut data) = self.0.lock() {
            let next_prompt_at_ms = meeting.end_at_ms;
            data.active_meeting = Some(ActiveMeeting {
                meeting,
                recording_session_id,
                end_request_id,
                next_prompt_at_ms,
                prompted: false,
            });
        }
    }

    pub(super) fn due_end_prompt(
        &self,
        now_ms: u64,
        active_session_id: u64,
    ) -> Option<(String, Meeting)> {
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
        Some((active.end_request_id.clone(), active.meeting.clone()))
    }

    pub(super) fn defer_end_prompt(&self, request_id: &str, next_prompt_at_ms: u64) {
        let Ok(mut data) = self.0.lock() else {
            return;
        };
        let Some(active) = data.active_meeting.as_mut() else {
            return;
        };
        if active.end_request_id == request_id {
            active.next_prompt_at_ms = next_prompt_at_ms;
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
            *data = SchedulerData::default();
            return request_ids;
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests;
