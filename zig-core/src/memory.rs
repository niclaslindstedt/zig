//! Memory scratch pad for workflows and steps.
//!
//! Memory is a managed folder of files with a `.manifest` JSON index that gets
//! injected into agent system prompts. It enables agents to accumulate and
//! search knowledge across workflow runs.
//!
//! Storage mirrors the resource tier layout:
//!
//! * `~/.zig/memory/_shared/` — global shared tier
//! * `~/.zig/memory/<workflow>/` — global per-workflow tier
//! * `<git-root>/.zig/memory/` — project-local tier
//!
//! Each tier directory contains a `.manifest` JSON file alongside the actual
//! memory files.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::ZigConfig;
use crate::error::ZigError;
use crate::paths;
use crate::workflow::model::MemoryMode;

// =====================================================================
// Data structures
// =====================================================================

/// The `.manifest` file contents — an index of all memory entries in a tier.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    /// Next ID to assign when adding a new entry.
    pub next_id: u64,
    /// Entries keyed by their string ID ("1", "2", ...).
    pub entries: BTreeMap<String, MemoryEntry>,
}

/// A single memory entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Display name for this memory entry.
    pub name: String,
    /// Filename of the memory file within the tier directory.
    pub file: String,
    /// Optional human-readable description of the memory contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for filtering and discovery.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Optional step name this memory is scoped to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    /// Original source path the file was added from.
    pub source: String,
    /// When the entry was added.
    pub added: DateTime<Utc>,
}

/// Where a memory command should target.
#[derive(Debug, Clone)]
pub enum MemoryTarget {
    /// `~/.zig/memory/_shared/`
    GlobalShared,
    /// `~/.zig/memory/<workflow>/`
    GlobalWorkflow(String),
    /// `<git-root>/.zig/memory/`
    Cwd,
}

impl MemoryTarget {
    /// Resolve from CLI flag combination. Same rules as `ResourceTarget`.
    pub fn from_flags(workflow: Option<&str>, global: bool, cwd: bool) -> Result<Self, ZigError> {
        if let Some(name) = workflow {
            if cwd {
                return Err(ZigError::Validation(
                    "--workflow cannot be combined with --cwd".into(),
                ));
            }
            return Ok(MemoryTarget::GlobalWorkflow(name.to_string()));
        }
        if cwd {
            return Ok(MemoryTarget::Cwd);
        }
        if global {
            return Ok(MemoryTarget::GlobalShared);
        }
        // Default: project-local.
        Ok(MemoryTarget::Cwd)
    }

    /// Resolve to an absolute directory path, creating it if it doesn't exist.
    pub fn ensure_dir(&self) -> Result<PathBuf, ZigError> {
        match self {
            MemoryTarget::GlobalShared => paths::ensure_global_memory_dir(Some("_shared")),
            MemoryTarget::GlobalWorkflow(name) => paths::ensure_global_memory_dir(Some(name)),
            MemoryTarget::Cwd => ensure_cwd_memory_dir(),
        }
    }

    /// Resolve to an absolute directory path without creating it.
    pub fn existing_dir(&self) -> Option<PathBuf> {
        match self {
            MemoryTarget::GlobalShared => paths::global_shared_memory_dir(),
            MemoryTarget::GlobalWorkflow(name) => paths::global_memory_for(name),
            MemoryTarget::Cwd => paths::cwd_memory_dir().or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.join(".zig").join("memory"))
            }),
        }
    }

    /// Short label for diagnostic messages.
    pub fn label(&self) -> String {
        match self {
            MemoryTarget::GlobalShared => "global:_shared".to_string(),
            MemoryTarget::GlobalWorkflow(n) => format!("global:{n}"),
            MemoryTarget::Cwd => "cwd".to_string(),
        }
    }
}

/// Search result granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    /// Return the full sentence containing the match.
    Sentence,
    /// Return the full paragraph containing the match.
    Paragraph,
    /// Return the full h2 section containing the match.
    Section,
    /// Return the entire file contents.
    File,
}

