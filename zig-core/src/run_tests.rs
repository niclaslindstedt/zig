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

// ── build_agent_config ──────────────────────────────────────────────────────

#[test]
fn build_agent_config_basic() {
    let mut s = step("test");
    s.provider = Some("claude".into());
    s.model = Some("sonnet".into());
    s.system_prompt = Some("be helpful".into());
    s.max_turns = Some(5);
    s.json = true;
    s.timeout = Some("5m".into());
    s.tags = vec!["review".into()];

    let cfg = build_agent_config(
        &s,
        "do stuff",
        "my-workflow",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );

    assert_eq!(cfg.command, "run");
    assert_eq!(cfg.prompt, "do stuff");
    assert_eq!(cfg.provider.as_deref(), Some("claude"));
    assert_eq!(cfg.model.as_deref(), Some("sonnet"));
    assert_eq!(cfg.system_prompt.as_deref(), Some("be helpful"));
    assert_eq!(cfg.max_turns, Some(5));
    assert!(cfg.json_mode);
    assert_eq!(cfg.timeout.as_deref(), Some("5m"));
    assert_eq!(cfg.session_name, "zig-my-workflow-test");
    assert!(cfg.tags.contains(&"zig-workflow".to_string()));
    assert!(cfg.tags.contains(&"review".to_string()));
}

#[test]
fn build_agent_config_auto_approve() {
    let mut s = step("test");
    s.auto_approve = true;
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(cfg.auto_approve);

    let s2 = step("test");
    let cfg2 = build_agent_config(
        &s2,
        "prompt",
        "wf",
        None,
        s2.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(!cfg2.auto_approve);
}

#[test]
fn build_agent_config_env() {
    let mut s = step("test");
    s.env = HashMap::from([("MODE".into(), "strict".into())]);
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.env, vec![("MODE".to_string(), "strict".to_string())]);
}

#[test]
fn build_agent_config_isolation() {
    let mut s = step("test");
    s.worktree = true;
    s.sandbox = Some("worker-box".into());
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(matches!(cfg.worktree, Some(None)));
    assert_eq!(cfg.sandbox.as_deref(), Some("worker-box"));
}

#[test]
fn build_agent_config_files_and_dirs() {
    let mut s = step("test");
    s.files = vec!["input.txt".into(), "data.json".into()];
    s.add_dirs = vec!["/tmp/shared".into()];
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );

    assert_eq!(cfg.files.len(), 2);
    assert!(cfg.files.contains(&"input.txt".to_string()));
    assert!(cfg.files.contains(&"data.json".to_string()));
    assert!(cfg.add_dirs.contains(&"/tmp/shared".to_string()));
}

#[test]
fn build_agent_config_description() {
    let mut s = step("test");
    s.description = "Analyze the code".into();
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.description.as_deref(), Some("Analyze the code"));

    // Empty description should produce None
    let s2 = step("test");
    let cfg2 = build_agent_config(
        &s2,
        "prompt",
        "wf",
        None,
        s2.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(cfg2.description.is_none());
}

