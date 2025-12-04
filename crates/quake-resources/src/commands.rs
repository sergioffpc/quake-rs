use crate::ResourcesManager;
use std::fmt::Write;
use std::sync::Arc;

#[derive(Clone)]
pub struct ResourcesCommands {
    resource_manager: Arc<ResourcesManager>,
}

impl ResourcesCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["cat", "flush", "ls"];

    pub fn new(resource_manager: Arc<ResourcesManager>) -> Self {
        Self { resource_manager }
    }

    async fn cat(&mut self, args: &[&str]) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        let buffer = self.resource_manager.by_name::<Vec<u8>>(args[0]).await?;
        Ok((
            Box::leak(buffer.into_boxed_slice()),
            quake_traits::ControlFlow::Poll,
        ))
    }

    async fn flush(&self) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        self.resource_manager.flush().await?;
        Ok((&[], quake_traits::ControlFlow::Poll))
    }

    fn ls(&mut self) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        let mut buffer = String::new();
        for name in self.resource_manager.file_names() {
            writeln!(&mut buffer, "{}", name)?;
        }
        Ok((
            Box::leak(buffer.into_bytes().into_boxed_slice()),
            quake_traits::ControlFlow::Poll,
        ))
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ResourcesCommands {
    async fn handle_command(
        &mut self,
        command: &[&str],
    ) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        match command[0] {
            "cat" => self.cat(&command[1..]).await,
            "flush" => self.flush().await,
            "ls" => self.ls(),
            _ => Ok((&[], quake_traits::ControlFlow::Poll)),
        }
    }
}
