//! `zig run --dry-run` — preview what a workflow would do without invoking zag.
//!
//! This module walks the tiers produced by `topological_sort` and renders,
//! for each step, everything a real run would compute up to the moment of
//! `zag` spawn: the resolved prompt, system prompt (including the
//! `<resources>` / `<memory>` / `<storage>` blocks), the condition outcome,
//! and the exact `zag` command-line arguments that *would* be invoked.
//!
//! No side effects: no session log is written, no storage directories are
//! created, no `zag` process is launched.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Serialize;

use crate::error::ZigError;
use crate::memory::MemoryCollector;
use crate::resources::ResourceCollector;
use crate::run::{
    build_zag_args, evaluate_condition, render_step_prompt, resolve_role_system_prompt,
};
use crate::storage::StorageManager;
use crate::workflow::model::{FailurePolicy, Role, Step, StepCommand, Workflow};
use crate::workflow::validate::extract_condition_vars;

/// Output format for a dry-run plan.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DryRunFormat {
    /// Human-readable, grouped per tier and step. Default.
    #[default]
    Text,
    /// Pretty-printed JSON — suitable for piping into `jq` or for tooling.
    /// The field names are part of zig's public contract; see
    /// `docs/dry-run.md` for the full schema.
    Json,
}

/// Inputs `print_plan` needs to build the plan. Mirrors the state
/// `execute()` has assembled just before it would open a session log.
pub struct DryRunContext<'a> {
    pub workflow: &'a Workflow,
    pub workflow_path: &'a Path,
    pub workflow_dir: &'a Path,
    pub vars: &'a HashMap<String, String>,
    pub user_prompt: Option<&'a str>,
    pub roles: &'a HashMap<String, Role>,
    pub resources: &'a ResourceCollector<'a>,
    pub memory: &'a MemoryCollector,
    pub storage: &'a StorageManager,
    pub wf_provider: Option<&'a str>,
    pub wf_model: Option<&'a str>,
    pub disable_resources: bool,
    pub disable_memory: bool,
    pub disable_storage: bool,
}

/// Build the plan and print it to stdout in the requested format.
pub fn print_plan(
    ctx: &DryRunContext<'_>,
    tiers: &[Vec<&Step>],
    format: DryRunFormat,
) -> Result<(), ZigError> {
    let plan = build_plan(ctx, tiers)?;
    match format {
        DryRunFormat::Text => print_text(&plan),
        DryRunFormat::Json => {
            let json = serde_json::to_string_pretty(&plan).map_err(|e| {
                ZigError::Execution(format!("failed to serialize dry-run plan as JSON: {e}"))
            })?;
            println!("{json}");
        }
    }
    Ok(())
}

