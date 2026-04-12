use std::path::PathBuf;

use crate::config::ServeConfig;

/// Directory for auto-generated TLS files: `~/.zig/tls/`.
fn tls_dir() -> PathBuf {
    zig_core::paths::global_base_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tls")
}

/// Generate a self-signed TLS certificate and save it to `~/.zig/tls/`.
///
/// If certificates already exist on disk, returns those paths without
/// regenerating. Returns `(cert_path, key_path)`.
pub fn ensure_self_signed_cert(
    host: &str,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let dir = tls_dir();
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");

    if cert_path.exists() && key_path.exists() {
        return Ok((
            cert_path.to_string_lossy().into_owned(),
            key_path.to_string_lossy().into_owned(),
        ));
    }

    std::fs::create_dir_all(&dir)?;

    let mut params = rcgen::CertificateParams::new(vec!["localhost".to_string()])?;
    params
        .subject_alt_names
        .push(rcgen::SanType::DnsName("localhost".try_into()?));
    if host != "localhost" && host != "0.0.0.0" && host != "127.0.0.1" {
        if let Ok(dns) = host.try_into() {
            params.subject_alt_names.push(rcgen::SanType::DnsName(dns));
        }
    }
    params
        .subject_alt_names
        .push(rcgen::SanType::IpAddress(std::net::IpAddr::V4(
            std::net::Ipv4Addr::new(127, 0, 0, 1),
        )));
    params
        .subject_alt_names
        .push(rcgen::SanType::IpAddress(std::net::IpAddr::V4(
            std::net::Ipv4Addr::new(0, 0, 0, 0),
        )));

    let key_pair = rcgen::KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    std::fs::write(&cert_path, cert.pem())?;
    std::fs::write(&key_path, key_pair.serialize_pem())?;

    Ok((
        cert_path.to_string_lossy().into_owned(),
        key_path.to_string_lossy().into_owned(),
    ))
}

/// Resolve TLS parameters from the serve config.
///
/// Returns `Some((cert_path, key_path))` when TLS is enabled, `None` for plain HTTP.
pub fn resolve_tls(
    config: &ServeConfig,
) -> Result<Option<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    if let (Some(cert), Some(key)) = (&config.tls_cert, &config.tls_key) {
        return Ok(Some((cert.clone(), key.clone())));
    }

    if config.tls {
        let (cert, key) = ensure_self_signed_cert(&config.host)?;
        return Ok(Some((cert, key)));
    }

    Ok(None)
}

#[cfg(test)]
#[path = "tls_tests.rs"]
mod tests;
