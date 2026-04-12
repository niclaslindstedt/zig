use serde::{Deserialize, Serialize};

/// Request body for POST /api/v1/login.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Response for POST /api/v1/login.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub home_dir: String,
}

/// Response for POST /api/v1/logout.
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

/// Request body for POST /api/v1/users/add.
#[derive(Debug, Deserialize)]
pub struct UserAddRequest {
    pub username: String,
    pub password: String,
    pub home_dir: String,
}

/// Request body for POST /api/v1/users/remove.
#[derive(Debug, Deserialize)]
pub struct UserRemoveRequest {
    pub username: String,
}

/// Request body for POST /api/v1/users/passwd.
#[derive(Debug, Deserialize)]
pub struct UserPasswdRequest {
    pub username: String,
    pub password: String,
}

/// Response for user management endpoints.
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub message: String,
}

/// A single user entry in list responses (no password hash).
#[derive(Debug, Serialize)]
pub struct UserListEntry {
    pub username: String,
    pub home_dir: String,
    pub enabled: bool,
    pub created_at: String,
}

/// Standard error response.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
