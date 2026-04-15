//! Workflow storage — structured, writable working data for a run.
//!
//! Storage is a first-class workflow concept alongside `vars` (scalar state)
//! and `resources` (read-only reference files). It gives a workflow a place to
//! accumulate files that its steps produce and consume — the canonical example
//! is a book-writing workflow that maintains character sheets, world-building
//! notes, and a consistency bible across many steps.
//!
//! ## Path resolution
//!
//! Storage paths resolve relative to `<cwd>/.zig/` — where `<cwd>` is the
//! directory the user invoked `zig run` from. Absolute paths pass through
//! unchanged. This differs from resources, which resolve relative to the
//! `.zwf` file: resources ship with the workflow, storage belongs to the
//! run.
//!
//! ## Backends
//!
//! The [`StorageBackend`] trait abstracts over the underlying store. The only
//! implementation today is [`FilesystemBackend`]; future sqlite/remote
//! backends slot in behind the same trait without workflow-format changes.

use std::path::{Path, PathBuf};

use crate::error::ZigError;
use crate::paths::expand_path;
use crate::workflow::model::{StorageKind, StorageSpec};

/// A single entry inside a folder-typed storage item, surfaced to the agent
/// so it can see what previous steps wrote.
#[derive(Debug, Clone)]
pub struct StorageEntry {
    /// File name relative to the storage folder (no path prefix).
    pub name: String,
    /// File size in bytes, when available.
    pub size: Option<u64>,
}

/// The current contents of a storage item at the moment a step starts.
#[derive(Debug, Clone, Default)]
pub struct StorageListing {
    /// Files currently present. For folder-typed storage this enumerates
    /// the folder contents; for file-typed storage it contains a single
    /// entry (or is empty when the file hasn't been created yet).
    pub entries: Vec<StorageEntry>,
}

/// Abstraction over a storage backend. Implementations decide how to
/// materialise a [`StorageSpec`] and how to enumerate its contents.
pub trait StorageBackend: std::fmt::Debug {
    /// Ensure the storage item exists and is ready for reads/writes.
    /// Called once per run before any step executes. Must be idempotent.
    fn ensure(&self, spec: &StorageSpec) -> Result<(), ZigError>;

    /// Enumerate the current contents of the storage item. Called each
    /// time a step's system prompt is rendered so the listing reflects
    /// files written by previous steps in the same run.
    fn listing(&self, spec: &StorageSpec) -> Result<StorageListing, ZigError>;

    /// Absolute on-disk path for the storage item. This is what gets
    /// embedded in the agent's system prompt so it can read/write with
    /// its normal file tools.
    fn abs_path(&self, spec: &StorageSpec) -> PathBuf;
}

/// Filesystem-backed storage rooted at `<cwd>/.zig/`.
#[derive(Debug, Clone)]
pub struct FilesystemBackend {
    /// Root directory — typically `<cwd>/.zig/`. Relative paths in a
    /// [`StorageSpec`] are joined onto this; absolute paths bypass it.
    zig_root: PathBuf,
}

impl FilesystemBackend {
    /// Create a backend rooted at the given directory. The directory is
    /// created on demand during [`StorageBackend::ensure`]; it does not
    /// need to exist yet.
    pub fn new(zig_root: PathBuf) -> Self {
        Self { zig_root }
    }

    /// Build a backend rooted at `<cwd>/.zig/` using the process's current
    /// working directory. Use [`FilesystemBackend::new`] in tests so you
    /// can pin the root to a tempdir.
    pub fn from_cwd() -> Result<Self, ZigError> {
        let cwd = std::env::current_dir()
            .map_err(|e| ZigError::Io(format!("failed to resolve cwd for storage: {e}")))?;
        Ok(Self::new(cwd.join(".zig")))
    }

    fn resolve(&self, raw_path: &str) -> PathBuf {
        let expanded = PathBuf::from(expand_path(raw_path));
        if expanded.is_absolute() {
            expanded
        } else {
            self.zig_root.join(expanded)
        }
    }
}

