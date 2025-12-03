use crate::Args;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

pub fn run_app(args: Args) -> anyhow::Result<()> {
    let width = args.width;
    let height = args.height;
    let app = App::new(args)?;
    quake_window::run_event_loop(app, width, height)
}

struct App {
    runtime: Runtime,
    console: Arc<quake_console::Console>,

    client_manager: Arc<Mutex<quake_network::client::ClientManager>>,
    input_manager: Arc<quake_input::InputManager>,
    render_manager: Option<quake_render::RenderManager>,
}

impl App {
    fn new(args: Args) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let console = Arc::new(quake_console::Console::default());
        let resources = runtime.block_on(async {
            Arc::new(
                quake_resources::Resources::new(args.base_path)
                    .await
                    .unwrap(),
            )
        });
        let audio_manager = Arc::new(quake_audio::AudioManager::new()?);
        let client_manager = Arc::new(Mutex::new(
            runtime.block_on(quake_network::client::ClientManager::new(args.ca_path))?,
        ));
        let input_manager = Arc::new(quake_input::InputManager::default());

        Self::register_console_commands(&runtime, console.clone(), resources.clone())?;
        Self::register_resources_commands(&runtime, console.clone(), resources.clone())?;
        Self::register_audio_commands(
            &runtime,
            console.clone(),
            resources.clone(),
            audio_manager.clone(),
        )?;
        Self::register_client_commands(&runtime, console.clone(), client_manager.clone())?;
        Self::register_input_commands(&runtime, console.clone(), input_manager.clone())?;

        Ok(Self {
            runtime,
            console,

            client_manager,
            input_manager,
            render_manager: None,
        })
    }

    fn register_console_commands(
        runtime: &Runtime,
        console: Arc<quake_console::Console>,
        resources: Arc<quake_resources::Resources>,
    ) -> anyhow::Result<()> {
        let console_commands =
            quake_console::commands::ConsoleCommands::new(console.clone(), resources.clone());
        runtime.block_on(console.register_commands_handler(
            quake_console::commands::ConsoleCommands::BUILTIN_COMMANDS,
            console_commands,
        ))
    }

    fn register_resources_commands(
        runtime: &Runtime,
        console: Arc<quake_console::Console>,
        resources: Arc<quake_resources::Resources>,
    ) -> anyhow::Result<()> {
        let resources_commands =
            quake_resources::commands::ResourcesCommands::new(resources.clone());
        runtime.block_on(console.register_commands_handler(
            quake_resources::commands::ResourcesCommands::BUILTIN_COMMANDS,
            resources_commands,
        ))
    }

    fn register_audio_commands(
        runtime: &Runtime,
        console: Arc<quake_console::Console>,
        resources: Arc<quake_resources::Resources>,
        audio_manager: Arc<quake_audio::AudioManager>,
    ) -> anyhow::Result<()> {
        let audio_manager_commands =
            quake_audio::commands::AudioCommands::new(audio_manager.clone(), resources.clone());
        runtime.block_on(console.register_commands_handler(
            quake_audio::commands::AudioCommands::BUILTIN_COMMANDS,
            audio_manager_commands,
        ))
    }

    fn register_client_commands(
        runtime: &Runtime,
        console: Arc<quake_console::Console>,
        client_manager: Arc<Mutex<quake_network::client::ClientManager>>,
    ) -> anyhow::Result<()> {
        let client_manager_commands =
            quake_network::commands::ClientCommands::new(client_manager.clone());
        runtime.block_on(console.register_commands_handler(
            quake_network::commands::ClientCommands::BUILTIN_COMMANDS,
            client_manager_commands,
        ))
    }

    fn register_input_commands(
        runtime: &Runtime,
        console: Arc<quake_console::Console>,
        input_manager: Arc<quake_input::InputManager>,
    ) -> anyhow::Result<()> {
        let input_manager_commands =
            quake_input::commands::InputCommands::new(input_manager.clone());
        runtime.block_on(console.register_commands_handler(
            quake_input::commands::InputCommands::BUILTIN_COMMANDS,
            input_manager_commands,
        ))
    }
}

impl quake_window::WindowHandler for App {}

impl quake_window::WindowLifecycleHandler for App {
    fn on_created(&mut self, window: quake_window::window::WindowHandle) -> anyhow::Result<()> {
        let render_manager = self.runtime.block_on(quake_render::RenderManager::new(
            window.clone(),
            window.width(),
            window.width(),
        ))?;
        self.render_manager = Some(render_manager);

        self.runtime
            .block_on(self.console.append_text("exec quake.rc"));

        let console = self.console.clone();
        self.runtime
            .spawn(async move { console.repl().await.unwrap() });

        Ok(())
    }

    fn on_destroyed(&self, window: quake_window::window::WindowHandle) -> anyhow::Result<()> {
        Ok(())
    }
}

impl quake_window::WindowEventHandler for App {
    fn on_key_pressed(&mut self, key: &str) -> anyhow::Result<()> {
        let input_manager = self.input_manager.clone();
        let console = self.console.clone();
        self.runtime.block_on(async move {
            if let Some(command) = input_manager.on_key_pressed(key).await {
                console.append_text(&command).await;
            }
        });
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