#[test]
fn build_agent_config_json_schema() {
    let mut s = step("test");
    s.json_schema = Some(r#"{"type":"object"}"#.into());
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.json_schema.as_deref(), Some(r#"{"type":"object"}"#));
}

#[test]
fn build_agent_config_root() {
    let mut s = step("test");
    s.root = Some("/tmp/work".into());
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.root.as_deref(), Some("/tmp/work"));
}

#[test]
fn build_agent_config_model_override() {
    let mut s = step("test");
    s.model = Some("sonnet".into());
    let cfg = build_agent_config(
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
    assert_eq!(cfg.model.as_deref(), Some("opus"));
}

#[test]
fn build_agent_config_model_no_override() {
    let mut s = step("test");
    s.model = Some("sonnet".into());
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.model.as_deref(), Some("sonnet"));
}

#[test]
fn build_agent_config_no_model() {
    let s = step("test");
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(cfg.model.is_none());
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

// ── build_pipe_context ──────────────────────────────────────────────────────

#[test]
fn build_pipe_context_errors_when_no_sessions_resolve() {
    // UUIDs that are not in any known session store must surface as a
    // concrete error rather than producing an empty/invalid prompt.
    let ids = vec![
        "00000000-0000-0000-0000-000000000001".to_string(),
        "00000000-0000-0000-0000-000000000002".to_string(),
    ];
    let err = build_pipe_context(&ids, None).unwrap_err();
    assert!(
        err.to_string().contains("no results available"),
        "unexpected error: {err}"
    );
}

#[test]
fn build_pipe_context_errors_on_empty_input() {
    let err = build_pipe_context(&[], None).unwrap_err();
    assert!(
        err.to_string().contains("no results available"),
        "unexpected error: {err}"
    );
}

// ── resolve_plan_output_path ────────────────────────────────────────────────

#[test]
fn resolve_plan_output_path_preserves_explicit_filename() {
    let target = resolve_plan_output_path("plans/oauth.md");
    assert_eq!(target, std::path::PathBuf::from("plans/oauth.md"));
}

#[test]
fn resolve_plan_output_path_appends_timestamped_file_for_directory_input() {
    let target = resolve_plan_output_path("plans");
    // No extension on input → generated filename inside the directory.
    assert_eq!(target.parent(), Some(std::path::Path::new("plans")));
    let name = target.file_name().and_then(|s| s.to_str()).unwrap();
    assert!(name.starts_with("plan-"), "bad filename: {name}");
    assert!(name.ends_with(".md"), "bad filename: {name}");
}

// ── build_agent_config: context / plan / mcp ────────────────────────────────

#[test]
fn build_agent_config_mcp_config() {
    let mut s = step("test");
    s.mcp_config = Some("config.json".into());
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.mcp_config.as_deref(), Some("config.json"));
}

#[test]
fn build_agent_config_no_context_by_default() {
    let s = step("test");
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(cfg.mcp_config.is_none());
}

// ── build_agent_config: output format ──────────────────────────────────────

#[test]
fn build_agent_config_output_format() {
    let mut s = step("test");
    s.output = Some("stream-json".into());
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.output_format.as_deref(), Some("stream-json"));
    assert!(!cfg.json_mode);
}

#[test]
fn build_agent_config_output_overrides_json() {
    let mut s = step("test");
    s.output = Some("text".into());
    s.json = true;
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.output_format.as_deref(), Some("text"));
    assert!(!cfg.json_mode);
}

#[test]
fn build_agent_config_json_fallback() {
    let mut s = step("test");
    s.json = true;
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(cfg.json_mode);
    assert!(cfg.output_format.is_none());
}

// ── build_agent_config: command step types ─────────────────────────────────

#[test]
fn build_agent_config_command_review() {
    let mut s = step("review-step");
    s.command = Some(StepCommand::Review);
    s.uncommitted = true;
    s.base = Some("main".into());
    s.commit = Some("abc123".into());
    s.title = Some("Security Review".into());
    let cfg = build_agent_config(
        &s,
        "focus on auth",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.command, "review");
    assert_eq!(cfg.prompt, "focus on auth");
    match cfg.command_params.as_ref().unwrap() {
        CommandParams::Review {
            uncommitted,
            base,
            commit,
            title,
        } => {
            assert!(*uncommitted);
            assert_eq!(base.as_deref(), Some("main"));
            assert_eq!(commit.as_deref(), Some("abc123"));
            assert_eq!(title.as_deref(), Some("Security Review"));
        }
        _ => panic!("expected Review params"),
    }
}

#[test]
fn build_agent_config_command_plan() {
    let mut s = step("plan-step");
    s.command = Some(StepCommand::Plan);
    s.plan_output = Some("auth-plan.md".into());
    s.instructions = Some("Focus on security".into());
    let cfg = build_agent_config(
        &s,
        "Design auth system",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.command, "plan");
    assert_eq!(cfg.prompt, "Design auth system");
    match cfg.command_params.as_ref().unwrap() {
        CommandParams::Plan {
            output,
            instructions,
        } => {
            assert_eq!(output.as_deref(), Some("auth-plan.md"));
            assert_eq!(instructions.as_deref(), Some("Focus on security"));
        }
        _ => panic!("expected Plan params"),
    }
}

