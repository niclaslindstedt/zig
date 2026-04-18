//! Web-chat handlers that back the React UI served by `zig serve --web`.
//!
//! Each POST to `/api/v1/web/chat` starts a new [`AgentBuilder::exec_streaming`]
//! session (Claude only — the bidirectional stream-json path is a Claude
//! feature). The resulting [`StreamingSession`] stays alive in
//! [`AppState::web_chats`] keyed by a UUID, and we wire up:
//!
//! - an `mpsc::Sender<String>` for follow-up user messages → session's
//!   stdin via [`StreamingSession::send_user_message`]
//! - a `broadcast::Sender<String>` that fans unified [`Event`] values out
//!   as `{role, text}` JSON to any number of SSE subscribers.
//!
//! The SSE endpoint (`GET /api/v1/web/chat/{id}/stream`) subscribes to that
//! broadcast channel until the streaming session ends. Auth for the SSE
//! endpoint also accepts a `?token=` query parameter because `EventSource`
//! cannot set headers.

use std::convert::Infallible;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;
use zag_agent::builder::AgentBuilder;
use zag_agent::output::{ContentBlock, Event as AgentEvent};
use zag_agent::streaming::StreamingSession;

use crate::error::ServeError;
use crate::state::AppState;

/// A single live conversation with a zag-agent streaming session.
pub struct WebChatSession {
    /// Channel used to send follow-up user messages into the session.
    stdin_tx: mpsc::Sender<String>,
    /// Fan-out channel emitting JSON-encoded chat events.
    events: broadcast::Sender<String>,
}

impl WebChatSession {
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.events.subscribe()
    }

    pub async fn send(&self, message: String) -> Result<(), ServeError> {
        self.stdin_tx
            .send(message)
            .await
            .map_err(|_| ServeError::bad_request("chat session has ended"))
    }
}

#[derive(Serialize, Deserialize)]
struct ChatEvent<'a> {
    role: &'a str,
    text: String,
}

fn broadcast_event(events: &broadcast::Sender<String>, role: &str, text: String) {
    let payload = serde_json::to_string(&ChatEvent { role, text })
        .unwrap_or_else(|_| String::from("{\"role\":\"system\",\"text\":\"encoding error\"}"));
    // Ignore send errors — they only mean no subscribers are listening yet.
    let _ = events.send(payload);
}

// -- POST /api/v1/web/chat --------------------------------------------------

#[derive(Deserialize)]
pub struct StartChatRequest {
    pub initial_prompt: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct StartChatResponse {
    pub session_id: String,
    pub output_path: String,
}

pub async fn start_chat(
    State(state): State<AppState>,
    Json(req): Json<StartChatRequest>,
) -> Result<Json<StartChatResponse>, ServeError> {
    // Build the create prompts (system prompt + resolved output path).
    let name = req.name.clone();
    let params = tokio::task::spawn_blocking(move || {
        zig_core::create::prepare_create(name.as_deref(), None, None)
    })
    .await
    .map_err(|e| ServeError::bad_request(format!("task join error: {e}")))?
    .map_err(ServeError::from)?;

    // Start a Claude streaming session with the rendered prompts.
    let session = AgentBuilder::new()
        .provider("claude")
        .system_prompt(&params.system_prompt)
        .name(&params.session_name)
        .tag(&params.session_tag)
        .exec_streaming(&req.initial_prompt)
        .await
        .map_err(|e| ServeError::bad_request(format!("failed to start agent: {e}")))?;

    let (stdin_tx, stdin_rx) = mpsc::channel::<String>(32);
    let (events_tx, _events_rx) = broadcast::channel::<String>(256);

    // The StreamingSession is not Clone — it owns the child process. Wrap it
    // in an Arc<Mutex<>> so the stdin-pump and event-reader tasks can both
    // drive it without racing.
    let session = Arc::new(Mutex::new(session));

    spawn_stdin_pump(Arc::clone(&session), stdin_rx, events_tx.clone());

    let session_id = Uuid::new_v4().to_string();
    let session_for_events = Arc::clone(&session);
    let events_for_reader = events_tx.clone();
    let chats_for_exit = Arc::clone(&state.web_chats);
    let session_id_for_task = session_id.clone();
    tokio::spawn(async move {
        pump_events(session_for_events, events_for_reader.clone()).await;
        broadcast_event(&events_for_reader, "system", "session ended".to_string());
        chats_for_exit.lock().await.remove(&session_id_for_task);
    });

    let chat = Arc::new(WebChatSession {
        stdin_tx,
        events: events_tx,
    });
    state
        .web_chats
        .lock()
        .await
        .insert(session_id.clone(), Arc::clone(&chat));

    Ok(Json(StartChatResponse {
        session_id,
        output_path: params.output_path,
    }))
}

/// Forward incoming user messages into the streaming session's stdin.
fn spawn_stdin_pump(
    session: Arc<Mutex<StreamingSession>>,
    mut rx: mpsc::Receiver<String>,
    events: broadcast::Sender<String>,
) {
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let mut guard = session.lock().await;
            if let Err(e) = guard.send_user_message(&msg).await {
                broadcast_event(&events, "system", format!("send failed: {e}"));
                return;
            }
        }
        // Channel closed — signal end of input to the agent.
        let mut guard = session.lock().await;
        guard.close_input();
    });
}

