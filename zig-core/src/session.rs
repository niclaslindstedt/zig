//! Zig session log: a recorded execution of `zig run`.
//!
//! A *zig session* is the parent layer above the zag session log: one zig
//! session orchestrates many child zag sessions (one per workflow step).
//! This module owns the writer, event schema, and on-disk index.
//!
//! Mirrors zag's session logging architecture so the two stay structurally
//! aligned and conceptually transferable. Key analogs:
//!
//! - `SessionWriter`        ↔ `zag-agent/src/session_log.rs:344` `SessionLogWriter`
//! - `SessionCoordinator`   ↔ `zag-agent/src/session_log.rs:565` `SessionLogCoordinator`
//! - `SessionLogEvent`      ↔ `zag-agent/src/session_log.rs:182` `AgentLogEvent`
//! - `SessionLogIndex`      ↔ `zag-agent/src/session_log.rs:197` `SessionLogIndex`
//! - `GlobalSessionIndex`   ↔ `zag-agent/src/session_log.rs:225` `GlobalSessionIndex`
//!
//! On-disk layout (mirrors `~/.zag/...` byte-for-byte):
//!
//! ```text
//! ~/.zig/
//!   projects/<sanitized-project-path>/logs/
//!     index.json
//!     sessions/<zig_session_id>.jsonl
//!   sessions_index.json
//! ```

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ZigError;
use crate::paths;

/// Heartbeat interval — mirrors zag's 10s default
/// (`zag-agent/src/session_log.rs:872`).
const HEARTBEAT_INTERVAL_SECS: u64 = 10;

/// Stream identifier for `step_output` events.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// Final status of a zig session.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Success,
    Failure,
}

/// Event payload variants. The envelope (`SessionLogEvent`) carries `seq`,
/// `ts`, and `zig_session_id`; this enum carries the type-specific fields.
///
/// Mirrors zag's `LogEventKind` (`zag-agent/src/session_log.rs:99`) in
/// shape: `#[serde(tag = "type", rename_all = "snake_case")]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEventKind {
    ZigSessionStarted {
        workflow_name: String,
        workflow_path: String,
        workspace_path: Option<String>,
        cwd: Option<String>,
        prompt: Option<String>,
        tier_count: usize,
    },
    TierStarted {
        tier_index: usize,
        step_names: Vec<String>,
    },
    StepStarted {
        step_name: String,
        tier_index: usize,
        zag_session_id: String,
        zag_command: String,
        model: Option<String>,
        prompt_preview: String,
    },
    StepOutput {
        step_name: String,
        stream: OutputStream,
        line: String,
    },
    StepCompleted {
        step_name: String,
        exit_code: i32,
        duration_ms: u64,
        saved_vars: Vec<String>,
    },
    StepFailed {
        step_name: String,
        exit_code: Option<i32>,
        attempt: u32,
        error: String,
    },
    StepSkipped {
        step_name: String,
        reason: String,
    },
    /// Periodic liveness indicator. Mirrors zag's `Heartbeat`
    /// (`zag-agent/src/session_log.rs:161`).
    Heartbeat {
        interval_secs: u64,
    },
    ZigSessionEnded {
        status: SessionStatus,
        duration_ms: u64,
    },
}

/// Event envelope written to the JSONL log. Field naming mirrors zag's
/// `AgentLogEvent` (`zag-agent/src/session_log.rs:182`): `seq`, `ts`, plus
/// a session id and a flattened kind discriminator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLogEvent {
    pub seq: u64,
    pub ts: String,
    pub zig_session_id: String,
    #[serde(flatten)]
    pub kind: SessionEventKind,
}

/// Per-project session index entry (`<project>/logs/index.json`).
///
/// Mirrors zag's `SessionLogIndexEntry` (`zag-agent/src/session_log.rs:201`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLogIndexEntry {
    pub zig_session_id: String,
    pub workflow_name: String,
    pub workflow_path: String,
    pub log_path: String,
    pub started_at: String,
    #[serde(default)]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub status: Option<SessionStatus>,
    #[serde(default)]
    pub workspace_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionLogIndex {
    pub sessions: Vec<SessionLogIndexEntry>,
}

