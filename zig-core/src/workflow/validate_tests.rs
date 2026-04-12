use super::*;
use crate::workflow::parser::parse;

#[test]
fn valid_minimal_workflow() {
    let wf = parse(
        r#"
[workflow]
name = "valid"

[[step]]
name = "hello"
prompt = "Say hello"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn valid_pipeline_workflow() {
    let wf = parse(
        r#"
[workflow]
name = "pipeline"

[vars.target]
type = "string"
default = "."

[[step]]
name = "step-a"
prompt = "Analyze ${target}"

[[step]]
name = "step-b"
prompt = "Review"
depends_on = ["step-a"]
inject_context = true

[[step]]
name = "step-c"
prompt = "Report"
depends_on = ["step-b"]
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn error_no_steps() {
    let wf = parse(
        r#"
[workflow]
name = "empty"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].to_string().contains("at least one step"));
}

#[test]
fn error_duplicate_step_names() {
    let wf = parse(
        r#"
[workflow]
name = "dupes"

[[step]]
name = "hello"
prompt = "First"

[[step]]
name = "hello"
prompt = "Second"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.to_string().contains("duplicate step name"))
    );
}

#[test]
fn error_unknown_dependency() {
    let wf = parse(
        r#"
[workflow]
name = "bad-dep"

[[step]]
name = "a"
prompt = "Hello"
depends_on = ["nonexistent"]
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.to_string().contains("unknown step 'nonexistent'"))
    );
}

#[test]
fn error_self_dependency() {
    let wf = parse(
        r#"
[workflow]
name = "self-dep"

[[step]]
name = "loop"
prompt = "I depend on myself"
depends_on = ["loop"]
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.to_string().contains("depends on itself"))
    );
}

#[test]
fn error_unknown_variable_in_prompt() {
    let wf = parse(
        r#"
[workflow]
name = "bad-var"

[[step]]
name = "a"
prompt = "Analyze ${nonexistent}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.to_string().contains("unknown variable '${nonexistent}'"))
    );
}

#[test]
fn error_saves_unknown_variable() {
    let wf = parse(
        r#"
[workflow]
name = "bad-saves"

[[step]]
name = "a"
prompt = "Score this"
json = true

[step.saves]
unknown_var = "$.score"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.to_string().contains("unknown variable 'unknown_var'"))
    );
}

#[test]
fn error_unknown_next_step() {
    let wf = parse(
        r#"
[workflow]
name = "bad-next"

[[step]]
name = "a"
prompt = "Do something"
next = "nowhere"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.to_string().contains("unknown next step 'nowhere'"))
    );
}

#[test]
fn error_dependency_cycle() {
    let wf = parse(
        r#"
[workflow]
name = "cycle"

[[step]]
name = "a"
prompt = "Step A"
depends_on = ["b"]

[[step]]
name = "b"
prompt = "Step B"
depends_on = ["a"]
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| e.to_string().contains("cycle")));
}

