use std::sync::Arc;

use tokio::runtime::Runtime;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    audio::{audio_command_executor_system, Audio},
    console::{console_command_postprocessor_system, console_command_preprocessor_system, Console},
    graphics::{graphics_present_system, Graphics},
    input::{input_command_executor_system, input_handler_system, Input, InputEvent},
    message::{message_command_executor_system, message_handler_system, MessageSource},
    ResourceFiles,
};

#[derive(Default)]
pub struct GameApp {
    inner: Option<InnerApp>,
}

impl GameApp {
    pub fn run_app(&mut self) -> anyhow::Result<()> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self)?;

        Ok(())
    }
}

impl ApplicationHandler for GameApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let rt = Runtime::new().unwrap();
        self.inner = Some(rt.block_on(InnerApp::new(event_loop)).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        self.inner
            .as_mut()
            .unwrap()
            .window_event(event_loop, event)
            .unwrap();
    }
}

struct InnerApp {
    window: Arc<Window>,

    _output_stream: rodio::OutputStream,

    entity_world: legion::World,
    shared_resources: legion::Resources,
    system_schedule: legion::Schedule,
}

impl InnerApp {
    async fn new(event_loop: &ActiveEventLoop) -> anyhow::Result<Self> {
        let window_size = PhysicalSize::new(2048, 1080);
        let window_attributes = WindowAttributes::default()
            .with_decorations(false)
            .with_inner_size(window_size)
            .with_resizable(false);
        let window = Arc::new(event_loop.create_window(window_attributes)?);

        let (_output_stream, output_stream_handle) = rodio::OutputStream::try_default()?;

        let entity_world = legion::World::default();
        let mut shared_resources = legion::Resources::default();
        let system_schedule = legion::Schedule::builder()
            .add_system(message_handler_system())
            .add_system(input_handler_system())
            .flush()
            .add_system(console_command_preprocessor_system())
            .flush()
            .add_system(audio_command_executor_system())
            .add_system(input_command_executor_system())
            .add_system(message_command_executor_system())
            .flush()
            .add_system(console_command_postprocessor_system())
            .flush()
            .add_system(graphics_present_system())
            .build();

        let audio = Audio::new(output_stream_handle)?;
        shared_resources.insert(audio);

        let graphics =
            Graphics::new(Arc::clone(&window), window_size.width, window_size.height).await?;
        shared_resources.insert(graphics);

        let resource_files = ResourceFiles::new("res/")?;
        shared_resources.insert(resource_files);

        let mut console = Console::default();
        console.register_command("cd");
        console.register_command("play");

        console.register_command("exec");
        console.register_command("alias");

        console.register_command("bind");
        console.register_command("unbind");
        console.register_command("unbindall");

        console.register_command("playdemo");
        console.register_command("stopdemo");
        console.register_command("startdemos");

        console.push_command("exec quake.rc");
        shared_resources.insert(console);

        let input = Input::default();
        shared_resources.insert(input);

        let input_event: Option<InputEvent> = None;
        shared_resources.insert(input_event);

        let message_stream: Option<MessageSource> = None;
        shared_resources.insert(message_stream);

        Ok(Self {
            window,

            _output_stream,

            entity_world,
            shared_resources,
            system_schedule,
        })
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
    ) -> anyhow::Result<()> {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => {
                let input_event = InputEvent::KeyboardInput { code, state };
                self.shared_resources.insert(Some(input_event));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let input_event = InputEvent::MouseInput { state, button };
                self.shared_resources.insert(Some(input_event));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let input_event = InputEvent::MouseWheel { delta };
                self.shared_resources.insert(Some(input_event));
            }
            WindowEvent::RedrawRequested => {
                self.system_schedule
                    .execute(&mut self.entity_world, &mut self.shared_resources);

                let input_event: Option<InputEvent> = None;
                self.shared_resources.insert(input_event);
            }
            _ => (),
        }
        self.window.request_redraw();

        Ok(())
    }
}
