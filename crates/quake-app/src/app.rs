use std::cell::RefCell;
use std::rc::Rc;
use tracing::log::{info, trace};

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
    input: quake_input::Input,
    resources: Rc<RefCell<quake_resource::Resources>>,
    renderer: Option<quake_renderer::Renderer>,
    console: quake_console::Console,
}

impl App {
    fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let input = quake_input::Input::default();
        let resources = Rc::new(RefCell::new(quake_resource::Resources::new(path)?));
        let console = quake_console::Console::new(resources.clone());

        Ok(Self {
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
        let renderer =
            quake_renderer::Renderer::new(window.clone(), window.width(), window.height()).unwrap();
        self.renderer = Some(renderer);
    }
}

impl quake_window::WindowEventHandler for App {
    fn on_key_pressed(&mut self, key: &str) {
        if let Some(script) = self.input.on_key(&key) {
            self.console.append_script(&script);
        }
    }

    fn on_redraw_requested(&mut self) {
        self.renderer.as_mut().unwrap().present().unwrap();
    }

    fn on_frame_update(&mut self, delta_time: f64) {
        trace!("Frame update with delta time: {}s", delta_time);

        self.console.execute();
    }
}
