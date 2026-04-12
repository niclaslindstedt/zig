use std::sync::Arc;

use tokio::sync::RwLock;

use crate::config::ServeConfig;
use crate::session_token::TokenStore;
use crate::user::UserStore;

/// Shared application state passed to all handlers via Axum's state mechanism.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServeConfig>,
    /// User store (Some when user-account mode is active).
    pub user_store: Option<Arc<UserStore>>,
    /// Session token store (Some when user-account mode is active).
    pub token_store: Option<Arc<RwLock<TokenStore>>>,
}
