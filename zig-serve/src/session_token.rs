use std::collections::HashMap;

/// Generate a cryptographically random 32-byte hex token.
pub fn generate_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Entry tracking a session token's owner and expiration.
#[derive(Debug, Clone)]
pub struct TokenEntry {
    pub username: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// In-memory store of active session tokens.
#[derive(Debug, Default)]
pub struct TokenStore {
    tokens: HashMap<String, TokenEntry>,
    /// Token lifetime in hours (default 24).
    token_lifetime_hours: i64,
}

impl TokenStore {
    /// Create a new token store with the default 24-hour token lifetime.
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            token_lifetime_hours: 24,
        }
    }

    /// Create a session token for the given username.
    pub fn create_token(&mut self, username: &str) -> String {
        let token = generate_token();
        let now = chrono::Utc::now();
        let entry = TokenEntry {
            username: username.to_string(),
            created_at: now,
            expires_at: now + chrono::Duration::hours(self.token_lifetime_hours),
        };
        self.tokens.insert(token.clone(), entry);
        token
    }

    /// Validate a token and return the username if valid and not expired.
    pub fn validate(&self, token: &str) -> Option<&str> {
        let entry = self.tokens.get(token)?;
        if chrono::Utc::now() > entry.expires_at {
            return None;
        }
        Some(&entry.username)
    }

    /// Revoke a session token.
    pub fn revoke(&mut self, token: &str) {
        self.tokens.remove(token);
    }

    /// Remove all expired tokens.
    pub fn cleanup_expired(&mut self) {
        let now = chrono::Utc::now();
        self.tokens.retain(|_, entry| entry.expires_at > now);
    }
}

#[cfg(test)]
#[path = "session_token_tests.rs"]
mod tests;
