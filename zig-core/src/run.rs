use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::task::JoinSet;
use zag_agent::builder::AgentBuilder;
use zag_agent::{plan as agent_plan, review as agent_review};
use zag_orch::collect as orch_collect;
use zag_orch::summary as orch_summary;

use crate::config::ZigConfig;
use crate::dry_run::{DryRunContext, DryRunFormat};
use crate::error::ZigError;
use crate::memory::{MemoryCollector, render_memory_block};
use crate::paths::expand_path;
use crate::resources::{ResourceCollector, render_system_block};
use crate::session::{OutputStream, SessionCoordinator, SessionStatus, SessionWriter};
use crate::storage::{FilesystemBackend, StorageManager};
use crate::workflow::model::{FailurePolicy, MemoryMode, Role, Step, StepCommand, Workflow};
use crate::workflow::{parser, validate};

/// Maximum number of loop iterations to prevent infinite loops from `next` fields.
const MAX_LOOP_ITERATIONS: usize = 100;

/// Execute a workflow file (`.zwf` or `.zwfz`).
///
/// Parses the workflow, validates it, resolves the step DAG, and executes
/// each step via the embedded [`zag_agent::builder::AgentBuilder`] (and
/// [`zag_orch`] for pipe/collect/summary commands). The optional
/// `user_prompt` is injected as additional context into every step's prompt.
///
/// Resource advertisement (the `<resources>` block prepended to each step's
/// system prompt) is enabled by default; pass `disable_resources = true` to
/// opt out, e.g. via `zig run --no-resources`.
///
/// Memory injection (the `<memory>` block) is similarly enabled by default;
/// pass `disable_memory = true` to opt out via `zig run --no-memory`.
///
/// Storage injection (the `<storage>` block) is similarly enabled by default;
/// pass `disable_storage = true` to opt out via `zig run --no-storage`.
///
/// When `dry_run = true`, the workflow is parsed, validated, and its plan is
/// printed in the requested `format` — no agent invocation, session log,
/// storage creation, or memory write occurs. The three `disable_*` flags are
/// respected and surface in the plan output.
#[allow(clippy::too_many_arguments)]
pub async fn run_workflow(
    workflow_path: &str,
    user_prompt: Option<&str>,
    disable_resources: bool,
    disable_memory: bool,
    disable_storage: bool,
    dry_run: bool,
    dry_run_format: DryRunFormat,
) -> Result<(), ZigError> {
    let path = resolve_workflow_path(workflow_path)?;
    let (workflow, source) = parser::parse_workflow(&path)?;

    if let Err(errors) = validate::validate(&workflow) {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        return Err(ZigError::Validation(msgs.join("; ")));
    }

    execute(
        &workflow,
        &path,
        user_prompt,
        source.dir(),
        disable_resources,
        disable_memory,
        disable_storage,
        dry_run,
        dry_run_format,
    )
    .await
}

/// Resolve a workflow argument to an actual file path.
///
/// Tries in order:
/// 1. Literal path as given
/// 2. With `.zwf` extension appended
/// 3. With `.zwfz` extension appended
/// 4. Under local project `.zig/workflows/` directory
/// 5. Under local project `.zig/workflows/` with `.zwf` appended
/// 6. Under local project `.zig/workflows/` with `.zwfz` appended
/// 7. Under global `~/.zig/workflows/` directory
/// 8. Under global `~/.zig/workflows/` with `.zwf` appended
/// 9. Under global `~/.zig/workflows/` with `.zwfz` appended
pub fn resolve_workflow_path(workflow: &str) -> Result<PathBuf, ZigError> {
    let mut candidates = vec![
        PathBuf::from(workflow),
        PathBuf::from(format!("{workflow}.zwf")),
        PathBuf::from(format!("{workflow}.zwfz")),
    ];

    if let Some(local_dir) = crate::paths::cwd_workflows_dir() {
        candidates.push(local_dir.join(workflow));
        candidates.push(local_dir.join(format!("{workflow}.zwf")));
        candidates.push(local_dir.join(format!("{workflow}.zwfz")));
    }

    if let Some(global_dir) = crate::paths::global_workflows_dir() {
        candidates.push(global_dir.join(workflow));
        candidates.push(global_dir.join(format!("{workflow}.zwf")));
        candidates.push(global_dir.join(format!("{workflow}.zwfz")));
    }

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    Err(ZigError::Io(format!(
        "workflow not found: '{workflow}' (tried: {})",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

/// Compute a topological ordering of steps grouped into tiers.
///
/// Each tier contains steps whose dependencies are all in earlier tiers,
/// meaning steps within a tier can (in principle) run in parallel.
/// Uses Kahn's algorithm.
pub(crate) fn topological_sort(steps: &[Step]) -> Result<Vec<Vec<&Step>>, ZigError> {
    let step_index: HashMap<&str, usize> = steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.as_str(), i))
        .collect();

    let mut in_degree = vec![0usize; steps.len()];
    for (i, step) in steps.iter().enumerate() {
        for dep in &step.depends_on {
            if step_index.contains_key(dep.as_str()) {
                in_degree[i] += 1;
            }
        }
    }

    let mut tiers = Vec::new();
    let mut remaining = in_degree.clone();
    let mut completed: Vec<bool> = vec![false; steps.len()];

    loop {
        let tier: Vec<usize> = (0..steps.len())
            .filter(|&i| !completed[i] && remaining[i] == 0)
            .collect();

        if tier.is_empty() {
            break;
        }

        for &i in &tier {
            completed[i] = true;
        }

        // Decrement in-degrees for dependents of this tier
        for &i in &tier {
            for (j, step) in steps.iter().enumerate() {
                if !completed[j] && step.depends_on.contains(&steps[i].name) {
                    remaining[j] -= 1;
                }
            }
        }

        tiers.push(tier.iter().map(|&i| &steps[i]).collect());
    }

    let completed_count: usize = completed.iter().filter(|&&c| c).count();
    if completed_count != steps.len() {
        return Err(ZigError::Execution(
            "could not resolve all steps — possible undetected cycle".into(),
        ));
    }

    Ok(tiers)
}

/// Replace `${var_name}` references in a template with values from the variable map.
///
/// Supports dotted paths like `${result.score}` — the root variable name is
/// looked up, and if its value is valid JSON, the nested path is traversed.
/// Unknown variables are left as-is.
pub(crate) fn substitute_vars(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];

        if let Some(end) = after_start.find('}') {
            let var_expr = &after_start[..end];
            let mut parts = var_expr.splitn(2, '.');
            let root = parts.next().unwrap_or(var_expr);

            if let Some(value) = vars.get(root) {
                if let Some(path) = parts.next() {
                    // Try to navigate a JSON path
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(value) {
                        let resolved = json_path_lookup(&json, path);
                        result.push_str(&resolved);
                    } else {
                        result.push_str(value);
                    }
                } else {
                    result.push_str(value);
                }
            } else {
                // Unknown variable — leave as-is
                result.push_str(&rest[start..start + 2 + end + 1]);
            }

            rest = &after_start[end + 1..];
        } else {
            result.push_str(&rest[start..]);
            rest = "";
        }
    }

    result.push_str(rest);
    result
}

