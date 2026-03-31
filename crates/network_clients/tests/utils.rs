use tracing::subscriber::DefaultGuard;
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_test_tracing() -> DefaultGuard {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("trace,serde=trace,serde_json=trace,reqwest=debug"));

    let subscriber = fmt()
        .with_env_filter(filter)
        .with_ansi_sanitization(true)
        .compact()
        .with_test_writer()
        .with_target(false)
        .with_thread_names(false)
        .with_line_number(false)
        .with_file(false)
        .without_time()
        .finish();

    tracing::subscriber::set_default(subscriber)
}