#[test]
fn valid_fan_out_gather() {
    let wf = parse(
        r#"
[workflow]
name = "fan-out"

[[step]]
name = "security"
prompt = "Security review"

[[step]]
name = "perf"
prompt = "Performance review"

[[step]]
name = "style"
prompt = "Style review"

[[step]]
name = "synthesize"
prompt = "Combine all findings"
depends_on = ["security", "perf", "style"]
inject_context = true
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn valid_generator_critic_loop() {
    let wf = parse(
        r#"
[workflow]
name = "gen-crit"

[vars.score]
type = "number"
default = 0

[vars.threshold]
type = "number"
default = 8

[vars.feedback]
type = "string"
default = ""

[[step]]
name = "generate"
prompt = "Write the code. Feedback: ${feedback}"

[[step]]
name = "critique"
prompt = "Score 1-10"
depends_on = ["generate"]
inject_context = true
json = true
saves = { score = "$.score", feedback = "$.suggestions" }

[[step]]
name = "refine"
prompt = "Improve based on: ${feedback}"
depends_on = ["critique"]
condition = "score < threshold"
next = "critique"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn error_condition_references_unknown_var() {
    let wf = parse(
        r#"
[workflow]
name = "bad-cond"

[[step]]
name = "a"
prompt = "Do something"
condition = "mystery > 5"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| e.to_string().contains("mystery")));
}

#[test]
fn valid_dotted_var_ref_in_prompt() {
    let wf = parse(
        r#"
[workflow]
name = "dotted"

[vars.result]
type = "json"

[[step]]
name = "use-nested"
prompt = "The score was ${result.score}"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn error_retry_model_without_retry_policy() {
    let wf = parse(
        r#"
[workflow]
name = "bad-retry-model"

[[step]]
name = "a"
prompt = "Do something"
retry_model = "large"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string()
            .contains("retry_model but on_failure is not 'retry'")
    }));
}

#[test]
fn error_race_group_internal_dependency() {
    let wf = parse(
        r#"
[workflow]
name = "bad-race"

[[step]]
name = "approach-a"
prompt = "Try approach A"
race_group = "solver"

[[step]]
name = "approach-b"
prompt = "Try approach B"
race_group = "solver"
depends_on = ["approach-a"]
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| e.to_string().contains("race_group")));
}

#[test]
fn valid_race_group() {
    let wf = parse(
        r#"
[workflow]
name = "good-race"

[[step]]
name = "approach-a"
prompt = "Try approach A"
race_group = "solver"

[[step]]
name = "approach-b"
prompt = "Try approach B"
race_group = "solver"

[[step]]
name = "use-result"
prompt = "Use the winning solution"
depends_on = ["approach-a", "approach-b"]
inject_context = true
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn error_three_node_cycle() {
    let wf = parse(
        r#"
[workflow]
name = "three-cycle"

[[step]]
name = "a"
prompt = "Step A"
depends_on = ["c"]

[[step]]
name = "b"
prompt = "Step B"
depends_on = ["a"]

[[step]]
name = "c"
prompt = "Step C"
depends_on = ["b"]
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| e.to_string().contains("cycle")));
}

#[test]
fn multiple_errors_reported_at_once() {
    let wf = parse(
        r#"
[workflow]
name = "multi-error"

[[step]]
name = "a"
prompt = "Uses ${missing_var}"
depends_on = ["nonexistent"]
next = "nowhere"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors.len() >= 3,
        "expected at least 3 errors, got {}",
        errors.len()
    );
}

// ── Variable constraint validation ──────────────────────────────────────────

#[test]
fn valid_variable_constraints() {
    let wf = parse(
        r#"
[workflow]
name = "good-constraints"

[vars.content]
type = "string"
from = "prompt"
required = true
min_length = 5
max_length = 100
pattern = "^[A-Z]"

[vars.priority]
type = "string"
default = "medium"
allowed_values = ["low", "medium", "high"]

[vars.score]
type = "number"
default = 50
min = 0.0
max = 100.0

[[step]]
name = "go"
prompt = "Process ${content} at ${priority}, score ${score}"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn error_unsupported_from_value() {
    let wf = parse(
        r#"
[workflow]
name = "bad-from"

[vars.data]
type = "string"
from = "stdin"

[[step]]
name = "go"
prompt = "Use ${data}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        let msg = e.to_string();
        msg.contains("unsupported from value 'stdin'")
    }));
}

#[test]
fn error_multiple_from_prompt() {
    let wf = parse(
        r#"
[workflow]
name = "multi-prompt"

[vars.a]
type = "string"
from = "prompt"

[vars.b]
type = "string"
from = "prompt"

[[step]]
name = "go"
prompt = "Use ${a} and ${b}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("multiple variables have from") })
    );
}

#[test]
fn error_min_length_on_number() {
    let wf = parse(
        r#"
[workflow]
name = "bad-constraint-type"

[vars.count]
type = "number"
min_length = 5

[[step]]
name = "go"
prompt = "Count: ${count}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("min_length") && e.to_string().contains("only valid for 'string'")
    }));
}

#[test]
fn error_min_on_string() {
    let wf = parse(
        r#"
[workflow]
name = "bad-min-type"

[vars.name]
type = "string"
min = 5.0

[[step]]
name = "go"
prompt = "Name: ${name}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("min") && e.to_string().contains("only valid for 'number'")
    }));
}

