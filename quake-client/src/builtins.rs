use std::cell::RefCell;
use std::rc::Rc;

pub fn connect() -> quake_console::command::Command {
    Box::new(move |_, args| quake_console::ControlFlow::Poll)
}

pub fn reconnect() -> quake_console::command::Command {
    Box::new(move |_, _| quake_console::ControlFlow::Poll)
}

pub fn disconnect() -> quake_console::command::Command {
    Box::new(move |_, _| quake_console::ControlFlow::Poll)
}

pub fn flush(
    resources: Rc<RefCell<quake_resources::Resources>>,
) -> quake_console::command::Command {
    Box::new(move |_, _| {
        resources.borrow_mut().flush();
        quake_console::ControlFlow::Poll
    })
}

pub fn playdemo(
    resources: Rc<RefCell<quake_resources::Resources>>,
) -> quake_console::command::Command {
    Box::new(move |_, args| {
        let dem = resources
            .borrow()
            .by_name::<quake_resources::dem::Dem>(args[0])
            .unwrap();
        quake_console::ControlFlow::Poll
    })
}

pub fn version() -> quake_console::command::Command {
    Box::new(move |ctx, _| {
        writeln!(
            ctx.writer,
            "Quake Client Version: {}",
            env!("CARGO_PKG_VERSION")
        )
        .unwrap();
        quake_console::ControlFlow::Poll
    })
}
