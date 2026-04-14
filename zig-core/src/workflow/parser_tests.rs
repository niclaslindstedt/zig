use super::*;

use crate::workflow::model::{FailurePolicy, StepCommand, VarType};

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

#[test]
fn parse_variable_constraints() {
    let wf = parse(
        r#"
[workflow]
name = "constraints-test"

[vars.content]
type = "string"
from = "prompt"
required = true
min_length = 10
max_length = 5000
pattern = "^[A-Z]"
description = "Main content"

[vars.priority]
type = "string"
default = "medium"
allowed_values = ["low", "medium", "high"]

[vars.score]
type = "number"
min = 0.0
max = 100.0

[[step]]
name = "process"
prompt = "Process ${content} with priority ${priority}"
"#,
    )
    .unwrap();

    let content = &wf.vars["content"];
    assert_eq!(content.from.as_deref(), Some("prompt"));
    assert!(content.required);
    assert_eq!(content.min_length, Some(10));
    assert_eq!(content.max_length, Some(5000));
    assert_eq!(content.pattern.as_deref(), Some("^[A-Z]"));

    let priority = &wf.vars["priority"];
    assert!(priority.allowed_values.is_some());
    let allowed = priority.allowed_values.as_ref().unwrap();
    assert_eq!(allowed.len(), 3);
    assert_eq!(allowed[0], toml::Value::String("low".into()));

    let score = &wf.vars["score"];
    assert_eq!(score.min, Some(0.0));
    assert_eq!(score.max, Some(100.0));
}

#[test]
fn roundtrip_variable_constraints() {
    let wf = parse(
        r#"
[workflow]
name = "roundtrip-constraints"

[vars.input]
type = "string"
from = "prompt"
required = true
min_length = 5

[[step]]
name = "go"
prompt = "Do ${input}"
"#,
    )
    .unwrap();

    let toml_str = to_toml(&wf).unwrap();
    let wf2 = parse(&toml_str).unwrap();

    let input = &wf2.vars["input"];
    assert_eq!(input.from.as_deref(), Some("prompt"));
    assert!(input.required);
    assert_eq!(input.min_length, Some(5));
}

#[test]
fn parse_context_injection_fields() {
    let wf = parse(
        r#"
[workflow]
name = "ctx-test"

[[step]]
name = "worker"
prompt = "Do work"
context = ["session-abc", "session-def"]
plan = "plan.md"
mcp_config = "mcp.json"
"#,
    )
    .unwrap();

    let step = &wf.steps[0];
    assert_eq!(step.context, vec!["session-abc", "session-def"]);
    assert_eq!(step.plan.as_deref(), Some("plan.md"));
    assert_eq!(step.mcp_config.as_deref(), Some("mcp.json"));
}

#[test]
fn parse_output_format() {
    let wf = parse(
        r#"
[workflow]
name = "output-test"

[[step]]
name = "worker"
prompt = "Do work"
output = "stream-json"
"#,
    )
    .unwrap();

    assert_eq!(wf.steps[0].output.as_deref(), Some("stream-json"));
}

#[test]
fn parse_command_review() {
    let wf = parse(
        r#"
[workflow]
name = "review-test"

[[step]]
name = "review-code"
prompt = "Focus on security"
command = "review"
uncommitted = true
base = "main"
title = "Security Review"
"#,
    )
    .unwrap();

    let step = &wf.steps[0];
    assert_eq!(step.command, Some(StepCommand::Review));
    assert!(step.uncommitted);
    assert_eq!(step.base.as_deref(), Some("main"));
    assert_eq!(step.title.as_deref(), Some("Security Review"));
}

#[test]
fn parse_command_plan() {
    let wf = parse(
        r#"
[workflow]
name = "plan-test"

[[step]]
name = "make-plan"
prompt = "Design the auth system"
command = "plan"
plan_output = "auth-plan.md"
instructions = "Focus on security"
"#,
    )
    .unwrap();

    let step = &wf.steps[0];
    assert_eq!(step.command, Some(StepCommand::Plan));
    assert_eq!(step.plan_output.as_deref(), Some("auth-plan.md"));
    assert_eq!(step.instructions.as_deref(), Some("Focus on security"));
}

