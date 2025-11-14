use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Default)]
pub struct Bindings {
    bindings: RwLock<HashMap<String, String>>,
}

impl Bindings {
    pub fn bind(&self, key: &str, command: &str) -> anyhow::Result<()> {
        let mut bindings = self
            .bindings
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        bindings.insert(key.to_string(), command.to_string());
        Ok(())
    }

    pub fn unbind(&self, key: &str) -> anyhow::Result<()> {
        let mut bindings = self
            .bindings
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        bindings.remove(key);
        Ok(())
    }

    pub fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        let bindings = self.bindings.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(bindings.get(key).cloned())
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        let mut bindings = self
            .bindings
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        bindings.clear();
        Ok(())
    }
}
