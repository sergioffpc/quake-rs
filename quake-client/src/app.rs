use crate::Args;
use std::sync::{Arc, Mutex, RwLock};

pub fn run_app(args: Args) -> anyhow::Result<()> {
    let app = App::new(args.base_path)?;
    quake_window::run_event_loop(app, args.width, args.height)
}

struct App {
    runtime: tokio::runtime::Runtime,

    resources: Arc<RwLock<quake_resources::Resources>>,
    console: Arc<Mutex<quake_console::Console>>,

    audio_manager: Option<Arc<Mutex<quake_audio::AudioManager>>>,
    client_manager: Option<Arc<Mutex<quake_network::client::ClientManager>>>,
    input_manager: Option<Arc<Mutex<quake_input::InputManager>>>,
    render_manager: Option<quake_render::RenderManager>,
}

impl App {
    fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let resources = Arc::new(RwLock::new(quake_resources::Resources::new(path)?));
        let console = Arc::new(Mutex::new(quake_console::Console::new(resources.clone())));

        Ok(Self {
            runtime,
            resources,
            console,

            audio_manager: None,
            client_manager: None,
            input_manager: None,
            render_manager: None,
        })
    }

    fn register_builtin_commands<H>(&mut self, commands: &[&str], handler: H)
    where
        H: quake_traits::CommandHandler + Clone + 'static,
    {
        self.console
            .lock()
            .unwrap()
            .register_commands_handler(commands, handler);
    }
}

impl quake_window::WindowHandler for App {}

impl quake_window::WindowLifecycleHandler for App {
    fn on_created(&mut self, window: quake_window::window::WindowHandle) -> anyhow::Result<()> {
        let resources_builtins =
            quake_resources::builtins::ResourcesBuiltins::new(self.resources.clone());
        self.register_builtin_commands(
            quake_resources::builtins::ResourcesBuiltins::BUILTIN_COMMANDS,
            resources_builtins,
        );

        let console_builtins = quake_console::builtins::ConsoleBuiltins::new(self.console.clone());
        self.register_builtin_commands(
            quake_console::builtins::ConsoleBuiltins::BUILTIN_COMMANDS,
            console_builtins,
        );

        let audio_manager = Arc::new(Mutex::new(quake_audio::AudioManager::new(
            self.resources.clone(),
        )?));
        let audio_manager_builtins =
            quake_audio::builtins::AudioBuiltins::new(audio_manager.clone());
        self.register_builtin_commands(
            quake_audio::builtins::AudioBuiltins::BUILTIN_COMMANDS,
            audio_manager_builtins,
        );
        self.audio_manager = Some(audio_manager);

        let client_manager = Arc::new(Mutex::new(
            self.runtime
                .block_on(quake_network::client::ClientManager::new())?,
        ));
        let client_manager_builtins =
            quake_network::builtins::ClientBuiltins::new(client_manager.clone());
        self.register_builtin_commands(
            quake_network::builtins::ClientBuiltins::BUILTIN_COMMANDS,
            client_manager_builtins,
        );
        self.client_manager = Some(client_manager);

        let input_manager = Arc::new(Mutex::new(quake_input::InputManager::default()));
        let input_manager_builtins =
            quake_input::builtins::InputBuiltins::new(input_manager.clone());
        self.register_builtin_commands(
            quake_input::builtins::InputBuiltins::BUILTIN_COMMANDS,
            input_manager_builtins,
        );
        self.input_manager = Some(input_manager);

        let render_manager = self.runtime.block_on(quake_render::RenderManager::new(
            window.clone(),
            window.width(),
            window.width(),
        ))?;
        self.render_manager = Some(render_manager);

        self.console.lock().unwrap().append_text("exec quake.rc");

        Ok(())
    }

    fn on_destroyed(&self, window: quake_window::window::WindowHandle) -> anyhow::Result<()> {
        Ok(())
    }
}

impl quake_window::WindowEventHandler for App {
    fn on_key_pressed(&mut self, key: &str) -> anyhow::Result<()> {
        let input_manager = self.input_manager.as_ref().unwrap().lock().unwrap();
        if let Some(command) = input_manager.on_key_pressed(key) {
            let mut console = self.console.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
            console.append_text(&command);
        }
        Ok(())
    }

    fn on_key_released(&mut self, key: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_update_frame(&mut self, delta_time: f64) -> anyhow::Result<()> {
        let mut console = self.console.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        self.runtime.block_on(console.execute())
    }

    fn on_render_frame(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_present_frame(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
