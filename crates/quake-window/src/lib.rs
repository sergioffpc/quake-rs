use crate::window::{Window, WindowHandle};
use winit::event_loop::{ControlFlow, EventLoop};

pub mod window;

pub trait WindowHandler: WindowLifecycleHandler + WindowEventHandler {}

pub trait WindowLifecycleHandler {
    fn on_created(&mut self, handle: WindowHandle);

    fn on_destroyed(&self, handle: WindowHandle);
}

pub trait WindowEventHandler {
    fn on_key_pressed(&mut self, key: &str);

    fn on_key_released(&mut self, key: &str);

    fn on_update_frame(&mut self, delta_time: f64);

    fn on_render_frame(&self);

    fn on_present_frame(&self);
}

pub fn run_event_loop<H>(event_handler: H, width: u32, height: u32) -> anyhow::Result<()>
where
    H: WindowHandler + 'static,
{
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut window = Window::new(Box::new(event_handler), width, height);
    event_loop.run_app(&mut window).map_err(|e| e.into())
}