impl StorageBackend for FilesystemBackend {
    fn ensure(&self, spec: &StorageSpec) -> Result<(), ZigError> {
        let target = self.resolve(&spec.path);
        match spec.kind {
            StorageKind::Folder => {
                std::fs::create_dir_all(&target).map_err(|e| {
                    ZigError::Io(format!(
                        "failed to create storage folder {}: {e}",
                        target.display()
                    ))
                })?;
            }
            StorageKind::File => {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        ZigError::Io(format!(
                            "failed to create parent for storage file {}: {e}",
                            target.display()
                        ))
                    })?;
                }
                if !target.exists() {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(false)
                        .open(&target)
                        .map_err(|e| {
                            ZigError::Io(format!(
                                "failed to create storage file {}: {e}",
                                target.display()
                            ))
                        })?;
                }
            }
        }
        Ok(())
    }

    fn listing(&self, spec: &StorageSpec) -> Result<StorageListing, ZigError> {
        let target = self.resolve(&spec.path);
        let mut entries = Vec::new();
        match spec.kind {
            StorageKind::Folder => {
                let read_dir = match std::fs::read_dir(&target) {
                    Ok(r) => r,
                    // Folder hasn't been created yet (race) or was removed —
                    // return an empty listing rather than failing the step.
                    Err(_) => return Ok(StorageListing::default()),
                };
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    let meta = match std::fs::metadata(&path) {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    if !meta.is_file() {
                        continue;
                    }
                    let name = match path.file_name() {
                        Some(n) => n.to_string_lossy().into_owned(),
                        None => continue,
                    };
                    entries.push(StorageEntry {
                        name,
                        size: Some(meta.len()),
                    });
                }
                entries.sort_by(|a, b| a.name.cmp(&b.name));
            }
            StorageKind::File => {
                if let Ok(meta) = std::fs::metadata(&target) {
                    if meta.is_file() {
                        let name = target
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| target.display().to_string());
                        entries.push(StorageEntry {
                            name,
                            size: Some(meta.len()),
                        });
                    }
                }
            }
        }
        Ok(StorageListing { entries })
    }

    fn abs_path(&self, spec: &StorageSpec) -> PathBuf {
        self.resolve(&spec.path)
    }
}

/// Owns the set of declared storage items for a single workflow run and
/// routes operations to the appropriate backend. Built once in
/// [`crate::run::execute`] and threaded through to every step.
#[derive(Debug)]
pub struct StorageManager {
    items: Vec<StorageItem>,
}

/// A single declared storage item bound to the backend that services it.
#[derive(Debug)]
pub struct StorageItem {
    /// The name the workflow author gave this storage entry in `[storage.*]`.
    pub name: String,
    /// The declaration from the workflow file.
    pub spec: StorageSpec,
    /// Backend that materialises this entry. Always `FilesystemBackend`
    /// today; the trait leaves room for future sqlite/remote backends.
    pub backend: Box<dyn StorageBackend + Send + Sync>,
}

impl StorageManager {
    /// Build a manager from the workflow's `storage` table, wiring every
    /// entry to a shared [`FilesystemBackend`]. Calls `ensure` on each
    /// item so downstream steps can trust that the path is live.
    pub fn build(
        storage: &std::collections::HashMap<String, StorageSpec>,
        backend: FilesystemBackend,
    ) -> Result<Self, ZigError> {
        let mut items = Vec::with_capacity(storage.len());
        // Sort names so the listing order in prompts is deterministic.
        let mut names: Vec<&String> = storage.keys().collect();
        names.sort();
        for name in names {
            let spec = storage[name].clone();
            backend.ensure(&spec)?;
            items.push(StorageItem {
                name: name.clone(),
                spec,
                backend: Box::new(backend.clone()),
            });
        }
        Ok(Self { items })
    }

