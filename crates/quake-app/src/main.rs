use std::cell::RefCell;
use std::rc::Rc;

struct App {
    resources: Rc<RefCell<quake_resource::Resources>>,
    console: quake_console::Console,
    input: quake_input::Input,
}

impl App {
    fn new<P>(resources_path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let resources = Rc::new(RefCell::new(quake_resource::Resources::new(
            resources_path,
        )?));
        let console = quake_console::Console::new(resources.clone());
        let input = quake_input::Input::default();

        Ok(Self {
            resources,
            console,
            input,
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

fn main() {
    let mut app = App::new("resources/").unwrap();
    app.register_builtin_commands();

    app.console.append_script("exec quake.rc");
    app.console.execute();
}
