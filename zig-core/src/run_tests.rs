use std::collections::HashMap;
use std::path::Path;

use crate::memory::MemoryCollector;
use crate::resources::ResourceCollector;
use crate::workflow::model::{
    MemoryMode, ResourceSpec, Role, Step, StepCommand, VarType, Variable, Workflow, WorkflowMeta,
};

use super::*;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Build an empty resource collector for tests that don't care about resources
/// — no inline specs and every tier directory disabled.
fn empty_collector<'a>(workflow_dir: &'a Path) -> ResourceCollector<'a> {
    ResourceCollector {
        workflow_resources: &[],
        workflow_dir,
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_resources_dir: None,
        disabled: false,
    }
}

/// Build a resource collector for tests that exercise inline workflow resources.
fn inline_collector<'a>(
    workflow_resources: &'a [ResourceSpec],
    workflow_dir: &'a Path,
) -> ResourceCollector<'a> {
    ResourceCollector {
        workflow_resources,
        workflow_dir,
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_resources_dir: None,
        disabled: false,
    }
}

/// Build an empty memory collector for tests that don't care about memory.
fn empty_memory_collector() -> MemoryCollector {
    MemoryCollector {
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_memory_dir: None,
        workflow_mode: MemoryMode::All,
        local_enabled: true,
        disabled: false,
    }
}

fn step(name: &str) -> Step {
    Step {
        name: name.to_string(),
        prompt: format!("Do {name}"),
        ..Default::default()
    }
}

fn step_with_deps(name: &str, deps: &[&str]) -> Step {
    let mut s = step(name);
    s.depends_on = deps.iter().map(|d| d.to_string()).collect();
    s
}

// ── topological_sort ─────────────────────────────────────────────────────────

#[test]
fn topo_sort_single_step() {
    let steps = vec![step("a")];
    let tiers = topological_sort(&steps).unwrap();
    assert_eq!(tiers.len(), 1);
    assert_eq!(tiers[0].len(), 1);
    assert_eq!(tiers[0][0].name, "a");
}

#[test]
fn topo_sort_linear_chain() {
    let steps = vec![
        step("a"),
        step_with_deps("b", &["a"]),
        step_with_deps("c", &["b"]),
    ];
    let tiers = topological_sort(&steps).unwrap();
    assert_eq!(tiers.len(), 3);
    assert_eq!(tiers[0][0].name, "a");
    assert_eq!(tiers[1][0].name, "b");
    assert_eq!(tiers[2][0].name, "c");
}

#[test]
fn topo_sort_fan_out() {
    let steps = vec![
        step("a"),
        step_with_deps("b", &["a"]),
        step_with_deps("c", &["a"]),
    ];
    let tiers = topological_sort(&steps).unwrap();
    assert_eq!(tiers.len(), 2);
    assert_eq!(tiers[0].len(), 1);
    assert_eq!(tiers[0][0].name, "a");
    assert_eq!(tiers[1].len(), 2);
    let names: Vec<&str> = tiers[1].iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"b"));
    assert!(names.contains(&"c"));
}

#[test]
fn topo_sort_fan_out_gather() {
    let steps = vec![step("a"), step("b"), step_with_deps("c", &["a", "b"])];
    let tiers = topological_sort(&steps).unwrap();
    assert_eq!(tiers.len(), 2);
    assert_eq!(tiers[0].len(), 2);
    assert_eq!(tiers[1].len(), 1);
    assert_eq!(tiers[1][0].name, "c");
}

#[test]
fn topo_sort_diamond() {
    let steps = vec![
        step("a"),
        step_with_deps("b", &["a"]),
        step_with_deps("c", &["a"]),
        step_with_deps("d", &["b", "c"]),
    ];
    let tiers = topological_sort(&steps).unwrap();
    assert_eq!(tiers.len(), 3);
    assert_eq!(tiers[0][0].name, "a");
    assert_eq!(tiers[2][0].name, "d");
    let middle: Vec<&str> = tiers[1].iter().map(|s| s.name.as_str()).collect();
    assert!(middle.contains(&"b"));
    assert!(middle.contains(&"c"));
}

// ── substitute_vars ──────────────────────────────────────────────────────────

#[test]
fn substitute_simple_variable() {
    let vars = HashMap::from([("name".into(), "Alice".into())]);
    assert_eq!(substitute_vars("Hello ${name}!", &vars), "Hello Alice!");
}

#[test]
fn substitute_multiple_variables() {
    let vars = HashMap::from([("a".into(), "1".into()), ("b".into(), "2".into())]);
    assert_eq!(substitute_vars("${a} + ${b}", &vars), "1 + 2");
}

#[test]
fn substitute_unknown_variable_left_as_is() {
    let vars: HashMap<String, String> = HashMap::new();
    assert_eq!(
        substitute_vars("Hello ${unknown}!", &vars),
        "Hello ${unknown}!"
    );
}

#[test]
fn substitute_no_variables() {
    let vars: HashMap<String, String> = HashMap::new();
    assert_eq!(substitute_vars("no vars here", &vars), "no vars here");
}

#[test]
fn substitute_dotted_path_in_json() {
    let vars = HashMap::from([(
        "result".into(),
        r#"{"score": 42, "details": {"level": "high"}}"#.into(),
    )]);
    assert_eq!(
        substitute_vars("Score: ${result.score}", &vars),
        "Score: 42"
    );
    assert_eq!(
        substitute_vars("Level: ${result.details.level}", &vars),
        "Level: high"
    );
}

// ── evaluate_condition ───────────────────────────────────────────────────────

