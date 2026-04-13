//! Resource file advertisement.
//!
//! Resources are reference files (a CV, a style guide, reference docs, ...) that
//! the agent is *told about* through its system prompt so it can choose to read
//! them with its file tools on demand. Unlike `Step.files`, resources are never
//! inlined into the user message — only their absolute paths are advertised,
//! keeping context cheap and letting the agent decide what to pull in.
//!
//! This module handles resolution from the inline `resources` field in `.zug`
//! files (workflow-level + step-level). Future tiers (global `~/.zig/resources`,
//! cwd `./.zig/resources`, built-in static resources) will compose on top of
//! this collector in later PRs.

use std::path::{Path, PathBuf};

use crate::error::ZigError;
use crate::workflow::model::ResourceSpec;

/// Where a resource was discovered. Later tiers beat earlier tiers for display
/// order, but collisions never drop entries — both paths are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceOrigin {
    /// Declared at the workflow level in the `.zug` file.
    Workflow,
    /// Declared at the step level in the `.zug` file.
    Step,
}

/// A single resolved resource entry — the form that gets rendered into the
/// system prompt.
#[derive(Debug, Clone)]
pub struct Resource {
    /// Canonicalized absolute path on disk. This is the string the agent will
    /// use when calling its file-read tool.
    pub abs_path: PathBuf,
    /// Display name — the explicit `name` from the spec, falling back to the
    /// file's basename.
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
    /// Two specs that resolve to the same file on disk collapse into the
    /// first-seen entry — this prevents double-advertising the same file when
    /// it's listed at both the workflow and step level.
    fn push(&mut self, res: Resource) {
        if self.entries.iter().any(|e| e.abs_path == res.abs_path) {
            return;
        }
        self.entries.push(res);
    }
}

/// Collect resources for a single step by merging workflow-level and
/// step-level inline specs, resolving paths against `workflow_dir`.
///
/// Missing files are tolerated when `required = false` (a warning is emitted
/// to stderr); a missing `required = true` file aborts with `ZigError::Execution`.
pub fn collect_resources(
    workflow_resources: &[ResourceSpec],
    step_resources: &[ResourceSpec],
    workflow_dir: &Path,
) -> Result<ResourceSet, ZigError> {
    let mut set = ResourceSet::new();

    for spec in workflow_resources {
        if let Some(res) = resolve_one(spec, workflow_dir, ResourceOrigin::Workflow)? {
            set.push(res);
        }
    }
    for spec in step_resources {
        if let Some(res) = resolve_one(spec, workflow_dir, ResourceOrigin::Step)? {
            set.push(res);
        }
    }

    Ok(set)
}

/// Resolve a single spec. Returns `Ok(None)` when the file is absent and
/// `required = false`; returns `Err` when a required file is missing.
fn resolve_one(
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

    let joined = workflow_dir.join(raw_path);
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
