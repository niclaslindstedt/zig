use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::memory::MemoryCollector;
use crate::resources::ResourceCollector;
use crate::storage::StorageManager;
use crate::workflow::model::{
    FailurePolicy, MemoryMode, Role, Step, StepCommand, Workflow, WorkflowMeta,
};

use super::*;

// ── helpers ──────────────────────────────────────────────────────────────────

fn empty_resources<'a>(workflow_dir: &'a Path) -> ResourceCollector<'a> {
    ResourceCollector {
        workflow_resources: &[],
        workflow_dir,
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_resources_dir: None,
        disabled: false,
    }
}

fn empty_memory() -> MemoryCollector {
    MemoryCollector {
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_memory_dir: None,
        workflow_mode: MemoryMode::All,
        local_enabled: true,
        disabled: false,
    }
}

fn step(name: &str, prompt: &str) -> Step {
    Step {
        name: name.to_string(),
        prompt: prompt.to_string(),
        ..Default::default()
    }
}

fn workflow_with(steps: Vec<Step>) -> Workflow {
    Workflow {
        workflow: WorkflowMeta {
            name: "test-wf".into(),
            ..Default::default()
        },
        roles: HashMap::new(),
        vars: HashMap::new(),
        steps,
        storage: Default::default(),
    }
}

#[allow(clippy::too_many_arguments)]
fn ctx_for<'a>(
    wf: &'a Workflow,
    workflow_path: &'a Path,
    workflow_dir: &'a Path,
    vars: &'a HashMap<String, String>,
    resources: &'a ResourceCollector<'a>,
    memory: &'a MemoryCollector,
    storage: &'a StorageManager,
    roles: &'a HashMap<String, Role>,
) -> DryRunContext<'a> {
    DryRunContext {
        workflow: wf,
        workflow_path,
        workflow_dir,
        vars,
        user_prompt: None,
        roles,
        resources,
        memory,
        storage,
        wf_provider: None,
        wf_model: None,
        disable_resources: false,
        disable_memory: false,
        disable_storage: false,
    }
}

// ── evaluate_with_resolvability ──────────────────────────────────────────────

#[test]
fn cond_none_when_no_expression() {
    let vars = HashMap::new();
    assert_eq!(evaluate_with_resolvability(None, &vars), CondOutcome::None);
}

#[test]
fn cond_true_when_all_vars_present() {
    let vars = HashMap::from([("score".into(), "5".into())]);
    assert_eq!(
        evaluate_with_resolvability(Some("score < 8"), &vars),
        CondOutcome::True
    );
}

#[test]
fn cond_false_when_all_vars_present() {
    let vars = HashMap::from([("score".into(), "9".into())]);
    assert_eq!(
        evaluate_with_resolvability(Some("score < 8"), &vars),
        CondOutcome::False
    );
}

#[test]
fn cond_unknown_when_var_missing() {
    let vars = HashMap::new();
    let outcome = evaluate_with_resolvability(Some("score < 8"), &vars);
    match outcome {
        CondOutcome::Unknown(missing) => assert_eq!(missing, vec!["score".to_string()]),
        other => panic!("expected Unknown, got {other:?}"),
    }
}

#[test]
fn cond_unknown_dedupes_missing_refs() {
    // "score < score" references "score" twice; missing list should have it once.
    let vars = HashMap::new();
    let outcome = evaluate_with_resolvability(Some("score < score"), &vars);
    match outcome {
        CondOutcome::Unknown(missing) => assert_eq!(missing, vec!["score".to_string()]),
        other => panic!("expected Unknown, got {other:?}"),
    }
}

// ── build_plan ───────────────────────────────────────────────────────────────

