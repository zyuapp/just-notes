use tauri::{AppHandle, State};

use crate::{
    agent_access::{AgentId, GuideLifecycle},
    ipc::AgentGuideStatusPayload,
    platform,
};

#[tauri::command]
pub(crate) fn get_agent_guide_statuses(
    lifecycle: State<'_, GuideLifecycle>,
) -> Result<Vec<AgentGuideStatusPayload>, String> {
    lifecycle
        .statuses()
        .map(|statuses| statuses.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub(crate) fn install_agent_guides(
    lifecycle: State<'_, GuideLifecycle>,
    agents: Vec<AgentId>,
) -> Result<Vec<AgentGuideStatusPayload>, String> {
    lifecycle
        .install_guides(&agents)
        .map(|statuses| statuses.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub(crate) fn remove_agent_guide(
    lifecycle: State<'_, GuideLifecycle>,
    agent: AgentId,
    remove_shared_memory: bool,
) -> Result<Vec<AgentGuideStatusPayload>, String> {
    lifecycle
        .remove_guide(agent, remove_shared_memory)
        .map(|statuses| statuses.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub(crate) fn reveal_agent_guide(
    app: AppHandle,
    lifecycle: State<'_, GuideLifecycle>,
    agent: AgentId,
) -> Result<(), String> {
    platform::reveal_in_finder(&app, &lifecycle.guide_dir(agent))
}