/// Look up a dotted path in a JSON value (e.g., "nested.field").
fn json_path_lookup(value: &serde_json::Value, path: &str) -> String {
    let mut current = value;
    for key in path.split('.') {
        match current.get(key) {
            Some(v) => current = v,
            None => return format!("${{?.{path}}}"),
        }
    }
    match current {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Resolve the effective system prompt for a step, with any advertised
/// resources prepended as a `<resources>` block.
///
/// Resolution order:
/// 1. If `step.system_prompt` is set, use it (with variable substitution).
/// 2. If `step.role` is set, resolve the role name (may contain `${var}`),
///    look it up in the roles table, and use the role's system prompt
///    (loaded from file if `system_prompt_file` is set).
/// 3. Otherwise the base prompt is empty.
///
/// Resources from all configured tiers (global shared, global per-workflow,
/// project cwd, inline workflow, inline step) are then collected by the
/// supplied [`ResourceCollector`] and rendered into a `<resources>` block
/// that is prepended to the base prompt. If both the resources set and the
/// base prompt are empty, returns `None` — keeping the current behavior when
/// nothing is configured.
#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_role_system_prompt(
    step: &Step,
    roles: &HashMap<String, Role>,
    resources: &ResourceCollector<'_>,
    memory: &MemoryCollector,
    storage: &StorageManager,
    vars: &HashMap<String, String>,
    workflow_dir: &Path,
    workflow_name: &str,
) -> Result<Option<String>, ZigError> {
    // Resolve the base system prompt (may be empty if neither is set).
    let base_prompt: Option<String> = if let Some(ref sp) = step.system_prompt {
        Some(substitute_vars(sp, vars))
    } else if let Some(ref role_ref) = step.role {
        let resolved_name = substitute_vars(role_ref, vars);
        let role = roles.get(&resolved_name).ok_or_else(|| {
            ZigError::Execution(format!(
                "step '{}' references role '{}' which does not exist",
                step.name, resolved_name
            ))
        })?;

        let raw_prompt = if let Some(ref file_path) = role.system_prompt_file {
            let full_path = workflow_dir.join(expand_path(file_path));
            Some(std::fs::read_to_string(&full_path).map_err(|e| {
                ZigError::Execution(format!(
                    "failed to read system_prompt_file '{}' for role '{}': {e}",
                    full_path.display(),
                    resolved_name
                ))
            })?)
        } else {
            role.system_prompt.clone()
        };

        raw_prompt.map(|p| substitute_vars(&p, vars))
    } else {
        None
    };

    // Collect and render resources.
    let set = resources.collect_for_step(&step.resources)?;
    let resource_block = render_system_block(&set);

    // Collect and render memory.
    let memory_entries = memory.collect_for_step(step.memory.as_deref())?;
    let memory_block = render_memory_block(&memory_entries, workflow_name, Some(&step.name));

    // Render storage block for this step's scope.
    let storage_block = match storage.render_block(step.storage.as_deref())? {
        Some(mut s) => {
            s.push('\n');
            s
        }
        None => String::new(),
    };

    let prefix = format!("{resource_block}{memory_block}{storage_block}");

    match (prefix.is_empty(), base_prompt) {
        (true, None) => Ok(None),
        (true, Some(p)) => Ok(Some(p)),
        (false, None) => Ok(Some(prefix.trim_end().to_string())),
        (false, Some(p)) => Ok(Some(format!("{prefix}{p}"))),
    }
}

/// Load file-backed default values for variables.
///
/// For each variable with `default_file` set (and no `default`), reads the
/// file contents relative to `workflow_dir` and inserts them into the vars map.
fn load_file_defaults(
    vars: &mut HashMap<String, String>,
    declarations: &HashMap<String, crate::workflow::model::Variable>,
    workflow_dir: &Path,
) -> Result<(), ZigError> {
    for (name, decl) in declarations {
        if decl.default.is_none() {
            if let Some(ref file_path) = decl.default_file {
                let full_path = workflow_dir.join(expand_path(file_path));
                let content = std::fs::read_to_string(&full_path).map_err(|e| {
                    ZigError::Execution(format!(
                        "failed to read default_file '{}' for variable '{name}': {e}",
                        full_path.display()
                    ))
                })?;
                vars.insert(name.clone(), content);
            }
        }
    }
    Ok(())
}

/// Evaluate a simple condition expression against the current variable state.
///
/// Supports:
/// - Numeric comparisons: `score < 8`, `retries <= max_retries`
/// - String equality: `status == "done"`, `status != "pending"`
/// - Truthy checks: `approved` (true if value is "true" or non-empty and non-zero)
pub(crate) fn evaluate_condition(
    condition: &str,
    vars: &HashMap<String, String>,
) -> Result<bool, ZigError> {
    let condition = condition.trim();

    // Try comparison operators (ordered by length to match `<=` before `<`)
    let operators = ["<=", ">=", "!=", "==", "<", ">"];
    for op in &operators {
        if let Some(pos) = condition.find(op) {
            let lhs = resolve_operand(condition[..pos].trim(), vars);
            let rhs = resolve_operand(condition[pos + op.len()..].trim(), vars);
            return Ok(compare(&lhs, &rhs, op));
        }
    }

    // Truthy check: single variable name
    let value = vars.get(condition).map(|s| s.as_str()).unwrap_or("");
    Ok(is_truthy(value))
}

/// Resolve a condition operand to its string value.
/// - String literals ("done") → done
/// - Variable names → looked up in vars
/// - Numeric literals → left as-is
fn resolve_operand(token: &str, vars: &HashMap<String, String>) -> String {
    // Strip surrounding quotes for string literals
    if (token.starts_with('"') && token.ends_with('"'))
        || (token.starts_with('\'') && token.ends_with('\''))
    {
        return token[1..token.len() - 1].to_string();
    }
    // Try variable lookup
    if let Some(val) = vars.get(token) {
        return val.clone();
    }
    // Return as-is (numeric literal or unknown)
    token.to_string()
}

/// Compare two string operands with the given operator.
/// Attempts numeric comparison first, falls back to lexicographic.
fn compare(lhs: &str, rhs: &str, op: &str) -> bool {
    if let (Ok(l), Ok(r)) = (lhs.parse::<f64>(), rhs.parse::<f64>()) {
        return match op {
            "==" => (l - r).abs() < f64::EPSILON,
            "!=" => (l - r).abs() >= f64::EPSILON,
            "<" => l < r,
            ">" => l > r,
            "<=" => l <= r,
            ">=" => l >= r,
            _ => false,
        };
    }
    match op {
        "==" => lhs == rhs,
        "!=" => lhs != rhs,
        "<" => lhs < rhs,
        ">" => lhs > rhs,
        "<=" => lhs <= rhs,
        ">=" => lhs >= rhs,
        _ => false,
    }
}

/// Check if a string value is truthy.
fn is_truthy(value: &str) -> bool {
    !value.is_empty() && value != "false" && value != "0"
}

/// Build the final prompt for a step, incorporating variable substitution,
/// dependency outputs, and the user's context prompt.
pub(crate) fn render_step_prompt(
    step: &Step,
    vars: &HashMap<String, String>,
    user_prompt: Option<&str>,
    dependency_outputs: &HashMap<String, String>,
) -> String {
    let mut prompt = String::new();

    // Prepend user context if provided
    if let Some(ctx) = user_prompt {
        prompt.push_str(&format!("User context: {ctx}\n\n"));
    }

    // Inject dependency outputs if requested
    if step.inject_context {
        for dep in &step.depends_on {
            if let Some(output) = dependency_outputs.get(dep) {
                prompt.push_str(&format!("--- Output from '{dep}' ---\n{output}\n\n"));
            }
        }
    }

    // Append the step's prompt with variable substitution
    prompt.push_str(&substitute_vars(&step.prompt, vars));

    prompt
}

/// Serializable snapshot of everything a step contributes to an agent
/// invocation. Produced by [`build_agent_config`] and used by both the
/// executor (to configure a [`AgentBuilder`]) and `--dry-run` (to show
/// the author what each step will ask of the agent).
///
/// The shape mirrors [`AgentBuilder`] field-for-field; additional
/// command-specific parameters (review/plan/pipe/collect/summary) are
/// stored in the optional `command_params` bag.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AgentConfig {
    /// Subcommand label — `"run"`, `"review"`, `"plan"`, `"pipe"`,
    /// `"collect"`, or `"summary"`.
    pub command: String,

    /// Agent-level knobs.
    pub provider: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub root: Option<String>,
    pub add_dirs: Vec<String>,
    #[serde(serialize_with = "serialize_env_pairs")]
    pub env: Vec<(String, String)>,
    pub files: Vec<String>,
    pub auto_approve: bool,
    /// `None` → no worktree, `Some(None)` → generated name, `Some(Some(n))` → explicit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,

    /// Output shaping.
    pub json_mode: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,

    /// Turn / timeout / MCP.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_config: Option<String>,

    /// Session metadata — always set (session name derives from workflow/step).
    pub session_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub tags: Vec<String>,

    /// The effective prompt after dependency/context/plan prepending.
    pub prompt: String,

    /// Whether `accepts_agent_args` was true for this command — pipe/run/
    /// review/plan/exec respect agent-level flags; collect/summary don't.
    pub accepts_agent_args: bool,

    /// Extra params for the non-plain commands. `None` for `run`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_params: Option<CommandParams>,

    /// Interactive flag — steps with `interactive = true` run through
    /// `AgentBuilder::run` instead of `exec`.
    pub interactive: bool,
}

