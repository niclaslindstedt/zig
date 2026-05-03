//! `zig continue` — re-open the most recent step's agent conversation
//! from the latest `zig run`.
//!
//! Resolves a target zig session from the per-project session index
//! (`~/.zig/projects/<id>/logs/index.json`), reads its JSONL log to find
//! the last `StepStarted` event, and hands the recorded `zag_session_id`
//! to [`zag_agent::builder::AgentBuilder::resume`] for an interactive
//! resume of that step's conversation.
//!
//! This MVP intentionally does not replay workflow orchestration: skipping
//! completed steps would lose their `saves` outputs, so any later step
//! depending on them would have undefined variables. Real orchestration
//! replay needs persisted variable state, which is a separate change.

use std::path::PathBuf;

use zag_agent::builder::AgentBuilder;

use crate::error::ZigError;
use crate::paths;
use crate::session::{
    SessionEventKind, SessionLogIndexEntry, load_project_index, read_session_events,
};

/// Options for `zig continue`.
pub struct ContinueOptions {
    /// Filter the most-recent lookup to a specific workflow name.
    pub workflow: Option<String>,
    /// Optional follow-up prompt. When `Some`, the resumed turn is driven
    /// non-interactively via `AgentBuilder::resume_with_prompt`; when
    /// `None`, the resumed session is opened interactively.
    pub prompt: Option<String>,
    /// Resume a specific zig session id (full UUID or unique prefix).
    pub session: Option<String>,
}

/// Resolved target for resumption.
#[derive(Debug)]
pub struct ResumeTarget {
    pub zig_session_id: String,
    pub workflow_name: String,
    pub zag_session_id: String,
    pub log_path: PathBuf,
}

/// Resolve which zag session to resume based on `opts`.
///
/// Resolution order:
/// 1. `opts.session` (exact id or unique prefix) → match in project index.
/// 2. `opts.workflow` → most recent project entry for that workflow name.
/// 3. Neither → most recent project entry.
///
/// In all cases the chosen entry's JSONL log is read and the **last**
/// `StepStarted` event determines the zag session id to resume.
pub fn resolve(opts: &ContinueOptions) -> Result<ResumeTarget, ZigError> {
    let entry = if let Some(id) = &opts.session {
        find_by_prefix(id)?
    } else if let Some(name) = &opts.workflow {
        find_latest_for_workflow(name)?
    } else {
        find_latest()?
    };

    let log_path = PathBuf::from(&entry.log_path);
    resolve_from_log(&log_path, entry)
}

/// Internal helper exposed for tests: given an already-loaded index entry
/// and its log path, pull out the last `StepStarted`'s zag session id.
pub fn resolve_from_log(
    log_path: &std::path::Path,
    entry: SessionLogIndexEntry,
) -> Result<ResumeTarget, ZigError> {
    let events = read_session_events(log_path)?;

    let zag_session_id = events
        .iter()
        .rev()
        .find_map(|e| match &e.kind {
            SessionEventKind::StepStarted { zag_session_id, .. } => Some(zag_session_id.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            ZigError::Io(format!(
                "session '{}' has no recorded step to resume",
                entry.zig_session_id
            ))
        })?;

    Ok(ResumeTarget {
        zig_session_id: entry.zig_session_id,
        workflow_name: entry.workflow_name,
        zag_session_id,
        log_path: log_path.to_path_buf(),
    })
}

fn project_sessions() -> Result<Vec<SessionLogIndexEntry>, ZigError> {
    let idx_path = paths::project_index_path(None)
        .ok_or_else(|| ZigError::Io("HOME environment variable not set".into()))?;
    if !idx_path.exists() {
        return Err(ZigError::Io(
            "no zig sessions yet — run `zig run` first.".into(),
        ));
    }
    Ok(load_project_index(&idx_path).sessions)
}

fn find_by_prefix(id: &str) -> Result<SessionLogIndexEntry, ZigError> {
    let sessions = project_sessions()?;
    let matches: Vec<_> = sessions
        .into_iter()
        .filter(|e| e.zig_session_id.starts_with(id))
        .collect();
    match matches.len() {
        0 => Err(ZigError::Io(format!("no session matches '{id}'"))),
        1 => Ok(matches.into_iter().next().unwrap()),
        n => Err(ZigError::Io(format!(
            "ambiguous session prefix '{id}' matches {n} sessions"
        ))),
    }
}

fn find_latest() -> Result<SessionLogIndexEntry, ZigError> {
    project_sessions()?
        .into_iter()
        .max_by(|a, b| a.started_at.cmp(&b.started_at))
        .ok_or_else(|| ZigError::Io("no zig sessions yet — run `zig run` first.".into()))
}

fn find_latest_for_workflow(name: &str) -> Result<SessionLogIndexEntry, ZigError> {
    project_sessions()?
        .into_iter()
        .filter(|e| e.workflow_name == name)
        .max_by(|a, b| a.started_at.cmp(&b.started_at))
        .ok_or_else(|| ZigError::Io(format!("no zig sessions found for workflow '{name}'.")))
}

/// Resume the most recent step's agent session.
///
/// With `opts.prompt = None`, the terminal attaches to the resumed
/// conversation interactively. With `opts.prompt = Some(p)`, the resumed
/// turn is driven non-interactively via
/// [`zag_agent::builder::AgentBuilder::resume_with_prompt`] and live event
/// output is streamed to stderr (matching the `zig run` UX).
pub async fn continue_run(opts: ContinueOptions) -> Result<(), ZigError> {
    let target = resolve(&opts)?;
    let short_id = &target.zig_session_id[..target.zig_session_id.len().min(8)];
    eprintln!(
        "resuming workflow '{}' (zig session {}, zag session {})",
        target.workflow_name, short_id, target.zag_session_id,
    );
    match opts.prompt {
        None => AgentBuilder::new()
            .resume(&target.zag_session_id)
            .await
            .map_err(|e| ZigError::Zag(format!("resume failed: {e}")))?,
        Some(prompt) => {
            let builder = crate::run::install_live_streaming(
                AgentBuilder::new(),
                &target.workflow_name,
                None,
                None,
            );
            builder
                .resume_with_prompt(&target.zag_session_id, &prompt)
                .await
                .map_err(|e| ZigError::Zag(format!("resume failed: {e}")))?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod tests;
