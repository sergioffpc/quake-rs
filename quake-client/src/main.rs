use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_thread_names(true)
                .with_span_events(fmt::format::FmtSpan::CLOSE)
                .with_file(true)
                .with_line_number(true),
        )
        .with(EnvFilter::from_default_env())
        .init();
    app::run().await
}
