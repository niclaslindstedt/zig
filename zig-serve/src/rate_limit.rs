use std::sync::Arc;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::DefaultClock;
use governor::state::keyed::DashMapStateStore;
use governor::{Quota, RateLimiter};

/// Keyed rate limiter using remote IP addresses as keys.
pub type KeyedRateLimiter = RateLimiter<String, DashMapStateStore<String>, DefaultClock>;

/// Build a keyed rate limiter that allows `per_second` requests per second
/// per unique client key, with a burst equal to the per-second rate.
pub fn build_rate_limiter(per_second: u64) -> Arc<KeyedRateLimiter> {
    let quota = Quota::per_second(
        std::num::NonZeroU32::new(per_second as u32).unwrap_or(std::num::NonZeroU32::MIN),
    )
    .allow_burst(std::num::NonZeroU32::new(per_second as u32).unwrap_or(std::num::NonZeroU32::MIN));

    Arc::new(RateLimiter::dashmap(quota))
}

/// Rate-limiting middleware. Checks the client key (from `x-forwarded-for`
/// header, falling back to a default key) against the rate limiter.
///
/// Returns `429 Too Many Requests` when the limit is exceeded.
pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
    limiter: Arc<KeyedRateLimiter>,
) -> Response {
    let key = extract_client_key(&request);

    match limiter.check_key(&key) {
        Ok(_) => next.run(request).await,
        Err(_) => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response(),
    }
}

/// Extract a client key from the request for rate limiting purposes.
fn extract_client_key(request: &Request) -> String {
    // Try x-forwarded-for first (for reverse proxies).
    if let Some(forwarded) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(first_ip) = forwarded.split(',').next() {
            return first_ip.trim().to_string();
        }
    }

    // Fall back to a shared default key (rate limits all non-proxied clients together).
    "default".to_string()
}

#[cfg(test)]
#[path = "rate_limit_tests.rs"]
mod tests;
