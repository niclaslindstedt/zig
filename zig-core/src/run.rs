use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use std::time::Instant;

use crate::config::ZigConfig;
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
/// each step by delegating to `zag`. The optional `user_prompt` is injected
/// as additional context into every step's prompt.
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
pub fn run_workflow(
    workflow_path: &str,
    user_prompt: Option<&str>,
    disable_resources: bool,
    disable_memory: bool,
    disable_storage: bool,
) -> Result<(), ZigError> {
    check_zag()?;

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
    )
}

/// Verify that `zag` is installed and available on PATH.
pub(crate) fn check_zag() -> Result<(), ZigError> {
    let zag_available = Command::new("zag")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());

    if !zag_available {
        return Err(ZigError::Zag(
            "zag is not installed or not in PATH. Install it from https://github.com/niclaslindstedt/zag".into(),
        ));
    }
    Ok(())
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
fn substitute_vars(template: &str, vars: &HashMap<String, String>) -> String {
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
fn resolve_role_system_prompt(
    step: &Step,
    roles: &HashMap<String, Role>,
    resources: &ResourceCollector,
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
fn evaluate_condition(condition: &str, vars: &HashMap<String, String>) -> Result<bool, ZigError> {
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
fn render_step_prompt(
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

/// Build the argument list for a zag invocation.
///
/// Separated from `execute_step` to allow unit testing of flag logic
/// without a real `zag` binary. The optional `model_override` is used
/// during retries when `retry_model` escalates to a different model.
///
/// The subcommand is determined by `step.command`:
/// - `None` → `zag run <prompt>` (default)
/// - `Review` → `zag review [prompt] [--uncommitted] [--base] [--commit] [--title]`
/// - `Plan` → `zag plan <prompt> [-o path] [--instructions]`
/// - `Pipe` → `zag pipe <session-ids...> -- <prompt>`
/// - `Collect` → `zag collect <session-ids...>`
/// - `Summary` → `zag summary <session-ids...>`
#[allow(clippy::too_many_arguments)]
fn build_zag_args(
    step: &Step,
    prompt: &str,
    workflow_name: &str,
    model_override: Option<&str>,
    rendered_system_prompt: Option<&str>,
    workflow_provider: Option<&str>,
    workflow_model: Option<&str>,
    extra_add_dirs: &[std::path::PathBuf],
) -> Vec<String> {
    let session_name = |dep: &str| format!("zig-{workflow_name}-{dep}");

    // Build command-specific prefix and determine if agent args apply
    let (mut args, accepts_agent_args) = match &step.command {
        None => (vec!["run".to_string(), prompt.to_string()], true),
        Some(StepCommand::Review) => {
            let mut a = vec!["review".to_string()];
            if !prompt.is_empty() {
                a.push(prompt.to_string());
            }
            if step.uncommitted {
                a.push("--uncommitted".into());
            }
            if let Some(base) = &step.base {
                a.extend(["--base".into(), base.clone()]);
            }
            if let Some(commit) = &step.commit {
                a.extend(["--commit".into(), commit.clone()]);
            }
            if let Some(title) = &step.title {
                a.extend(["--title".into(), title.clone()]);
            }
            (a, true)
        }
        Some(StepCommand::Plan) => {
            let mut a = vec!["plan".to_string(), prompt.to_string()];
            if let Some(output) = &step.plan_output {
                a.extend(["-o".into(), expand_path(output)]);
            }
            if let Some(instructions) = &step.instructions {
                a.extend(["--instructions".into(), instructions.clone()]);
            }
            (a, true)
        }
        Some(StepCommand::Pipe) => {
            let mut a = vec!["pipe".to_string()];
            for dep in &step.depends_on {
                a.push(session_name(dep));
            }
            a.push("--".into());
            a.push(prompt.to_string());
            (a, true)
        }
        Some(StepCommand::Collect) => {
            let mut a = vec!["collect".to_string()];
            for dep in &step.depends_on {
                a.push(session_name(dep));
            }
            (a, false)
        }
        Some(StepCommand::Summary) => {
            let mut a = vec!["summary".to_string()];
            for dep in &step.depends_on {
                a.push(session_name(dep));
            }
            (a, false)
        }
    };

    // Agent args (provider, model, prompts, output, etc.) only apply to
    // commands that launch an agent: run, review, plan, pipe.
    if accepts_agent_args {
        let effective_provider = step.provider.as_deref().or(workflow_provider);
        if let Some(provider) = effective_provider {
            args.extend(["--provider".into(), provider.to_string()]);
        }

        let effective_model = model_override.or(step.model.as_deref()).or(workflow_model);
        if let Some(model) = effective_model {
            args.extend(["--model".into(), model.to_string()]);
        }

        if let Some(sp) = rendered_system_prompt {
            args.extend(["--system-prompt".into(), sp.to_string()]);
        }
        if let Some(max_turns) = step.max_turns {
            args.extend(["--max-turns".into(), max_turns.to_string()]);
        }

        // Output format: explicit format overrides the json bool
        if let Some(output) = &step.output {
            args.extend(["-o".into(), output.clone()]);
        } else if step.json {
            args.push("--json".into());
        }
        if let Some(schema) = &step.json_schema {
            args.extend(["--json-schema".into(), schema.clone()]);
        }

        if let Some(mcp_config) = &step.mcp_config {
            args.extend(["--mcp-config".into(), expand_path(mcp_config)]);
        }

        // Execution environment
        if step.auto_approve {
            args.push("--auto-approve".into());
        }
        if let Some(root) = &step.root {
            args.extend(["--root".into(), expand_path(root)]);
        }
        for dir in &step.add_dirs {
            args.extend(["--add-dir".into(), expand_path(dir)]);
        }
        for dir in extra_add_dirs {
            args.extend(["--add-dir".into(), dir.display().to_string()]);
        }
        for (key, value) in &step.env {
            args.extend(["--env".into(), format!("{key}={value}")]);
        }
        for file in &step.files {
            args.extend(["--file".into(), expand_path(file)]);
        }

        // Context injection
        for ctx in &step.context {
            args.extend(["--context".into(), ctx.clone()]);
        }
        if let Some(plan) = &step.plan {
            args.extend(["--plan".into(), expand_path(plan)]);
        }

        // Isolation
        if step.worktree {
            args.push("--worktree".into());
        }
        if let Some(sandbox) = &step.sandbox {
            args.extend(["--sandbox".into(), sandbox.clone()]);
        }
    }

    // Session metadata applies to all commands
    let name = session_name(&step.name);
    args.extend(["--name".into(), name]);

    if !step.description.is_empty() {
        args.extend(["--description".into(), step.description.clone()]);
    }

    args.extend(["--tag".into(), "zig-workflow".into()]);
    for tag in &step.tags {
        args.extend(["--tag".into(), tag.clone()]);
    }

    if let Some(timeout) = &step.timeout {
        args.extend(["--timeout".into(), timeout.clone()]);
    }

    args
}

/// Spawn `zag` with all three stdio streams inherited so the agent's
/// interactive TUI can take over the terminal. No output is captured,
/// so `saves` cannot apply — validation rejects that combination.
fn run_zag_interactive(
    args: &[String],
    step_name: &str,
) -> Result<std::process::ExitStatus, ZigError> {
    let mut cmd = Command::new("zag");
    cmd.args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    let mut child = cmd.spawn().map_err(|e| {
        ZigError::Zag(format!(
            "failed to launch zag (interactive) for step '{step_name}': {e}"
        ))
    })?;

    child
        .wait()
        .map_err(|e| ZigError::Execution(format!("failed to wait for child: {e}")))
}

/// Spawn `zag` and stream its stdout/stderr live to our stderr while
/// also accumulating stdout into a buffer for `saves` extraction.
///
/// If `prefix` is `Some`, every emitted line is prefixed with `[prefix] `
/// — used to disambiguate output from steps running in parallel.
fn run_zag_streaming(
    args: &[String],
    step_name: &str,
    prefix: Option<&str>,
    session: Option<&Arc<SessionWriter>>,
) -> Result<(std::process::ExitStatus, String), ZigError> {
    let mut cmd = Command::new("zag");
    cmd.args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| ZigError::Zag(format!("failed to launch zag for step '{step_name}': {e}")))?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    let buffer = Arc::new(Mutex::new(String::new()));
    let buffer_clone = Arc::clone(&buffer);
    let prefix_stdout = prefix.map(String::from);
    let prefix_stderr = prefix.map(String::from);
    let session_stdout = session.cloned();
    let session_stderr = session.cloned();
    let step_name_stdout = step_name.to_string();
    let step_name_stderr = step_name.to_string();

    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let stderr_handle = std::io::stderr();
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(mut buf) = buffer_clone.lock() {
                buf.push_str(&line);
                buf.push('\n');
            }
            if let Some(w) = &session_stdout {
                let _ = w.step_output(&step_name_stdout, OutputStream::Stdout, &line);
            }
            let mut h = stderr_handle.lock();
            let _ = match &prefix_stdout {
                Some(p) => writeln!(h, "[{p}] {line}"),
                None => writeln!(h, "{line}"),
            };
        }
    });

    let stderr_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let stderr_handle = std::io::stderr();
        for line in reader.lines().map_while(Result::ok) {
            if let Some(w) = &session_stderr {
                let _ = w.step_output(&step_name_stderr, OutputStream::Stderr, &line);
            }
            let mut h = stderr_handle.lock();
            let _ = match &prefix_stderr {
                Some(p) => writeln!(h, "[{p}] {line}"),
                None => writeln!(h, "{line}"),
            };
        }
    });

    let status = child
        .wait()
        .map_err(|e| ZigError::Execution(format!("failed to wait for child: {e}")))?;

    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    let captured = Arc::try_unwrap(buffer)
        .map_err(|_| ZigError::Execution("buffer still shared after threads joined".into()))?
        .into_inner()
        .map_err(|_| ZigError::Execution("output buffer poisoned".into()))?;

    Ok((status, captured))
}

