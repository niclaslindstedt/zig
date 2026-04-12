use std::collections::{HashMap, HashSet};

use regex::Regex;

use crate::error::ZigError;
use crate::workflow::model::{FailurePolicy, StepCommand, VarType, Variable, Workflow};

/// Validate a parsed workflow for structural correctness.
///
/// Checks:
/// - At least one step exists
/// - Step names are unique
/// - `depends_on` references exist
/// - No dependency cycles
/// - `next` references exist
/// - Variable references in prompts refer to declared variables
/// - `saves` variable names are declared
/// - Condition variable references are declared
pub fn validate(workflow: &Workflow) -> Result<(), Vec<ZigError>> {
    let mut errors = Vec::new();

    if workflow.steps.is_empty() {
        errors.push(ZigError::Validation(
            "workflow must have at least one step".into(),
        ));
        return Err(errors);
    }

    let step_names: HashSet<&str> = workflow.steps.iter().map(|s| s.name.as_str()).collect();
    let var_names: HashSet<&str> = workflow.vars.keys().map(|k| k.as_str()).collect();
    let role_names: HashSet<&str> = workflow.roles.keys().map(|k| k.as_str()).collect();

    // Check unique step names
    let mut seen_names = HashSet::new();
    for step in &workflow.steps {
        if !seen_names.insert(&step.name) {
            errors.push(ZigError::Validation(format!(
                "duplicate step name: '{}'",
                step.name
            )));
        }
    }

    for step in &workflow.steps {
        // Check depends_on references
        for dep in &step.depends_on {
            if !step_names.contains(dep.as_str()) {
                errors.push(ZigError::Validation(format!(
                    "step '{}' depends on unknown step '{dep}'",
                    step.name
                )));
            }
            if dep == &step.name {
                errors.push(ZigError::Validation(format!(
                    "step '{}' depends on itself",
                    step.name
                )));
            }
        }

        // Check next references
        if let Some(next) = &step.next {
            if !step_names.contains(next.as_str()) {
                errors.push(ZigError::Validation(format!(
                    "step '{}' references unknown next step '{next}'",
                    step.name
                )));
            }
        }

        // Check variable references in prompt
        for var_ref in extract_var_refs(&step.prompt) {
            if !var_names.contains(var_ref.as_str()) {
                errors.push(ZigError::Validation(format!(
                    "step '{}' prompt references unknown variable '${{{var_ref}}}'",
                    step.name
                )));
            }
        }

        // Check variable references in system_prompt
        if let Some(system_prompt) = &step.system_prompt {
            for var_ref in extract_var_refs(system_prompt) {
                if !var_names.contains(var_ref.as_str()) {
                    errors.push(ZigError::Validation(format!(
                        "step '{}' system_prompt references unknown variable '${{{var_ref}}}'",
                        step.name
                    )));
                }
            }
        }

        // Check role and system_prompt are mutually exclusive
        if step.role.is_some() && step.system_prompt.is_some() {
            errors.push(ZigError::Validation(format!(
                "step '{}' sets both 'role' and 'system_prompt' (they are mutually exclusive)",
                step.name
            )));
        }

        // Check role references
        if let Some(role_ref) = &step.role {
            let var_refs = extract_var_refs(role_ref);
            if var_refs.is_empty() {
                // Static role reference — must exist in [roles]
                if !role_names.contains(role_ref.as_str()) {
                    errors.push(ZigError::Validation(format!(
                        "step '{}' role references unknown role '{role_ref}'",
                        step.name
                    )));
                }
            } else {
                // Dynamic role reference — validate variable refs
                for var_ref in var_refs {
                    if !var_names.contains(var_ref.as_str()) {
                        errors.push(ZigError::Validation(format!(
                            "step '{}' role references unknown variable '${{{var_ref}}}'",
                            step.name
                        )));
                    }
                }
            }
        }

        // Check saves reference declared variables
        for var_name in step.saves.keys() {
            if !var_names.contains(var_name.as_str()) {
                errors.push(ZigError::Validation(format!(
                    "step '{}' saves to unknown variable '{var_name}'",
                    step.name
                )));
            }
        }

        // Check condition variable references
        if let Some(cond) = &step.condition {
            for var_ref in extract_condition_vars(cond) {
                if !var_names.contains(var_ref.as_str()) && !step_names.contains(var_ref.as_str()) {
                    errors.push(ZigError::Validation(format!(
                        "step '{}' condition references unknown variable '{var_ref}'",
                        step.name
                    )));
                }
            }
        }

        // Check retry_model requires on_failure = "retry"
        if step.retry_model.is_some() && step.on_failure.as_ref() != Some(&FailurePolicy::Retry) {
            errors.push(ZigError::Validation(format!(
                "step '{}' sets retry_model but on_failure is not 'retry'",
                step.name
            )));
        }

        // Check mcp_config requires claude provider (or no provider specified).
        // The effective provider considers both step-level and workflow-level defaults.
        if step.mcp_config.is_some() {
            let effective_provider = step
                .provider
                .as_ref()
                .or(workflow.workflow.provider.as_ref());
            if let Some(provider) = effective_provider {
                if provider != "claude" {
                    errors.push(ZigError::Validation(format!(
                        "step '{}' sets mcp_config but provider is '{}' \
                         (mcp_config is only supported with the claude provider)",
                        step.name, provider
                    )));
                }
            }
        }

        // Check output format is a valid value
        if let Some(ref output) = step.output {
            let valid_formats = ["text", "json", "json-pretty", "stream-json", "native-json"];
            if !valid_formats.contains(&output.as_str()) {
                errors.push(ZigError::Validation(format!(
                    "step '{}' has invalid output format '{}' \
                     (must be one of: text, json, json-pretty, stream-json, native-json)",
                    step.name, output
                )));
            }
        }

        // Check review-only fields require command = "review"
        let is_review = step.command.as_ref() == Some(&StepCommand::Review);
        if !is_review {
            for (field, set) in [("uncommitted", step.uncommitted)] {
                if set {
                    errors.push(ZigError::Validation(format!(
                        "step '{}' sets '{}' but command is not 'review'",
                        step.name, field
                    )));
                }
            }
            for (field, set) in [
                ("base", step.base.is_some()),
                ("commit", step.commit.is_some()),
                ("title", step.title.is_some()),
            ] {
                if set {
                    errors.push(ZigError::Validation(format!(
                        "step '{}' sets '{}' but command is not 'review'",
                        step.name, field
                    )));
                }
            }
        }

        // Check plan-only fields require command = "plan"
        let is_plan = step.command.as_ref() == Some(&StepCommand::Plan);
        if !is_plan {
            for (field, set) in [
                ("plan_output", step.plan_output.is_some()),
                ("instructions", step.instructions.is_some()),
            ] {
                if set {
                    errors.push(ZigError::Validation(format!(
                        "step '{}' sets '{}' but command is not 'plan'",
                        step.name, field
                    )));
                }
            }
        }

        // Check pipe/collect/summary require depends_on
        if let Some(ref cmd) = step.command {
            match cmd {
                StepCommand::Pipe | StepCommand::Collect | StepCommand::Summary => {
                    if step.depends_on.is_empty() {
                        errors.push(ZigError::Validation(format!(
                            "step '{}' uses command '{}' but has no depends_on \
                             (pipe/collect/summary operate on prior session outputs)",
                            step.name,
                            match cmd {
                                StepCommand::Pipe => "pipe",
                                StepCommand::Collect => "collect",
                                StepCommand::Summary => "summary",
                                _ => unreachable!(),
                            }
                        )));
                    }
                }
                _ => {}
            }
        }
    }

    // Check role definitions
    for (role_name, role) in &workflow.roles {
        // system_prompt and system_prompt_file are mutually exclusive
        if role.system_prompt.is_some() && role.system_prompt_file.is_some() {
            errors.push(ZigError::Validation(format!(
                "role '{role_name}' sets both 'system_prompt' and 'system_prompt_file' \
                 (they are mutually exclusive)"
            )));
        }

        // Validate ${var} references in role system_prompt
        if let Some(ref sp) = role.system_prompt {
            for var_ref in extract_var_refs(sp) {
                if !var_names.contains(var_ref.as_str()) {
                    errors.push(ZigError::Validation(format!(
                        "role '{role_name}' system_prompt references unknown variable \
                         '${{{var_ref}}}'"
                    )));
                }
            }
        }
    }

    // Check race_group: steps in the same group must not depend on each other
    let mut race_groups: HashMap<&str, Vec<&str>> = HashMap::new();
    for step in &workflow.steps {
        if let Some(ref group) = step.race_group {
            race_groups
                .entry(group.as_str())
                .or_default()
                .push(step.name.as_str());
        }
    }
    for (group, members) in &race_groups {
        let member_set: HashSet<&str> = members.iter().copied().collect();
        for step in &workflow.steps {
            if step.race_group.as_deref() == Some(*group) {
                for dep in &step.depends_on {
                    if member_set.contains(dep.as_str()) {
                        errors.push(ZigError::Validation(format!(
                            "step '{}' depends on '{}' but both are in race_group '{}' \
                             (race members must not depend on each other)",
                            step.name, dep, group
                        )));
                    }
                }
            }
        }
    }

    // Check variable constraints
    validate_var_constraints(&workflow.vars, &mut errors);

    // Check for dependency cycles
    if let Some(cycle) = detect_cycle(&workflow.steps) {
        errors.push(ZigError::Validation(format!(
            "dependency cycle detected: {}",
            cycle.join(" -> ")
        )));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate variable constraint declarations for structural correctness.
fn validate_var_constraints(vars: &HashMap<String, Variable>, errors: &mut Vec<ZigError>) {
    let mut prompt_bound_count = 0;

    for (name, var) in vars {
        // default and default_file are mutually exclusive
        if var.default.is_some() && var.default_file.is_some() {
            errors.push(ZigError::Validation(format!(
                "variable '{name}' sets both 'default' and 'default_file' \
                 (they are mutually exclusive)"
            )));
        }

        // Validate `from` field
        if let Some(ref from) = var.from {
            if from != "prompt" {
                errors.push(ZigError::Validation(format!(
                    "variable '{name}' has unsupported from value '{from}' (only 'prompt' is supported)"
                )));
            } else {
                prompt_bound_count += 1;
            }
        }

        // String-only constraints on non-string types
        if var.var_type != VarType::String {
            if var.min_length.is_some() {
                errors.push(ZigError::Validation(format!(
                    "variable '{name}' has min_length but type is '{}' (only valid for 'string')",
                    var.var_type
                )));
            }
            if var.max_length.is_some() {
                errors.push(ZigError::Validation(format!(
                    "variable '{name}' has max_length but type is '{}' (only valid for 'string')",
                    var.var_type
                )));
            }
            if var.pattern.is_some() {
                errors.push(ZigError::Validation(format!(
                    "variable '{name}' has pattern but type is '{}' (only valid for 'string')",
                    var.var_type
                )));
            }
        }

        // Number-only constraints on non-number types
        if var.var_type != VarType::Number {
            if var.min.is_some() {
                errors.push(ZigError::Validation(format!(
                    "variable '{name}' has min but type is '{}' (only valid for 'number')",
                    var.var_type
                )));
            }
            if var.max.is_some() {
                errors.push(ZigError::Validation(format!(
                    "variable '{name}' has max but type is '{}' (only valid for 'number')",
                    var.var_type
                )));
            }
        }

        // Range consistency
        if let (Some(min_len), Some(max_len)) = (var.min_length, var.max_length) {
            if min_len > max_len {
                errors.push(ZigError::Validation(format!(
                    "variable '{name}' has min_length ({min_len}) greater than max_length ({max_len})"
                )));
            }
        }
        if let (Some(min), Some(max)) = (var.min, var.max) {
            if min > max {
                errors.push(ZigError::Validation(format!(
                    "variable '{name}' has min ({min}) greater than max ({max})"
                )));
            }
        }

        // Validate pattern compiles
        if let Some(ref pattern) = var.pattern {
            if Regex::new(pattern).is_err() {
                errors.push(ZigError::Validation(format!(
                    "variable '{name}' has invalid regex pattern: '{pattern}'"
                )));
            }
        }

        // Validate allowed_values type compatibility
        if let Some(ref allowed) = var.allowed_values {
            for val in allowed {
                let ok = match var.var_type {
                    VarType::String => val.is_str(),
                    VarType::Number => val.is_integer() || val.is_float(),
                    VarType::Bool => matches!(val, toml::Value::Boolean(_)),
                    VarType::Json => true,
                };
                if !ok {
                    errors.push(ZigError::Validation(format!(
                        "variable '{name}' has allowed_values entry {val} incompatible with type '{}'",
                        var.var_type
                    )));
                }
            }
        }

        // Validate default satisfies constraints
        if let Some(ref default) = var.default {
            let default_str = toml_value_to_string(default);
            let constraint_errors = check_value_constraints(name, &default_str, var);
            for msg in constraint_errors {
                errors.push(ZigError::Validation(format!(
                    "variable '{name}' default value violates constraint: {msg}"
                )));
            }
        }
    }

    if prompt_bound_count > 1 {
        errors.push(ZigError::Validation(
            "multiple variables have from = \"prompt\" (only one is allowed)".into(),
        ));
    }
}

/// Convert a TOML value to its string representation for constraint checking.
fn toml_value_to_string(val: &toml::Value) -> String {
    match val {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// Check a single value against a variable's constraints.
/// Returns a list of human-readable violation messages (empty if valid).
fn check_value_constraints(name: &str, value: &str, var: &Variable) -> Vec<String> {
    let mut violations = Vec::new();

    if var.required && value.is_empty() {
        violations.push(format!(
            "variable '{name}' is required but was not provided"
        ));
    }

    // Skip further checks for empty non-required values
    if value.is_empty() && !var.required {
        return violations;
    }

    if let Some(min_len) = var.min_length {
        let len = value.len() as u32;
        if len < min_len {
            violations.push(format!(
                "variable '{name}' must be at least {min_len} characters (got {len})"
            ));
        }
    }

    if let Some(max_len) = var.max_length {
        let len = value.len() as u32;
        if len > max_len {
            violations.push(format!(
                "variable '{name}' must be at most {max_len} characters (got {len})"
            ));
        }
    }

    if let Some(min) = var.min {
        if let Ok(num) = value.parse::<f64>() {
            if num < min {
                violations.push(format!(
                    "variable '{name}' must be at least {min} (got {num})"
                ));
            }
        }
    }

    if let Some(max) = var.max {
        if let Ok(num) = value.parse::<f64>() {
            if num > max {
                violations.push(format!(
                    "variable '{name}' must be at most {max} (got {num})"
                ));
            }
        }
    }

    if let Some(ref pattern) = var.pattern {
        if let Ok(re) = Regex::new(pattern) {
            if !re.is_match(value) {
                violations.push(format!("variable '{name}' must match pattern '{pattern}'"));
            }
        }
    }

    if let Some(ref allowed) = var.allowed_values {
        let allowed_strs: Vec<String> = allowed.iter().map(toml_value_to_string).collect();
        if !allowed_strs.iter().any(|a| a == value) {
            violations.push(format!(
                "variable '{name}' must be one of: {}",
                allowed_strs.join(", ")
            ));
        }
    }

    violations
}

/// Validate variable values against their declared constraints at runtime.
///
/// Called after `init_vars` and prompt binding, before step execution begins.
pub fn validate_var_values(
    vars: &HashMap<String, String>,
    declarations: &HashMap<String, Variable>,
) -> Result<(), Vec<ZigError>> {
    let mut errors = Vec::new();

    for (name, decl) in declarations {
        let value = vars.get(name).map(|s| s.as_str()).unwrap_or("");
        let violations = check_value_constraints(name, value, decl);
        for msg in violations {
            errors.push(ZigError::Validation(msg));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Extract `${var_name}` references from a prompt template.
fn extract_var_refs(template: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        let after_start = &rest[start + 2..];
        if let Some(end) = after_start.find('}') {
            let var_name = &after_start[..end];
            // Support dotted paths like ${quality.score} — take the root variable
            let root = var_name.split('.').next().unwrap_or(var_name);
            refs.push(root.to_string());
            rest = &after_start[end + 1..];
        } else {
            break;
        }
    }
    refs
}

/// Extract variable names from a condition expression.
///
/// Simple heuristic: split on whitespace and operators, keep identifiers
/// that are not numeric literals, string literals, or comparison operators.
fn extract_condition_vars(condition: &str) -> Vec<String> {
    let operators = ["==", "!=", "<", ">", "<=", ">=", "&&", "||", "!"];
    let keywords = ["true", "false"];

    condition
        .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
        .filter(|token| {
            !token.is_empty()
                && !operators.contains(token)
                && !keywords.contains(token)
                && !token.starts_with('"')
                && !token.starts_with('\'')
                && token.parse::<f64>().is_err()
        })
        .map(|token| {
            // Handle dotted paths: score.value → score
            token.split('.').next().unwrap_or(token).to_string()
        })
        .collect()
}

/// Detect cycles in the step dependency graph using DFS.
/// Returns the cycle path if found, or None.
fn detect_cycle(steps: &[crate::workflow::model::Step]) -> Option<Vec<String>> {
    let adjacency: HashMap<&str, Vec<&str>> = steps
        .iter()
        .map(|s| {
            (
                s.name.as_str(),
                s.depends_on.iter().map(|d| d.as_str()).collect(),
            )
        })
        .collect();

    let mut visited = HashSet::new();
    let mut in_stack = HashSet::new();
    let mut path = Vec::new();

    for step in steps {
        if !visited.contains(step.name.as_str())
            && dfs_cycle(
                step.name.as_str(),
                &adjacency,
                &mut visited,
                &mut in_stack,
                &mut path,
            )
        {
            return Some(path);
        }
    }
    None
}

fn dfs_cycle<'a>(
    node: &'a str,
    adjacency: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>,
    in_stack: &mut HashSet<&'a str>,
    path: &mut Vec<String>,
) -> bool {
    visited.insert(node);
    in_stack.insert(node);
    path.push(node.to_string());

    if let Some(neighbors) = adjacency.get(node) {
        for &neighbor in neighbors {
            if !visited.contains(neighbor) {
                if dfs_cycle(neighbor, adjacency, visited, in_stack, path) {
                    return true;
                }
            } else if in_stack.contains(neighbor) {
                path.push(neighbor.to_string());
                return true;
            }
        }
    }

    in_stack.remove(node);
    path.pop();
    false
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;
