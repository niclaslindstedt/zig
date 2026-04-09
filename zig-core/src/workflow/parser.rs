use std::path::Path;

use crate::error::ZigError;
use crate::workflow::model::Workflow;

/// Parse a `.zug` workflow file from a TOML string.
pub fn parse(content: &str) -> Result<Workflow, ZigError> {
    let workflow: Workflow = toml::from_str(content).map_err(|e| ZigError::Parse(e.to_string()))?;
    Ok(workflow)
}

/// Parse a `.zug` workflow file from disk.
pub fn parse_file(path: &Path) -> Result<Workflow, ZigError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ZigError::Io(format!("failed to read {}: {e}", path.display())))?;
    parse(&content)
}

/// Serialize a workflow back to TOML (for the `create` command).
pub fn to_toml(workflow: &Workflow) -> Result<String, ZigError> {
    toml::to_string_pretty(workflow).map_err(|e| ZigError::Serialize(e.to_string()))
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
