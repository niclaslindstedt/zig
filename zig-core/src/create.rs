use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use crate::error::ZigError;
use crate::prompt;

/// Build the system prompt for the create agent by rendering the template
/// with injected variables.
fn build_system_prompt(zag_help: &str, zag_orch: &str) -> String {
    let vars = HashMap::from([
        ("zug_format_spec", prompt::templates::CONFIG_SIDECAR),
        ("zag_help", zag_help),
        ("zag_orch", zag_orch),
    ]);
    prompt::render(prompt::templates::CREATE, &vars)
}

/// Attempt to capture zag CLI reference text via `zag --help-agent`.
fn get_zag_help() -> String {
    Command::new("zag")
        .arg("--help-agent")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "(zag --help-agent not available — zag may not be installed)".into())
}

/// Attempt to capture zag orchestration reference via `zag man orchestration`.
fn get_zag_orch_reference() -> String {
    Command::new("zag")
        .args(["man", "orchestration"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| {
            "(zag man orchestration not available — zag may not be installed)".into()
        })
}

/// Launch an interactive zag session for workflow creation.
///
/// The agent is given full context about zag and the .zug format, and guides
/// the user through designing their workflow.
pub fn run_create(name: Option<&str>, output: Option<&str>) -> Result<(), ZigError> {
    // Check that zag is available
    let zag_available = Command::new("zag")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());

    if !zag_available {
        return Err(ZigError::Zag(
            "zag is not installed or not in PATH. Install it from https://github.com/niclaslindstedt/zag".into(),
        ));
    }

    let zag_help = get_zag_help();
    let zag_orch = get_zag_orch_reference();
    let system_prompt = build_system_prompt(&zag_help, &zag_orch);

    let output_path = output
        .map(|s| s.to_string())
        .or_else(|| name.map(|n| format!("{n}.zug")))
        .unwrap_or_else(|| "workflow.zug".to_string());

    let initial_prompt = if let Some(n) = name {
        format!(
            "I want to create a workflow called \"{n}\". \
             The output will be saved to {output_path}. \
             Please help me design it — start by asking what process I want to automate."
        )
    } else {
        format!(
            "I want to create a new workflow. \
             The output will be saved to {output_path}. \
             Please help me design it — start by asking what process I want to automate."
        )
    };

    let status = Command::new("zag")
        .args(["run", &initial_prompt])
        .args(["--system-prompt", &system_prompt])
        .args(["--name", "zig-create"])
        .args(["--tag", "zig-workflow-creation"])
        .status()
        .map_err(|e| ZigError::Zag(format!("failed to launch zag: {e}")))?;

    if !status.success() {
        return Err(ZigError::Zag(format!("zag exited with status {status}")));
    }

    // Validate the output if it was created
    if Path::new(&output_path).exists() {
        match crate::workflow::parser::parse_file(Path::new(&output_path)) {
            Ok(workflow) => {
                if let Err(errors) = crate::workflow::validate::validate(&workflow) {
                    eprintln!("Warning: generated workflow has validation issues:");
                    for e in &errors {
                        eprintln!("  - {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: could not parse generated file: {e}");
            }
        }
    }

    Ok(())
}
