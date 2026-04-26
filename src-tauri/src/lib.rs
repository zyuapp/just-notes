use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};

mod app;
mod capture;
mod ipc;
mod recording;
mod threads;
mod transcription;

use app::AppPaths;
use ipc::{AppInfo, RecordingPayload};
use recording::RecorderState;
use threads::repository::{
    create_thread as create_thread_record, list_threads as list_thread_records, load_thread_by_id,
    reset_stale_recording_threads,
};
use threads::{ThreadDetail, ThreadSummary};
use transcription::{transcription_status, TranscriptionStatusPayload};

#[tauri::command]
fn get_app_info(paths: tauri::State<'_, AppPaths>) -> AppInfo {
    let paths = paths.inner();
    AppInfo {
        data_dir: paths.data_dir.display().to_string(),
        threads_dir: paths.threads_dir.display().to_string(),
        fixture_mode: cfg!(any(debug_assertions, feature = "qa-fixtures")),
    }
}

#[tauri::command]
fn list_threads(paths: tauri::State<'_, AppPaths>) -> Result<Vec<ThreadSummary>, String> {
    list_thread_records(paths.inner())
}

#[tauri::command]
fn create_thread(paths: tauri::State<'_, AppPaths>) -> Result<ThreadDetail, String> {
    create_thread_record(paths.inner())
}

#[tauri::command]
fn get_thread(
    paths: tauri::State<'_, AppPaths>,
    thread_id: String,
) -> Result<ThreadDetail, String> {
    load_thread_by_id(paths.inner(), &thread_id)
}

#[tauri::command]
fn get_transcription_status(
    paths: tauri::State<'_, AppPaths>,
) -> Result<TranscriptionStatusPayload, String> {
    Ok(transcription_status(paths.inner()))
}

#[tauri::command]
async fn start_recording(
    app: AppHandle,
    paths: tauri::State<'_, AppPaths>,
    recorder: tauri::State<'_, RecorderState>,
    thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        recording::start_recording(app, paths, recorder, thread_id)
    })
    .await
    .map_err(|err| format!("Audio startup task failed: {err}"))?
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
#[tauri::command]
async fn start_fixture_recording(
    app: AppHandle,
    paths: tauri::State<'_, AppPaths>,
    recorder: tauri::State<'_, RecorderState>,
    thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        recording::start_fixture_recording(app, paths, recorder, thread_id)
    })
    .await
    .map_err(|err| format!("Fixture startup task failed: {err}"))?
}

#[tauri::command]
async fn stop_recording(
    paths: tauri::State<'_, AppPaths>,
    recorder: tauri::State<'_, RecorderState>,
) -> Result<ThreadDetail, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || recording::stop_recording(paths, recorder))
        .await
        .map_err(|err| format!("Audio stop task failed: {err}"))?
}

pub(crate) fn now_ms() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("System clock is before UNIX epoch: {err}"))?
        .as_millis() as u64)
}

