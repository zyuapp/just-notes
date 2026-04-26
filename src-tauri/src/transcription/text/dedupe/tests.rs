use std::collections::VecDeque;

use super::unique_transcript_text;

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
        Some("Chapter two begins with a calendar reminder and a project checkpoint.".to_string())
    );
}

#[test]
fn unique_transcript_text_trims_repeated_suffix() {
    let mut recent = VecDeque::new();
    recent.push_back(
        "Section 4 says the design notes mention a purple marker and a glass keyboard.".to_string(),
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
    recent.push_back("Section 1 says the green calendar moved beside the copper lamp.".to_string());

    assert_eq!(
        unique_transcript_text(
            "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow folder stayed under the quiet monitor.",
            &recent,
        ),
        Some("Section 2 says the yellow folder stayed under the quiet monitor.".to_string()),
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
