use crate::platform::calendar;

#[derive(Clone)]
pub(crate) struct MeetingCalendar {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) account: String,
    pub(crate) color: String,
}

#[derive(Clone)]
pub(crate) struct MeetingAccess {
    pub(crate) calendar_authorization: String,
    pub(crate) notification_authorization: String,
    pub(crate) calendars: Vec<MeetingCalendar>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Meeting {
    pub(crate) id: String,
    pub(crate) calendar_id: String,
    pub(crate) title: String,
    pub(crate) start_at_ms: u64,
    pub(crate) end_at_ms: u64,
    pub(crate) attendees: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeetingPrompt {
    pub(crate) request_id: String,
    pub(crate) meeting: Meeting,
    pub(crate) auto_start: bool,
    pub(crate) auto_start_after_ms: Option<u64>,
    pub(crate) auto_start_revision: u64,
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
            attendees: attendee_names(event.participants),
        })
    }
}

/// Names to remember a meeting by: everyone but the current user, who appears in
/// every meeting and so cannot narrow a search. Falls back to the invite address
/// when a participant has no display name, and keeps invite order.
fn attendee_names(participants: Vec<calendar::CalendarParticipant>) -> Vec<String> {
    let mut names = Vec::new();
    for participant in participants {
        if participant.is_current_user {
            continue;
        }
        let name = participant
            .name
            .filter(|name| !name.trim().is_empty())
            .or(participant.email)
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty());
        if let Some(name) = name {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}
