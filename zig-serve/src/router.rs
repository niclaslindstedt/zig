use axum::Router;
use axum::middleware;
use axum::routing::{get, post};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::auth;
use crate::handlers::{health, man, sessions, workflows};
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let public = Router::new().route("/api/v1/health", get(health::health));

    let authed = Router::new()
        // Workflows
        .route("/api/v1/workflows", get(workflows::list))
        .route("/api/v1/workflows/validate", post(workflows::validate))
        .route("/api/v1/workflows/run", post(workflows::run))
        .route("/api/v1/workflows/create", post(workflows::create))
        .route(
            "/api/v1/workflows/{name}",
            get(workflows::show).delete(workflows::delete),
        )
        // Sessions
        .route("/api/v1/sessions", get(sessions::list))
        .route("/api/v1/sessions/{id}", get(sessions::detail))
        .route("/api/v1/sessions/{id}/stream", get(sessions::stream))
        // Manpages
        .route("/api/v1/man", get(man::list))
        .route("/api/v1/man/{topic}", get(man::show))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_token,
        ));

    Router::new()
        .merge(public)
        .merge(authed)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
