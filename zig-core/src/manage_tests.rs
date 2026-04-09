use std::fs;

use super::*;

const WORKFLOW_A: &str = r#"
[workflow]
name = "alpha"
description = "First workflow"
tags = ["test"]

[vars.target]
type = "string"
default = "."
description = "Target path"

[[step]]
name = "greet"
prompt = "Say hello"

[[step]]
name = "farewell"
prompt = "Say goodbye"
depends_on = ["greet"]
"#;

const WORKFLOW_B: &str = r#"
[workflow]
name = "beta"
description = "Second workflow"

[[step]]
name = "work"
prompt = "Do work"
provider = "claude"
model = "sonnet"
condition = "ready"
"#;

#[test]
fn discover_zug_files_in_base_dir() {
    let dir = tempfile::tempdir().unwrap();

    fs::write(dir.path().join("one.zug"), WORKFLOW_A).unwrap();
    fs::write(dir.path().join("two.zug"), WORKFLOW_B).unwrap();
    fs::write(dir.path().join("not-a-workflow.txt"), "hello").unwrap();

    let files = discover_zug_files(dir.path());

    assert_eq!(files.len(), 2);
    assert!(
        files
            .iter()
            .any(|f| f.to_string_lossy().contains("one.zug"))
    );
    assert!(
        files
            .iter()
            .any(|f| f.to_string_lossy().contains("two.zug"))
    );
    assert!(
        !files
            .iter()
            .any(|f| f.to_string_lossy().contains("not-a-workflow"))
    );
}

#[test]
fn discover_zug_files_in_workflows_subdir() {
    let dir = tempfile::tempdir().unwrap();

    let workflows_dir = dir.path().join("workflows");
    fs::create_dir(&workflows_dir).unwrap();
    fs::write(workflows_dir.join("nested.zug"), WORKFLOW_A).unwrap();

    let files = discover_zug_files(dir.path());

    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().contains("nested.zug"));
}

#[test]
fn discover_empty_directory() {
    let dir = tempfile::tempdir().unwrap();

    let files = discover_zug_files(dir.path());

    assert!(files.is_empty());
}

#[test]
fn discover_both_locations() {
    let dir = tempfile::tempdir().unwrap();

    fs::write(dir.path().join("root.zug"), WORKFLOW_A).unwrap();
    let workflows_dir = dir.path().join("workflows");
    fs::create_dir(&workflows_dir).unwrap();
    fs::write(workflows_dir.join("nested.zug"), WORKFLOW_B).unwrap();

    let files = discover_zug_files(dir.path());

    assert_eq!(files.len(), 2);
}

#[test]
fn show_workflow_parses_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let wf_path = dir.path().join("alpha.zug");
    fs::write(&wf_path, WORKFLOW_A).unwrap();

    let path = wf_path.to_str().unwrap();
    let resolved = resolve_workflow_path(path).unwrap();
    let wf = parser::parse_file(&resolved).unwrap();
    assert_eq!(wf.workflow.name, "alpha");
    assert_eq!(wf.workflow.description, "First workflow");
    assert_eq!(wf.workflow.tags, vec!["test"]);
    assert_eq!(wf.vars.len(), 1);
    assert_eq!(wf.steps.len(), 2);
}

#[test]
fn delete_workflow_removes_file() {
    let dir = tempfile::tempdir().unwrap();
    let wf_path = dir.path().join("to-delete.zug");
    fs::write(&wf_path, WORKFLOW_A).unwrap();
    assert!(wf_path.exists());

    let path = wf_path.to_str().unwrap();
    delete_workflow(path).unwrap();
    assert!(!wf_path.exists());
}

#[test]
fn delete_workflow_not_found() {
    let result = delete_workflow("/nonexistent/path/missing.zug");
    assert!(result.is_err());
}

#[test]
fn show_workflow_not_found() {
    let result = show_workflow("/nonexistent/path/missing.zug");
    assert!(result.is_err());
}

#[test]
fn discover_zug_files_in_global_style_dir() {
    let dir = tempfile::tempdir().unwrap();
    let global_wf_dir = crate::paths::global_workflows_dir_from(dir.path());
    fs::create_dir_all(&global_wf_dir).unwrap();

    fs::write(global_wf_dir.join("global.zug"), WORKFLOW_A).unwrap();

    let files = discover_zug_files(&global_wf_dir);
    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().contains("global.zug"));
}
