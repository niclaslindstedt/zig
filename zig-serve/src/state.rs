use std::sync::Arc;

use crate::config::ServeConfig;

/// Shared application state passed to all handlers via Axum's state mechanism.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServeConfig>,
}
