use std::collections::HashSet;

use crate::meetings::model::{Meeting, MeetingPrompt};

use super::MeetingSchedulerState;

const AUTO_START_LATE_GRACE_MS: u64 = 60 * 1000;

impl MeetingSchedulerState {
    pub(in crate::meetings) fn start_prompt_is_automatic(&self, request_id: &str) -> bool {
        self.0
            .lock()
            .ok()
            .and_then(|data| {
                data.start_prompts
                    .get(request_id)
                    .map(|prompt| prompt.auto_start)
            })
            .unwrap_or(false)
    }

    pub(in crate::meetings) fn upsert_start_prompt(
        &self,
        request_id: String,
        meeting: Meeting,
        auto_start: bool,
    ) -> bool {
        let Ok(mut data) = self.0.lock() else {
            return false;
        };
        let auto_start_revision = data.auto_start_revision;
        if let Some(prompt) = data.start_prompts.get_mut(&request_id) {
            if prompt.meeting == meeting && prompt.auto_start == auto_start {
                return false;
            }
            *prompt = MeetingPrompt {
                request_id,
                meeting,
                auto_start,
                auto_start_after_ms: None,
                auto_start_revision,
            };
            return true;
        }
        if !data.prompted_meetings.insert(meeting.id.clone()) {
            return false;
        }
        data.start_prompts.insert(
            request_id.clone(),
            MeetingPrompt {
                request_id,
                meeting,
                auto_start,
                auto_start_after_ms: None,
                auto_start_revision,
            },
        );
        true
    }

    pub(in crate::meetings) fn take_start_prompt(&self, request_id: &str) -> Option<MeetingPrompt> {
        self.0.lock().ok()?.start_prompts.remove(request_id)
    }

    pub(crate) fn current_start_prompt(&self) -> Option<MeetingPrompt> {
        let data = self.0.lock().ok()?;
        data.start_prompts
            .iter()
            .min_by_key(|(_, prompt)| (prompt.meeting.start_at_ms, &prompt.meeting.id))
            .map(|(_, prompt)| prompt.clone())
    }

    pub(crate) fn dismiss_start_prompt(&self, request_id: &str) -> bool {
        self.0
            .lock()
            .ok()
            .and_then(|mut data| data.start_prompts.remove(request_id))
            .is_some()
    }

    pub(in crate::meetings) fn restore_start_prompt(&self, prompt: MeetingPrompt) {
        if let Ok(mut data) = self.0.lock() {
            if prompt.auto_start_revision != data.auto_start_revision {
                return;
            }
            data.start_prompts.insert(prompt.request_id.clone(), prompt);
        }
    }

    pub(in crate::meetings) fn arm_auto_start(
        &self,
        request_id: &str,
        expected_meeting: &Meeting,
        auto_start_after_ms: u64,
    ) -> bool {
        let Ok(mut data) = self.0.lock() else {
            return false;
        };
        let Some(prompt) = data.start_prompts.get_mut(request_id) else {
            return false;
        };
        if !prompt.auto_start || prompt.meeting != *expected_meeting {
            return false;
        }
        prompt.auto_start_after_ms = Some(auto_start_after_ms);
        true
    }

    pub(in crate::meetings) fn fallback_to_manual_start(
        &self,
        request_id: &str,
        expected_meeting: &Meeting,
    ) {
        let Ok(mut data) = self.0.lock() else {
            return;
        };
        let Some(prompt) = data.start_prompts.get_mut(request_id) else {
            return;
        };
        if prompt.meeting == *expected_meeting {
            prompt.auto_start = false;
            prompt.auto_start_after_ms = None;
        }
    }

    pub(in crate::meetings) fn take_due_auto_start(&self, now_ms: u64) -> Option<String> {
        let mut data = self.0.lock().ok()?;
        let (request_id, meeting_id) = data
            .start_prompts
            .iter()
            .filter(|(_, prompt)| {
                prompt.auto_start
                    && prompt
                        .auto_start_after_ms
                        .is_some_and(|start_after_ms| now_ms >= start_after_ms)
                    && now_ms >= prompt.meeting.start_at_ms
                    && now_ms
                        < prompt
                            .meeting
                            .start_at_ms
                            .saturating_add(AUTO_START_LATE_GRACE_MS)
                    && now_ms < prompt.meeting.end_at_ms
            })
            .filter(|(_, prompt)| !data.auto_start_attempted.contains(&prompt.meeting.id))
            .min_by_key(|(_, prompt)| (prompt.meeting.start_at_ms, &prompt.meeting.id))
            .map(|(request_id, prompt)| (request_id.clone(), prompt.meeting.id.clone()))?;
        data.auto_start_attempted.insert(meeting_id);
        Some(request_id)
    }

    pub(in crate::meetings) fn with_auto_start_consent<T>(
        &self,
        revision: u64,
        start: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let data = self
            .0
            .lock()
            .map_err(|_| "Meeting automation state is unavailable".to_string())?;
        if data.auto_start_revision != revision {
            return Err("Automatic recording was disabled".to_string());
        }
        start()
    }

    pub(in crate::meetings) fn reconcile_start_prompts(
        &self,
        valid_meeting_ids: &HashSet<String>,
    ) -> Vec<String> {
        let Ok(mut data) = self.0.lock() else {
            return Vec::new();
        };
        let removed = data
            .start_prompts
            .iter()
            .filter(|(_, prompt)| !valid_meeting_ids.contains(&prompt.meeting.id))
            .map(|(request_id, _)| request_id.clone())
            .collect::<Vec<_>>();
        data.start_prompts
            .retain(|_, prompt| valid_meeting_ids.contains(&prompt.meeting.id));
        data.prompted_meetings
            .retain(|meeting_id| valid_meeting_ids.contains(meeting_id));
        data.auto_start_attempted
            .retain(|meeting_id| valid_meeting_ids.contains(meeting_id));
        removed
    }

    pub(in crate::meetings) fn clear_start_prompts(&self) -> Vec<String> {
        let Ok(mut data) = self.0.lock() else {
            return Vec::new();
        };
        let request_ids = data.start_prompts.keys().cloned().collect::<Vec<_>>();
        data.start_prompts.clear();
        data.prompted_meetings.clear();
        data.auto_start_attempted.clear();
        data.auto_start_revision = data.auto_start_revision.wrapping_add(1);
        request_ids
    }
}