// =====================================================================
// Manifest I/O
// =====================================================================

fn manifest_path(dir: &Path) -> PathBuf {
    dir.join(".manifest")
}

/// Load the manifest from a tier directory. Returns an empty manifest if the
/// file does not exist.
pub fn load_manifest(dir: &Path) -> Result<Manifest, ZigError> {
    let path = manifest_path(dir);
    if !path.exists() {
        return Ok(Manifest {
            next_id: 1,
            entries: BTreeMap::new(),
        });
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| ZigError::Io(format!("failed to read {}: {e}", path.display())))?;
    serde_json::from_str(&content).map_err(|e| {
        ZigError::Io(format!(
            "failed to parse manifest at {}: {e}",
            path.display()
        ))
    })
}

/// Save the manifest to a tier directory.
pub fn save_manifest(dir: &Path, manifest: &Manifest) -> Result<(), ZigError> {
    let path = manifest_path(dir);
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|e| ZigError::Serialize(format!("failed to serialize manifest: {e}")))?;
    std::fs::write(&path, content)
        .map_err(|e| ZigError::Io(format!("failed to write {}: {e}", path.display())))
}

fn ensure_cwd_memory_dir() -> Result<PathBuf, ZigError> {
    if let Some(existing) = paths::cwd_memory_dir() {
        return Ok(existing);
    }
    let cwd = std::env::current_dir()
        .map_err(|e| ZigError::Io(format!("failed to read current directory: {e}")))?;
    let dir = cwd.join(".zig").join("memory");
    std::fs::create_dir_all(&dir)
        .map_err(|e| ZigError::Io(format!("failed to create {}: {e}", dir.display())))?;
    Ok(dir)
}

// =====================================================================
// Tier traversal helpers
// =====================================================================

/// Build a list of (label, dir) pairs for all potentially relevant tiers.
fn candidate_dirs(workflow: Option<&str>) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    if let Some(d) = paths::global_shared_memory_dir() {
        out.push(("global:_shared".into(), d));
    }
    if let Some(name) = workflow {
        if let Some(d) = paths::global_memory_for(name) {
            out.push((format!("global:{name}"), d));
        }
    }
    if let Some(d) = paths::cwd_memory_dir() {
        out.push(("cwd".into(), d));
    } else if let Ok(cwd) = std::env::current_dir() {
        out.push(("cwd".into(), cwd.join(".zig").join("memory")));
    }
    out
}

/// Search all tier manifests for an entry with the given ID.
/// Returns (tier_dir, tier_label, manifest, entry_clone).
fn find_entry_across_tiers(
    id: u64,
    workflow: Option<&str>,
) -> Result<(PathBuf, String, Manifest, MemoryEntry), ZigError> {
    let id_str = id.to_string();
    // Search project-local first (most specific), then global.
    let dirs = candidate_dirs(workflow);
    for (label, dir) in dirs.iter().rev() {
        if !dir.is_dir() {
            continue;
        }
        let manifest = load_manifest(dir)?;
        if let Some(entry) = manifest.entries.get(&id_str).cloned() {
            return Ok((dir.clone(), label.clone(), manifest, entry));
        }
    }
    Err(ZigError::Io(format!(
        "memory entry with id {id} not found in any tier"
    )))
}

// =====================================================================
// Public command functions
// =====================================================================

