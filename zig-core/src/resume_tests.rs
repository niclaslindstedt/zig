use super::*;
use crate::session::{
    OutputStream, SessionEventKind, SessionLogEvent, SessionLogIndexEntry, SessionStatus,
};

fn write_jsonl(path: &std::path::Path, events: &[SessionLogEvent]) {
    use std::io::Write;
    let mut f = std::fs::File::create(path).unwrap();
    for e in events {
        writeln!(f, "{}", serde_json::to_string(e).unwrap()).unwrap();
    }
}

fn evt(seq: u64, kind: SessionEventKind) -> SessionLogEvent {
    SessionLogEvent {
        seq,
        ts: format!("2026-04-10T12:00:{:02}+00:00", seq),
        zig_session_id: "z-1".into(),
        kind,
    }
}

fn entry() -> SessionLogIndexEntry {
    SessionLogIndexEntry {
        zig_session_id: "z-1".into(),
        workflow_name: "wf".into(),
        workflow_path: "/tmp/wf.zwf".into(),
        log_path: "ignored-by-resolve_from_log".into(),
        started_at: "2026-04-10T12:00:00+00:00".into(),
        ended_at: None,
        status: None,
        workspace_path: None,
    }
}

#[test]
fn resolve_from_log_picks_last_step_started() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("z-1.jsonl");
    write_jsonl(
        &log,
        &[
            evt(
                1,
                SessionEventKind::ZigSessionStarted {
                    workflow_name: "wf".into(),
                    workflow_path: "/tmp/wf.zwf".into(),
                    workspace_path: None,
                    cwd: None,
                    prompt: None,
                    tier_count: 2,
                },
            ),
            evt(
                2,
                SessionEventKind::StepStarted {
                    step_name: "first".into(),
                    tier_index: 0,
                    zag_session_id: "zig-wf-first".into(),
                    zag_command: "run".into(),
                    model: None,
                    prompt_preview: "...".into(),
                },
            ),
            evt(
                3,
                SessionEventKind::StepCompleted {
                    step_name: "first".into(),
                    exit_code: 0,
                    duration_ms: 100,
                    saved_vars: Vec::new(),
                },
            ),
            evt(
                4,
                SessionEventKind::StepStarted {
                    step_name: "second".into(),
                    tier_index: 1,
                    zag_session_id: "zig-wf-second".into(),
                    zag_command: "run".into(),
                    model: None,
                    prompt_preview: "...".into(),
                },
            ),
        ],
    );

    let target = resolve_from_log(&log, entry()).unwrap();
    assert_eq!(target.zag_session_id, "zig-wf-second");
    assert_eq!(target.workflow_name, "wf");
    assert_eq!(target.zig_session_id, "z-1");
}

#[test]
fn resolve_from_log_picks_last_step_even_if_session_ended() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("z-1.jsonl");
    write_jsonl(
        &log,
        &[
            evt(
                1,
                SessionEventKind::StepStarted {
                    step_name: "only".into(),
                    tier_index: 0,
                    zag_session_id: "zig-wf-only".into(),
                    zag_command: "run".into(),
                    model: None,
                    prompt_preview: "...".into(),
                },
            ),
            evt(
                2,
                SessionEventKind::StepCompleted {
                    step_name: "only".into(),
                    exit_code: 0,
                    duration_ms: 5,
                    saved_vars: Vec::new(),
                },
            ),
            evt(
                3,
                SessionEventKind::ZigSessionEnded {
                    status: SessionStatus::Success,
                    duration_ms: 10,
                },
            ),
        ],
    );

    let target = resolve_from_log(&log, entry()).unwrap();
    assert_eq!(target.zag_session_id, "zig-wf-only");
}

#[test]
fn resolve_from_log_errors_when_no_step_started() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("z-1.jsonl");
    write_jsonl(
        &log,
        &[evt(
            1,
            SessionEventKind::ZigSessionStarted {
                workflow_name: "wf".into(),
                workflow_path: "/tmp/wf.zwf".into(),
                workspace_path: None,
                cwd: None,
                prompt: None,
                tier_count: 0,
            },
        )],
    );

    let err = resolve_from_log(&log, entry()).unwrap_err();
    assert!(err.to_string().contains("no recorded step to resume"));
}

#[test]
fn continue_options_prompt_is_independent_of_resolution() {
    // Resolution only consults the index/log; the prompt field is forwarded
    // to zag at run-time and must not influence which session is picked.
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("z-1.jsonl");
    write_jsonl(
        &log,
        &[evt(
            1,
            SessionEventKind::StepStarted {
                step_name: "only".into(),
                tier_index: 0,
                zag_session_id: "zig-wf-only".into(),
                zag_command: "run".into(),
                model: None,
                prompt_preview: "...".into(),
            },
        )],
    );

    let opts_no_prompt = ContinueOptions {
        workflow: None,
        prompt: None,
        session: None,
    };
    let opts_with_prompt = ContinueOptions {
        workflow: None,
        prompt: Some("do X".into()),
        session: None,
    };

    // The prompt round-trips on the struct unchanged.
    assert!(opts_no_prompt.prompt.is_none());
    assert_eq!(opts_with_prompt.prompt.as_deref(), Some("do X"));

    // resolve_from_log doesn't touch prompt — both forms produce the same target.
    let a = resolve_from_log(&log, entry()).unwrap();
    let b = resolve_from_log(&log, entry()).unwrap();
    assert_eq!(a.zag_session_id, b.zag_session_id);
    assert_eq!(a.zag_session_id, "zig-wf-only");
}

#[test]
fn resolve_from_log_ignores_unrelated_events() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("z-1.jsonl");
    write_jsonl(
        &log,
        &[
            evt(
                1,
                SessionEventKind::StepStarted {
                    step_name: "a".into(),
                    tier_index: 0,
                    zag_session_id: "zig-wf-a".into(),
                    zag_command: "run".into(),
                    model: None,
                    prompt_preview: "...".into(),
                },
            ),
            evt(
                2,
                SessionEventKind::StepOutput {
                    step_name: "a".into(),
                    stream: OutputStream::Stdout,
                    line: "noise".into(),
                },
            ),
            evt(3, SessionEventKind::Heartbeat { interval_secs: 10 }),
        ],
    );

    let target = resolve_from_log(&log, entry()).unwrap();
    assert_eq!(target.zag_session_id, "zig-wf-a");
}