/// Drain the streaming session's event stream and fan each unified event
/// out as a `{role, text}` broadcast message. Returns when the session
/// ends (subprocess exits).
async fn pump_events(session: Arc<Mutex<StreamingSession>>, events: broadcast::Sender<String>) {
    loop {
        let mut guard = session.lock().await;
        match guard.next_event().await {
            Ok(Some(event)) => {
                drop(guard);
                route_event(&events, event);
            }
            Ok(None) => return,
            Err(e) => {
                drop(guard);
                broadcast_event(&events, "system", format!("event read error: {e}"));
                return;
            }
        }
    }
}

/// Map a unified [`AgentEvent`] to the `{role, text}` shape the React UI
/// expects. Events without a user-visible text body (init, usage, partial
/// token chunks) are ignored.
fn route_event(events: &broadcast::Sender<String>, event: AgentEvent) {
    match event {
        AgentEvent::AssistantMessage { content, .. } => {
            let text = flatten_content(&content);
            if !text.is_empty() {
                broadcast_event(events, "agent", text);
            }
        }
        AgentEvent::UserMessage { content, .. } => {
            let text = flatten_content(&content);
            if !text.is_empty() {
                broadcast_event(events, "user", text);
            }
        }
        AgentEvent::ToolExecution {
            tool_name, input, ..
        } => {
            let input_preview = serde_json::to_string(&input).unwrap_or_default();
            broadcast_event(
                events,
                "system",
                format!("tool: {tool_name}({input_preview})"),
            );
        }
        AgentEvent::Error { message, .. } => {
            broadcast_event(events, "system", format!("error: {message}"));
        }
        AgentEvent::Result {
            success, message, ..
        } => {
            if !success && let Some(m) = message {
                broadcast_event(events, "system", format!("turn failed: {m}"));
            }
        }
        _ => {}
    }
}

fn flatten_content(blocks: &[ContentBlock]) -> String {
    let mut out = String::new();
    for block in blocks {
        if let ContentBlock::Text { text } = block {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(text);
        }
    }
    out
}

// -- POST /api/v1/web/chat/{id} ---------------------------------------------

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub message: String,
}

#[derive(Serialize)]
pub struct SendMessageResponse {
    pub ok: bool,
}

pub async fn send_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, ServeError> {
    let session = {
        let chats = state.web_chats.lock().await;
        chats
            .get(&id)
            .cloned()
            .ok_or_else(|| ServeError::not_found(format!("chat session {id} not found")))?
    };

    // Echo the user message back onto the broadcast stream so every subscriber
    // (including late ones) has a consistent transcript.
    broadcast_event(&session.events, "user", req.message.clone());
    session.send(req.message).await?;

    Ok(Json(SendMessageResponse { ok: true }))
}

// -- GET /api/v1/web/chat/{id}/stream ---------------------------------------

pub async fn stream_chat(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<SseEvent, Infallible>>>, ServeError> {
    let session = {
        let chats = state.web_chats.lock().await;
        chats
            .get(&id)
            .cloned()
            .ok_or_else(|| ServeError::not_found(format!("chat session {id} not found")))?
    };

    let rx = session.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(payload) => Some(Ok(SseEvent::default().data(payload))),
        Err(_) => None,
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
