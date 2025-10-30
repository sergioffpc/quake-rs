use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowAttributes, WindowId};

pub trait WindowHandler: WindowLifecycleHandler + WindowEventHandler {}

pub trait WindowLifecycleHandler {
    fn on_created(&mut self, target: WindowTarget);
}

#[derive(Clone)]
pub struct WindowTarget {
    window: Arc<Window>,
}

impl WindowTarget {
    pub fn new(window: Arc<Window>) -> Self {
        Self { window }
    }

    pub fn width(&self) -> u32 {
        self.window.inner_size().width
    }

    pub fn height(&self) -> u32 {
        self.window.inner_size().height
    }
}

impl winit::raw_window_handle::HasWindowHandle for WindowTarget {
    fn window_handle(
        &self,
    ) -> Result<winit::raw_window_handle::WindowHandle<'_>, winit::raw_window_handle::HandleError>
    {
        self.window.window_handle()
    }
}

impl winit::raw_window_handle::HasDisplayHandle for WindowTarget {
    fn display_handle(
        &self,
    ) -> Result<winit::raw_window_handle::DisplayHandle<'_>, winit::raw_window_handle::HandleError>
    {
        self.window.display_handle()
    }
}

pub trait WindowEventHandler {
    fn on_key_pressed(&mut self, key: &str);

    fn on_redraw_requested(&mut self);
}

pub fn run_app<H>(handler: H) -> anyhow::Result<()>
where
    H: WindowHandler + 'static,
{
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut window_app = WindowApp::new(Box::new(handler));
    event_loop.run_app(&mut window_app).map_err(|e| e.into())
}

struct WindowApp {
    window: Option<Arc<Window>>,
    handler: Box<dyn WindowHandler>,
}

impl WindowApp {
    fn new(handler: Box<dyn WindowHandler>) -> Self {
        Self {
            window: None,
            handler,
        }
    }
}

impl ApplicationHandler for WindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let width = 2048;
        let height = 1080;

        let window_attributes = WindowAttributes::default()
            .with_active(true)
            .with_decorations(false)
            .with_inner_size(winit::dpi::LogicalSize::new(width, height))
            .with_resizable(false)
            .with_visible(true);
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());

        self.handler.on_created(WindowTarget::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if window_id != self.window.as_ref().unwrap().id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        text,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                let key = match text {
                    Some(ref key) => key.as_str(),
                    None => &format!("{:?}", key_code),
                };
                self.handler.on_key_pressed(&key);
            }
            WindowEvent::RedrawRequested => {
                self.handler.on_redraw_requested();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}
