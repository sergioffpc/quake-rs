mod bindings;
mod builtins;
mod mapping;

use crate::bindings::Bindings;
use crate::mapping::Mappings;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::log::trace;

#[derive(Debug)]
pub struct InputManager {
    bindings: Rc<RefCell<Bindings>>,
    mappings: Mappings,
}

impl InputManager {
    pub fn new(console: &mut quake_console::Console) -> Self {
        let bindings = Rc::new(RefCell::new(Bindings::default()));
        console.register_command("bind", builtins::bind(bindings.clone()));
        console.register_command("unbind", builtins::unbind(bindings.clone()));
        console.register_command("unbindall", builtins::unbindall(bindings.clone()));

        Self {
            bindings,
            mappings: Mappings::default(),
        }
    }

    pub fn on_key_pressed(&self, key: &str) -> Option<String> {
        trace!("Key pressed: {}", key);

        self.get_binding(key)
    }

    pub fn on_key_released(&self, key: &str) -> Option<String> {
        trace!("Key released: {}", key);

        self.get_binding(key)
    }

    fn get_binding(&self, key: &str) -> Option<String> {
        let key = self.mappings.get(key);
        self.bindings.borrow().get(&key)
    }
}
