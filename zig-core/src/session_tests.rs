use super::*;

#[test]
fn event_envelope_round_trips_through_jsonl() {
    let event = SessionLogEvent {
        seq: 7,
        ts: "2026-04-10T12:00:00+00:00".to_string(),
        zig_session_id: "abc-123".to_string(),
        kind: SessionEventKind::StepOutput {
            step_name: "analyze".to_string(),
            stream: OutputStream::Stdout,
            line: "hello world".to_string(),
        },
    };

    let line = serde_json::to_string(&event).unwrap();
    let parsed: SessionLogEvent = serde_json::from_str(&line).unwrap();

    assert_eq!(parsed.seq, 7);
    assert_eq!(parsed.zig_session_id, "abc-123");
    match parsed.kind {
        SessionEventKind::StepOutput {
            step_name,
            stream,
            line,
        } => {
            assert_eq!(step_name, "analyze");
            assert_eq!(stream, OutputStream::Stdout);
            assert_eq!(line, "hello world");
        }
        _ => panic!("expected StepOutput"),
    }
}

#[test]
fn step_started_serializes_with_snake_case_type_tag() {
    let event = SessionLogEvent {
        seq: 1,
        ts: "2026-04-10T12:00:00+00:00".into(),
        zig_session_id: "id".into(),
        kind: SessionEventKind::StepStarted {
            step_name: "s".into(),
            tier_index: 0,
            zag_session_id: "zig-wf-s".into(),
            zag_command: "run".into(),
            model: Some("sonnet".into()),
            prompt_preview: "do the thing".into(),
        },
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"type\":\"step_started\""));
    assert!(json.contains("\"zag_session_id\":\"zig-wf-s\""));
}

#[test]
fn project_index_save_and_load_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.json");

    let mut idx = SessionLogIndex::default();
    idx.sessions.push(SessionLogIndexEntry {
        zig_session_id: "id1".into(),
        workflow_name: "wf".into(),
        workflow_path: "/tmp/wf.zug".into(),
        log_path: "/tmp/id1.jsonl".into(),
        started_at: "2026-04-10T12:00:00+00:00".into(),
        ended_at: None,
        status: None,
        workspace_path: None,
    });
    save_project_index(&path, &idx).unwrap();

    let loaded = load_project_index(&path);
    assert_eq!(loaded.sessions.len(), 1);
    assert_eq!(loaded.sessions[0].zig_session_id, "id1");
}

#[test]
fn writer_create_writes_started_event_and_index_entries() {
    // Point HOME at a tmp dir so the writer touches a sandboxed ~/.zig.
    let home = tempfile::tempdir().unwrap();
    // Lock to serialize HOME mutation across tests in this file.
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: tests in this file are serialized via HOME_LOCK; no other
    // test thread reads HOME concurrently.
    unsafe {
        std::env::set_var("HOME", home.path());
    }

    let writer = SessionWriter::create("wf", "/tmp/wf.zug", Some("hi"), 2).unwrap();
    let id = writer.session_id().to_string();
    let log_path = writer.log_path().to_path_buf();

    // Emit a couple of events then drop the writer.
    writer.tier_started(0, vec!["a".into()]).unwrap();
    writer
        .step_started("a", 0, "zig-wf-a", "run", None, "preview")
        .unwrap();
    writer.ended(SessionStatus::Success, 42).unwrap();
    drop(writer);

    let contents = std::fs::read_to_string(&log_path).unwrap();
    let lines: Vec<&str> = contents.lines().collect();
    assert!(lines.len() >= 4, "expected ≥4 events, got {}", lines.len());

    // First event must be ZigSessionStarted with seq=1.
    let first: SessionLogEvent = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first.seq, 1);
    assert!(matches!(
        first.kind,
        SessionEventKind::ZigSessionStarted { .. }
    ));

    // Indexes should contain our session id.
    let project_idx = load_project_index(&paths::project_index_path(None).unwrap());
    assert!(project_idx.sessions.iter().any(|e| e.zig_session_id == id));
    let global_idx = load_global_index(&paths::global_sessions_index_path().unwrap());
    assert!(global_idx.sessions.iter().any(|e| e.zig_session_id == id));
}

// Tests in this file mutate the global HOME env var; serialize them.
static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
