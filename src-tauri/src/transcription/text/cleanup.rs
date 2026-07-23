use super::words::normalized_words;

/// Filler-only decodes that local ASR models commonly hallucinate on faint
/// non-speech audio (room tone, breaths, distant murmur). Applied only to
/// utterances whose audio never reaches a confident speech level, so loud
/// real backchannels ("yeah", "okay") are unaffected.
pub(crate) fn is_probable_filler_text(text: &str) -> bool {
    const FILLER_WORDS: [&str; 28] = [
        "mm", "mmm", "hmm", "hm", "mhm", "mhmm", "mmhm", "mmhmm", "mmmhmm", "uh", "um", "uhhuh",
        "huh", "oh", "ah", "okay", "ok", "yeah", "yes", "yep", "yup", "cool", "right", "sure",
        "alright", "thank", "thanks", "you",
    ];
    let words = normalized_words(text);
    !words.is_empty()
        && words.len() <= 3
        && words
            .iter()
            .all(|word| FILLER_WORDS.contains(&word.as_str()))
}

pub(crate) fn is_ignored_transcript_text(text: &str) -> bool {
    matches!(
        text.trim().to_ascii_lowercase().as_str(),
        "[blank_audio]" | "[silence]" | "(silence)" | "[music]" | "(music)"
    )
}

pub(crate) fn clean_transcript_text(text: &str) -> String {
    let mut cleaned = text.trim();
    loop {
        let lower = cleaned.to_ascii_lowercase();
        let Some(prefix) = [
            "[blank_audio]",
            "[silence]",
            "(silence)",
            "[music]",
            "(music)",
            "(no audio)",
        ]
        .iter()
        .find(|prefix| lower.starts_with(**prefix)) else {
            break;
        };
        cleaned = cleaned[prefix.len()..].trim();
    }
    cleaned.to_string()
}

#[cfg(test)]
mod tests {
    use super::{clean_transcript_text, is_probable_filler_text};

    #[test]
    fn clean_transcript_text_removes_leading_non_speech_marker() {
        assert_eq!(
            clean_transcript_text("(no audio) Long recording quality test begins now."),
            "Long recording quality test begins now.".to_string(),
        );
    }

    #[test]
    fn filler_only_text_is_flagged() {
        for text in [
            "Mm-hmm.",
            "Mmm-hmm.",
            "Mm hm",
            "Uh-huh.",
            "Okay.",
            "Yeah!",
            "Okay, yeah.",
            "Thank you.",
        ] {
            assert!(is_probable_filler_text(text), "did not flag {text:?}");
        }
    }

    #[test]
    fn real_speech_is_not_flagged_as_filler() {
        for text in [
            "Yeah, let me check the logs.",
            "Okay, I will send it.",
            "No.",
            "You should retry.",
            "Thank you for the review.",
            "",
        ] {
            assert!(!is_probable_filler_text(text), "falsely flagged {text:?}");
        }
    }
}
