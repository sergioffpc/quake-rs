use crate::Resources;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct ResourcesBuiltins {
    inner: Arc<RwLock<Resources>>,
}

impl ResourcesBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["cat", "flush", "ls"];

    pub fn new(resources: Arc<RwLock<Resources>>) -> Self {
        Self { inner: resources }
    }

    pub fn builtin_cat(&mut self, args: &[&str]) -> anyhow::Result<()> {
        let resources = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on resources"))?;
        let contents = resources.by_name::<String>(args[0])?;
        println!("{}", contents);
        Ok(())
    }

    pub fn builtin_flush(&mut self) -> anyhow::Result<()> {
        let mut resources = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to acquire write lock on resources"))?;
        resources.flush();
        Ok(())
    }

    pub fn builtin_ls(&mut self) -> anyhow::Result<()> {
        let resources = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on resources"))?;
        resources.file_names().for_each(|name| println!("{}", name));
        Ok(())
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ResourcesBuiltins {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<()> {
        match command[0] {
            "cat" => self.builtin_cat(&command[1..]),
            "flush" => self.builtin_flush(),
            "ls" => self.builtin_ls(),
            _ => Ok(()),
        }
    }
}
