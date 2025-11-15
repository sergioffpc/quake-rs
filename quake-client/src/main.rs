use clap::Parser;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tracing::log::error;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

mod app;

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Quake Client")]
struct Args {
    #[arg(long, default_value = "resources/", help = "Base path for resources")]
    base_path: PathBuf,

    #[arg(long, default_value = "2048", help = "Window width")]
    width: u32,

    #[arg(long, default_value = "1080", help = "Window height")]
    height: u32,
}

fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    thread::spawn(|| {
        loop {
            thread::sleep(Duration::from_secs(5));
            let deadlocks = parking_lot::deadlock::check_deadlock();
            if deadlocks.is_empty() {
                continue;
            }

            error!("{} deadlocks detected!", deadlocks.len());
            for (i, threads) in deadlocks.iter().enumerate() {
                error!("Deadlock #{}", i);
                for t in threads {
                    error!("Thread Id {:#?}", t.thread_id());
                    error!("{:#?}", t.backtrace());
                }
            }
        }
    });

    app::run_app(Args::parse()).unwrap();
}
