use crate::meetings::model::{Meeting, MeetingPrompt};

use super::{start::AUTO_START_LATE_GRACE_MS, MeetingSchedulerState};

impl MeetingSchedulerState {
    /// Removes a start prompt the user turned down and remembers the meeting,
    /// so a call detected during it is not offered either.
    pub(in crate::meetings) fn decline_start_prompt(
        &self,
        request_id: &str,
    ) -> Option<MeetingPrompt> {
        let mut data = self.0.lock().ok()?;
        let prompt = data.start_prompts.remove(request_id)?;
        data.declined_meetings
            .insert(prompt.meeting.id.clone(), prompt.meeting.end_at_ms);
        Some(prompt)
    }

    /// Whether the calendar flow already decided this meeting: the user
    /// skipped it, or an automatic start is armed and still due.
    pub(in crate::meetings) fn calendar_owns_call(&self, meeting: &Meeting, now_ms: u64) -> bool {
        let Ok(mut data) = self.0.lock() else {
            return false;
        };
        data.declined_meetings
            .retain(|_, end_at_ms| now_ms < *end_at_ms);
        if data.declined_meetings.contains_key(&meeting.id) {
            return true;
        }
        let attempted = data.auto_start_attempted.contains(&meeting.id);
        data.start_prompts.values().any(|prompt| {
            prompt.meeting.id == meeting.id
                && prompt.auto_start
                && prompt.auto_start_after_ms.is_some()
                && !attempted
                && now_ms
                    < prompt
                        .meeting
                        .start_at_ms
                        .saturating_add(AUTO_START_LATE_GRACE_MS)
        })
    }

    /// Marks a meeting as recorded outside its start prompt, returning the
    /// prompts to withdraw so the calendar does not ask again.
    pub(in crate::meetings) fn claim_meeting(&self, meeting: &Meeting) -> Vec<String> {
        let Ok(mut data) = self.0.lock() else {
            return Vec::new();
        };
        data.prompted_meetings.insert(meeting.id.clone());
        let claimed = data
            .start_prompts
            .iter()
            .filter(|(_, prompt)| prompt.meeting.id == meeting.id)
            .map(|(request_id, _)| request_id.clone())
            .collect::<Vec<_>>();
        for request_id in &claimed {
            data.start_prompts.remove(request_id);
        }
        claimed
    }
}

#[cfg(test)]
mod tests;
