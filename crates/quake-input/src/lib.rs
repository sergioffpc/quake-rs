use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
pub struct Input {
    key_bindings: Rc<RefCell<KeyBindings>>,
}

impl Input {
    pub fn register_builtin_commands(&mut self, console: &mut quake_console::Console) {
        self.register_bind_command(console);
        self.register_unbind_command(console);
        self.register_unbindall_command(console);
    }

    fn register_bind_command(&mut self, console: &mut quake_console::Console) {
        let key_bindings = self.key_bindings.clone();
        console.register_command("bind", move |_, args| {
            let key = args[0];
            if args.len() > 1 {
                let s = args[1..].join(" ");
                let command_text = s
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(&s)
                    .replace(";", "\n");
                key_bindings.borrow_mut().bind_key(key, &command_text);
            } else {
                key_bindings.borrow_mut().unbind_key(key);
            }
        });
    }

    fn register_unbind_command(&mut self, console: &mut quake_console::Console) {
        let key_bindings = self.key_bindings.clone();
        console.register_command("unbind", move |_, args| {
            let key = args[0];
            key_bindings.borrow_mut().unbind_key(key);
        });
    }

    fn register_unbindall_command(&mut self, console: &mut quake_console::Console) {
        let key_bindings = self.key_bindings.clone();
        console.register_command("unbindall", move |_, _| {
            key_bindings.borrow_mut().clear();
        });
    }
}

#[derive(Debug, Default)]
struct KeyBindings {
    key_bindings: HashMap<String, String>,
}

impl KeyBindings {
    fn bind_key(&mut self, key: &str, command: &str) {
        self.key_bindings
            .insert(key.to_string(), command.to_string());
    }

    fn unbind_key(&mut self, key: &str) {
        self.key_bindings.remove(key);
    }

    fn clear(&mut self) {
        self.key_bindings.clear();
    }
}
