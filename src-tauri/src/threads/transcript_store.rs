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

fn append_transcript_jsonl_line(
    path: &Path,
    segment: &TranscriptSegment,
) -> Result<(), String> {
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

fn write_transcript_jsonl(path: &Path, segments: &[TranscriptSegment]) -> Result<(), String> {
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
