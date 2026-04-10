use std::path::{Path, PathBuf};

use crate::error::ZigError;

/// Return the global workflows directory derived from a given home directory.
pub fn global_workflows_dir_from(home: &Path) -> PathBuf {
    home.join(".zig").join("workflows")
}

/// Return the global workflows directory: `~/.zig/workflows/`.
/// Returns `None` if the HOME environment variable is not set.
pub fn global_workflows_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|home| global_workflows_dir_from(Path::new(&home)))
}

/// Ensure the global workflows directory exists, creating it if necessary.
pub fn ensure_global_workflows_dir() -> Result<PathBuf, ZigError> {
    let dir = global_workflows_dir()
        .ok_or_else(|| ZigError::Io("HOME environment variable not set".into()))?;
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| ZigError::Io(format!("failed to create {}: {e}", dir.display())))?;
    }
    Ok(dir)
}

// =====================================================================
// Session storage paths.
//
// Layout mirrors zag (`zag-agent/src/config.rs:183` `resolve_project_dir`):
//
//   ~/.zig/
//     projects/<sanitized-project-path>/logs/
//                                       index.json
//                                       sessions/<id>.jsonl
//     sessions_index.json     (global cross-project index)
//
// Keeping this layout byte-for-byte aligned with `~/.zag/` is intentional
// so future changes to zag's session/listen architecture can be mirrored
// into zig with minimal churn.
// =====================================================================

/// Return the global zig base directory: `~/.zig/`.
pub fn global_base_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| Path::new(&h).join(".zig"))
}

/// Sanitize an absolute path into a directory name.
///
/// Strips leading `/` and replaces remaining `/` with `-`. Mirrors zag's
/// `Config::sanitize_path` (`zag-agent/src/config.rs:179`).
pub fn sanitize_project_path(path: &str) -> String {
    path.trim_start_matches('/').replace('/', "-")
}

/// Find the git repository root containing `start`, walking parents.
fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start;
    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

/// Resolve the project directory for session storage.
///
/// Mirrors zag's `Config::resolve_project_dir` (`zag-agent/src/config.rs:188`):
///   1. If `root` is provided, sanitize it directly.
///   2. Otherwise locate the git repository root containing `cwd`.
///   3. Otherwise fall back to the global base directory (no project subdir).
pub fn project_dir(root: Option<&str>) -> Option<PathBuf> {
    let base = global_base_dir()?;
    if let Some(r) = root {
        return Some(base.join("projects").join(sanitize_project_path(r)));
    }
    let cwd = std::env::current_dir().ok()?;
    if let Some(git_root) = find_git_root(&cwd) {
        let sanitized = sanitize_project_path(&git_root.to_string_lossy());
        return Some(base.join("projects").join(sanitized));
    }
    Some(base)
}

/// Return the per-project logs directory: `<project>/logs/`.
pub fn project_logs_dir(root: Option<&str>) -> Option<PathBuf> {
    project_dir(root).map(|p| p.join("logs"))
}

/// Return the per-project sessions directory: `<project>/logs/sessions/`.
pub fn project_sessions_dir(root: Option<&str>) -> Option<PathBuf> {
    project_logs_dir(root).map(|p| p.join("sessions"))
}

/// Return the per-project index file: `<project>/logs/index.json`.
pub fn project_index_path(root: Option<&str>) -> Option<PathBuf> {
    project_logs_dir(root).map(|p| p.join("index.json"))
}

/// Return the global cross-project session index: `~/.zig/sessions_index.json`.
pub fn global_sessions_index_path() -> Option<PathBuf> {
    global_base_dir().map(|p| p.join("sessions_index.json"))
}

/// Ensure the per-project sessions directory exists, creating it if needed.
pub fn ensure_project_sessions_dir(root: Option<&str>) -> Result<PathBuf, ZigError> {
    let dir = project_sessions_dir(root)
        .ok_or_else(|| ZigError::Io("HOME environment variable not set".into()))?;
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| ZigError::Io(format!("failed to create {}: {e}", dir.display())))?;
    }
    Ok(dir)
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
