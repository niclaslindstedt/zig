/// Embedded documentation pages, compiled from `docs/` markdown files.
///
/// Docs cover concepts (the `.zwf`/`.zwfz` format, patterns, variables, …) as
/// opposed to command references, which live under `zig man`.
mod pages {
    pub const ZWF: &str = include_str!("../docs/zwf.md");
    pub const PATTERNS: &str = include_str!("../docs/patterns.md");
    pub const VARIABLES: &str = include_str!("../docs/variables.md");
    pub const CONDITIONS: &str = include_str!("../docs/conditions.md");
    pub const MEMORY: &str = include_str!("../docs/memory.md");
    pub const STORAGE: &str = include_str!("../docs/storage.md");
}

/// All available docs topics in display order.
pub const TOPICS: &[(&str, &str)] = &[
    ("zwf", "The .zwf/.zwfz workflow format"),
    ("patterns", "Orchestration patterns"),
    ("variables", "Variable system and data flow"),
    ("conditions", "Condition expressions"),
    ("memory", "Memory scratch pad and the `<memory>` block"),
    (
        "storage",
        "Workflow storage — structured writable working data",
    ),
];

/// Look up a docs page by topic name.
///
/// Returns the markdown content if the topic exists, or `None`.
pub fn get(topic: &str) -> Option<&'static str> {
    match topic {
        "zwf" => Some(pages::ZWF),
        "patterns" => Some(pages::PATTERNS),
        "variables" => Some(pages::VARIABLES),
        "conditions" => Some(pages::CONDITIONS),
        "memory" => Some(pages::MEMORY),
        "storage" => Some(pages::STORAGE),
        _ => None,
    }
}

/// List all available docs topics with their descriptions.
pub fn list_topics() -> String {
    let mut out = String::from("Available docs:\n\n");
    let max_name_len = TOPICS.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    for (name, description) in TOPICS {
        out.push_str(&format!(
            "  {name:<width$}  {description}\n",
            width = max_name_len
        ));
    }
    out.push_str("\nUsage: zig docs <topic>");
    out
}

#[cfg(test)]
#[path = "docs_tests.rs"]
mod tests;
