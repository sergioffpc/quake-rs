use crate::builtins;
use clap::Parser;
use std::path::PathBuf;
use std::rc::Rc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Quake client")]
struct Args {
    #[arg(long, default_value = "resources/")]
    base_path: PathBuf,

    #[arg(long, default_value = "2048")]
    width: u32,

    #[arg(long, default_value = "1080")]
    height: u32,
}

pub fn run_app() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let app = App::new(args.base_path);
    quake_window::run_event_loop(app, args.width, args.height)
}

struct App {
    resources: Rc<quake_resources::Resources>,
    console: quake_console::Console,

    input_manager: quake_input::InputManager,
}

impl App {
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<std::path::Path>,
    {
        let resources = Rc::new(quake_resources::Resources::new(path).unwrap());
        let mut console = quake_console::Console::new(resources.clone());
        console.register_command("connect", builtins::connect());
        console.register_command("reconnect", builtins::reconnect());
        console.register_command("disconnect", builtins::disconnect());
        console.register_command("playdemo", builtins::playdemo(resources.clone()));

        let input_manager = quake_input::InputManager::new(&mut console);

        Self {
            resources,
            console,
            input_manager,
        }
    }
}

impl quake_window::WindowHandler for App {}

impl quake_window::WindowLifecycleHandler for App {
    fn on_created(&mut self, window: &quake_window::window::WindowHandle) {
        self.console.append_text("exec quake.rc");
    }

    fn on_destroyed(&self, window: &quake_window::window::WindowHandle) {}
}

impl quake_window::WindowEventHandler for App {
    fn on_key_pressed(&mut self, key: &str) {
        if let Some(command) = self.input_manager.on_key_pressed(key) {
            self.console.append_text(&command);
        }
    }

    fn on_key_released(&mut self, key: &str) {}

    fn on_update_frame(&mut self, delta_time: f64) {
        self.console.execute()
    }

    fn on_render_frame(&self) {}

    fn on_present_frame(&self) {}
}