#[test]
fn error_min_length_greater_than_max_length() {
    let wf = parse(
        r#"
[workflow]
name = "bad-range"

[vars.text]
type = "string"
min_length = 100
max_length = 10

[[step]]
name = "go"
prompt = "Text: ${text}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string()
            .contains("min_length (100) greater than max_length (10)")
    }));
}

#[test]
fn error_min_greater_than_max() {
    let wf = parse(
        r#"
[workflow]
name = "bad-num-range"

[vars.val]
type = "number"
min = 100.0
max = 10.0

[[step]]
name = "go"
prompt = "Val: ${val}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("min (100) greater than max (10)") })
    );
}

#[test]
fn error_invalid_regex_pattern() {
    let wf = parse(
        r#"
[workflow]
name = "bad-regex"

[vars.text]
type = "string"
pattern = "[invalid("

[[step]]
name = "go"
prompt = "Text: ${text}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("invalid regex pattern") })
    );
}

#[test]
fn error_allowed_values_type_mismatch() {
    let wf = parse(
        r#"
[workflow]
name = "bad-allowed"

[vars.count]
type = "number"
allowed_values = ["one", "two"]

[[step]]
name = "go"
prompt = "Count: ${count}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("incompatible with type") })
    );
}

#[test]
fn error_default_violates_min_length() {
    let wf = parse(
        r#"
[workflow]
name = "bad-default"

[vars.content]
type = "string"
default = "hi"
min_length = 10

[[step]]
name = "go"
prompt = "Content: ${content}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("default value violates constraint") })
    );
}

#[test]
fn error_default_violates_allowed_values() {
    let wf = parse(
        r#"
[workflow]
name = "bad-default-allowed"

[vars.priority]
type = "string"
default = "urgent"
allowed_values = ["low", "medium", "high"]

[[step]]
name = "go"
prompt = "Priority: ${priority}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("default value violates constraint") })
    );
}

// ── Runtime value validation ────────────────────────────────────────────────

#[test]
fn runtime_required_empty_fails() {
    let decls = std::collections::HashMap::from([(
        "content".to_string(),
        crate::workflow::model::Variable {
            var_type: crate::workflow::model::VarType::String,
            required: true,
            ..Default::default()
        },
    )]);
    let vars = std::collections::HashMap::from([("content".to_string(), String::new())]);

    let result = validate_var_values(&vars, &decls);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("required but was not provided") })
    );
}

#[test]
fn runtime_required_nonempty_passes() {
    let decls = std::collections::HashMap::from([(
        "content".to_string(),
        crate::workflow::model::Variable {
            var_type: crate::workflow::model::VarType::String,
            required: true,
            ..Default::default()
        },
    )]);
    let vars =
        std::collections::HashMap::from([("content".to_string(), "hello world".to_string())]);

    assert!(validate_var_values(&vars, &decls).is_ok());
}

#[test]
fn runtime_min_length_fails() {
    let decls = std::collections::HashMap::from([(
        "text".to_string(),
        crate::workflow::model::Variable {
            var_type: crate::workflow::model::VarType::String,
            min_length: Some(10),
            ..Default::default()
        },
    )]);
    let vars = std::collections::HashMap::from([("text".to_string(), "short".to_string())]);

    let errors = validate_var_values(&vars, &decls).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("at least 10 characters") })
    );
}