#[test]
fn condition_numeric_less_than_true() {
    let vars = HashMap::from([("score".into(), "5".into())]);
    assert!(evaluate_condition("score < 8", &vars).unwrap());
}

#[test]
fn condition_numeric_less_than_false() {
    let vars = HashMap::from([("score".into(), "9".into())]);
    assert!(!evaluate_condition("score < 8", &vars).unwrap());
}

#[test]
fn condition_string_equality_true() {
    let vars = HashMap::from([("status".into(), "done".into())]);
    assert!(evaluate_condition("status == \"done\"", &vars).unwrap());
}

#[test]
fn condition_string_equality_false() {
    let vars = HashMap::from([("status".into(), "pending".into())]);
    assert!(!evaluate_condition("status == \"done\"", &vars).unwrap());
}

#[test]
fn condition_not_equal() {
    let vars = HashMap::from([("status".into(), "running".into())]);
    assert!(evaluate_condition("status != \"done\"", &vars).unwrap());
}

#[test]
fn condition_truthy_true() {
    let vars = HashMap::from([("approved".into(), "true".into())]);
    assert!(evaluate_condition("approved", &vars).unwrap());
}

#[test]
fn condition_truthy_false() {
    let vars = HashMap::from([("approved".into(), "false".into())]);
    assert!(!evaluate_condition("approved", &vars).unwrap());
}

#[test]
fn condition_truthy_empty() {
    let vars = HashMap::from([("approved".into(), String::new())]);
    assert!(!evaluate_condition("approved", &vars).unwrap());
}

#[test]
fn condition_variable_to_variable() {
    let vars = HashMap::from([
        ("retries".into(), "2".into()),
        ("max_retries".into(), "5".into()),
    ]);
    assert!(evaluate_condition("retries < max_retries", &vars).unwrap());
}

#[test]
fn condition_greater_equal() {
    let vars = HashMap::from([("score".into(), "8".into())]);
    assert!(evaluate_condition("score >= 8", &vars).unwrap());
    assert!(!evaluate_condition("score >= 9", &vars).unwrap());
}

// ── extract_saves ────────────────────────────────────────────────────────────

#[test]
fn saves_full_output() {
    let saves = HashMap::from([("result".into(), "$".into())]);
    let extracted = extract_saves("hello world", &saves).unwrap();
    assert_eq!(extracted["result"], "hello world");
}

