use std::{
    env, fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use super::{
    guide::{GUIDE_CONTENT, MANIFEST_NAME},
    AgentAccessPaths, AgentGuideState, AgentId, GuideLifecycle,
};

mod safety;

struct TestRoots {
    root: PathBuf,
    home: PathBuf,
    data: PathBuf,
}

impl TestRoots {
    fn new(name: &str) -> Self {
        let stamp = UNIX_EPOCH.elapsed().unwrap().as_nanos();
        let root = env::temp_dir().join(format!("just-notes-agent-access-{name}-{stamp}"));
        let home = root.join("home");
        let data = root.join("data");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&data).unwrap();
        Self { root, home, data }
    }

    fn lifecycle(&self) -> GuideLifecycle {
        let paths = AgentAccessPaths::from_roots(&self.data, &self.home).unwrap();
        GuideLifecycle::new(paths)
    }

    fn guide_dir(&self, agent: AgentId) -> PathBuf {
        match agent {
            AgentId::Codex => self.home.join(".agents/skills/just-notes"),
            AgentId::ClaudeCode => self.home.join(".claude/skills/just-notes"),
        }
    }

    fn memory_file(&self) -> PathBuf {
        self.data.join("agent-access/MEMORY.md")
    }
}

impl Drop for TestRoots {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn paths_resolve_to_the_two_supported_agent_destinations() {
    let roots = TestRoots::new("paths");
    let lifecycle = roots.lifecycle();

