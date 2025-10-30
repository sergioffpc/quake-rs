use std::cell::RefCell;
use std::collections::HashMap;
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

    fn get(&self, key: &str) -> Option<String> {
        self.key_bindings.get(key).cloned()
    }

    fn clear(&mut self) {
        self.key_bindings.clear();
    }
}

#[derive(Debug)]
struct KeyMapping {
    key_mapping: HashMap<String, String>,
}

impl KeyMapping {
    fn map_key(&mut self, key: &str, map: &str) {
        self.key_mapping.insert(key.to_string(), map.to_string());
    }

    fn unmap_key(&mut self, key: &str) {
        self.key_mapping.remove(key);
    }

    fn get(&self, key: &str) -> String {
        self.key_mapping
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    fn clear(&mut self) {
        self.key_mapping.clear();
    }
}

impl Default for KeyMapping {
    fn default() -> Self {
        let mut key_mapping = HashMap::new();
        key_mapping.insert("\t".to_string(), "TAB".to_string());
        key_mapping.insert("\r".to_string(), "ENTER".to_string());
        key_mapping.insert("\u{1b}".to_string(), "ESCAPE".to_string());
        key_mapping.insert(" ".to_string(), "SPACE".to_string());
        key_mapping.insert("\u{8}".to_string(), "BACKSPACE".to_string());
        key_mapping.insert("ArrowUp".to_string(), "UPARROW".to_string());
        key_mapping.insert("ArrowDown".to_string(), "DOWNARROW".to_string());
        key_mapping.insert("ArrowLeft".to_string(), "LEFTARROW".to_string());
        key_mapping.insert("ArrowRight".to_string(), "RIGHTARROW".to_string());

        key_mapping.insert("AltLeft".to_string(), "ALT".to_string());
        key_mapping.insert("AltRight".to_string(), "ALT".to_string());
        key_mapping.insert("ControlLeft".to_string(), "CTRL".to_string());
        key_mapping.insert("ControlRight".to_string(), "CTRL".to_string());
        key_mapping.insert("ShiftLeft".to_string(), "SHIFT".to_string());
        key_mapping.insert("ShiftRight".to_string(), "SHIFT".to_string());

        key_mapping.insert("Insert".to_string(), "INS".to_string());
        key_mapping.insert("Delete".to_string(), "DEL".to_string());
        key_mapping.insert("PageDown".to_string(), "PGDN".to_string());
        key_mapping.insert("PageUp".to_string(), "PGUP".to_string());
        key_mapping.insert("Home".to_string(), "HOME".to_string());
        key_mapping.insert("End".to_string(), "END".to_string());

        key_mapping.insert("ButtonLeft".to_string(), "MOUSE1".to_string());
        key_mapping.insert("ButtonRight".to_string(), "MOUSE2".to_string());
        key_mapping.insert("ButtonMiddle".to_string(), "MOUSE3".to_string());

        key_mapping.insert("Pause".to_string(), "PAUSE".to_string());

        key_mapping.insert("ScrollUp".to_string(), "MWHEELUP".to_string());
        key_mapping.insert("ScrollDown".to_string(), "MWHEELDOWN".to_string());

        Self { key_mapping }
    }
}
