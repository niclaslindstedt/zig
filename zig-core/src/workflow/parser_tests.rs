use super::*;

use crate::workflow::model::{FailurePolicy, VarType};

const MINIMAL_WORKFLOW: &str = r#"
[workflow]
name = "minimal"

[[step]]
name = "hello"
prompt = "Say hello"
"#;

const FULL_WORKFLOW: &str = r#"
[workflow]
name = "code-review"
description = "Multi-perspective code review with synthesis"
tags = ["review", "quality"]

[vars.target]
type = "string"
default = "."
description = "Path to review"

[vars.threshold]
type = "number"
default = 8

[vars.score]
type = "number"

[vars.feedback]
type = "string"
default = ""

[[step]]
name = "analyze"
prompt = "Analyze the code structure of ${target}"
provider = "claude"
model = "sonnet"
description = "Initial code structure analysis"

[[step]]
name = "security-review"
prompt = "Review for security vulnerabilities"
depends_on = ["analyze"]
inject_context = true
provider = "claude"
tags = ["review"]
auto_approve = true

[[step]]
name = "perf-review"
prompt = "Review for performance issues"
depends_on = ["analyze"]
inject_context = true
provider = "gemini"
tags = ["review"]
root = "./src"

[[step]]
name = "synthesize"
prompt = "Create a unified code review report"
depends_on = ["security-review", "perf-review"]
inject_context = true
worktree = true

[[step]]
name = "quality-gate"
prompt = "Score this report 1-10"
depends_on = ["synthesize"]
inject_context = true
json = true
saves = { score = "$.score", feedback = "$.suggestions" }
timeout = "5m"
files = ["docs/policy.md"]

[[step]]
name = "refine"
prompt = "Improve based on: ${feedback}"
depends_on = ["quality-gate"]
condition = "score < threshold"
next = "quality-gate"
on_failure = "retry"
max_retries = 2
retry_model = "large"
"#;

#[test]
fn parse_minimal_workflow() {
    let wf = parse(MINIMAL_WORKFLOW).unwrap();
    assert_eq!(wf.workflow.name, "minimal");
    assert_eq!(wf.steps.len(), 1);
    assert_eq!(wf.steps[0].name, "hello");
    assert_eq!(wf.steps[0].prompt, "Say hello");
}

#[test]
fn parse_full_workflow() {
    let wf = parse(FULL_WORKFLOW).unwrap();

    // Metadata
    assert_eq!(wf.workflow.name, "code-review");
    assert_eq!(
        wf.workflow.description,
        "Multi-perspective code review with synthesis"
    );
    assert_eq!(wf.workflow.tags, vec!["review", "quality"]);

    // Variables
    assert_eq!(wf.vars.len(), 4);
    let target = &wf.vars["target"];
    assert_eq!(target.var_type, VarType::String);
    assert_eq!(target.default, Some(toml::Value::String(".".into())));
    assert_eq!(target.description, "Path to review");

    let threshold = &wf.vars["threshold"];
    assert_eq!(threshold.var_type, VarType::Number);
    assert_eq!(threshold.default, Some(toml::Value::Integer(8)));

    let score = &wf.vars["score"];
    assert_eq!(score.var_type, VarType::Number);
    assert!(score.default.is_none());

    // Steps
    assert_eq!(wf.steps.len(), 6);

    let analyze = &wf.steps[0];
    assert_eq!(analyze.name, "analyze");
    assert_eq!(analyze.provider.as_deref(), Some("claude"));
    assert_eq!(analyze.model.as_deref(), Some("sonnet"));
    assert!(analyze.depends_on.is_empty());
    assert_eq!(analyze.description, "Initial code structure analysis");

    let security = &wf.steps[1];
    assert_eq!(security.depends_on, vec!["analyze"]);
    assert!(security.inject_context);
    assert_eq!(security.tags, vec!["review"]);
    assert!(security.auto_approve);

    let perf = &wf.steps[2];
    assert_eq!(perf.provider.as_deref(), Some("gemini"));
    assert_eq!(perf.root.as_deref(), Some("./src"));

    let synthesize = &wf.steps[3];
    assert_eq!(
        synthesize.depends_on,
        vec!["security-review", "perf-review"]
    );
    assert!(synthesize.worktree);

    let quality = &wf.steps[4];
    assert!(quality.json);
    assert_eq!(quality.saves.len(), 2);
    assert_eq!(quality.saves["score"], "$.score");
    assert_eq!(quality.saves["feedback"], "$.suggestions");
    assert_eq!(quality.timeout.as_deref(), Some("5m"));
    assert_eq!(quality.files, vec!["docs/policy.md"]);

    let refine = &wf.steps[5];
    assert_eq!(refine.condition.as_deref(), Some("score < threshold"));
    assert_eq!(refine.next.as_deref(), Some("quality-gate"));
    assert_eq!(refine.on_failure, Some(FailurePolicy::Retry));
    assert_eq!(refine.max_retries, Some(2));
    assert_eq!(refine.retry_model.as_deref(), Some("large"));
}

#[test]
fn parse_invalid_toml() {
    let result = parse("not valid toml [[[");
    assert!(result.is_err());
}

#[test]
fn parse_missing_workflow_section() {
    let result = parse(
        r#"
[[step]]
name = "hello"
prompt = "Hi"
"#,
    );
    assert!(result.is_err());
}

#[test]
fn roundtrip_serialization() {
    let wf = parse(MINIMAL_WORKFLOW).unwrap();
    let toml_str = to_toml(&wf).unwrap();
    let wf2 = parse(&toml_str).unwrap();
    assert_eq!(wf2.workflow.name, wf.workflow.name);
    assert_eq!(wf2.steps.len(), wf.steps.len());
    assert_eq!(wf2.steps[0].name, wf.steps[0].name);
}

#[test]
fn parse_variable_types() {
    let wf = parse(
        r#"
[workflow]
name = "types-test"

[vars.flag]
type = "bool"
default = true

[vars.data]
type = "json"
description = "Structured data"

[[step]]
name = "test"
prompt = "Test"
"#,
    )
    .unwrap();

    assert_eq!(wf.vars["flag"].var_type, VarType::Bool);
    assert_eq!(wf.vars["flag"].default, Some(toml::Value::Boolean(true)));
    assert_eq!(wf.vars["data"].var_type, VarType::Json);
}

#[test]
fn parse_new_step_fields() {
    let wf = parse(
        r#"
[workflow]
name = "new-fields"

[[step]]
name = "worker"
prompt = "Do work"
description = "A worker step"
interactive = true
auto_approve = true
root = "/tmp/work"
add_dirs = ["/tmp/shared"]
files = ["input.txt"]
worktree = true
sandbox = "worker-box"
race_group = "approach"
retry_model = "large"
on_failure = "retry"

[step.env]
MODE = "strict"
"#,
    )
    .unwrap();

    let step = &wf.steps[0];
    assert_eq!(step.description, "A worker step");
    assert!(step.interactive);
    assert!(step.auto_approve);
    assert_eq!(step.root.as_deref(), Some("/tmp/work"));
    assert_eq!(step.add_dirs, vec!["/tmp/shared"]);
    assert_eq!(step.files, vec!["input.txt"]);
    assert!(step.worktree);
    assert_eq!(step.sandbox.as_deref(), Some("worker-box"));
    assert_eq!(step.race_group.as_deref(), Some("approach"));
    assert_eq!(step.retry_model.as_deref(), Some("large"));
    assert_eq!(step.env["MODE"], "strict");
}