#[test]
fn build_agent_config_command_pipe() {
    let mut s = step("synth");
    s.command = Some(StepCommand::Pipe);
    s.depends_on = vec!["analyze".into(), "review".into()];
    let cfg = build_agent_config(
        &s,
        "Combine results",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.command, "pipe");
    match cfg.command_params.as_ref().unwrap() {
        CommandParams::Pipe { session_ids } => {
            assert_eq!(
                session_ids,
                &vec!["zig-wf-analyze".to_string(), "zig-wf-review".to_string()]
            );
        }
        _ => panic!("expected Pipe params"),
    }
    assert_eq!(cfg.prompt, "Combine results");
}

#[test]
fn build_agent_config_command_collect() {
    let mut s = step("gather");
    s.command = Some(StepCommand::Collect);
    s.depends_on = vec!["worker-a".into(), "worker-b".into()];
    let cfg = build_agent_config(
        &s,
        "",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.command, "collect");
    match cfg.command_params.as_ref().unwrap() {
        CommandParams::Collect { session_ids } => {
            assert_eq!(
                session_ids,
                &vec!["zig-wf-worker-a".to_string(), "zig-wf-worker-b".to_string()]
            );
        }
        _ => panic!("expected Collect params"),
    }
    // collect doesn't accept agent args
    assert!(!cfg.accepts_agent_args);
    assert!(cfg.provider.is_none());
}

#[test]
fn build_agent_config_command_summary() {
    let mut s = step("stats");
    s.command = Some(StepCommand::Summary);
    s.depends_on = vec!["worker".into()];
    let cfg = build_agent_config(
        &s,
        "",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.command, "summary");
    match cfg.command_params.as_ref().unwrap() {
        CommandParams::Summary { session_ids } => {
            assert_eq!(session_ids, &vec!["zig-wf-worker".to_string()]);
        }
        _ => panic!("expected Summary params"),
    }
}

