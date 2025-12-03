use crate::Resources;
use quake_traits::ControlFlow;
use std::sync::Arc;

#[derive(Clone)]
pub struct ResourcesCommands {
    inner: Arc<Resources>,
}

impl ResourcesCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["cat", "flush", "ls"];

    pub fn new(inner: Arc<Resources>) -> Self {
        Self { inner }
    }

    pub async fn builtin_cat(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        let contents = self.inner.by_name::<String>(args[0]).await?;
        println!("{}", contents);
        Ok(ControlFlow::Poll)
    }

    pub async fn builtin_flush(&self) -> anyhow::Result<ControlFlow> {
        self.inner.flush().await?;
        Ok(ControlFlow::Poll)
    }

    pub fn builtin_ls(&mut self) -> anyhow::Result<ControlFlow> {
        self.inner
            .file_names()
            .for_each(|name| println!("{}", name));
        Ok(ControlFlow::Poll)
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ResourcesCommands {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "cat" => self.builtin_cat(&command[1..]).await,
            "flush" => self.builtin_flush().await,
            "ls" => self.builtin_ls(),
            _ => Ok(ControlFlow::Poll),
        }
    }
}
