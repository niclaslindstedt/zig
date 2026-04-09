use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::error::ZigError;
use crate::workflow::model::{FailurePolicy, Step, Workflow};
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
    let candidates = [
        PathBuf::from(workflow),
        PathBuf::from(format!("{workflow}.zug")),
        PathBuf::from(format!("workflows/{workflow}")),
        PathBuf::from(format!("workflows/{workflow}.zug")),
    ];

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

/// Execute a single step by invoking `zag run`.
///
/// Returns the captured stdout from zag.
fn execute_step(step: &Step, prompt: &str, workflow_name: &str) -> Result<String, ZigError> {
    let mut cmd = Command::new("zag");
    cmd.args(["run", prompt]);

    if let Some(provider) = &step.provider {
        cmd.args(["--provider", provider]);
    }
    if let Some(model) = &step.model {
        cmd.args(["--model", model]);
    }
    if let Some(system_prompt) = &step.system_prompt {
        cmd.args(["--system-prompt", system_prompt]);
    }
    if let Some(max_turns) = step.max_turns {
        cmd.args(["--max-turns", &max_turns.to_string()]);
    }
    if step.json {
        cmd.arg("--json");
    }

    let session_name = format!("zig-{}-{}", workflow_name, step.name);
    cmd.args(["--name", &session_name]);

    cmd.args(["--tag", "zig-workflow"]);
    for tag in &step.tags {
        cmd.args(["--tag", tag]);
    }

    if let Some(timeout) = &step.timeout {
        cmd.args(["--timeout", timeout]);
    }

    let output = cmd.output().map_err(|e| {
        ZigError::Zag(format!(
            "failed to launch zag for step '{}': {e}",
            step.name
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ZigError::Execution(format!(
            "step '{}' failed (exit {}): {}",
            step.name,
            output.status,
            stderr.trim()
        )));
    }

    String::from_utf8(output.stdout).map_err(|e| {
        ZigError::Execution(format!("step '{}' produced invalid UTF-8: {e}", step.name))
    })
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
            for step in tier {
                // Evaluate condition
                if let Some(condition) = &step.condition {
                    if !evaluate_condition(condition, &vars)? {
                        eprintln!(
                            "  skipping '{}' (condition not met: {condition})",
                            step.name
                        );
                        continue;
                    }
                }

                eprintln!("  running step '{}'...", step.name);

                let prompt = render_step_prompt(step, &vars, user_prompt, &step_outputs);

                let mut attempts = 0;
                let max_attempts = if step.on_failure.as_ref() == Some(&FailurePolicy::Retry) {
                    step.max_retries.unwrap_or(1) + 1
                } else {
                    1
                };

                let result = loop {
                    attempts += 1;
                    match execute_step(step, &prompt, &workflow.workflow.name) {
                        Ok(output) => break Ok(output),
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
                            break Err(e);
                        }
                    }
                };

                match result {
                    Ok(output) => {
                        // Extract saves
                        if !step.saves.is_empty() {
                            let saved = extract_saves(&output, &step.saves)?;
                            for (k, v) in &saved {
                                eprintln!("    saved {k} = {v}");
                            }
                            vars.extend(saved);
                        }

                        step_outputs.insert(step.name.clone(), output);
                        eprintln!("  completed '{}'", step.name);

                        // Handle `next` jump
                        if step.next.is_some() {
                            pending_next = step.next.clone();
                        }
                    }
                    Err(e) => match step.on_failure.as_ref().unwrap_or(&FailurePolicy::Fail) {
                        FailurePolicy::Fail => return Err(e),
                        FailurePolicy::Continue => {
                            eprintln!("  step '{}' failed (continuing): {e}", step.name);
                        }
                        FailurePolicy::Retry => {
                            // All retries exhausted
                            return Err(e);
                        }
                    },
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
