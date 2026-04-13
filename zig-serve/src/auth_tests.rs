use std::collections::HashMap;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use tokio::sync::Mutex;
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
            shutdown_timeout: Duration::from_secs(30),
            tls: false,
            tls_cert: None,
            tls_key: None,
            rate_limit: None,
            web: false,
        }),
        user_store: None,
        token_store: None,
        web_chats: Arc::new(Mutex::new(HashMap::new())),
    }
}

async fn dummy_handler() -> &'static str {
    "ok"
}

fn test_router(state: AppState) -> Router {
    Router::new()
        .route("/protected", get(dummy_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            super::auth_middleware,
        ))
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

#[tokio::test]
async fn skips_auth_for_health() {
    let state = test_state("secret");
    let app = Router::new()
        .route("/api/v1/health", get(dummy_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            super::auth_middleware,
        ))
        .with_state(state);

    let req = Request::builder()
        .uri("/api/v1/health")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn user_account_mode_validates_session_token() {
    use crate::session_token::TokenStore;
    use crate::user::{UserEntry, UserStore};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let mut token_store = TokenStore::new();
    let session_token = token_store.create_token("alice");

    let user_store = UserStore {
        users: vec![UserEntry {
            username: "alice".into(),
            password_hash: "unused".into(),
            home_dir: "/home/alice".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            enabled: true,
        }],
    };

    let state = AppState {
        config: Arc::new(ServeConfig {
            host: "127.0.0.1".into(),
            port: 3000,
            token: "legacy-token".into(),
            shutdown_timeout: Duration::from_secs(30),
            tls: false,
            tls_cert: None,
            tls_key: None,
            rate_limit: None,
            web: false,
        }),
        user_store: Some(Arc::new(user_store)),
        token_store: Some(Arc::new(RwLock::new(token_store))),
        web_chats: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = test_router(state);

    let req = Request::builder()
        .uri("/protected")
        .header("authorization", format!("Bearer {session_token}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
