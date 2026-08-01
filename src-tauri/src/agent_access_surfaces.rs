use tauri::{Manager, Wry};

use crate::{
    agent_access::{AgentId, GuideLifecycle, GuideReconciliationOutcome},
    platform,
};

const UPDATE_NOTIFICATION_ID: &str = "agent-access-guides-updated";

pub(crate) fn reconcile_on_launch(app: &tauri::App<Wry>) {
    let outcome = app.state::<GuideLifecycle>().reconcile_installed_guides();
    match outcome {
        Ok(outcome) => publish(outcome),
        Err(error) => eprintln!("Agent Access startup reconciliation failed: {error}"),
    }
}

fn publish(outcome: GuideReconciliationOutcome) {
    for failure in outcome.failures {
        eprintln!("{failure}");
    }
    let Some(body) = notification_body(&outcome.changed_agents) else {
        return;
    };
    platform::notifications::show_informational(
        UPDATE_NOTIFICATION_ID,
        "Agent Access guides updated",
        &body,
    );
}

fn notification_body(changed: &[AgentId]) -> Option<String> {
    match changed {
        [] => None,
        [agent] => Some(format!("Updated the {} guide.", agent.label())),
        _ => Some("Updated the Codex and Claude Code guides.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciliation_notification_is_once_per_changed_batch() {
        assert_eq!(notification_body(&[]), None);
        assert_eq!(
            notification_body(&[AgentId::Codex]).as_deref(),
            Some("Updated the Codex guide.")
        );
        assert_eq!(
            notification_body(&[AgentId::Codex, AgentId::ClaudeCode]).as_deref(),
            Some("Updated the Codex and Claude Code guides.")
        );
    }
}