    /// Empty manager — used when a workflow declares no storage.
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// Returns true when no storage is declared.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate over every declared item, regardless of scoping.
    pub fn iter(&self) -> impl Iterator<Item = &StorageItem> {
        self.items.iter()
    }

    /// Return the items a step is allowed to see, applying the step's
    /// `storage` scoping field.
    ///
    /// - `scope = None` (field omitted) → every declared item.
    /// - `scope = Some(&[])` → no items.
    /// - `scope = Some(names)` → only items whose name appears in `names`.
    pub fn items_for_step(&self, scope: Option<&[String]>) -> Vec<&StorageItem> {
        match scope {
            None => self.items.iter().collect(),
            Some([]) => Vec::new(),
            Some(names) => self
                .items
                .iter()
                .filter(|item| names.iter().any(|n| n == &item.name))
                .collect(),
        }
    }

    /// Render the `<storage>` XML block for a single step. Returns `None`
    /// when the step is scoped to zero items (or no storage is declared),
    /// which tells the caller to omit the block entirely.
    pub fn render_block(&self, scope: Option<&[String]>) -> Result<Option<String>, ZigError> {
        let items = self.items_for_step(scope);
        if items.is_empty() {
            return Ok(None);
        }
        let mut out = String::from("<storage>\n");
        for item in items {
            let abs = item.backend.abs_path(&item.spec);
            out.push_str(&format!(
                "  <item name=\"{}\" type=\"{}\" path=\"{}\">\n",
                escape_xml(&item.name),
                item.spec.kind,
                escape_xml(&abs.display().to_string()),
            ));
            if let Some(desc) = item.spec.description.as_deref() {
                out.push_str(&format!(
                    "    <description>{}</description>\n",
                    escape_xml(desc)
                ));
            }
            if let Some(hint) = item.spec.hint.as_deref() {
                out.push_str(&format!("    <hint>{}</hint>\n", escape_xml(hint)));
            }
            if !item.spec.files.is_empty() {
                out.push_str("    <expected>\n");
                for file in &item.spec.files {
                    match file.description.as_deref() {
                        Some(d) => out.push_str(&format!(
                            "      - {}: {}\n",
                            escape_xml(&file.name),
                            escape_xml(d)
                        )),
                        None => out.push_str(&format!("      - {}\n", escape_xml(&file.name))),
                    }
                }
                out.push_str("    </expected>\n");
            }
            let listing = item.backend.listing(&item.spec)?;
            if !listing.entries.is_empty() {
                out.push_str("    <contents>\n");
                for entry in listing.entries {
                    out.push_str(&format!("      - {}\n", escape_xml(&entry.name)));
                }
                out.push_str("    </contents>\n");
            }
            out.push_str("  </item>\n");
        }
        out.push_str("</storage>");
        Ok(Some(out))
    }

    /// Return absolute on-disk paths for every storage folder the step can
    /// see. The caller feeds these into the step's `add_dirs` so the
    /// agent sandbox can actually read/write them.
    pub fn add_dirs_for_step(&self, scope: Option<&[String]>) -> Vec<PathBuf> {
        self.items_for_step(scope)
            .into_iter()
            .map(|item| {
                let mut path = item.backend.abs_path(&item.spec);
                // For file-typed storage, widen scope to the parent dir so
                // the agent can actually open the file.
                if matches!(item.spec.kind, StorageKind::File) {
                    if let Some(parent) = path.parent() {
                        path = parent.to_path_buf();
                    }
                }
                path
            })
            .collect()
    }
}

fn escape_xml(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Resolve a raw storage path against `<cwd>/.zig/` for callers that
/// don't have a full [`StorageManager`] — used by tests and by code that
/// wants to know the absolute path without going through the backend.
pub fn resolve_against(root: &Path, raw_path: &str) -> PathBuf {
    let expanded = PathBuf::from(expand_path(raw_path));
    if expanded.is_absolute() {
        expanded
    } else {
        root.join(expanded)
    }
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;
