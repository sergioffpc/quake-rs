use parking_lot::RwLock;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Bindings {
    bindings: RwLock<HashMap<String, String>>,
}

impl Bindings {
    pub fn bind(&self, key: &str, command: &str) -> anyhow::Result<()> {
        self.bindings
            .write()
            .insert(key.to_string(), command.to_string());
        Ok(())
    }

    pub fn unbind(&self, key: &str) -> anyhow::Result<()> {
        self.bindings.write().remove(key);
        Ok(())
    }

    pub fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        Ok(self.bindings.read().get(key).cloned())
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        self.bindings.write().clear();
        Ok(())
    }
}