#[test]
fn runtime_max_length_fails() {
    let decls = std::collections::HashMap::from([(
        "text".to_string(),
        crate::workflow::model::Variable {
            var_type: crate::workflow::model::VarType::String,
            max_length: Some(5),
            ..Default::default()
        },
    )]);
    let vars = std::collections::HashMap::from([("text".to_string(), "way too long".to_string())]);

    let errors = validate_var_values(&vars, &decls).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("at most 5 characters") })
    );
}

#[test]
fn runtime_min_number_fails() {
    let decls = std::collections::HashMap::from([(
        "score".to_string(),
        crate::workflow::model::Variable {
            var_type: crate::workflow::model::VarType::Number,
            min: Some(0.0),
            ..Default::default()
        },
    )]);
    let vars = std::collections::HashMap::from([("score".to_string(), "-5".to_string())]);

    let errors = validate_var_values(&vars, &decls).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("at least 0") })
    );
}

#[test]
fn runtime_max_number_fails() {
    let decls = std::collections::HashMap::from([(
        "score".to_string(),
        crate::workflow::model::Variable {
            var_type: crate::workflow::model::VarType::Number,
            max: Some(100.0),
            ..Default::default()
        },
    )]);
    let vars = std::collections::HashMap::from([("score".to_string(), "150".to_string())]);

    let errors = validate_var_values(&vars, &decls).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("at most 100") })
    );
}

#[test]
fn runtime_pattern_fails() {
    let decls = std::collections::HashMap::from([(
        "code".to_string(),
        crate::workflow::model::Variable {
            var_type: crate::workflow::model::VarType::String,
            pattern: Some("^[A-Z]{3}-\\d+$".to_string()),
            ..Default::default()
        },
    )]);
    let vars = std::collections::HashMap::from([("code".to_string(), "invalid".to_string())]);

    let errors = validate_var_values(&vars, &decls).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("must match pattern") })
    );
}

#[test]
fn runtime_pattern_passes() {
    let decls = std::collections::HashMap::from([(
        "code".to_string(),
        crate::workflow::model::Variable {
            var_type: crate::workflow::model::VarType::String,
            pattern: Some("^[A-Z]{3}-\\d+$".to_string()),
            ..Default::default()
        },
    )]);
    let vars = std::collections::HashMap::from([("code".to_string(), "ABC-123".to_string())]);

    assert!(validate_var_values(&vars, &decls).is_ok());
}

#[test]
fn runtime_allowed_values_fails() {
    let decls = std::collections::HashMap::from([(
        "priority".to_string(),
        crate::workflow::model::Variable {
            var_type: crate::workflow::model::VarType::String,
            allowed_values: Some(vec![
                toml::Value::String("low".into()),
                toml::Value::String("medium".into()),
                toml::Value::String("high".into()),
            ]),
            ..Default::default()
        },
    )]);
    let vars = std::collections::HashMap::from([("priority".to_string(), "urgent".to_string())]);

    let errors = validate_var_values(&vars, &decls).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("must be one of") })
    );
}

#[test]
fn runtime_allowed_values_passes() {
    let decls = std::collections::HashMap::from([(
        "priority".to_string(),
        crate::workflow::model::Variable {
            var_type: crate::workflow::model::VarType::String,
            allowed_values: Some(vec![
                toml::Value::String("low".into()),
                toml::Value::String("medium".into()),
                toml::Value::String("high".into()),
            ]),
            ..Default::default()
        },
    )]);
    let vars = std::collections::HashMap::from([("priority".to_string(), "high".to_string())]);

    assert!(validate_var_values(&vars, &decls).is_ok());
}

