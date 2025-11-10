use clap::Parser;
use std::path::PathBuf;
use std::rc::Rc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

mod builtins;

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Quake Server")]
struct Args {
    #[arg(long, default_value = "resources/")]
    base_path: PathBuf,
}

fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let resources = Rc::new(quake_resources::Resources::new(&args.base_path).unwrap());
    let mut console = quake_console::Console::new(resources.clone());
    console.register_command("version", builtins::version());
    console.register_command("map", builtins::map(resources.clone()));
    console.repl().unwrap();
}