#[test]
fn parse_command_pipe() {
    let wf = parse(
        r#"
[workflow]
name = "pipe-test"

[[step]]
name = "analyze"
prompt = "Analyze the code"

[[step]]
name = "synthesize"
prompt = "Combine the results"
command = "pipe"
depends_on = ["analyze"]
"#,
    )
    .unwrap();

    assert_eq!(wf.steps[1].command, Some(StepCommand::Pipe));
}

#[test]
fn parse_command_collect() {
    let wf = parse(
        r#"
[workflow]
name = "collect-test"

[[step]]
name = "worker-a"
prompt = "Do A"

[[step]]
name = "worker-b"
prompt = "Do B"

[[step]]
name = "gather"
prompt = ""
command = "collect"
depends_on = ["worker-a", "worker-b"]
"#,
    )
    .unwrap();

    assert_eq!(wf.steps[2].command, Some(StepCommand::Collect));
}

#[test]
fn parse_command_summary() {
    let wf = parse(
        r#"
[workflow]
name = "summary-test"

[[step]]
name = "worker"
prompt = "Do work"

[[step]]
name = "stats"
prompt = ""
command = "summary"
depends_on = ["worker"]
"#,
    )
    .unwrap();

    assert_eq!(wf.steps[1].command, Some(StepCommand::Summary));
}

// ── roles parsing ───────────────────────────────────────────────────────────

#[test]
fn parse_roles_section() {
    let wf = parse(
        r#"
[workflow]
name = "roles-test"

[roles.doctor]
system_prompt = "You are a doctor."

[roles.nurse]
system_prompt_file = "prompts/nurse.md"

[[step]]
name = "triage"
prompt = "Triage the patient"
role = "nurse"
"#,
    )
    .unwrap();

    assert_eq!(wf.roles.len(), 2);
    assert_eq!(
        wf.roles["doctor"].system_prompt.as_deref(),
        Some("You are a doctor.")
    );
    assert_eq!(
        wf.roles["nurse"].system_prompt_file.as_deref(),
        Some("prompts/nurse.md")
    );
    assert_eq!(wf.steps[0].role.as_deref(), Some("nurse"));
}

#[test]
fn parse_step_with_role_and_no_system_prompt() {
    let wf = parse(
        r#"
[workflow]
name = "role-step"

[roles.analyst]
system_prompt = "You are an analyst."

[[step]]
name = "analyze"
prompt = "Analyze this"
role = "analyst"
"#,
    )
    .unwrap();

    assert_eq!(wf.steps[0].role.as_deref(), Some("analyst"));
    assert!(wf.steps[0].system_prompt.is_none());
}

#[test]
fn parse_variable_default_file() {
    let wf = parse(
        r#"
[workflow]
name = "default-file"

[vars.instructions]
type = "string"
default_file = "defaults/instructions.txt"

[[step]]
name = "go"
prompt = "Follow ${instructions}"
"#,
    )
    .unwrap();

    assert_eq!(
        wf.vars["instructions"].default_file.as_deref(),
        Some("defaults/instructions.txt")
    );
}

// ── zip archive parsing ─────────────────────────────────────────────────────

#[test]
fn parse_workflow_from_zip_archive() {
    use std::io::Write;

    let tmp = tempfile::TempDir::new().unwrap();
    let zip_path = tmp.path().join("test.zwfz");

    // Create a zip archive containing a workflow TOML
    let file = std::fs::File::create(&zip_path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);

    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    zip_writer.start_file("workflow.toml", options).unwrap();
    zip_writer
        .write_all(
            br#"[workflow]
name = "from-zip"

[[step]]
name = "hello"
prompt = "Say hello"
"#,
        )
        .unwrap();
    zip_writer.finish().unwrap();

    let (wf, source) = parse_workflow(&zip_path).unwrap();
    assert_eq!(wf.workflow.name, "from-zip");
    assert_eq!(wf.steps.len(), 1);
    assert!(matches!(source, WorkflowSource::Zip { .. }));
}