#[test]
fn build_plan_captures_every_step_per_tier() {
    let wf = workflow_with(vec![step("a", "Do a"), step("b", "Do b")]);
    let vars = HashMap::new();
    let wf_dir = PathBuf::from(".");
    let wf_path = PathBuf::from("./test.zwf");
    let resources = empty_resources(&wf_dir);
    let memory = empty_memory();
    let storage = StorageManager::empty();
    let roles = HashMap::new();
    let ctx = ctx_for(
        &wf, &wf_path, &wf_dir, &vars, &resources, &memory, &storage, &roles,
    );

    let tiers: Vec<Vec<&Step>> = vec![wf.steps.iter().collect()];
    let plan = build_plan(&ctx, &tiers).unwrap();

    assert_eq!(plan.workflow.name, "test-wf");
    assert_eq!(plan.workflow.step_count, 2);
    assert_eq!(plan.workflow.tier_count, 1);
    assert_eq!(plan.tiers.len(), 1);
    let names: Vec<&str> = plan.tiers[0]
        .steps
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, vec!["a", "b"]);
}

#[test]
fn build_plan_substitutes_vars_in_prompt() {
    let wf = workflow_with(vec![step("s", "Review ${target}")]);
    let vars = HashMap::from([("target".into(), "src/main.rs".into())]);
    let wf_dir = PathBuf::from(".");
    let wf_path = PathBuf::from("./test.zwf");
    let resources = empty_resources(&wf_dir);
    let memory = empty_memory();
    let storage = StorageManager::empty();
    let roles = HashMap::new();
    let ctx = ctx_for(
        &wf, &wf_path, &wf_dir, &vars, &resources, &memory, &storage, &roles,
    );

    let tiers: Vec<Vec<&Step>> = vec![wf.steps.iter().collect()];
    let plan = build_plan(&ctx, &tiers).unwrap();

    assert_eq!(plan.tiers[0].steps[0].prompt, "Review src/main.rs");
}

#[test]
fn build_plan_leaves_unresolvable_step_refs_as_placeholder() {
    // `${steps.earlier.result}` cannot be resolved in a dry run — it should
    // survive substitution as a literal placeholder so the reader knows
    // which prior-step output will feed in at runtime.
    let wf = workflow_with(vec![step("s", "Use ${steps.earlier.result} please")]);
    let vars = HashMap::new();
    let wf_dir = PathBuf::from(".");
    let wf_path = PathBuf::from("./test.zwf");
    let resources = empty_resources(&wf_dir);
    let memory = empty_memory();
    let storage = StorageManager::empty();
    let roles = HashMap::new();
    let ctx = ctx_for(
        &wf, &wf_path, &wf_dir, &vars, &resources, &memory, &storage, &roles,
    );

    let tiers: Vec<Vec<&Step>> = vec![wf.steps.iter().collect()];
    let plan = build_plan(&ctx, &tiers).unwrap();

    assert!(
        plan.tiers[0].steps[0]
            .prompt
            .contains("${steps.earlier.result}"),
        "prompt was: {}",
        plan.tiers[0].steps[0].prompt
    );
}

#[test]
fn build_plan_condition_tri_state() {
    let mut s_true = step("a", "do a");
    s_true.condition = Some("score < 8".into());
    let mut s_unknown = step("b", "do b");
    s_unknown.condition = Some("missing == \"go\"".into());
    let s_none = step("c", "do c");

    let wf = workflow_with(vec![s_true, s_unknown, s_none]);
    let vars = HashMap::from([("score".into(), "5".into())]);
    let wf_dir = PathBuf::from(".");
    let wf_path = PathBuf::from("./test.zwf");
    let resources = empty_resources(&wf_dir);
    let memory = empty_memory();
    let storage = StorageManager::empty();
    let roles = HashMap::new();
    let ctx = ctx_for(
        &wf, &wf_path, &wf_dir, &vars, &resources, &memory, &storage, &roles,
    );

    let tiers: Vec<Vec<&Step>> = vec![wf.steps.iter().collect()];
    let plan = build_plan(&ctx, &tiers).unwrap();
    let outcomes: Vec<&str> = plan.tiers[0]
        .steps
        .iter()
        .map(|s| s.condition.outcome.as_str())
        .collect();
    assert_eq!(outcomes, vec!["true", "unknown", "none"]);
    assert_eq!(plan.tiers[0].steps[1].condition.missing, vec!["missing"]);
}

