use std::collections::HashMap;

/// Prompt templates are embedded at compile time from `prompts/`.
pub mod templates {
    /// System prompt for `zig create` — the interactive workflow design agent.
    pub const CREATE: &str = include_str!("../../prompts/create/1_0.md");

    /// .zug format specification — injected as a reference sidecar into prompts
    /// that need to produce or reason about `.zug` files.
    pub const CONFIG_SIDECAR: &str = include_str!("../../prompts/config-sidecar/1_0.md");
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
