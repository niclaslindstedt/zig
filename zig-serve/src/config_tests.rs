use super::{FileConfig, ServerSection};

#[test]
fn file_config_default_is_empty() {
    let c = FileConfig::default();
    assert!(c.server.host.is_none());
    assert!(c.server.port.is_none());
    assert!(c.server.token.is_none());
    assert!(c.server.shutdown_timeout.is_none());
    assert!(!c.server.tls);
    assert!(c.server.tls_cert.is_none());
    assert!(c.server.tls_key.is_none());
    assert!(c.server.rate_limit.is_none());
    assert!(!c.server.web);
}

#[test]
fn file_config_parses_full_server_section() {
    let src = r#"
[server]
host = "0.0.0.0"
port = 4000
token = "abc"
shutdown_timeout = 15
tls = true
tls_cert = "/etc/cert.pem"
tls_key = "/etc/key.pem"
rate_limit = 100
"#;
    let c: FileConfig = toml::from_str(src).unwrap();
    assert_eq!(c.server.host.as_deref(), Some("0.0.0.0"));
    assert_eq!(c.server.port, Some(4000));
    assert_eq!(c.server.token.as_deref(), Some("abc"));
    assert_eq!(c.server.shutdown_timeout, Some(15));
    assert!(c.server.tls);
    assert_eq!(c.server.tls_cert.as_deref(), Some("/etc/cert.pem"));
    assert_eq!(c.server.tls_key.as_deref(), Some("/etc/key.pem"));
    assert_eq!(c.server.rate_limit, Some(100));
}

#[test]
fn file_config_parses_partial_server_section() {
    let src = r#"
[server]
host = "127.0.0.1"
port = 3000
"#;
    let c: FileConfig = toml::from_str(src).unwrap();
    assert_eq!(c.server.host.as_deref(), Some("127.0.0.1"));
    assert_eq!(c.server.port, Some(3000));
    assert!(c.server.token.is_none());
    assert!(c.server.shutdown_timeout.is_none());
    assert!(!c.server.tls);
    assert!(c.server.rate_limit.is_none());
}

#[test]
fn file_config_parses_missing_server_section() {
    let c: FileConfig = toml::from_str("").unwrap();
    assert!(c.server.host.is_none());
    assert!(!c.server.tls);
}

#[test]
fn file_config_serializes_to_toml_roundtrip() {
    // Serialize -> parse gives an equivalent value. This exercises both the
    // Serialize and Deserialize impls without touching HOME or the filesystem.
    let original = FileConfig {
        server: ServerSection {
            host: Some("1.2.3.4".into()),
            port: Some(9999),
            token: Some("round-trip".into()),
            shutdown_timeout: Some(7),
            tls: true,
            tls_cert: Some("/tmp/cert.pem".into()),
            tls_key: Some("/tmp/key.pem".into()),
            rate_limit: Some(42),
            web: true,
        },
    };
    let serialized = toml::to_string_pretty(&original).unwrap();
    let parsed: FileConfig = toml::from_str(&serialized).unwrap();

    assert_eq!(parsed.server.host, original.server.host);
    assert_eq!(parsed.server.port, original.server.port);
    assert_eq!(parsed.server.token, original.server.token);
    assert_eq!(
        parsed.server.shutdown_timeout,
        original.server.shutdown_timeout
    );
    assert_eq!(parsed.server.tls, original.server.tls);
    assert_eq!(parsed.server.tls_cert, original.server.tls_cert);
    assert_eq!(parsed.server.tls_key, original.server.tls_key);
    assert_eq!(parsed.server.rate_limit, original.server.rate_limit);
    assert_eq!(parsed.server.web, original.server.web);
}

#[test]
fn file_config_config_path_ends_with_serve_toml() {
    let path = FileConfig::config_path();
    assert!(path.ends_with("serve.toml"), "path = {path:?}");
}
