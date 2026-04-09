use std::path::{Path, PathBuf};

use crate::error::ZigError;
use crate::run::resolve_workflow_path;
use crate::workflow::parser;

/// List all `.zug` workflow files found in the current directory, `./workflows/`,
/// and the global `~/.zig/workflows/` directory.
pub fn list_workflows() -> Result<(), ZigError> {
    let mut entries = discover_zug_files(Path::new("."));

    if let Some(global_dir) = crate::paths::global_workflows_dir() {
        for f in discover_zug_files(&global_dir) {
            if !entries.iter().any(|e| e.file_name() == f.file_name()) {
                entries.push(f);
            }
        }
    }

    if entries.is_empty() {
        println!("No workflows found.");
        println!("Hint: create one with `zig workflow create <name>`");
        return Ok(());
    }

    // Determine column widths
    let mut rows: Vec<(String, String, String, String)> = Vec::new();
    for path in &entries {
        let display = path.display().to_string();
        match parser::parse_file(path) {
            Ok(wf) => {
                let steps = format!("{} steps", wf.steps.len());
                rows.push((wf.workflow.name, wf.workflow.description, steps, display));
            }
            Err(_) => {
                rows.push((
                    "(parse error)".to_string(),
                    String::new(),
                    String::new(),
                    display,
                ));
            }
        }
    }

    let name_w = rows.iter().map(|r| r.0.len()).max().unwrap_or(0).max(4);
    let desc_w = rows.iter().map(|r| r.1.len()).max().unwrap_or(0).max(11);
    let steps_w = rows.iter().map(|r| r.2.len()).max().unwrap_or(0).max(5);

    println!(
        "{:<name_w$}  {:<desc_w$}  {:<steps_w$}  PATH",
        "NAME", "DESCRIPTION", "STEPS"
    );
    for (name, desc, steps, path) in &rows {
        println!("{name:<name_w$}  {desc:<desc_w$}  {steps:<steps_w$}  {path}");
    }

    Ok(())
}

/// Show detailed information about a workflow.
pub fn show_workflow(workflow: &str) -> Result<(), ZigError> {
    let path = resolve_workflow_path(workflow)?;
    let wf = parser::parse_file(&path)?;

    println!("Name:        {}", wf.workflow.name);
    println!("Path:        {}", path.display());
    if !wf.workflow.description.is_empty() {
        println!("Description: {}", wf.workflow.description);
    }
    if !wf.workflow.tags.is_empty() {
        println!("Tags:        {}", wf.workflow.tags.join(", "));
    }

    if !wf.vars.is_empty() {
        println!("\nVariables:");
        let mut vars: Vec<_> = wf.vars.iter().collect();
        vars.sort_by_key(|(name, _)| (*name).clone());
        for (name, var) in &vars {
            let default = match &var.default {
                Some(v) => format!(" = {v}"),
                None => String::new(),
            };
            println!("  {name}: {}{default}", var.var_type);
            if !var.description.is_empty() {
                println!("    {}", var.description);
            }
        }
    }

    if !wf.steps.is_empty() {
        println!("\nSteps ({}):", wf.steps.len());
        for (i, step) in wf.steps.iter().enumerate() {
            print!("  {}. {}", i + 1, step.name);
            if !step.depends_on.is_empty() {
                print!(" (depends on: {})", step.depends_on.join(", "));
            }
            println!();
            if !step.description.is_empty() {
                println!("     {}", step.description);
            }
            if let Some(condition) = &step.condition {
                println!("     condition: {condition}");
            }
            if let Some(provider) = &step.provider {
                print!("     provider: {provider}");
                if let Some(model) = &step.model {
                    print!(" / {model}");
                }
                println!();
            } else if let Some(model) = &step.model {
                println!("     model: {model}");
            }
        }
    }

    Ok(())
}

/// Delete a workflow file.
pub fn delete_workflow(workflow: &str) -> Result<(), ZigError> {
    let path = resolve_workflow_path(workflow)?;
    std::fs::remove_file(&path)
        .map_err(|e| ZigError::Io(format!("failed to delete {}: {e}", path.display())))?;
    println!("deleted {}", path.display());
    Ok(())
}

/// Discover all `.zug` files in a base directory and its `workflows/` subdirectory.
fn discover_zug_files(base: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    collect_zug_files(base, &mut files);
    collect_zug_files(&base.join("workflows"), &mut files);

    files.sort();
    files
}

/// Collect `.zug` files from a single directory into `out`.
fn collect_zug_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "zug") && path.is_file() {
                out.push(path);
            }
        }
    }
}

#[cfg(test)]
#[path = "manage_tests.rs"]
mod tests;