/// Add a file to the memory scratch pad.
///
/// Copies the file into the target tier directory, assigns a numeric ID, and
/// updates the manifest. Returns the assigned ID.
pub fn add(
    file_path: &str,
    target: MemoryTarget,
    step: Option<&str>,
    name: Option<&str>,
    description: Option<&str>,
    tags: &[String],
) -> Result<u64, ZigError> {
    let src = Path::new(file_path);
    if !src.exists() {
        return Err(ZigError::Io(format!("source file not found: {file_path}")));
    }
    if !src.is_file() {
        return Err(ZigError::Io(format!("not a regular file: {file_path}")));
    }

    let dir = target.ensure_dir()?;
    let mut manifest = load_manifest(&dir)?;

    let id = manifest.next_id;
    manifest.next_id += 1;

    let file_name = name
        .map(str::to_string)
        .or_else(|| src.file_name().map(|n| n.to_string_lossy().into_owned()))
        .ok_or_else(|| ZigError::Io(format!("could not derive a name from {}", src.display())))?;

    let dest = dir.join(&file_name);
    if dest.exists() {
        return Err(ZigError::Io(format!(
            "file '{}' already exists in {} — remove it first or use --name to rename",
            file_name,
            dir.display()
        )));
    }

    std::fs::copy(src, &dest).map_err(|e| {
        ZigError::Io(format!(
            "failed to copy {} → {}: {e}",
            src.display(),
            dest.display()
        ))
    })?;

    let source_abs = std::fs::canonicalize(src)
        .unwrap_or_else(|_| src.to_path_buf())
        .display()
        .to_string();

    let entry = MemoryEntry {
        name: file_name.clone(),
        file: file_name,
        description: description.map(str::to_string),
        tags: tags.to_vec(),
        step: step.map(str::to_string),
        source: source_abs,
        added: Utc::now(),
    };

    manifest.entries.insert(id.to_string(), entry);
    save_manifest(&dir, &manifest)?;

    println!(
        "added memory entry id={id} '{}' to {}",
        manifest.entries[&id.to_string()].name,
        target.label()
    );

    if description.is_none() {
        eprintln!("hint: add a description with `zig memory update {id} --description \"...\"`");
    }

    Ok(id)
}

/// Update metadata for an existing memory entry.
pub fn update(
    id: u64,
    workflow: Option<&str>,
    name: Option<&str>,
    description: Option<&str>,
    tags: Option<&[String]>,
) -> Result<(), ZigError> {
    let (dir, label, mut manifest, _entry) = find_entry_across_tiers(id, workflow)?;
    let id_str = id.to_string();

    let entry = manifest
        .entries
        .get_mut(&id_str)
        .ok_or_else(|| ZigError::Io(format!("memory entry {id} vanished during update")))?;

    if let Some(n) = name {
        // Rename the file on disk if the name changed.
        let old_path = dir.join(&entry.file);
        let new_path = dir.join(n);
        if old_path != new_path {
            if new_path.exists() {
                return Err(ZigError::Io(format!(
                    "cannot rename: '{}' already exists in {}",
                    n,
                    dir.display()
                )));
            }
            std::fs::rename(&old_path, &new_path).map_err(|e| {
                ZigError::Io(format!(
                    "failed to rename {} → {}: {e}",
                    old_path.display(),
                    new_path.display()
                ))
            })?;
            entry.file = n.to_string();
        }
        entry.name = n.to_string();
    }
    if let Some(d) = description {
        entry.description = Some(d.to_string());
    }
    if let Some(t) = tags {
        entry.tags = t.to_vec();
    }

    save_manifest(&dir, &manifest)?;
    println!("updated memory entry id={id} in {label}");
    Ok(())
}

/// Delete a memory entry and its file.
pub fn delete(id: u64, workflow: Option<&str>) -> Result<(), ZigError> {
    let (dir, label, mut manifest, entry) = find_entry_across_tiers(id, workflow)?;
    let id_str = id.to_string();

    let file_path = dir.join(&entry.file);
    if file_path.is_file() {
        std::fs::remove_file(&file_path)
            .map_err(|e| ZigError::Io(format!("failed to remove {}: {e}", file_path.display())))?;
    }

    manifest.entries.remove(&id_str);
    save_manifest(&dir, &manifest)?;
    println!("deleted memory entry id={id} '{}' from {label}", entry.name);
    Ok(())
}

