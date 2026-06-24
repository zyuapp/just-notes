use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::Path,
};

use super::TranscriptSegment;

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
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // A crash mid-append can leave a torn last line; skip unparseable lines
        // instead of failing the whole load.
        if let Ok(segment) = serde_json::from_str::<TranscriptSegment>(trimmed) {
            segments.push(segment);
        }
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

// Non-speech sounds are rendered as a bracketed annotation like
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

/// Appends segments as JSONL lines, creating the file if needed. O(1) per call,
/// used by live transcription; a crash can leave a torn final line, which
/// [`read_transcript_jsonl`] tolerates.
pub(crate) fn append_transcript_jsonl(
    path: &Path,
    segments: &[TranscriptSegment],
) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("Failed to open transcript {}: {err}", path.display()))?;
    for segment in segments {
        let line = serde_json::to_string(segment)
            .map_err(|err| format!("Failed to encode transcript segment: {err}"))?;
        writeln!(file, "{line}")
            .map_err(|err| format!("Failed to append transcript {}: {err}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{env, fs, time::UNIX_EPOCH};

    use super::{read_first_segment_text, write_transcript_jsonl};
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
    fn snippet_skips_non_speech_annotations_unless_nothing_else_exists() {
        let path = temp_jsonl("transcript-snippet");
        let chirp = segment("You", "mic", 0, 2_000, "(birds chirping)");
        let speech = segment("Others", "system", 1_000, 4_000, "Actual spoken words.");

        write_transcript_jsonl(&path, std::slice::from_ref(&chirp)).unwrap();
        let annotation_only = read_first_segment_text(&path).unwrap();

        write_transcript_jsonl(&path, &[chirp, speech]).unwrap();
        let with_speech = read_first_segment_text(&path).unwrap();

        let _ = fs::remove_file(&path);
        assert_eq!(annotation_only.as_deref(), Some("(birds chirping)"));
        assert_eq!(with_speech.as_deref(), Some("Actual spoken words."));
    }
}
