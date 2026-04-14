use std::collections::HashMap;

/// Prompt templates are embedded at compile time from `prompts/`.
///
/// Markdown prompt files carry YAML front matter (`name`, `version`,
/// `description`, `references`) that is metadata for humans and tooling. It
/// must never be sent to agents, so the accessors below strip front matter
/// before returning the template content.
pub mod templates {
    use std::sync::LazyLock;

    /// System prompt for `zig create` — the interactive workflow design agent.
    pub fn create() -> &'static str {
        static STRIPPED: LazyLock<&'static str> =
            LazyLock::new(|| super::strip_front_matter(include_str!("../prompts/create/1_3.md")));
        *STRIPPED
    }

    /// .zug format specification — injected as a reference sidecar into prompts
    /// that need to produce or reason about `.zug` files.
    pub fn config_sidecar() -> &'static str {
        static STRIPPED: LazyLock<&'static str> = LazyLock::new(|| {
            super::strip_front_matter(include_str!("../prompts/config-sidecar/1_2.md"))
        });
        *STRIPPED
    }

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

/// Strip YAML front matter from a prompt template.
///
/// Front matter is the block at the start of the file delimited by `---` lines:
///
/// ```text
/// ---
/// name: my-prompt
/// version: "1.0"
/// ---
///
/// the actual prompt body starts here
/// ```
///
/// The leading delimiter must be on the very first line. If no front matter is
/// present, the input is returned unchanged. A single blank line immediately
/// following the closing delimiter is also consumed so the returned content
/// starts with the prompt body.
///
/// Both LF and CRLF line endings are supported so the helper behaves the same
/// on Unix and Windows checkouts.
pub fn strip_front_matter(content: &str) -> &str {
    // Match the opening delimiter line (`---` followed by LF or CRLF).
    let rest = if let Some(r) = content.strip_prefix("---\n") {
        r
    } else if let Some(r) = content.strip_prefix("---\r\n") {
        r
    } else {
        return content;
    };

    // Scan line-by-line for the closing `---` delimiter so we tolerate either
    // line ending and a missing trailing newline.
    let mut offset = 0;
    while offset <= rest.len() {
        let remainder = &rest[offset..];
        let (line, advance) = match remainder.find('\n') {
            Some(nl) => (&remainder[..nl], nl + 1),
            None => (remainder, remainder.len()),
        };
        let trimmed = line.strip_suffix('\r').unwrap_or(line);
        if trimmed == "---" {
            let body_start = offset + advance;
            let body = &rest[body_start..];
            // Consume one optional blank line after the closing delimiter so
            // the returned content starts at the prompt body.
            return body
                .strip_prefix("\r\n")
                .or_else(|| body.strip_prefix('\n'))
                .unwrap_or(body);
        }
        if advance == 0 {
            break;
        }
        offset += advance;
    }

    // No closing delimiter found — treat as no front matter rather than
    // swallowing the whole file.
    content
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
