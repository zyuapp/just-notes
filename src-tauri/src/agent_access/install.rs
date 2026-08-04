use std::collections::HashMap;

use super::{
    guide::{install_new_guide, update_managed_guide},
    memory,
    model::normalized_agents,
    paths::GUIDE_DIR_NAME,
    AgentGuideState, AgentGuideStatus, AgentId, GuideLifecycle,
};

impl GuideLifecycle {
    pub(crate) fn install_guides(
        &self,
        requested: &[AgentId],
    ) -> Result<Vec<AgentGuideStatus>, String> {
        let _guard = self.lock()?;
        let requested = normalized_agents(requested)?;
        let preflight = self.statuses_unlocked()?;
        let mut failures = HashMap::new();
        if preflight.iter().any(|status| {
            requested.contains(&status.agent) && status.state != AgentGuideState::Conflict
        }) {
            if let Err(error) = memory::ensure(&self.paths) {
                for agent in requested {
                    if preflight.iter().any(|status| {
                        status.agent == agent && status.state != AgentGuideState::Conflict
                    }) {
                        failures.insert(agent, error.clone());
                    }
                }
                return Ok(with_failures(preflight, failures));
            }
        }
        for agent in requested {
            let status = preflight
                .iter()
                .find(|status| status.agent == agent)
                .expect("every known agent has a status");
            if status.state == AgentGuideState::Conflict {
                continue;
            }
            if let Err(error) = self.install_one(agent, status.state) {
                failures.insert(agent, error);
            }
        }
        self.statuses_unlocked()
            .map(|statuses| with_failures(statuses, failures))
    }

    fn install_one(&self, agent: AgentId, state: AgentGuideState) -> Result<(), String> {
        let parent = self
            .open_parent(agent, state == AgentGuideState::NotInstalled)?
            .ok_or_else(|| format!("{} guide folder disappeared", agent.label()))?;
        match state {
            AgentGuideState::Installed => {
                let guide = parent
                    .open_child(GUIDE_DIR_NAME)?
                    .ok_or_else(|| format!("{} guide folder disappeared", agent.label()))?;
                update_managed_guide(&guide, agent, self.paths.memory_file())
            }
            AgentGuideState::NotInstalled => {
                install_new_guide(&parent, agent, self.paths.memory_file())
            }
            AgentGuideState::Conflict => Ok(()),
        }
    }
}

fn with_failures(
    mut statuses: Vec<AgentGuideStatus>,
    failures: HashMap<AgentId, String>,
) -> Vec<AgentGuideStatus> {
    for status in &mut statuses {
        if let Some(error) = failures.get(&status.agent) {
            status.detail = Some(format!("Installation failed: {error}"));
        }
    }
    statuses
}
