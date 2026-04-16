//! Management commands for resource files: list / add / delete / show / where.
//!
//! These functions back the `zig resources …` subcommands. They operate on the
//! same tiered layout that [`crate::resources::ResourceCollector`] consumes at
//! run time:
//!
//! * `~/.zig/resources/_shared/` — the global shared tier
//! * `~/.zig/resources/<workflow>/` — the global per-workflow tier
//! * `<git-root>/.zig/resources/` — the project (cwd) tier
//!
//! Inline resources declared in `.zwf` files are *not* manipulated by these
//! commands — they live inside the workflow file itself and the user edits
//! them by hand.

use std::path::{Path, PathBuf};

use crate::error::ZigError;
use crate::paths;

/// Reject resource names that include path separators or traversal
/// segments. API/CLI callers must only supply plain filenames — otherwise
/// `dir.join(name)` lets `../../` escape the tier directory.
fn validate_resource_filename(name: &str) -> Result<(), ZigError> {
    if name.is_empty() {
        return Err(ZigError::Validation("name must not be empty".into()));
    }
    if name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name == "."
        || name == ".."
        || name.starts_with('-')
    {
        return Err(ZigError::Validation(format!(
            "name '{name}' must not contain path separators or traversal segments"
        )));
    }
    Ok(())
}

/// Which tier(s) a `list` or `where` command should consider.
///
/// `Both` (the default when neither `--global` nor `--cwd` is passed) walks
/// every tier that exists on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceScope {
    /// Show only the global tiers under `~/.zig/resources/`.
    Global,
    /// Show only the project tier under `<git-root>/.zig/resources/`.
    Cwd,
    /// Show every tier (the default).
    Both,
}

impl ResourceScope {
    /// Build a scope from the mutually-exclusive `--global` / `--cwd` flags.
    pub fn from_flags(global: bool, cwd: bool) -> Self {
        match (global, cwd) {
            (true, false) => ResourceScope::Global,
            (false, true) => ResourceScope::Cwd,
            _ => ResourceScope::Both,
        }
    }
}

/// Where an `add` or `delete` command should place / look for a resource.
///
/// Constructed with [`ResourceTarget::from_flags`], which enforces the
/// mutually-exclusive flag rules (a target must be unambiguous).
#[derive(Debug, Clone)]
pub enum ResourceTarget {
    /// `~/.zig/resources/_shared/`
    GlobalShared,
    /// `~/.zig/resources/<workflow>/`
    GlobalWorkflow(String),
    /// `<git-root>/.zig/resources/` — falls back to `./.zig/resources/` when
    /// not inside a git repo.
    Cwd,
}

impl ResourceTarget {
    /// Resolve a target from the CLI flag combination passed by the user.
    ///
    /// Rules:
    /// * `--workflow <name>` always means the global per-workflow tier.
    /// * `--cwd` means the project tier.
    /// * `--global` (without `--workflow`) means the global shared tier.
    /// * No flags at all is treated as `--cwd` (the most local tier).
    pub fn from_flags(workflow: Option<&str>, global: bool, cwd: bool) -> Result<Self, ZigError> {
        if let Some(name) = workflow {
            if cwd {
                return Err(ZigError::Validation(
                    "--workflow cannot be combined with --cwd".into(),
                ));
            }
            return Ok(ResourceTarget::GlobalWorkflow(name.to_string()));
        }
        if cwd {
            return Ok(ResourceTarget::Cwd);
        }
        if global {
            return Ok(ResourceTarget::GlobalShared);
        }
        Ok(ResourceTarget::Cwd)
    }

    /// Resolve to an absolute directory path, creating it if it doesn't exist.
    pub fn ensure_dir(&self) -> Result<PathBuf, ZigError> {
        let dir = match self {
            ResourceTarget::GlobalShared => paths::ensure_global_resources_dir(Some("_shared"))?,
            ResourceTarget::GlobalWorkflow(name) => paths::ensure_global_resources_dir(Some(name))?,
            ResourceTarget::Cwd => ensure_cwd_resources_dir()?,
        };
        Ok(dir)
    }

    /// Resolve to an absolute directory path *without* creating it. Returns
    /// `None` when the directory cannot be derived (e.g. `$HOME` unset).
    pub fn existing_dir(&self) -> Option<PathBuf> {
        match self {
            ResourceTarget::GlobalShared => paths::global_shared_resources_dir(),
            ResourceTarget::GlobalWorkflow(name) => paths::global_resources_for(name),
            ResourceTarget::Cwd => paths::cwd_resources_dir().or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.join(".zig").join("resources"))
            }),
        }
    }

    /// Short label for diagnostic messages.
    pub fn label(&self) -> String {
        match self {
            ResourceTarget::GlobalShared => "global:_shared".to_string(),
            ResourceTarget::GlobalWorkflow(n) => format!("global:{n}"),
            ResourceTarget::Cwd => "cwd".to_string(),
        }
    }
}

