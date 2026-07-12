#[derive(Clone)]
pub(crate) struct MeetingCalendar {
    pub(crate) id: String,
    pub(crate) title: String,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeetingPrompt {
    pub(crate) request_id: String,
    pub(crate) meeting: Meeting,
}
