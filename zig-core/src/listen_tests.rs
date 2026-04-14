use super::*;
use crate::session::{SessionEventKind, SessionLogEvent, SessionStatus};

fn evt(seq: u64, kind: SessionEventKind) -> SessionLogEvent {
    SessionLogEvent {
        seq,
        ts: "2026-04-10T12:00:00+00:00".into(),
        zig_session_id: "id".into(),
        kind,
    }
}

#[test]
fn format_event_text_renders_session_started() {
    let e = evt(
        1,
        SessionEventKind::ZigSessionStarted {
            workflow_name: "code-review".into(),
            workflow_path: "/tmp/x.zwf".into(),
            workspace_path: None,
            cwd: None,
            prompt: None,
            tier_count: 3,
        },
    );
    let text = format_event_text(&e).unwrap();
    assert!(text.contains("code-review"));
    assert!(text.contains("3 tier"));
}

#[test]
fn format_event_text_renders_step_output() {
    let e = evt(
        2,
        SessionEventKind::StepOutput {
            step_name: "analyze".into(),
            stream: crate::session::OutputStream::Stdout,
            line: "found 4 issues".into(),
        },
    );
    let text = format_event_text(&e).unwrap();
    assert_eq!(text, "[analyze] found 4 issues");
}

#[test]
fn format_event_text_suppresses_heartbeat() {
    let e = evt(3, SessionEventKind::Heartbeat { interval_secs: 10 });
    assert!(format_event_text(&e).is_none());
}

#[test]
fn format_event_text_renders_session_ended() {
    let e = evt(
        99,
        SessionEventKind::ZigSessionEnded {
            status: SessionStatus::Success,
            duration_ms: 1234,
        },
    );
    let text = format_event_text(&e).unwrap();
    assert!(text.contains("1234ms"));
    assert!(text.contains("Success"));
}

#[test]
fn tail_session_log_replays_a_completed_session() {
    use std::io::Write;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let mut f = std::fs::File::create(&path).unwrap();
    let events = vec![
        evt(
            1,
            SessionEventKind::ZigSessionStarted {
                workflow_name: "wf".into(),
                workflow_path: "/x".into(),
                workspace_path: None,
                cwd: None,
                prompt: None,
                tier_count: 1,
            },
        ),
        evt(
            2,
            SessionEventKind::StepOutput {
                step_name: "s".into(),
                stream: crate::session::OutputStream::Stdout,
                line: "hello".into(),
            },
        ),
        evt(
            3,
            SessionEventKind::ZigSessionEnded {
                status: SessionStatus::Success,
                duration_ms: 5,
            },
        ),
    ];
    for e in &events {
        writeln!(f, "{}", serde_json::to_string(e).unwrap()).unwrap();
    }
    drop(f);

    // tail_session_log returns once it sees ZigSessionEnded, so this is bounded.
    tail_session_log(&path, &ListenOptions::default()).unwrap();
}
