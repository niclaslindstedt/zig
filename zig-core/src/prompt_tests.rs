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
    assert!(!templates::CREATE.is_empty());
    assert!(!templates::CONFIG_SIDECAR.is_empty());
    assert!(templates::CREATE.contains("{{zug_format_spec}}"));
    assert!(templates::CONFIG_SIDECAR.contains(".zug"));
}

#[test]
fn create_prompt_renders_with_sidecar() {
    let vars = HashMap::from([
        ("zug_format_spec", templates::CONFIG_SIDECAR),
        ("zag_help", "(zag help placeholder)"),
        ("zag_orch", "(zag orch placeholder)"),
    ]);
    let rendered = render(templates::CREATE, &vars);
    assert!(!rendered.contains("{{zug_format_spec}}"));
    assert!(!rendered.contains("{{zag_help}}"));
    assert!(!rendered.contains("{{zag_orch}}"));
    assert!(rendered.contains(".zug Workflow Format Specification"));
    assert!(rendered.contains("(zag help placeholder)"));
}
