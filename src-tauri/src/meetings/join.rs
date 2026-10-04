use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use super::{model::Meeting, notifications, scheduler::notification_id};
use crate::platform::calendar::CalendarEvent;

mod link;

const JOIN_LEAD_MS: u64 = 60 * 1000;
const JOIN_LATE_GRACE_MS: u64 = 5 * 60 * 1000;

type JoinReminders = HashMap<String, JoinReminder>;

/// Join reminders delivered for the current calendar window, keyed by meeting id.
#[derive(Clone, Default)]
pub(crate) struct JoinReminderState(Arc<Mutex<JoinReminders>>);

#[derive(Clone, Debug, PartialEq, Eq)]
struct JoinReminder {
    request_id: String,
    join_url: String,
}

#[derive(Debug, PartialEq, Eq)]
struct DueJoinReminder {
    request_id: String,
    meeting_title: String,
    started: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct JoinReconciliation {
    removed: Vec<String>,
    due: Vec<DueJoinReminder>,
}

impl JoinReminderState {
    pub(super) fn join_url(&self, request_id: &str) -> Option<String> {
        self.0.lock().ok()?.values().find_map(|reminder| {
            (reminder.request_id == request_id).then(|| reminder.join_url.clone())
        })
    }

    pub(super) fn clear(&self) -> Vec<String> {
        self.0
            .lock()
            .map(|mut reminders| {
                reminders
                    .drain()
                    .map(|(_, reminder)| reminder.request_id)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Drops reminders whose meeting ended or left the calendar, refreshes links
    /// of delivered ones, and claims each meeting that just entered its join window.
    fn reconcile(&self, meetings: Vec<(Meeting, String)>, now_ms: u64) -> JoinReconciliation {
        let Ok(mut reminders) = self.0.lock() else {
            return JoinReconciliation::default();
        };
        let live_ids = meetings
            .iter()
            .filter(|(meeting, _)| now_ms < meeting.end_at_ms)
            .map(|(meeting, _)| meeting.id.clone())
            .collect::<HashSet<_>>();
        let mut outcome = JoinReconciliation::default();
        reminders.retain(|meeting_id, reminder| {
            let live = live_ids.contains(meeting_id);
            if !live {
                outcome.removed.push(reminder.request_id.clone());
            }
            live
        });
        for (meeting, join_url) in meetings {
            if let Some(reminder) = reminders.get_mut(&meeting.id) {
                reminder.join_url = join_url;
                continue;
            }
            if !join_reminder_is_due(&meeting, now_ms) {
                continue;
            }
            let request_id = notification_id("join", &meeting.id);
            reminders.insert(
                meeting.id.clone(),
                JoinReminder {
                    request_id: request_id.clone(),
                    join_url,
                },
            );
            outcome.due.push(DueJoinReminder {
                request_id,
                meeting_title: meeting.title,
                started: now_ms >= meeting.start_at_ms,
            });
        }
        outcome
    }
}

pub(super) fn refresh(state: &JoinReminderState, events: &[CalendarEvent], now_ms: u64) {
    let meetings = events
        .iter()
        .filter_map(|event| {
            let join_url = link::join_link(&[
                event.url.as_deref(),
                event.location.as_deref(),
                event.notes.as_deref(),
            ])?;
            let meeting = Meeting::try_from(event.clone()).ok()?;
            Some((meeting, join_url))
        })
        .collect();
    let outcome = state.reconcile(meetings, now_ms);
    notifications::remove(&outcome.removed);
    for reminder in outcome.due {
        notifications::show_join_reminder(
            &reminder.request_id,
            &reminder.meeting_title,
            reminder.started,
        );
    }
}

fn join_reminder_is_due(meeting: &Meeting, now_ms: u64) -> bool {
    now_ms >= meeting.start_at_ms.saturating_sub(JOIN_LEAD_MS)
        && now_ms < meeting.start_at_ms.saturating_add(JOIN_LATE_GRACE_MS)
        && now_ms < meeting.end_at_ms
}

#[cfg(test)]
mod tests;
