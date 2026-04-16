use std::convert::Infallible;
use std::path::PathBuf;
use std::time::Duration;

use axum::Json;
use axum::extract::Path;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use serde::Serialize;

use crate::error::ServeError;

#[cfg(test)]
#[path = "sessions_tests.rs"]
mod tests;

// -- List sessions --

pub async fn list() -> Result<Json<Vec<zig_core::session::SessionLogIndexEntry>>, ServeError> {
    let sessions = tokio::task::spawn_blocking(zig_core::session::list_sessions)
        .await
        .map_err(|e| ServeError::bad_request(format!("task join error: {e}")))?
        .map_err(ServeError::from)?;
    Ok(Json(sessions))
}

// -- Session detail --

#[derive(Serialize)]
pub struct SessionDetail {
    #[serde(flatten)]
    pub entry: zig_core::session::SessionLogIndexEntry,
    pub events: Vec<zig_core::session::SessionLogEvent>,
}

pub async fn detail(Path(id): Path<String>) -> Result<Json<SessionDetail>, ServeError> {
    let result = tokio::task::spawn_blocking(move || {
        let entry = zig_core::session::find_session(&id)?;
        let log_path = PathBuf::from(&entry.log_path);
        let events = zig_core::session::read_session_events(&log_path)?;
        Ok::<_, zig_core::error::ZigError>(SessionDetail { entry, events })
    })
    .await
    .map_err(|e| ServeError::bad_request(format!("task join error: {e}")))?
    .map_err(ServeError::from)?;
    Ok(Json(result))
}

// -- WebSocket stream --

pub async fn stream(
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ServeError> {
    // Resolve the session and log path before upgrading
    let entry = tokio::task::spawn_blocking(move || zig_core::session::find_session(&id))
        .await
        .map_err(|e| ServeError::bad_request(format!("task join error: {e}")))?
        .map_err(ServeError::from)?;

    let log_path = PathBuf::from(&entry.log_path);

    Ok(ws.on_upgrade(move |socket| handle_stream(socket, log_path)))
}

async fn handle_stream(mut socket: WebSocket, log_path: PathBuf) {
    let mut last_line = 0usize;

    loop {
        // Read new events from the JSONL file
        let events = match std::fs::read_to_string(&log_path) {
            Ok(content) => content,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
        };

        let lines: Vec<&str> = events.lines().collect();
        // Clamp so a truncated log (or a client-supplied oversize last_line
        // on SSE) can never panic at the slice boundary.
        let start = last_line.min(lines.len());
        let new_lines = &lines[start..];

        for line in new_lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if socket
                .send(Message::Text(line.to_string().into()))
                .await
                .is_err()
            {
                return; // Client disconnected
            }
        }

        last_line = lines.len();

        if session_ended(&lines) {
            return;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Returns true if the last non-empty JSONL line parses to a
/// `ZigSessionEnded` event. Substring-only matching is avoided so an agent
/// that echoes the literal string "zig_session_ended" doesn't close the
/// stream prematurely.
fn session_ended(lines: &[&str]) -> bool {
    for line in lines.iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        return matches!(
            serde_json::from_str::<zig_core::session::SessionLogEvent>(trimmed),
            Ok(event) if matches!(
                event.kind,
                zig_core::session::SessionEventKind::ZigSessionEnded { .. }
            )
        );
    }
    false
}

// -- SSE stream (alternative to WebSocket) --

/// GET /api/v1/sessions/{id}/events/stream
///
/// Server-Sent Events endpoint for live session streaming. Supports the
/// `Last-Event-ID` header for automatic reconnection — the client resumes
/// from the line number it last received.
pub async fn stream_sse(
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, ServeError> {
    let entry = tokio::task::spawn_blocking(move || zig_core::session::find_session(&id))
        .await
        .map_err(|e| ServeError::bad_request(format!("task join error: {e}")))?
        .map_err(ServeError::from)?;

    let log_path = PathBuf::from(&entry.log_path);

    let last_event_id: usize = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let stream = async_stream::stream! {
        let mut last_line = last_event_id;

        loop {
            let content = match std::fs::read_to_string(&log_path) {
                Ok(c) => c,
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }
            };

            let lines: Vec<&str> = content.lines().collect();
            // Clamp the start index — Last-Event-ID is client-supplied, and
            // the log can be truncated between polls. `&lines[start..]` must
            // never index past the end.
            let start = last_line.min(lines.len());

            for (i, line) in lines[start..].iter().enumerate() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let event_id = start + i + 1;
                yield Ok(Event::default()
                    .data(line)
                    .id(event_id.to_string()));
            }

            last_line = lines.len();

            if session_ended(&lines) {
                return;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