// ── plan data ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct DryRunPlan {
    pub workflow: DryRunWorkflow,
    pub disabled: DryRunDisabled,
    pub vars: HashMap<String, String>,
    pub tiers: Vec<DryRunTier>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunWorkflow {
    pub name: String,
    pub path: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub step_count: usize,
    pub tier_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunDisabled {
    pub resources: bool,
    pub memory: bool,
    pub storage: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunTier {
    pub index: usize,
    pub steps: Vec<DryRunStep>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunStep {
    pub name: String,
    pub command: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub failure: String,
    pub depends_on: Vec<String>,
    pub condition: DryRunCondition,
    pub saves: Vec<DryRunSave>,
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub blocks: DryRunBlocks,
    pub zag_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunCondition {
    pub expr: Option<String>,
    /// `"true" | "false" | "unknown" | "none"`.
    pub outcome: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunSave {
    pub name: String,
    pub selector: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunBlocks {
    pub resources: DryRunBlock,
    pub memory: DryRunBlock,
    pub storage: DryRunBlock,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunBlock {
    /// `"no_resources" | "no_memory" | "no_storage"` when the block is
    /// suppressed by a `--no-*` flag; otherwise `None`.
    pub omitted_reason: Option<String>,
    /// Rendered block text when present. `None` when the block is disabled
    /// or nothing would be rendered for this step.
    pub content: Option<String>,
}

// ── condition tri-state ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CondOutcome {
    True,
    False,
    Unknown(Vec<String>),
    None,
}

/// Evaluate a condition with explicit resolvability tracking.
///
/// Returns [`CondOutcome::Unknown`] if any referenced variable is absent
/// from `vars` (and is not a numeric/string literal). Otherwise delegates
/// to [`crate::run::evaluate_condition`].
pub(crate) fn evaluate_with_resolvability(
    expr: Option<&str>,
    vars: &HashMap<String, String>,
) -> CondOutcome {
    let Some(cond) = expr else {
        return CondOutcome::None;
    };
    let refs = extract_condition_vars(cond);
    // Deduplicate while preserving order of first appearance.
    let mut seen: HashSet<String> = HashSet::new();
    let missing: Vec<String> = refs
        .into_iter()
        .filter(|name| !vars.contains_key(name))
        .filter(|name| seen.insert(name.clone()))
        .collect();
    if !missing.is_empty() {
        return CondOutcome::Unknown(missing);
    }
    match evaluate_condition(cond, vars) {
        Ok(true) => CondOutcome::True,
        Ok(false) => CondOutcome::False,
        // Evaluation errors in dry-run are reported as "unknown" with the
        // expression stashed in `missing` for visibility.
        Err(_) => CondOutcome::Unknown(Vec::new()),
    }
}

// ── plan builder ─────────────────────────────────────────────────────────────

fn build_plan(ctx: &DryRunContext<'_>, tiers: &[Vec<&Step>]) -> Result<DryRunPlan, ZigError> {
    // An empty dependency-output map — in a dry run, prior-step outputs
    // don't exist, so `inject_context = true` becomes a no-op and
    // `${steps.X.Y}` references in the prompt stay as literal placeholders
    // (substitute_vars leaves unknown `${...}` unchanged).
    let empty_outputs: HashMap<String, String> = HashMap::new();

    let mut plan_tiers = Vec::with_capacity(tiers.len());
    for (tier_index, tier) in tiers.iter().enumerate() {
        let mut steps = Vec::with_capacity(tier.len());
        for step in tier {
            steps.push(build_step(ctx, step, &empty_outputs)?);
        }
        plan_tiers.push(DryRunTier {
            index: tier_index,
            steps,
        });
    }

    Ok(DryRunPlan {
        workflow: DryRunWorkflow {
            name: ctx.workflow.workflow.name.clone(),
            path: ctx.workflow_path.display().to_string(),
            provider: ctx.wf_provider.map(String::from),
            model: ctx.wf_model.map(String::from),
            step_count: ctx.workflow.steps.len(),
            tier_count: tiers.len(),
        },
        disabled: DryRunDisabled {
            resources: ctx.disable_resources,
            memory: ctx.disable_memory,
            storage: ctx.disable_storage,
        },
        vars: ctx.vars.clone(),
        tiers: plan_tiers,
    })
}

fn build_step(
    ctx: &DryRunContext<'_>,
    step: &Step,
    empty_outputs: &HashMap<String, String>,
) -> Result<DryRunStep, ZigError> {
    let prompt = render_step_prompt(step, ctx.vars, ctx.user_prompt, empty_outputs);

    let rendered_sp = resolve_role_system_prompt(
        step,
        ctx.roles,
        ctx.resources,
        ctx.memory,
        ctx.storage,
        ctx.vars,
        ctx.workflow_dir,
        &ctx.workflow.workflow.name,
    )?;

    let storage_dirs = ctx.storage.add_dirs_for_step(step.storage.as_deref());

    let zag_args = build_zag_args(
        step,
        &prompt,
        &ctx.workflow.workflow.name,
        None,
        rendered_sp.as_deref(),
        ctx.wf_provider,
        ctx.wf_model,
        &storage_dirs,
    );

    let condition = condition_to_plan(step.condition.as_deref(), ctx.vars);

    let mut saves: Vec<DryRunSave> = step
        .saves
        .iter()
        .map(|(name, selector)| DryRunSave {
            name: name.clone(),
            selector: selector.clone(),
        })
        .collect();
    saves.sort_by(|a, b| a.name.cmp(&b.name));

    let blocks = build_blocks(ctx, step)?;

    Ok(DryRunStep {
        name: step.name.clone(),
        command: zag_command_label(&step.command).to_string(),
        provider: step.provider.clone(),
        model: step.model.clone(),
        failure: failure_label(step.on_failure.as_ref()).to_string(),
        depends_on: step.depends_on.clone(),
        condition,
        saves,
        prompt,
        system_prompt: rendered_sp,
        blocks,
        zag_args,
    })
}

fn condition_to_plan(expr: Option<&str>, vars: &HashMap<String, String>) -> DryRunCondition {
    let outcome = evaluate_with_resolvability(expr, vars);
    let (label, missing) = match outcome {
        CondOutcome::None => ("none", Vec::new()),
        CondOutcome::True => ("true", Vec::new()),
        CondOutcome::False => ("false", Vec::new()),
        CondOutcome::Unknown(m) => ("unknown", m),
    };
    DryRunCondition {
        expr: expr.map(String::from),
        outcome: label.to_string(),
        missing,
    }
}

fn build_blocks(ctx: &DryRunContext<'_>, step: &Step) -> Result<DryRunBlocks, ZigError> {
    // Resources
    let resources = if ctx.disable_resources {
        DryRunBlock {
            omitted_reason: Some("no_resources".into()),
            content: None,
        }
    } else {
        let set = ctx.resources.collect_for_step(&step.resources)?;
        let rendered = crate::resources::render_system_block(&set);
        DryRunBlock {
            omitted_reason: None,
            content: if rendered.is_empty() {
                None
            } else {
                Some(rendered.trim_end().to_string())
            },
        }
    };

    // Memory
    let memory = if ctx.disable_memory {
        DryRunBlock {
            omitted_reason: Some("no_memory".into()),
            content: None,
        }
    } else {
        let entries = ctx.memory.collect_for_step(step.memory.as_deref())?;
        let rendered = crate::memory::render_memory_block(
            &entries,
            &ctx.workflow.workflow.name,
            Some(&step.name),
        );
        DryRunBlock {
            omitted_reason: None,
            content: if rendered.is_empty() {
                None
            } else {
                Some(rendered.trim_end().to_string())
            },
        }
    };

    // Storage
    let storage = if ctx.disable_storage {
        DryRunBlock {
            omitted_reason: Some("no_storage".into()),
            content: None,
        }
    } else {
        let rendered = ctx.storage.render_block(step.storage.as_deref())?;
        DryRunBlock {
            omitted_reason: None,
            content: rendered,
        }
    };

    Ok(DryRunBlocks {
        resources,
        memory,
        storage,
    })
}

fn zag_command_label(cmd: &Option<StepCommand>) -> &'static str {
    match cmd {
        None => "run",
        Some(StepCommand::Review) => "review",
        Some(StepCommand::Plan) => "plan",
        Some(StepCommand::Pipe) => "pipe",
        Some(StepCommand::Collect) => "collect",
        Some(StepCommand::Summary) => "summary",
    }
}

fn failure_label(policy: Option<&FailurePolicy>) -> &'static str {
    match policy.unwrap_or(&FailurePolicy::Fail) {
        FailurePolicy::Fail => "fail",
        FailurePolicy::Continue => "continue",
        FailurePolicy::Retry => "retry",
    }
}

// ── text renderer ────────────────────────────────────────────────────────────

fn print_text(plan: &DryRunPlan) {
    let wf = &plan.workflow;
    println!(
        "workflow: {name}  ({steps} step{step_plural} in {tiers} tier{tier_plural})",
        name = wf.name,
        steps = wf.step_count,
        step_plural = if wf.step_count == 1 { "" } else { "s" },
        tiers = wf.tier_count,
        tier_plural = if wf.tier_count == 1 { "" } else { "s" },
    );
    println!("path:     {}", wf.path);
    if let Some(ref provider) = wf.provider {
        println!("provider: {provider}");
    }
    if let Some(ref model) = wf.model {
        println!("model:    {model}");
    }
    if plan.disabled.resources || plan.disabled.memory || plan.disabled.storage {
        let mut disabled = Vec::new();
        if plan.disabled.resources {
            disabled.push("resources");
        }
        if plan.disabled.memory {
            disabled.push("memory");
        }
        if plan.disabled.storage {
            disabled.push("storage");
        }
        println!("disabled: {}", disabled.join(", "));
    }
    if !plan.vars.is_empty() {
        let mut names: Vec<&String> = plan.vars.keys().collect();
        names.sort();
        println!("vars:");
        for name in names {
            let value = &plan.vars[name];
            let preview = preview(value, 80);
            println!("  {name} = {preview}");
        }
    }
    println!();

    for tier in &plan.tiers {
        println!("=== Tier {} ===", tier.index);
        for (i, step) in tier.steps.iter().enumerate() {
            print_step_text(i + 1, step);
        }
    }
}

fn print_step_text(position: usize, step: &DryRunStep) {
    println!(
        "[{pos}] step: {name}   command: {cmd}{provider}{model}",
        pos = position,
        name = step.name,
        cmd = step.command,
        provider = step
            .provider
            .as_ref()
            .map(|p| format!("   provider: {p}"))
            .unwrap_or_default(),
        model = step
            .model
            .as_ref()
            .map(|m| format!("   model: {m}"))
            .unwrap_or_default(),
    );
    println!("    failure: {}", step.failure);
    if !step.depends_on.is_empty() {
        println!("    depends_on: {}", step.depends_on.join(", "));
    }

    match step.condition.outcome.as_str() {
        "none" => {
            println!("    condition: <none>");
        }
        "unknown" => {
            let expr = step.condition.expr.as_deref().unwrap_or("");
            let missing = if step.condition.missing.is_empty() {
                String::new()
            } else {
                format!(" (missing: {})", step.condition.missing.join(", "))
            };
            println!("    condition: \"{expr}\" => unknown{missing}");
        }
        outcome => {
            let expr = step.condition.expr.as_deref().unwrap_or("");
            println!("    condition: \"{expr}\" => {outcome}");
        }
    }

    if !step.saves.is_empty() {
        let joined = step
            .saves
            .iter()
            .map(|s| format!("{}={}", s.name, s.selector))
            .collect::<Vec<_>>()
            .join(", ");
        println!("    saves: {joined}");
    }

    println!("    prompt:");
    print_indented(&step.prompt, "      ");

    if let Some(ref sp) = step.system_prompt {
        println!("    system_prompt:");
        print_indented(sp, "      ");
    }

    print_block_text("resources", &step.blocks.resources);
    print_block_text("memory", &step.blocks.memory);
    print_block_text("storage", &step.blocks.storage);

    let quoted: Vec<String> = step.zag_args.iter().map(|a| quote_arg(a)).collect();
    println!("    zag args: [{}]", quoted.join(", "));
    println!();
}

fn print_block_text(label: &str, block: &DryRunBlock) {
    if let Some(ref reason) = block.omitted_reason {
        println!("    {label}: (omitted — --{})", reason.replace('_', "-"));
        return;
    }
    match &block.content {
        None => println!("    {label}: (none)"),
        Some(content) => {
            println!("    {label}:");
            print_indented(content, "      ");
        }
    }
}

fn print_indented(content: &str, prefix: &str) {
    if content.is_empty() {
        println!("{prefix}");
        return;
    }
    for line in content.lines() {
        println!("{prefix}{line}");
    }
}

fn preview(value: &str, max: usize) -> String {
    let collapsed: String = value
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    if collapsed.chars().count() <= max {
        collapsed
    } else {
        let truncated: String = collapsed.chars().take(max).collect();
        format!("{truncated}…")
    }
}

fn quote_arg(arg: &str) -> String {
    if arg.is_empty() || arg.chars().any(|c| c.is_whitespace() || c == '"') {
        let escaped = arg.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        format!("\"{arg}\"")
    }
}

#[cfg(test)]
#[path = "dry_run_tests.rs"]
mod tests;
