use std::collections::{HashMap, HashSet};

use crate::error::ZigError;
use crate::workflow::model::{FailurePolicy, Workflow};

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
