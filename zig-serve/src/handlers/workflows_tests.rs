use super::validate_workflow_name;

#[test]
fn accepts_simple_name() {
    assert!(validate_workflow_name("my-workflow").is_ok());
    assert!(validate_workflow_name("foo_bar.v2").is_ok());
}

#[test]
fn rejects_empty() {
    assert!(validate_workflow_name("").is_err());
}

#[test]
fn rejects_slash() {
    assert!(validate_workflow_name("foo/bar").is_err());
    assert!(validate_workflow_name("/etc/passwd").is_err());
}

#[test]
fn rejects_backslash() {
    assert!(validate_workflow_name("foo\\bar").is_err());
}

#[test]
fn rejects_dotdot() {
    assert!(validate_workflow_name("..").is_err());
    assert!(validate_workflow_name(".").is_err());
}

#[test]
fn rejects_null_and_control() {
    assert!(validate_workflow_name("foo\0bar").is_err());
    assert!(validate_workflow_name("foo\nbar").is_err());
}

#[test]
fn rejects_leading_dash() {
    assert!(validate_workflow_name("-rm").is_err());
}
