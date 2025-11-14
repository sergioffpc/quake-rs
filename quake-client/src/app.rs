use crate::Args;
use std::sync::Arc;

pub fn run_app(args: Args) -> anyhow::Result<()> {
    let app = App::new(args.base_path)?;
    quake_window::run_event_loop(app, args.width, args.height)
}

struct App {
    runtime: tokio::runtime::Runtime,

    resources: Arc<quake_resources::Resources>,
    console: Arc<quake_console::Console>,

    audio_manager: Option<Arc<quake_audio::AudioManager>>,
    client_manager: Option<Arc<quake_network::client::ClientManager>>,
    input_manager: Option<Arc<quake_input::InputManager>>,
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
        let resources = Arc::new(quake_resources::Resources::new(path)?);
        let console = Arc::new(quake_console::Console::new(resources.clone()));

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
}

impl quake_window::WindowHandler for App {}

impl quake_window::WindowLifecycleHandler for App {
    fn on_created(&mut self, window: quake_window::window::WindowHandle) -> anyhow::Result<()> {
        let resources_builtins =
            quake_resources::builtins::ResourcesBuiltins::new(self.resources.clone());
        self.console.register_commands_handler(
            quake_resources::builtins::ResourcesBuiltins::BUILTIN_COMMANDS,
            resources_builtins,
        )?;

        let console_builtins = quake_console::builtins::ConsoleBuiltins::new(
            self.console.clone(),
            self.resources.clone(),
        );
        self.console.register_commands_handler(
            quake_console::builtins::ConsoleBuiltins::BUILTIN_COMMANDS,
            console_builtins,
        )?;

        let audio_manager = Arc::new(quake_audio::AudioManager::new()?);
        let audio_manager_builtins = quake_audio::builtins::AudioBuiltins::new(
            audio_manager.clone(),
            self.resources.clone(),
        );
        self.console.register_commands_handler(
            quake_audio::builtins::AudioBuiltins::BUILTIN_COMMANDS,
            audio_manager_builtins,
        )?;
        self.audio_manager = Some(audio_manager);

        let client_manager = Arc::new(
            self.runtime
                .block_on(quake_network::client::ClientManager::new())?,
        );
        let client_manager_builtins =
            quake_network::builtins::ClientBuiltins::new(client_manager.clone());
        self.console.register_commands_handler(
            quake_network::builtins::ClientBuiltins::BUILTIN_COMMANDS,
            client_manager_builtins,
        )?;
        self.client_manager = Some(client_manager);

        let input_manager = Arc::new(quake_input::InputManager::default());
        let input_manager_builtins =
            quake_input::builtins::InputBuiltins::new(input_manager.clone());
        self.console.register_commands_handler(
            quake_input::builtins::InputBuiltins::BUILTIN_COMMANDS,
            input_manager_builtins,
        )?;
        self.input_manager = Some(input_manager);

        let render_manager = self.runtime.block_on(quake_render::RenderManager::new(
            window.clone(),
            window.width(),
            window.width(),
        ))?;
        self.render_manager = Some(render_manager);

        self.console.append_text("exec quake.rc");

        Ok(())
    }

    fn on_destroyed(&self, window: quake_window::window::WindowHandle) -> anyhow::Result<()> {
        Ok(())
    }
}

impl quake_window::WindowEventHandler for App {
    fn on_key_pressed(&mut self, key: &str) -> anyhow::Result<()> {
        if let Some(command) = self.input_manager.as_ref().unwrap().on_key_pressed(key)? {
            self.console.append_text(&command);
        }
        Ok(())
    }

    fn on_key_released(&mut self, key: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_update_frame(&mut self, delta_time: f64) -> anyhow::Result<()> {
        self.runtime.block_on(self.console.execute())
    }

    fn on_render_frame(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_present_frame(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
