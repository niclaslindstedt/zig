use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::ZigError;
use crate::workflow::parser;

/// Pack a workflow directory into a `.zug` zip archive.
///
/// The directory must contain exactly one workflow TOML file (`.toml` or `.zug`
/// that contains a `[workflow]` section). All files in the directory are included
/// in the archive. The resulting zip file can be used directly with `zig run`
/// and `zig validate`.
pub fn pack(dir_path: &str, output: Option<&str>) -> Result<PathBuf, ZigError> {
    let dir = Path::new(dir_path);
    if !dir.is_dir() {
        return Err(ZigError::Io(format!(
            "'{}' is not a directory",
            dir.display()
        )));
    }

    // Find the workflow TOML file
    let toml_file = find_workflow_toml(dir)?;

    // Validate it parses correctly
    let content = std::fs::read_to_string(&toml_file)
        .map_err(|e| ZigError::Io(format!("failed to read {}: {e}", toml_file.display())))?;
    let wf = parser::parse(&content)?;

    // Determine output path
    let output_path = if let Some(out) = output {
        PathBuf::from(out)
    } else {
        let name = wf.workflow.name.replace(' ', "-").to_lowercase();
        PathBuf::from(format!("{name}.zug"))
    };

    // Collect all files in the directory
    let files = collect_files(dir)?;

    // Create the zip archive
    let file = std::fs::File::create(&output_path)
        .map_err(|e| ZigError::Io(format!("failed to create {}: {e}", output_path.display())))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for file_path in &files {
        let relative = file_path
            .strip_prefix(dir)
            .map_err(|e| ZigError::Io(format!("path error: {e}")))?;
        let name = relative.to_string_lossy().replace('\\', "/");

        zip.start_file(&name, options)
            .map_err(|e| ZigError::Io(format!("failed to add {name} to archive: {e}")))?;

        let contents = std::fs::read(file_path)
            .map_err(|e| ZigError::Io(format!("failed to read {}: {e}", file_path.display())))?;
        zip.write_all(&contents)
            .map_err(|e| ZigError::Io(format!("failed to write {name} to archive: {e}")))?;
    }

    zip.finish()
        .map_err(|e| ZigError::Io(format!("failed to finalize archive: {e}")))?;

    eprintln!(
        "packed {} files into '{}' (workflow: '{}')",
        files.len(),
        output_path.display(),
        wf.workflow.name
    );

    Ok(output_path)
}

/// Find the single workflow TOML file in a directory.
fn find_workflow_toml(dir: &Path) -> Result<PathBuf, ZigError> {
    let mut candidates = Vec::new();

    for entry in std::fs::read_dir(dir)
        .map_err(|e| ZigError::Io(format!("failed to read directory {}: {e}", dir.display())))?
    {
        let entry =
            entry.map_err(|e| ZigError::Io(format!("failed to read directory entry: {e}")))?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "toml" || ext == "zug" {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if content.contains("[workflow]") {
                            candidates.push(path);
                        }
                    }
                }
            }
        }
    }

    match candidates.len() {
        0 => Err(ZigError::Io(format!(
            "no workflow TOML file found in '{}'",
            dir.display()
        ))),
        1 => Ok(candidates.into_iter().next().unwrap()),
        n => Err(ZigError::Io(format!(
            "found {n} workflow files in '{}' (expected exactly one): {}",
            dir.display(),
            candidates
                .iter()
                .map(|p| p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Recursively collect all files in a directory.
fn collect_files(dir: &Path) -> Result<Vec<PathBuf>, ZigError> {
    let mut files = Vec::new();

    fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), ZigError> {
        for entry in std::fs::read_dir(dir)
            .map_err(|e| ZigError::Io(format!("failed to read directory: {e}")))?
        {
            let entry =
                entry.map_err(|e| ZigError::Io(format!("failed to read directory entry: {e}")))?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, files)?;
            } else {
                files.push(path);
            }
        }
        Ok(())
    }

    walk(dir, &mut files)?;
    files.sort();
    Ok(files)
}

#[cfg(test)]
#[path = "pack_tests.rs"]
mod tests;
