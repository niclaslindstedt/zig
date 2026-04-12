mod auth;
pub mod config;
mod error;
mod handlers;
mod rate_limit;
mod router;
pub mod session_token;
mod shutdown;
mod state;
mod tls;
pub mod types;
pub mod user;

use std::sync::Arc;

use config::ServeConfig;
use state::AppState;
use tokio::sync::RwLock;

/// Start the zig API server.
pub async fn start_server(
    config: ServeConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("{}:{}", config.host, config.port);
    let token = config.token.clone();
    let shutdown_timeout = config.shutdown_timeout;

    // Determine auth mode: user accounts (if users.json exists) or legacy token.
    let (user_store, token_store) = if user::UserStore::exists() {
        let store = user::UserStore::load()?;
        eprintln!("user accounts mode: loaded {} user(s)", store.users.len());
        (
            Some(Arc::new(store)),
            Some(Arc::new(RwLock::new(session_token::TokenStore::new()))),
        )
    } else {
        (None, None)
    };

    // Resolve TLS configuration.
    let tls_params = tls::resolve_tls(&config)?;

    let state = AppState {
        config: Arc::new(config),
        user_store,
        token_store,
    };

    let router = router::build_router(state);

    if let Some((cert, key)) = tls_params {
        eprintln!("zig serve listening on https://{addr}");
        eprintln!("Token: {token}");

        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert, &key).await?;

        let handle = axum_server::Handle::new();
        let shutdown_handle = handle.clone();

        tokio::spawn(async move {
            shutdown::shutdown_signal().await;
            shutdown_handle.graceful_shutdown(Some(shutdown_timeout));
        });

        axum_server::bind_rustls(addr.parse()?, tls_config)
            .handle(handle)
            .serve(router.into_make_service())
            .await?;
    } else {
        eprintln!("zig serve listening on http://{addr}");
        eprintln!("Token: {token}");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown::shutdown_signal())
            .await?;
    }

    eprintln!("server stopped");
    Ok(())
}
