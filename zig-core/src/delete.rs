use std::fs;

use crate::error::ZigError;
use crate::run::resolve_workflow_path;

/// Delete a `.zug` workflow file.
///
/// Resolves the workflow name or path using the same resolution logic as
/// `zig run`, then removes the file from disk.
pub fn run_delete(workflow: &str) -> Result<(), ZigError> {
    let path = resolve_workflow_path(workflow)?;

    eprintln!("deleting workflow: {}", path.display());

    fs::remove_file(&path)
        .map_err(|e| ZigError::Io(format!("failed to delete '{}': {e}", path.display())))?;

    eprintln!("deleted {}", path.display());
    Ok(())
}

#[cfg(test)]
#[path = "delete_tests.rs"]
mod tests;