fn serialize_env_pairs<S>(pairs: &[(String, String)], s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = s.serialize_seq(Some(pairs.len()))?;
    for (k, v) in pairs {
        seq.serialize_element(&format!("{k}={v}"))?;
    }
    seq.end()
}

/// Command-specific parameter bag. Only populated for non-`run` commands.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandParams {
    Review {
        uncommitted: bool,
        base: Option<String>,
        commit: Option<String>,
        title: Option<String>,
    },
    Plan {
        output: Option<String>,
        instructions: Option<String>,
    },
    Pipe {
        session_ids: Vec<String>,
    },
    Collect {
        session_ids: Vec<String>,
    },
    Summary {
        session_ids: Vec<String>,
    },
}

/// Build an [`AgentConfig`] snapshot for a step. Replaces the old argv
/// builder (`build_zag_args`) — the returned config is applied to an
/// [`AgentBuilder`] at execution time by [`apply_agent_config`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_agent_config(
    step: &Step,
    prompt: &str,
    workflow_name: &str,
    model_override: Option<&str>,
    rendered_system_prompt: Option<&str>,
    workflow_provider: Option<&str>,
    workflow_model: Option<&str>,
    extra_add_dirs: &[std::path::PathBuf],
) -> AgentConfig {
    let session_name = |dep: &str| format!("zig-{workflow_name}-{dep}");

    let (command_label, accepts_agent_args, command_params) = match &step.command {
        None => ("run".to_string(), true, None),
        Some(StepCommand::Review) => (
            "review".to_string(),
            true,
            Some(CommandParams::Review {
                uncommitted: step.uncommitted,
                base: step.base.clone(),
                commit: step.commit.clone(),
                title: step.title.clone(),
            }),
        ),
        Some(StepCommand::Plan) => (
            "plan".to_string(),
            true,
            Some(CommandParams::Plan {
                output: step.plan_output.as_deref().map(expand_path),
                instructions: step.instructions.clone(),
            }),
        ),
        Some(StepCommand::Pipe) => {
            let session_ids: Vec<String> =
                step.depends_on.iter().map(|d| session_name(d)).collect();
            (
                "pipe".to_string(),
                true,
                Some(CommandParams::Pipe { session_ids }),
            )
        }
        Some(StepCommand::Collect) => {
            let session_ids: Vec<String> =
                step.depends_on.iter().map(|d| session_name(d)).collect();
            (
                "collect".to_string(),
                false,
                Some(CommandParams::Collect { session_ids }),
            )
        }
        Some(StepCommand::Summary) => {
            let session_ids: Vec<String> =
                step.depends_on.iter().map(|d| session_name(d)).collect();
            (
                "summary".to_string(),
                false,
                Some(CommandParams::Summary { session_ids }),
            )
        }
    };

    // Build the agent config. Agent-level knobs only apply when the
    // command accepts them (matching the old `accepts_agent_args` gate).
    let mut cfg = AgentConfig {
        command: command_label,
        session_name: session_name(&step.name),
        description: if step.description.is_empty() {
            None
        } else {
            Some(step.description.clone())
        },
        tags: {
            let mut t = vec!["zig-workflow".to_string()];
            t.extend(step.tags.iter().cloned());
            t
        },
        timeout: step.timeout.clone(),
        prompt: prompt.to_string(),
        accepts_agent_args,
        command_params,
        interactive: step.interactive,
        ..Default::default()
    };

    if !accepts_agent_args {
        return cfg;
    }

    cfg.provider = step
        .provider
        .clone()
        .or_else(|| workflow_provider.map(String::from));
    cfg.model = model_override
        .map(String::from)
        .or_else(|| step.model.clone())
        .or_else(|| workflow_model.map(String::from));
    cfg.system_prompt = rendered_system_prompt.map(String::from);
    cfg.max_turns = step.max_turns;

    // Output format: explicit `output` overrides the `json` bool; the
    // two map onto AgentBuilder::output_format and AgentBuilder::json.
    if let Some(output) = &step.output {
        cfg.output_format = Some(output.clone());
    } else if step.json {
        cfg.json_mode = true;
    }
    cfg.json_schema = step.json_schema.clone();
    cfg.mcp_config = step.mcp_config.as_deref().map(expand_path);

    cfg.auto_approve = step.auto_approve;
    cfg.root = step.root.as_deref().map(expand_path);
    cfg.add_dirs = step
        .add_dirs
        .iter()
        .map(|d| expand_path(d))
        .chain(extra_add_dirs.iter().map(|p| p.display().to_string()))
        .collect();
    cfg.env = step
        .env
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    cfg.files = step.files.iter().map(|f| expand_path(f)).collect();

    // Isolation
    if step.worktree {
        cfg.worktree = Some(None);
    }
    cfg.sandbox = step.sandbox.clone();

    cfg
}

/// Apply an [`AgentConfig`] to an [`AgentBuilder`]. The prompt itself
/// is NOT set here — callers pass it to `exec`/`run` directly.
pub(crate) fn apply_agent_config(mut builder: AgentBuilder, cfg: &AgentConfig) -> AgentBuilder {
    if let Some(ref p) = cfg.provider {
        builder = builder.provider(p);
    }
    if let Some(ref m) = cfg.model {
        builder = builder.model(m);
    }
    if let Some(ref sp) = cfg.system_prompt {
        builder = builder.system_prompt(sp);
    }
    if let Some(ref r) = cfg.root {
        builder = builder.root(r);
    }
    if cfg.auto_approve {
        builder = builder.auto_approve(true);
    }
    for dir in &cfg.add_dirs {
        builder = builder.add_dir(dir);
    }
    for (k, v) in &cfg.env {
        builder = builder.env(k, v);
    }
    for f in &cfg.files {
        builder = builder.file(f);
    }
    if let Some(ref wt) = cfg.worktree {
        builder = builder.worktree(wt.as_deref());
    }
    if let Some(ref sb) = cfg.sandbox {
        builder = builder.sandbox(Some(sb));
    }
    if let Some(ref fmt) = cfg.output_format {
        builder = builder.output_format(fmt);
    }
    if cfg.json_mode {
        if let Some(ref schema) = cfg.json_schema {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(schema) {
                builder = builder.json_schema(v);
            } else {
                builder = builder.json();
            }
        } else {
            builder = builder.json();
        }
    } else if let Some(ref schema) = cfg.json_schema {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(schema) {
            builder = builder.json_schema(v);
        }
    }
    if let Some(turns) = cfg.max_turns {
        builder = builder.max_turns(turns);
    }
    if let Some(ref t) = cfg.timeout {
        if let Some(dur) = parse_timeout_string(t) {
            builder = builder.timeout(dur);
        }
    }
    if let Some(ref mcp) = cfg.mcp_config {
        builder = builder.mcp_config(mcp);
    }
    builder = builder.name(&cfg.session_name);
    if let Some(ref d) = cfg.description {
        builder = builder.description(d);
    }
    for tag in &cfg.tags {
        builder = builder.tag(tag);
    }
    builder
}

