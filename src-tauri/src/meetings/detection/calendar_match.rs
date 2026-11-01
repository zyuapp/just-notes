use super::call_app::CallApp;
use crate::{
    meetings::{
        join::link::{join_link, link_host},
        model::Meeting,
    },
    platform::calendar::CalendarEvent,
};

/// How early a call may start before its calendar meeting and still be it.
pub(super) const EARLY_JOIN_MS: u64 = 10 * 60 * 1000;

/// The calendar meeting a detected call most likely belongs to: one under
/// way (or about to start) whose join link this app can open, else one with
/// other attendees and no link. A meeting whose link points at another
/// service never matches.
pub(super) fn matching_meeting(
    app: &CallApp,
    events: Vec<CalendarEvent>,
    now_ms: u64,
) -> Option<Meeting> {
    events
        .into_iter()
        .filter_map(|event| {
            let link = join_link(&[
                event.url.as_deref(),
                event.location.as_deref(),
                event.notes.as_deref(),
            ]);
            let meeting = Meeting::try_from(event).ok()?;
            let under_way = meeting.start_at_ms.saturating_sub(EARLY_JOIN_MS) <= now_ms
                && now_ms < meeting.end_at_ms;
            if !under_way {
                return None;
            }
            let rank = match link.as_deref().and_then(link_host) {
                Some(host) if app.opens_link_host(&host) => 2,
                Some(_) => return None,
                None if !meeting.attendees.is_empty() => 1,
                None => return None,
            };
            Some((rank, meeting))
        })
        .max_by_key(|(rank, meeting)| {
            (
                *rank,
                std::cmp::Reverse(meeting.start_at_ms.abs_diff(now_ms)),
            )
        })
        .map(|(_, meeting)| meeting)
}

#[cfg(test)]
mod tests;
