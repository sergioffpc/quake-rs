use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

mod app;
mod stream;
mod v3;

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Quake Server")]
struct Args {
    #[arg(long, default_value = "resources/", help = "Base path for resources")]
    base_path: PathBuf,

    #[arg(
        long,
        default_value = "certs/cert.pem",
        help = "Path to server certificate"
    )]
    cert_path: PathBuf,

    #[arg(
        long,
        default_value = "certs/key.pem",
        help = "Path to server private key"
    )]
    key_path: PathBuf,

    #[arg(long, default_value_t = false, help = "Run as a daemon")]
    daemon: bool,

    #[arg(
        long,
        default_value = "127.0.0.1:26000",
        help = "Listen address for server"
    )]
    listen: String,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    stuffcmds: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    app::run_app(Args::parse())
}
