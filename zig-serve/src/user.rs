use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A single user account entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEntry {
    pub username: String,
    pub password_hash: String,
    pub home_dir: String,
    pub created_at: String,
    pub enabled: bool,
}

/// Persistent user store backed by a JSON file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserStore {
    pub users: Vec<UserEntry>,
}

impl UserStore {
    /// Path to the users.json file: `~/.zig/users.json`.
    pub fn path() -> PathBuf {
        zig_core::paths::global_base_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("users.json")
    }

    /// Check if a users.json file exists (determines whether user-account mode is active).
    pub fn exists() -> bool {
        Self::path().exists()
    }

    /// Load user store from disk. Returns empty store if file doesn't exist.
    pub fn load() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let store: Self = serde_json::from_str(&content)?;
        Ok(store)
    }

    /// Save user store to disk.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Add a new user. Hashes the password with bcrypt.
    pub fn add_user(
        &mut self,
        username: &str,
        password: &str,
        home_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.find_user(username).is_some() {
            return Err(format!("user '{}' already exists", username).into());
        }
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        let entry = UserEntry {
            username: username.to_string(),
            password_hash,
            home_dir: home_dir.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            enabled: true,
        };
        self.users.push(entry);
        self.save()
    }

    /// Remove a user by username.
    pub fn remove_user(
        &mut self,
        username: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let len_before = self.users.len();
        self.users.retain(|u| u.username != username);
        if self.users.len() == len_before {
            return Err(format!("user '{}' not found", username).into());
        }
        self.save()
    }

    /// Change a user's password.
    pub fn change_password(
        &mut self,
        username: &str,
        new_password: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let user = self
            .users
            .iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| format!("user '{}' not found", username))?;
        user.password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)?;
        self.save()
    }

    /// Authenticate a user by username and password. Returns the user entry if valid.
    pub fn authenticate(&self, username: &str, password: &str) -> Option<&UserEntry> {
        let user = self.find_user(username)?;
        if !user.enabled {
            return None;
        }
        if bcrypt::verify(password, &user.password_hash).unwrap_or(false) {
            Some(user)
        } else {
            None
        }
    }

    /// Find a user by username.
    pub fn find_user(&self, username: &str) -> Option<&UserEntry> {
        self.users.iter().find(|u| u.username == username)
    }

    /// List all users.
    pub fn list_users(&self) -> &[UserEntry] {
        &self.users
    }
}

#[cfg(test)]
#[path = "user_tests.rs"]
mod tests;
