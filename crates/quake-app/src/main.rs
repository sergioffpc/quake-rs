struct App {
    resources: quake_resource::Resources,
    console: quake_console::Console,
}

impl App {
    fn new<P>(resources_path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let resources = quake_resource::Resources::new(resources_path)?;
        let mut console = quake_console::Console::default();
        quake_console::register_builtin_commands(&mut console, &resources);

        Ok(Self { resources, console })
    }
}

fn main() {
    let mut app = App::new("resources/").unwrap();
    app.console.append_script("exec quake.rc");
    app.console.execute(&app.resources);
}