fn ensure_cwd_resources_dir() -> Result<PathBuf, ZigError> {
    if let Some(existing) = paths::cwd_resources_dir() {
        return Ok(existing);
    }
    let cwd = std::env::current_dir()
        .map_err(|e| ZigError::Io(format!("failed to read current directory: {e}")))?;
    let dir = cwd.join(".zig").join("resources");
    std::fs::create_dir_all(&dir)
        .map_err(|e| ZigError::Io(format!("failed to create {}: {e}", dir.display())))?;
    Ok(dir)
}

/// A single resource entry returned by [`list_resources`].
#[derive(Debug, Clone)]
pub struct ListedResource {
    pub tier: String,
    pub name: String,
    pub path: PathBuf,
}

/// List resources discovered under the requested scope.
///
/// Prints a human-readable table to stdout. When `workflow` is `Some`, the
/// global tier is restricted to that workflow's directory; otherwise every
/// `~/.zig/resources/<name>/` subdirectory is walked.
pub fn list_resources(workflow: Option<&str>, scope: ResourceScope) -> Result<(), ZigError> {
    let mut entries: Vec<ListedResource> = Vec::new();

    let walk_global = matches!(scope, ResourceScope::Global | ResourceScope::Both);
    let walk_cwd = matches!(scope, ResourceScope::Cwd | ResourceScope::Both);

    if walk_global {
        if let Some(shared) = paths::global_shared_resources_dir() {
            collect_listing(&shared, "global:_shared", &mut entries);
        }
        if let Some(name) = workflow {
            if let Some(wf_dir) = paths::global_resources_for(name) {
                collect_listing(&wf_dir, &format!("global:{name}"), &mut entries);
            }
        } else if let Some(root) = paths::global_resources_dir() {
            // Walk every immediate subdirectory of ~/.zig/resources/ (skipping
            // _shared which we already covered) and treat each one as a
            // workflow-scoped tier.
            if let Ok(read) = std::fs::read_dir(&root) {
                for entry in read.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let name = match path.file_name().and_then(|n| n.to_str()) {
                        Some(n) => n,
                        None => continue,
                    };
                    if name == "_shared" {
                        continue;
                    }
                    collect_listing(&path, &format!("global:{name}"), &mut entries);
                }
            }
        }
    }

    if walk_cwd {
        if let Some(cwd_dir) = paths::cwd_resources_dir() {
            collect_listing(&cwd_dir, "cwd", &mut entries);
        }
    }

    if entries.is_empty() {
        println!("No resources found.");
        println!(
            "Hint: add one with `zig resources add <file> [--global|--cwd|--workflow <name>]`"
        );
        return Ok(());
    }

    let tier_w = entries
        .iter()
        .map(|e| e.tier.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let name_w = entries
        .iter()
        .map(|e| e.name.len())
        .max()
        .unwrap_or(4)
        .max(4);

    println!(
        "{:<tier_w$}  {:<name_w$}  PATH",
        "TIER",
        "NAME",
        tier_w = tier_w,
        name_w = name_w,
    );
    for e in &entries {
        println!(
            "{:<tier_w$}  {:<name_w$}  {}",
            e.tier,
            e.name,
            e.path.display(),
            tier_w = tier_w,
            name_w = name_w,
        );
    }

    Ok(())
}

fn collect_listing(dir: &Path, tier: &str, out: &mut Vec<ListedResource>) {
    if !dir.is_dir() {
        return;
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let read = match std::fs::read_dir(&current) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for entry in read.flatten() {
            let path = entry.path();
            let metadata = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            out.push(ListedResource {
                tier: tier.to_string(),
                name,
                path: path.clone(),
            });
        }
    }
}

/// Copy a file into the chosen tier directory, optionally renaming it.
///
/// Returns the absolute path of the destination file. Refuses to overwrite an
/// existing file (the user must `zig resources delete <name>` first).
pub fn add_resource(
    file: &str,
    target: ResourceTarget,
    name: Option<&str>,
) -> Result<PathBuf, ZigError> {
    let src = Path::new(file);
    if !src.exists() {
        return Err(ZigError::Io(format!("source file not found: {file}")));
    }
    if !src.is_file() {
        return Err(ZigError::Io(format!("not a regular file: {file}")));
    }

    let dir = target.ensure_dir()?;
    let dest = add_to_dir(src, &dir, name)?;
    println!(
        "added resource '{}' to {} ({})",
        dest.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default(),
        target.label(),
        dest.display()
    );
    Ok(dest)
}

