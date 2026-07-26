use super::{
    model::ScheduledMeeting,
    state::{classify_selected_thread, ThreadSelection},
};
use crate::{
    app::AppPaths,
    threads::{
        create::create_thread_with_external_title, repository::load_thread_by_id, ThreadDetail,
    },
};

pub(super) struct SelectedThread {
    pub(super) thread: ThreadDetail,
    pub(super) resume_offset_ms: Option<u64>,
    pub(super) newly_created: bool,
}

pub(super) fn select_recording_thread(
    paths: &AppPaths,
    requested_thread_id: Option<String>,
    scheduled_meeting: Option<ScheduledMeeting>,
) -> Result<SelectedThread, String> {
    let Some(thread_id) = requested_thread_id else {
        // A blank title falls back to the thread domain's own default.
        let (title, calendar) = match scheduled_meeting {
            Some(meeting) => {
                let (title, provenance) = meeting.into_thread_parts();
                (title, Some(provenance))
            }
            None => (String::new(), None),
        };
        return Ok(SelectedThread {
            thread: create_thread_with_external_title(paths, &title, calendar)?,
            resume_offset_ms: None,
            newly_created: true,
        });
    };

    let thread = load_thread_by_id(paths, &thread_id)?;
    match classify_selected_thread(thread)? {
        ThreadSelection::Reuse(thread) => Ok(SelectedThread {
            thread: *thread,
            resume_offset_ms: None,
            newly_created: false,
        }),
        ThreadSelection::Resume(thread) => Ok(SelectedThread {
            resume_offset_ms: Some(prior_recording_end_ms(&thread)),
            thread: *thread,
            newly_created: false,
        }),
    }
}

// A resumed session starts after the prior recording. Floor to the last segment
// end so a stale or zero stored duration can't drop new content before it.
fn prior_recording_end_ms(thread: &ThreadDetail) -> u64 {
    let last_segment_end_ms = thread
        .segments
        .iter()
        .map(|segment| segment.end_ms)
        .max()
        .unwrap_or(0);
    thread.summary.duration_ms.max(last_segment_end_ms)
}
