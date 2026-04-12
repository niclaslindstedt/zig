use super::*;

#[test]
fn create_and_validate_token() {
    let mut store = TokenStore::new();
    let token = store.create_token("alice");
    assert!(!token.is_empty());
    assert_eq!(store.validate(&token), Some("alice"));
}

#[test]
fn validate_unknown_token_returns_none() {
    let store = TokenStore::new();
    assert!(store.validate("nonexistent").is_none());
}

#[test]
fn revoke_token() {
    let mut store = TokenStore::new();
    let token = store.create_token("alice");
    assert!(store.validate(&token).is_some());
    store.revoke(&token);
    assert!(store.validate(&token).is_none());
}

#[test]
fn generate_token_produces_hex() {
    let token = generate_token();
    assert_eq!(token.len(), 64); // 32 bytes = 64 hex chars
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn cleanup_expired_removes_old_tokens() {
    let mut store = TokenStore::new();
    let token = store.create_token("alice");

    // Manually expire the token.
    if let Some(entry) = store.tokens.get_mut(&token) {
        entry.expires_at = chrono::Utc::now() - chrono::Duration::hours(1);
    }

    assert!(store.validate(&token).is_none());
    store.cleanup_expired();
    assert!(store.tokens.is_empty());
}