/// Lower-level helper: copy `src` into `dir`, optionally renaming it.
///
/// Refuses to overwrite. Used internally by [`add_resource`] and exposed for
/// tests that want to operate on an explicit directory without touching the
/// global `$HOME` layout.
pub fn add_to_dir(src: &Path, dir: &Path, name: Option<&str>) -> Result<PathBuf, ZigError> {
    if !src.exists() {
        return Err(ZigError::Io(format!(
            "source file not found: {}",
            src.display()
        )));
    }
    if !src.is_file() {
        return Err(ZigError::Io(format!(
            "not a regular file: {}",
            src.display()
        )));
    }

    if !dir.exists() {
        std::fs::create_dir_all(dir)
            .map_err(|e| ZigError::Io(format!("failed to create {}: {e}", dir.display())))?;
    }

    let dest_name = name
        .map(str::to_string)
        .or_else(|| src.file_name().map(|n| n.to_string_lossy().into_owned()))
        .ok_or_else(|| {
            ZigError::Io(format!(
                "could not derive a destination name from {}",
                src.display()
            ))
        })?;
    validate_resource_filename(&dest_name)?;

    let dest = dir.join(&dest_name);
    if dest.exists() {
        return Err(ZigError::Io(format!(
            "resource '{}' already exists at {} — delete it first",
            dest_name,
            dest.display()
        )));
    }

    std::fs::copy(src, &dest).map_err(|e| {
        ZigError::Io(format!(
            "failed to copy {} → {}: {e}",
            src.display(),
            dest.display()
        ))
    })?;
    Ok(dest)
}

/// Delete a resource by name from the chosen tier.
pub fn delete_resource(name: &str, target: ResourceTarget) -> Result<(), ZigError> {
    let dir = target
        .existing_dir()
        .ok_or_else(|| ZigError::Io("could not resolve target directory (HOME unset?)".into()))?;
    let path = delete_from_dir(name, &dir)?;
    println!(
        "deleted resource '{}' from {} ({})",
        name,
        target.label(),
        path.display()
    );
    Ok(())
}

/// Lower-level helper: delete a single resource from an explicit directory.
pub fn delete_from_dir(name: &str, dir: &Path) -> Result<PathBuf, ZigError> {
    validate_resource_filename(name)?;
    if !dir.is_dir() {
        return Err(ZigError::Io(format!(
            "tier directory does not exist: {}",
            dir.display()
        )));
    }
    let path = dir.join(name);
    if !path.exists() {
        return Err(ZigError::Io(format!(
            "resource '{}' not found in {}",
            name,
            dir.display()
        )));
    }
    std::fs::remove_file(&path)
        .map_err(|e| ZigError::Io(format!("failed to delete {}: {e}", path.display())))?;
    Ok(path)
}

/// Print the absolute path and contents of a resource discovered in any tier.
pub fn show_resource(name: &str, workflow: Option<&str>) -> Result<(), ZigError> {
    validate_resource_filename(name)?;
    let candidates = candidate_dirs(workflow);
    for (label, dir) in &candidates {
        let path = dir.join(name);
        if path.is_file() {
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| ZigError::Io(format!("failed to read {}: {e}", path.display())))?;
            println!("# {} ({})", path.display(), label);
            print!("{contents}");
            if !contents.ends_with('\n') {
                println!();
            }
            return Ok(());
        }
    }
    Err(ZigError::Io(format!(
        "resource '{name}' not found in any tier"
    )))
}

/// Print the directories the collector would search for the current
/// invocation, in tier order.
pub fn print_search_paths(workflow: Option<&str>) -> Result<(), ZigError> {
    println!("Resource search paths (in collection order):");
    for (label, dir) in candidate_dirs(workflow) {
        let exists = if dir.is_dir() { "" } else { " (missing)" };
        println!("  {label:<16}  {}{exists}", dir.display());
    }
    Ok(())
}

fn candidate_dirs(workflow: Option<&str>) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    if let Some(d) = paths::global_shared_resources_dir() {
        out.push(("global:_shared".into(), d));
    }
    if let Some(name) = workflow {
        if let Some(d) = paths::global_resources_for(name) {
            out.push((format!("global:{name}"), d));
        }
    }
    if let Some(d) = paths::cwd_resources_dir() {
        out.push(("cwd".into(), d));
    } else if let Ok(cwd) = std::env::current_dir() {
        out.push(("cwd".into(), cwd.join(".zig").join("resources")));
    }
    out
}

#[cfg(test)]
#[path = "resources_manage_tests.rs"]
mod tests;
