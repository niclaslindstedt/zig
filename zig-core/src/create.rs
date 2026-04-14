use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::error::ZigError;
use crate::prompt;
use crate::run;

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

/// Write all embedded example `.zug` files to `~/.zig/examples/`.
fn write_examples_to_global_dir() -> Result<(), ZigError> {
    let dir = crate::paths::ensure_global_examples_dir()?;
    for (filename, content) in prompt::all_examples() {
        let path = dir.join(filename);
        std::fs::write(&path, content)
            .map_err(|e| ZigError::Io(format!("failed to write {}: {e}", path.display())))?;
    }
    Ok(())
}

/// Return pattern-specific guidance sentences to inject into the initial prompt.
/// Includes the full example `.zug` content when a matching example exists.
fn pattern_guidance(pattern: &str) -> String {
    let description = match pattern {
        "sequential" => {
            "Use the Sequential Pipeline pattern: steps run in order, each feeding \
            its output to the next via inject_context. Structure it as a linear \
            depends_on chain."
        }
        "fan-out" => {
            "Use the Fan-Out / Gather pattern: multiple independent steps run in \
            parallel and a final step synthesizes their results. The gathering step \
            should depends_on all parallel steps with inject_context = true."
        }
        "generator-critic" => {
            "Use the Generator / Critic pattern: a generation step is followed by a \
            critique step that scores the output, looping back via the next field \
            until a quality threshold is met. Use saves to extract score and \
            feedback variables, and a condition to control the loop."
        }
        "coordinator-dispatcher" => {
            "Use the Coordinator / Dispatcher pattern: an initial classification \
            step routes to specialized handler steps via condition expressions. \
            Use json + saves to extract the classification, then condition guards \
            on each handler step."
        }
        "hierarchical-decomposition" => {
            "Use the Hierarchical Decomposition pattern: a planning step breaks the \
            problem into sub-tasks, multiple worker steps handle each sub-task in \
            parallel, and a synthesis step combines the results."
        }
        "human-in-the-loop" => {
            "Use the Human-in-the-Loop pattern: incorporate human approval gates \
            between automated steps. Use condition expressions on approval \
            variables that are set by human review."
        }
        "inter-agent-communication" => {
            "Use the Inter-Agent Communication pattern: agents collaborate via \
            shared variables. Design the vars section carefully so agents can \
            read and write to common state."
        }
        _ => return String::new(),
    };

    let mut guidance = description.to_string();
    if let Some(example) = prompt::example_for_pattern(pattern) {
        guidance
            .push_str("\n\nHere is a complete reference example for this pattern:\n\n```toml\n");
        guidance.push_str(example);
        guidance.push_str("\n```");
    }
    guidance
}

/// Prepared parameters for workflow creation — used by both the CLI
/// (which spawns zag directly) and the API (which returns data to the frontend).
#[derive(Debug, Clone, Serialize)]
pub struct CreateParams {
    pub system_prompt: String,
    pub initial_prompt: String,
    pub output_path: String,
    pub session_name: String,
    pub session_tag: String,
}

/// Prepare the prompts and configuration for workflow creation without
/// launching zag. Returns structured data that can be used by the CLI
/// to spawn zag or by the API to return to the frontend.
pub fn prepare_create(
    name: Option<&str>,
    output: Option<&str>,
    pattern: Option<&str>,
) -> Result<CreateParams, ZigError> {
    // Write example files to ~/.zig/examples/ so the agent can inspect them.
    if let Err(e) = write_examples_to_global_dir() {
        eprintln!("Warning: could not write example files: {e}");
    }

    let zag_help = get_zag_help();
    let zag_orch = get_zag_orch_reference();
    let system_prompt = build_system_prompt(&zag_help, &zag_orch);

    let output_path = if let Some(o) = output {
        o.to_string()
    } else {
        let global_dir = crate::paths::ensure_global_workflows_dir()?;
        let filename = name
            .map(|n| format!("{n}.zug"))
            .unwrap_or_else(|| "workflow.zug".to_string());
        global_dir.join(&filename).to_string_lossy().to_string()
    };

    let mut initial_prompt = if let Some(n) = name {
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

    if let Some(p) = pattern {
        let guidance = pattern_guidance(p);
        if !guidance.is_empty() {
            initial_prompt.push(' ');
            initial_prompt.push_str(&guidance);
        }
    }

    Ok(CreateParams {
        system_prompt,
        initial_prompt,
        output_path,
        session_name: "zig-create".to_string(),
        session_tag: "zig-workflow-creation".to_string(),
    })
}

/// Launch an interactive zag session for workflow creation.
///
/// The agent is given full context about zag and the .zug format, and guides
/// the user through designing their workflow.
pub fn run_create(
    name: Option<&str>,
    output: Option<&str>,
    pattern: Option<&str>,
) -> Result<(), ZigError> {
    run::check_zag()?;

    let params = prepare_create(name, output, pattern)?;

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

    // Validate the output if it was created
    if Path::new(&params.output_path).exists() {
        match crate::workflow::parser::parse_file(Path::new(&params.output_path)) {
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

#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
