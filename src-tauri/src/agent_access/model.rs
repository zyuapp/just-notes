use std::{collections::HashSet, path::Path};

pub(super) const INSTALL_OWNER: &str = "dev.just-notes";
pub(super) const INSTALL_SCHEMA_VERSION: u32 = 1;

#[derive(
    serde::Serialize, serde::Deserialize, ts_rs::TS, Clone, Copy, Debug, PartialEq, Eq, Hash,
)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) enum AgentId {
    Codex,
    ClaudeCode,
}

impl AgentId {
    pub(super) const ALL: [Self; 2] = [Self::Codex, Self::ClaudeCode];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::ClaudeCode => "Claude Code",
        }
    }
}

#[derive(serde::Serialize, ts_rs::TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) enum AgentGuideState {
    NotInstalled,
    Installed,
    Conflict,
}

#[derive(Clone, Debug)]
pub(crate) struct AgentGuideStatus {
    pub(crate) agent: AgentId,
    pub(crate) state: AgentGuideState,
    pub(crate) path: String,
    pub(crate) detail: Option<String>,
}

impl AgentGuideStatus {
    pub(super) fn new(
        agent: AgentId,
        state: AgentGuideState,
        path: &Path,
        detail: Option<String>,
    ) -> Self {
        Self {
            agent,
            state,
            path: path.display().to_string(),
            detail,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct InstallManifest {
    pub(super) schema_version: u32,
    pub(super) owner: String,
    pub(super) agent: AgentId,
}

impl InstallManifest {
    pub(super) fn expected(agent: AgentId) -> Self {
        Self {
            schema_version: INSTALL_SCHEMA_VERSION,
            owner: INSTALL_OWNER.to_string(),
            agent,
        }
    }
}

pub(super) fn normalized_agents(requested: &[AgentId]) -> Result<Vec<AgentId>, String> {
    let mut seen = HashSet::new();
    let agents = requested
        .iter()
        .copied()
        .filter(|agent| seen.insert(*agent))
        .collect::<Vec<_>>();
    if agents.is_empty() {
        return Err("Choose at least one agent guide to install".to_string());
    }
    Ok(agents)
}
