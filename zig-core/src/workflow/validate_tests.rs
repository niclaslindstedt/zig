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
