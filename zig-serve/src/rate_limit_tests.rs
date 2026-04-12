#[test]
fn build_rate_limiter_accepts_valid_rate() {
    let limiter = super::build_rate_limiter(100);
    // First request should succeed.
    assert!(limiter.check_key(&"test".to_string()).is_ok());
}

#[test]
fn rate_limiter_enforces_limit() {
    // Allow only 1 request per second with burst of 1.
    let limiter = super::build_rate_limiter(1);
    let key = "client-1".to_string();

    // First request succeeds (uses the burst allowance).
    assert!(limiter.check_key(&key).is_ok());

    // Second immediate request should be rate limited.
    assert!(limiter.check_key(&key).is_err());
}

#[test]
fn rate_limiter_keys_are_independent() {
    let limiter = super::build_rate_limiter(1);

    assert!(limiter.check_key(&"client-a".to_string()).is_ok());
    assert!(limiter.check_key(&"client-b".to_string()).is_ok());

    // client-a is now rate limited, but client-b still had its burst.
    assert!(limiter.check_key(&"client-a".to_string()).is_err());
}
