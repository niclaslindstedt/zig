use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};

use crate::config::ServeConfig;
use crate::handlers::chat::WebChatSession;
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
    /// Active web-chat sessions (used by the `--web` UI). Shared across handlers
    /// so follow-up messages can locate the spawned zag subprocess by id.
    pub web_chats: Arc<Mutex<HashMap<String, Arc<WebChatSession>>>>,
}