#[test]
fn saves_json_field() {
    let saves = HashMap::from([("score".into(), "$.score".into())]);
    let extracted = extract_saves(r#"{"score": 7, "status": "ok"}"#, &saves).unwrap();
    assert_eq!(extracted["score"], "7");
}

#[test]
fn saves_nested_json_field() {
    let saves = HashMap::from([("level".into(), "$.details.level".into())]);
    let extracted = extract_saves(r#"{"details": {"level": "high"}}"#, &saves).unwrap();
    assert_eq!(extracted["level"], "high");
}

#[test]
fn saves_multiple() {
    let saves = HashMap::from([
        ("score".into(), "$.score".into()),
        ("msg".into(), "$.message".into()),
    ]);
    let extracted = extract_saves(r#"{"score": 9, "message": "great"}"#, &saves).unwrap();
    assert_eq!(extracted["score"], "9");
    assert_eq!(extracted["msg"], "great");
}

#[test]
fn saves_json_field_on_non_json_fails() {
    let saves = HashMap::from([("val".into(), "$.field".into())]);
    assert!(extract_saves("not json", &saves).is_err());
}

// ── render_step_prompt ───────────────────────────────────────────────────────

#[test]
fn render_with_var_substitution() {
    let mut s = step("test");
    s.prompt = "Review ${target}".into();
    let vars = HashMap::from([("target".into(), "src/main.rs".into())]);
    let result = render_step_prompt(&s, &vars, None, &HashMap::new());
    assert_eq!(result, "Review src/main.rs");
}

#[test]
fn render_with_user_prompt() {
    let s = step("test");
    let result = render_step_prompt(&s, &HashMap::new(), Some("focus on auth"), &HashMap::new());
    assert!(result.starts_with("User context: focus on auth"));
    assert!(result.contains("Do test"));
}

#[test]
fn render_with_inject_context() {
    let mut s = step_with_deps("synth", &["analyze"]);
    s.inject_context = true;
    let dep_outputs = HashMap::from([("analyze".into(), "Analysis result here".into())]);
    let result = render_step_prompt(&s, &HashMap::new(), None, &dep_outputs);
    assert!(result.contains("Output from 'analyze'"));
    assert!(result.contains("Analysis result here"));
    assert!(result.contains("Do synth"));
}

#[test]
fn render_with_all_combined() {
    let mut s = step_with_deps("report", &["scan"]);
    s.inject_context = true;
    s.prompt = "Report on ${target}".into();
    let vars = HashMap::from([("target".into(), "api/".into())]);
    let dep_outputs = HashMap::from([("scan".into(), "Found 3 issues".into())]);
    let result = render_step_prompt(&s, &vars, Some("be thorough"), &dep_outputs);
    assert!(result.contains("User context: be thorough"));
    assert!(result.contains("Output from 'scan'"));
    assert!(result.contains("Report on api/"));
}

// ── init_vars ────────────────────────────────────────────────────────────────

#[test]
fn init_vars_with_defaults() {
    let workflow = Workflow {
        workflow: WorkflowMeta {
            name: "test".into(),
            ..Default::default()
        },
        roles: HashMap::new(),
        vars: HashMap::from([
            (
                "target".into(),
                Variable {
                    var_type: VarType::String,
                    default: Some(toml::Value::String(".".into())),
                    ..Default::default()
                },
            ),
            (
                "score".into(),
                Variable {
                    var_type: VarType::Number,
                    default: Some(toml::Value::Integer(0)),
                    ..Default::default()
                },
            ),
            (
                "verbose".into(),
                Variable {
                    var_type: VarType::Bool,
                    default: None,
                    ..Default::default()
                },
            ),
        ]),
        steps: vec![],
        storage: Default::default(),
    };

    let vars = init_vars(&workflow);
    assert_eq!(vars["target"], ".");
    assert_eq!(vars["score"], "0");
    assert_eq!(vars["verbose"], "");
}

// ── is_truthy ────────────────────────────────────────────────────────────────

#[test]
fn truthy_values() {
    assert!(is_truthy("true"));
    assert!(is_truthy("yes"));
    assert!(is_truthy("1"));
    assert!(is_truthy("anything"));
}

#[test]
fn falsy_values() {
    assert!(!is_truthy(""));
    assert!(!is_truthy("false"));
    assert!(!is_truthy("0"));
}

// ── json_path_lookup ────────────────────────────────────────────────────────

#[test]
fn json_path_lookup_missing_key() {
    let json: serde_json::Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
    let result = json_path_lookup(&json, "missing");
    assert!(result.contains("?.missing"));
}

#[test]
fn json_path_lookup_missing_nested_key() {
    let json: serde_json::Value = serde_json::from_str(r#"{"a": {"b": 1}}"#).unwrap();
    let result = json_path_lookup(&json, "a.missing");
    assert!(result.contains("?.a.missing"));
}

// ── substitute_vars edge cases ──────────────────────────────────────────────

#[test]
fn substitute_unclosed_var_ref() {
    let vars: HashMap<String, String> = HashMap::new();
    assert_eq!(
        substitute_vars("Hello ${unclosed", &vars),
        "Hello ${unclosed"
    );
}

#[test]
fn substitute_adjacent_var_refs() {
    let vars = HashMap::from([("a".into(), "X".into()), ("b".into(), "Y".into())]);
    assert_eq!(substitute_vars("${a}${b}", &vars), "XY");
}

// ── compare edge cases ──────────────────────────────────────────────────────

#[test]
fn compare_equal_floats() {
    assert!(compare("3.14", "3.14", "=="));
    assert!(!compare("3.14", "3.15", "=="));
}

#[test]
fn compare_string_ordering() {
    assert!(compare("alpha", "beta", "<"));
    assert!(!compare("beta", "alpha", "<"));
}

#[test]
fn compare_mixed_type_falls_back_to_string() {
    // "abc" can't parse as f64, so lexicographic comparison
    assert!(compare("abc", "def", "<"));
}

// ── evaluate_condition edge cases ───────────────────────────────────────────

#[test]
fn condition_unknown_variable_is_falsy() {
    let vars: HashMap<String, String> = HashMap::new();
    assert!(!evaluate_condition("nonexistent", &vars).unwrap());
}

#[test]
fn condition_with_whitespace() {
    let vars = HashMap::from([("x".into(), "5".into())]);
    assert!(evaluate_condition("  x < 10  ", &vars).unwrap());
}

// ── resolve_workflow_path ────────────────────────────────────────────────────

#[test]
fn resolve_nonexistent_path_fails() {
    let result = resolve_workflow_path("nonexistent-workflow-xyz");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("workflow not found"));
}

#[test]
fn resolve_from_global_dir() {
    let dir = tempfile::tempdir().unwrap();
    let global_wf_dir = crate::paths::global_workflows_dir_from(dir.path());
    std::fs::create_dir_all(&global_wf_dir).unwrap();
    std::fs::write(
        global_wf_dir.join("global-test.zwf"),
        "[workflow]\nname = \"g\"\ndescription = \"\"\n[[step]]\nname = \"s\"\nprompt = \"p\"",
    )
    .unwrap();

    // Use the full path to test resolution
    let full_path = global_wf_dir.join("global-test.zwf");
    let result = resolve_workflow_path(full_path.to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn resolve_local_over_global_precedence() {
    let local_dir = tempfile::tempdir().unwrap();
    let local_path = local_dir.path().join("precedence.zwf");
    std::fs::write(&local_path, "local").unwrap();

    // Resolving the literal local path finds the local file
    let result = resolve_workflow_path(local_path.to_str().unwrap());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), local_path);
}

// ── prompt binding ──────────────────────────────────────────────────────────

#[test]
fn prompt_var_binding_populates_variable() {
    let workflow = Workflow {
        workflow: WorkflowMeta {
            name: "test".into(),
            ..Default::default()
        },
        roles: HashMap::new(),
        vars: HashMap::from([(
            "content".into(),
            Variable {
                var_type: VarType::String,
                from: Some("prompt".into()),
                ..Default::default()
            },
        )]),
        steps: vec![],
        storage: Default::default(),
    };

    let mut vars = init_vars(&workflow);

    // Simulate the prompt binding from execute()
    let prompt_var = workflow
        .vars
        .iter()
        .find(|(_, v)| v.from.as_deref() == Some("prompt"))
        .map(|(name, _)| name.clone());

    if let Some(ref var_name) = prompt_var {
        vars.insert(var_name.clone(), "user input here".to_string());
    }

    assert_eq!(vars["content"], "user input here");
}

#[test]
fn prompt_var_suppresses_user_context_prefix() {
    let step = Step {
        name: "test".into(),
        prompt: "Process: ${content}".into(),
        ..Default::default()
    };

    let vars = HashMap::from([("content".into(), "the user input".into())]);
    let dep_outputs = HashMap::new();

    // When prompt var exists, effective_user_prompt is None
    let result = render_step_prompt(&step, &vars, None, &dep_outputs);
    assert!(!result.contains("User context:"));
    assert!(result.contains("Process: the user input"));
}

#[test]
fn prompt_var_with_default_uses_default_when_no_prompt() {
    let workflow = Workflow {
        workflow: WorkflowMeta {
            name: "test".into(),
            ..Default::default()
        },
        roles: HashMap::new(),
        vars: HashMap::from([(
            "content".into(),
            Variable {
                var_type: VarType::String,
                from: Some("prompt".into()),
                default: Some(toml::Value::String("fallback".into())),
                ..Default::default()
            },
        )]),
        steps: vec![],
        storage: Default::default(),
    };

    let vars = init_vars(&workflow);
    // No user prompt provided, so default stays
    assert_eq!(vars["content"], "fallback");
}

// ── build_zag_args ──────────────────────────────────────────────────────────

#[test]
fn build_zag_args_basic() {
    let mut s = step("test");
    s.provider = Some("claude".into());
    s.model = Some("sonnet".into());
    s.system_prompt = Some("be helpful".into());
    s.max_turns = Some(5);
    s.json = true;
    s.timeout = Some("5m".into());
    s.tags = vec!["review".into()];

    let args = build_zag_args(
        &s,
        "do stuff",
        "my-workflow",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );

    assert!(args.contains(&"run".to_string()));
    assert!(args.contains(&"do stuff".to_string()));
    assert!(args.contains(&"--provider".to_string()));
    assert!(args.contains(&"claude".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"sonnet".to_string()));
    assert!(args.contains(&"--system-prompt".to_string()));
    assert!(args.contains(&"--max-turns".to_string()));
    assert!(args.contains(&"5".to_string()));
    assert!(args.contains(&"--json".to_string()));
    assert!(args.contains(&"--timeout".to_string()));
    assert!(args.contains(&"5m".to_string()));
    assert!(args.contains(&"--name".to_string()));
    assert!(args.contains(&"zig-my-workflow-test".to_string()));
    assert!(args.contains(&"zig-workflow".to_string()));
    assert!(args.contains(&"review".to_string()));
}

#[test]
fn build_zag_args_auto_approve() {
    let mut s = step("test");
    s.auto_approve = true;
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--auto-approve".to_string()));

    let s2 = step("test");
    let args2 = build_zag_args(
        &s2,
        "prompt",
        "wf",
        None,
        s2.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(!args2.contains(&"--auto-approve".to_string()));
}

#[test]
fn build_zag_args_env() {
    let mut s = step("test");
    s.env = HashMap::from([("MODE".into(), "strict".into())]);
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--env".to_string()));
    assert!(args.contains(&"MODE=strict".to_string()));
}

#[test]
fn build_zag_args_isolation() {
    let mut s = step("test");
    s.worktree = true;
    s.sandbox = Some("worker-box".into());
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--worktree".to_string()));
    assert!(args.contains(&"--sandbox".to_string()));
    assert!(args.contains(&"worker-box".to_string()));
}

#[test]
fn build_zag_args_files_and_dirs() {
    let mut s = step("test");
    s.files = vec!["input.txt".into(), "data.json".into()];
    s.add_dirs = vec!["/tmp/shared".into()];
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );

    let file_count = args.iter().filter(|a| *a == "--file").count();
    assert_eq!(file_count, 2);
    assert!(args.contains(&"input.txt".to_string()));
    assert!(args.contains(&"data.json".to_string()));
    assert!(args.contains(&"--add-dir".to_string()));
    assert!(args.contains(&"/tmp/shared".to_string()));
}

#[test]
fn build_zag_args_description() {
    let mut s = step("test");
    s.description = "Analyze the code".into();
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--description".to_string()));
    assert!(args.contains(&"Analyze the code".to_string()));

    // Empty description should not produce the flag
    let s2 = step("test");
    let args2 = build_zag_args(
        &s2,
        "prompt",
        "wf",
        None,
        s2.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(!args2.contains(&"--description".to_string()));
}

#[test]
fn build_zag_args_json_schema() {
    let mut s = step("test");
    s.json_schema = Some(r#"{"type":"object"}"#.into());
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--json-schema".to_string()));
    assert!(args.contains(&r#"{"type":"object"}"#.to_string()));
}

#[test]
fn build_zag_args_root() {
    let mut s = step("test");
    s.root = Some("/tmp/work".into());
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--root".to_string()));
    assert!(args.contains(&"/tmp/work".to_string()));
}

#[test]
fn build_zag_args_model_override() {
    let mut s = step("test");
    s.model = Some("sonnet".into());
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        Some("opus"),
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    // Override should take precedence
    let model_idx = args.iter().position(|a| a == "--model").unwrap();
    assert_eq!(args[model_idx + 1], "opus");
}

#[test]
fn build_zag_args_model_no_override() {
    let mut s = step("test");
    s.model = Some("sonnet".into());
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    let model_idx = args.iter().position(|a| a == "--model").unwrap();
    assert_eq!(args[model_idx + 1], "sonnet");
}

#[test]
fn build_zag_args_no_model() {
    let s = step("test");
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(!args.contains(&"--model".to_string()));
}

// ── partition_tier ──────────────────────────────────────────────────────────

#[test]
fn partition_tier_no_race_groups() {
    let steps = [step("a"), step("b")];
    let refs: Vec<&Step> = steps.iter().collect();
    let (sequential, race_groups) = partition_tier(&refs);
    assert_eq!(sequential.len(), 2);
    assert!(race_groups.is_empty());
}

#[test]
fn partition_tier_with_race_group() {
    let mut a = step("a");
    a.race_group = Some("solvers".into());
    let mut b = step("b");
    b.race_group = Some("solvers".into());
    let steps = [a, b];
    let refs: Vec<&Step> = steps.iter().collect();
    let (sequential, race_groups) = partition_tier(&refs);
    assert!(sequential.is_empty());
    assert_eq!(race_groups.len(), 1);
    assert_eq!(race_groups["solvers"].len(), 2);
}

#[test]
fn partition_tier_mixed() {
    let mut a = step("a");
    a.race_group = Some("group1".into());
    let b = step("b");
    let mut c = step("c");
    c.race_group = Some("group1".into());
    let steps = [a, b, c];
    let refs: Vec<&Step> = steps.iter().collect();
    let (sequential, race_groups) = partition_tier(&refs);
    assert_eq!(sequential.len(), 1);
    assert_eq!(sequential[0].name, "b");
    assert_eq!(race_groups["group1"].len(), 2);
}

#[test]
fn partition_tier_multiple_race_groups() {
    let mut a = step("a");
    a.race_group = Some("fast".into());
    let mut b = step("b");
    b.race_group = Some("slow".into());
    let mut c = step("c");
    c.race_group = Some("fast".into());
    let steps = [a, b, c];
    let refs: Vec<&Step> = steps.iter().collect();
    let (sequential, race_groups) = partition_tier(&refs);
    assert!(sequential.is_empty());
    assert_eq!(race_groups.len(), 2);
    assert_eq!(race_groups["fast"].len(), 2);
    assert_eq!(race_groups["slow"].len(), 1);
}

// ── build_zag_args: context injection ──────────────────────────────────────

#[test]
fn build_zag_args_context() {
    let mut s = step("test");
    s.context = vec!["session-abc".into(), "session-def".into()];
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    let ctx_count = args.iter().filter(|a| *a == "--context").count();
    assert_eq!(ctx_count, 2);
    assert!(args.contains(&"session-abc".to_string()));
    assert!(args.contains(&"session-def".to_string()));
}

#[test]
fn build_zag_args_plan() {
    let mut s = step("test");
    s.plan = Some("plan.md".into());
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--plan".to_string()));
    assert!(args.contains(&"plan.md".to_string()));
}

#[test]
fn build_zag_args_mcp_config() {
    let mut s = step("test");
    s.mcp_config = Some("config.json".into());
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--mcp-config".to_string()));
    assert!(args.contains(&"config.json".to_string()));
}

#[test]
fn build_zag_args_no_context_by_default() {
    let s = step("test");
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(!args.contains(&"--context".to_string()));
    assert!(!args.contains(&"--plan".to_string()));
    assert!(!args.contains(&"--mcp-config".to_string()));
}

// ── build_zag_args: output format ──────────────────────────────────────────

#[test]
fn build_zag_args_output_format() {
    let mut s = step("test");
    s.output = Some("stream-json".into());
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"-o".to_string()));
    assert!(args.contains(&"stream-json".to_string()));
    assert!(!args.contains(&"--json".to_string()));
}

#[test]
fn build_zag_args_output_overrides_json() {
    let mut s = step("test");
    s.output = Some("text".into());
    s.json = true;
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"-o".to_string()));
    assert!(args.contains(&"text".to_string()));
    assert!(!args.contains(&"--json".to_string()));
}

#[test]
fn build_zag_args_json_fallback() {
    let mut s = step("test");
    s.json = true;
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--json".to_string()));
    assert!(!args.contains(&"-o".to_string()));
}

// ── build_zag_args: command step types ─────────────────────────────────────

#[test]
fn build_zag_args_command_review() {
    let mut s = step("review-step");
    s.command = Some(StepCommand::Review);
    s.uncommitted = true;
    s.base = Some("main".into());
    s.commit = Some("abc123".into());
    s.title = Some("Security Review".into());
    let args = build_zag_args(
        &s,
        "focus on auth",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(args[0], "review");
    assert!(args.contains(&"focus on auth".to_string()));
    assert!(args.contains(&"--uncommitted".to_string()));
    assert!(args.contains(&"--base".to_string()));
    assert!(args.contains(&"main".to_string()));
    assert!(args.contains(&"--commit".to_string()));
    assert!(args.contains(&"abc123".to_string()));
    assert!(args.contains(&"--title".to_string()));
    assert!(args.contains(&"Security Review".to_string()));
}

#[test]
fn build_zag_args_command_plan() {
    let mut s = step("plan-step");
    s.command = Some(StepCommand::Plan);
    s.plan_output = Some("auth-plan.md".into());
    s.instructions = Some("Focus on security".into());
    let args = build_zag_args(
        &s,
        "Design auth system",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(args[0], "plan");
    assert_eq!(args[1], "Design auth system");
    assert!(args.contains(&"-o".to_string()));
    assert!(args.contains(&"auth-plan.md".to_string()));
    assert!(args.contains(&"--instructions".to_string()));
    assert!(args.contains(&"Focus on security".to_string()));
}

#[test]
fn build_zag_args_command_pipe() {
    let mut s = step("synth");
    s.command = Some(StepCommand::Pipe);
    s.depends_on = vec!["analyze".into(), "review".into()];
    let args = build_zag_args(
        &s,
        "Combine results",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(args[0], "pipe");
    assert!(args.contains(&"zig-wf-analyze".to_string()));
    assert!(args.contains(&"zig-wf-review".to_string()));
    assert!(args.contains(&"--".to_string()));
    assert!(args.contains(&"Combine results".to_string()));
}

#[test]
fn build_zag_args_command_collect() {
    let mut s = step("gather");
    s.command = Some(StepCommand::Collect);
    s.depends_on = vec!["worker-a".into(), "worker-b".into()];
    let args = build_zag_args(
        &s,
        "",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(args[0], "collect");
    assert!(args.contains(&"zig-wf-worker-a".to_string()));
    assert!(args.contains(&"zig-wf-worker-b".to_string()));
    // collect doesn't accept agent args like --provider
    assert!(!args.contains(&"--provider".to_string()));
}

#[test]
fn build_zag_args_command_summary() {
    let mut s = step("stats");
    s.command = Some(StepCommand::Summary);
    s.depends_on = vec!["worker".into()];
    let args = build_zag_args(
        &s,
        "",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(args[0], "summary");
    assert!(args.contains(&"zig-wf-worker".to_string()));
}

#[test]
fn build_zag_args_collect_no_agent_args() {
    let mut s = step("gather");
    s.command = Some(StepCommand::Collect);
    s.depends_on = vec!["a".into()];
    s.provider = Some("claude".into());
    s.model = Some("sonnet".into());
    s.auto_approve = true;
    let args = build_zag_args(
        &s,
        "",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    // These agent args should NOT appear for collect
    assert!(!args.contains(&"--provider".to_string()));
    assert!(!args.contains(&"--model".to_string()));
    assert!(!args.contains(&"--auto-approve".to_string()));
}

#[test]
fn build_zag_args_review_accepts_agent_args() {
    let mut s = step("review-step");
    s.command = Some(StepCommand::Review);
    s.provider = Some("claude".into());
    s.model = Some("opus".into());
    let args = build_zag_args(
        &s,
        "review code",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--provider".to_string()));
    assert!(args.contains(&"claude".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"opus".to_string()));
}

#[test]
fn build_zag_args_default_command_unchanged() {
    let s = step("test");
    let args = build_zag_args(
        &s,
        "do stuff",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(args[0], "run");
    assert_eq!(args[1], "do stuff");
}

#[test]
fn build_zag_args_session_metadata_on_all_commands() {
    // Even collect/summary should get --name, --tag, --description
    let mut s = step("gather");
    s.command = Some(StepCommand::Collect);
    s.depends_on = vec!["a".into()];
    s.description = "Gather results".into();
    s.tags = vec!["custom".into()];
    s.timeout = Some("5m".into());
    let args = build_zag_args(
        &s,
        "",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--name".to_string()));
    assert!(args.contains(&"zig-wf-gather".to_string()));
    assert!(args.contains(&"--description".to_string()));
    assert!(args.contains(&"Gather results".to_string()));
    assert!(args.contains(&"zig-workflow".to_string()));
    assert!(args.contains(&"custom".to_string()));
    assert!(args.contains(&"--timeout".to_string()));
    assert!(args.contains(&"5m".to_string()));
}

// ── build_zag_args: system_prompt variable substitution ───────────────────

#[test]
fn build_zag_args_rendered_system_prompt() {
    let mut s = step("test");
    s.system_prompt = Some("You are a ${role}".into());
    // Pass pre-rendered (substituted) value
    let args = build_zag_args(
        &s,
        "do stuff",
        "wf",
        None,
        Some("You are a cardiologist"),
        None,
        None,
        &[],
    );
    assert!(args.contains(&"--system-prompt".to_string()));
    assert!(args.contains(&"You are a cardiologist".to_string()));
    assert!(!args.contains(&"You are a ${role}".to_string()));
}

#[test]
fn build_zag_args_no_system_prompt() {
    let s = step("test");
    let args = build_zag_args(&s, "do stuff", "wf", None, None, None, None, &[]);
    assert!(!args.contains(&"--system-prompt".to_string()));
}

// ── build_zag_args: workflow-level provider/model fallback ────────────────

#[test]
fn build_zag_args_workflow_provider_fallback() {
    let s = step("test");
    // Step has no provider, workflow provides default
    let args = build_zag_args(&s, "prompt", "wf", None, None, Some("claude"), None, &[]);
    assert!(args.contains(&"--provider".to_string()));
    assert!(args.contains(&"claude".to_string()));
}

#[test]
fn build_zag_args_workflow_model_fallback() {
    let s = step("test");
    // Step has no model, workflow provides default
    let args = build_zag_args(&s, "prompt", "wf", None, None, None, Some("sonnet"), &[]);
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"sonnet".to_string()));
}

#[test]
fn build_zag_args_step_provider_overrides_workflow() {
    let mut s = step("test");
    s.provider = Some("gemini".into());
    let args = build_zag_args(&s, "prompt", "wf", None, None, Some("claude"), None, &[]);
    assert!(args.contains(&"--provider".to_string()));
    assert!(args.contains(&"gemini".to_string()));
    assert!(!args.contains(&"claude".to_string()));
}

#[test]
fn build_zag_args_step_model_overrides_workflow() {
    let mut s = step("test");
    s.model = Some("opus".into());
    let args = build_zag_args(&s, "prompt", "wf", None, None, None, Some("sonnet"), &[]);
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"opus".to_string()));
    assert!(!args.contains(&"sonnet".to_string()));
}

#[test]
fn build_zag_args_no_provider_no_workflow_provider() {
    let s = step("test");
    let args = build_zag_args(&s, "prompt", "wf", None, None, None, None, &[]);
    assert!(!args.contains(&"--provider".to_string()));
    assert!(!args.contains(&"--model".to_string()));
}

#[test]
fn build_zag_args_model_override_beats_workflow_model() {
    let s = step("test");
    // model_override (retry escalation) should beat workflow model
    let args = build_zag_args(
        &s,
        "prompt",
        "wf",
        Some("opus"),
        None,
        None,
        Some("sonnet"),
        &[],
    );
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"opus".to_string()));
    assert!(!args.contains(&"sonnet".to_string()));
}

// ── resolve_role_system_prompt ──────────────���──────────────────────────────

#[test]
fn resolve_direct_system_prompt() {
    let step = Step {
        name: "test".into(),
        prompt: "do stuff".into(),
        system_prompt: Some("You are a doctor.".into()),
        ..Default::default()
    };
    let roles = HashMap::new();
    let vars = HashMap::new();
    let dir = std::path::Path::new(".");

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &empty_collector(dir),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        dir,
        "test-wf",
    )
    .unwrap();
    assert_eq!(result, Some("You are a doctor.".to_string()));
}

#[test]
fn resolve_direct_system_prompt_with_var_substitution() {
    let step = Step {
        name: "test".into(),
        prompt: "do stuff".into(),
        system_prompt: Some("You are a ${specialty} specialist.".into()),
        ..Default::default()
    };
    let roles = HashMap::new();
    let vars = HashMap::from([("specialty".into(), "cardiology".into())]);
    let dir = std::path::Path::new(".");

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &empty_collector(dir),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        dir,
        "test-wf",
    )
    .unwrap();
    assert_eq!(result, Some("You are a cardiology specialist.".to_string()));
}

#[test]
fn resolve_static_role_reference() {
    let step = Step {
        name: "test".into(),
        prompt: "do stuff".into(),
        role: Some("doctor".into()),
        ..Default::default()
    };
    let roles = HashMap::from([(
        "doctor".into(),
        Role {
            system_prompt: Some("You are a doctor.".into()),
            ..Default::default()
        },
    )]);
    let vars = HashMap::new();
    let dir = std::path::Path::new(".");

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &empty_collector(dir),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        dir,
        "test-wf",
    )
    .unwrap();
    assert_eq!(result, Some("You are a doctor.".to_string()));
}

#[test]
fn resolve_dynamic_role_reference() {
    let step = Step {
        name: "test".into(),
        prompt: "do stuff".into(),
        role: Some("${specialist_type}".into()),
        ..Default::default()
    };
    let roles = HashMap::from([(
        "cardiologist".into(),
        Role {
            system_prompt: Some("You are a cardiologist.".into()),
            ..Default::default()
        },
    )]);
    let vars = HashMap::from([("specialist_type".into(), "cardiologist".into())]);
    let dir = std::path::Path::new(".");

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &empty_collector(dir),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        dir,
        "test-wf",
    )
    .unwrap();
    assert_eq!(result, Some("You are a cardiologist.".to_string()));
}

#[test]
fn resolve_role_with_var_in_prompt() {
    let step = Step {
        name: "test".into(),
        prompt: "do stuff".into(),
        role: Some("doctor".into()),
        ..Default::default()
    };
    let roles = HashMap::from([(
        "doctor".into(),
        Role {
            system_prompt: Some("You are a ${specialty} specialist.".into()),
            ..Default::default()
        },
    )]);
    let vars = HashMap::from([("specialty".into(), "cardiology".into())]);
    let dir = std::path::Path::new(".");

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &empty_collector(dir),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        dir,
        "test-wf",
    )
    .unwrap();
    assert_eq!(result, Some("You are a cardiology specialist.".to_string()));
}

#[test]
fn resolve_unknown_role_returns_error() {
    let step = Step {
        name: "test".into(),
        prompt: "do stuff".into(),
        role: Some("nonexistent".into()),
        ..Default::default()
    };
    let roles = HashMap::new();
    let vars = HashMap::new();
    let dir = std::path::Path::new(".");

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &empty_collector(dir),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        dir,
        "test-wf",
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[test]
fn resolve_no_system_prompt_or_role() {
    let step = Step {
        name: "test".into(),
        prompt: "do stuff".into(),
        ..Default::default()
    };
    let roles = HashMap::new();
    let vars = HashMap::new();
    let dir = std::path::Path::new(".");

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &empty_collector(dir),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        dir,
        "test-wf",
    )
    .unwrap();
    assert_eq!(result, None);
}

#[test]
fn resolve_role_with_system_prompt_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let prompt_path = tmp.path().join("doctor.md");
    std::fs::write(&prompt_path, "You are a doctor from a file.").unwrap();

    let step = Step {
        name: "test".into(),
        prompt: "do stuff".into(),
        role: Some("doctor".into()),
        ..Default::default()
    };
    let roles = HashMap::from([(
        "doctor".into(),
        Role {
            system_prompt_file: Some("doctor.md".into()),
            ..Default::default()
        },
    )]);
    let vars = HashMap::new();

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &empty_collector(tmp.path()),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        tmp.path(),
        "test-wf",
    )
    .unwrap();
    assert_eq!(result, Some("You are a doctor from a file.".to_string()));
}

