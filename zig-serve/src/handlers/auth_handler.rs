use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::state::AppState;
use crate::types::*;

/// POST /api/v1/login
///
/// Authenticates a user with username/password and returns a session token.
/// Only available when user-account mode is active (users.json exists).
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let user_store = match state.user_store {
        Some(ref store) => store,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "User accounts are not enabled on this server".to_string(),
                }),
            )
                .into_response();
        }
    };

    let token_store = match state.token_store {
        Some(ref store) => store,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Token store not available".to_string(),
                }),
            )
                .into_response();
        }
    };

    let authenticated = {
        let store = user_store.read().await;
        store
            .authenticate(&req.username, &req.password)
            .map(|u| (u.username.clone(), u.home_dir.clone()))
    };

    match authenticated {
        Some((username, home_dir)) => {
            let token = token_store.write().await.create_token(&username);
            (
                StatusCode::OK,
                Json(LoginResponse {
                    token,
                    username,
                    home_dir,
                }),
            )
                .into_response()
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid username or password".to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/logout
///
/// Revokes the current session token.
pub async fn logout(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> impl IntoResponse {
    let token_store = match state.token_store {
        Some(ref store) => store,
        None => {
            return (
                StatusCode::OK,
                Json(LogoutResponse {
                    message: "No session to revoke".to_string(),
                }),
            )
                .into_response();
        }
    };

    if let Some(auth_header) = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            token_store.write().await.revoke(token);
        }
    }

    (
        StatusCode::OK,
        Json(LogoutResponse {
            message: "Logged out successfully".to_string(),
        }),
    )
        .into_response()
}
