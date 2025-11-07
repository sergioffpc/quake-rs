use std::rc::Rc;

pub fn map(resources: Rc<quake_resources::Resources>) -> quake_console::command::Command {
    Box::new(move |_, args| {
        if let Ok(bsp) = resources.by_name::<quake_resources::bsp::Bsp>(args[0]) {
            dbg!(bsp);
        }
        quake_console::ControlFlow::Poll
    })
}
