use crate::platform::notifications::{self, NotificationActionSpec, NotificationCategorySpec};

pub(super) const START_ACTION: &str = "meeting-start";
pub(super) const SKIP_ACTION: &str = "meeting-skip";
pub(super) const STOP_ACTION: &str = "meeting-stop";
pub(super) const KEEP_ACTION: &str = "meeting-keep-recording";
pub(super) const JOIN_ACTION: &str = "meeting-join";
pub(super) const CALL_RECORD_ACTION: &str = "call-record";
pub(super) const CALL_DISMISS_ACTION: &str = "call-dismiss";
pub(super) const CALL_IGNORE_ACTION: &str = "call-ignore";
pub(super) const CALL_STOP_ACTION: &str = "call-stop";
pub(super) const CALL_KEEP_ACTION: &str = "call-keep";

const START_CATEGORY: &str = "meeting-start-prompt";
const END_CATEGORY: &str = "meeting-end-prompt";
const JOIN_CATEGORY: &str = "meeting-join-reminder";
const CALL_OFFER_CATEGORY: &str = "call-offer";
const CALL_ENDED_CATEGORY: &str = "call-ended";

pub(crate) fn categories() -> Vec<NotificationCategorySpec> {
    vec![
        NotificationCategorySpec {
            id: START_CATEGORY,
            actions: vec![
                action(START_ACTION, "Start recording", true),
                action(SKIP_ACTION, "Skip", false),
            ],
        },
        NotificationCategorySpec {
            id: END_CATEGORY,
            actions: vec![
                action(STOP_ACTION, "Stop recording", false),
                action(KEEP_ACTION, "Keep recording", false),
            ],
        },
        NotificationCategorySpec {
            id: JOIN_CATEGORY,
            actions: vec![action(JOIN_ACTION, "Join meeting", false)],
        },
        NotificationCategorySpec {
            id: CALL_OFFER_CATEGORY,
            actions: vec![
                action(CALL_RECORD_ACTION, "Record", true),
                action(CALL_DISMISS_ACTION, "Not now", false),
                action(CALL_IGNORE_ACTION, "Never for this app", false),
            ],
        },
        NotificationCategorySpec {
            id: CALL_ENDED_CATEGORY,
            actions: vec![
                action(CALL_STOP_ACTION, "Stop recording", false),
                action(CALL_KEEP_ACTION, "Keep recording", false),
            ],
        },
    ]
}

fn action(id: &'static str, title: &'static str, foreground: bool) -> NotificationActionSpec {
    NotificationActionSpec {
        id,
        title,
        foreground,
    }
}

pub(super) fn show_call_offer(
    request_id: &str,
    title: &str,
    body: &str,
    on_error: impl Fn(String) + Send + Sync + 'static,
) {
    notifications::show_with_error_handler(request_id, title, body, CALL_OFFER_CATEGORY, on_error);
}

pub(super) fn show_call_recording_started(request_id: &str, title: &str, body: &str) {
    notifications::show_informational(request_id, title, body);
}

pub(super) fn show_call_ended(request_id: &str, app_title: &str) {
    notifications::show(
        request_id,
        &format!("{app_title} ended"),
        "The call left the microphone. Stop recording?",
        CALL_ENDED_CATEGORY,
    );
}

pub(super) fn show_start_prompt(request_id: &str, meeting_title: &str, timing: &str) {
    notifications::show(
        request_id,
        "Meeting starting soon",
        &format!("{meeting_title} starts {timing}. Start recording?"),
        START_CATEGORY,
    );
}

pub(super) fn show_automatic_start_prompt(
    request_id: &str,
    meeting_title: &str,
    timing: &str,
    on_complete: impl Fn(Result<(), String>) + Send + Sync + 'static,
) {
    notifications::show_with_completion_handler(
        request_id,
        "Meeting recording scheduled",
        &format!("{meeting_title} starts {timing}. Recording will start automatically."),
        START_CATEGORY,
        on_complete,
    );
}

pub(super) fn show_end_prompt(
    request_id: &str,
    meeting_title: &str,
    auto_stop: bool,
    on_error: impl Fn(String) + Send + Sync + 'static,
) {
    let (title, body) = if auto_stop {
        (
            "Recording stops in one minute",
            format!(
                "{meeting_title} is scheduled to end. Keep recording if the meeting runs over."
            ),
        )
    } else {
        (
            "Meeting scheduled to end",
            format!("{meeting_title} was scheduled to end. Stop recording?"),
        )
    };
    notifications::show_with_error_handler(request_id, title, &body, END_CATEGORY, on_error);
}

pub(super) fn show_join_reminder(
    request_id: &str,
    meeting_title: &str,
    started: bool,
    on_error: impl Fn(String) + Send + Sync + 'static,
) {
    let title = if started {
        "Meeting has started"
    } else {
        "Meeting starts in 1 minute"
    };
    notifications::show_with_error_handler(
        request_id,
        title,
        &format!("{meeting_title}. Click to join."),
        JOIN_CATEGORY,
        on_error,
    );
}

pub(crate) fn show_start_failure(request_id: &str, meeting_title: &str, error: &str) {
    notifications::show(
        request_id,
        "Recording could not start",
        &format!("{meeting_title}: {error}. Try again?"),
        START_CATEGORY,
    );
}

pub(crate) fn remove(request_ids: &[String]) {
    notifications::remove(request_ids);
}
