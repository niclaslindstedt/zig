use std::collections::HashMap;

use crate::workflow::model::{
    Step, TriggerOutput, TriggerSpec, VarType, Variable, Workflow, WorkflowMeta,
};
use crate::workflow::validate;

use super::*;

// ── helpers ──────────────────────────────────────────────────────────────────

fn event_var() -> Variable {
    Variable {
        var_type: VarType::Json,
        from: Some("event".into()),
        ..Default::default()
    }
}

fn step_named(name: &str) -> Step {
    Step {
        name: name.to_string(),
        prompt: "noop".into(),
        condition: Some("false".into()),
        ..Default::default()
    }
}

fn workflow_with_trigger(trigger: TriggerSpec, vars: HashMap<String, Variable>) -> Workflow {
    Workflow {
        workflow: WorkflowMeta {
            name: "trigger-test".into(),
            ..Default::default()
        },
        roles: HashMap::new(),
        vars,
        steps: vec![step_named("a")],
        trigger: Some(trigger),
        storage: Default::default(),
    }
}

fn basic_trigger() -> TriggerSpec {
    TriggerSpec {
        source: "stdin".into(),
        format: Some("jsonl".into()),
        bind: "event".into(),
        output: Some(TriggerOutput {
            kind: "stdout-jsonl".into(),
            mode: Some("all-steps".into()),
            include_vars: false,
        }),
    }
}

// ── validator ────────────────────────────────────────────────────────────────

#[test]
fn parse_valid_trigger() {
    let toml_src = r#"
        [workflow]
        name = "echo-bot"

        [trigger]
        source = "stdin"
        format = "jsonl"
        bind = "event"

        [trigger.output]
        kind = "stdout-jsonl"
        mode = "all-steps"

        [vars.event]
        type = "json"
        from = "event"

        [[step]]
        name = "echo"
        prompt = "Reply to: ${event}"
        condition = "false"
    "#;
    let workflow: Workflow = toml::from_str(toml_src).expect("workflow parses");
    validate::validate(&workflow).expect("workflow validates");
    let trigger = workflow.trigger.as_ref().expect("trigger present");
    assert_eq!(trigger.source, "stdin");
    assert_eq!(trigger.bind, "event");
}

#[test]
fn reject_event_var_without_trigger() {
    let mut vars = HashMap::new();
    vars.insert("event".into(), event_var());
    let workflow = Workflow {
        workflow: WorkflowMeta {
            name: "no-trigger".into(),
            ..Default::default()
        },
        roles: HashMap::new(),
        vars,
        steps: vec![step_named("a")],
        trigger: None,
        storage: Default::default(),
    };
    let errs = validate::validate(&workflow).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("from = \"event\""))
    );
}

#[test]
fn reject_prompt_and_event_both_set() {
    let mut vars = HashMap::new();
    vars.insert("event".into(), event_var());
    vars.insert(
        "ask".into(),
        Variable {
            var_type: VarType::String,
            from: Some("prompt".into()),
            ..Default::default()
        },
    );
    let workflow = workflow_with_trigger(basic_trigger(), vars);
    let errs = validate::validate(&workflow).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("mutually exclusive"))
    );
}

#[test]
fn reject_event_var_type_not_json() {
    let mut vars = HashMap::new();
    vars.insert(
        "event".into(),
        Variable {
            var_type: VarType::String,
            from: Some("event".into()),
            ..Default::default()
        },
    );
    let workflow = workflow_with_trigger(basic_trigger(), vars);
    let errs = validate::validate(&workflow).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("type = \"json\""))
    );
}

#[test]
fn reject_bind_to_unknown_variable() {
    let mut vars = HashMap::new();
    vars.insert("event".into(), event_var());
    let mut trigger = basic_trigger();
    trigger.bind = "missing".into();
    let workflow = workflow_with_trigger(trigger, vars);
    let errs = validate::validate(&workflow).unwrap_err();
    assert!(errs.iter().any(|e| {
        e.to_string()
            .contains("does not match any declared variable")
    }));
}

