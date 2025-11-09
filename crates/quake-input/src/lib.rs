mod bindings;
mod builtins;
mod mapping;

use crate::bindings::KeyBindings;
use crate::mapping::KeyMapping;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::log::trace;

#[derive(Debug)]
pub struct InputSys {
    bindings: Rc<RefCell<KeyBindings>>,
    mapping: KeyMapping,
}

impl InputSys {
    pub fn new(console: &mut quake_console::Console) -> Self {
        let bindings = Rc::new(RefCell::new(KeyBindings::default()));
        console.register_command("bind", builtins::bind(bindings.clone()));
        console.register_command("unbind", builtins::unbind(bindings.clone()));
        console.register_command("unbindall", builtins::unbindall(bindings.clone()));

        Self {
            bindings: Rc::new(RefCell::new(KeyBindings::default())),
            mapping: KeyMapping::default(),
        }
    }

    pub fn on_key_pressed(&self, key: &str) -> Option<String> {
        trace!("Key pressed: {}", key);

        let key = self.mapping.get(key);
        self.bindings.borrow().get(&key)
    }
}
