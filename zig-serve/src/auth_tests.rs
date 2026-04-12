use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use tower::ServiceExt;

use crate::config::ServeConfig;
use crate::state::AppState;
use std::sync::Arc;

fn test_state(token: &str) -> AppState {
    AppState {
        config: Arc::new(ServeConfig {
            host: "127.0.0.1".into(),
            port: 3000,
            token: token.into(),
        }),
    }
}

async fn dummy_handler() -> &'static str {
    "ok"
}

fn test_router(state: AppState) -> Router {
    Router::new()
        .route(
            "/protected",
            get(dummy_handler).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                super::require_token,
            )),
        )
        .with_state(state)
}

#[tokio::test]
async fn rejects_missing_token() {
    let state = test_state("secret");
    let app = test_router(state);

    let req = Request::builder()
        .uri("/protected")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejects_wrong_token() {
    let state = test_state("secret");
    let app = test_router(state);

    let req = Request::builder()
        .uri("/protected")
        .header("authorization", "Bearer wrong")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn accepts_valid_token() {
    let state = test_state("secret");
    let app = test_router(state);

    let req = Request::builder()
        .uri("/protected")
        .header("authorization", "Bearer secret")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn rejects_non_bearer_auth() {
    let state = test_state("secret");
    let app = test_router(state);

    let req = Request::builder()
        .uri("/protected")
        .header("authorization", "Basic secret")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