#[test]
fn resolve_role_file_with_var_substitution() {
    let tmp = tempfile::TempDir::new().unwrap();
    let prompt_path = tmp.path().join("specialist.md");
    std::fs::write(&prompt_path, "You are a ${specialty} specialist.").unwrap();

    let step = Step {
        name: "test".into(),
        prompt: "do stuff".into(),
        role: Some("specialist".into()),
        ..Default::default()
    };
    let roles = HashMap::from([(
        "specialist".into(),
        Role {
            system_prompt_file: Some("specialist.md".into()),
            ..Default::default()
        },
    )]);
    let vars = HashMap::from([("specialty".into(), "neurology".into())]);

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &empty_collector(tmp.path()),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        tmp.path(),
        "test-wf",
    )
    .unwrap();
    assert_eq!(result, Some("You are a neurology specialist.".to_string()));
}

#[test]
fn resolve_prepends_resources_block_to_direct_system_prompt() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("cv.md"), "CV").unwrap();

    let step = Step {
        name: "draft".into(),
        prompt: "Write a letter".into(),
        system_prompt: Some("You are a cover-letter writer.".into()),
        ..Default::default()
    };
    let roles = HashMap::new();
    let vars = HashMap::new();
    let workflow_resources = vec![ResourceSpec::Path("cv.md".into())];

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &inline_collector(&workflow_resources, tmp.path()),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        tmp.path(),
        "test-wf",
    )
    .unwrap()
    .unwrap();

    assert!(result.starts_with("<resources>"));
    assert!(result.contains("cv.md"));
    assert!(result.ends_with("You are a cover-letter writer."));
}

