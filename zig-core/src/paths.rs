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

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
