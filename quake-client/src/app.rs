use clap::Parser;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

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

pub fn run_app() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let app = App::new(args.base_path)?;
    quake_window::run_event_loop(app, args.width, args.height)
}

pub struct App {
    resources: Arc<RwLock<quake_resources::Resources>>,
    console: Arc<Mutex<quake_console::Console>>,

    audio_manager: Option<Arc<Mutex<quake_audio::AudioManager>>>,
    client_manager: Option<Arc<Mutex<quake_network::client::ClientManager>>>,
    input_manager: Option<Arc<Mutex<quake_input::InputManager>>>,
    render_manager: Option<quake_render::RenderManager>,
}

impl App {
    pub fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let resources = Arc::new(RwLock::new(quake_resources::Resources::new(path)?));
        let console = Arc::new(Mutex::new(quake_console::Console::new(resources.clone())));

        Ok(Self {
            resources,
            console,

            audio_manager: None,
            client_manager: None,
            input_manager: None,
            render_manager: None,
        })
    }
}

impl quake_window::WindowHandler for App {}

impl quake_window::WindowLifecycleHandler for App {
    fn on_created(&mut self, window: quake_window::window::WindowHandle) {
        let resources_builtins =
            quake_resources::builtins::ResourcesBuiltins::new(self.resources.clone());
        self.console.lock().unwrap().register_commands_handler(
            quake_resources::builtins::ResourcesBuiltins::BUILTIN_COMMANDS,
            resources_builtins.clone(),
        );

        let console_builtins = quake_console::builtins::ConsoleBuiltins::new(self.console.clone());
        self.console.lock().unwrap().register_commands_handler(
            quake_console::builtins::ConsoleBuiltins::BUILTIN_COMMANDS,
            console_builtins.clone(),
        );

        let audio_manager = Arc::new(Mutex::new(
            quake_audio::AudioManager::new(self.resources.clone()).unwrap(),
        ));
        let audio_manager_builtins =
            quake_audio::builtins::AudioBuiltins::new(audio_manager.clone());
        self.console.lock().unwrap().register_commands_handler(
            quake_audio::builtins::AudioBuiltins::BUILTIN_COMMANDS,
            audio_manager_builtins.clone(),
        );
        self.audio_manager = Some(audio_manager.clone());

        let client_manager = Arc::new(Mutex::new(
            pollster::block_on(quake_network::client::ClientManager::new()).unwrap(),
        ));
        let client_manager_builtins =
            quake_network::builtins::ClientBuiltins::new(client_manager.clone());
        self.console.lock().unwrap().register_commands_handler(
            quake_network::builtins::ClientBuiltins::BUILTIN_COMMANDS,
            client_manager_builtins.clone(),
        );
        self.client_manager = Some(client_manager.clone());

        let input_manager = Arc::new(Mutex::new(quake_input::InputManager::default()));
        let input_manager_builtins =
            quake_input::builtins::InputBuiltins::new(input_manager.clone());
        self.console.lock().unwrap().register_commands_handler(
            quake_input::builtins::InputBuiltins::BUILTIN_COMMANDS,
            input_manager_builtins.clone(),
        );
        self.input_manager = Some(input_manager.clone());

        self.render_manager = Some(
            quake_render::RenderManager::new(window.clone(), window.width(), window.width())
                .unwrap(),
        );

        self.console.lock().unwrap().append_text("exec quake.rc");
    }

    fn on_destroyed(&self, window: quake_window::window::WindowHandle) {}
}

impl quake_window::WindowEventHandler for App {
    fn on_key_pressed(&mut self, key: &str) {
        let input_manager = self.input_manager.as_ref().unwrap().lock().unwrap();
        if let Some(command) = input_manager.on_key_pressed(key) {
            self.console.lock().unwrap().append_text(&command);
        }
    }

    fn on_key_released(&mut self, key: &str) {}

    fn on_update_frame(&mut self, delta_time: f64) {
        self.console.lock().unwrap().execute().unwrap()
    }

    fn on_render_frame(&self) {}

    fn on_present_frame(&self) {}
}
