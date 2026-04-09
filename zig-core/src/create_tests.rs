use super::*;

#[test]
fn pattern_guidance_returns_nonempty_for_all_known_patterns() {
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
        let g = pattern_guidance(p);
        assert!(
            !g.is_empty(),
            "pattern_guidance for '{p}' should be non-empty"
        );
    }
}

#[test]
fn pattern_guidance_unknown_returns_empty() {
    assert_eq!(pattern_guidance("nonexistent"), "");
    assert_eq!(pattern_guidance(""), "");
}

#[test]
fn build_system_prompt_replaces_all_placeholders() {
    let rendered = build_system_prompt("HELP_TEXT", "ORCH_TEXT");
    assert!(!rendered.contains("{{zug_format_spec}}"));
    assert!(!rendered.contains("{{zag_help}}"));
    assert!(!rendered.contains("{{zag_orch}}"));
    assert!(rendered.contains("HELP_TEXT"));
    assert!(rendered.contains("ORCH_TEXT"));
    assert!(rendered.contains(".zug"));
}

#[test]
fn build_system_prompt_includes_format_spec() {
    let rendered = build_system_prompt("", "");
    assert!(rendered.contains(".zug Workflow Format Specification"));
}

#[test]
fn build_system_prompt_includes_run_instructions() {
    let rendered = build_system_prompt("", "");
    assert!(rendered.contains("zig run"));
    assert!(rendered.contains("NOT `zig workflow run`"));
}
