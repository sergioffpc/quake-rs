use crate::Args;
use crate::v3::protocol::{BadPacketHandler, DisconnectPacketHandler, NopPacketHandler};
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
    console_manager: Arc<quake_console::ConsoleManager>,
    input_manager: Arc<quake_input::InputManager>,
    render_manager: Option<quake_render::RenderManager>,
}

impl App {
    fn new(args: Args) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let console_manager = Arc::new(quake_console::ConsoleManager::default());
        let resources_manager = runtime.block_on(async {
            Arc::new(
                quake_resources::ResourcesManager::new(args.base_path)
                    .await
                    .unwrap(),
            )
        });
        let audio_manager = Arc::new(quake_audio::AudioManager::new()?);
        let client_manager = Arc::new(Mutex::new(
            runtime.block_on(quake_network::client::ClientManager::new(args.ca_path))?,
        ));
        let input_manager = Arc::new(quake_input::InputManager::default());

        Self::register_console_commands(
            &runtime,
            console_manager.clone(),
            resources_manager.clone(),
            args.stuffcmds,
        )?;
        Self::register_resources_commands(
            &runtime,
            console_manager.clone(),
            resources_manager.clone(),
        )?;
        Self::register_audio_commands(
            &runtime,
            console_manager.clone(),
            resources_manager.clone(),
            audio_manager.clone(),
        )?;
        Self::register_network_commands(&runtime, console_manager.clone(), client_manager.clone())?;
        Self::register_input_commands(&runtime, console_manager.clone(), input_manager.clone())?;

        Ok(Self {
            runtime,
            console_manager,
            input_manager,
            render_manager: None,
        })
    }

    fn build_packet_dispatcher(
        console_manager: Arc<quake_console::ConsoleManager>,
    ) -> quake_network::PacketDispatcher {
        let mut packet_dispatcher = quake_network::PacketDispatcher::default();
        packet_dispatcher.register_handler(
            BadPacketHandler::OPCODE,
            Box::new(BadPacketHandler::default()),
        );
        packet_dispatcher.register_handler(
            NopPacketHandler::OPCODE,
            Box::new(NopPacketHandler::default()),
        );
        packet_dispatcher.register_handler(
            DisconnectPacketHandler::OPCODE,
            Box::new(DisconnectPacketHandler::new(console_manager)),
        );

        packet_dispatcher
    }

    fn register_console_commands(
        runtime: &Runtime,
        console_manager: Arc<quake_console::ConsoleManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
        stuffcmds: Vec<String>,
    ) -> anyhow::Result<()> {
        let mut console_manager_commands = quake_console::commands::ConsoleCommands::new(
            console_manager.clone(),
            resources_manager,
        );
        console_manager_commands.extend_stuffcmds(stuffcmds);

        runtime.block_on(console_manager.register_commands_handler(
            quake_console::commands::ConsoleCommands::BUILTIN_COMMANDS,
            console_manager_commands,
        ))
    }

    fn register_resources_commands(
        runtime: &Runtime,
        console_manager: Arc<quake_console::ConsoleManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
    ) -> anyhow::Result<()> {
        let resources_manager_commands =
            quake_resources::commands::ResourcesCommands::new(resources_manager);
        runtime.block_on(console_manager.register_commands_handler(
            quake_resources::commands::ResourcesCommands::BUILTIN_COMMANDS,
            resources_manager_commands,
        ))
    }

    fn register_audio_commands(
        runtime: &Runtime,
        console_manager: Arc<quake_console::ConsoleManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
        audio_manager: Arc<quake_audio::AudioManager>,
    ) -> anyhow::Result<()> {
        let audio_manager_commands =
            quake_audio::commands::AudioCommands::new(audio_manager, resources_manager);
        runtime.block_on(console_manager.register_commands_handler(
            quake_audio::commands::AudioCommands::BUILTIN_COMMANDS,
            audio_manager_commands,
        ))
    }

    fn register_network_commands(
        runtime: &Runtime,
        console_manager: Arc<quake_console::ConsoleManager>,
        client_manager: Arc<Mutex<quake_network::client::ClientManager>>,
    ) -> anyhow::Result<()> {
        let client_manager_commands = quake_network::commands::ClientCommands::new(client_manager);
        runtime.block_on(console_manager.register_commands_handler(
            quake_network::commands::ClientCommands::BUILTIN_COMMANDS,
            client_manager_commands,
        ))
    }

    fn register_input_commands(
        runtime: &Runtime,
        console_manager: Arc<quake_console::ConsoleManager>,
        input_manager: Arc<quake_input::InputManager>,
    ) -> anyhow::Result<()> {
        let input_manager_commands = quake_input::commands::InputCommands::new(input_manager);
        runtime.block_on(console_manager.register_commands_handler(
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
            .block_on(self.console_manager.append_text("exec quake.rc"));

        let console_manager = self.console_manager.clone();
        self.runtime
            .spawn(async move { console_manager.repl().await.unwrap() });

        Ok(())
    }

    fn on_destroyed(&self, _window: quake_window::window::WindowHandle) -> anyhow::Result<()> {
        Ok(())
    }
}

impl quake_window::WindowEventHandler for App {
    fn on_key_pressed(&mut self, key: &str) -> anyhow::Result<()> {
        let input_manager = self.input_manager.clone();
        let console_manager = self.console_manager.clone();
        self.runtime.block_on(async move {
            if let Some(command) = input_manager.on_key_pressed(key).await {
                console_manager.append_text(&command).await;
            }
        });
        Ok(())
    }

    fn on_key_released(&mut self, _key: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_update_frame(&mut self, _delta_time: f64) -> anyhow::Result<()> {
        self.runtime.block_on(self.console_manager.execute())?;

        Ok(())
    }

    fn on_render_frame(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_present_frame(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