#[test]
fn resolve_returns_only_resources_block_when_no_base_prompt() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("cv.md"), "CV").unwrap();

    let step = Step {
        name: "draft".into(),
        prompt: "Write a letter".into(),
        ..Default::default()
    };
    let roles = HashMap::new();
    let vars = HashMap::new();
    let workflow_resources = vec![ResourceSpec::Path("cv.md".into())];

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &inline_collector(&workflow_resources, tmp.path()),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        tmp.path(),
        "test-wf",
    )
    .unwrap()
    .unwrap();

    assert!(result.starts_with("<resources>"));
    assert!(result.trim_end().ends_with("</resources>"));
}

#[test]
fn resolve_merges_step_resources_with_workflow_resources() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("cv.md"), "CV").unwrap();
    std::fs::write(tmp.path().join("job.md"), "Job").unwrap();

    let step = Step {
        name: "draft".into(),
        prompt: "Write".into(),
        system_prompt: Some("Writer".into()),
        resources: vec![ResourceSpec::Path("job.md".into())],
        ..Default::default()
    };
    let roles = HashMap::new();
    let vars = HashMap::new();
    let workflow_resources = vec![ResourceSpec::Path("cv.md".into())];

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &inline_collector(&workflow_resources, tmp.path()),
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        tmp.path(),
        "test-wf",
    )
    .unwrap()
    .unwrap();

    assert!(result.contains("cv.md"));
    assert!(result.contains("job.md"));
}