/// Show metadata and contents of a memory entry.
pub fn show(id: u64, workflow: Option<&str>) -> Result<(), ZigError> {
    let (dir, label, _manifest, entry) = find_entry_across_tiers(id, workflow)?;
    let file_path = dir.join(&entry.file);

    println!("id:          {id}");
    println!("name:        {}", entry.name);
    println!("tier:        {label}");
    println!("source:      {}", entry.source);
    println!(
        "added:       {}",
        entry.added.format("%Y-%m-%d %H:%M:%S UTC")
    );
    if let Some(ref desc) = entry.description {
        println!("description: {desc}");
    }
    if !entry.tags.is_empty() {
        println!("tags:        {}", entry.tags.join(", "));
    }
    if let Some(ref step) = entry.step {
        println!("step:        {step}");
    }

    if file_path.is_file() {
        let contents = std::fs::read_to_string(&file_path)
            .map_err(|e| ZigError::Io(format!("failed to read {}: {e}", file_path.display())))?;
        println!("\n--- contents ({}) ---", file_path.display());
        print!("{contents}");
        if !contents.ends_with('\n') {
            println!();
        }
    } else {
        println!("\n(file not found: {})", file_path.display());
    }

    Ok(())
}

/// List all memory entries across all tiers.
pub fn list(workflow: Option<&str>) -> Result<(), ZigError> {
    let mut rows: Vec<(String, String, String, String, String, String)> = Vec::new();

    let dirs = candidate_dirs(workflow);
    for (label, dir) in &dirs {
        if !dir.is_dir() {
            continue;
        }
        let manifest = load_manifest(dir)?;
        for (id_str, entry) in &manifest.entries {
            let desc = entry
                .description
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(50)
                .collect::<String>();
            let tags = entry.tags.join(", ");
            rows.push((
                id_str.clone(),
                entry.name.clone(),
                tags,
                desc,
                label.clone(),
                entry.step.clone().unwrap_or_default(),
            ));
        }
    }

    if rows.is_empty() {
        println!("No memory entries found.");
        println!("Hint: add one with `zig memory add <file> [--workflow <name>]`");
        return Ok(());
    }

    let id_w = rows.iter().map(|r| r.0.len()).max().unwrap_or(2).max(2);
    let name_w = rows.iter().map(|r| r.1.len()).max().unwrap_or(4).max(4);
    let tags_w = rows.iter().map(|r| r.2.len()).max().unwrap_or(4).max(4);
    let tier_w = rows.iter().map(|r| r.4.len()).max().unwrap_or(4).max(4);

    println!(
        "{:<id_w$}  {:<name_w$}  {:<tags_w$}  {:<tier_w$}  DESCRIPTION",
        "ID", "NAME", "TAGS", "TIER",
    );
    for (id, name, tags, desc, tier, _step) in &rows {
        println!(
            "{:<id_w$}  {:<name_w$}  {:<tags_w$}  {:<tier_w$}  {desc}",
            id, name, tags, tier,
        );
    }

    Ok(())
}

// =====================================================================
// Search
// =====================================================================

/// Full-text search across all memory files.
pub fn search(query: &str, scope: SearchScope, workflow: Option<&str>) -> Result<(), ZigError> {
    let query_lower = query.to_lowercase();
    let mut found = false;

    let dirs = candidate_dirs(workflow);
    for (label, dir) in &dirs {
        if !dir.is_dir() {
            continue;
        }
        let manifest = load_manifest(dir)?;
        for (id_str, entry) in &manifest.entries {
            let file_path = dir.join(&entry.file);
            if !file_path.is_file() {
                continue;
            }
            let content = match std::fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if !content.to_lowercase().contains(&query_lower) {
                continue;
            }

            let matches = extract_matches(&content, &query_lower, scope);
            for m in matches {
                if !found {
                    found = true;
                }
                println!(
                    "[id:{} {} {}:{}]",
                    id_str,
                    label,
                    entry.name,
                    m.line_number.unwrap_or(0)
                );
                println!("{}", m.text);
                println!();
            }
        }
    }

    if !found {
        println!("No matches found for '{query}'.");
    }

    Ok(())
}

struct MatchFragment {
    text: String,
    line_number: Option<usize>,
}

