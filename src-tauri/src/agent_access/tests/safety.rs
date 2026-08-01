use std::{env, fs, path::PathBuf, time::UNIX_EPOCH};

use super::super::{guide, memory, AgentAccessPaths, AgentGuideState, AgentId, GuideLifecycle};

struct Roots {
    root: PathBuf,
    home: PathBuf,
    data: PathBuf,
}

impl Roots {
    fn new(name: &str) -> Self {
        let stamp = UNIX_EPOCH.elapsed().unwrap().as_nanos();
        let root = env::temp_dir().join(format!("just-notes-agent-safety-{name}-{stamp}"));
        let home = root.join("home");
        let data = root.join("data");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&data).unwrap();
        Self { root, home, data }
    }

    fn paths(&self) -> AgentAccessPaths {
        AgentAccessPaths::from_roots(&self.data, &self.home).unwrap()
    }

    fn lifecycle(&self) -> GuideLifecycle {
        GuideLifecycle::new(self.paths())
    }

    fn guide(&self, agent: AgentId) -> PathBuf {
        self.paths().guide_dir(agent)
    }
}

impl Drop for Roots {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn conflicting_peer_blocks_removal_and_shared_memory_cleanup() {
    let roots = Roots::new("peer-conflict");
    let lifecycle = roots.lifecycle();
    lifecycle
        .install_guides(&[AgentId::Codex, AgentId::ClaudeCode])
        .unwrap();
    fs::write(roots.guide(AgentId::ClaudeCode).join("unexpected"), "keep").unwrap();

    assert!(lifecycle.remove_guide(AgentId::Codex, true).is_err());
    assert!(roots.guide(AgentId::Codex).exists());
    assert!(roots.data.join("agent-access/MEMORY.md").exists());
}

#[test]
fn a_preexisting_staging_directory_is_never_cleaned_as_owned() {
    let roots = Roots::new("staging-conflict");
    let parent = roots.home.join(".agents/skills");
    fs::create_dir_all(&parent).unwrap();
    for sequence in 0..128 {
        let staging = parent.join(format!(
            ".just-notes-install-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&staging).unwrap();
        fs::write(staging.join("SKILL.md"), "foreign").unwrap();
    }

    let statuses = roots.lifecycle().install_guides(&[AgentId::Codex]).unwrap();

    let codex = statuses
        .iter()
        .find(|status| status.agent == AgentId::Codex)
        .unwrap();
    assert_eq!(codex.state, AgentGuideState::NotInstalled);
    assert!(codex
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("failed")));
    assert_eq!(
        fs::read_to_string(parent.join(format!(
            ".just-notes-install-{}-0/SKILL.md",
            std::process::id()
        )))
        .unwrap(),
        "foreign"
    );
}

#[cfg(unix)]
#[test]
fn batch_install_reports_a_second_agent_write_failure() {
    use std::os::unix::fs::PermissionsExt;

    let roots = Roots::new("batch-failure");
    let claude_parent = roots.home.join(".claude");
    fs::create_dir(&claude_parent).unwrap();
    fs::set_permissions(&claude_parent, fs::Permissions::from_mode(0o500)).unwrap();

    let statuses = roots
        .lifecycle()
        .install_guides(&[AgentId::Codex, AgentId::ClaudeCode])
        .unwrap();
    fs::set_permissions(&claude_parent, fs::Permissions::from_mode(0o700)).unwrap();

    let codex = statuses
        .iter()
        .find(|status| status.agent == AgentId::Codex)
        .unwrap();
    let claude = statuses
        .iter()
        .find(|status| status.agent == AgentId::ClaudeCode)
        .unwrap();
    assert_eq!(codex.state, AgentGuideState::Installed);
    assert_eq!(claude.state, AgentGuideState::NotInstalled);
    assert!(claude
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("failed")));
}

#[test]
fn prepared_memory_cleanup_never_deletes_from_a_replacement_directory() {
    let roots = Roots::new("memory-swap");
    let paths = roots.paths();
    memory::ensure(&paths).unwrap();
    let cleanup = memory::prepare_removal(&paths).unwrap();
    fs::rename(
        roots.data.join("agent-access"),
        roots.data.join("original-agent-access"),
    )
    .unwrap();
    fs::create_dir(roots.data.join("agent-access")).unwrap();
    let replacement = roots.data.join("agent-access/MEMORY.md");
    fs::write(&replacement, "replacement").unwrap();

    assert!(cleanup.remove().is_err());
    assert_eq!(fs::read_to_string(replacement).unwrap(), "replacement");
}

#[cfg(unix)]
#[test]
fn confirmed_retry_finishes_memory_cleanup_after_the_guide_was_removed() {
    use std::os::unix::fs::PermissionsExt;

    let roots = Roots::new("memory-cleanup-retry");
    let lifecycle = roots.lifecycle();
    lifecycle.install_guides(&[AgentId::Codex]).unwrap();
    let shared = roots.data.join("agent-access");
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o500)).unwrap();

    assert!(lifecycle.remove_guide(AgentId::Codex, true).is_err());
    assert!(!roots.guide(AgentId::Codex).exists());
    assert!(shared.join("MEMORY.md").exists());
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o700)).unwrap();

    lifecycle.remove_guide(AgentId::Codex, true).unwrap();
    assert!(!shared.exists());
}

#[cfg(unix)]
#[test]
fn parent_permission_failure_keeps_the_owned_guide_retryable() {
    use std::os::unix::fs::PermissionsExt;

    let roots = Roots::new("guide-removal-retry");
    let lifecycle = roots.lifecycle();
    lifecycle.install_guides(&[AgentId::Codex]).unwrap();
    let skills = roots.home.join(".agents/skills");
    fs::set_permissions(&skills, fs::Permissions::from_mode(0o500)).unwrap();

    assert!(lifecycle.remove_guide(AgentId::Codex, true).is_err());
    assert!(roots
        .guide(AgentId::Codex)
        .join(".just-notes-install.json")
        .is_file());
    fs::set_permissions(&skills, fs::Permissions::from_mode(0o700)).unwrap();

    lifecycle.remove_guide(AgentId::Codex, true).unwrap();
    assert!(!roots.guide(AgentId::Codex).exists());
}

#[test]
fn manifest_restore_failure_still_restores_the_visible_guide_path() {
    let roots = Roots::new("manifest-restore-failure");
    let lifecycle = roots.lifecycle();
    lifecycle.install_guides(&[AgentId::Codex]).unwrap();
    guide::fail_manifest_restore_during_next_removal();

    assert!(lifecycle.remove_guide(AgentId::Codex, true).is_err());
    assert!(roots.guide(AgentId::Codex).is_dir());
    assert!(roots.data.join("agent-access/MEMORY.md").exists());
    assert_eq!(
        lifecycle.statuses().unwrap()[0].state,
        AgentGuideState::Conflict
    );
    let skills = roots.home.join(".agents/skills");
    assert!(!fs::read_dir(skills).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".just-notes-remove-")
    }));
}
