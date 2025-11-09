use crate::bindings::KeyBindings;
use std::cell::RefCell;
use std::rc::Rc;

pub fn bind(bindings: Rc<RefCell<KeyBindings>>) -> quake_console::command::Command {
    Box::new(move |_, args| {
        let key = args[0];
        if args.len() > 1 {
            let s = args[1..].join(" ");
            let command_text = s
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&s)
                .replace(";", "\n");
            bindings.borrow_mut().bind_key(key, &command_text);
        } else {
            bindings.borrow_mut().unbind_key(key);
        }

        quake_console::ControlFlow::Poll
    })
}

pub fn unbind(bindings: Rc<RefCell<KeyBindings>>) -> quake_console::command::Command {
    Box::new(move |_, args| {
        bindings.borrow_mut().unbind_key(args[0]);

        quake_console::ControlFlow::Poll
    })
}

pub fn unbindall(bindings: Rc<RefCell<KeyBindings>>) -> quake_console::command::Command {
    Box::new(move |_, args| {
        bindings.borrow_mut().clear();

        quake_console::ControlFlow::Poll
    })
}
