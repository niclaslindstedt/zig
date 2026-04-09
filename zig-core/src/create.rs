use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

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

/// Return pattern-specific guidance sentences to inject into the initial prompt.
fn pattern_guidance(pattern: &str) -> &'static str {
    match pattern {
        "sequential" => {
            "\
            Use the Sequential Pipeline pattern: steps run in order, each feeding \
            its output to the next via inject_context. Structure it as a linear \
            depends_on chain."
        }
        "fan-out" => {
            "\
            Use the Fan-Out / Gather pattern: multiple independent steps run in \
            parallel and a final step synthesizes their results. The gathering step \
            should depends_on all parallel steps with inject_context = true."
        }
        "generator-critic" => {
            "\
            Use the Generator / Critic pattern: a generation step is followed by a \
            critique step that scores the output, looping back via the next field \
            until a quality threshold is met. Use saves to extract score and \
            feedback variables, and a condition to control the loop."
        }
        "coordinator-dispatcher" => {
            "\
            Use the Coordinator / Dispatcher pattern: an initial classification \
            step routes to specialized handler steps via condition expressions. \
            Use json + saves to extract the classification, then condition guards \
            on each handler step."
        }
        "hierarchical-decomposition" => {
            "\
            Use the Hierarchical Decomposition pattern: a planning step breaks the \
            problem into sub-tasks, multiple worker steps handle each sub-task in \
            parallel, and a synthesis step combines the results."
        }
        "human-in-the-loop" => {
            "\
            Use the Human-in-the-Loop pattern: incorporate human approval gates \
            between automated steps. Use condition expressions on approval \
            variables that are set by human review."
        }
        "inter-agent-communication" => {
            "\
            Use the Inter-Agent Communication pattern: agents collaborate via \
            shared variables. Design the vars section carefully so agents can \
            read and write to common state."
        }
        _ => "",
    }
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

    let zag_help = get_zag_help();
    let zag_orch = get_zag_orch_reference();
    let system_prompt = build_system_prompt(&zag_help, &zag_orch);

    let output_path = output
        .map(|s| s.to_string())
        .or_else(|| name.map(|n| format!("{n}.zug")))
        .unwrap_or_else(|| "workflow.zug".to_string());

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
            initial_prompt.push_str(guidance);
        }
    }

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

#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
