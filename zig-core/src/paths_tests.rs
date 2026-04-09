use std::path::Path;

use super::*;

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