#[test]
fn build_plan_disable_flags_mark_blocks_omitted() {
    let wf = workflow_with(vec![step("a", "do a")]);
    let vars = HashMap::new();
    let wf_dir = PathBuf::from(".");
    let wf_path = PathBuf::from("./test.zwf");
    let resources = empty_resources(&wf_dir);
    let memory = empty_memory();
    let storage = StorageManager::empty();
    let roles = HashMap::new();
    let mut ctx = ctx_for(
        &wf, &wf_path, &wf_dir, &vars, &resources, &memory, &storage, &roles,
    );
    ctx.disable_resources = true;
    ctx.disable_memory = true;
    ctx.disable_storage = true;

    let tiers: Vec<Vec<&Step>> = vec![wf.steps.iter().collect()];
    let plan = build_plan(&ctx, &tiers).unwrap();
    let blocks = &plan.tiers[0].steps[0].blocks;
    assert_eq!(
        blocks.resources.omitted_reason.as_deref(),
        Some("no_resources")
    );
    assert_eq!(blocks.memory.omitted_reason.as_deref(), Some("no_memory"));
    assert_eq!(blocks.storage.omitted_reason.as_deref(), Some("no_storage"));
    assert!(plan.disabled.resources);
    assert!(plan.disabled.memory);
    assert!(plan.disabled.storage);
}

#[test]
fn build_plan_saves_sorted_by_name() {
    let mut s = step("a", "do a");
    s.saves.insert("zebra".into(), "$.z".into());
    s.saves.insert("alpha".into(), "$.a".into());
    s.saves.insert("middle".into(), "$.m".into());

    let wf = workflow_with(vec![s]);
    let vars = HashMap::new();
    let wf_dir = PathBuf::from(".");
    let wf_path = PathBuf::from("./test.zwf");
    let resources = empty_resources(&wf_dir);
    let memory = empty_memory();
    let storage = StorageManager::empty();
    let roles = HashMap::new();
    let ctx = ctx_for(
        &wf, &wf_path, &wf_dir, &vars, &resources, &memory, &storage, &roles,
    );

    let tiers: Vec<Vec<&Step>> = vec![wf.steps.iter().collect()];
    let plan = build_plan(&ctx, &tiers).unwrap();
    let save_names: Vec<&str> = plan.tiers[0].steps[0]
        .saves
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(save_names, vec!["alpha", "middle", "zebra"]);
}

#[test]
fn build_plan_command_and_failure_labels() {
    let mut s_review = step("r", "review");
    s_review.command = Some(StepCommand::Review);
    s_review.on_failure = Some(FailurePolicy::Continue);
    let s_default = step("d", "do d");

    let wf = workflow_with(vec![s_review, s_default]);
    let vars = HashMap::new();
    let wf_dir = PathBuf::from(".");
    let wf_path = PathBuf::from("./test.zwf");
    let resources = empty_resources(&wf_dir);
    let memory = empty_memory();
    let storage = StorageManager::empty();
    let roles = HashMap::new();
    let ctx = ctx_for(
        &wf, &wf_path, &wf_dir, &vars, &resources, &memory, &storage, &roles,
    );

    let tiers: Vec<Vec<&Step>> = vec![wf.steps.iter().collect()];
    let plan = build_plan(&ctx, &tiers).unwrap();
    assert_eq!(plan.tiers[0].steps[0].command, "review");
    assert_eq!(plan.tiers[0].steps[0].failure, "continue");
    assert_eq!(plan.tiers[0].steps[1].command, "run");
    assert_eq!(plan.tiers[0].steps[1].failure, "fail");
}

