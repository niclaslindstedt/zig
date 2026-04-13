use super::*;

#[test]
fn scope_from_flags_defaults_to_both() {
    assert_eq!(ResourceScope::from_flags(false, false), ResourceScope::Both);
}

#[test]
fn scope_from_flags_global_only() {
    assert_eq!(
        ResourceScope::from_flags(true, false),
        ResourceScope::Global
    );
}

#[test]
fn scope_from_flags_cwd_only() {
    assert_eq!(ResourceScope::from_flags(false, true), ResourceScope::Cwd);
}

#[test]
fn target_from_flags_workflow_takes_precedence() {
    let t = ResourceTarget::from_flags(Some("cv-writer"), false, false).unwrap();
    match t {
        ResourceTarget::GlobalWorkflow(name) => assert_eq!(name, "cv-writer"),
        _ => panic!("expected GlobalWorkflow"),
    }
}

#[test]
fn target_from_flags_workflow_and_cwd_conflict() {
    let err = ResourceTarget::from_flags(Some("x"), false, true).unwrap_err();
    assert!(err.to_string().contains("--workflow"));
}

#[test]
fn target_from_flags_global_returns_shared() {
    let t = ResourceTarget::from_flags(None, true, false).unwrap();
    assert!(matches!(t, ResourceTarget::GlobalShared));
}

#[test]
fn target_from_flags_cwd_returns_cwd() {
    let t = ResourceTarget::from_flags(None, false, true).unwrap();
    assert!(matches!(t, ResourceTarget::Cwd));
}

#[test]
fn target_from_flags_default_is_cwd() {
    let t = ResourceTarget::from_flags(None, false, false).unwrap();
    assert!(matches!(t, ResourceTarget::Cwd));
}

#[test]
fn add_to_dir_copies_file_and_keeps_basename() {
    let src_dir = tempfile::TempDir::new().unwrap();
    let src = src_dir.path().join("cv.md");
    std::fs::write(&src, "CV content").unwrap();

    let dest_dir = tempfile::TempDir::new().unwrap();
    let dest = add_to_dir(&src, dest_dir.path(), None).unwrap();

    assert_eq!(dest.file_name().unwrap(), "cv.md");
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), "CV content");
}

#[test]
fn add_to_dir_renames_with_explicit_name() {
    let src_dir = tempfile::TempDir::new().unwrap();
    let src = src_dir.path().join("original.md");
    std::fs::write(&src, "x").unwrap();

    let dest_dir = tempfile::TempDir::new().unwrap();
    let dest = add_to_dir(&src, dest_dir.path(), Some("renamed.md")).unwrap();

    assert!(dest.ends_with("renamed.md"));
    assert!(dest.exists());
}

#[test]
fn add_to_dir_creates_destination_when_missing() {
    let src_dir = tempfile::TempDir::new().unwrap();
    let src = src_dir.path().join("a.md");
    std::fs::write(&src, "x").unwrap();

    let parent = tempfile::TempDir::new().unwrap();
    let dest_dir = parent.path().join("nested").join("subdir");
    let dest = add_to_dir(&src, &dest_dir, None).unwrap();

    assert!(dest.exists());
    assert!(dest_dir.is_dir());
}

#[test]
fn add_to_dir_refuses_to_overwrite() {
    let src_dir = tempfile::TempDir::new().unwrap();
    let src = src_dir.path().join("cv.md");
    std::fs::write(&src, "new").unwrap();

    let dest_dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dest_dir.path().join("cv.md"), "old").unwrap();

    let err = add_to_dir(&src, dest_dir.path(), None).unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn add_to_dir_rejects_missing_source() {
    let dest_dir = tempfile::TempDir::new().unwrap();
    let err = add_to_dir(
        std::path::Path::new("/definitely/not/a/real/file.md"),
        dest_dir.path(),
        None,
    )
    .unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[test]
fn remove_from_dir_deletes_file() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("cv.md"), "x").unwrap();

    let path = remove_from_dir("cv.md", dir.path()).unwrap();
    assert!(!path.exists());
}

#[test]
fn remove_from_dir_errors_when_missing() {
    let dir = tempfile::TempDir::new().unwrap();
    let err = remove_from_dir("ghost.md", dir.path()).unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[test]
fn remove_from_dir_errors_when_directory_missing() {
    let tmp = tempfile::TempDir::new().unwrap();
    let missing = tmp.path().join("does-not-exist");
    let err = remove_from_dir("anything", &missing).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn collect_listing_walks_subdirectories() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("a.md"), "a").unwrap();
    std::fs::write(dir.path().join("sub").join("b.md"), "b").unwrap();

    let mut out = Vec::new();
    collect_listing(dir.path(), "test", &mut out);
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|e| e.tier == "test"));
    let mut names: Vec<&str> = out.iter().map(|e| e.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["a.md", "b.md"]);
}

#[test]
fn collect_listing_no_op_on_missing_directory() {
    let tmp = tempfile::TempDir::new().unwrap();
    let missing = tmp.path().join("nope");
    let mut out = Vec::new();
    collect_listing(&missing, "test", &mut out);
    assert!(out.is_empty());
}

#[test]
fn target_label_formats_known_variants() {
    assert_eq!(ResourceTarget::GlobalShared.label(), "global:_shared");
    assert_eq!(
        ResourceTarget::GlobalWorkflow("cv".into()).label(),
        "global:cv"
    );
    assert_eq!(ResourceTarget::Cwd.label(), "cwd");
}