/// Parse a duration string like `"5m"`, `"30s"`, `"1h30m"` into a
/// [`std::time::Duration`]. Returns `None` if the format is unrecognised.
fn parse_timeout_string(s: &str) -> Option<Duration> {
    // Mirrors zag_orch::duration::parse_duration — allows `1h30m`, `5m`,
    // `30s`, `500ms`, bare seconds (`60`). Keep the dependency inline so
    // we don't have to thread its error type through ZigError.
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(secs) = s.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    let mut total = Duration::ZERO;
    let mut current = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() || c == '.' {
            current.push(c);
            continue;
        }
        let mut unit = String::from(c);
        if c == 'm' && chars.peek() == Some(&'s') {
            unit.push(chars.next().unwrap());
        }
        let value: f64 = current.parse().ok()?;
        current.clear();
        let piece = match unit.as_str() {
            "ms" => Duration::from_millis(value as u64),
            "s" => Duration::from_secs_f64(value),
            "m" => Duration::from_secs_f64(value * 60.0),
            "h" => Duration::from_secs_f64(value * 3600.0),
            _ => return None,
        };
        total += piece;
    }
    if !current.is_empty() {
        return None;
    }
    Some(total)
}

/// Dispatch a built [`AgentConfig`] through the appropriate `zag-agent`
/// / `zag-orch` entry point and return the captured result text used for
/// `saves` and dependency injection.
///
/// Every path that drives an agent attaches a live `.on_log_event` hook
/// via [`install_live_streaming`] so per-turn activity (assistant
/// messages, tool calls, reasoning) streams to stderr and the zig session
/// log as it happens — restoring the old `zag run` streaming UX after the
/// subprocess-drop refactor.
///
/// - `run` → [`AgentBuilder::exec`] (or [`AgentBuilder::run`] for `interactive`).
/// - `review` (non-codex) → prompt built inline, then `AgentBuilder::exec`.
/// - `review` (codex) → [`zag_agent::review::run_review`] (no live stream; codex has a native review flow).
/// - `plan` → prompt built via [`zag_agent::plan::build_plan_prompt`], then `AgentBuilder::exec`, result optionally written to a file.
/// - `pipe` → context built inline via [`zag_orch::collect::extract_last_assistant_message`], then `AgentBuilder::exec`.
/// - `collect` → [`zag_orch::collect::collect_results`] (serialized as JSON; no agent).
/// - `summary` → [`zag_orch::summary::summarize_sessions`] (serialized as JSON; no agent).
async fn dispatch_agent(
    cfg: &AgentConfig,
    step_name: &str,
    session: Option<&Arc<SessionWriter>>,
    prefix: Option<&str>,
) -> Result<String, ZigError> {
    match cfg.command.as_str() {
        "run" => {
            if cfg.interactive {
                // Interactive sessions inherit stdio — the provider TUI
                // takes over the terminal and renders events directly, so
                // no log-event hook is wired here.
                let builder = apply_agent_config(AgentBuilder::new(), cfg);
                builder.run(Some(&cfg.prompt)).await.map_err(|e| {
                    ZigError::Zag(format!("agent run failed for step '{step_name}': {e}"))
                })?;
                Ok(String::new())
            } else {
                let mut builder = apply_agent_config(AgentBuilder::new(), cfg);
                builder = install_live_streaming(builder, step_name, session, prefix);
                let output = builder.exec(&cfg.prompt).await.map_err(|e| {
                    ZigError::Zag(format!("agent exec failed for step '{step_name}': {e}"))
                })?;
                Ok(output.result.unwrap_or_default())
            }
        }
        "review" => {
            let provider = cfg.provider.clone().unwrap_or_else(|| "claude".to_string());
            let (uncommitted, base, commit, title) = match &cfg.command_params {
                Some(CommandParams::Review {
                    uncommitted,
                    base,
                    commit,
                    title,
                }) => (*uncommitted, base.clone(), commit.clone(), title.clone()),
                _ => (false, None, None, None),
            };

            // Codex has a native review flow inside zag-agent that we
            // don't want to reimplement — fall back to the library call.
            // No live stream in that branch; same as before this fix.
            if provider == "codex" {
                let params = agent_review::ReviewParams {
                    provider,
                    uncommitted,
                    base,
                    commit,
                    title,
                    prompt: if cfg.prompt.is_empty() {
                        None
                    } else {
                        Some(cfg.prompt.clone())
                    },
                    system_prompt: cfg.system_prompt.clone(),
                    model: cfg.model.clone(),
                    root: cfg.root.clone(),
                    auto_approve: cfg.auto_approve,
                    add_dirs: cfg.add_dirs.clone(),
                    progress: Box::new(zag_agent::progress::SilentProgress),
                };
                let output = agent_review::run_review(params).await.map_err(|e| {
                    ZigError::Zag(format!("review failed for step '{step_name}': {e}"))
                })?;
                return Ok(output.and_then(|o| o.result).unwrap_or_default());
            }

            // Non-codex review: build the diff and prompt in-process, then
            // drive AgentBuilder directly so we can attach the live hook.
            let diff = agent_review::gather_diff(
                uncommitted,
                base.as_deref(),
                commit.as_deref(),
                cfg.root.as_deref(),
            )
            .map_err(|e| {
                ZigError::Zag(format!(
                    "review gather_diff failed for step '{step_name}': {e}"
                ))
            })?;
            let user_prompt = if cfg.prompt.is_empty() {
                None
            } else {
                Some(cfg.prompt.as_str())
            };
            let review_prompt =
                agent_review::build_review_prompt(&diff, title.as_deref(), user_prompt);

            let mut builder = apply_agent_config(AgentBuilder::new(), cfg);
            builder = install_live_streaming(builder, step_name, session, prefix);
            let output = builder.exec(&review_prompt).await.map_err(|e| {
                ZigError::Zag(format!("review exec failed for step '{step_name}': {e}"))
            })?;
            Ok(output.result.unwrap_or_default())
        }
        "plan" => {
            let (plan_output_path, instructions) = match &cfg.command_params {
                Some(CommandParams::Plan {
                    output,
                    instructions,
                }) => (output.clone(), instructions.clone()),
                _ => (None, None),
            };

            let plan_prompt = agent_plan::build_plan_prompt(&cfg.prompt, instructions.as_deref());

            let mut builder = apply_agent_config(AgentBuilder::new(), cfg);
            builder = install_live_streaming(builder, step_name, session, prefix);
            let output = builder.exec(&plan_prompt).await.map_err(|e| {
                ZigError::Zag(format!("plan exec failed for step '{step_name}': {e}"))
            })?;
            let text = output.result.unwrap_or_default();

            if let Some(path_str) = plan_output_path {
                let target = resolve_plan_output_path(&path_str);
                if let Some(parent) = target.parent()
                    && !parent.as_os_str().is_empty()
                {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        ZigError::Io(format!(
                            "failed to create plan output directory {}: {e}",
                            parent.display()
                        ))
                    })?;
                }
                std::fs::write(&target, &text).map_err(|e| {
                    ZigError::Io(format!(
                        "failed to write plan output to {}: {e}",
                        target.display()
                    ))
                })?;
                eprintln!("plan written to {}", target.display());
            }

            Ok(text)
        }
        "pipe" => {
            let session_ids = match &cfg.command_params {
                Some(CommandParams::Pipe { session_ids }) => session_ids.as_slice(),
                _ => &[] as &[String],
            };
            let context = build_pipe_context(session_ids, cfg.root.as_deref())?;
            let combined = format!(
                "Here are results from previous agent sessions:\n\n{context}\n\n{}",
                cfg.prompt
            );

            let mut builder = apply_agent_config(AgentBuilder::new(), cfg);
            builder = install_live_streaming(builder, step_name, session, prefix);
            let output = builder.exec(&combined).await.map_err(|e| {
                ZigError::Zag(format!("pipe exec failed for step '{step_name}': {e}"))
            })?;
            Ok(output.result.unwrap_or_default())
        }
        "collect" => {
            let session_ids = match &cfg.command_params {
                Some(CommandParams::Collect { session_ids }) => session_ids.clone(),
                _ => Vec::new(),
            };
            let params = orch_collect::CollectParams {
                session_ids,
                tag: None,
                json: true,
                root: cfg.root.clone(),
            };
            let results = orch_collect::collect_results(&params).map_err(|e| {
                ZigError::Zag(format!("collect failed for step '{step_name}': {e}"))
            })?;
            let json = serde_json::to_string(&results)
                .map_err(|e| ZigError::Execution(format!("collect serialization failed: {e}")))?;
            emit_captured(&json, step_name, session, prefix);
            Ok(json)
        }
        "summary" => {
            let session_ids = match &cfg.command_params {
                Some(CommandParams::Summary { session_ids }) => session_ids.clone(),
                _ => Vec::new(),
            };
            let params = orch_summary::SummaryParams {
                session_ids,
                tag: None,
                stats: false,
                json: true,
                root: cfg.root.clone(),
            };
            let results = orch_summary::summarize_sessions(&params).map_err(|e| {
                ZigError::Zag(format!("summary failed for step '{step_name}': {e}"))
            })?;
            let json = serde_json::to_string(&results)
                .map_err(|e| ZigError::Execution(format!("summary serialization failed: {e}")))?;
            emit_captured(&json, step_name, session, prefix);
            Ok(json)
        }
        other => Err(ZigError::Execution(format!(
            "unknown command '{other}' for step '{step_name}'"
        ))),
    }
}

