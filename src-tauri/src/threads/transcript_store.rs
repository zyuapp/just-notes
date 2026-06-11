use std::{
    fs,
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::Path,
};

use super::TranscriptSegment;

pub(crate) fn append_live_segment(path: &Path, segment: &TranscriptSegment) -> Result<(), String> {
    if let Some(last) = read_last_transcript_segment(path)? {
        if transcript_segment_order(&last, segment).is_gt() {
            let mut segments = read_transcript_jsonl(path)?;
            segments.push(segment.clone());
            segments.sort_by(transcript_segment_order);
            return write_transcript_jsonl(path, &segments);
        }
    }

    append_transcript_jsonl_line(path, segment)
}

pub(crate) fn read_transcript_jsonl(path: &Path) -> Result<Vec<TranscriptSegment>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path)
        .map_err(|err| format!("Failed to read transcript {}: {err}", path.display()))?;
    let reader = BufReader::new(file);
    let mut segments = Vec::new();

    for line in reader.lines() {
        let line =
            line.map_err(|err| format!("Failed to read transcript {}: {err}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        segments.push(
            serde_json::from_str::<TranscriptSegment>(&line)
                .map_err(|err| format!("Invalid transcript line in {}: {err}", path.display()))?,
        );
    }

    segments.sort_by(|left, right| {
        left.start_ms
            .cmp(&right.start_ms)
            .then_with(|| left.source.cmp(&right.source))
    });
    Ok(segments)
}

pub(crate) fn read_first_segment_text(path: &Path) -> Result<Option<String>, String> {
    if !path.is_file() {
        return Ok(None);
    }

    let file = fs::File::open(path)
        .map_err(|err| format!("Failed to read transcript {}: {err}", path.display()))?;
    let mut annotation_fallback: Option<String> = None;
    for line in BufReader::new(file).lines() {
        let line =
            line.map_err(|err| format!("Failed to read transcript {}: {err}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let segment = serde_json::from_str::<TranscriptSegment>(&line)
            .map_err(|err| format!("Invalid transcript line in {}: {err}", path.display()))?;
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }
        if is_non_speech_annotation(text) {
            annotation_fallback.get_or_insert_with(|| make_snippet(text));
            continue;
        }
        return Ok(Some(make_snippet(text)));
    }
    Ok(annotation_fallback)
}

fn make_snippet(text: &str) -> String {
    const SNIPPET_MAX_CHARS: usize = 120;
    let mut snippet: String = text.chars().take(SNIPPET_MAX_CHARS).collect();
    if text.chars().count() > SNIPPET_MAX_CHARS {
        snippet.push('…');
    }
    snippet
}

// Whisper renders non-speech sounds as a bracketed annotation like
// "(birds chirping)" or "[music]"; spoken text makes a better preview.
fn is_non_speech_annotation(text: &str) -> bool {
    let inner = trim_wrapped(text, '(', ')').or_else(|| trim_wrapped(text, '[', ']'));
    matches!(inner, Some(inner) if !inner.contains(['(', ')', '[', ']']))
}

fn trim_wrapped(text: &str, open: char, close: char) -> Option<&str> {
    text.strip_prefix(open)?.strip_suffix(close)
}

pub(crate) fn count_jsonl_lines(path: &Path) -> Result<usize, String> {
    if !path.is_file() {
        return Ok(0);
    }

    let file = fs::File::open(path)
        .map_err(|err| format!("Failed to read transcript {}: {err}", path.display()))?;
    Ok(BufReader::new(file)
        .lines()
        .filter(|line| line.is_ok())
        .count())
}

pub(crate) fn write_text_atomic(path: &Path, content: &str) -> Result<(), String> {
    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)
        .map_err(|err| format!("Failed to create {}: {err}", tmp_path.display()))?;
    file.write_all(content.as_bytes())
        .map_err(|err| format!("Failed to write {}: {err}", tmp_path.display()))?;
    file.sync_all()
        .map_err(|err| format!("Failed to sync {}: {err}", tmp_path.display()))?;
    fs::rename(&tmp_path, path).map_err(|err| {
        format!(
            "Failed to replace {} with {}: {err}",
            path.display(),
            tmp_path.display()
        )
    })
}

fn append_transcript_jsonl_line(path: &Path, segment: &TranscriptSegment) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("Failed to open transcript {}: {err}", path.display()))?;
    let line = serde_json::to_string(segment)
        .map_err(|err| format!("Failed to encode transcript segment: {err}"))?;
    writeln!(file, "{line}")
        .map_err(|err| format!("Failed to append transcript {}: {err}", path.display()))
}

fn read_last_transcript_segment(path: &Path) -> Result<Option<TranscriptSegment>, String> {
    if !path.is_file() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read transcript {}: {err}", path.display()))?;
    let Some(line) = content.lines().rev().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    serde_json::from_str(line)
        .map(Some)
        .map_err(|err| format!("Invalid transcript line in {}: {err}", path.display()))
}

pub(crate) fn write_transcript_jsonl(
    path: &Path,
    segments: &[TranscriptSegment],
) -> Result<(), String> {
    let mut content = String::new();
    for segment in segments {
        let line = serde_json::to_string(segment)
            .map_err(|err| format!("Failed to encode transcript segment: {err}"))?;
        content.push_str(&line);
        content.push('\n');
    }
    write_text_atomic(path, &content)
}

fn transcript_segment_order(
    left: &TranscriptSegment,
    right: &TranscriptSegment,
) -> std::cmp::Ordering {
    left.start_ms
        .cmp(&right.start_ms)
        .then_with(|| left.end_ms.cmp(&right.end_ms))
        .then_with(|| left.source.cmp(&right.source))
}

#[cfg(test)]
mod tests {
    use std::{env, fs, time::UNIX_EPOCH};

    use super::{append_live_segment, read_first_segment_text, read_transcript_jsonl};
    use crate::threads::TranscriptSegment;

    fn temp_jsonl(name: &str) -> std::path::PathBuf {
        let stamp = UNIX_EPOCH.elapsed().unwrap().as_nanos();
        env::temp_dir().join(format!("just-notes-{name}-{stamp}.jsonl"))
    }

    fn segment(
        speaker: &str,
        source: &str,
        start_ms: u64,
        end_ms: u64,
        text: &str,
    ) -> TranscriptSegment {
        TranscriptSegment {
            speaker: speaker.to_string(),
            source: source.to_string(),
            start_ms,
            end_ms,
            text: text.to_string(),
        }
    }

    #[test]
    fn append_live_segment_keeps_jsonl_chronological() {
        let path = temp_jsonl("transcript-order");
        let later = segment("Others", "system", 4_000, 8_000, "System section two.");
        let earlier = segment("You", "mic", 3_000, 12_000, "Microphone checkpoint alpha.");

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

    #[test]
    fn snippet_skips_non_speech_annotations_unless_nothing_else_exists() {
        let path = temp_jsonl("transcript-snippet");
        let chirp = segment("You", "mic", 0, 2_000, "(birds chirping)");
        let speech = segment("Others", "system", 1_000, 4_000, "Actual spoken words.");

        append_live_segment(&path, &chirp).unwrap();
        let annotation_only = read_first_segment_text(&path).unwrap();

        append_live_segment(&path, &speech).unwrap();
        let with_speech = read_first_segment_text(&path).unwrap();

        let _ = fs::remove_file(&path);
        assert_eq!(annotation_only.as_deref(), Some("(birds chirping)"));
        assert_eq!(with_speech.as_deref(), Some("Actual spoken words."));
    }
}
