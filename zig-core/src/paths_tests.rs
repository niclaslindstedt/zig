use std::path::Path;

use super::*;

// =====================================================================
// Local workflow directory tests
// =====================================================================

#[test]
fn cwd_workflows_dir_from_finds_directory_in_start() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".zig").join("workflows")).unwrap();

    let found = cwd_workflows_dir_from(tmp.path());
    assert!(found.is_some());
    assert!(found.unwrap().ends_with(".zig/workflows"));
}

#[test]
fn cwd_workflows_dir_from_walks_up_to_find_directory() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".zig").join("workflows")).unwrap();
    let nested = tmp.path().join("a").join("b").join("c");
    std::fs::create_dir_all(&nested).unwrap();

    let found = cwd_workflows_dir_from(&nested);
    assert!(found.is_some());
    let abs = std::fs::canonicalize(found.unwrap()).unwrap();
    let expected = std::fs::canonicalize(tmp.path().join(".zig").join("workflows")).unwrap();
    assert_eq!(abs, expected);
}

#[test]
fn cwd_workflows_dir_from_returns_none_when_absent() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let nested = tmp.path().join("a").join("b");
    std::fs::create_dir_all(&nested).unwrap();

    let found = cwd_workflows_dir_from(&nested);
    assert!(found.is_none());
}

#[test]
fn cwd_workflows_dir_from_does_not_walk_past_git_root() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tmp.path().join("outside");
    std::fs::create_dir_all(outside.join(".zig").join("workflows")).unwrap();
    let repo = outside.join("repo");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let sub = repo.join("sub");
    std::fs::create_dir_all(&sub).unwrap();

    let found = cwd_workflows_dir_from(&sub);
    assert!(
        found.is_none(),
        "walk should stop at git root, not see outside/.zig/workflows, but got {found:?}"
    );
}

#[test]
fn global_workflows_dir_from_returns_correct_path() {
    let home = Path::new("/home/testuser");
    let result = global_workflows_dir_from(home);
    assert_eq!(result, Path::new("/home/testuser/.zig/workflows"));
}

#[test]
fn ensure_global_workflows_dir_creates_directories() {
    let dir = tempfile::tempdir().unwrap();
    let workflows_dir = global_workflows_dir_from(dir.path());

    assert!(!workflows_dir.exists());

    std::fs::create_dir_all(&workflows_dir).unwrap();

    assert!(workflows_dir.exists());
    assert!(workflows_dir.is_dir());
}

#[test]
fn global_resources_dir_from_returns_correct_path() {
    let home = Path::new("/home/testuser");
    let result = global_resources_dir_from(home);
    assert_eq!(result, Path::new("/home/testuser/.zig/resources"));
}

#[test]
fn cwd_resources_dir_from_finds_directory_in_start() {
    let tmp = tempfile::tempdir().unwrap();
    // Make this look like a git root so the walk-up has a stop boundary.
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".zig").join("resources")).unwrap();

    let found = cwd_resources_dir_from(tmp.path());
    assert!(found.is_some());
    assert!(found.unwrap().ends_with(".zig/resources"));
}

#[test]
fn cwd_resources_dir_from_walks_up_to_find_directory() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".zig").join("resources")).unwrap();
    let nested = tmp.path().join("a").join("b").join("c");
    std::fs::create_dir_all(&nested).unwrap();

    let found = cwd_resources_dir_from(&nested);
    assert!(found.is_some());
    let abs = std::fs::canonicalize(found.unwrap()).unwrap();
    let expected = std::fs::canonicalize(tmp.path().join(".zig").join("resources")).unwrap();
    assert_eq!(abs, expected);
}

#[test]
fn cwd_resources_dir_from_returns_none_when_absent() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let nested = tmp.path().join("a").join("b");
    std::fs::create_dir_all(&nested).unwrap();

    let found = cwd_resources_dir_from(&nested);
    assert!(found.is_none());
}

#[test]
fn cwd_resources_dir_from_does_not_walk_past_git_root() {
    // Create:
    //   <tmp>/outside/.zig/resources       (should NOT be found)
    //   <tmp>/outside/repo/.git              (git root stops the walk)
    //   <tmp>/outside/repo/sub               (start here)
    let tmp = tempfile::tempdir().unwrap();
    let outside = tmp.path().join("outside");
    std::fs::create_dir_all(outside.join(".zig").join("resources")).unwrap();
    let repo = outside.join("repo");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let sub = repo.join("sub");
    std::fs::create_dir_all(&sub).unwrap();

    let found = cwd_resources_dir_from(&sub);
    assert!(
        found.is_none(),
        "walk should stop at git root, not see outside/.zig/resources, but got {found:?}"
    );
}
