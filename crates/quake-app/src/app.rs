use std::cell::RefCell;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowAttributes, WindowId};

pub fn run_app() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).map_err(|e| e.into())
}

#[derive(Default)]
struct App {
    inner: Option<InnerApp>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_active(true)
            .with_decorations(false)
            .with_inner_size(winit::dpi::LogicalSize::new(2048, 1080))
            .with_resizable(false)
            .with_visible(true);
        let window = event_loop.create_window(window_attributes).unwrap();

        let mut inner = InnerApp::new(window).unwrap();
        inner.register_builtin_commands();
        inner.console.append_script("exec quake.rc");

        self.inner = Some(inner);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(inner) = &mut self.inner {
            inner.window_event(event_loop, window_id, event);
        }
    }
}

struct InnerApp {
    window: Window,
    input: quake_input::Input,
    resources: Rc<RefCell<quake_resource::Resources>>,
    console: quake_console::Console,
}

impl InnerApp {
    fn new(window: Window) -> anyhow::Result<Self> {
        let input = quake_input::Input::default();
        let resources = Rc::new(RefCell::new(quake_resource::Resources::new("resources/")?));
        let console = quake_console::Console::new(resources.clone());

        Ok(Self {
            window,
            input,
            resources,
            console,
        })
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if window_id != self.window.id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        text,
                        ..
                    },
                ..
            } => {
                let key = match text {
                    Some(ref key) => key.as_str(),
                    None => &format!("{:?}", key_code),
                };
                if let Some(script) = self.input.on_key(&key) {
                    self.console.append_script(&script);
                }
            }
            WindowEvent::RedrawRequested => {
                self.console.execute();

                self.window.request_redraw();
            }
            _ => (),
        }
    }

    fn register_builtin_commands(&mut self) {
        self.console.register_builtin_commands();
        self.input.register_builtin_commands(&mut self.console);

        self.register_quit_command();
    }

    fn register_quit_command(&mut self) {
        self.console
            .register_command("quit", move |_, _| std::process::exit(0));
    }
}
