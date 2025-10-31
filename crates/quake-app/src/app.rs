use std::cell::RefCell;
use std::rc::Rc;
use tracing::log::trace;

pub trait AppHandler {
    fn on_created(&mut self);
}

pub fn run_app() -> anyhow::Result<()> {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let mut app = App::new("resources/")?;
    app.on_created();

    quake_window::run_app(app)
}

struct App {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,

    input: quake_input::Input,
    resources: Rc<RefCell<quake_resource::Resources>>,
    renderer: Option<quake_renderer::Renderer>,
    console: quake_console::console::Console,
}

impl App {
    fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            flags: wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))?;

        let input = quake_input::Input::default();
        let resources = Rc::new(RefCell::new(quake_resource::Resources::new(path)?));
        let console = quake_console::console::Console::new(resources.clone());

        Ok(Self {
            instance,
            adapter,

            input,
            resources,
            renderer: None,
            console,
        })
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

impl AppHandler for App {
    fn on_created(&mut self) {
        self.register_builtin_commands();
        self.console.append_script("exec quake.rc");
    }
}

impl quake_window::WindowHandler for App {}

impl quake_window::WindowLifecycleHandler for App {
    fn on_created(&mut self, window: quake_window::WindowTarget) {
        let renderer = quake_renderer::Renderer::new(
            &self.instance,
            &self.adapter,
            window.clone(),
            window.width(),
            window.height(),
        )
        .unwrap();
        self.renderer = Some(renderer);
    }
}

impl quake_window::WindowEventHandler for App {
    fn on_key_pressed(&mut self, key: &str) {
        if let Some(script) = self.input.on_key(&key) {
            self.console.append_script(&script);
        }
    }

    fn on_update_frame(&mut self, delta_time: f64) {
        trace!("Frame update with delta time: {}s", delta_time);

        self.console.execute();
    }

    fn on_render_frame(&mut self) {}

    fn on_present_frame(&mut self) {
        self.renderer.as_mut().unwrap().present().unwrap();
    }
}