#[test]
fn build_plan_zag_args_include_prompt_and_name() {
    let wf = workflow_with(vec![step("a", "Do the thing")]);
    let vars = HashMap::new();
    let wf_dir = PathBuf::from(".");
    let wf_path = PathBuf::from("./test.zwf");
    let resources = empty_resources(&wf_dir);
    let memory = empty_memory();
    let storage = StorageManager::empty();
    let roles = HashMap::new();
    let ctx = ctx_for(
        &wf, &wf_path, &wf_dir, &vars, &resources, &memory, &storage, &roles,
    );

    let tiers: Vec<Vec<&Step>> = vec![wf.steps.iter().collect()];
    let plan = build_plan(&ctx, &tiers).unwrap();
    let cfg = &plan.tiers[0].steps[0].agent_config;
    assert_eq!(cfg.command, "run");
    assert_eq!(cfg.prompt, "Do the thing");
    // session name is derived from workflow name + step name
    assert_eq!(cfg.session_name, "zig-test-wf-a");
}

// ── JSON serialization ───────────────────────────────────────────────────────

#[test]
fn json_output_has_expected_top_level_keys() {
    let wf = workflow_with(vec![step("a", "do a")]);
    let vars = HashMap::new();
    let wf_dir = PathBuf::from(".");
    let wf_path = PathBuf::from("./test.zwf");
    let resources = empty_resources(&wf_dir);
    let memory = empty_memory();
    let storage = StorageManager::empty();
    let roles = HashMap::new();
    let ctx = ctx_for(
        &wf, &wf_path, &wf_dir, &vars, &resources, &memory, &storage, &roles,
    );
    let tiers: Vec<Vec<&Step>> = vec![wf.steps.iter().collect()];
    let plan = build_plan(&ctx, &tiers).unwrap();

    let json = serde_json::to_value(&plan).unwrap();
    assert!(json.get("workflow").is_some());
    assert!(json.get("disabled").is_some());
    assert!(json.get("vars").is_some());
    assert!(json.get("tiers").is_some());

    let step_zero = &json["tiers"][0]["steps"][0];
    assert_eq!(step_zero["name"], "a");
    assert_eq!(step_zero["command"], "run");
    assert!(step_zero["agent_config"].is_object());
    assert_eq!(step_zero["agent_config"]["command"], "run");
    assert_eq!(step_zero["agent_config"]["session_name"], "zig-test-wf-a");
    assert_eq!(step_zero["condition"]["outcome"], "none");
    // `missing` should be skipped when empty (per serde skip_serializing_if).
    assert!(step_zero["condition"].get("missing").is_none());
}

#[test]
fn json_condition_outcomes_serialize_as_strings() {
    let mut s_t = step("t", "do t");
    s_t.condition = Some("v < 10".into());
    let mut s_f = step("f", "do f");
    s_f.condition = Some("v > 10".into());
    let mut s_u = step("u", "do u");
    s_u.condition = Some("missing == \"go\"".into());
    let s_n = step("n", "do n");

    let wf = workflow_with(vec![s_t, s_f, s_u, s_n]);
    let vars = HashMap::from([("v".into(), "5".into())]);
    let wf_dir = PathBuf::from(".");
    let wf_path = PathBuf::from("./test.zwf");
    let resources = empty_resources(&wf_dir);
    let memory = empty_memory();
    let storage = StorageManager::empty();
    let roles = HashMap::new();
    let ctx = ctx_for(
        &wf, &wf_path, &wf_dir, &vars, &resources, &memory, &storage, &roles,
    );
    let tiers: Vec<Vec<&Step>> = vec![wf.steps.iter().collect()];
    let plan = build_plan(&ctx, &tiers).unwrap();
    let json = serde_json::to_value(&plan).unwrap();

    let outcomes: Vec<&str> = json["tiers"][0]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["condition"]["outcome"].as_str().unwrap())
        .collect();
    assert_eq!(outcomes, vec!["true", "false", "unknown", "none"]);
}
