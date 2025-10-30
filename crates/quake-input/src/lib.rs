mod bindings;
mod mapping;

use crate::bindings::KeyBindings;
use crate::mapping::KeyMapping;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::trace;

#[derive(Debug, Default)]
pub struct Input {
    key_bindings: Rc<RefCell<KeyBindings>>,
    key_mapping: KeyMapping,
}

impl Input {
    pub fn on_key(&self, key: &str) -> Option<String> {
        trace!("Key pressed: {}", key);

        let key = self.key_mapping.get(key);
        self.key_bindings.borrow().get(&key)
    }

    pub fn register_builtin_commands(&mut self, console: &mut quake_console::console::Console) {
        self.register_bind_command(console);
        self.register_unbind_command(console);
        self.register_unbindall_command(console);
    }

    fn register_bind_command(&mut self, console: &mut quake_console::console::Console) {
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

    fn register_unbind_command(&mut self, console: &mut quake_console::console::Console) {
        let key_bindings = self.key_bindings.clone();
        console.register_command("unbind", move |_, args| {
            let key = args[0];
            key_bindings.borrow_mut().unbind_key(key);
        });
    }

    fn register_unbindall_command(&mut self, console: &mut quake_console::console::Console) {
        let key_bindings = self.key_bindings.clone();
        console.register_command("unbindall", move |_, _| {
            key_bindings.borrow_mut().clear();
        });
    }
}