#[test]
fn parse_workflow_plain_toml_returns_directory_source() {
    let tmp = tempfile::TempDir::new().unwrap();
    let toml_path = tmp.path().join("workflow.zwf");
    std::fs::write(
        &toml_path,
        r#"[workflow]
name = "plain"

[[step]]
name = "hello"
prompt = "Say hello"
"#,
    )
    .unwrap();

    let (wf, source) = parse_workflow(&toml_path).unwrap();
    assert_eq!(wf.workflow.name, "plain");
    assert!(matches!(source, WorkflowSource::Directory(_)));
}

#[test]
fn parse_zip_with_role_prompt_files() {
    use std::io::Write;

    let tmp = tempfile::TempDir::new().unwrap();
    let zip_path = tmp.path().join("healthcare.zwfz");

    let file = std::fs::File::create(&zip_path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);

    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Add workflow TOML
    zip_writer.start_file("healthcare.toml", options).unwrap();
    zip_writer
        .write_all(
            br#"[workflow]
name = "healthcare"

[roles.doctor]
system_prompt_file = "prompts/doctor.md"

[[step]]
name = "examine"
prompt = "Examine the patient"
role = "doctor"
"#,
        )
        .unwrap();

    // Add prompt file
    zip_writer.start_file("prompts/doctor.md", options).unwrap();
    zip_writer
        .write_all(b"You are a doctor. Examine the patient carefully.")
        .unwrap();

    zip_writer.finish().unwrap();

    let (wf, source) = parse_workflow(&zip_path).unwrap();
    assert_eq!(wf.workflow.name, "healthcare");

    // Verify the prompt file exists in the extracted directory
    let prompt_path = source.dir().join("prompts/doctor.md");
    assert!(prompt_path.exists());
    assert_eq!(
        std::fs::read_to_string(&prompt_path).unwrap(),
        "You are a doctor. Examine the patient carefully."
    );
}

