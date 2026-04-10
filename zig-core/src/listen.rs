//! `zig listen` — tail a zig session log file in real time.
//!
//! Mirrors zag's listen architecture (`zag-orch/src/listen.rs`) so the two
//! commands stay structurally aligned. Same JSONL + 100ms-poll tailing,
//! same resolver shapes (`Id`/`Latest`/`Active`), same `ListenFormat` /
//! `ListenOptions` extension points so future zag features (rich text,
//! JSON output, filters, `--ps`, websocket streaming) can be mirrored
//! into zig as small additions instead of refactors.

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::error::ZigError;
use crate::paths;
use crate::session::{
    GlobalSessionIndex, SessionEventKind, SessionLogEvent, SessionLogIndex, load_global_index,
    load_project_index,
};

/// How to identify the session to tail.
///
/// Mirrors the selector dispatch in `zag-orch/src/listen.rs:84`
/// `resolve_session_log`. A future `Ps(String)` variant (zig has no
/// process store yet) drops in here without changing call sites.
#[derive(Debug, Clone)]
pub enum SessionSelector {
    Id(String),
    Latest,
    Active,
}

/// Output format for `zig listen`.
///
/// Only `Text` is implemented for the first cut, but the enum exists so
/// adding `Json` / `RichText` later mirrors `zag-orch/src/listen.rs:14`
/// without churning callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListenFormat {
    #[default]
    Text,
}

/// Options bundle passed into [`tail_session_log`]. Existing as a struct
/// (rather than positional args) means future flags like `--filter`,
/// `--show-thinking`, `--timestamps` are additive — same shape as the
/// `tail_session_log` signature in `zag-orch/src/listen.rs:357`.
#[derive(Debug, Clone, Default)]
pub struct ListenOptions {
    pub format: ListenFormat,
}

/// Top-level entry point invoked from the CLI.
pub fn listen(selector: SessionSelector, options: ListenOptions) -> Result<(), ZigError> {
    let path = resolve_session_log(&selector)?;
    eprintln!("listening to session log: {}", path.display());
    tail_session_log(&path, &options)
}

// ---------------------------------------------------------------------
// Resolver
// ---------------------------------------------------------------------

/// Resolve a session log path from a selector.
///
/// Mirrors `zag-orch/src/listen.rs:84` `resolve_session_log`:
///   1. `Id`     → direct file → project index → global index → prefix match
///   2. `Latest` → newest `started_at` across project then global index
///   3. `Active` → most recently modified `.jsonl` whose entry has no `ended_at`
pub fn resolve_session_log(selector: &SessionSelector) -> Result<PathBuf, ZigError> {
    match selector {
        SessionSelector::Id(id) => resolve_by_id(id),
        SessionSelector::Latest => resolve_latest_session(),
        SessionSelector::Active => resolve_active_session(),
    }
}

