use std::sync::Arc;

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use axum::middleware;
use axum::routing::{get, post};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::auth;
use crate::config::ServeConfig;
use crate::handlers::{auth_handler, chat, health, man, sessions, user_handler, workflows};
use crate::rate_limit;
use crate::state::AppState;
use crate::web;

/// Build the CORS layer. We intentionally do **not** use
/// [`CorsLayer::permissive`] because the API is authenticated: any origin
/// that can read an `Authorization` bearer response would otherwise be able
/// to issue authenticated cross-origin requests on behalf of the user.
///
/// Allowed origins default to the server's own http/https host + localhost
/// variants so the bundled web UI works out of the box.
fn build_cors_layer(config: &ServeConfig) -> CorsLayer {
    let mut origins: Vec<HeaderValue> = Vec::new();
    let port = config.port;

    for host in ["127.0.0.1", "localhost", "[::1]"] {
        for scheme in ["http", "https"] {
            if let Ok(v) = HeaderValue::from_str(&format!("{scheme}://{host}:{port}")) {
                origins.push(v);
            }
        }
    }

    if !matches!(
        config.host.as_str(),
        "127.0.0.1" | "localhost" | "0.0.0.0" | "::" | "[::]" | "[::1]"
    ) {
        let scheme = if config.tls || config.tls_cert.is_some() {
            "https"
        } else {
            "http"
        };
        if let Ok(v) = HeaderValue::from_str(&format!("{scheme}://{}:{}", config.host, port)) {
            origins.push(v);
        }
    }

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_credentials(true)
}

pub fn build_router(state: AppState) -> Router {
    let web_enabled = state.config.web;

    // User management routes are gated with an extra admin middleware so
    // that only the legacy/super token can touch them (session-token users
    // get 403).
    let user_routes: Router<AppState> = Router::new()
        .route("/api/v1/users", get(user_handler::list))
        .route("/api/v1/users/add", post(user_handler::add))
        .route("/api/v1/users/remove", post(user_handler::remove))
        .route("/api/v1/users/passwd", post(user_handler::passwd))
        .layer(middleware::from_fn(auth::require_admin));

    let mut app = Router::new()
        // Health (no auth)
        .route("/api/v1/health", get(health::health))
        // Login (auth middleware skips this path)
        .route("/api/v1/login", post(auth_handler::login))
        .route("/api/v1/logout", post(auth_handler::logout))
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
        .route(
            "/api/v1/sessions/{id}/events/stream",
            get(sessions::stream_sse),
        )
        // Manpages
        .route("/api/v1/man", get(man::list))
        .route("/api/v1/man/{topic}", get(man::show))
        .merge(user_routes);

    // Web-chat routes (only when --web is enabled).
    if web_enabled {
        app = app
            .route("/api/v1/web/chat", post(chat::start_chat))
            .route("/api/v1/web/chat/{id}", post(chat::send_message))
            .route("/api/v1/web/chat/{id}/stream", get(chat::stream_chat));
    }

    // Auth middleware (skips /health and /login internally).
    app = app.layer(middleware::from_fn_with_state(
        state.clone(),
        auth::auth_middleware,
    ));

    // Rate limiting (optional).
    if let Some(rps) = state.config.rate_limit {
        let limiter = rate_limit::build_rate_limiter(rps);
        app = app.layer(middleware::from_fn(move |req, next| {
            let limiter = Arc::clone(&limiter);
            rate_limit::rate_limit_middleware(req, next, limiter)
        }));
    }

    // Static web UI routes are mounted AFTER auth so the SPA is public.
    // API routes are matched first thanks to Axum's route precedence on the
    // specific `/api/v1/*` prefix.
    if web_enabled {
        app = app
            .route("/", get(web::index))
            .route("/{*path}", get(web::asset));
    }

    let cors = build_cors_layer(&state.config);

    app.layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
