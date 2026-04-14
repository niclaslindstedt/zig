use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::create::{get_zag_help, get_zag_orch_reference};
use crate::error::ZigError;
use crate::pack;
use crate::prompt;
use crate::run;
use crate::workflow::parser;

/// File kind on disk — either a plain `.zwf` TOML file or a zipped `.zwfz`
/// archive. Determines how the binary stages the workflow for editing and how
/// it writes it back when the agent session ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WorkflowKind {
    Plain,
    Zipped,
}

impl WorkflowKind {
    fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|s| s.to_str()) {
            Some("zwfz") => WorkflowKind::Zipped,
            _ => WorkflowKind::Plain,
        }
    }
}

/// Prepared parameters for workflow update — the system prompt, initial user
/// prompt, and the staging path that the agent should edit in place.
#[derive(Debug, Serialize)]
pub struct UpdateParams {
    pub system_prompt: String,
    pub initial_prompt: String,
    pub original_path: PathBuf,
    pub staging_path: PathBuf,
    pub kind: WorkflowKind,
    pub session_name: String,
    pub session_tag: String,
    /// Owned tempdir that holds the staging copy. Kept alive until the
    /// update completes so the scratch files are available to the agent;
    /// dropped to clean them up afterwards.
    #[serde(skip)]
    pub _staging_dir: tempfile::TempDir,
}

fn build_system_prompt(zag_help: &str, zag_orch: &str, examples_reference: &str) -> String {
    let vars = HashMap::from([
        ("zwf_format_spec", prompt::templates::config_sidecar()),
        ("zag_help", zag_help),
        ("zag_orch", zag_orch),
        ("examples_reference", examples_reference),
    ]);
    prompt::render(prompt::templates::update(), &vars)
}

/// Prepare an update session without launching zag. Resolves the workflow,
/// copies or unzips it to a tempdir, validates that it parses, and builds
/// the system + initial prompts.
pub fn prepare_update(workflow: &str) -> Result<UpdateParams, ZigError> {
    // Make sure the agent can read the canonical examples.
    if let Err(e) = prompt::write_examples_to_global_dir() {
        eprintln!("Warning: could not write example files: {e}");
    }

    let original_path = run::resolve_workflow_path(workflow)?;
    let kind = WorkflowKind::from_path(&original_path);

    let staging_dir = tempfile::TempDir::new()
        .map_err(|e| ZigError::Io(format!("failed to create staging directory: {e}")))?;

    let staging_path = match kind {
        WorkflowKind::Plain => {
            let file_name = original_path
                .file_name()
                .ok_or_else(|| ZigError::Io("workflow path has no file name".into()))?;
            let dest = staging_dir.path().join(file_name);
            std::fs::copy(&original_path, &dest).map_err(|e| {
                ZigError::Io(format!(
                    "failed to copy {} to staging: {e}",
                    original_path.display()
                ))
            })?;
            dest
        }
        WorkflowKind::Zipped => {
            parser::extract_zip(&original_path, staging_dir.path())?;
            let toml_files = parser::find_workflow_files(staging_dir.path())?;
            match toml_files.len() {
                0 => {
                    return Err(ZigError::Parse(
                        "archive contains no .toml or .zwf workflow file".into(),
                    ));
                }
                1 => toml_files.into_iter().next().unwrap(),
                n => {
                    return Err(ZigError::Parse(format!(
                        "archive contains {n} workflow files (expected exactly one)"
                    )));
                }
            }
        }
    };

    // Parse up-front to fail fast on a broken workflow before paying for a zag session.
    parser::parse_file(&staging_path)?;

    let zag_help = get_zag_help();
    let zag_orch = get_zag_orch_reference();
    let examples_reference = prompt::examples_reference_block();
    let system_prompt = build_system_prompt(&zag_help, &zag_orch, &examples_reference);

    let initial_prompt = format!(
        "I want to update the workflow file at `{}`. Please read it first, \
         then help me make the changes I describe. Edit the file in place at \
         that exact path — do not rename, move, or copy it.",
        staging_path.display()
    );

    Ok(UpdateParams {
        system_prompt,
        initial_prompt,
        original_path,
        staging_path,
        kind,
        session_name: "zig-update".to_string(),
        session_tag: "zig-workflow-update".to_string(),
        _staging_dir: staging_dir,
    })
}

