use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct KeyBindings {
    key_bindings: HashMap<String, String>,
}

impl KeyBindings {
    pub fn bind_key(&mut self, key: &str, command: &str) {
        self.key_bindings
            .insert(key.to_string(), command.to_string());
    }

    pub fn unbind_key(&mut self, key: &str) {
        self.key_bindings.remove(key);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.key_bindings.get(key).cloned()
    }

    pub fn clear(&mut self) {
        self.key_bindings.clear();
    }
}
