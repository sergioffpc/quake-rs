use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Bindings {
    bindings: HashMap<String, String>,
}

impl Bindings {
    pub fn bind(&mut self, key: &str, command: &str) {
        self.bindings.insert(key.to_string(), command.to_string());
    }

    pub fn unbind(&mut self, key: &str) {
        self.bindings.remove(key);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.bindings.get(key).cloned()
    }

    pub fn clear(&mut self) {
        self.bindings.clear();
    }
}