/// Attach a live event stream to `builder`. Every [`AgentLogEvent`] that
/// reaches the session log fires the closure: rendered via
/// [`zag_agent::listen::format_event_text`] and routed to both stderr
/// (with optional `[prefix]` tagging for parallel tiers) and the zig
/// [`SessionWriter`] as `StepOutput` events — the same surface the old
/// `run_zag_streaming` helper produced before we dropped the subprocess.
fn install_live_streaming(
    builder: AgentBuilder,
    step_name: &str,
    session: Option<&Arc<SessionWriter>>,
    prefix: Option<&str>,
) -> AgentBuilder {
    let step_name_owned = step_name.to_string();
    let prefix_owned = prefix.map(String::from);
    let session_owned = session.cloned();
    builder.on_log_event(move |evt| {
        let Some(text) = zag_agent::listen::format_event_text(evt, false) else {
            return;
        };
        emit_live_line(
            &text,
            &step_name_owned,
            session_owned.as_ref(),
            prefix_owned.as_deref(),
        );
    })
}

/// Write one or more rendered lines to stderr (with optional prefix) and
/// mirror each line to the zig session writer.
fn emit_live_line(
    text: &str,
    step_name: &str,
    session: Option<&Arc<SessionWriter>>,
    prefix: Option<&str>,
) {
    use std::io::Write;
    if text.is_empty() {
        return;
    }
    let stderr = std::io::stderr();
    for line in text.lines() {
        if let Some(w) = session {
            let _ = w.step_output(step_name, OutputStream::Stdout, line);
        }
        let mut h = stderr.lock();
        let _ = match prefix {
            Some(p) => writeln!(h, "[{p}] {line}"),
            None => writeln!(h, "{line}"),
        };
    }
}

/// Emit captured non-agent output (collect / summary JSON) line-by-line
/// to stderr and the session writer. Kept for the two paths that don't
/// run an agent and therefore can't use [`install_live_streaming`].
fn emit_captured(
    text: &str,
    step_name: &str,
    session: Option<&Arc<SessionWriter>>,
    prefix: Option<&str>,
) {
    emit_live_line(text, step_name, session, prefix);
}

/// Build the `<session-result>` context block from upstream session IDs.
///
/// Mirrors `zag_orch::pipe::build_context` (`zag-orch/src/pipe.rs:86-118`)
/// byte-for-byte so the combined prompt zig feeds the agent matches what
/// the `zag pipe` CLI would have produced.
fn build_pipe_context(session_ids: &[String], root: Option<&str>) -> Result<String, ZigError> {
    let mut parts = Vec::new();
    for (i, id) in session_ids.iter().enumerate() {
        let Some(text) = orch_collect::extract_last_assistant_message(id, root) else {
            eprintln!("warning: no result found for upstream session {id}");
            continue;
        };
        let short = &id[..id.len().min(8)];
        let block = if session_ids.len() == 1 {
            format!("<session-result session=\"{short}\">\n{text}\n</session-result>")
        } else {
            format!(
                "<session-result index=\"{}\" session=\"{short}\">\n{text}\n</session-result>",
                i + 1
            )
        };
        parts.push(block);
    }

    if parts.is_empty() {
        return Err(ZigError::Execution(
            "pipe: no results available from the specified sessions".into(),
        ));
    }
    Ok(parts.join("\n\n"))
}

/// Resolve a `plan_output` path. If the caller specified a bare directory
/// name (no extension), append a timestamped `plan-YYYYMMDD-HHMMSS.md`
/// inside it — matching the behavior documented on
/// [`zag_agent::plan::PlanParams::output`].
fn resolve_plan_output_path(path_str: &str) -> std::path::PathBuf {
    let expanded = expand_path(path_str);
    let path = std::path::PathBuf::from(&expanded);
    if path.extension().is_some() {
        return path;
    }
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    path.join(format!("plan-{stamp}.md"))
}

/// Execute a single step through the agent-builder dispatch. Returns the
/// captured result text used for `saves` and dependency injection. The
/// optional `model_override` is used during retries to escalate to a
/// different model.
#[allow(clippy::too_many_arguments)]
async fn execute_step(
    step: &Step,
    prompt: &str,
    workflow_name: &str,
    model_override: Option<&str>,
    prefix: Option<&str>,
    session: Option<&Arc<SessionWriter>>,
    rendered_system_prompt: Option<&str>,
    workflow_provider: Option<&str>,
    workflow_model: Option<&str>,
    extra_add_dirs: &[std::path::PathBuf],
) -> Result<String, ZigError> {
    let cfg = build_agent_config(
        step,
        prompt,
        workflow_name,
        model_override,
        rendered_system_prompt,
        workflow_provider,
        workflow_model,
        extra_add_dirs,
    );
    dispatch_agent(&cfg, &step.name, session, prefix).await
}

/// Run a step with retry logic, returning its captured output on success.
///
/// Extracted so both sequential and parallel execution paths share the
/// same retry / model-escalation behavior.
#[allow(clippy::too_many_arguments)]
async fn run_step_attempts(
    step: &Step,
    prompt: &str,
    workflow_name: &str,
    prefix: Option<&str>,
    session: Option<&Arc<SessionWriter>>,
    rendered_system_prompt: Option<&str>,
    workflow_provider: Option<&str>,
    workflow_model: Option<&str>,
    extra_add_dirs: &[std::path::PathBuf],
) -> Result<String, ZigError> {
    let mut attempts = 0;
    let max_attempts = if step.on_failure.as_ref() == Some(&FailurePolicy::Retry) {
        step.max_retries.unwrap_or(1) + 1
    } else {
        1
    };

    loop {
        attempts += 1;
        let model_override = if attempts > 1 {
            step.retry_model.as_deref()
        } else {
            None
        };
        match execute_step(
            step,
            prompt,
            workflow_name,
            model_override,
            prefix,
            session,
            rendered_system_prompt,
            workflow_provider,
            workflow_model,
            extra_add_dirs,
        )
        .await
        {
            Ok(output) => return Ok(output),
            Err(e) => {
                if let Some(w) = session {
                    let _ = w.step_failed(&step.name, None, attempts, &e.to_string());
                }
                if attempts < max_attempts {
                    eprintln!(
                        "    retry {}/{} for step '{}'",
                        attempts,
                        max_attempts - 1,
                        step.name
                    );
                    continue;
                }
                return Err(e);
            }
        }
    }
}

/// Extract variable values from step output using `saves` selectors.
///
/// Selectors:
/// - `"$"` — the entire output
/// - `"$.field"` — a top-level JSON field
/// - `"$.nested.field"` — a nested JSON field
fn extract_saves(
    output: &str,
    saves: &HashMap<String, String>,
) -> Result<HashMap<String, String>, ZigError> {
    let mut extracted = HashMap::new();

    for (var_name, selector) in saves {
        let value = if selector == "$" {
            output.trim().to_string()
        } else if let Some(path) = selector.strip_prefix("$.") {
            let json: serde_json::Value = serde_json::from_str(output.trim()).map_err(|e| {
                ZigError::Execution(format!(
                    "saves selector '{selector}' requires JSON output, but got parse error: {e}"
                ))
            })?;
            json_path_lookup(&json, path)
        } else {
            output.trim().to_string()
        };

        extracted.insert(var_name.clone(), value);
    }

    Ok(extracted)
}