/// Global cross-project index entry (`~/.zig/sessions_index.json`).
///
/// Mirrors zag's `GlobalSessionEntry` (`zag-agent/src/session_log.rs:229`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSessionEntry {
    pub zig_session_id: String,
    pub workflow_name: String,
    pub project: String,
    pub log_path: String,
    pub started_at: String,
    #[serde(default)]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub status: Option<SessionStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalSessionIndex {
    pub sessions: Vec<GlobalSessionEntry>,
}

// ---------------------------------------------------------------------
// Index I/O
// ---------------------------------------------------------------------

pub fn load_project_index(path: &Path) -> SessionLogIndex {
    if !path.exists() {
        return SessionLogIndex::default();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_project_index(path: &Path, index: &SessionLogIndex) -> Result<(), ZigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ZigError::Io(format!("failed to create {}: {e}", parent.display())))?;
    }
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| ZigError::Io(format!("failed to serialize project index: {e}")))?;
    std::fs::write(path, json)
        .map_err(|e| ZigError::Io(format!("failed to write {}: {e}", path.display())))?;
    Ok(())
}

pub fn load_global_index(path: &Path) -> GlobalSessionIndex {
    if !path.exists() {
        return GlobalSessionIndex::default();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_global_index(path: &Path, index: &GlobalSessionIndex) -> Result<(), ZigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ZigError::Io(format!("failed to create {}: {e}", parent.display())))?;
    }
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| ZigError::Io(format!("failed to serialize global index: {e}")))?;
    std::fs::write(path, json)
        .map_err(|e| ZigError::Io(format!("failed to write {}: {e}", path.display())))?;
    Ok(())
}

// ---------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------

/// Append-only writer for a single zig session log file.
///
/// Mirrors zag's `SessionLogWriter` (`zag-agent/src/session_log.rs:344`).
/// Every emit increments `seq`, stamps `ts`, serializes one JSON line, and
/// flushes — so a tailer reading the file sees the event within one poll
/// cycle.
pub struct SessionWriter {
    zig_session_id: String,
    log_path: PathBuf,
    project_index_path: Option<PathBuf>,
    global_index_path: Option<PathBuf>,
    inner: Mutex<WriterInner>,
}

struct WriterInner {
    file: File,
    seq: u64,
}

impl SessionWriter {
    /// Create a new session: generate a UUID, ensure the project sessions
    /// dir, open the log file for append, emit `ZigSessionStarted`, and
    /// upsert both indexes.
    pub fn create(
        workflow_name: &str,
        workflow_path: &str,
        prompt: Option<&str>,
        tier_count: usize,
    ) -> Result<Self, ZigError> {
        let zig_session_id = Uuid::new_v4().to_string();

        let sessions_dir = paths::ensure_project_sessions_dir(None)?;
        let log_path = sessions_dir.join(format!("{zig_session_id}.jsonl"));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| ZigError::Io(format!("failed to open {}: {e}", log_path.display())))?;

        let writer = Self {
            zig_session_id: zig_session_id.clone(),
            log_path: log_path.clone(),
            project_index_path: paths::project_index_path(None),
            global_index_path: paths::global_sessions_index_path(),
            inner: Mutex::new(WriterInner { file, seq: 0 }),
        };

        let cwd = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        let workspace_path = paths::project_dir(None).map(|p| p.to_string_lossy().into_owned());
        let started_at = now_rfc3339();

        writer.emit(SessionEventKind::ZigSessionStarted {
            workflow_name: workflow_name.to_string(),
            workflow_path: workflow_path.to_string(),
            workspace_path: workspace_path.clone(),
            cwd,
            prompt: prompt.map(str::to_string),
            tier_count,
        })?;

        writer.upsert_indexes(
            workflow_name,
            workflow_path,
            &workspace_path,
            &started_at,
            None,
        )?;

