fn main() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let resources = quake_resources::Resources::new("res/").unwrap();
    resources.file_names().for_each(|name| println!("{}", name));
}
