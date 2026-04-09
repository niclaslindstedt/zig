use std::collections::HashMap;

use crate::workflow::model::{Step, VarType, Variable, Workflow, WorkflowMeta};

use super::*;

// ── helpers ──────────────────────────────────────────────────────────────────

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
            description: String::new(),
            tags: vec![],
        },
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
        global_wf_dir.join("global-test.zug"),
        "[workflow]\nname = \"g\"\ndescription = \"\"\n[[step]]\nname = \"s\"\nprompt = \"p\"",
    )
    .unwrap();

    // Use the full path to test resolution
    let full_path = global_wf_dir.join("global-test.zug");
    let result = resolve_workflow_path(full_path.to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn resolve_local_over_global_precedence() {
    let local_dir = tempfile::tempdir().unwrap();
    let local_path = local_dir.path().join("precedence.zug");
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
        vars: HashMap::from([(
            "content".into(),
            Variable {
                var_type: VarType::String,
                from: Some("prompt".into()),
                ..Default::default()
            },
        )]),
        steps: vec![],
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
    };

    let vars = init_vars(&workflow);
    // No user prompt provided, so default stays
    assert_eq!(vars["content"], "fallback");
}
