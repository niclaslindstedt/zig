//! Resource file advertisement.
//!
//! Resources are reference files (a CV, a style guide, reference docs, ...) that
//! the agent is *told about* through its system prompt so it can choose to read
//! them with its file tools on demand. Unlike `Step.files`, resources are never
//! inlined into the user message — only their absolute paths are advertised,
//! keeping context cheap and letting the agent decide what to pull in.
//!
//! Resources come from four tiers, merged at run time in this order:
//!
//! 1. **Global shared** — every file under `~/.zig/resources/_shared/`
//! 2. **Global per-workflow** — every file under `~/.zig/resources/<workflow-name>/`
//! 3. **Project (cwd)** — every file under `<git-root>/.zig/resources/`
//! 4. **Inline workflow** — `resources = [...]` in `[workflow]` of the `.zwf` file
//! 5. **Inline step** — `resources = [...]` on a single `[[step]]`
//!
//! Entries are deduplicated by canonicalized absolute path; the first tier to
//! discover a path wins for display ordering. Name collisions across different
//! paths are *not* dropped — both files are advertised so the agent can see
//! everything that's actually on disk.

use std::path::{Path, PathBuf};

use crate::error::ZigError;
use crate::paths::expand_path;
use crate::workflow::model::ResourceSpec;

/// Where a resource was discovered. Tiers are emitted in declaration order;
/// the first tier to register a given canonical path wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceOrigin {
    /// `~/.zig/resources/_shared/`
    GlobalShared,
    /// `~/.zig/resources/<workflow-name>/`
    GlobalWorkflow,
    /// `<git-root>/.zig/resources/`
    Cwd,
    /// `resources = [...]` in `[workflow]`
    Workflow,
    /// `resources = [...]` on a `[[step]]`
    Step,
}

impl ResourceOrigin {
    /// Short human-readable label, used by the `zig resources list` command.
    pub fn label(self) -> &'static str {
        match self {
            ResourceOrigin::GlobalShared => "global:_shared",
            ResourceOrigin::GlobalWorkflow => "global:workflow",
            ResourceOrigin::Cwd => "cwd",
            ResourceOrigin::Workflow => "inline:workflow",
            ResourceOrigin::Step => "inline:step",
        }
    }
}

/// A single resolved resource entry — the form that gets rendered into the
/// system prompt.
#[derive(Debug, Clone)]
pub struct Resource {
    /// Canonicalized absolute path on disk. This is the string the agent will
    /// use when calling its file-read tool.
    pub abs_path: PathBuf,
    /// Display name — the explicit `name` from a detailed spec, or the file's
    /// basename when discovered from a directory or a bare path.
    pub name: String,
    /// Optional description, shown in the rendered block after the path.
    pub description: Option<String>,
    /// Which tier this resource came from.
    pub origin: ResourceOrigin,
}

/// An ordered, deduplicated collection of resources.
#[derive(Debug, Clone, Default)]
pub struct ResourceSet {
    entries: Vec<Resource>,
}

impl ResourceSet {
    /// Create an empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true when no resources were collected.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of resolved resources in the set.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over the resolved resources in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &Resource> {
        self.entries.iter()
    }

    /// Push a resource, deduplicating by canonical absolute path.
    ///
    /// The first occurrence wins — this prevents double-advertising the same
    /// file when it's listed at multiple tiers (e.g. inline workflow + cwd).
    fn push(&mut self, res: Resource) {
        if self.entries.iter().any(|e| e.abs_path == res.abs_path) {
            return;
        }
        self.entries.push(res);
    }
}

/// Run-time configuration for resource collection, built once per workflow
/// invocation in [`crate::run::run_workflow`] and threaded through to every
/// step.
///
/// Holds pre-resolved tier directories so tests can construct collectors
/// directly without mutating `$HOME`. Use [`ResourceCollector::from_env`]
/// at runtime to populate the directories from `crate::paths`.
pub struct ResourceCollector<'a> {
    /// Inline workflow-level resources from the `.zwf` file.
    pub workflow_resources: &'a [ResourceSpec],
    /// Directory the `.zwf` file lives in — relative paths in inline specs
    /// are resolved against this.
    pub workflow_dir: &'a Path,
    /// `~/.zig/resources/_shared/`, when present.
    pub global_shared_dir: Option<PathBuf>,
    /// `~/.zig/resources/<workflow-name>/`, when present.
    pub global_workflow_dir: Option<PathBuf>,
    /// `<git-root>/.zig/resources/`, when present.
    pub cwd_resources_dir: Option<PathBuf>,
    /// When true, all tiers are skipped and `collect_for_step` returns an
    /// empty set. Set by `--no-resources` on `zig run`.
    pub disabled: bool,
}

impl<'a> ResourceCollector<'a> {
    /// Build a collector for a workflow at runtime, populating tier
    /// directories from the user's `$HOME` and current working directory.
    pub fn from_env(
        workflow_name: &str,
        workflow_resources: &'a [ResourceSpec],
        workflow_dir: &'a Path,
        disabled: bool,
    ) -> Self {
        Self {
            workflow_resources,
            workflow_dir,
            global_shared_dir: crate::paths::global_shared_resources_dir(),
            global_workflow_dir: crate::paths::global_resources_for(workflow_name),
            cwd_resources_dir: crate::paths::cwd_resources_dir(),
            disabled,
        }
    }

