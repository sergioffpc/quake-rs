use crate::bindings::Bindings;
use std::cell::RefCell;
use std::rc::Rc;

pub fn bind(bindings: Rc<RefCell<Bindings>>) -> quake_console::command::Command {
    Box::new(move |_, args| {
        let key = args[0];
        if args.len() > 1 {
            let s = args[1..].join(" ");
            let command_text = s
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&s)
                .replace(";", "\n");
            bindings.borrow_mut().bind(key, &command_text);
        } else {
            bindings.borrow_mut().unbind(key);
        }

        quake_console::ControlFlow::Poll
    })
}

pub fn unbind(bindings: Rc<RefCell<Bindings>>) -> quake_console::command::Command {
    Box::new(move |_, args| {
        bindings.borrow_mut().unbind(args[0]);

        quake_console::ControlFlow::Poll
    })
}

pub fn unbindall(bindings: Rc<RefCell<Bindings>>) -> quake_console::command::Command {
    Box::new(move |_, _| {
        bindings.borrow_mut().clear();

        quake_console::ControlFlow::Poll
    })
}