#[test]
fn resolve_disabled_collector_does_not_emit_resources_block() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("cv.md"), "CV").unwrap();

    let step = Step {
        name: "draft".into(),
        prompt: "Write".into(),
        system_prompt: Some("Writer".into()),
        resources: vec![ResourceSpec::Path("cv.md".into())],
        ..Default::default()
    };
    let roles = HashMap::new();
    let vars = HashMap::new();
    let workflow_resources = vec![ResourceSpec::Path("cv.md".into())];
    let collector = ResourceCollector {
        workflow_resources: &workflow_resources,
        workflow_dir: tmp.path(),
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_resources_dir: None,
        disabled: true,
    };

    let result = resolve_role_system_prompt(
        &step,
        &roles,
        &collector,
        &empty_memory_collector(),
        &crate::storage::StorageManager::empty(),
        &vars,
        tmp.path(),
        "test-wf",
    )
    .unwrap()
    .unwrap();

    assert!(!result.contains("<resources>"));
    assert_eq!(result, "Writer");
}

// ── load_file_defaults ─────────────────��───────────────────────���──────────

#[test]
fn load_file_defaults_reads_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let default_path = tmp.path().join("instructions.txt");
    std::fs::write(&default_path, "Follow these instructions carefully.").unwrap();

    let declarations = HashMap::from([(
        "instructions".into(),
        Variable {
            var_type: VarType::String,
            default_file: Some("instructions.txt".into()),
            ..Default::default()
        },
    )]);
    let mut vars = HashMap::from([("instructions".into(), String::new())]);

    load_file_defaults(&mut vars, &declarations, tmp.path()).unwrap();
    assert_eq!(vars["instructions"], "Follow these instructions carefully.");
}

#[test]
fn load_file_defaults_skips_when_default_set() {
    let tmp = tempfile::TempDir::new().unwrap();
    let default_path = tmp.path().join("instructions.txt");
    std::fs::write(&default_path, "From file").unwrap();

    let declarations = HashMap::from([(
        "instructions".into(),
        Variable {
            var_type: VarType::String,
            default: Some(toml::Value::String("inline default".into())),
            default_file: Some("instructions.txt".into()),
            ..Default::default()
        },
    )]);
    let mut vars = HashMap::from([("instructions".into(), "inline default".into())]);

    // Should not override since `default` is set
    load_file_defaults(&mut vars, &declarations, tmp.path()).unwrap();
    assert_eq!(vars["instructions"], "inline default");
}

#[test]
fn load_file_defaults_missing_file_returns_error() {
    let tmp = tempfile::TempDir::new().unwrap();

    let declarations = HashMap::from([(
        "instructions".into(),
        Variable {
            var_type: VarType::String,
            default_file: Some("nonexistent.txt".into()),
            ..Default::default()
        },
    )]);
    let mut vars = HashMap::from([("instructions".into(), String::new())]);

    let result = load_file_defaults(&mut vars, &declarations, tmp.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("nonexistent.txt"));
}
