use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

mod app;
mod v3;

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Quake Client")]
struct Args {
    #[arg(long, default_value = "resources/", help = "Base path for resources")]
    base_path: PathBuf,

    #[arg(long, default_value = "certs/ca.pem", help = "Path to CA certificate")]
    ca_path: PathBuf,

    #[arg(long, default_value = "2048", help = "Window width")]
    width: u32,

    #[arg(long, default_value = "1080", help = "Window height")]
    height: u32,

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