pub fn run() {
    let paths = AppPaths::discover().expect("failed to locate Just Notes data directory");

    let builder = tauri::Builder::default()
        .manage(paths)
        .manage(RecorderState::default())
        .setup(|app| {
            reset_stale_recording_threads(app.state::<AppPaths>().inner())?;
            Ok(())
        });

    #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_app_info,
        list_threads,
        create_thread,
        get_thread,
        get_transcription_status,
        start_recording,
        start_fixture_recording,
        stop_recording
    ]);

    #[cfg(not(any(debug_assertions, feature = "qa-fixtures")))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_app_info,
        list_threads,
        create_thread,
        get_thread,
        get_transcription_status,
        start_recording,
        stop_recording
    ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running Just Notes");
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, env, fs};

    use super::*;
    use crate::threads::{
        transcript_store::{append_live_segment, read_transcript_jsonl},
        TranscriptSegment,
    };
    use crate::transcription::{
        clean_transcript_text, common_transcript_prefix, completed_transcript_text,
        estimate_text_end_ms, first_audible_ms, unique_transcript_text, LiveChannelState,
    };

    const LIVE_SILENCE_RMS_THRESHOLD: f32 = 0.005;

    #[test]
    fn repeated_transcript_text_matches_overlapping_rollup() {
        let mut recent = VecDeque::new();
        recent.push_back("This is a Just Notes quality assurance test.".to_string());
        recent.push_back(
            "The blue notebook is beside the silver microphone. Every local transcript should preserve these exactly."
                .to_string(),
        );

        assert_eq!(unique_transcript_text(
            "This is a Just Notes quality assurance test. The blue notebook is beside the silver microphone. Every local transcript should preserve these exact words.",
            &recent,
        ), None);
    }

    #[test]
    fn repeated_transcript_text_allows_new_sentence() {
        let mut recent = VecDeque::new();
        recent.push_back("This is a Just Notes quality assurance test.".to_string());

        assert_eq!(
            unique_transcript_text(
                "Chapter two begins with a calendar reminder and a project checkpoint.",
                &recent,
            ),
            Some(
                "Chapter two begins with a calendar reminder and a project checkpoint.".to_string()
            )
        );
    }

    #[test]
    fn unique_transcript_text_trims_repeated_suffix() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 4 says the design notes mention a purple marker and a glass keyboard."
                .to_string(),
        );
        recent.push_back(
            "Section 5 says the engineering plan includes local storage. Audio can also include an"
                .to_string(),
        );

        assert_eq!(
            unique_transcript_text(
                "audio capture and live transcription. Section 4 says the design notes mention a purple marker and a glass keyboard. Section 5 says the engineering plan includes local storage, audio capture, and live transcription.",
                &recent,
            ),
            Some("audio capture and live transcription.".to_string()),
        );
    }

    #[test]
    fn unique_transcript_text_removes_repeated_middle_span() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 18 says the test is half-way through and the steady voice should continue."
                .to_string(),
        );

        assert_eq!(
            unique_transcript_text(
                "the local application and long-running stability. Section 18 says the test is half-way through and the steady voice should continue. Section 19 says the local app must not require cloud services for the transcript.",
                &recent,
            ),
            Some(
                "the local application and long-running stability. Section 19 says the local app must not require cloud services for the transcript."
                    .to_string()
            ),
        );
    }

    #[test]
    fn unique_transcript_text_trims_longest_repeated_prefix() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 1 says the green calendar moved beside the copper lamp.".to_string(),
        );

        assert_eq!(
            unique_transcript_text(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow folder stayed under the quiet monitor.",
                &recent,
            ),
            Some(
                "Section 2 says the yellow folder stayed under the quiet monitor.".to_string()
            ),
        );
    }

    #[test]
    fn unique_transcript_text_removes_internal_repeated_sentence() {
        let recent = VecDeque::new();

        assert_eq!(
            unique_transcript_text(
                "Section 21 says the navy notebook contains project tasks. Section 21 says the navy notebook contains project tasks.",
                &recent,
            ),
            Some("Section 21 says the navy notebook contains project tasks.".to_string()),
        );
    }

    #[test]
    fn unique_transcript_text_drops_short_duplicate_fragments() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 7 says the transcript should advance steadily without repeating earlier phrases."
                .to_string(),
        );

        assert_eq!(unique_transcript_text("phrases.", &recent), None);
    }

    #[test]
    fn unique_transcript_text_removes_numeric_sentence_artifacts() {
        let recent = VecDeque::new();

        assert_eq!(
            unique_transcript_text(
                "3. Section 4 says the design notes mention a purple marker and a glass keyboard. 4.",
                &recent,
            ),
            Some(
                "Section 4 says the design notes mention a purple marker and a glass keyboard."
                    .to_string()
            ),
        );
    }

    #[test]
    fn completed_transcript_text_drops_live_partial_sentence() {
        assert_eq!(
            completed_transcript_text(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow",
                false,
            ),
            "Section 1 says the green calendar moved beside the copper lamp.".to_string(),
        );
        assert_eq!(
            completed_transcript_text("Section 2 says the yellow", false),
            "".to_string(),
        );
        assert_eq!(
            completed_transcript_text("Section 2 says the yellow", true),
            "Section 2 says the yellow".to_string(),
        );
    }

    #[test]
    fn common_transcript_prefix_returns_agreed_words_from_latest_text() {
        assert_eq!(
            common_transcript_prefix(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow",
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow folder stayed under the quiet monitor.",
            ),
            Some(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow"
                    .to_string()
            ),
        );
    }

    #[test]
    fn live_channel_state_requires_two_matching_hypotheses() {
        let mut state = LiveChannelState::new("system", "Others", 48_000);
        assert_eq!(
            state.agreed_text(
                "Section 1 says the green calendar moved beside the copper lamp.",
                false,
            ),
            None,
        );
        assert_eq!(
            state.agreed_text(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow folder.",
                false,
            ),
            Some("Section 1 says the green calendar moved beside the copper lamp.".to_string()),
        );
    }

    #[test]
    fn clean_transcript_text_removes_leading_non_speech_marker() {
        assert_eq!(
            clean_transcript_text("(no audio) Long recording quality test begins now."),
            "Long recording quality test begins now.".to_string(),
        );
    }

    #[test]
    fn estimate_text_end_ms_tracks_confirmed_prefix() {
        let segments = vec![
            TranscriptSegment {
                speaker: "Others".to_string(),
                source: "system".to_string(),
                start_ms: 0,
                end_ms: 4_000,
                text: "Section 1 says the green calendar moved beside the copper lamp.".to_string(),
            },
            TranscriptSegment {
                speaker: "Others".to_string(),
                source: "system".to_string(),
                start_ms: 4_000,
                end_ms: 8_000,
                text: "Section 2 says the yellow folder stayed under the quiet monitor."
                    .to_string(),
            },
        ];

        assert_eq!(
            estimate_text_end_ms(
                &segments,
                "Section 1 says the green calendar moved beside the copper lamp.",
                10_000,
                22_000,
            ),
            14_000,
        );
    }

    #[test]
    fn first_audible_ms_skips_leading_silence() {
        let mut samples = vec![0.0; 16_000 * 3];
        samples.extend(vec![0.04; 16_000]);

        assert_eq!(
            first_audible_ms(&samples, 16_000, LIVE_SILENCE_RMS_THRESHOLD),
            Some(3_000)
        );
    }

    #[test]
    fn append_live_segment_keeps_jsonl_chronological() {
        let path = env::temp_dir().join(format!(
            "just-notes-transcript-order-{}.jsonl",
            now_ms().unwrap()
        ));
        let later = TranscriptSegment {
            speaker: "Others".to_string(),
            source: "system".to_string(),
            start_ms: 4_000,
            end_ms: 8_000,
            text: "System section two.".to_string(),
        };
        let earlier = TranscriptSegment {
            speaker: "You".to_string(),
            source: "mic".to_string(),
            start_ms: 3_000,
            end_ms: 12_000,
            text: "Microphone checkpoint alpha.".to_string(),
        };

        append_live_segment(&path, &later).unwrap();
        append_live_segment(&path, &earlier).unwrap();

        let segments = read_transcript_jsonl(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.source.as_str())
                .collect::<Vec<_>>(),
            vec!["mic", "system"],
        );
    }
}