fn extract_matches(content: &str, query_lower: &str, scope: SearchScope) -> Vec<MatchFragment> {
    match scope {
        SearchScope::Sentence => extract_sentences(content, query_lower),
        SearchScope::Paragraph => extract_paragraphs(content, query_lower),
        SearchScope::Section => extract_sections(content, query_lower),
        SearchScope::File => extract_file(content, query_lower),
    }
}

fn extract_sentences(content: &str, query_lower: &str) -> Vec<MatchFragment> {
    let mut results = Vec::new();
    // Track character offset → line number mapping.
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(content.match_indices('\n').map(|(i, _)| i + 1))
        .collect();

    let find_line = |byte_offset: usize| -> usize {
        match line_starts.binary_search(&byte_offset) {
            Ok(i) => i + 1,
            Err(i) => i,
        }
    };

    // Split on sentence-ending punctuation followed by whitespace or EOF.
    let chars: Vec<char> = content.chars().collect();
    let mut byte_pos = 0;
    let mut sentence_start_byte = 0;

    for (i, &ch) in chars.iter().enumerate() {
        let ch_len = ch.len_utf8();
        if (ch == '.' || ch == '!' || ch == '?')
            && (i + 1 >= chars.len() || chars[i + 1].is_whitespace())
        {
            let sentence_end_byte = byte_pos + ch_len;
            let sentence = &content[sentence_start_byte..sentence_end_byte];
            if sentence.to_lowercase().contains(query_lower) {
                results.push(MatchFragment {
                    text: sentence.trim().to_string(),
                    line_number: Some(find_line(sentence_start_byte)),
                });
            }
            sentence_start_byte = sentence_end_byte;
        }
        byte_pos += ch_len;
    }

    // Handle trailing text without sentence-ending punctuation.
    if sentence_start_byte < content.len() {
        let sentence = &content[sentence_start_byte..];
        if sentence.to_lowercase().contains(query_lower) {
            results.push(MatchFragment {
                text: sentence.trim().to_string(),
                line_number: Some(find_line(sentence_start_byte)),
            });
        }
    }

    results
}

fn extract_paragraphs(content: &str, query_lower: &str) -> Vec<MatchFragment> {
    let mut results = Vec::new();
    let mut line_num = 1;

    for paragraph in content.split("\n\n") {
        if paragraph.to_lowercase().contains(query_lower) {
            results.push(MatchFragment {
                text: paragraph.trim().to_string(),
                line_number: Some(line_num),
            });
        }
        // Count lines in this paragraph + the 2 newlines of the separator.
        line_num += paragraph.matches('\n').count() + 2;
    }

    results
}

fn extract_sections(content: &str, query_lower: &str) -> Vec<MatchFragment> {
    let mut results = Vec::new();
    let mut sections: Vec<(usize, String)> = Vec::new();

    let mut current_start_line = 1;
    let mut current_section = String::new();
    let mut line_num = 0;

    for line in content.lines() {
        line_num += 1;
        if line.starts_with("## ") && !current_section.is_empty() {
            sections.push((current_start_line, current_section.clone()));
            current_section.clear();
            current_start_line = line_num;
        }
        if !current_section.is_empty() {
            current_section.push('\n');
        }
        current_section.push_str(line);
    }
    if !current_section.is_empty() {
        sections.push((current_start_line, current_section));
    }

    for (start_line, section) in sections {
        if section.to_lowercase().contains(query_lower) {
            results.push(MatchFragment {
                text: section.trim().to_string(),
                line_number: Some(start_line),
            });
        }
    }

    results
}

fn extract_file(content: &str, query_lower: &str) -> Vec<MatchFragment> {
    if content.to_lowercase().contains(query_lower) {
        vec![MatchFragment {
            text: content.trim().to_string(),
            line_number: Some(1),
        }]
    } else {
        vec![]
    }
}

// =====================================================================
// Memory collector for system prompt injection
// =====================================================================

