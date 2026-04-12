use std::path::PathBuf;

use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::state::AppState;

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

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

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
                        if let Some(user) = user_store.find_user(&username) {
                            let ctx = UserContext {
                                username,
                                home_dir: PathBuf::from(&user.home_dir),
                            };
                            let mut request = request;
                            request.extensions_mut().insert(ctx);
                            return next.run(request).await;
                        }
                    }
                }
                // Fall through to check legacy token before rejecting.
            }

            // Legacy token: acts as a super token in both modes.
            if token == state.config.token {
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
