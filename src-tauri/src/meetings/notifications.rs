use crate::platform::notifications::{self, NotificationActionSpec, NotificationCategorySpec};

pub(super) const START_ACTION: &str = "meeting-start";
pub(super) const SKIP_ACTION: &str = "meeting-skip";
pub(super) const STOP_ACTION: &str = "meeting-stop";
pub(super) const KEEP_ACTION: &str = "meeting-keep-recording";

const START_CATEGORY: &str = "meeting-start-prompt";
const END_CATEGORY: &str = "meeting-end-prompt";

pub(crate) fn categories() -> Vec<NotificationCategorySpec> {
    vec![
        NotificationCategorySpec {
            id: START_CATEGORY,
            actions: vec![
                NotificationActionSpec {
                    id: START_ACTION,
                    title: "Start recording",
                    foreground: true,
                },
                NotificationActionSpec {
                    id: SKIP_ACTION,
                    title: "Skip",
                    foreground: false,
                },
            ],
        },
        NotificationCategorySpec {
            id: END_CATEGORY,
            actions: vec![
                NotificationActionSpec {
                    id: STOP_ACTION,
                    title: "Stop recording",
                    foreground: false,
                },
                NotificationActionSpec {
                    id: KEEP_ACTION,
                    title: "Keep recording",
                    foreground: false,
                },
            ],
        },
    ]
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
