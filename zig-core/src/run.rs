use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::error::ZigError;
use crate::workflow::model::{FailurePolicy, Step, StepCommand, Workflow};
use crate::workflow::{parser, validate};

/// Maximum number of loop iterations to prevent infinite loops from `next` fields.
const MAX_LOOP_ITERATIONS: usize = 100;

/// Execute a `.zug` workflow file.
///
/// Parses the workflow, validates it, resolves the step DAG, and executes
/// each step by delegating to `zag`. The optional `user_prompt` is injected
/// as additional context into every step's prompt.
pub fn run_workflow(workflow_path: &str, user_prompt: Option<&str>) -> Result<(), ZigError> {
    check_zag()?;

    let path = resolve_workflow_path(workflow_path)?;
    let workflow = parser::parse_file(&path)?;

    if let Err(errors) = validate::validate(&workflow) {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        return Err(ZigError::Validation(msgs.join("; ")));
    }

    execute(&workflow, user_prompt)
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
/// 2. With `.zug` extension appended
/// 3. Under `./workflows/` directory
/// 4. Under `./workflows/` with `.zug` appended
pub fn resolve_workflow_path(workflow: &str) -> Result<PathBuf, ZigError> {
    let mut candidates = vec![
        PathBuf::from(workflow),
        PathBuf::from(format!("{workflow}.zug")),
        PathBuf::from(format!("workflows/{workflow}")),
        PathBuf::from(format!("workflows/{workflow}.zug")),
    ];

    if let Some(global_dir) = crate::paths::global_workflows_dir() {
        candidates.push(global_dir.join(workflow));
        candidates.push(global_dir.join(format!("{workflow}.zug")));
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
fn topological_sort(steps: &[Step]) -> Result<Vec<Vec<&Step>>, ZigError> {
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
fn build_zag_args(
    step: &Step,
    prompt: &str,
    workflow_name: &str,
    model_override: Option<&str>,
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
                a.extend(["-o".into(), output.clone()]);
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
        if let Some(provider) = &step.provider {
            args.extend(["--provider".into(), provider.clone()]);
        }

        let effective_model = model_override.or(step.model.as_deref());
        if let Some(model) = effective_model {
            args.extend(["--model".into(), model.to_string()]);
        }

        if let Some(system_prompt) = &step.system_prompt {
            args.extend(["--system-prompt".into(), system_prompt.clone()]);
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
            args.extend(["--mcp-config".into(), mcp_config.clone()]);
        }

        // Execution environment
        if step.auto_approve {
            args.push("--auto-approve".into());
        }
        if let Some(root) = &step.root {
            args.extend(["--root".into(), root.clone()]);
        }
        for dir in &step.add_dirs {
            args.extend(["--add-dir".into(), dir.clone()]);
        }
        for (key, value) in &step.env {
            args.extend(["--env".into(), format!("{key}={value}")]);
        }
        for file in &step.files {
            args.extend(["--file".into(), file.clone()]);
        }

        // Context injection
        for ctx in &step.context {
            args.extend(["--context".into(), ctx.clone()]);
        }
        if let Some(plan) = &step.plan {
            args.extend(["--plan".into(), plan.clone()]);
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

/// Spawn `zag` and stream its stdout/stderr live to our stderr while
/// also accumulating stdout into a buffer for `saves` extraction.
///
/// If `prefix` is `Some`, every emitted line is prefixed with `[prefix] `
/// — used to disambiguate output from steps running in parallel.
fn run_zag_streaming(
    args: &[String],
    step_name: &str,
    prefix: Option<&str>,
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

    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let stderr_handle = std::io::stderr();
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(mut buf) = buffer_clone.lock() {
                buf.push_str(&line);
                buf.push('\n');
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
fn execute_step(
    step: &Step,
    prompt: &str,
    workflow_name: &str,
    model_override: Option<&str>,
    prefix: Option<&str>,
) -> Result<String, ZigError> {
    let args = build_zag_args(step, prompt, workflow_name, model_override);
    let (status, stdout) = run_zag_streaming(&args, &step.name, prefix)?;

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
fn run_step_attempts(
    step: &Step,
    prompt: &str,
    workflow_name: &str,
    prefix: Option<&str>,
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
        match execute_step(step, prompt, workflow_name, model_override, prefix) {
            Ok(output) => return Ok(output),
            Err(e) => {
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
) -> Result<std::process::Child, ZigError> {
    let args = build_zag_args(step, prompt, workflow_name, None);
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
fn execute_race_group(
    steps: &[&Step],
    prompts: &HashMap<String, String>,
    workflow_name: &str,
) -> Result<(String, String), ZigError> {
    let mut children: Vec<(String, std::process::Child)> = Vec::new();

    for step in steps {
        let prompt = prompts.get(&step.name).ok_or_else(|| {
            ZigError::Execution(format!("missing prompt for step '{}'", step.name))
        })?;
        eprintln!("  racing step '{}'...", step.name);
        let child = spawn_step(step, prompt, workflow_name)?;
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

                if !exit_status.success() {
                    let stderr = winner_child
                        .stderr
                        .map(|mut s| {
                            let mut buf = String::new();
                            std::io::Read::read_to_string(&mut s, &mut buf).ok();
                            buf
                        })
                        .unwrap_or_default();
                    return Err(ZigError::Execution(format!(
                        "race winner '{}' failed (exit {}): {}",
                        winner_name,
                        exit_status,
                        stderr.trim()
                    )));
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
                return Ok((winner_name, stdout));
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Execute a single sequential step with retry logic, saves, and next-jump handling.
fn execute_sequential_step(
    step: &Step,
    vars: &mut HashMap<String, String>,
    user_prompt: Option<&str>,
    step_outputs: &mut HashMap<String, String>,
    workflow_name: &str,
    pending_next: &mut Option<String>,
) -> Result<(), ZigError> {
    if let Some(condition) = &step.condition {
        if !evaluate_condition(condition, vars)? {
            eprintln!(
                "  skipping '{}' (condition not met: {condition})",
                step.name
            );
            return Ok(());
        }
    }

    eprintln!("  running step '{}'...", step.name);

    let prompt = render_step_prompt(step, vars, user_prompt, step_outputs);
    let result = run_step_attempts(step, &prompt, workflow_name, None);

    match result {
        Ok(output) => {
            if !step.saves.is_empty() {
                let saved = extract_saves(&output, &step.saves)?;
                for (k, v) in &saved {
                    eprintln!("    saved {k} = {v}");
                }
                vars.extend(saved);
            }

            step_outputs.insert(step.name.clone(), output);
            eprintln!("  completed '{}'", step.name);

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
fn execute_parallel_tier(
    steps: &[&Step],
    vars: &mut HashMap<String, String>,
    user_prompt: Option<&str>,
    step_outputs: &mut HashMap<String, String>,
    workflow_name: &str,
    pending_next: &mut Option<String>,
) -> Result<(), ZigError> {
    // Evaluate conditions and render prompts up front, so threads receive
    // the same variable snapshot they would have under sequential execution.
    let mut active: Vec<&Step> = Vec::new();
    let mut prompts: HashMap<String, String> = HashMap::new();
    for step in steps {
        if let Some(condition) = &step.condition {
            if !evaluate_condition(condition, vars)? {
                eprintln!(
                    "  skipping '{}' (condition not met: {condition})",
                    step.name
                );
                continue;
            }
        }
        let prompt = render_step_prompt(step, vars, user_prompt, step_outputs);
        prompts.insert(step.name.clone(), prompt);
        active.push(*step);
    }

    if active.is_empty() {
        return Ok(());
    }

    eprintln!("  running {} steps in parallel...", active.len());

    let mut handles: Vec<thread::JoinHandle<(String, Result<String, ZigError>)>> = Vec::new();
    for step in &active {
        let step_clone: Step = (*step).clone();
        let prompt = prompts.remove(&step.name).unwrap_or_default();
        let workflow_name = workflow_name.to_string();
        let name = step.name.clone();
        eprintln!("  starting '{name}'...");
        let handle = thread::spawn(move || {
            let res = run_step_attempts(&step_clone, &prompt, &workflow_name, Some(&name));
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
        match res {
            Ok(output) => {
                if !step.saves.is_empty() {
                    let saved = extract_saves(&output, &step.saves)?;
                    for (k, v) in &saved {
                        eprintln!("    saved {k} = {v}");
                    }
                    vars.extend(saved);
                }
                step_outputs.insert(step.name.clone(), output);
                eprintln!("  completed '{}'", step.name);
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
fn execute(workflow: &Workflow, user_prompt: Option<&str>) -> Result<(), ZigError> {
    let mut vars = init_vars(workflow);

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

    let tiers = topological_sort(&workflow.steps)?;

    eprintln!(
        "running workflow '{}' ({} steps in {} tiers)",
        workflow.workflow.name,
        workflow.steps.len(),
        tiers.len()
    );

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

        for tier in &tiers_to_run {
            let (non_race, race_groups) = partition_tier(tier);

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
                )?;
            }

            // Run race groups in parallel
            for (group_name, race_steps) in &race_groups {
                eprintln!("  starting race group '{group_name}'...");

                // Build prompts for all racers (conditions evaluated here)
                let mut prompts = HashMap::new();
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
                    active_steps.push(step);
                }

                if active_steps.is_empty() {
                    continue;
                }

                match execute_race_group(&active_steps, &prompts, &workflow.workflow.name) {
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
    Ok(())
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
