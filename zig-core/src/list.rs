use std::fs;
use std::path::{Path, PathBuf};

use crate::error::ZigError;
use crate::run::resolve_workflow_path;
use crate::workflow::parser;

/// Information about a discovered workflow file.
struct WorkflowInfo {
    path: PathBuf,
    name: String,
    description: String,
    steps: usize,
}

/// Discover all `.zug` workflow files under a base directory.
///
/// Searches `base/` for `.zug` files and also `base/workflows/` if it exists.
fn discover_workflows(base: &Path) -> Result<Vec<WorkflowInfo>, ZigError> {
    let mut workflows = Vec::new();

    let search_dirs = [base.to_path_buf(), base.join("workflows")];

    for dir in &search_dirs {
        if !dir.is_dir() {
            continue;
        }
        let entries = fs::read_dir(dir).map_err(|e| {
            ZigError::Io(format!("failed to read directory '{}': {e}", dir.display()))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "zug") {
                match parser::parse_file(&path) {
                    Ok(wf) => {
                        workflows.push(WorkflowInfo {
                            path,
                            name: wf.workflow.name,
                            description: wf.workflow.description,
                            steps: wf.steps.len(),
                        });
                    }
                    Err(_) => {
                        // Include unparseable files with minimal info
                        let name = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        workflows.push(WorkflowInfo {
                            path,
                            name,
                            description: "(parse error)".into(),
                            steps: 0,
                        });
                    }
                }
            }
        }
    }

    workflows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(workflows)
}

/// List all available workflows, printing a summary table.
pub fn run_list() -> Result<(), ZigError> {
    let workflows = discover_workflows(Path::new("."))?;

    if workflows.is_empty() {
        println!("No workflows found.");
        println!("  Looked in: ./ and ./workflows/");
        println!("  Create one with: zig workflow create");
        return Ok(());
    }

    let max_name = workflows
        .iter()
        .map(|w| w.name.len())
        .max()
        .unwrap_or(0)
        .max(4);
    let max_path = workflows
        .iter()
        .map(|w| w.path.display().to_string().len())
        .max()
        .unwrap_or(0)
        .max(4);

    println!(
        "{:<name_w$}  {:<path_w$}  {:>5}  DESCRIPTION",
        "NAME",
        "PATH",
        "STEPS",
        name_w = max_name,
        path_w = max_path,
    );

    for wf in &workflows {
        println!(
            "{:<name_w$}  {:<path_w$}  {:>5}  {}",
            wf.name,
            wf.path.display(),
            wf.steps,
            wf.description,
            name_w = max_name,
            path_w = max_path,
        );
    }

    Ok(())
}

/// Show detailed information about a single workflow.
pub fn run_show(workflow: &str) -> Result<(), ZigError> {
    let path = resolve_workflow_path(workflow)?;
    let wf = parser::parse_file(&path)?;

    println!("Workflow: {}", wf.workflow.name);
    println!("Path:     {}", path.display());
    if !wf.workflow.description.is_empty() {
        println!("Desc:     {}", wf.workflow.description);
    }
    if !wf.workflow.tags.is_empty() {
        println!("Tags:     {}", wf.workflow.tags.join(", "));
    }

    if !wf.vars.is_empty() {
        println!("\nVariables:");
        let mut var_names: Vec<&String> = wf.vars.keys().collect();
        var_names.sort();
        for name in var_names {
            let var = &wf.vars[name];
            let default_str = match &var.default {
                Some(v) => format!(" = {v}"),
                None => String::new(),
            };
            println!("  {name} ({}{default_str})", var.var_type);
            if !var.description.is_empty() {
                println!("    {}", var.description);
            }
        }
    }

    if !wf.steps.is_empty() {
        println!("\nSteps ({}):", wf.steps.len());
        for (i, step) in wf.steps.iter().enumerate() {
            let deps = if step.depends_on.is_empty() {
                String::new()
            } else {
                format!(" [depends on: {}]", step.depends_on.join(", "))
            };
            println!("  {}. {}{deps}", i + 1, step.name);
            if !step.description.is_empty() {
                println!("     {}", step.description);
            }
            if let Some(provider) = &step.provider {
                print!("     provider: {provider}");
                if let Some(model) = &step.model {
                    print!(", model: {model}");
                }
                println!();
            } else if let Some(model) = &step.model {
                println!("     model: {model}");
            }
            if let Some(condition) = &step.condition {
                println!("     condition: {condition}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;
