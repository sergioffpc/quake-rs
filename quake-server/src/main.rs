use clap::Parser;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Quake server")]
struct Args {
    #[arg(long, default_value = "resources/")]
    base_path: PathBuf,
}

fn main() {
    let args = Args::parse();

    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let resources = Rc::new(quake_resources::Resources::new(&args.base_path).unwrap());
    let mut console = quake_console::Console::new(resources.clone());
    console.repl().unwrap();
}
