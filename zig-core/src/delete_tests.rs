use std::fs;

use super::*;

#[test]
fn delete_nonexistent_workflow_fails() {
    let result = run_delete("nonexistent-workflow-xyz");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("workflow not found"));
}

#[test]
fn delete_existing_file() {
    let dir = std::env::temp_dir().join("zig-delete-test");
    let _ = fs::create_dir_all(&dir);
    let file = dir.join("test-delete.zug");
    fs::write(
        &file,
        "[workflow]\nname = \"test\"\ndescription = \"test\"\n",
    )
    .unwrap();

    let result = run_delete(file.to_str().unwrap());
    assert!(result.is_ok());
    assert!(!file.exists());

    let _ = fs::remove_dir_all(&dir);
}