        Ok(writer)
    }

    /// The session id (UUID) for this writer.
    pub fn session_id(&self) -> &str {
        &self.zig_session_id
    }

    /// The on-disk log path.
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub fn tier_started(&self, tier_index: usize, step_names: Vec<String>) -> Result<(), ZigError> {
        self.emit(SessionEventKind::TierStarted {
            tier_index,
            step_names,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn step_started(
        &self,
        step_name: &str,
        tier_index: usize,
        zag_session_id: &str,
        zag_command: &str,
        model: Option<&str>,
        prompt_preview: &str,
    ) -> Result<(), ZigError> {
        self.emit(SessionEventKind::StepStarted {
            step_name: step_name.to_string(),
            tier_index,
            zag_session_id: zag_session_id.to_string(),
            zag_command: zag_command.to_string(),
            model: model.map(str::to_string),
            prompt_preview: prompt_preview.to_string(),
        })
    }

    pub fn step_output(
        &self,
        step_name: &str,
        stream: OutputStream,
        line: &str,
    ) -> Result<(), ZigError> {
        self.emit(SessionEventKind::StepOutput {
            step_name: step_name.to_string(),
            stream,
            line: line.to_string(),
        })
    }

    pub fn step_completed(
        &self,
        step_name: &str,
        exit_code: i32,
        duration_ms: u64,
        saved_vars: Vec<String>,
    ) -> Result<(), ZigError> {
        self.emit(SessionEventKind::StepCompleted {
            step_name: step_name.to_string(),
            exit_code,
            duration_ms,
            saved_vars,
        })
    }

    pub fn step_failed(
        &self,
        step_name: &str,
        exit_code: Option<i32>,
        attempt: u32,
        error: &str,
    ) -> Result<(), ZigError> {
        self.emit(SessionEventKind::StepFailed {
            step_name: step_name.to_string(),
            exit_code,
            attempt,
            error: error.to_string(),
        })
    }

    pub fn step_skipped(&self, step_name: &str, reason: &str) -> Result<(), ZigError> {
        self.emit(SessionEventKind::StepSkipped {
            step_name: step_name.to_string(),
            reason: reason.to_string(),
        })
    }

    pub fn heartbeat(&self) -> Result<(), ZigError> {
        self.emit(SessionEventKind::Heartbeat {
            interval_secs: HEARTBEAT_INTERVAL_SECS,
        })
    }

    /// Emit `ZigSessionEnded` and stamp the indexes with `ended_at`/`status`.
    pub fn ended(&self, status: SessionStatus, duration_ms: u64) -> Result<(), ZigError> {
        self.emit(SessionEventKind::ZigSessionEnded {
            status,
            duration_ms,
        })?;
        self.stamp_ended(status)?;
        Ok(())
    }

    fn emit(&self, kind: SessionEventKind) -> Result<(), ZigError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ZigError::Io("session writer mutex poisoned".into()))?;
        inner.seq += 1;
        let event = SessionLogEvent {
            seq: inner.seq,
            ts: now_rfc3339(),
            zig_session_id: self.zig_session_id.clone(),
            kind,
        };
        let line = serde_json::to_string(&event)
            .map_err(|e| ZigError::Io(format!("failed to serialize session event: {e}")))?;
        writeln!(inner.file, "{line}")
            .map_err(|e| ZigError::Io(format!("failed to write session event: {e}")))?;
        inner
            .file
            .flush()
            .map_err(|e| ZigError::Io(format!("failed to flush session log: {e}")))?;
        Ok(())
    }

    fn upsert_indexes(
        &self,
        workflow_name: &str,
        workflow_path: &str,
        workspace_path: &Option<String>,
        started_at: &str,
        ended_at: Option<String>,
    ) -> Result<(), ZigError> {
        let log_path_str = self.log_path.to_string_lossy().into_owned();

        if let Some(idx_path) = &self.project_index_path {
            let mut index = load_project_index(idx_path);
            index
                .sessions
                .retain(|e| e.zig_session_id != self.zig_session_id);
            index.sessions.push(SessionLogIndexEntry {
                zig_session_id: self.zig_session_id.clone(),
                workflow_name: workflow_name.to_string(),
                workflow_path: workflow_path.to_string(),
                log_path: log_path_str.clone(),
                started_at: started_at.to_string(),
                ended_at,
                status: None,
                workspace_path: workspace_path.clone(),
            });
            save_project_index(idx_path, &index)?;
        }

        if let Some(idx_path) = &self.global_index_path {
            let mut index = load_global_index(idx_path);
            index
                .sessions
                .retain(|e| e.zig_session_id != self.zig_session_id);
            index.sessions.push(GlobalSessionEntry {
                zig_session_id: self.zig_session_id.clone(),
                workflow_name: workflow_name.to_string(),
                project: workspace_path.clone().unwrap_or_default(),
                log_path: log_path_str,
                started_at: started_at.to_string(),
                ended_at: None,
                status: None,
            });
            save_global_index(idx_path, &index)?;
        }

        Ok(())
    }

    fn stamp_ended(&self, status: SessionStatus) -> Result<(), ZigError> {
        let ended_at = now_rfc3339();

        if let Some(idx_path) = &self.project_index_path {
            let mut index = load_project_index(idx_path);
            for entry in &mut index.sessions {
                if entry.zig_session_id == self.zig_session_id {
                    entry.ended_at = Some(ended_at.clone());
                    entry.status = Some(status);
                }
            }
            save_project_index(idx_path, &index)?;
        }

        if let Some(idx_path) = &self.global_index_path {
            let mut index = load_global_index(idx_path);
            for entry in &mut index.sessions {
                if entry.zig_session_id == self.zig_session_id {
                    entry.ended_at = Some(ended_at.clone());
                    entry.status = Some(status);
                }
            }
            save_global_index(idx_path, &index)?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------
// Coordinator
// ---------------------------------------------------------------------

/// Wraps a `SessionWriter` in an `Arc` and runs a background thread that
/// emits a `Heartbeat` event every 10 seconds. The handle's `Drop` impl
/// stops the heartbeat thread and stamps `ended_at` defensively if
/// `finish()` was never called (crash/panic safety).
///
/// Mirrors zag's `SessionLogCoordinator`
/// (`zag-agent/src/session_log.rs:565`).
pub struct SessionCoordinator {
    writer: Arc<SessionWriter>,
    started: Instant,
    stop_flag: Arc<AtomicBool>,
    heartbeat: Option<JoinHandle<()>>,
    finished: bool,
}

impl SessionCoordinator {
    pub fn start(writer: SessionWriter) -> Self {
        let writer = Arc::new(writer);
        let stop_flag = Arc::new(AtomicBool::new(false));

        let hb_writer = Arc::clone(&writer);
        let hb_stop = Arc::clone(&stop_flag);
        let heartbeat = thread::spawn(move || {
            let interval = Duration::from_secs(HEARTBEAT_INTERVAL_SECS);
            // Sleep in short ticks so shutdown is responsive.
            let tick = Duration::from_millis(200);
            let mut elapsed = Duration::ZERO;
            while !hb_stop.load(Ordering::Relaxed) {
                thread::sleep(tick);
                elapsed += tick;
                if elapsed >= interval {
                    elapsed = Duration::ZERO;
                    let _ = hb_writer.heartbeat();
                }
            }
        });

        Self {
            writer,
            started: Instant::now(),
            stop_flag,
            heartbeat: Some(heartbeat),
            finished: false,
        }
    }

    pub fn writer(&self) -> Arc<SessionWriter> {
        Arc::clone(&self.writer)
    }

    /// Mark the session ended cleanly. Stops the heartbeat thread and
    /// emits `ZigSessionEnded`.
    pub fn finish(mut self, status: SessionStatus) -> Result<(), ZigError> {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(h) = self.heartbeat.take() {
            let _ = h.join();
        }
        let duration_ms = self.started.elapsed().as_millis() as u64;
        self.writer.ended(status, duration_ms)?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for SessionCoordinator {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        // Crash/panic path: stop heartbeat and best-effort stamp the
        // indexes so `--latest`/`--active` resolution stays consistent.
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(h) = self.heartbeat.take() {
            let _ = h.join();
        }
        let duration_ms = self.started.elapsed().as_millis() as u64;
        let _ = self.writer.ended(SessionStatus::Failure, duration_ms);
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
