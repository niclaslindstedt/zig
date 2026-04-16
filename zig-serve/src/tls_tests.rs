use crate::config::ServeConfig;
use std::time::Duration;

#[test]
fn ensure_self_signed_cert_creates_files() {
    let dir = tempfile::tempdir().unwrap();
    // SAFETY: test-only; we restore HOME immediately after.
    let original_home = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", dir.path()) };

    let result = super::ensure_self_signed_cert("127.0.0.1");

    if let Some(home) = original_home {
        unsafe { std::env::set_var("HOME", home) };
    }

    let (cert, key) = result.unwrap();
    assert!(
        std::path::Path::new(&cert).exists(),
        "cert file should exist"
    );
    assert!(std::path::Path::new(&key).exists(), "key file should exist");
    assert!(cert.ends_with("cert.pem"));
    assert!(key.ends_with("key.pem"));
}

#[test]
fn resolve_tls_returns_none_when_disabled() {
    let config = ServeConfig {
        host: "127.0.0.1".into(),
        port: 3000,
        token: "test".into(),
        shutdown_timeout: Duration::from_secs(30),
        tls: false,
        tls_cert: None,
        tls_key: None,
        rate_limit: None,
        web: false,
    };
    assert!(super::resolve_tls(&config).unwrap().is_none());
}

#[cfg(unix)]
#[test]
fn write_secret_file_is_mode_0600() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secret.txt");

    // First write creates the file with 0600.
    super::write_secret_file(&path, b"first").unwrap();
    let meta = std::fs::metadata(&path).unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o600,
        "new file should be mode 0600",
    );

    // Loosen perms then rewrite — should re-tighten back to 0600.
    let mut perms = meta.permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(&path, perms).unwrap();
    super::write_secret_file(&path, b"second").unwrap();
    let meta = std::fs::metadata(&path).unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o600,
        "rewritten file should be mode 0600",
    );
}

#[test]
fn resolve_tls_uses_custom_cert_paths() {
    let config = ServeConfig {
        host: "127.0.0.1".into(),
        port: 3000,
        token: "test".into(),
        shutdown_timeout: Duration::from_secs(30),
        tls: false,
        tls_cert: Some("/path/to/cert.pem".into()),
        tls_key: Some("/path/to/key.pem".into()),
        rate_limit: None,
        web: false,
    };
    let result = super::resolve_tls(&config).unwrap().unwrap();
    assert_eq!(result.0, "/path/to/cert.pem");
    assert_eq!(result.1, "/path/to/key.pem");
}
