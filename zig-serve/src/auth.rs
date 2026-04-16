use std::path::PathBuf;

use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use subtle::ConstantTimeEq;

use crate::state::AppState;

/// Compare two byte strings in constant time so a timing side channel can't
/// leak a prefix of the expected value to a repeating attacker.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

/// Decode a percent-encoded query value. Returns `None` if any escape is
/// malformed so we don't silently accept a corrupted token.
fn urlencoding_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => {
                // A `%` must be followed by exactly two hex digits. Anything
                // else (truncated escape or non-hex characters) is malformed
                // — reject the input rather than silently pass the `%` through.
                if i + 3 > bytes.len() {
                    return None;
                }
                let hi = (bytes[i + 1] as char).to_digit(16)?;
                let lo = (bytes[i + 2] as char).to_digit(16)?;
                out.push((hi * 16 + lo) as u8);
                i += 3;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// User context attached to requests when user-account mode is active.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UserContext {
    pub username: String,
    pub home_dir: PathBuf,
}

/// Marker inserted into request extensions when the legacy token is used.
#[derive(Debug, Clone)]
pub struct LegacyTokenContext;

/// Admin gate: allow only requests authenticated with the legacy/super token.
///
/// Session-token users (user-account mode) are rejected with 403. Must be
/// layered AFTER [`auth_middleware`] so the context is already populated.
pub async fn require_admin(request: Request, next: Next) -> Response {
    if request.extensions().get::<LegacyTokenContext>().is_some() {
        next.run(request).await
    } else {
        (StatusCode::FORBIDDEN, "Admin token required").into_response()
    }
}

/// Authentication middleware supporting two modes:
///
/// - **User accounts** (if `users.json` exists): validates session tokens issued
///   via `/api/v1/login` and attaches [`UserContext`] to the request.
/// - **Legacy single token** (fallback): validates `Authorization: Bearer <token>`
///   against the configured server token and attaches [`LegacyTokenContext`].
///
/// Skips auth for `/api/v1/health` and `/api/v1/login`.
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if path == "/api/v1/health" || path == "/api/v1/login" {
        return next.run(request).await;
    }

    // SSE endpoints (web-chat streaming) can't set the Authorization header
    // from the browser's EventSource API, so we also accept the token as a
    // query parameter on those paths.
    let query_token = if path.starts_with("/api/v1/web/chat/") && path.ends_with("/stream") {
        request
            .uri()
            .query()
            .and_then(|q| {
                q.split('&').find_map(|kv| {
                    let mut parts = kv.splitn(2, '=');
                    match (parts.next(), parts.next()) {
                        (Some("token"), Some(val)) => Some(val.to_string()),
                        _ => None,
                    }
                })
            })
            .and_then(|encoded| urlencoding_decode(&encoded))
    } else {
        None
    };

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .or_else(|| query_token.map(|t| format!("Bearer {t}")));

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = header[7..].to_string();

            // User-account mode: validate session token and attach UserContext.
            if let Some(ref token_store) = state.token_store {
                let ts = token_store.read().await;
                if let Some(username) = ts.validate(&token) {
                    let username = username.to_string();
                    drop(ts);
                    if let Some(ref user_store) = state.user_store {
                        let store = user_store.read().await;
                        if let Some(user) = store.find_user(&username) {
                            let ctx = UserContext {
                                username,
                                home_dir: PathBuf::from(&user.home_dir),
                            };
                            drop(store);
                            let mut request = request;
                            request.extensions_mut().insert(ctx);
                            return next.run(request).await;
                        }
                    }
                }
                // Fall through to check legacy token before rejecting.
            }

            // Legacy token: acts as a super token in both modes. Use a
            // constant-time comparison so we don't leak a prefix of the
            // configured token via request-timing side channels.
            if constant_time_eq(token.as_bytes(), state.config.token.as_bytes()) {
                let mut request = request;
                request.extensions_mut().insert(LegacyTokenContext);
                return next.run(request).await;
            }

            (StatusCode::UNAUTHORIZED, "Invalid token").into_response()
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization header",
        )
            .into_response(),
    }
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