fn resolve_by_id(id: &str) -> Result<PathBuf, ZigError> {
    // 1. Direct lookup against the current project's sessions dir.
    if let Some(sessions_dir) = paths::project_sessions_dir(None) {
        let direct = sessions_dir.join(format!("{id}.jsonl"));
        if direct.exists() {
            return Ok(direct);
        }
    }

    // 2. Project index exact + prefix match.
    if let Some(idx_path) = paths::project_index_path(None)
        && idx_path.exists()
    {
        let index = load_project_index(&idx_path);
        if let Some(entry) = index.sessions.iter().find(|e| e.zig_session_id == id) {
            let path = PathBuf::from(&entry.log_path);
            if path.exists() {
                return Ok(path);
            }
        }
        let matches: Vec<_> = index
            .sessions
            .iter()
            .filter(|e| e.zig_session_id.starts_with(id))
            .collect();
        if matches.len() == 1 {
            let path = PathBuf::from(&matches[0].log_path);
            if path.exists() {
                return Ok(path);
            }
        } else if matches.len() > 1 {
            return Err(ZigError::Io(format!(
                "ambiguous session id prefix '{id}'. Matches: {}",
                matches
                    .iter()
                    .map(|e| e.zig_session_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    }

    // 3. Global cross-project index exact + prefix match.
    if let Some(idx_path) = paths::global_sessions_index_path()
        && idx_path.exists()
    {
        let index = load_global_index(&idx_path);
        if let Some(entry) = index.sessions.iter().find(|e| e.zig_session_id == id) {
            let path = PathBuf::from(&entry.log_path);
            if path.exists() {
                return Ok(path);
            }
        }
        let matches: Vec<_> = index
            .sessions
            .iter()
            .filter(|e| e.zig_session_id.starts_with(id))
            .collect();
        if matches.len() == 1 {
            let path = PathBuf::from(&matches[0].log_path);
            if path.exists() {
                return Ok(path);
            }
        } else if matches.len() > 1 {
            return Err(ZigError::Io(format!(
                "ambiguous session id prefix '{id}' across projects. Matches: {}",
                matches
                    .iter()
                    .map(|e| e.zig_session_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    }

    Err(ZigError::Io(format!("no session log found for '{id}'")))
}

/// Most recent `started_at` across project then global index.
///
/// Mirrors `resolve_latest_session` (`zag-orch/src/listen.rs:156`).
pub fn resolve_latest_session() -> Result<PathBuf, ZigError> {
    if let Some(idx_path) = paths::project_index_path(None)
        && idx_path.exists()
    {
        let index = load_project_index(&idx_path);
        if let Some(newest) = index
            .sessions
            .iter()
            .max_by(|a, b| a.started_at.cmp(&b.started_at))
        {
            let path = PathBuf::from(&newest.log_path);
            if path.exists() {
                return Ok(path);
            }
        }
    }

    if let Some(idx_path) = paths::global_sessions_index_path()
        && idx_path.exists()
    {
        let index: GlobalSessionIndex = load_global_index(&idx_path);
        if let Some(newest) = index
            .sessions
            .iter()
            .max_by(|a, b| a.started_at.cmp(&b.started_at))
        {
            let path = PathBuf::from(&newest.log_path);
            if path.exists() {
                return Ok(path);
            }
        }
    }

    Err(ZigError::Io(
        "no zig session index found. Run `zig run` first.".into(),
    ))
}

/// Most recently modified `.jsonl` whose index entry has no `ended_at`.
///
/// Mirrors `resolve_active_session` (`zag-orch/src/listen.rs:224`).
pub fn resolve_active_session() -> Result<PathBuf, ZigError> {
    let sessions_dir = paths::project_sessions_dir(None)
        .ok_or_else(|| ZigError::Io("HOME environment variable not set".into()))?;

    if !sessions_dir.exists() {
        return Err(ZigError::Io(
            "no sessions directory found. Run `zig run` first.".into(),
        ));
    }

    // Build a set of session ids that are still active per the project index.
    let active_ids: Option<SessionLogIndex> =
        paths::project_index_path(None).map(|p| load_project_index(&p));
    let is_active = |id: &str| -> bool {
        active_ids
            .as_ref()
            .map(|idx| {
                idx.sessions
                    .iter()
                    .any(|e| e.zig_session_id == id && e.ended_at.is_none())
            })
            .unwrap_or(true) // no index → assume any file is fair game
    };

    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let entries = std::fs::read_dir(&sessions_dir)
        .map_err(|e| ZigError::Io(format!("failed to read {}: {e}", sessions_dir.display())))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "jsonl") {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if !is_active(id) {
                continue;
            }
            if let Ok(meta) = entry.metadata()
                && let Ok(modified) = meta.modified()
                && newest
                    .as_ref()
                    .map(|(cur, _)| modified > *cur)
                    .unwrap_or(true)
            {
                newest = Some((modified, path));
            }
        }
    }

    newest
        .map(|(_, p)| p)
        .ok_or_else(|| ZigError::Io("no active zig session found".into()))
}

// ---------------------------------------------------------------------
// Tail loop
// ---------------------------------------------------------------------

/// Tail a session log file, printing events as they arrive. Returns when
/// a `ZigSessionEnded` event is observed.
///
/// Mirrors `tail_session_log` (`zag-orch/src/listen.rs:357`): blocking
/// `read_line()` loop, sleep 100ms on EOF, re-seek to current position to
/// pick up appended bytes, exit on the session-ended event.
pub fn tail_session_log(path: &Path, options: &ListenOptions) -> Result<(), ZigError> {
    let file = File::open(path)
        .map_err(|e| ZigError::Io(format!("failed to open {}: {e}", path.display())))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let stdout = std::io::stdout();

    loop {
        line.clear();
        let bytes_read = reader
            .read_line(&mut line)
            .map_err(|e| ZigError::Io(format!("failed to read session log: {e}")))?;

        if bytes_read > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<SessionLogEvent>(trimmed) {
                Ok(event) => {
                    let is_ended = matches!(event.kind, SessionEventKind::ZigSessionEnded { .. });
                    if let Some(text) = format_event(&event, options.format) {
                        let mut h = stdout.lock();
                        let _ = writeln!(h, "{text}");
                    }
                    if is_ended {
                        return Ok(());
                    }
                }
                Err(e) => {
                    eprintln!("[parse error] {e}: {}", truncate(trimmed, 80));
                }
            }
        } else {
            thread::sleep(Duration::from_millis(100));
            let pos = reader
                .stream_position()
                .map_err(|e| ZigError::Io(format!("failed to stream_position: {e}")))?;
            reader
                .seek(SeekFrom::Start(pos))
                .map_err(|e| ZigError::Io(format!("failed to seek: {e}")))?;
        }
    }
}

// ---------------------------------------------------------------------
// Async stub
// ---------------------------------------------------------------------

/// Stub mirroring `stream_session_events` (`zag-orch/src/listen.rs:299`).
///
/// Not wired up yet — there's no `zig-serve` analog. Kept here so that
/// when one arrives, the call site lands at the same module path the
/// zag equivalent uses, with no listen.rs restructuring required.
#[allow(dead_code)]
pub fn stream_session_events_stub(_path: &Path) -> Result<(), ZigError> {
    Err(ZigError::Io(
        "stream_session_events not yet implemented for zig".into(),
    ))
}

// ---------------------------------------------------------------------
// Formatter
// ---------------------------------------------------------------------

/// Format an event for display. Currently only `Text`; the dispatch shape
/// matches `zag-orch/src/listen.rs:392` so adding `RichText`/`Json`
/// formatters later is local to this function.
pub fn format_event(event: &SessionLogEvent, format: ListenFormat) -> Option<String> {
    match format {
        ListenFormat::Text => format_event_text(event),
    }
}

/// Plain-text formatter that mirrors the visual style `zig run` already
/// prints to stderr, so attaching mid-run feels familiar.
pub fn format_event_text(event: &SessionLogEvent) -> Option<String> {
    match &event.kind {
        SessionEventKind::ZigSessionStarted {
            workflow_name,
            tier_count,
            ..
        } => Some(format!(
            "▶ zig session started: {workflow_name} ({tier_count} tier{s})",
            s = if *tier_count == 1 { "" } else { "s" }
        )),
        SessionEventKind::TierStarted {
            tier_index,
            step_names,
        } => Some(format!(
            "── tier {} ({})",
            tier_index + 1,
            step_names.join(", ")
        )),
        SessionEventKind::StepStarted {
            step_name,
            zag_session_id,
            zag_command,
            model,
            ..
        } => {
            let model_info = model
                .as_deref()
                .map(|m| format!(" model={m}"))
                .unwrap_or_default();
            Some(format!(
                "  running step '{step_name}' (zag {zag_command}{model_info}, session={zag_session_id})..."
            ))
        }
        SessionEventKind::StepOutput {
            step_name, line, ..
        } => Some(format!("[{step_name}] {line}")),
        SessionEventKind::StepCompleted {
            step_name,
            duration_ms,
            saved_vars,
            ..
        } => {
            let saved = if saved_vars.is_empty() {
                String::new()
            } else {
                format!(" (saved: {})", saved_vars.join(", "))
            };
            Some(format!(
                "  completed '{step_name}' in {duration_ms}ms{saved}"
            ))
        }
        SessionEventKind::StepFailed {
            step_name,
            attempt,
            error,
            ..
        } => Some(format!(
            "  step '{step_name}' failed (attempt {attempt}): {error}"
        )),
        SessionEventKind::StepSkipped { step_name, reason } => {
            Some(format!("  skipping '{step_name}' ({reason})"))
        }
        SessionEventKind::Heartbeat { .. } => None, // suppress in text view
        SessionEventKind::ZigSessionEnded {
            status,
            duration_ms,
        } => Some(format!(
            "■ zig session ended ({:?}) in {duration_ms}ms",
            status
        )),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

#[cfg(test)]
#[path = "listen_tests.rs"]
mod tests;
