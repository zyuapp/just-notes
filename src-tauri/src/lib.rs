use std::{
    env, fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use tauri::Manager;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ThreadMetadata {
    id: String,
    title: String,
    created_at_ms: u64,
    updated_at_ms: u64,
    status: ThreadStatus,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum ThreadStatus {
    Idle,
    Recording,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ThreadSummary {
    id: String,
    title: String,
    created_at_ms: u64,
    updated_at_ms: u64,
    status: ThreadStatus,
    segment_count: usize,
    path: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TranscriptSegment {
    speaker: String,
    source: String,
    start_ms: u64,
    end_ms: u64,
    text: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ThreadDetail {
    summary: ThreadSummary,
    segments: Vec<TranscriptSegment>,
    transcript_markdown_path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    data_dir: String,
    threads_dir: String,
}

#[derive(Clone)]
struct AppPaths {
    data_dir: PathBuf,
    threads_dir: PathBuf,
}

impl AppPaths {
    fn discover() -> Result<Self, String> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set; cannot locate ~/.just-notes".to_string())?;
        let data_dir = home.join(".just-notes");
        Ok(Self {
            threads_dir: data_dir.join("threads"),
            data_dir,
        })
    }

    fn ensure(&self) -> Result<(), String> {
        fs::create_dir_all(&self.threads_dir).map_err(|err| {
            format!(
                "Failed to create thread storage at {}: {err}",
                self.threads_dir.display()
            )
        })
    }

    fn thread_dir(&self, thread_id: &str) -> PathBuf {
        self.threads_dir.join(thread_id)
    }
}

#[tauri::command]
fn list_threads(paths: tauri::State<'_, AppPaths>) -> Result<Vec<ThreadSummary>, String> {
    let paths = paths.inner();
    paths.ensure()?;

    let mut threads = fs::read_dir(&paths.threads_dir)
        .map_err(|err| format!("Failed to read {}: {err}", paths.threads_dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| load_thread_summary(&entry.path()).ok())
        .collect::<Vec<_>>();

    threads.sort_by(|left, right| {
        right
            .updated_at_ms
            .cmp(&left.updated_at_ms)
            .then_with(|| right.created_at_ms.cmp(&left.created_at_ms))
    });
    Ok(threads)
}

#[tauri::command]
fn create_thread(paths: tauri::State<'_, AppPaths>) -> Result<ThreadDetail, String> {
    let paths = paths.inner();
    paths.ensure()?;

    let now = now_ms()?;
    let id = format!("thread-{now}");
    let thread_dir = paths.thread_dir(&id);
    fs::create_dir_all(thread_dir.join("work")).map_err(|err| {
        format!(
            "Failed to create thread folder at {}: {err}",
            thread_dir.display()
        )
    })?;

    let metadata = ThreadMetadata {
        id,
        title: "Untitled thread".to_string(),
        created_at_ms: now,
        updated_at_ms: now,
        status: ThreadStatus::Idle,
    };
    save_thread_metadata(&thread_dir, &metadata)?;
    write_text_atomic(&thread_dir.join("transcript.md"), "# Untitled thread\n\n")?;

    load_thread_detail(&thread_dir)
}

#[tauri::command]
fn get_thread(paths: tauri::State<'_, AppPaths>, thread_id: String) -> Result<ThreadDetail, String> {
    let paths = paths.inner();
    let thread_dir = paths.thread_dir(&thread_id);
    if !thread_dir.is_dir() {
        return Err(format!("Thread does not exist: {thread_id}"));
    }

    load_thread_detail(&thread_dir)
}

#[tauri::command]
fn get_app_info(paths: tauri::State<'_, AppPaths>) -> AppInfo {
    let paths = paths.inner();
    AppInfo {
        data_dir: paths.data_dir.display().to_string(),
        threads_dir: paths.threads_dir.display().to_string(),
    }
}

fn load_thread_detail(thread_dir: &Path) -> Result<ThreadDetail, String> {
    let summary = load_thread_summary(thread_dir)?;
    let segments = read_transcript_jsonl(&thread_dir.join("transcript.jsonl"))?;
    Ok(ThreadDetail {
        summary,
        segments,
        transcript_markdown_path: thread_dir
            .join("transcript.md")
            .display()
            .to_string(),
    })
}

fn load_thread_summary(thread_dir: &Path) -> Result<ThreadSummary, String> {
    if !thread_dir.is_dir() {
        return Err(format!("Not a thread folder: {}", thread_dir.display()));
    }

    let metadata_path = thread_dir.join("thread.json");
    let metadata = read_thread_metadata(&metadata_path)?;
    Ok(ThreadSummary {
        id: metadata.id,
        title: metadata.title,
        created_at_ms: metadata.created_at_ms,
        updated_at_ms: metadata.updated_at_ms,
        status: metadata.status,
        segment_count: count_jsonl_lines(&thread_dir.join("transcript.jsonl"))?,
        path: thread_dir.display().to_string(),
    })
}

fn read_thread_metadata(path: &Path) -> Result<ThreadMetadata, String> {
    let json =
        fs::read_to_string(path).map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&json).map_err(|err| format!("Invalid {}: {err}", path.display()))
}

fn save_thread_metadata(thread_dir: &Path, metadata: &ThreadMetadata) -> Result<(), String> {
    let json = serde_json::to_string_pretty(metadata)
        .map_err(|err| format!("Failed to encode thread metadata: {err}"))?;
    write_text_atomic(&thread_dir.join("thread.json"), &json)
}

fn read_transcript_jsonl(path: &Path) -> Result<Vec<TranscriptSegment>, String> {
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

    Ok(segments)
}

fn count_jsonl_lines(path: &Path) -> Result<usize, String> {
    if !path.is_file() {
        return Ok(0);
    }

    let file = fs::File::open(path)
        .map_err(|err| format!("Failed to read transcript {}: {err}", path.display()))?;
    Ok(BufReader::new(file).lines().filter(|line| line.is_ok()).count())
}

fn write_text_atomic(path: &Path, content: &str) -> Result<(), String> {
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

fn now_ms() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("System clock is before UNIX epoch: {err}"))?
        .as_millis() as u64)
}

pub fn run() {
    let paths = AppPaths::discover().expect("failed to locate Just Notes data directory");

    tauri::Builder::default()
        .manage(paths)
        .setup(|app| {
            app.state::<AppPaths>().ensure()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            list_threads,
            create_thread,
            get_thread
        ])
        .run(tauri::generate_context!())
        .expect("error while running Just Notes");
}