/// Launch an interactive zag session for workflow revision.
///
/// Flow:
/// 1. Resolve the workflow by name or path.
/// 2. Copy (plain `.zwf`) or unzip (`.zwfz`) it into a temp staging directory.
/// 3. Spawn `zag run` with an update system prompt and the staging path.
/// 4. On success, move (plain) or re-zip (zipped) the staging contents back
///    over the original path.
pub fn run_update(workflow: &str) -> Result<(), ZigError> {
    run::check_zag()?;

    let params = prepare_update(workflow)?;

    let status = Command::new("zag")
        .args(["run", &params.initial_prompt])
        .args(["--system-prompt", &params.system_prompt])
        .args(["--name", &params.session_name])
        .args(["--tag", &params.session_tag])
        .status()
        .map_err(|e| ZigError::Zag(format!("failed to launch zag: {e}")))?;

    if !status.success() {
        return Err(ZigError::Zag(format!("zag exited with status {status}")));
    }

    // Re-validate the edited workflow; warn (but do not abort) on issues so
    // the user still ends up with the file the agent produced.
    if params.staging_path.exists() {
        match parser::parse_file(&params.staging_path) {
            Ok(workflow) => {
                if let Err(errors) = crate::workflow::validate::validate(&workflow) {
                    eprintln!("Warning: updated workflow has validation issues:");
                    for e in &errors {
                        eprintln!("  - {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: could not parse updated file: {e}");
            }
        }
    } else {
        return Err(ZigError::Io(format!(
            "expected updated workflow at {} but the file is missing — \
             did the agent move or rename it?",
            params.staging_path.display()
        )));
    }

    commit_update(&params)?;

    println!("updated {}", params.original_path.display());
    Ok(())
}

/// Move (plain) or re-zip (zipped) the staging copy back over the original
/// workflow path. Writes through a sibling temp path + rename so a failure
/// mid-write can't leave the original file truncated.
fn commit_update(params: &UpdateParams) -> Result<(), ZigError> {
    match params.kind {
        WorkflowKind::Plain => {
            let tmp = sibling_temp_path(&params.original_path)?;
            std::fs::copy(&params.staging_path, &tmp).map_err(|e| {
                ZigError::Io(format!(
                    "failed to write updated workflow to {}: {e}",
                    tmp.display()
                ))
            })?;
            std::fs::rename(&tmp, &params.original_path).map_err(|e| {
                ZigError::Io(format!(
                    "failed to replace {}: {e}",
                    params.original_path.display()
                ))
            })?;
        }
        WorkflowKind::Zipped => {
            let tmp = sibling_temp_path(&params.original_path)?;
            pack::zip_directory(params._staging_dir.path(), &tmp)?;
            std::fs::rename(&tmp, &params.original_path).map_err(|e| {
                ZigError::Io(format!(
                    "failed to replace {}: {e}",
                    params.original_path.display()
                ))
            })?;
        }
    }
    Ok(())
}

/// Build a sibling path next to `target` suitable for an atomic-rename
/// write-then-rename sequence.
fn sibling_temp_path(target: &Path) -> Result<PathBuf, ZigError> {
    let parent = target
        .parent()
        .ok_or_else(|| ZigError::Io("workflow path has no parent directory".into()))?;
    let file_name = target
        .file_name()
        .ok_or_else(|| ZigError::Io("workflow path has no file name".into()))?
        .to_string_lossy();
    let pid = std::process::id();
    Ok(parent.join(format!(".{file_name}.update.{pid}.tmp")))
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