    /// Build the merged resource set for a single step, walking all tiers in
    /// declaration order.
    pub fn collect_for_step(
        &self,
        step_resources: &[ResourceSpec],
    ) -> Result<ResourceSet, ZigError> {
        let mut set = ResourceSet::new();
        if self.disabled {
            return Ok(set);
        }

        // Tier 1: ~/.zig/resources/_shared/
        if let Some(dir) = self.global_shared_dir.as_deref() {
            scan_directory_into(dir, ResourceOrigin::GlobalShared, &mut set);
        }

        // Tier 2: ~/.zig/resources/<workflow-name>/
        if let Some(dir) = self.global_workflow_dir.as_deref() {
            scan_directory_into(dir, ResourceOrigin::GlobalWorkflow, &mut set);
        }

        // Tier 3: <git-root>/.zig/resources/
        if let Some(dir) = self.cwd_resources_dir.as_deref() {
            scan_directory_into(dir, ResourceOrigin::Cwd, &mut set);
        }

        // Tier 4: inline workflow resources
        for spec in self.workflow_resources {
            if let Some(res) = resolve_inline(spec, self.workflow_dir, ResourceOrigin::Workflow)? {
                set.push(res);
            }
        }

        // Tier 5: inline step resources
        for spec in step_resources {
            if let Some(res) = resolve_inline(spec, self.workflow_dir, ResourceOrigin::Step)? {
                set.push(res);
            }
        }

        Ok(set)
    }
}

/// Recursively scan a directory and add every regular file to the set as a
/// resource of the given origin. Symlinks and entries that fail to canonicalize
/// are silently skipped — this is best-effort discovery, not a hard contract.
fn scan_directory_into(dir: &Path, origin: ResourceOrigin, set: &mut ResourceSet) {
    if !dir.is_dir() {
        return;
    }

    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // `entry.metadata()` returns symlink metadata, which would silently
            // drop linked-in resource files. Follow symlinks via `fs::metadata`.
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
            let abs_path = match std::fs::canonicalize(&path) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let name = abs_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| abs_path.display().to_string());
            set.push(Resource {
                abs_path,
                name,
                description: None,
                origin,
            });
        }
    }
}

/// Resolve a single inline `ResourceSpec` against `workflow_dir`. Returns
/// `Ok(None)` when the file is absent and `required = false`; returns `Err`
/// when a required file is missing.
fn resolve_inline(
    spec: &ResourceSpec,
    workflow_dir: &Path,
    origin: ResourceOrigin,
) -> Result<Option<Resource>, ZigError> {
    let raw_path = spec.path();
    if raw_path.is_empty() {
        return Err(ZigError::Execution(
            "resource entry has an empty path".into(),
        ));
    }

    let joined = workflow_dir.join(expand_path(raw_path));
    let abs_path = match std::fs::canonicalize(&joined) {
        Ok(p) => p,
        Err(_) => {
            if spec.required() {
                return Err(ZigError::Execution(format!(
                    "required resource '{}' not found (looked at {})",
                    raw_path,
                    joined.display()
                )));
            }
            eprintln!(
                "  warning: resource '{}' not found at {} — skipping",
                raw_path,
                joined.display()
            );
            return Ok(None);
        }
    };

    let name = spec
        .name()
        .map(str::to_string)
        .or_else(|| {
            abs_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| raw_path.to_string());

    Ok(Some(Resource {
        abs_path,
        name,
        description: spec.description().map(str::to_string),
        origin,
    }))
}

/// Convenience for tests and the original PR 1 API: collect inline-only
/// resources without walking any global/cwd tiers.
pub fn collect_inline_resources(
    workflow_resources: &[ResourceSpec],
    step_resources: &[ResourceSpec],
    workflow_dir: &Path,
) -> Result<ResourceSet, ZigError> {
    let mut set = ResourceSet::new();
    for spec in workflow_resources {
        if let Some(res) = resolve_inline(spec, workflow_dir, ResourceOrigin::Workflow)? {
            set.push(res);
        }
    }
    for spec in step_resources {
        if let Some(res) = resolve_inline(spec, workflow_dir, ResourceOrigin::Step)? {
            set.push(res);
        }
    }
    Ok(set)
}

/// Render a `<resources>` XML-ish block to prepend to a system prompt.
///
/// Returns an empty string when the set is empty, so callers can concatenate
/// unconditionally without producing stray whitespace.
pub fn render_system_block(set: &ResourceSet) -> String {
    if set.is_empty() {
        return String::new();
    }

    let mut out = String::from("<resources>\n");
    out.push_str(
        "You have access to the following reference files. Read them with your file tools when the user's request relates to them.\n\n",
    );
    for res in set.iter() {
        out.push_str("- ");
        out.push_str(&res.abs_path.display().to_string());
        if let Some(desc) = res.description.as_deref() {
            out.push_str(" — ");
            out.push_str(desc);
        } else {
            out.push_str(" (");
            out.push_str(&res.name);
            out.push(')');
        }
        out.push('\n');
    }
    out.push_str("</resources>\n\n");
    out
}

#[cfg(test)]
#[path = "resources_tests.rs"]
mod tests;
