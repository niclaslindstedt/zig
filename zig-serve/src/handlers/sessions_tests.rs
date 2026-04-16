use super::session_ended;

const END_LINE: &str = r#"{"seq":3,"ts":"2026-01-01T00:00:00Z","zig_session_id":"abc","type":"zig_session_ended","status":"success","duration_ms":1000}"#;

#[test]
fn detects_zig_session_ended_event() {
    assert!(session_ended(&[END_LINE]));
}

#[test]
fn ignores_literal_substring_in_non_end_event() {
    // An agent could echo the literal string; a substring match would
    // incorrectly close the stream. Parsed-kind matching must not.
    let line = r#"{"seq":3,"ts":"2026-01-01T00:00:00Z","zig_session_id":"abc","type":"step_output","step_name":"s","stream":"stdout","line":"saw zig_session_ended in log"}"#;
    assert!(!session_ended(&[line]));
}

#[test]
fn skips_trailing_blank_lines() {
    assert!(session_ended(&[END_LINE, "", "   "]));
}

#[test]
fn empty_slice_is_not_ended() {
    let empty: [&str; 0] = [];
    assert!(!session_ended(&empty));
}

#[test]
fn clamp_guards_against_oversize_index() {
    // Simulates the slice operation used by the SSE handler.
    let lines: Vec<&str> = vec!["a", "b", "c"];
    let last_line: usize = 99_999;
    let start = last_line.min(lines.len());
    // Must not panic.
    let _slice = &lines[start..];
}