/// Partition a tier of steps into sequential steps and race groups.
///
/// Steps without a `race_group` are returned as sequential. Steps sharing
/// the same `race_group` value are grouped together for parallel execution.
fn partition_tier<'a>(tier: &[&'a Step]) -> (Vec<&'a Step>, HashMap<String, Vec<&'a Step>>) {
    let mut sequential = Vec::new();
    let mut race_groups: HashMap<String, Vec<&'a Step>> = HashMap::new();

    for step in tier {
        if let Some(group) = &step.race_group {
            race_groups.entry(group.clone()).or_default().push(step);
        } else {
            sequential.push(*step);
        }
    }

    (sequential, race_groups)
}

/// Execute a race group: run all steps concurrently via a [`JoinSet`] and
/// return the first winner. Once one step succeeds, the remaining tasks
/// are aborted (the underlying `tokio::process::Child` is dropped, which
/// kills the provider subprocess if `kill_on_drop` is set — zag-agent
/// sets this on its internal commands).
#[allow(clippy::too_many_arguments)]
async fn execute_race_group(
    steps: &[&Step],
    prompts: &HashMap<String, String>,
    system_prompts: &HashMap<String, String>,
    workflow_name: &str,
    tier_index: usize,
    session: Option<&Arc<SessionWriter>>,
    workflow_provider: Option<&str>,
    workflow_model: Option<&str>,
    storage_dirs: &HashMap<String, Vec<std::path::PathBuf>>,
) -> Result<(String, String), ZigError> {
    if let Some(w) = session {
        for step in steps {
            let zag_session_id = format!("zig-{workflow_name}-{}", step.name);
            let preview = prompts
                .get(&step.name)
                .map(|p| prompt_preview(p))
                .unwrap_or_default();
            let _ = w.step_started(
                &step.name,
                tier_index,
                &zag_session_id,
                zag_command_name(&step.command),
                step.model.as_deref(),
                &preview,
            );
        }
    }

    let race_started = Instant::now();
    let mut set: JoinSet<(String, Result<String, ZigError>)> = JoinSet::new();

    for step in steps {
        let prompt = prompts
            .get(&step.name)
            .ok_or_else(|| ZigError::Execution(format!("missing prompt for step '{}'", step.name)))?
            .clone();
        eprintln!("  racing step '{}'...", step.name);
        let rendered_sp = system_prompts.get(&step.name).cloned();
        let empty: Vec<std::path::PathBuf> = Vec::new();
        let extra_dirs = storage_dirs.get(&step.name).unwrap_or(&empty).clone();
        let step_clone: Step = (*step).clone();
        let wf_name = workflow_name.to_string();
        let wf_provider = workflow_provider.map(String::from);
        let wf_model = workflow_model.map(String::from);
        let session_clone = session.cloned();
        let name = step.name.clone();
        set.spawn(async move {
            let res = execute_step(
                &step_clone,
                &prompt,
                &wf_name,
                None,
                None,
                session_clone.as_ref(),
                rendered_sp.as_deref(),
                wf_provider.as_deref(),
                wf_model.as_deref(),
                &extra_dirs,
            )
            .await;
            (name, res)
        });
    }

    // Wait for the first winner — drop (abort) the rest.
    while let Some(joined) = set.join_next().await {
        let (winner_name, result) = match joined {
            Ok(pair) => pair,
            Err(e) if e.is_cancelled() => continue,
            Err(e) => return Err(ZigError::Execution(format!("race task panicked: {e}"))),
        };
        match result {
            Ok(stdout) => {
                // Abort losers.
                set.abort_all();
                while let Some(r) = set.join_next().await {
                    if let Ok((name, _)) = r {
                        eprintln!("  cancelling step '{name}' (race lost)");
                    }
                }
                let elapsed = race_started.elapsed().as_millis() as u64;
                eprintln!("  race won by '{winner_name}'");
                if let Some(w) = session {
                    let _ = w.step_completed(&winner_name, 0, elapsed, Vec::new());
                }
                return Ok((winner_name, stdout));
            }
            Err(e) => {
                if let Some(w) = session {
                    let _ = w.step_failed(&winner_name, None, 1, &e.to_string());
                }
                // Keep racing remaining tasks; this one lost.
                continue;
            }
        }
    }

    Err(ZigError::Execution(
        "all racers failed without a winner".into(),
    ))
}

/// Execute a single sequential step with retry logic, saves, and next-jump handling.
#[allow(clippy::too_many_arguments)]
async fn execute_sequential_step(
    step: &Step,
    vars: &mut HashMap<String, String>,
    user_prompt: Option<&str>,
    step_outputs: &mut HashMap<String, String>,
    workflow_name: &str,
    pending_next: &mut Option<String>,
    tier_index: usize,
    session: Option<&Arc<SessionWriter>>,
    roles: &HashMap<String, Role>,
    resources: &ResourceCollector<'_>,
    memory: &MemoryCollector,
    storage: &StorageManager,
    workflow_dir: &Path,
    workflow_provider: Option<&str>,
    workflow_model: Option<&str>,
) -> Result<(), ZigError> {
    if let Some(condition) = &step.condition {
        if !evaluate_condition(condition, vars)? {
            eprintln!(
                "  skipping '{}' (condition not met: {condition})",
                step.name
            );
            if let Some(w) = session {
                let _ = w.step_skipped(&step.name, &format!("condition not met: {condition}"));
            }
            return Ok(());
        }
    }

    eprintln!("  running step '{}'...", step.name);

    let prompt = render_step_prompt(step, vars, user_prompt, step_outputs);
    let rendered_sp = resolve_role_system_prompt(
        step,
        roles,
        resources,
        memory,
        storage,
        vars,
        workflow_dir,
        workflow_name,
    )?;
    let storage_dirs = storage.add_dirs_for_step(step.storage.as_deref());
    if let Some(w) = session {
        let zag_session_id = format!("zig-{workflow_name}-{}", step.name);
        let _ = w.step_started(
            &step.name,
            tier_index,
            &zag_session_id,
            zag_command_name(&step.command),
            step.model.as_deref(),
            &prompt_preview(&prompt),
        );
    }
    let started = Instant::now();
    let result = run_step_attempts(
        step,
        &prompt,
        workflow_name,
        None,
        session,
        rendered_sp.as_deref(),
        workflow_provider,
        workflow_model,
        &storage_dirs,
    )
    .await;

    match result {
        Ok(output) => {
            let mut saved_keys: Vec<String> = Vec::new();
            if !step.saves.is_empty() {
                let saved = extract_saves(&output, &step.saves)?;
                for (k, v) in &saved {
                    eprintln!("    saved {k} = {v}");
                    saved_keys.push(k.clone());
                }
                vars.extend(saved);
            }

            step_outputs.insert(step.name.clone(), output);
            eprintln!("  completed '{}'", step.name);
            if let Some(w) = session {
                let _ = w.step_completed(
                    &step.name,
                    0,
                    started.elapsed().as_millis() as u64,
                    saved_keys,
                );
            }

            if step.next.is_some() {
                *pending_next = step.next.clone();
            }
        }
        Err(e) => match step.on_failure.as_ref().unwrap_or(&FailurePolicy::Fail) {
            FailurePolicy::Fail => return Err(e),
            FailurePolicy::Continue => {
                eprintln!("  step '{}' failed (continuing): {e}", step.name);
            }
            FailurePolicy::Retry => {
                return Err(e);
            }
        },
    }

    Ok(())
}

