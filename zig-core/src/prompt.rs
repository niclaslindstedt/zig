use std::collections::HashMap;

/// Prompt templates are embedded at compile time from `prompts/`.
pub mod templates {
    /// System prompt for `zig create` — the interactive workflow design agent.
    pub const CREATE: &str = include_str!("../prompts/create/1_3.md");

    /// .zug format specification — injected as a reference sidecar into prompts
    /// that need to produce or reason about `.zug` files.
    pub const CONFIG_SIDECAR: &str = include_str!("../prompts/config-sidecar/1_2.md");

    /// Example `.zug` files for each orchestration pattern — embedded at compile
    /// time and written to `~/.zig/examples/` during workflow creation.
    pub mod examples {
        pub const SEQUENTIAL: &str = include_str!("../prompts/examples/sequential.zug");
        pub const FAN_OUT: &str = include_str!("../prompts/examples/fan-out.zug");
        pub const GENERATOR_CRITIC: &str = include_str!("../prompts/examples/generator-critic.zug");
        pub const COORDINATOR_DISPATCHER: &str =
            include_str!("../prompts/examples/coordinator-dispatcher.zug");
        pub const HIERARCHICAL_DECOMPOSITION: &str =
            include_str!("../prompts/examples/hierarchical-decomposition.zug");
        pub const HUMAN_IN_THE_LOOP: &str =
            include_str!("../prompts/examples/human-in-the-loop.zug");
        pub const INTER_AGENT_COMMUNICATION: &str =
            include_str!("../prompts/examples/inter-agent-communication.zug");
    }
}

/// Return the embedded example `.zug` content for a given pattern name.
pub fn example_for_pattern(pattern: &str) -> Option<&'static str> {
    match pattern {
        "sequential" => Some(templates::examples::SEQUENTIAL),
        "fan-out" => Some(templates::examples::FAN_OUT),
        "generator-critic" => Some(templates::examples::GENERATOR_CRITIC),
        "coordinator-dispatcher" => Some(templates::examples::COORDINATOR_DISPATCHER),
        "hierarchical-decomposition" => Some(templates::examples::HIERARCHICAL_DECOMPOSITION),
        "human-in-the-loop" => Some(templates::examples::HUMAN_IN_THE_LOOP),
        "inter-agent-communication" => Some(templates::examples::INTER_AGENT_COMMUNICATION),
        _ => None,
    }
}

/// Return all embedded example files as `(filename, content)` pairs.
pub fn all_examples() -> Vec<(&'static str, &'static str)> {
    vec![
        ("sequential.zug", templates::examples::SEQUENTIAL),
        ("fan-out.zug", templates::examples::FAN_OUT),
        (
            "generator-critic.zug",
            templates::examples::GENERATOR_CRITIC,
        ),
        (
            "coordinator-dispatcher.zug",
            templates::examples::COORDINATOR_DISPATCHER,
        ),
        (
            "hierarchical-decomposition.zug",
            templates::examples::HIERARCHICAL_DECOMPOSITION,
        ),
        (
            "human-in-the-loop.zug",
            templates::examples::HUMAN_IN_THE_LOOP,
        ),
        (
            "inter-agent-communication.zug",
            templates::examples::INTER_AGENT_COMMUNICATION,
        ),
    ]
}

/// Render a prompt template by replacing `{{variable}}` placeholders with
/// values from the provided map.
///
/// Unknown variables are left as-is so callers can detect missing bindings.
pub fn render(template: &str, vars: &HashMap<&str, &str>) -> String {
    let mut result = template.to_string();
    for (&key, &value) in vars {
        let placeholder = format!("{{{{{key}}}}}");
        result = result.replace(&placeholder, value);
    }
    result
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;
