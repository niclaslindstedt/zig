/// Embedded manpage content, compiled from `manpages/` markdown files.
mod pages {
    pub const ZIG: &str = include_str!("../../manpages/zig.md");
    pub const RUN: &str = include_str!("../../manpages/run.md");
    pub const CREATE: &str = include_str!("../../manpages/create.md");
    pub const DESCRIBE: &str = include_str!("../../manpages/describe.md");
    pub const VALIDATE: &str = include_str!("../../manpages/validate.md");
    pub const ZUG: &str = include_str!("../../manpages/zug.md");
    pub const PATTERNS: &str = include_str!("../../manpages/patterns.md");
    pub const VARIABLES: &str = include_str!("../../manpages/variables.md");
    pub const CONDITIONS: &str = include_str!("../../manpages/conditions.md");
}

/// All available manpage topics in display order.
pub const TOPICS: &[(&str, &str)] = &[
    ("zig", "Overview of the zig CLI"),
    ("run", "Execute a .zug workflow file"),
    ("create", "Create a new workflow interactively"),
    ("describe", "Generate a .zug file from a prompt"),
    ("validate", "Validate a .zug workflow file"),
    ("zug", "The .zug workflow format"),
    ("patterns", "Orchestration patterns"),
    ("variables", "Variable system and data flow"),
    ("conditions", "Condition expressions"),
];

/// Look up a manpage by topic name.
///
/// Returns the markdown content if the topic exists, or `None`.
pub fn get(topic: &str) -> Option<&'static str> {
    match topic {
        "zig" => Some(pages::ZIG),
        "run" => Some(pages::RUN),
        "create" => Some(pages::CREATE),
        "describe" => Some(pages::DESCRIBE),
        "validate" => Some(pages::VALIDATE),
        "zug" => Some(pages::ZUG),
        "patterns" => Some(pages::PATTERNS),
        "variables" => Some(pages::VARIABLES),
        "conditions" => Some(pages::CONDITIONS),
        _ => None,
    }
}

/// List all available manpage topics with their descriptions.
pub fn list_topics() -> String {
    let mut out = String::from("Available manpages:\n\n");
    let max_name_len = TOPICS.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    for (name, description) in TOPICS {
        out.push_str(&format!(
            "  {name:<width$}  {description}\n",
            width = max_name_len
        ));
    }
    out.push_str("\nUsage: zig man <topic>");
    out
}

#[cfg(test)]
#[path = "man_tests.rs"]
mod tests;
