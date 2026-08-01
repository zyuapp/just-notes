use std::path::{Path, PathBuf};

use super::AgentId;

pub(super) const GUIDE_DIR_NAME: &str = "just-notes";

#[derive(Clone, Debug)]
pub(crate) struct AgentAccessPaths {
    data_dir: PathBuf,
    home_dir: PathBuf,
    memory_file: PathBuf,
}

impl AgentAccessPaths {
    pub(crate) fn from_roots(data_dir: &Path, home_dir: &Path) -> Result<Self, String> {
        if !data_dir.is_absolute() || !home_dir.is_absolute() {
            return Err("Agent Access requires absolute app-data and home paths".to_string());
        }
        let shared_dir = data_dir.join("agent-access");
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            home_dir: home_dir.to_path_buf(),
            memory_file: shared_dir.join("MEMORY.md"),
        })
    }

    pub(crate) fn guide_dir(&self, agent: AgentId) -> PathBuf {
        let mut path = self.home_dir.clone();
        for component in guide_parent_components(agent) {
            path.push(component);
        }
        path.join(GUIDE_DIR_NAME)
    }

    pub(super) fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    pub(super) fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub(super) fn memory_file(&self) -> &Path {
        &self.memory_file
    }
}

pub(super) fn guide_parent_components(agent: AgentId) -> [&'static str; 2] {
    match agent {
        AgentId::Codex => [".agents", "skills"],
        AgentId::ClaudeCode => [".claude", "skills"],
    }
}
