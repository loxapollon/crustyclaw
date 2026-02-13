//! Tracing initialisation helpers for tests.
//!
//! Call [`init_test_tracing`] at the top of any test that emits tracing events
//! and wants them captured by the test harness.
//!
//! The subscriber is initialised at most once per process (idempotent), so it
//! is safe to call from every test function.

use tracing_subscriber::EnvFilter;

/// Initialise a tracing subscriber that writes to the test-harness writer
/// and respects the `RUST_LOG` environment variable.
///
/// Safe to call multiple times — subsequent calls are silently ignored.
///
/// # Example
///
/// ```ignore
/// #[tokio::test]
/// async fn my_test() {
///     crustyclaw_test_utils::tracing_setup::init_test_tracing();
///     tracing::info!("visible when RUST_LOG=info");
/// }
/// ```
pub fn init_test_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_test_writer()
        .try_init();
}
