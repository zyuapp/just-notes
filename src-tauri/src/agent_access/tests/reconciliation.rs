use std::fs;

use super::super::{
    filesystem_ext::fail_next_sync,
    guide::{GUIDE_CONTENT, MANIFEST_NAME, MEMORY_NAME},
    AgentId,
};
use super::TestRoots;

#[test]
fn startup_reconciliation_updates_only_stale_owned_guides() {
    let roots = TestRoots::new("reconcile-stale");
    let lifecycle = roots.lifecycle();
    lifecycle
        .install_guides(&[AgentId::Codex, AgentId::ClaudeCode])
        .unwrap();
    fs::write(
        roots.guide_dir(AgentId::Codex).join("SKILL.md"),
        "stale guide",
    )
    .unwrap();
    fs::write(roots.memory_file(), b"user memory\0bytes").unwrap();
    let manifest = fs::read(roots.guide_dir(AgentId::Codex).join(MANIFEST_NAME)).unwrap();
    let memory_target = fs::read_link(roots.guide_dir(AgentId::Codex).join(MEMORY_NAME)).unwrap();

    let outcome = lifecycle.reconcile_installed_guides().unwrap();

    assert_eq!(outcome.changed_agents, vec![AgentId::Codex]);
    assert!(outcome.failures.is_empty());
    assert_eq!(
        fs::read_to_string(roots.guide_dir(AgentId::Codex).join("SKILL.md")).unwrap(),
        GUIDE_CONTENT
    );
    assert_eq!(
        fs::read(roots.memory_file()).unwrap(),
        b"user memory\0bytes"
    );
    assert_eq!(
        fs::read(roots.guide_dir(AgentId::Codex).join(MANIFEST_NAME)).unwrap(),
        manifest
    );
    assert_eq!(
        fs::read_link(roots.guide_dir(AgentId::Codex).join(MEMORY_NAME)).unwrap(),
        memory_target
    );
}

#[test]
fn startup_reconciliation_ignores_foreign_and_missing_destinations() {
    let roots = TestRoots::new("reconcile-foreign");
    let foreign = roots.guide_dir(AgentId::Codex);
    fs::create_dir_all(&foreign).unwrap();
    fs::write(foreign.join("SKILL.md"), "foreign guide").unwrap();

    let outcome = roots.lifecycle().reconcile_installed_guides().unwrap();

    assert!(outcome.changed_agents.is_empty());
    assert!(outcome.failures.is_empty());
    assert_eq!(
        fs::read_to_string(foreign.join("SKILL.md")).unwrap(),
        "foreign guide"
    );
    assert!(!roots.home.join(".claude").exists());
}

#[cfg(unix)]
#[test]
fn failed_reconciliation_preserves_the_previous_guide_for_retry() {
    use std::os::unix::fs::PermissionsExt;

    let roots = TestRoots::new("reconcile-retry");
    let lifecycle = roots.lifecycle();
    lifecycle.install_guides(&[AgentId::Codex]).unwrap();
    let guide_dir = roots.guide_dir(AgentId::Codex);
    fs::write(guide_dir.join("SKILL.md"), "stale guide").unwrap();
    fs::set_permissions(&guide_dir, fs::Permissions::from_mode(0o500)).unwrap();

    let failed = lifecycle.reconcile_installed_guides().unwrap();
    assert!(failed.changed_agents.is_empty());
    assert_eq!(failed.failures.len(), 1);
    assert_eq!(
        fs::read_to_string(guide_dir.join("SKILL.md")).unwrap(),
        "stale guide"
    );

    fs::set_permissions(&guide_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let retried = lifecycle.reconcile_installed_guides().unwrap();
    assert_eq!(retried.changed_agents, vec![AgentId::Codex]);
    assert!(retried.failures.is_empty());
}

#[cfg(unix)]
#[test]
fn current_guide_is_not_rewritten_during_reconciliation() {
    use std::os::unix::fs::PermissionsExt;

    let roots = TestRoots::new("reconcile-current");
    let lifecycle = roots.lifecycle();
    lifecycle.install_guides(&[AgentId::Codex]).unwrap();
    let guide_dir = roots.guide_dir(AgentId::Codex);
    fs::set_permissions(&guide_dir, fs::Permissions::from_mode(0o500)).unwrap();

    let outcome = lifecycle.reconcile_installed_guides().unwrap();

    fs::set_permissions(&guide_dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(outcome.changed_agents.is_empty());
    assert!(outcome.failures.is_empty());
}

#[test]
fn post_replace_sync_failure_still_reports_the_changed_guide() {
    let roots = TestRoots::new("reconcile-sync-failure");
    let lifecycle = roots.lifecycle();
    lifecycle.install_guides(&[AgentId::Codex]).unwrap();
    let guide = roots.guide_dir(AgentId::Codex).join("SKILL.md");
    fs::write(&guide, "stale guide").unwrap();
    fail_next_sync();

    let outcome = lifecycle.reconcile_installed_guides().unwrap();

    assert_eq!(outcome.changed_agents, vec![AgentId::Codex]);
    assert_eq!(outcome.failures.len(), 1);
    assert_eq!(fs::read_to_string(guide).unwrap(), GUIDE_CONTENT);
}