#[test]
fn reject_unsupported_source() {
    let mut trigger = basic_trigger();
    trigger.source = "http".into();
    let mut vars = HashMap::new();
    vars.insert("event".into(), event_var());
    let workflow = workflow_with_trigger(trigger, vars);
    let errs = validate::validate(&workflow).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("trigger.source 'http'"))
    );
}

#[test]
fn reject_unsupported_format() {
    let mut trigger = basic_trigger();
    trigger.format = Some("yaml".into());
    let mut vars = HashMap::new();
    vars.insert("event".into(), event_var());
    let workflow = workflow_with_trigger(trigger, vars);
    let errs = validate::validate(&workflow).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("trigger.format 'yaml'"))
    );
}

#[test]
fn reject_unsupported_output_kind() {
    let mut trigger = basic_trigger();
    trigger.output = Some(TriggerOutput {
        kind: "file".into(),
        mode: None,
        include_vars: false,
    });
    let mut vars = HashMap::new();
    vars.insert("event".into(), event_var());
    let workflow = workflow_with_trigger(trigger, vars);
    let errs = validate::validate(&workflow).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("trigger.output.kind 'file'"))
    );
}

#[test]
fn reject_opt_in_mode_with_no_emit_step() {
    let mut trigger = basic_trigger();
    trigger.output = Some(TriggerOutput {
        kind: "stdout-jsonl".into(),
        mode: Some("opt-in".into()),
        include_vars: false,
    });
    let mut vars = HashMap::new();
    vars.insert("event".into(), event_var());
    let workflow = workflow_with_trigger(trigger, vars);
    let errs = validate::validate(&workflow).unwrap_err();
    assert!(errs.iter().any(|e| {
        e.to_string()
            .contains("requires at least one step with emit = true")
    }));
}

#[test]
fn opt_in_mode_passes_when_step_marks_emit() {
    let mut trigger = basic_trigger();
    trigger.output = Some(TriggerOutput {
        kind: "stdout-jsonl".into(),
        mode: Some("opt-in".into()),
        include_vars: false,
    });
    let mut vars = HashMap::new();
    vars.insert("event".into(), event_var());
    let mut workflow = workflow_with_trigger(trigger, vars);
    workflow.steps[0].emit = Some(true);
    validate::validate(&workflow).expect("validates when at least one step has emit = true");
}

#[test]
fn reject_multiple_event_bound_vars() {
    let mut vars = HashMap::new();
    vars.insert("a".into(), event_var());
    vars.insert("b".into(), event_var());
    let mut trigger = basic_trigger();
    trigger.bind = "a".into();
    let workflow = workflow_with_trigger(trigger, vars);
    let errs = validate::validate(&workflow).unwrap_err();
    assert!(errs.iter().any(|e| {
        e.to_string()
            .contains("multiple variables have from = \"event\"")
    }));
}

// ── prompt + trigger mutually exclusive at run_workflow ─────────────────────

#[tokio::test]
async fn reject_prompt_flag_in_event_mode() {
    // Write a minimal triggered workflow to a temp file and call run_workflow
    // with a non-None user_prompt to assert the rejection.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trig.zwf");
    std::fs::write(
        &path,
        r#"
[workflow]
name = "test-trigger"

[trigger]
source = "stdin"
format = "jsonl"
bind = "event"

[trigger.output]
kind = "stdout-jsonl"
mode = "all-steps"

[vars.event]
type = "json"
from = "event"

[[step]]
name = "noop"
prompt = "${event}"
condition = "false"
"#,
    )
    .unwrap();

    let err = run_workflow(
        path.to_str().unwrap(),
        Some("hi"),
        false,
        false,
        false,
        false,
        crate::dry_run::DryRunFormat::Text,
    )
    .await
    .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("--prompt is not supported"),
        "unexpected error: {msg}"
    );
}

