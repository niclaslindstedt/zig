use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::DefaultClock;
use governor::state::keyed::DashMapStateStore;
use governor::{Quota, RateLimiter};

/// Keyed rate limiter. The key is the client's real peer IP (or a configured
/// fallback) — **never** a client-supplied header value, so attackers cannot
/// rotate the key per request to bypass the limit or inflate the state store
/// to exhaust memory.
pub type KeyedRateLimiter = RateLimiter<String, DashMapStateStore<String>, DefaultClock>;

/// Maximum number of keys held in the bucket store before we aggressively
/// prune. This is a soft upper bound — `retain_recent` only evicts entries
/// whose bucket has fully refilled, so active clients survive a prune.
const MAX_BUCKETS: usize = 10_000;

pub fn build_rate_limiter(per_second: u64) -> Arc<KeyedRateLimiter> {
    let quota = Quota::per_second(
        std::num::NonZeroU32::new(per_second as u32).unwrap_or(std::num::NonZeroU32::MIN),
    )
    .allow_burst(std::num::NonZeroU32::new(per_second as u32).unwrap_or(std::num::NonZeroU32::MIN));

    Arc::new(RateLimiter::dashmap(quota))
}

pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
    limiter: Arc<KeyedRateLimiter>,
) -> Response {
    let key = extract_client_key(&request);

    // Opportunistic bounded growth: if the key store has grown past the soft
    // cap, prune buckets that have fully refilled. Active clients are kept.
    if limiter.len() > MAX_BUCKETS {
        limiter.retain_recent();
    }

    match limiter.check_key(&key) {
        Ok(_) => next.run(request).await,
        Err(_) => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response(),
    }
}

/// Derive the rate-limit key from the TCP peer address. Client-supplied
/// headers (`x-forwarded-for` and friends) are intentionally ignored: any
/// client could otherwise rotate the value per request to bypass the limiter,
/// or inflate its state map by supplying random values.
///
/// Proxied deployments should run zig-serve bound to localhost with the
/// reverse proxy enforcing rate limits there.
fn extract_client_key(request: &Request) -> String {
    if let Some(ConnectInfo(addr)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return ip_key(addr.ip());
    }
    // Only reachable in tests that build the router without the make-service
    // connect-info layer. Returning a constant key means the test sees
    // deterministic behavior rather than an accidental bypass.
    "unknown".to_string()
}

fn ip_key(ip: IpAddr) -> String {
    ip.to_string()
}

#[cfg(test)]
#[path = "rate_limit_tests.rs"]
mod tests;
