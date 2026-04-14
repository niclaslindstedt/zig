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
    assert!(pattern_guidance("nonexistent").is_empty());
    assert!(pattern_guidance("").is_empty());
}

#[test]
fn pattern_guidance_is_short_description_only() {
    // After the refactor, pattern_guidance returns a one-sentence hint that
    // is appended to the initial user prompt. The canonical example files
    // now live on disk at ~/.zig/examples/ and are referenced via the
    // system prompt's {{examples_reference}} block.
    for p in ["sequential", "fan-out", "generator-critic"] {
        let g = pattern_guidance(p);
        assert!(
            !g.contains("[workflow]"),
            "pattern_guidance for '{p}' should no longer inline example TOML"
        );
        assert!(
            !g.contains("```toml"),
            "pattern_guidance for '{p}' should no longer wrap examples in code fences"
        );
    }
}

#[test]
fn build_system_prompt_replaces_all_placeholders() {
    let rendered = build_system_prompt("HELP_TEXT", "ORCH_TEXT", "EXAMPLES_REF");
    assert!(!rendered.contains("{{zwf_format_spec}}"));
    assert!(!rendered.contains("{{zag_help}}"));
    assert!(!rendered.contains("{{zag_orch}}"));
    assert!(!rendered.contains("{{examples_reference}}"));
    assert!(rendered.contains("HELP_TEXT"));
    assert!(rendered.contains("ORCH_TEXT"));
    assert!(rendered.contains("EXAMPLES_REF"));
    assert!(rendered.contains(".zwf"));
}

#[test]
fn build_system_prompt_includes_format_spec() {
    let rendered = build_system_prompt("", "", "");
    assert!(rendered.contains(".zwf Workflow Format Specification"));
}

#[test]
fn build_system_prompt_includes_run_instructions() {
    let rendered = build_system_prompt("", "", "");
    assert!(rendered.contains("zig run"));
    assert!(rendered.contains("NOT `zig workflow run`"));
}