// ── emit-record serialization and flush modes ───────────────────────────────

fn record(step: &str, status: &str) -> EmitRecord {
    EmitRecord {
        event_id: "evt-000001".into(),
        event_seq: 1,
        step: step.to_string(),
        status: status.to_string(),
        output: if status == "ok" {
            Some("payload".into())
        } else {
            None
        },
        error: None,
        reason: None,
        ts: "2026-05-12T00:00:00.000Z".into(),
        vars_snapshot: None,
    }
}

#[test]
fn flush_all_steps_writes_one_line_per_record() {
    let buf: Vec<EmitRecord> = vec![record("a", "ok"), record("b", "skipped")];
    let mut out: Vec<u8> = Vec::new();
    flush_emits(&mut out, &buf, EmitMode::AllSteps, &[]).unwrap();
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"step\":\"a\""));
    assert!(lines[1].contains("\"step\":\"b\""));
}

#[test]
fn flush_final_writes_only_last_ok_record() {
    let buf: Vec<EmitRecord> = vec![record("a", "ok"), record("b", "ok"), record("c", "skipped")];
    let mut out: Vec<u8> = Vec::new();
    flush_emits(&mut out, &buf, EmitMode::Final, &[]).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert_eq!(text.lines().count(), 1);
    assert!(text.contains("\"step\":\"b\""));
}

#[test]
fn flush_opt_in_writes_only_opt_in_steps_and_terminal_failure() {
    let buf: Vec<EmitRecord> = vec![
        record("a", "ok"),
        record("b", "ok"),
        EmitRecord {
            event_id: "evt-000001".into(),
            event_seq: 1,
            step: "__workflow__".into(),
            status: "failed".into(),
            output: None,
            error: Some("boom".into()),
            reason: None,
            ts: "2026-05-12T00:00:00.000Z".into(),
            vars_snapshot: None,
        },
    ];
    let steps = vec![
        Step {
            name: "a".into(),
            prompt: "".into(),
            emit: Some(false),
            ..Default::default()
        },
        Step {
            name: "b".into(),
            prompt: "".into(),
            emit: Some(true),
            ..Default::default()
        },
    ];
    let mut out: Vec<u8> = Vec::new();
    flush_emits(&mut out, &buf, EmitMode::OptIn, &steps).unwrap();
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "expected b + __workflow__, got {text}");
    assert!(lines[0].contains("\"step\":\"b\""));
    assert!(lines[1].contains("__workflow__"));
}

#[test]
fn emit_timestamp_has_iso8601_shape() {
    let ts = emit_timestamp();
    // Expected: "YYYY-MM-DDTHH:MM:SS.MMMZ"
    assert_eq!(ts.len(), 24, "ts = {ts}");
    assert_eq!(ts.chars().nth(4), Some('-'));
    assert_eq!(ts.chars().nth(7), Some('-'));
    assert_eq!(ts.chars().nth(10), Some('T'));
    assert_eq!(ts.chars().nth(13), Some(':'));
    assert_eq!(ts.chars().nth(16), Some(':'));
    assert_eq!(ts.chars().nth(19), Some('.'));
    assert_eq!(ts.chars().last(), Some('Z'));
}

#[test]
fn emit_mode_parse_defaults_to_all_steps() {
    assert_eq!(EmitMode::parse(None), EmitMode::AllSteps);
    assert_eq!(EmitMode::parse(Some("all-steps")), EmitMode::AllSteps);
    assert_eq!(EmitMode::parse(Some("final")), EmitMode::Final);
    assert_eq!(EmitMode::parse(Some("opt-in")), EmitMode::OptIn);
    // Unknown → falls back to AllSteps (validator rejects bad values upstream).
    assert_eq!(EmitMode::parse(Some("weird")), EmitMode::AllSteps);
}