#[test]
fn parse_zip_with_no_toml_fails() {
    use std::io::Write;

    let tmp = tempfile::TempDir::new().unwrap();
    let zip_path = tmp.path().join("empty.zwfz");

    let file = std::fs::File::create(&zip_path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);

    let options = zip::write::SimpleFileOptions::default();
    zip_writer.start_file("readme.md", options).unwrap();
    zip_writer.write_all(b"No workflow here").unwrap();
    zip_writer.finish().unwrap();

    let result = parse_workflow(&zip_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no .toml"));
}

// ── Workflow-level version, provider, model ────────────────────────────────

#[test]
fn parse_workflow_version() {
    let wf = parse(
        r#"
[workflow]
name = "versioned"
version = "1.2.3"

[[step]]
name = "hello"
prompt = "Hi"
"#,
    )
    .unwrap();
    assert_eq!(wf.workflow.version.as_deref(), Some("1.2.3"));
}

#[test]
fn parse_workflow_provider_and_model() {
    let wf = parse(
        r#"
[workflow]
name = "defaults"
provider = "claude"
model = "sonnet"

[[step]]
name = "hello"
prompt = "Hi"
"#,
    )
    .unwrap();
    assert_eq!(wf.workflow.provider.as_deref(), Some("claude"));
    assert_eq!(wf.workflow.model.as_deref(), Some("sonnet"));
}

#[test]
fn parse_workflow_all_new_meta_fields() {
    let wf = parse(
        r#"
[workflow]
name = "full-meta"
description = "All metadata fields"
tags = ["test"]
version = "2.0.0"
provider = "gemini"
model = "large"

[[step]]
name = "hello"
prompt = "Hi"
"#,
    )
    .unwrap();
    assert_eq!(wf.workflow.name, "full-meta");
    assert_eq!(wf.workflow.version.as_deref(), Some("2.0.0"));
    assert_eq!(wf.workflow.provider.as_deref(), Some("gemini"));
    assert_eq!(wf.workflow.model.as_deref(), Some("large"));
}

#[test]
fn parse_workflow_without_new_fields_defaults_to_none() {
    let wf = parse(MINIMAL_WORKFLOW).unwrap();
    assert!(wf.workflow.version.is_none());
    assert!(wf.workflow.provider.is_none());
    assert!(wf.workflow.model.is_none());
}

#[test]
fn roundtrip_workflow_level_fields() {
    let wf = parse(
        r#"
[workflow]
name = "roundtrip"
version = "1.0.0"
provider = "claude"
model = "opus"

[[step]]
name = "hello"
prompt = "Hi"
"#,
    )
    .unwrap();
    let toml_str = to_toml(&wf).unwrap();
    let wf2 = parse(&toml_str).unwrap();
    assert_eq!(wf2.workflow.version.as_deref(), Some("1.0.0"));
    assert_eq!(wf2.workflow.provider.as_deref(), Some("claude"));
    assert_eq!(wf2.workflow.model.as_deref(), Some("opus"));
}

// ── resources ───────────────────────────────────────────────────────────────

#[test]
fn parse_workflow_resources_as_bare_paths() {
    let wf = parse(
        r#"
[workflow]
name = "with-resources"
resources = ["./cv.md", "./style.md"]

[[step]]
name = "draft"
prompt = "Draft the letter"
"#,
    )
    .unwrap();
    assert_eq!(wf.workflow.resources.len(), 2);
    assert_eq!(wf.workflow.resources[0].path(), "./cv.md");
    assert!(wf.workflow.resources[0].name().is_none());
    assert!(!wf.workflow.resources[0].required());
}

#[test]
fn parse_workflow_resources_as_detailed_table() {
    let wf = parse(
        r#"
[workflow]
name = "with-resources"

[[workflow.resources]]
path = "./cv.md"
name = "cv"
description = "Candidate CV"
required = true

[[step]]
name = "draft"
prompt = "Draft the letter"
"#,
    )
    .unwrap();
    assert_eq!(wf.workflow.resources.len(), 1);
    let r = &wf.workflow.resources[0];
    assert_eq!(r.path(), "./cv.md");
    assert_eq!(r.name(), Some("cv"));
    assert_eq!(r.description(), Some("Candidate CV"));
    assert!(r.required());
}

#[test]
fn parse_step_resources_bare_and_detailed_mix() {
    let wf = parse(
        r#"
[workflow]
name = "mixed"

[[step]]
name = "draft"
prompt = "Draft"
resources = [
    "./bare.md",
    { path = "./detailed.md", name = "d", description = "Detailed", required = false },
]
"#,
    )
    .unwrap();
    let step = &wf.steps[0];
    assert_eq!(step.resources.len(), 2);
    assert_eq!(step.resources[0].path(), "./bare.md");
    assert!(step.resources[0].name().is_none());
    assert_eq!(step.resources[1].path(), "./detailed.md");
    assert_eq!(step.resources[1].name(), Some("d"));
    assert_eq!(step.resources[1].description(), Some("Detailed"));
}

#[test]
fn parse_workflow_without_resources_defaults_to_empty() {
    let wf = parse(MINIMAL_WORKFLOW).unwrap();
    assert!(wf.workflow.resources.is_empty());
    assert!(wf.steps[0].resources.is_empty());
}

#[test]
fn roundtrip_workflow_resources() {
    let wf = parse(
        r#"
[workflow]
name = "rt"
resources = ["./cv.md"]

[[step]]
name = "hello"
prompt = "Hi"
resources = ["./job.md"]
"#,
    )
    .unwrap();
    let toml_str = to_toml(&wf).unwrap();
    let wf2 = parse(&toml_str).unwrap();
    assert_eq!(wf2.workflow.resources.len(), 1);
    assert_eq!(wf2.workflow.resources[0].path(), "./cv.md");
    assert_eq!(wf2.steps[0].resources.len(), 1);
    assert_eq!(wf2.steps[0].resources[0].path(), "./job.md");
}