#[test]
fn runtime_empty_nonrequired_skips_constraints() {
    let decls = std::collections::HashMap::from([(
        "text".to_string(),
        crate::workflow::model::Variable {
            var_type: crate::workflow::model::VarType::String,
            min_length: Some(10),
            pattern: Some("^[A-Z]".to_string()),
            ..Default::default()
        },
    )]);
    let vars = std::collections::HashMap::from([("text".to_string(), String::new())]);

    // Empty non-required variable should pass — constraints only apply to provided values
    assert!(validate_var_values(&vars, &decls).is_ok());
}

// ── mcp_config validation ────────────────────────────────────────────────────

#[test]
fn error_mcp_config_with_non_claude_provider() {
    let wf = parse(
        r#"
[workflow]
name = "bad-mcp"

[[step]]
name = "a"
prompt = "Do something"
provider = "gemini"
mcp_config = "config.json"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("mcp_config")
            && e.to_string()
                .contains("only supported with the claude provider")
    }));
}

#[test]
fn valid_mcp_config_with_claude_provider() {
    let wf = parse(
        r#"
[workflow]
name = "good-mcp"

[[step]]
name = "a"
prompt = "Do something"
provider = "claude"
mcp_config = "config.json"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn valid_mcp_config_without_provider() {
    let wf = parse(
        r#"
[workflow]
name = "mcp-no-provider"

[[step]]
name = "a"
prompt = "Do something"
mcp_config = "config.json"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

// ── output format validation ─────────────────────────────────────────────────

#[test]
fn error_invalid_output_format() {
    let wf = parse(
        r#"
[workflow]
name = "bad-output"

[[step]]
name = "a"
prompt = "Do something"
output = "csv"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("invalid output format 'csv'") })
    );
}

#[test]
fn valid_output_formats() {
    for fmt in &["text", "json", "json-pretty", "stream-json", "native-json"] {
        let toml = format!(
            r#"
[workflow]
name = "output-test"

[[step]]
name = "a"
prompt = "Do something"
output = "{fmt}"
"#
        );
        let wf = parse(&toml).unwrap();
        assert!(validate(&wf).is_ok(), "format '{fmt}' should be valid");
    }
}

// ── command step type validation ─────────────────────────────────────────────

#[test]
fn error_review_fields_without_review_command() {
    let wf = parse(
        r#"
[workflow]
name = "bad-review"

[[step]]
name = "a"
prompt = "Do something"
uncommitted = true
base = "main"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("'uncommitted'") && e.to_string().contains("not 'review'")
    }));
    assert!(
        errors.iter().any(|e| {
            e.to_string().contains("'base'") && e.to_string().contains("not 'review'")
        })
    );
}

#[test]
fn error_plan_fields_without_plan_command() {
    let wf = parse(
        r#"
[workflow]
name = "bad-plan"

[[step]]
name = "a"
prompt = "Do something"
plan_output = "out.md"
instructions = "focus"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("'plan_output'") && e.to_string().contains("not 'plan'")
    }));
    assert!(errors.iter().any(|e| {
        e.to_string().contains("'instructions'") && e.to_string().contains("not 'plan'")
    }));
}

#[test]
fn error_pipe_without_depends_on() {
    let wf = parse(
        r#"
[workflow]
name = "bad-pipe"

[[step]]
name = "a"
prompt = "Do something"
command = "pipe"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("command 'pipe'") && e.to_string().contains("no depends_on")
    }));
}

#[test]
fn error_collect_without_depends_on() {
    let wf = parse(
        r#"
[workflow]
name = "bad-collect"

[[step]]
name = "a"
prompt = ""
command = "collect"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("command 'collect'") && e.to_string().contains("no depends_on")
    }));
}

#[test]
fn error_summary_without_depends_on() {
    let wf = parse(
        r#"
[workflow]
name = "bad-summary"

[[step]]
name = "a"
prompt = ""
command = "summary"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("command 'summary'") && e.to_string().contains("no depends_on")
    }));
}

