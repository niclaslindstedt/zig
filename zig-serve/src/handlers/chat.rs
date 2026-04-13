//! Web-chat handlers that back the React UI served by `zig serve --web`.
//!
//! The CLI's `workflow create` flow spawns `zag run <initial_prompt> --system-prompt ...`
//! as a short-lived interactive process. For the web UI we need the same
//! subprocess but long-lived so the user can exchange follow-up messages with
//! the agent. Each POST to `/api/v1/web/chat` spawns one subprocess, stores it
//! in [`AppState::web_chats`] keyed by a UUID, and wires up:
//!
//! - an `mpsc::Sender<String>` for follow-up messages → child's stdin
//! - a `broadcast::Sender<String>` that fans agent stdout/stderr out to any
//!   number of SSE subscribers.
//!
//! The SSE endpoint (`GET /api/v1/web/chat/{id}/stream`) subscribes to that
//! broadcast channel and yields `{role, text}` JSON events until the subprocess
//! exits. Auth for the SSE endpoint also accepts a `?token=` query parameter
//! because `EventSource` cannot set headers.

use std::convert::Infallible;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::error::ServeError;
use crate::state::AppState;

/// A single live conversation with a zag subprocess.
pub struct WebChatSession {
    /// Channel used to send follow-up messages to the child's stdin.
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

    // Spawn `zag run` with the rendered prompts. stdin is piped so we can feed
    // follow-up messages into the interactive session.
    let mut child = Command::new("zag")
        .arg("run")
        .arg(&req.initial_prompt)
        .arg("--system-prompt")
        .arg(&params.system_prompt)
        .arg("--name")
        .arg(&params.session_name)
        .arg("--tag")
        .arg(&params.session_tag)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| ServeError::bad_request(format!("failed to spawn zag: {e}")))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| ServeError::bad_request("zag child has no stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ServeError::bad_request("zag child has no stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ServeError::bad_request("zag child has no stderr"))?;

    let (stdin_tx, stdin_rx) = mpsc::channel::<String>(32);
    let (events_tx, _events_rx) = broadcast::channel::<String>(256);

    // Pump: user messages → child stdin.
    let stdin = Arc::new(Mutex::new(stdin));
    spawn_stdin_pump(Arc::clone(&stdin), stdin_rx);

    // Pump: child stdout lines → broadcast as agent events.
    spawn_line_reader(stdout, events_tx.clone(), "agent");
    // Pump: child stderr lines → broadcast as system events.
    spawn_line_reader(stderr, events_tx.clone(), "system");

    // Watchdog: when the child exits, announce it and drop it from the map.
    let session_id = Uuid::new_v4().to_string();
    let session_id_for_task = session_id.clone();
    let events_for_exit = events_tx.clone();
    let chats_for_exit = Arc::clone(&state.web_chats);
    tokio::spawn(async move {
        let _ = wait_for_exit(&mut child, &events_for_exit).await;
        chats_for_exit.lock().await.remove(&session_id_for_task);
    });

    let session = Arc::new(WebChatSession {
        stdin_tx,
        events: events_tx,
    });
    state
        .web_chats
        .lock()
        .await
        .insert(session_id.clone(), Arc::clone(&session));

    Ok(Json(StartChatResponse {
        session_id,
        output_path: params.output_path,
    }))
}

fn spawn_stdin_pump(stdin: Arc<Mutex<ChildStdin>>, mut rx: mpsc::Receiver<String>) {
    tokio::spawn(async move {
        while let Some(mut msg) = rx.recv().await {
            if !msg.ends_with('\n') {
                msg.push('\n');
            }
            let mut guard = stdin.lock().await;
            if let Err(e) = guard.write_all(msg.as_bytes()).await {
                tracing::debug!("chat stdin write failed: {e}");
                return;
            }
            let _ = guard.flush().await;
        }
    });
}

fn spawn_line_reader<R>(reader: R, events: broadcast::Sender<String>, role: &'static str)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => broadcast_event(&events, role, line),
                Ok(None) => return,
                Err(e) => {
                    tracing::debug!("chat line reader error: {e}");
                    return;
                }
            }
        }
    });
}

async fn wait_for_exit(
    child: &mut Child,
    events: &broadcast::Sender<String>,
) -> std::io::Result<()> {
    let status = child.wait().await?;
    broadcast_event(events, "system", format!("session ended ({status})"));
    Ok(())
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
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, ServeError> {
    let session = {
        let chats = state.web_chats.lock().await;
        chats
            .get(&id)
            .cloned()
            .ok_or_else(|| ServeError::not_found(format!("chat session {id} not found")))?
    };

    let rx = session.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(payload) => Some(Ok(Event::default().data(payload))),
        Err(_) => None,
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
