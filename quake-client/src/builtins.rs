pub fn connect() -> quake_console::command::Command {
    Box::new(move |_, args| quake_console::ControlFlow::Poll)
}

pub fn reconnect() -> quake_console::command::Command {
    Box::new(move |_, _| quake_console::ControlFlow::Poll)
}

pub fn disconnect() -> quake_console::command::Command {
    Box::new(move |_, _| quake_console::ControlFlow::Poll)
}