#[test]
fn valid_review_command() {
    let wf = parse(
        r#"
[workflow]
name = "good-review"

[[step]]
name = "review"
prompt = "Review the code"
command = "review"
uncommitted = true
base = "main"
commit = "abc123"
title = "Code Review"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn valid_plan_command() {
    let wf = parse(
        r#"
[workflow]
name = "good-plan"

[[step]]
name = "design"
prompt = "Design the auth system"
command = "plan"
plan_output = "auth-plan.md"
instructions = "Focus on security"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn valid_pipe_with_depends_on() {
    let wf = parse(
        r#"
[workflow]
name = "good-pipe"

[[step]]
name = "analyze"
prompt = "Analyze code"

[[step]]
name = "synthesize"
prompt = "Combine results"
command = "pipe"
depends_on = ["analyze"]
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

// ── role validation ──────────────────────────────────────────────────────

#[test]
fn valid_step_with_static_role() {
    let wf = parse(
        r#"
[workflow]
name = "static-role"

[roles.doctor]
system_prompt = "You are a doctor."

[[step]]
name = "examine"
prompt = "Examine the patient"
role = "doctor"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn valid_step_with_dynamic_role() {
    let wf = parse(
        r#"
[workflow]
name = "dynamic-role"

[roles.cardiologist]
system_prompt = "You are a cardiologist."

[vars.specialist_type]
type = "string"
default = "cardiologist"

[[step]]
name = "examine"
prompt = "Examine the patient"
role = "${specialist_type}"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn error_step_role_and_system_prompt_conflict() {
    let wf = parse(
        r#"
[workflow]
name = "role-conflict"

[roles.doctor]
system_prompt = "You are a doctor."

[[step]]
name = "examine"
prompt = "Examine the patient"
role = "doctor"
system_prompt = "You are a nurse."
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("both 'role' and 'system_prompt'")
            && e.to_string().contains("mutually exclusive")
    }));
}

#[test]
fn error_step_references_unknown_role() {
    let wf = parse(
        r#"
[workflow]
name = "bad-role-ref"

[[step]]
name = "examine"
prompt = "Examine the patient"
role = "nonexistent"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| { e.to_string().contains("unknown role 'nonexistent'") })
    );
}

#[test]
fn error_dynamic_role_references_unknown_variable() {
    let wf = parse(
        r#"
[workflow]
name = "bad-dynamic-role"

[roles.doctor]
system_prompt = "You are a doctor."

[[step]]
name = "examine"
prompt = "Examine the patient"
role = "${unknown_var}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string()
            .contains("role references unknown variable '${unknown_var}'")
    }));
}

#[test]
fn error_role_system_prompt_and_file_conflict() {
    let wf = parse(
        r#"
[workflow]
name = "role-file-conflict"

[roles.doctor]
system_prompt = "You are a doctor."
system_prompt_file = "prompts/doctor.md"

[[step]]
name = "examine"
prompt = "Examine the patient"
role = "doctor"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string()
            .contains("both 'system_prompt' and 'system_prompt_file'")
            && e.to_string().contains("mutually exclusive")
    }));
}

#[test]
fn error_role_system_prompt_references_unknown_variable() {
    let wf = parse(
        r#"
[workflow]
name = "bad-role-var"

[roles.doctor]
system_prompt = "You are a ${specialty} specialist."

[[step]]
name = "examine"
prompt = "Examine the patient"
role = "doctor"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string()
            .contains("role 'doctor' system_prompt references unknown variable '${specialty}'")
    }));
}

#[test]
fn valid_role_system_prompt_with_variable() {
    let wf = parse(
        r#"
[workflow]
name = "role-with-var"

[vars.specialty]
type = "string"
default = "cardiology"

[roles.doctor]
system_prompt = "You are a ${specialty} specialist."

[[step]]
name = "examine"
prompt = "Examine the patient"
role = "doctor"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn valid_role_with_system_prompt_file_only() {
    let wf = parse(
        r#"
[workflow]
name = "role-file-only"

[roles.doctor]
system_prompt_file = "prompts/doctor.md"

[[step]]
name = "examine"
prompt = "Examine the patient"
role = "doctor"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

// ── variable default_file validation ────────────────────────────────────────

#[test]
fn error_variable_default_and_default_file_conflict() {
    let wf = parse(
        r#"
[workflow]
name = "var-file-conflict"

[vars.instructions]
type = "string"
default = "inline default"
default_file = "defaults/instructions.txt"

[[step]]
name = "go"
prompt = "Follow ${instructions}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("both 'default' and 'default_file'")
            && e.to_string().contains("mutually exclusive")
    }));
}

#[test]
fn valid_variable_with_default_file() {
    let wf = parse(
        r#"
[workflow]
name = "var-file"

[vars.instructions]
type = "string"
default_file = "defaults/instructions.txt"

[[step]]
name = "go"
prompt = "Follow ${instructions}"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

// ── system_prompt variable references ─────────────────────────────────────

#[test]
fn error_unknown_variable_in_system_prompt() {
    let wf = parse(
        r#"
[workflow]
name = "bad-sys-var"

[[step]]
name = "a"
prompt = "Do something"
system_prompt = "You are a ${nonexistent_role}"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string()
            .contains("system_prompt references unknown variable '${nonexistent_role}'")
    }));
}

#[test]
fn valid_variable_in_system_prompt() {
    let wf = parse(
        r#"
[workflow]
name = "good-sys-var"

[vars.role]
type = "string"
default = "doctor"

[[step]]
name = "a"
prompt = "Do something"
system_prompt = "You are a ${role}"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn valid_dotted_var_ref_in_system_prompt() {
    let wf = parse(
        r#"
[workflow]
name = "dotted-sys"

[vars.config]
type = "json"

[[step]]
name = "a"
prompt = "Do something"
system_prompt = "Expertise level: ${config.level}"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

// ── Workflow-level provider/model validation ──────────────────────────────────

#[test]
fn valid_workflow_level_provider_model() {
    let wf = parse(
        r#"
[workflow]
name = "wf-defaults"
provider = "claude"
model = "sonnet"

[[step]]
name = "a"
prompt = "Do something"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn error_mcp_config_with_workflow_level_non_claude_provider() {
    let wf = parse(
        r#"
[workflow]
name = "wf-bad-mcp"
provider = "gemini"

[[step]]
name = "a"
prompt = "Do something"
mcp_config = "config.json"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("mcp_config")
            && e.to_string()
                .contains("only supported with the claude provider")
    }));
}

#[test]
fn valid_mcp_config_with_workflow_level_claude_provider() {
    let wf = parse(
        r#"
[workflow]
name = "wf-good-mcp"
provider = "claude"

[[step]]
name = "a"
prompt = "Do something"
mcp_config = "config.json"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}

#[test]
fn step_provider_overrides_workflow_provider_for_mcp_validation() {
    // Workflow says claude, but step overrides to gemini — should fail
    let wf = parse(
        r#"
[workflow]
name = "override-bad"
provider = "claude"

[[step]]
name = "a"
prompt = "Do something"
provider = "gemini"
mcp_config = "config.json"
"#,
    )
    .unwrap();

    let errors = validate(&wf).unwrap_err();
    assert!(errors.iter().any(|e| {
        e.to_string().contains("mcp_config")
            && e.to_string()
                .contains("only supported with the claude provider")
    }));
}

#[test]
fn step_provider_overrides_workflow_provider_for_mcp_positive() {
    // Workflow says gemini, but step overrides to claude — should pass
    let wf = parse(
        r#"
[workflow]
name = "override-good"
provider = "gemini"

[[step]]
name = "a"
prompt = "Do something"
provider = "claude"
mcp_config = "config.json"
"#,
    )
    .unwrap();

    assert!(validate(&wf).is_ok());
}
