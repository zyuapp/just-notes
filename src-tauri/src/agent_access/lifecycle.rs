use std::sync::{Mutex, MutexGuard};

use super::{
    entry::EntryKind,
    filesystem::SafeDirectory,
    guide::{remove_managed_guide, validate_managed_guide},
    memory,
    paths::{guide_parent_components, AgentAccessPaths, GUIDE_DIR_NAME},
    AgentGuideState, AgentGuideStatus, AgentId,
};

enum ParentState {
    Missing,
    Conflict(String),
    Open(SafeDirectory),
}

pub(crate) struct GuideLifecycle {
    pub(super) paths: AgentAccessPaths,
    operation: Mutex<()>,
}

impl GuideLifecycle {
    pub(crate) fn new(paths: AgentAccessPaths) -> Self {
        Self {
            paths,
            operation: Mutex::new(()),
        }
    }

    pub(crate) fn statuses(&self) -> Result<Vec<AgentGuideStatus>, String> {
        let _guard = self.lock()?;
        self.statuses_unlocked()
    }

    pub(crate) fn remove_guide(
        &self,
        agent: AgentId,
        remove_shared_memory: bool,
    ) -> Result<Vec<AgentGuideStatus>, String> {
        let _guard = self.lock()?;
        let status = self.status_unlocked(agent)?;
        match status.state {
            AgentGuideState::NotInstalled => {
                if remove_shared_memory {
                    self.retry_orphaned_memory_cleanup(agent)?;
                }
                return self.statuses_unlocked();
            }
            AgentGuideState::Conflict => {
                return Err(status
                    .detail
                    .unwrap_or_else(|| "The guide folder is not owned by Just Notes".to_string()))
            }
            AgentGuideState::Installed => {}
        }
        let shared_cleanup = self.shared_cleanup_for_removal(agent, remove_shared_memory)?;
        let parent = self
            .open_parent(agent, false)?
            .ok_or_else(|| format!("{} guide folder disappeared", agent.label()))?;
        let guide = parent
            .open_child(GUIDE_DIR_NAME)?
            .ok_or_else(|| format!("{} guide folder disappeared", agent.label()))?;
        remove_managed_guide(&parent, &guide, agent, self.paths.memory_file())?;
        if let Some(cleanup) = shared_cleanup {
            cleanup.remove()?;
        }
        self.statuses_unlocked()
    }

    fn shared_cleanup_for_removal(
        &self,
        agent: AgentId,
        confirmed: bool,
    ) -> Result<Option<memory::SharedMemoryRemoval>, String> {
        let other = AgentId::ALL
            .into_iter()
            .find(|candidate| *candidate != agent)
            .expect("Agent Access supports exactly two agents");
        match self.status_unlocked(other)?.state {
            AgentGuideState::Installed => Ok(None),
            AgentGuideState::Conflict => Err(format!(
                "Resolve the {} guide conflict before removing {}",
                other.label(),
                agent.label()
            )),
            AgentGuideState::NotInstalled if confirmed => {
                memory::prepare_removal(&self.paths).map(Some)
            }
            AgentGuideState::NotInstalled => {
                Err("Removing the final guide requires shared-memory confirmation".to_string())
            }
        }
    }

    fn retry_orphaned_memory_cleanup(&self, agent: AgentId) -> Result<(), String> {
        let other = AgentId::ALL
            .into_iter()
            .find(|candidate| *candidate != agent)
            .expect("Agent Access supports exactly two agents");
        match self.status_unlocked(other)?.state {
            AgentGuideState::NotInstalled => memory::prepare_removal(&self.paths)?.remove(),
            AgentGuideState::Installed => Ok(()),
            AgentGuideState::Conflict => Err(format!(
                "Resolve the {} guide conflict before removing shared memory",
                other.label()
            )),
        }
    }

    pub(crate) fn guide_dir(&self, agent: AgentId) -> String {
        self.paths.guide_dir(agent).display().to_string()
    }

    pub(super) fn statuses_unlocked(&self) -> Result<Vec<AgentGuideStatus>, String> {
        AgentId::ALL
            .into_iter()
            .map(|agent| self.status_unlocked(agent))
            .collect()
    }

    fn status_unlocked(&self, agent: AgentId) -> Result<AgentGuideStatus, String> {
        let path = self.paths.guide_dir(agent);
        let parent = match self.parent_state(agent)? {
            ParentState::Missing => {
                return Ok(AgentGuideStatus::new(
                    agent,
                    AgentGuideState::NotInstalled,
                    &path,
                    None,
                ))
            }
            ParentState::Conflict(detail) => {
                return Ok(AgentGuideStatus::new(
                    agent,
                    AgentGuideState::Conflict,
                    &path,
                    Some(detail),
                ))
            }
            ParentState::Open(parent) => parent,
        };
        self.status_for_parent(agent, &path, &parent)
    }

    fn status_for_parent(
        &self,
        agent: AgentId,
        path: &std::path::Path,
        parent: &SafeDirectory,
    ) -> Result<AgentGuideStatus, String> {
        match parent.entry_kind(GUIDE_DIR_NAME)? {
            EntryKind::Missing => Ok(AgentGuideStatus::new(
                agent,
                AgentGuideState::NotInstalled,
                path,
                None,
            )),
            EntryKind::Directory => {
                let guide = parent
                    .open_child(GUIDE_DIR_NAME)?
                    .ok_or_else(|| format!("{} guide folder disappeared", agent.label()))?;
                match validate_managed_guide(&guide, agent, self.paths.memory_file()) {
                    Ok(()) => Ok(AgentGuideStatus::new(
                        agent,
                        AgentGuideState::Installed,
                        path,
                        None,
                    )),
                    Err(detail) => Ok(AgentGuideStatus::new(
                        agent,
                        AgentGuideState::Conflict,
                        path,
                        Some(detail),
                    )),
                }
            }
            _ => Ok(AgentGuideStatus::new(
                agent,
                AgentGuideState::Conflict,
                path,
                Some("The destination exists and is not a regular directory".to_string()),
            )),
        }
    }

    fn parent_state(&self, agent: AgentId) -> Result<ParentState, String> {
        let mut current = SafeDirectory::open_absolute(self.paths.home_dir())?;
        for component in guide_parent_components(agent) {
            match current.entry_kind(component)? {
                EntryKind::Missing => return Ok(ParentState::Missing),
                EntryKind::Directory => {
                    current = current
                        .open_child(component)?
                        .ok_or_else(|| format!("Agent guide parent {component} disappeared"))?;
                }
                _ => {
                    return Ok(ParentState::Conflict(format!(
                        "The guide parent {component} is not a regular directory"
                    )))
                }
            }
        }
        Ok(ParentState::Open(current))
    }

    pub(super) fn open_parent(
        &self,
        agent: AgentId,
        create: bool,
    ) -> Result<Option<SafeDirectory>, String> {
        if !create {
            return match self.parent_state(agent)? {
                ParentState::Missing => Ok(None),
                ParentState::Conflict(detail) => Err(detail),
                ParentState::Open(parent) => Ok(Some(parent)),
            };
        }
        let mut current = SafeDirectory::open_absolute(self.paths.home_dir())?;
        for component in guide_parent_components(agent) {
            current = current.create_child(component)?;
        }
        Ok(Some(current))
    }

    pub(super) fn lock(&self) -> Result<MutexGuard<'_, ()>, String> {
        self.operation
            .lock()
            .map_err(|_| "Agent Access operations are unavailable".to_string())
    }
}
