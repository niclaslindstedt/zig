use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::error::ServeError;
use crate::types::*;
use crate::user::UserStore;

/// GET /api/v1/users
///
/// List all user accounts. Only available when user-account mode is active.
pub async fn list() -> Result<Json<Vec<UserListEntry>>, ServeError> {
    let store = tokio::task::spawn_blocking(UserStore::load)
        .await
        .map_err(|e| ServeError::bad_request(format!("task join error: {e}")))?
        .map_err(|e| ServeError::bad_request(e.to_string()))?;

    let entries: Vec<UserListEntry> = store
        .list_users()
        .iter()
        .map(|u| UserListEntry {
            username: u.username.clone(),
            home_dir: u.home_dir.clone(),
            enabled: u.enabled,
            created_at: u.created_at.clone(),
        })
        .collect();

    Ok(Json(entries))
}

/// POST /api/v1/users/add
///
/// Add a new user account. Requires legacy/super token.
pub async fn add(Json(req): Json<UserAddRequest>) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || {
        let mut store = UserStore::load()?;
        store.add_user(&req.username, &req.password, &req.home_dir)?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    })
    .await;

    match result {
        Ok(Ok(())) => (
            StatusCode::CREATED,
            Json(UserResponse {
                message: "User created".to_string(),
            }),
        )
            .into_response(),
        Ok(Err(e)) => (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("task join error: {e}"),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/users/remove
///
/// Remove a user account. Requires legacy/super token.
pub async fn remove(Json(req): Json<UserRemoveRequest>) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || {
        let mut store = UserStore::load()?;
        store.remove_user(&req.username)?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    })
    .await;

    match result {
        Ok(Ok(())) => (
            StatusCode::OK,
            Json(UserResponse {
                message: "User removed".to_string(),
            }),
        )
            .into_response(),
        Ok(Err(e)) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("task join error: {e}"),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/users/passwd
///
/// Change a user's password. Requires legacy/super token.
pub async fn passwd(Json(req): Json<UserPasswdRequest>) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || {
        let mut store = UserStore::load()?;
        store.change_password(&req.username, &req.password)?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    })
    .await;

    match result {
        Ok(Ok(())) => (
            StatusCode::OK,
            Json(UserResponse {
                message: "Password updated".to_string(),
            }),
        )
            .into_response(),
        Ok(Err(e)) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("task join error: {e}"),
            }),
        )
            .into_response(),
    }
}