    assert_eq!(
        lifecycle.guide_dir(AgentId::Codex),
        roots.guide_dir(AgentId::Codex).display().to_string()
    );
    assert_eq!(
        lifecycle.guide_dir(AgentId::ClaudeCode),
        roots.guide_dir(AgentId::ClaudeCode).display().to_string()
    );
}

#[test]
fn status_reads_do_not_create_agent_directories() {
    let roots = TestRoots::new("read-only-status");
    let statuses = roots.lifecycle().statuses().unwrap();

    assert!(statuses
        .iter()
        .all(|status| status.state == AgentGuideState::NotInstalled));
    assert!(!roots.home.join(".agents").exists());
    assert!(!roots.home.join(".claude").exists());
}

#[test]
fn install_creates_an_owned_guide_and_empty_shared_memory() {
    let roots = TestRoots::new("install");
    let lifecycle = roots.lifecycle();

    let statuses = lifecycle.install_guides(&[AgentId::Codex]).unwrap();

    assert_eq!(state(&statuses, AgentId::Codex), AgentGuideState::Installed);
    assert_eq!(
        fs::read_to_string(roots.guide_dir(AgentId::Codex).join("SKILL.md")).unwrap(),
        GUIDE_CONTENT
    );
    assert_eq!(fs::read(roots.memory_file()).unwrap(), b"");
    assert_eq!(
        fs::read_link(roots.guide_dir(AgentId::Codex).join("MEMORY.md")).unwrap(),
        roots.memory_file()
    );
    assert!(roots
        .guide_dir(AgentId::Codex)
        .join(MANIFEST_NAME)
        .is_file());
}

#[test]
fn both_agents_share_memory_without_rewriting_it() {
    let roots = TestRoots::new("shared-memory");
    let lifecycle = roots.lifecycle();
    lifecycle.install_guides(&[AgentId::Codex]).unwrap();
    fs::write(roots.memory_file(), "Use client names\n").unwrap();

    let statuses = lifecycle
        .install_guides(&[AgentId::Codex, AgentId::ClaudeCode])
        .unwrap();

    assert_eq!(state(&statuses, AgentId::Codex), AgentGuideState::Installed);
    assert_eq!(
        state(&statuses, AgentId::ClaudeCode),
        AgentGuideState::Installed
    );
    assert_eq!(
        fs::read_to_string(roots.memory_file()).unwrap(),
        "Use client names\n"
    );
}

#[test]
fn reinstall_replaces_the_owned_guide_but_preserves_memory_bytes() {
    let roots = TestRoots::new("guide-update");
    let lifecycle = roots.lifecycle();
    lifecycle.install_guides(&[AgentId::Codex]).unwrap();
    fs::write(
        roots.guide_dir(AgentId::Codex).join("SKILL.md"),
        "stale guide",
    )
    .unwrap();
    fs::write(roots.memory_file(), b"user memory\0bytes").unwrap();

    lifecycle.install_guides(&[AgentId::Codex]).unwrap();

    assert_eq!(
        fs::read_to_string(roots.guide_dir(AgentId::Codex).join("SKILL.md")).unwrap(),
        GUIDE_CONTENT
    );
    assert_eq!(
        fs::read(roots.memory_file()).unwrap(),
        b"user memory\0bytes"
    );
}

#[test]
fn batch_install_reports_a_foreign_conflict_without_modifying_it() {
    let roots = TestRoots::new("conflict");
    let foreign = roots.guide_dir(AgentId::Codex);
    fs::create_dir_all(&foreign).unwrap();
    fs::write(foreign.join("keep.txt"), "foreign").unwrap();

    let statuses = roots
        .lifecycle()
        .install_guides(&[AgentId::Codex, AgentId::ClaudeCode])
        .unwrap();

    assert_eq!(state(&statuses, AgentId::Codex), AgentGuideState::Conflict);
    assert_eq!(
        state(&statuses, AgentId::ClaudeCode),
        AgentGuideState::Installed
    );
    assert_eq!(
        fs::read_to_string(foreign.join("keep.txt")).unwrap(),
        "foreign"
    );
}

#[test]
fn removing_one_guide_preserves_memory_and_final_removal_requires_confirmation() {
    let roots = TestRoots::new("removal");
    let lifecycle = roots.lifecycle();
    lifecycle
        .install_guides(&[AgentId::Codex, AgentId::ClaudeCode])
        .unwrap();
    fs::write(roots.memory_file(), "preserve me").unwrap();

    lifecycle.remove_guide(AgentId::Codex, false).unwrap();
    assert_eq!(
        fs::read_to_string(roots.memory_file()).unwrap(),
        "preserve me"
    );
    assert!(lifecycle.remove_guide(AgentId::ClaudeCode, false).is_err());
    assert!(roots.guide_dir(AgentId::ClaudeCode).exists());

    lifecycle.remove_guide(AgentId::ClaudeCode, true).unwrap();
    assert!(!roots.memory_file().exists());
    assert!(!roots.guide_dir(AgentId::ClaudeCode).exists());
}

#[test]
fn unexpected_owned_folder_content_blocks_removal() {
    let roots = TestRoots::new("unexpected-removal");
    let lifecycle = roots.lifecycle();
    lifecycle.install_guides(&[AgentId::Codex]).unwrap();
    let unexpected = roots.guide_dir(AgentId::Codex).join("user.txt");
    fs::write(&unexpected, "keep").unwrap();

    assert!(lifecycle.remove_guide(AgentId::Codex, true).is_err());
    assert_eq!(fs::read_to_string(unexpected).unwrap(), "keep");
}

#[test]
fn unexpected_shared_content_blocks_final_cleanup_before_the_guide_is_removed() {
    let roots = TestRoots::new("unexpected-shared-removal");
    let lifecycle = roots.lifecycle();
    lifecycle.install_guides(&[AgentId::Codex]).unwrap();
    fs::write(roots.data.join("agent-access/unknown.txt"), "keep").unwrap();

    assert!(lifecycle.remove_guide(AgentId::Codex, true).is_err());
    assert!(roots.guide_dir(AgentId::Codex).exists());
    assert!(roots.memory_file().exists());
}

#[cfg(unix)]
#[test]
fn linked_parent_is_a_conflict_and_is_never_followed() {
    use std::os::unix::fs::symlink;

    let roots = TestRoots::new("linked-parent");
    let external = roots.root.join("external");
    fs::create_dir_all(&external).unwrap();
    symlink(&external, roots.home.join(".agents")).unwrap();

    let statuses = roots.lifecycle().install_guides(&[AgentId::Codex]).unwrap();

    assert_eq!(state(&statuses, AgentId::Codex), AgentGuideState::Conflict);
    assert!(directory_is_empty(&external));
}

fn state(statuses: &[super::AgentGuideStatus], agent: AgentId) -> AgentGuideState {
    statuses
        .iter()
        .find(|status| status.agent == agent)
        .unwrap()
        .state
}

fn directory_is_empty(path: &Path) -> bool {
    fs::read_dir(path).unwrap().next().is_none()
}