/// Execute a single step by invoking `zag`, streaming its output live.
///
/// Returns the captured stdout from zag. The optional `model_override`
/// is used during retries to escalate to a different model. The optional
/// `prefix` tags streamed lines with the step name (used for parallel runs).
#[allow(clippy::too_many_arguments)]
fn execute_step(
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
    let args = build_zag_args(
        step,
        prompt,
        workflow_name,
        model_override,
        rendered_system_prompt,
        workflow_provider,
        workflow_model,
        extra_add_dirs,
    );

    if step.interactive {
        let status = run_zag_interactive(&args, &step.name)?;
        if !status.success() {
            return Err(ZigError::Execution(format!(
                "step '{}' failed (exit {})",
                step.name, status,
            )));
        }
        return Ok(String::new());
    }

    let (status, stdout) = run_zag_streaming(&args, &step.name, prefix, session)?;

    if !status.success() {
        return Err(ZigError::Execution(format!(
            "step '{}' failed (exit {})",
            step.name, status,
        )));
    }

    Ok(stdout)
}

/// Run a step with retry logic, returning its captured stdout on success.
///
/// Extracted so both sequential and parallel execution paths share the
/// same retry / model-escalation behavior.
#[allow(clippy::too_many_arguments)]
fn run_step_attempts(
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
        ) {
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

/// Spawn a step as a child process without waiting for it to finish.
fn spawn_step(
    step: &Step,
    prompt: &str,
    workflow_name: &str,
    rendered_system_prompt: Option<&str>,
    workflow_provider: Option<&str>,
    workflow_model: Option<&str>,
    extra_add_dirs: &[std::path::PathBuf],
) -> Result<std::process::Child, ZigError> {
    debug_assert!(
        !step.interactive,
        "spawn_step called for interactive step '{}' — validation should reject \
         interactive steps in parallel tiers and race groups",
        step.name
    );
    let args = build_zag_args(
        step,
        prompt,
        workflow_name,
        None,
        rendered_system_prompt,
        workflow_provider,
        workflow_model,
        extra_add_dirs,
    );
    let mut cmd = Command::new("zag");
    cmd.args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    cmd.spawn()
        .map_err(|e| ZigError::Zag(format!("failed to spawn zag for step '{}': {e}", step.name)))
}

/// Execute a race group: run all steps in parallel, return the first winner.
///
/// When one step finishes successfully, all remaining steps are killed.
/// Returns the winning step's name and its stdout output.
#[allow(clippy::too_many_arguments)]
fn execute_race_group(
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
    let mut children: Vec<(String, std::process::Child)> = Vec::new();

    for step in steps {
        let prompt = prompts.get(&step.name).ok_or_else(|| {
            ZigError::Execution(format!("missing prompt for step '{}'", step.name))
        })?;
        eprintln!("  racing step '{}'...", step.name);
        let rendered_sp = system_prompts.get(&step.name).map(|s| s.as_str());
        let empty: Vec<std::path::PathBuf> = Vec::new();
        let extra_dirs = storage_dirs.get(&step.name).unwrap_or(&empty);
        let child = spawn_step(
            step,
            prompt,
            workflow_name,
            rendered_sp,
            workflow_provider,
            workflow_model,
            extra_dirs,
        )?;
        children.push((step.name.clone(), child));
    }

    // Poll until one finishes
    loop {
        for i in 0..children.len() {
            let status = children[i]
                .1
                .try_wait()
                .map_err(|e| ZigError::Execution(format!("failed to poll child: {e}")))?;

            if let Some(exit_status) = status {
                let (winner_name, winner_child) = children.remove(i);

                // Kill remaining children
                for (name, mut child) in children {
                    eprintln!("  cancelling step '{name}' (race lost)");
                    let _ = child.kill();
                    let _ = child.wait();
                }

                let elapsed = race_started.elapsed().as_millis() as u64;
                if !exit_status.success() {
                    let stderr = winner_child
                        .stderr
                        .map(|mut s| {
                            let mut buf = String::new();
                            std::io::Read::read_to_string(&mut s, &mut buf).ok();
                            buf
                        })
                        .unwrap_or_default();
                    let err_msg = format!(
                        "race winner '{}' failed (exit {}): {}",
                        winner_name,
                        exit_status,
                        stderr.trim()
                    );
                    if let Some(w) = session {
                        let _ = w.step_failed(&winner_name, exit_status.code(), 1, &err_msg);
                    }
                    return Err(ZigError::Execution(err_msg));
                }

                let stdout = winner_child
                    .stdout
                    .map(|mut s| {
                        let mut buf = String::new();
                        std::io::Read::read_to_string(&mut s, &mut buf).ok();
                        buf
                    })
                    .unwrap_or_default();

                eprintln!("  race won by '{winner_name}'");
                if let Some(w) = session {
                    let _ = w.step_completed(&winner_name, 0, elapsed, Vec::new());
                }
                return Ok((winner_name, stdout));
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Execute a single sequential step with retry logic, saves, and next-jump handling.
#[allow(clippy::too_many_arguments)]
fn execute_sequential_step(
    step: &Step,
    vars: &mut HashMap<String, String>,
    user_prompt: Option<&str>,
    step_outputs: &mut HashMap<String, String>,
    workflow_name: &str,
    pending_next: &mut Option<String>,
    tier_index: usize,
    session: Option<&Arc<SessionWriter>>,
    roles: &HashMap<String, Role>,
    resources: &ResourceCollector,
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
    );

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
/// All non-skipped steps are spawned in their own threads and we wait for
/// every one to finish (unlike race groups, which kill losers). Output
/// lines from each step are streamed live to stderr, prefixed with the
/// step name to disambiguate. After completion, results are processed in
/// tier-declaration order so `saves`, `next`, and `on_failure` semantics
/// remain deterministic.
#[allow(clippy::too_many_arguments)]
fn execute_parallel_tier(
    steps: &[&Step],
    vars: &mut HashMap<String, String>,
    user_prompt: Option<&str>,
    step_outputs: &mut HashMap<String, String>,
    workflow_name: &str,
    pending_next: &mut Option<String>,
    tier_index: usize,
    session: Option<&Arc<SessionWriter>>,
    roles: &HashMap<String, Role>,
    resources: &ResourceCollector,
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
    let mut handles: Vec<thread::JoinHandle<(String, Result<String, ZigError>)>> = Vec::new();
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
        let handle = thread::spawn(move || {
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
            );
            (name, res)
        });
        handles.push(handle);
    }

    let mut results: HashMap<String, Result<String, ZigError>> = HashMap::new();
    for handle in handles {
        match handle.join() {
            Ok((name, res)) => {
                results.insert(name, res);
            }
            Err(_) => {
                return Err(ZigError::Execution("parallel step thread panicked".into()));
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
fn execute(
    workflow: &Workflow,
    workflow_path: &std::path::Path,
    user_prompt: Option<&str>,
    workflow_dir: &Path,
    disable_resources: bool,
    disable_memory: bool,
    disable_storage: bool,
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
    let storage_manager = if disable_storage || workflow.storage.is_empty() {
        StorageManager::empty()
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
                    )?;
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
                )?;
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
                ) {
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
