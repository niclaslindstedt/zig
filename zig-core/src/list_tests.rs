use std::fs;

use super::*;

#[test]
fn list_empty_directory() {
    let dir = std::env::temp_dir().join("zig-list-empty-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let workflows = discover_workflows(&dir).unwrap();
    assert!(workflows.is_empty());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn list_discovers_root_workflows() {
    let dir = std::env::temp_dir().join("zig-list-root-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("deploy.zug"),
        "[workflow]\nname = \"deploy\"\ndescription = \"Deploy app\"\n",
    )
    .unwrap();

    let workflows = discover_workflows(&dir).unwrap();
    assert_eq!(workflows.len(), 1);
    assert_eq!(workflows[0].name, "deploy");
    assert_eq!(workflows[0].description, "Deploy app");
    assert_eq!(workflows[0].steps, 0);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn list_discovers_workflows_subdir() {
    let dir = std::env::temp_dir().join("zig-list-subdir-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("workflows")).unwrap();

    fs::write(
        dir.join("workflows/test.zug"),
        "[workflow]\nname = \"test\"\ndescription = \"Run tests\"\n",
    )
    .unwrap();

    let workflows = discover_workflows(&dir).unwrap();
    assert_eq!(workflows.len(), 1);
    assert_eq!(workflows[0].name, "test");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn list_handles_unparseable_files() {
    let dir = std::env::temp_dir().join("zig-list-bad-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("bad.zug"), "not valid toml {{{{").unwrap();

    let workflows = discover_workflows(&dir).unwrap();
    assert_eq!(workflows.len(), 1);
    assert_eq!(workflows[0].name, "bad");
    assert_eq!(workflows[0].description, "(parse error)");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn list_discovers_both_root_and_subdir() {
    let dir = std::env::temp_dir().join("zig-list-both-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("workflows")).unwrap();

    fs::write(
        dir.join("alpha.zug"),
        "[workflow]\nname = \"alpha\"\ndescription = \"First\"\n",
    )
    .unwrap();
    fs::write(
        dir.join("workflows/beta.zug"),
        "[workflow]\nname = \"beta\"\ndescription = \"Second\"\n",
    )
    .unwrap();

    let workflows = discover_workflows(&dir).unwrap();
    assert_eq!(workflows.len(), 2);
    assert_eq!(workflows[0].name, "alpha");
    assert_eq!(workflows[1].name, "beta");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn show_nonexistent_workflow_fails() {
    let result = run_show("nonexistent-workflow-xyz");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("workflow not found"));
}

#[test]
fn show_existing_workflow() {
    let dir = std::env::temp_dir().join("zig-show-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let content = r#"
[workflow]
name = "ci-pipeline"
description = "Run CI checks"
tags = ["ci", "testing"]

[[step]]
name = "lint"
prompt = "Run linter"

[[step]]
name = "test"
prompt = "Run tests"
depends_on = ["lint"]
"#;

    let file = dir.join("ci-pipeline.zug");
    fs::write(&file, content).unwrap();

    let result = run_show(file.to_str().unwrap());
    assert!(result.is_ok());

    let _ = fs::remove_dir_all(&dir);
}