/// Run multiple independent steps in a tier concurrently.
///
/// All non-skipped steps are spawned as tokio tasks via [`JoinSet`] and
/// we wait for every one to finish (unlike race groups, which abort
/// losers). Captured output lines are written back to stderr after each
/// task completes, prefixed with the step name to disambiguate. Results
/// are processed in tier-declaration order so `saves`, `next`, and
/// `on_failure` semantics remain deterministic.
#[allow(clippy::too_many_arguments)]
async fn execute_parallel_tier(
    steps: &[&Step],
    vars: &mut HashMap<String, String>,
    user_prompt: Option<&str>,
    step_outputs: &mut HashMap<String, String>,
    workflow_name: &str,
    pending_next: &mut Option<String>,
    tier_index: usize,
    session: Option<&Arc<SessionWriter>>,
    roles: &HashMap<String, Role>,
    resources: &ResourceCollector<'_>,
    memory: &MemoryCollector,
    storage: &StorageManager,
    workflow_dir: &Path,
    workflow_provider: Option<&str>,
    workflow_model: Option<&str>,
) -> Result<(), ZigError> {
    // Evaluate conditions and render prompts up front, so threads receive
    // the same variable snapshot they would have under sequential execution.
    let mut active: Vec<&Step> = Vec::new();
    let mut prompts: HashMap<String, String> = HashMap::new();
    let mut rendered_sps: HashMap<String, String> = HashMap::new();
    let mut storage_dirs_map: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();
    for step in steps {
        if let Some(condition) = &step.condition {
            if !evaluate_condition(condition, vars)? {
                eprintln!(
                    "  skipping '{}' (condition not met: {condition})",
                    step.name
                );
                if let Some(w) = session {
                    let _ = w.step_skipped(&step.name, &format!("condition not met: {condition}"));
                }
                continue;
            }
        }
        let prompt = render_step_prompt(step, vars, user_prompt, step_outputs);
        prompts.insert(step.name.clone(), prompt);
        if let Some(sp) = resolve_role_system_prompt(
            step,
            roles,
            resources,
            memory,
            storage,
            vars,
            workflow_dir,
            workflow_name,
        )? {
            rendered_sps.insert(step.name.clone(), sp);
        }
        storage_dirs_map.insert(
            step.name.clone(),
            storage.add_dirs_for_step(step.storage.as_deref()),
        );
        active.push(*step);
    }

    if active.is_empty() {
        return Ok(());
    }

    eprintln!("  running {} steps in parallel...", active.len());

    let mut start_times: HashMap<String, Instant> = HashMap::new();
    let mut set: JoinSet<(String, Result<String, ZigError>)> = JoinSet::new();
    for step in &active {
        let step_clone: Step = (*step).clone();
        let prompt = prompts.remove(&step.name).unwrap_or_default();
        let rendered_sp = rendered_sps.remove(&step.name);
        let workflow_name_owned = workflow_name.to_string();
        let name = step.name.clone();
        eprintln!("  starting '{name}'...");
        if let Some(w) = session {
            let zag_session_id = format!("zig-{workflow_name}-{name}");
            let _ = w.step_started(
                &name,
                tier_index,
                &zag_session_id,
                zag_command_name(&step.command),
                step.model.as_deref(),
                &prompt_preview(&prompt),
            );
        }
        start_times.insert(name.clone(), Instant::now());
        let session_clone = session.cloned();
        let wf_provider = workflow_provider.map(String::from);
        let wf_model = workflow_model.map(String::from);
        let storage_dirs = storage_dirs_map.remove(&step.name).unwrap_or_default();
        set.spawn(async move {
            let res = run_step_attempts(
                &step_clone,
                &prompt,
                &workflow_name_owned,
                Some(&name),
                session_clone.as_ref(),
                rendered_sp.as_deref(),
                wf_provider.as_deref(),
                wf_model.as_deref(),
                &storage_dirs,
            )
            .await;
            (name, res)
        });
    }

    let mut results: HashMap<String, Result<String, ZigError>> = HashMap::new();
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok((name, res)) => {
                results.insert(name, res);
            }
            Err(e) => {
                return Err(ZigError::Execution(format!(
                    "parallel step task panicked: {e}"
                )));
            }
        }
    }

    // Process results in tier-declaration order so `next` is deterministic.
    let mut errors: Vec<String> = Vec::new();
    for step in &active {
        let Some(res) = results.remove(&step.name) else {
            continue;
        };
        let elapsed = start_times
            .remove(&step.name)
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        match res {
            Ok(output) => {
                let mut saved_keys: Vec<String> = Vec::new();
                if !step.saves.is_empty() {
                    let saved = extract_saves(&output, &step.saves)?;
                    for (k, v) in &saved {
                        eprintln!("    saved {k} = {v}");
                        saved_keys.push(k.clone());
                    }
                    vars.extend(saved);
                }
                step_outputs.insert(step.name.clone(), output);
                eprintln!("  completed '{}'", step.name);
                if let Some(w) = session {
                    let _ = w.step_completed(&step.name, 0, elapsed, saved_keys);
                }
                if step.next.is_some() && pending_next.is_none() {
                    *pending_next = step.next.clone();
                }
            }
            Err(e) => match step.on_failure.as_ref().unwrap_or(&FailurePolicy::Fail) {
                FailurePolicy::Continue => {
                    eprintln!("  step '{}' failed (continuing): {e}", step.name);
                }
                FailurePolicy::Fail | FailurePolicy::Retry => {
                    errors.push(format!("'{}': {e}", step.name));
                }
            },
        }
    }

    if !errors.is_empty() {
        return Err(ZigError::Execution(format!(
            "parallel step(s) failed: {}",
            errors.join("; ")
        )));
    }

    Ok(())
}

/// Initialize the variable map from workflow variable definitions.
/// Variables with defaults are set to their default value; others are empty.
fn init_vars(workflow: &Workflow) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for (name, var) in &workflow.vars {
        let value = match &var.default {
            Some(toml::Value::String(s)) => s.clone(),
            Some(toml::Value::Integer(n)) => n.to_string(),
            Some(toml::Value::Float(f)) => f.to_string(),
            Some(toml::Value::Boolean(b)) => b.to_string(),
            Some(other) => other.to_string(),
            None => String::new(),
        };
        vars.insert(name.clone(), value);
    }
    vars
}

