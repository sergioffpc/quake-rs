use std::cell::RefCell;
use std::rc::Rc;

pub fn map(resources: Rc<RefCell<quake_resources::Resources>>) -> quake_console::command::Command {
    Box::new(move |_, args| {
        if let Ok(bsp) = resources
            .borrow()
            .by_name::<quake_resources::bsp::Bsp>(args[0])
        {
            dbg!(bsp);
        }
        quake_console::ControlFlow::Poll
    })
}

pub fn version() -> quake_console::command::Command {
    Box::new(move |ctx, _| {
        writeln!(
            ctx.writer,
            "Quake Server Version: {}",
            env!("CARGO_PKG_VERSION")
        )
        .unwrap();

        quake_console::ControlFlow::Poll
    })
}
