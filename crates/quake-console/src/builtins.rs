use crate::command::Command;
use crate::ControlFlow;
use std::cell::RefCell;
use std::rc::Rc;

pub fn alias() -> Command {
    Box::new(|ctx, args| {
        let alias = args[0];
        if args.len() > 1 {
            let s = args[1..].join(" ");
            let command_text = s
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&s)
                .replace(";", "\n");
            ctx.aliases.register_alias(alias, &command_text);
        } else {
            ctx.aliases.unregister_alias(alias);
        }
        ControlFlow::Poll
    })
}

pub fn echo() -> Command {
    Box::new(move |ctx, args| {
        writeln!(ctx.writer, "{}", args.join(" ")).unwrap();
        ControlFlow::Poll
    })
}

pub fn exec(resources: Rc<RefCell<quake_resources::Resources>>) -> Command {
    Box::new(move |ctx, args| {
        if let Ok(text) = resources.borrow().by_name::<String>(args[0]) {
            ctx.buffer.push_front(&text);
        }
        ControlFlow::Poll
    })
}

pub fn quit() -> Command {
    Box::new(|_, _| std::process::exit(0))
}

pub fn rlist(resources: Rc<RefCell<quake_resources::Resources>>) -> Command {
    Box::new(move |ctx, _| {
        resources.borrow().file_names().for_each(|name| {
            writeln!(ctx.writer, "{}", name).unwrap();
        });
        ControlFlow::Poll
    })
}

pub fn wait() -> Command {
    Box::new(|_, _| ControlFlow::Wait)
}
