use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{metrics::FixtureMetrics, SUPPORTED_MODE_NAMES};

type FixtureModes = Option<Vec<String>>;

#[derive(Deserialize)]
struct ReferenceFile {
    segments: Vec<ReferenceSegment>,
    modes: FixtureModes,
}

#[derive(Deserialize)]
pub(super) struct ReferenceSegment {
    pub(super) source: String,
    pub(super) start_ms: u64,
    pub(super) end_ms: u64,
    pub(super) text: String,
}

pub(super) struct Fixture {
    pub(super) name: String,
    pub(super) mic_path: PathBuf,
    pub(super) system_path: PathBuf,
    pub(super) reference: Vec<ReferenceSegment>,
    modes: FixtureModes,
}

impl Fixture {
    pub(super) fn runs_mode(&self, mode: &str) -> bool {
        self.modes
            .as_ref()
            .is_none_or(|modes| modes.iter().any(|candidate| candidate == mode))
    }
}

pub(super) fn validate_modes(modes: FixtureModes) -> Result<FixtureModes, String> {
    let Some(modes) = modes else {
        return Ok(None);
    };
    if modes.is_empty() {
        return Err("fixture modes must not be empty".to_string());
    }
    if let Some(mode) = modes
        .iter()
        .find(|mode| !SUPPORTED_MODE_NAMES.contains(&mode.as_str()))
    {
        return Err(format!("unsupported fixture mode {mode:?}"));
    }
    Ok(Some(modes))
}

/// Fixture directories under the fixtures root, sorted by name. A directory
/// without a `reference.json` is skipped so partial setups fail loudly at the
/// empty-fixtures assert instead of mid-run.
pub(super) fn discover_fixtures() -> Vec<Fixture> {
    let root = fixtures_dir();
    let Ok(entries) = fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();

    dirs.into_iter()
        .filter_map(|dir| {
            let reference_path = dir.join("reference.json");
            if !reference_path.is_file() {
                return None;
            }
            let raw = fs::read_to_string(&reference_path)
                .unwrap_or_else(|err| panic!("read {}: {err}", reference_path.display()));
            let parsed: ReferenceFile = serde_json::from_str(&raw)
                .unwrap_or_else(|err| panic!("parse {}: {err}", reference_path.display()));
            let modes = validate_modes(parsed.modes)
                .unwrap_or_else(|err| panic!("parse {}: {err}", reference_path.display()));
            Some(Fixture {
                name: dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                mic_path: dir.join("mic.wav"),
                system_path: dir.join("system.wav"),
                reference: parsed.segments,
                modes,
            })
        })
        .collect()
}

fn fixtures_dir() -> PathBuf {
    if let Some(dir) = env::var_os("QUALITY_FIXTURES_DIR") {
        return PathBuf::from(dir);
    }
    let home = env::var_os("HOME").expect("HOME is not set");
    PathBuf::from(home)
        .join(".just-notes")
        .join("quality-fixtures")
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub(super) struct BaselineEntry {
    pub(super) wer: f32,
    pub(super) missed_segments: usize,
    pub(super) hallucinated_segments: usize,
}

pub(super) type Baseline = BTreeMap<String, BaselineEntry>;

fn baseline_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("quality")
        .join("baseline.json")
}

pub(super) fn load_baseline() -> Option<Baseline> {
    let raw = fs::read_to_string(baseline_path()).ok()?;
    serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("parse {}: {err}", baseline_path().display()))
}

pub(super) fn write_baseline(results: &[(String, FixtureMetrics)]) -> Result<(), String> {
    let baseline: Baseline = results
        .iter()
        .map(|(name, metrics)| {
            (
                name.clone(),
                BaselineEntry {
                    wer: metrics.wer,
                    missed_segments: metrics.missed_segments,
                    hallucinated_segments: metrics.hallucinated_segments,
                },
            )
        })
        .collect();
    let path = baseline_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&baseline)
        .map_err(|err| format!("serialize baseline: {err}"))?;
    fs::write(&path, json + "\n").map_err(|err| format!("write {}: {err}", path.display()))
}
