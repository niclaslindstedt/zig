use std::path::PathBuf;
use std::time::Duration;

use axum::Json;
use axum::extract::Path;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::Serialize;

use crate::error::ServeError;

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
        let new_lines = &lines[last_line..];

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

        // Check if the session has ended (look for ZigSessionEnded event)
        if let Some(last) = lines.last() {
            if last.contains("\"zig_session_ended\"") {
                return;
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