/// Run-time collector for memory entries, similar to `ResourceCollector`.
pub struct MemoryCollector {
    pub global_shared_dir: Option<PathBuf>,
    pub global_workflow_dir: Option<PathBuf>,
    pub cwd_memory_dir: Option<PathBuf>,
    /// Workflow-level memory mode (from `.zug` file).
    pub workflow_mode: MemoryMode,
    /// Whether project-local memory is enabled globally.
    pub local_enabled: bool,
    /// When true, all tiers are skipped (e.g., `--no-memory` flag).
    pub disabled: bool,
}

impl MemoryCollector {
    /// Build a collector from the environment.
    pub fn from_env(
        workflow_name: &str,
        workflow_mode: MemoryMode,
        config: &ZigConfig,
        disabled: bool,
    ) -> Self {
        Self {
            global_shared_dir: paths::global_shared_memory_dir(),
            global_workflow_dir: paths::global_memory_for(workflow_name),
            cwd_memory_dir: paths::cwd_memory_dir(),
            workflow_mode,
            local_enabled: config.memory.local,
            disabled,
        }
    }

    /// Collect memory entries for a specific step, respecting mode overrides.
    ///
    /// Returns `(abs_path, id_string, entry)` tuples for rendering.
    pub fn collect_for_step(
        &self,
        step_memory: Option<&str>,
    ) -> Result<Vec<(PathBuf, String, MemoryEntry)>, ZigError> {
        if self.disabled {
            return Ok(Vec::new());
        }

        // Step mode overrides workflow mode.
        let effective_mode = if step_memory.is_some() {
            MemoryMode::from_str_opt(step_memory)
        } else {
            self.workflow_mode
        };

        if effective_mode == MemoryMode::None {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        let include_local = effective_mode == MemoryMode::All && self.local_enabled;

        // Global shared tier.
        if let Some(dir) = self.global_shared_dir.as_deref() {
            collect_from_dir(dir, &mut entries)?;
        }

        // Global per-workflow tier.
        if let Some(dir) = self.global_workflow_dir.as_deref() {
            collect_from_dir(dir, &mut entries)?;
        }

        // Project-local tier.
        if include_local {
            if let Some(dir) = self.cwd_memory_dir.as_deref() {
                collect_from_dir(dir, &mut entries)?;
            }
        }

        Ok(entries)
    }
}

fn collect_from_dir(
    dir: &Path,
    out: &mut Vec<(PathBuf, String, MemoryEntry)>,
) -> Result<(), ZigError> {
    if !dir.is_dir() {
        return Ok(());
    }
    let manifest = load_manifest(dir)?;
    for (id_str, entry) in &manifest.entries {
        let abs_path = dir.join(&entry.file);
        if abs_path.is_file() {
            out.push((abs_path, id_str.clone(), entry.clone()));
        }
    }
    Ok(())
}

/// Render a `<memory>` block to prepend to a system prompt.
///
/// Returns an empty string when there are no entries.
pub fn render_memory_block(
    entries: &[(PathBuf, String, MemoryEntry)],
    workflow_name: &str,
    step_name: Option<&str>,
) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let mut out = String::from("<memory>\n");
    out.push_str(
        "You have access to the following memory files — a scratch pad of accumulated knowledge. \
         Read them with your file tools when relevant.\n",
    );

    // Build the hint command with pre-filled --workflow and optional --step.
    let step_flag = step_name
        .map(|s| format!(" --step {s}"))
        .unwrap_or_default();
    out.push_str(&format!(
        "To add new memories: `zig memory add <path> --workflow {workflow_name}{step_flag}`\n"
    ));
    out.push_str(
        "To update metadata: `zig memory update <id> --description \"...\" --tags \"...\"`\n\n",
    );

    for (path, id, entry) in entries {
        out.push_str("- ");
        out.push_str(&path.display().to_string());
        if let Some(desc) = &entry.description {
            out.push_str(&format!(" (id: {id}) — {desc}"));
        } else {
            out.push_str(&format!(
                " (id: {id}, no description — run: zig memory update {id} --description \"...\")"
            ));
        }
        if !entry.tags.is_empty() {
            out.push_str(&format!(" [{}]", entry.tags.join(", ")));
        }
        out.push('\n');
    }
    out.push_str("</memory>\n\n");
    out
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
