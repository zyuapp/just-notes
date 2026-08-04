use super::{
    entry::AtomicWriteOutcome, guide::reconcile_managed_guide, paths::GUIDE_DIR_NAME,
    AgentGuideState, AgentId, GuideLifecycle, GuideReconciliationOutcome,
};

impl GuideLifecycle {
    pub(crate) fn reconcile_installed_guides(&self) -> Result<GuideReconciliationOutcome, String> {
        let _guard = self.lock()?;
        let mut outcome = GuideReconciliationOutcome::default();
        for agent in AgentId::ALL {
            match self.reconcile_one(agent) {
                Ok(Some(AtomicWriteOutcome::Synced)) => outcome.changed(agent),
                Ok(Some(AtomicWriteOutcome::ReplacedButUnsynced(error))) => {
                    outcome.changed_with_warning(agent, error)
                }
                Ok(None) => {}
                Err(error) => outcome.failed(agent, error),
            }
        }
        Ok(outcome)
    }

    fn reconcile_one(&self, agent: AgentId) -> Result<Option<AtomicWriteOutcome>, String> {
        if self.status_unlocked(agent)?.state != AgentGuideState::Installed {
            return Ok(None);
        }
        let parent = self
            .open_parent(agent, false)?
            .ok_or_else(|| format!("{} guide folder disappeared", agent.label()))?;
        let guide = parent
            .open_child(GUIDE_DIR_NAME)?
            .ok_or_else(|| format!("{} guide folder disappeared", agent.label()))?;
        reconcile_managed_guide(&guide, agent, self.paths.memory_file())
    }
}
