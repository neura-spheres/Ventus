use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("neura_browser=debug,wry=warn,tao=warn"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).compact())
        .with(filter)
        .init();
}