#[test]
fn build_agent_config_collect_no_agent_args() {
    let mut s = step("gather");
    s.command = Some(StepCommand::Collect);
    s.depends_on = vec!["a".into()];
    s.provider = Some("claude".into());
    s.model = Some("sonnet".into());
    s.auto_approve = true;
    let cfg = build_agent_config(
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
    assert!(cfg.provider.is_none());
    assert!(cfg.model.is_none());
    assert!(!cfg.auto_approve);
}

#[test]
fn build_agent_config_review_accepts_agent_args() {
    let mut s = step("review-step");
    s.command = Some(StepCommand::Review);
    s.provider = Some("claude".into());
    s.model = Some("opus".into());
    let cfg = build_agent_config(
        &s,
        "review code",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert!(cfg.accepts_agent_args);
    assert_eq!(cfg.provider.as_deref(), Some("claude"));
    assert_eq!(cfg.model.as_deref(), Some("opus"));
}

#[test]
fn build_agent_config_default_command_unchanged() {
    let s = step("test");
    let cfg = build_agent_config(
        &s,
        "do stuff",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.command, "run");
    assert_eq!(cfg.prompt, "do stuff");
    assert!(cfg.command_params.is_none());
}

#[test]
fn build_agent_config_session_metadata_on_all_commands() {
    // Even collect/summary should get session_name, tags, description
    let mut s = step("gather");
    s.command = Some(StepCommand::Collect);
    s.depends_on = vec!["a".into()];
    s.description = "Gather results".into();
    s.tags = vec!["custom".into()];
    s.timeout = Some("5m".into());
    let cfg = build_agent_config(
        &s,
        "",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.session_name, "zig-wf-gather");
    assert_eq!(cfg.description.as_deref(), Some("Gather results"));
    assert!(cfg.tags.contains(&"zig-workflow".to_string()));
    assert!(cfg.tags.contains(&"custom".to_string()));
    assert_eq!(cfg.timeout.as_deref(), Some("5m"));
}

// ── build_agent_config: system_prompt variable substitution ───────────────

#[test]
fn build_agent_config_rendered_system_prompt() {
    let mut s = step("test");
    s.system_prompt = Some("You are a ${role}".into());
    // Pass pre-rendered (substituted) value
    let cfg = build_agent_config(
        &s,
        "do stuff",
        "wf",
        None,
        Some("You are a cardiologist"),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.system_prompt.as_deref(), Some("You are a cardiologist"));
}

#[test]
fn build_agent_config_no_system_prompt() {
    let s = step("test");
    let cfg = build_agent_config(&s, "do stuff", "wf", None, None, None, None, &[]);
    assert!(cfg.system_prompt.is_none());
}

// ── build_agent_config: workflow-level provider/model fallback ────────────

#[test]
fn build_agent_config_workflow_provider_fallback() {
    let s = step("test");
    let cfg = build_agent_config(&s, "prompt", "wf", None, None, Some("claude"), None, &[]);
    assert_eq!(cfg.provider.as_deref(), Some("claude"));
}

#[test]
fn build_agent_config_workflow_model_fallback() {
    let s = step("test");
    let cfg = build_agent_config(&s, "prompt", "wf", None, None, None, Some("sonnet"), &[]);
    assert_eq!(cfg.model.as_deref(), Some("sonnet"));
}

#[test]
fn build_agent_config_step_provider_overrides_workflow() {
    let mut s = step("test");
    s.provider = Some("gemini".into());
    let cfg = build_agent_config(&s, "prompt", "wf", None, None, Some("claude"), None, &[]);
    assert_eq!(cfg.provider.as_deref(), Some("gemini"));
}

#[test]
fn build_agent_config_step_model_overrides_workflow() {
    let mut s = step("test");
    s.model = Some("opus".into());
    let cfg = build_agent_config(&s, "prompt", "wf", None, None, None, Some("sonnet"), &[]);
    assert_eq!(cfg.model.as_deref(), Some("opus"));
}

#[test]
fn build_agent_config_no_provider_no_workflow_provider() {
    let s = step("test");
    let cfg = build_agent_config(&s, "prompt", "wf", None, None, None, None, &[]);
    assert!(cfg.provider.is_none());
    assert!(cfg.model.is_none());
}

#[test]
fn build_agent_config_model_override_beats_workflow_model() {
    let s = step("test");
    let cfg = build_agent_config(
        &s,
        "prompt",
        "wf",
        Some("opus"),
        None,
        None,
        Some("sonnet"),
        &[],
    );
    assert_eq!(cfg.model.as_deref(), Some("opus"));
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

#[test]
fn build_agent_config_interactive_appends_self_terminate_instruction() {
    let mut s = step("chat");
    s.interactive = true;
    let cfg = build_agent_config(
        &s,
        "hello",
        "wf",
        None,
        Some("You are helpful."),
        None,
        None,
        &[],
    );
    let sp = cfg
        .system_prompt
        .expect("interactive step should have a system prompt");
    assert!(sp.starts_with("You are helpful."), "base prompt preserved");
    assert!(
        sp.contains("zig self terminate"),
        "self-terminate instruction appended: {sp}"
    );
}

#[test]
fn build_agent_config_interactive_no_base_system_prompt_still_gets_instruction() {
    let mut s = step("chat");
    s.interactive = true;
    let cfg = build_agent_config(&s, "hello", "wf", None, None, None, None, &[]);
    let sp = cfg
        .system_prompt
        .expect("interactive step should have a system prompt even without a base");
    assert!(sp.contains("zig self terminate"));
}

#[test]
fn build_agent_config_non_interactive_has_no_self_terminate_instruction() {
    let mut s = step("chat");
    s.interactive = false;
    let cfg = build_agent_config(
        &s,
        "hello",
        "wf",
        None,
        Some("You are helpful."),
        None,
        None,
        &[],
    );
    assert_eq!(cfg.system_prompt.as_deref(), Some("You are helpful."));
}

#[test]
fn build_agent_config_interactive_keeps_run_command_and_omits_json() {
    let mut s = step("chat");
    s.interactive = true;
    let cfg = build_agent_config(
        &s,
        "hello",
        "wf",
        None,
        s.system_prompt.as_deref(),
        None,
        None,
        &[],
    );
    // Default `run` command (zag is interactive by default on `run`).
    assert_eq!(cfg.command, "run");
    assert!(cfg.interactive);
    // Never set json_mode for interactive steps — validation forbids
    // json=true, and the interactive flag alone must not flip on JSON mode.
    assert!(!cfg.json_mode);
    assert!(cfg.output_format.is_none());
    // Session metadata still applies.
    assert_eq!(cfg.session_name, "zig-wf-chat");
}
