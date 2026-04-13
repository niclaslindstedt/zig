use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::ZigError;
use crate::run::resolve_workflow_path;
use crate::workflow::model::Workflow;
use crate::workflow::parser;

/// Summary information about a discovered workflow file.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowInfo {
    pub name: String,
    pub description: String,
    pub step_count: usize,
    pub path: String,
}

/// Return structured data about all discovered workflows.
pub fn get_workflow_list() -> Result<Vec<WorkflowInfo>, ZigError> {
    let mut entries = discover_zug_files(Path::new("."));

    if let Some(global_dir) = crate::paths::global_workflows_dir() {
        for f in discover_zug_files(&global_dir) {
            if !entries.iter().any(|e| e.file_name() == f.file_name()) {
                entries.push(f);
            }
        }
    }

    let mut infos = Vec::new();
    for path in &entries {
        let display = path.display().to_string();
        match parser::parse_file(path) {
            Ok(wf) => {
                infos.push(WorkflowInfo {
                    name: wf.workflow.name,
                    description: wf.workflow.description,
                    step_count: wf.steps.len(),
                    path: display,
                });
            }
            Err(_) => {
                infos.push(WorkflowInfo {
                    name: "(parse error)".to_string(),
                    description: String::new(),
                    step_count: 0,
                    path: display,
                });
            }
        }
    }

    Ok(infos)
}

/// Return the parsed workflow for a given workflow name or path.
pub fn get_workflow_detail(workflow: &str) -> Result<Workflow, ZigError> {
    let path = resolve_workflow_path(workflow)?;
    parser::parse_file(&path)
}

/// List all `.zug` workflow files found in the current directory, `./workflows/`,
/// and the global `~/.zig/workflows/` directory.
pub fn list_workflows() -> Result<(), ZigError> {
    let infos = get_workflow_list()?;

    if infos.is_empty() {
        println!("No workflows found.");
        println!("Hint: create one with `zig workflow create <name>`");
        return Ok(());
    }

    // Determine terminal width, default to 100 if unavailable.
    let term_width = terminal_width().unwrap_or(100);

    let name_w = infos.iter().map(|r| r.name.len()).max().unwrap_or(0).max(4);
    let steps_w = infos
        .iter()
        .map(|r| format_steps(r.step_count).len())
        .max()
        .unwrap_or(0)
        .max(5);

    // Reserve space for name, steps, separators, and a minimum path column,
    // then give the rest to description.
    let fixed = name_w + steps_w + 8; // 3 x 2-char gaps + 2 for padding
    let desc_w = if term_width > fixed + 20 {
        term_width - fixed - 20
    } else {
        30
    };
    let desc_w = desc_w.max(11);

    println!(
        "\x1b[1m{:<name_w$}\x1b[0m  {:<desc_w$}  {:<steps_w$}  PATH",
        "NAME", "DESCRIPTION", "STEPS"
    );
    println!(
        "{}  {}  {}  {}",
        "─".repeat(name_w),
        "─".repeat(desc_w),
        "─".repeat(steps_w),
        "─".repeat(4)
    );
    for info in &infos {
        let desc = truncate(&info.description, desc_w);
        let steps = format_steps(info.step_count);
        println!(
            "\x1b[1m{:<name_w$}\x1b[0m  {:<desc_w$}  {:<steps_w$}  {}",
            info.name, desc, steps, info.path
        );
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
    if let Some(ref version) = wf.workflow.version {
        println!("Version:     {version}");
    }
    if let Some(ref provider) = wf.workflow.provider {
        print!("Provider:    {provider}");
        if let Some(ref model) = wf.workflow.model {
            print!(" / {model}");
        }
        println!();
    } else if let Some(ref model) = wf.workflow.model {
        println!("Model:       {model}");
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

/// Truncate a string to `max` characters, appending "…" if truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max <= 1 {
        "…".to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

/// Format step count concisely.
fn format_steps(count: usize) -> String {
    if count == 1 {
        "1 step".to_string()
    } else {
        format!("{count} steps")
    }
}

/// Try to detect terminal width from the COLUMNS environment variable.
fn terminal_width() -> Option<usize> {
    std::env::var("COLUMNS").ok().and_then(|v| v.parse().ok())
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
