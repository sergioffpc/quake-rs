use crate::WindowHandler;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::PhysicalKey;
use winit::raw_window_handle;

pub struct Window {
    window_handle: Option<WindowHandle>,
    width: u32,
    height: u32,
    event_handler: Box<dyn WindowHandler>,
    last_frame_time: Instant,
}

impl Window {
    pub fn new(event_handler: Box<dyn WindowHandler>, width: u32, height: u32) -> Self {
        Self {
            window_handle: None,
            width,
            height,
            event_handler,
            last_frame_time: Instant::now(),
        }
    }
}

impl ApplicationHandler for Window {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window_handle.is_some() {
            return;
        }

        let window_attributes = winit::window::WindowAttributes::default()
            .with_active(true)
            .with_decorations(false)
            .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height))
            .with_resizable(false)
            .with_visible(true);
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window_handle = Some(WindowHandle {
            handle: window.clone(),
        });

        self.event_handler
            .on_created(self.window_handle.clone().unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let window = self.window_handle.as_ref().unwrap();
        if window_id != window.handle.id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.event_handler
                    .on_destroyed(self.window_handle.clone().unwrap());
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        text,
                        state,
                        ..
                    },
                ..
            } => {
                let key = match text {
                    Some(ref key) => key.as_str(),
                    None => &format!("{:?}", key_code),
                };
                match state {
                    ElementState::Pressed => self.event_handler.on_key_pressed(key),
                    ElementState::Released => self.event_handler.on_key_released(key),
                }
            }
            WindowEvent::RedrawRequested => {
                self.event_handler.on_render_frame();

                window.handle.pre_present_notify();
                self.event_handler.on_present_frame();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.event_handler
            .on_update_frame(self.last_frame_time.elapsed().as_secs_f64());
        self.last_frame_time = Instant::now();
        self.window_handle.as_ref().unwrap().handle.request_redraw();
    }
}

#[derive(Clone, Debug)]
pub struct WindowHandle {
    handle: Arc<winit::window::Window>,
}

impl WindowHandle {
    pub fn width(&self) -> u32 {
        self.handle.inner_size().width
    }

    pub fn height(&self) -> u32 {
        self.handle.inner_size().height
    }
}

impl raw_window_handle::HasWindowHandle for WindowHandle {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        self.handle.window_handle()
    }
}

impl raw_window_handle::HasDisplayHandle for WindowHandle {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        self.handle.display_handle()
    }
}
