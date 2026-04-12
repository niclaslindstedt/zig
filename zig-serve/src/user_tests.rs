use super::*;

fn make_store_with_user() -> UserStore {
    let hash = bcrypt::hash("password123", 4).unwrap(); // cost=4 for fast tests
    UserStore {
        users: vec![UserEntry {
            username: "alice".into(),
            password_hash: hash,
            home_dir: "/home/alice".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            enabled: true,
        }],
    }
}

#[test]
fn find_user_returns_existing() {
    let store = make_store_with_user();
    let user = store.find_user("alice");
    assert!(user.is_some());
    assert_eq!(user.unwrap().username, "alice");
}

#[test]
fn find_user_returns_none_for_unknown() {
    let store = make_store_with_user();
    assert!(store.find_user("bob").is_none());
}

#[test]
fn authenticate_valid_credentials() {
    let store = make_store_with_user();
    let user = store.authenticate("alice", "password123");
    assert!(user.is_some());
}

#[test]
fn authenticate_wrong_password() {
    let store = make_store_with_user();
    assert!(store.authenticate("alice", "wrong").is_none());
}

#[test]
fn authenticate_disabled_user() {
    let mut store = make_store_with_user();
    store.users[0].enabled = false;
    assert!(store.authenticate("alice", "password123").is_none());
}

#[test]
fn list_users_returns_all() {
    let store = make_store_with_user();
    assert_eq!(store.list_users().len(), 1);
}

#[test]
fn default_store_is_empty() {
    let store = UserStore::default();
    assert!(store.users.is_empty());
}