/// Main execution loop for a validated workflow.
#[allow(clippy::too_many_arguments)]
async fn execute(
    workflow: &Workflow,
    workflow_path: &std::path::Path,
    user_prompt: Option<&str>,
    workflow_dir: &Path,
    disable_resources: bool,
    disable_memory: bool,
    disable_storage: bool,
    dry_run: bool,
    dry_run_format: DryRunFormat,
) -> Result<(), ZigError> {
    let mut vars = init_vars(workflow);

    let resource_collector = ResourceCollector::from_env(
        &workflow.workflow.name,
        &workflow.workflow.resources,
        workflow_dir,
        disable_resources,
    );

    let config = ZigConfig::load();
    let workflow_memory_mode = MemoryMode::from_str_opt(workflow.workflow.memory.as_deref());
    let memory_collector = MemoryCollector::from_env(
        &workflow.workflow.name,
        workflow_memory_mode,
        &config,
        disable_memory,
    );

    // Build storage manager for this run. Paths resolve against <cwd>/.zig/;
    // absolute paths pass through. `ensure` is called on every declared item
    // so step agents can trust the paths exist before they run.
    // When `--no-storage` is passed, skip building entirely so storage dirs
    // are not created and the `<storage>` block is omitted from prompts.
    // When `--dry-run` is passed, build the manager without `ensure` so the
    // block still renders with correct paths but nothing touches disk.
    let storage_manager = if disable_storage || workflow.storage.is_empty() {
        StorageManager::empty()
    } else if dry_run {
        let backend = FilesystemBackend::from_cwd()?;
        StorageManager::build_dry(&workflow.storage, backend)
    } else {
        let backend = FilesystemBackend::from_cwd()?;
        StorageManager::build(&workflow.storage, backend)?
    };

    // Load file-backed variable defaults before prompt binding.
    load_file_defaults(&mut vars, &workflow.vars, workflow_dir)?;

    // Bind user prompt to the variable with `from = "prompt"`, if any.
    let prompt_var = workflow
        .vars
        .iter()
        .find(|(_, v)| v.from.as_deref() == Some("prompt"))
        .map(|(name, _)| name.clone());

    if let Some(ref var_name) = prompt_var {
        if let Some(prompt) = user_prompt {
            vars.insert(var_name.clone(), prompt.to_string());
        }
    }

    // Validate variable values against constraints before executing.
    if let Err(errors) = validate::validate_var_values(&vars, &workflow.vars) {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        return Err(ZigError::Validation(msgs.join("; ")));
    }

    // When prompt is bound to a variable, don't also prepend "User context:".
    let effective_user_prompt = if prompt_var.is_some() {
        None
    } else {
        user_prompt
    };

    let mut step_outputs: HashMap<String, String> = HashMap::new();

    let wf_provider = workflow.workflow.provider.as_deref();
    let wf_model = workflow.workflow.model.as_deref();

    let tiers = topological_sort(&workflow.steps)?;

    if dry_run {
        let ctx = DryRunContext {
            workflow,
            workflow_path,
            workflow_dir,
            vars: &vars,
            user_prompt: effective_user_prompt,
            roles: &workflow.roles,
            resources: &resource_collector,
            memory: &memory_collector,
            storage: &storage_manager,
            wf_provider,
            wf_model,
            disable_resources,
            disable_memory,
            disable_storage,
        };
        return crate::dry_run::print_plan(&ctx, &tiers, dry_run_format);
    }

    eprintln!(
        "running workflow '{}' ({} steps in {} tiers)",
        workflow.workflow.name,
        workflow.steps.len(),
        tiers.len()
    );

    // Open a zig session log for this run. Failure to open the log is
    // non-fatal — `zig run` should still execute even if `~/.zig` is
    // unwritable. The session writer is `Option<Arc<...>>` everywhere
    // downstream so the writer-less path stays intact for tests.
    let coordinator = match SessionWriter::create(
        &workflow.workflow.name,
        &workflow_path.to_string_lossy(),
        user_prompt,
        tiers.len(),
    ) {
        Ok(writer) => {
            eprintln!("zig session: {}", writer.session_id());
            Some(SessionCoordinator::start(writer))
        }
        Err(e) => {
            eprintln!("warning: failed to open zig session log: {e}");
            None
        }
    };
    let session_writer: Option<Arc<SessionWriter>> = coordinator.as_ref().map(|c| c.writer());
    let session_ref = session_writer.as_ref();

    let mut iteration = 0;
    let mut pending_next: Option<String> = None;

    loop {
        let tiers_to_run = if let Some(ref next_step) = pending_next {
            // Re-run from the target step onward
            let remaining: Vec<Vec<&Step>> = tiers
                .iter()
                .map(|tier| {
                    tier.iter()
                        .filter(|s| s.name == *next_step)
                        .copied()
                        .collect::<Vec<_>>()
                })
                .filter(|tier| !tier.is_empty())
                .collect();
            pending_next = None;
            remaining
        } else if iteration == 0 {
            tiers.clone()
        } else {
            break;
        };

        for (tier_index, tier) in tiers_to_run.iter().enumerate() {
            let (non_race, race_groups) = partition_tier(tier);

            if let Some(w) = session_ref {
                let names: Vec<String> = tier.iter().map(|s| s.name.clone()).collect();
                let _ = w.tier_started(tier_index, names);
            }

            // Independent steps in the same tier run concurrently. A single
            // step takes the sequential path so its output streams without
            // a name prefix; multiple steps go through the parallel path.
            if non_race.len() <= 1 {
                for step in &non_race {
                    execute_sequential_step(
                        step,
                        &mut vars,
                        effective_user_prompt,
                        &mut step_outputs,
                        &workflow.workflow.name,
                        &mut pending_next,
                        tier_index,
                        session_ref,
                        &workflow.roles,
                        &resource_collector,
                        &memory_collector,
                        &storage_manager,
                        workflow_dir,
                        wf_provider,
                        wf_model,
                    )
                    .await?;
                }
            } else {
                execute_parallel_tier(
                    &non_race,
                    &mut vars,
                    effective_user_prompt,
                    &mut step_outputs,
                    &workflow.workflow.name,
                    &mut pending_next,
                    tier_index,
                    session_ref,
                    &workflow.roles,
                    &resource_collector,
                    &memory_collector,
                    &storage_manager,
                    workflow_dir,
                    wf_provider,
                    wf_model,
                )
                .await?;
            }

            // Run race groups in parallel
            for (group_name, race_steps) in &race_groups {
                eprintln!("  starting race group '{group_name}'...");

                // Build prompts for all racers (conditions evaluated here)
                let mut prompts = HashMap::new();
                let mut race_sps: HashMap<String, String> = HashMap::new();
                let mut race_storage_dirs: HashMap<String, Vec<std::path::PathBuf>> =
                    HashMap::new();
                let mut active_steps: Vec<&Step> = Vec::new();
                for step in race_steps {
                    if let Some(condition) = &step.condition {
                        if !evaluate_condition(condition, &vars)? {
                            eprintln!(
                                "  skipping '{}' (condition not met: {condition})",
                                step.name
                            );
                            continue;
                        }
                    }
                    let prompt =
                        render_step_prompt(step, &vars, effective_user_prompt, &step_outputs);
                    prompts.insert(step.name.clone(), prompt);
                    if let Some(sp) = resolve_role_system_prompt(
                        step,
                        &workflow.roles,
                        &resource_collector,
                        &memory_collector,
                        &storage_manager,
                        &vars,
                        workflow_dir,
                        &workflow.workflow.name,
                    )? {
                        race_sps.insert(step.name.clone(), sp);
                    }
                    race_storage_dirs.insert(
                        step.name.clone(),
                        storage_manager.add_dirs_for_step(step.storage.as_deref()),
                    );
                    active_steps.push(step);
                }

                if active_steps.is_empty() {
                    continue;
                }

                match execute_race_group(
                    &active_steps,
                    &prompts,
                    &race_sps,
                    &workflow.workflow.name,
                    tier_index,
                    session_ref,
                    wf_provider,
                    wf_model,
                    &race_storage_dirs,
                )
                .await
                {
                    Ok((winner_name, output)) => {
                        // Find the winning step to process saves/next
                        if let Some(winner) = active_steps.iter().find(|s| s.name == winner_name) {
                            if !winner.saves.is_empty() {
                                let saved = extract_saves(&output, &winner.saves)?;
                                for (k, v) in &saved {
                                    eprintln!("    saved {k} = {v}");
                                }
                                vars.extend(saved);
                            }
                            if winner.next.is_some() {
                                pending_next = winner.next.clone();
                            }
                        }
                        step_outputs.insert(winner_name.clone(), output);
                        eprintln!(
                            "  completed race group '{group_name}' (winner: '{winner_name}')"
                        );
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        iteration += 1;
        if pending_next.is_none() || iteration >= MAX_LOOP_ITERATIONS {
            if iteration >= MAX_LOOP_ITERATIONS {
                eprintln!("warning: reached maximum loop iterations ({MAX_LOOP_ITERATIONS})");
            }
            break;
        }
    }

    eprintln!("workflow '{}' completed", workflow.workflow.name);
    if let Some(c) = coordinator {
        let _ = c.finish(SessionStatus::Success);
    }
    Ok(())
}

/// Short label for the zag subcommand a step will invoke. Used in
/// `step_started` events so listeners can distinguish run/review/plan/etc.
fn zag_command_name(cmd: &Option<StepCommand>) -> &'static str {
    match cmd {
        None => "run",
        Some(StepCommand::Review) => "review",
        Some(StepCommand::Plan) => "plan",
        Some(StepCommand::Pipe) => "pipe",
        Some(StepCommand::Collect) => "collect",
        Some(StepCommand::Summary) => "summary",
    }
}

/// Truncated single-line preview of a rendered prompt for the session log.
fn prompt_preview(prompt: &str) -> String {
    const MAX: usize = 200;
    let collapsed: String = prompt
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    if collapsed.chars().count() <= MAX {
        collapsed
    } else {
        let truncated: String = collapsed.chars().take(MAX).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
