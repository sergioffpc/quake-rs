use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

mod app;

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Quake Server")]
struct Args {
    #[arg(long, default_value = "resources/", help = "Base path for resources")]
    base_path: PathBuf,

    #[arg(long, default_value_t = false, help = "Run as a daemon")]
    daemon: bool,

    #[arg(
        long,
        default_value = "127.0.0.1:26000",
        help = "Listen address for server"
    )]
    listen: String,
}

fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    app::run_app(Args::parse()).unwrap();
}
