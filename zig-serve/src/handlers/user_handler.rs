use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::error::ServeError;
use crate::state::AppState;
use crate::types::*;
use crate::user::UserStore;

/// GET /api/v1/users
///
/// List all user accounts. Admin-only (gated by `require_admin` middleware).
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<UserListEntry>>, ServeError> {
    let Some(store_lock) = state.user_store.clone() else {
        return Err(ServeError::bad_request("user accounts are not enabled"));
    };

    let store = store_lock.read().await;
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
/// Add a new user account. Admin-only.
pub async fn add(
    State(state): State<AppState>,
    Json(req): Json<UserAddRequest>,
) -> impl IntoResponse {
    // Hold a write lock across the load → mutate → save cycle so two
    // concurrent admin requests can't race and clobber each other's writes.
    let Some(store_lock) = state.user_store.clone() else {
        return bad_request("user accounts are not enabled");
    };
    let mut store = store_lock.write().await;

    let add_result = tokio::task::block_in_place(|| -> Result<UserStore, String> {
        let mut fresh = UserStore::load().map_err(|e| e.to_string())?;
        fresh
            .add_user(&req.username, &req.password, &req.home_dir)
            .map_err(|e| e.to_string())?;
        Ok(fresh)
    });

    match add_result {
        Ok(fresh) => {
            *store = fresh;
            (
                StatusCode::CREATED,
                Json(UserResponse {
                    message: "User created".to_string(),
                }),
            )
                .into_response()
        }
        Err(e) => (StatusCode::CONFLICT, Json(ErrorResponse { error: e })).into_response(),
    }
}

/// POST /api/v1/users/remove
///
/// Remove a user account. Admin-only.
pub async fn remove(
    State(state): State<AppState>,
    Json(req): Json<UserRemoveRequest>,
) -> impl IntoResponse {
    let Some(store_lock) = state.user_store.clone() else {
        return bad_request("user accounts are not enabled");
    };
    let mut store = store_lock.write().await;

    let remove_result = tokio::task::block_in_place(|| -> Result<UserStore, String> {
        let mut fresh = UserStore::load().map_err(|e| e.to_string())?;
        fresh
            .remove_user(&req.username)
            .map_err(|e| e.to_string())?;
        Ok(fresh)
    });

    match remove_result {
        Ok(fresh) => {
            *store = fresh;
            (
                StatusCode::OK,
                Json(UserResponse {
                    message: "User removed".to_string(),
                }),
            )
                .into_response()
        }
        Err(e) => (StatusCode::NOT_FOUND, Json(ErrorResponse { error: e })).into_response(),
    }
}

/// POST /api/v1/users/passwd
///
/// Change a user's password. Admin-only.
pub async fn passwd(
    State(state): State<AppState>,
    Json(req): Json<UserPasswdRequest>,
) -> impl IntoResponse {
    let Some(store_lock) = state.user_store.clone() else {
        return bad_request("user accounts are not enabled");
    };
    let mut store = store_lock.write().await;

    let passwd_result = tokio::task::block_in_place(|| -> Result<UserStore, String> {
        let mut fresh = UserStore::load().map_err(|e| e.to_string())?;
        fresh
            .change_password(&req.username, &req.password)
            .map_err(|e| e.to_string())?;
        Ok(fresh)
    });

    match passwd_result {
        Ok(fresh) => {
            *store = fresh;
            (
                StatusCode::OK,
                Json(UserResponse {
                    message: "Password updated".to_string(),
                }),
            )
                .into_response()
        }
        Err(e) => (StatusCode::NOT_FOUND, Json(ErrorResponse { error: e })).into_response(),
    }
}

fn bad_request(msg: &str) -> axum::response::Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
        .into_response()
}
