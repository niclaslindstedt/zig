use std::collections::HashMap;

use super::*;

#[test]
fn render_replaces_variables() {
    let template = "Hello {{name}}, welcome to {{project}}!";
    let vars = HashMap::from([("name", "Alice"), ("project", "zig")]);
    assert_eq!(render(template, &vars), "Hello Alice, welcome to zig!");
}

#[test]
fn render_leaves_unknown_variables() {
    let template = "Hello {{name}}, your id is {{id}}";
    let vars = HashMap::from([("name", "Bob")]);
    assert_eq!(render(template, &vars), "Hello Bob, your id is {{id}}");
}

#[test]
fn render_empty_vars() {
    let template = "No variables here";
    let vars = HashMap::new();
    assert_eq!(render(template, &vars), "No variables here");
}

#[test]
fn render_multiple_occurrences() {
    let template = "{{x}} + {{x}} = 2 * {{x}}";
    let vars = HashMap::from([("x", "5")]);
    assert_eq!(render(template, &vars), "5 + 5 = 2 * 5");
}

#[test]
fn render_multiline_template() {
    let template = "## Reference\n\n{{content}}\n\n## End";
    let vars = HashMap::from([("content", "line1\nline2\nline3")]);
    assert_eq!(
        render(template, &vars),
        "## Reference\n\nline1\nline2\nline3\n\n## End"
    );
}

#[test]
fn templates_are_embedded() {
    assert!(!templates::create().is_empty());
    assert!(!templates::config_sidecar().is_empty());
    assert!(templates::create().contains("{{zug_format_spec}}"));
    assert!(templates::config_sidecar().contains(".zug"));
}

#[test]
fn templates_have_front_matter_stripped() {
    // Front matter delimiters and metadata fields must not leak into agent
    // input. The substring checks below are line-ending agnostic so the test
    // catches regressions on both Unix (LF) and Windows (CRLF) checkouts.
    let create = templates::create();
    assert!(
        !create.starts_with("---"),
        "create prompt still begins with front matter delimiter"
    );
    assert!(
        !create.contains("name: create"),
        "create prompt still contains front matter `name` field"
    );
    assert!(
        !create.contains("references:"),
        "create prompt still contains front matter `references` field"
    );

    let sidecar = templates::config_sidecar();
    assert!(
        !sidecar.starts_with("---"),
        "config sidecar still begins with front matter delimiter"
    );
    assert!(
        !sidecar.contains("name: config-sidecar"),
        "config sidecar still contains front matter `name` field"
    );
}

#[test]
fn strip_front_matter_removes_block() {
    let input = "---\nname: foo\nversion: \"1.0\"\n---\n\nbody line one\nbody line two\n";
    assert_eq!(strip_front_matter(input), "body line one\nbody line two\n");
}

#[test]
fn strip_front_matter_without_trailing_blank_line() {
    let input = "---\nname: foo\n---\nbody\n";
    assert_eq!(strip_front_matter(input), "body\n");
}

#[test]
fn strip_front_matter_no_front_matter_passthrough() {
    let input = "no front matter here\n---\nnot a delimiter\n";
    assert_eq!(strip_front_matter(input), input);
}

#[test]
fn strip_front_matter_does_not_match_mid_file_delimiters() {
    // A `---` that is not on the first line is just markdown content.
    let input = "# heading\n\n---\n\nbody\n";
    assert_eq!(strip_front_matter(input), input);
}

#[test]
fn strip_front_matter_handles_unterminated_block() {
    // Missing closing delimiter — treat as no front matter rather than swallowing
    // the whole file.
    let input = "---\nname: foo\nversion: 1.0\nbody without close\n";
    assert_eq!(strip_front_matter(input), input);
}

#[test]
fn strip_front_matter_handles_crlf_line_endings() {
    // Windows checkouts embed CRLF line endings via `include_str!`.
    let input =
        "---\r\nname: foo\r\nversion: \"1.0\"\r\n---\r\n\r\nbody line one\r\nbody line two\r\n";
    assert_eq!(
        strip_front_matter(input),
        "body line one\r\nbody line two\r\n"
    );
}

#[test]
fn strip_front_matter_crlf_without_trailing_blank_line() {
    let input = "---\r\nname: foo\r\n---\r\nbody\r\n";
    assert_eq!(strip_front_matter(input), "body\r\n");
}

#[test]
fn example_templates_are_embedded() {
    let examples = all_examples();
    assert_eq!(examples.len(), 7);
    for (filename, content) in &examples {
        assert!(
            !content.is_empty(),
            "example {filename} should not be empty"
        );
        assert!(
            content.contains("[workflow]"),
            "example {filename} should contain [workflow] section"
        );
    }
}

#[test]
fn example_for_pattern_returns_some_for_all_known() {
    let patterns = [
        "sequential",
        "fan-out",
        "generator-critic",
        "coordinator-dispatcher",
        "hierarchical-decomposition",
        "human-in-the-loop",
        "inter-agent-communication",
    ];
    for p in &patterns {
        assert!(
            example_for_pattern(p).is_some(),
            "example_for_pattern('{p}') should return Some"
        );
    }
}

#[test]
fn example_for_pattern_returns_none_for_unknown() {
    assert!(example_for_pattern("nonexistent").is_none());
    assert!(example_for_pattern("").is_none());
}

#[test]
fn create_prompt_renders_with_sidecar() {
    let vars = HashMap::from([
        ("zug_format_spec", templates::config_sidecar()),
        ("zag_help", "(zag help placeholder)"),
        ("zag_orch", "(zag orch placeholder)"),
    ]);
    let rendered = render(templates::create(), &vars);
    assert!(!rendered.contains("{{zug_format_spec}}"));
    assert!(!rendered.contains("{{zag_help}}"));
    assert!(!rendered.contains("{{zag_orch}}"));
    assert!(rendered.contains(".zug Workflow Format Specification"));
    assert!(rendered.contains("(zag help placeholder)"));
}
