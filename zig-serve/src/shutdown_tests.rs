use std::time::Duration;

#[tokio::test]
async fn shutdown_signal_is_pending_without_signal() {
    // The future should not resolve immediately (no signal sent).
    let result = tokio::time::timeout(Duration::from_millis(50), super::shutdown_signal()).await;
    assert!(
        result.is_err(),
        "shutdown_signal should not resolve without a signal"
    );
}
