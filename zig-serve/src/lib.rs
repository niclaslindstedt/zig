mod auth;
pub mod config;
mod error;
mod handlers;
mod router;
mod state;

use std::sync::Arc;

use config::ServeConfig;
use state::AppState;

/// Start the zig API server.
pub async fn start_server(config: ServeConfig) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", config.host, config.port);
    let token = config.token.clone();

    let state = AppState {
        config: Arc::new(config),
    };

    let router = router::build_router(state);

    eprintln!("zig serve listening on http://{addr}");
    eprintln!("Token: {token}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
