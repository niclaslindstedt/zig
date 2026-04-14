/// Embedded manpage content, compiled from `manpages/` markdown files.
mod pages {
    pub const ZIG: &str = include_str!("../manpages/zig.md");
    pub const RUN: &str = include_str!("../manpages/run.md");
    pub const LISTEN: &str = include_str!("../manpages/listen.md");
    pub const SERVE: &str = include_str!("../manpages/serve.md");
    pub const WORKFLOW: &str = include_str!("../manpages/workflow.md");
    pub const DESCRIBE: &str = include_str!("../manpages/describe.md");
    pub const VALIDATE: &str = include_str!("../manpages/validate.md");
    pub const RESOURCES: &str = include_str!("../manpages/resources.md");
}

/// All available manpage topics in display order.
pub const TOPICS: &[(&str, &str)] = &[
    ("zig", "Overview of the zig CLI"),
    ("run", "Execute a .zwf/.zwfz workflow file"),
    ("listen", "Tail a running or completed zig session"),
    ("serve", "Start an HTTP API server"),
    (
        "workflow",
        "Manage workflows (list, show, create, update, delete, pack)",
    ),
    ("describe", "Generate a .zwf file from a prompt"),
    ("validate", "Validate a .zwf or .zwfz workflow file"),
    (
        "resources",
        "Manage reference files advertised to step agents",
    ),
];

/// Look up a manpage by topic name.
///
/// Returns the markdown content if the topic exists, or `None`.
pub fn get(topic: &str) -> Option<&'static str> {
    match topic {
        "zig" => Some(pages::ZIG),
        "run" => Some(pages::RUN),
        "listen" => Some(pages::LISTEN),
        "serve" => Some(pages::SERVE),
        "workflow" => Some(pages::WORKFLOW),
        "describe" => Some(pages::DESCRIBE),
        "validate" => Some(pages::VALIDATE),
        "resources" => Some(pages::RESOURCES),
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
