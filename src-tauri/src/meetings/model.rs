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
pub(super) struct Meeting {
    pub(super) id: String,
    pub(super) calendar_id: String,
    pub(super) title: String,
    pub(super) start_at_ms: u64,
    pub(super) end_at_ms: u64,
}
