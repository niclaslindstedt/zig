use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::ZigError;
use crate::workflow::model::Workflow;

/// Parse a `.zug` workflow file from a TOML string.
pub fn parse(content: &str) -> Result<Workflow, ZigError> {
    let workflow: Workflow = toml::from_str(content).map_err(|e| ZigError::Parse(e.to_string()))?;
    Ok(workflow)
}

/// Parse a `.zug` workflow file from disk.
///
/// If the file is a zip archive, it is extracted to a temp directory and
/// the TOML workflow inside is parsed. The returned `WorkflowSource` must
/// be kept alive for the duration of execution — dropping it cleans up
/// any temp directory.
pub fn parse_file(path: &Path) -> Result<Workflow, ZigError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ZigError::Io(format!("failed to read {}: {e}", path.display())))?;
    parse(&content)
}

/// Parse a `.zug` file, handling both plain TOML and zip archives.
///
/// Returns the parsed `Workflow` and a `WorkflowSource` that tracks the
/// effective directory for resolving relative file paths. The source must
/// be kept alive during execution.
pub fn parse_workflow(path: &Path) -> Result<(Workflow, WorkflowSource), ZigError> {
    if is_zip_archive(path)? {
        parse_zip(path)
    } else {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ZigError::Io(format!("failed to read {}: {e}", path.display())))?;
        let wf = parse(&content)?;
        let dir = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Ok((wf, WorkflowSource::Directory(dir)))
    }
}

/// Tracks where a workflow's associated files live.
///
/// For plain `.zug` files, this is the parent directory. For zip archives,
/// this is a temp directory containing the extracted contents. Dropping
/// the `Zip` variant cleans up the temp directory.
#[derive(Debug)]
pub enum WorkflowSource {
    /// Plain TOML file on disk — resolve paths relative to this directory.
    Directory(PathBuf),
    /// Extracted zip archive — temp dir is cleaned up on drop.
    Zip {
        _temp_dir: tempfile::TempDir,
        extract_dir: PathBuf,
    },
}

impl WorkflowSource {
    /// Get the effective directory for resolving relative file paths.
    pub fn dir(&self) -> &Path {
        match self {
            WorkflowSource::Directory(dir) => dir,
            WorkflowSource::Zip { extract_dir, .. } => extract_dir,
        }
    }
}

/// Check if a file is a zip archive by reading its magic bytes.
fn is_zip_archive(path: &Path) -> Result<bool, ZigError> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| ZigError::Io(format!("failed to open {}: {e}", path.display())))?;
    let mut magic = [0u8; 4];
    match file.read_exact(&mut magic) {
        Ok(()) => Ok(&magic == b"PK\x03\x04"),
        Err(_) => Ok(false), // File too short to be a zip
    }
}

/// Parse a `.zug` zip archive.
///
/// Extracts the archive to a temp directory, finds the single TOML workflow
/// file inside, and parses it.
fn parse_zip(path: &Path) -> Result<(Workflow, WorkflowSource), ZigError> {
    let file = std::fs::File::open(path)
        .map_err(|e| ZigError::Io(format!("failed to open {}: {e}", path.display())))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ZigError::Parse(format!("failed to read zip archive: {e}")))?;

    let temp_dir = tempfile::TempDir::new()
        .map_err(|e| ZigError::Io(format!("failed to create temp directory: {e}")))?;

    // Extract all files
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ZigError::Parse(format!("failed to read zip entry: {e}")))?;

        let out_path = temp_dir.path().join(
            entry
                .enclosed_name()
                .ok_or_else(|| ZigError::Parse("zip entry has invalid path".into()))?,
        );

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| {
                ZigError::Io(format!(
                    "failed to create directory {}: {e}",
                    out_path.display()
                ))
            })?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ZigError::Io(format!(
                        "failed to create directory {}: {e}",
                        parent.display()
                    ))
                })?;
            }
            let mut outfile = std::fs::File::create(&out_path).map_err(|e| {
                ZigError::Io(format!("failed to create file {}: {e}", out_path.display()))
            })?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| {
                ZigError::Io(format!("failed to extract {}: {e}", out_path.display()))
            })?;
        }
    }

    // Find the single TOML workflow file
    let toml_files: Vec<PathBuf> = find_toml_files(temp_dir.path())?;

    if toml_files.is_empty() {
        return Err(ZigError::Parse(
            "zip archive contains no .toml or .zug workflow file".into(),
        ));
    }
    if toml_files.len() > 1 {
        return Err(ZigError::Parse(format!(
            "zip archive contains {} workflow files (expected exactly one): {}",
            toml_files.len(),
            toml_files
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let toml_path = &toml_files[0];
    let content = std::fs::read_to_string(toml_path)
        .map_err(|e| ZigError::Io(format!("failed to read {}: {e}", toml_path.display())))?;
    let wf = parse(&content)?;

    // The effective dir is the parent of the toml file within the temp dir
    let extract_dir = toml_path.parent().unwrap_or(temp_dir.path()).to_path_buf();

    Ok((
        wf,
        WorkflowSource::Zip {
            _temp_dir: temp_dir,
            extract_dir,
        },
    ))
}

/// Recursively find `.toml` and `.zug` files in a directory (non-recursive,
/// only checks the top level and immediate subdirectories).
fn find_toml_files(dir: &Path) -> Result<Vec<PathBuf>, ZigError> {
    let mut results = Vec::new();

    fn scan_dir(dir: &Path, results: &mut Vec<PathBuf>, depth: usize) -> Result<(), ZigError> {
        let entries = std::fs::read_dir(dir).map_err(|e| {
            ZigError::Io(format!("failed to read directory {}: {e}", dir.display()))
        })?;

        for entry in entries {
            let entry =
                entry.map_err(|e| ZigError::Io(format!("failed to read directory entry: {e}")))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "toml" || ext == "zug" {
                        // Quick check: does it look like a workflow TOML?
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if content.contains("[workflow]") {
                                results.push(path);
                            }
                        }
                    }
                }
            } else if path.is_dir() && depth < 1 {
                scan_dir(&path, results, depth + 1)?;
            }
        }
        Ok(())
    }

    scan_dir(dir, &mut results, 0)?;
    Ok(results)
}

/// Serialize a workflow back to TOML (for the `create` command).
pub fn to_toml(workflow: &Workflow) -> Result<String, ZigError> {
    toml::to_string_pretty(workflow).map_err(|e| ZigError::Serialize(e.to_string()))
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
