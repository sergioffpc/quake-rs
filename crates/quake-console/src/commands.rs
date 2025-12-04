use crate::ConsoleManager;
use std::fmt::Write;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConsoleCommands {
    console_manager: Arc<ConsoleManager>,
    resources_manager: Arc<quake_resources::ResourcesManager>,
}

impl ConsoleCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] =
        &["alias", "echo", "exec", "quit", "wait", "version"];

    pub fn new(
        console_manager: Arc<ConsoleManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
    ) -> Self {
        Self {
            console_manager,
            resources_manager,
        }
    }

    async fn alias(&mut self, args: &[&str]) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        let alias = args[0];
        if args.len() > 1 {
            let s = args[1..].join(" ");
            let command_text = s
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&s)
                .replace(";", "\n");
            self.console_manager
                .command_aliases
                .write()
                .await
                .register_alias(alias, &command_text);
        } else {
            self.console_manager
                .command_aliases
                .write()
                .await
                .unregister_alias(alias);
        }
        Ok((&[], quake_traits::ControlFlow::Poll))
    }

    fn echo(&mut self, args: &[&str]) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        let mut buffer = String::new();
        writeln!(&mut buffer, "{}", args.join(" "))?;
        Ok((
            Box::leak(buffer.into_bytes().into_boxed_slice()),
            quake_traits::ControlFlow::Poll,
        ))
    }

    async fn exec(&mut self, args: &[&str]) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        if let Ok(text) = self.resources_manager.by_name::<String>(args[0]).await {
            self.console_manager
                .command_buffer
                .lock()
                .await
                .push_front(&text);
        }
        Ok((&[], quake_traits::ControlFlow::Poll))
    }

    fn quit(&mut self) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        std::process::exit(0);
    }

    fn wait(&mut self) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        Ok((&[], quake_traits::ControlFlow::Wait))
    }

    fn version(&mut self) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        let mut buffer = String::new();
        writeln!(&mut buffer, "Quake Version: {}", env!("CARGO_PKG_VERSION"))?;
        Ok((
            Box::leak(buffer.into_bytes().into_boxed_slice()),
            quake_traits::ControlFlow::Poll,
        ))
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ConsoleCommands {
    async fn handle_command(
        &mut self,
        command: &[&str],
    ) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        match command[0] {
            "alias" => self.alias(&command[1..]).await,
            "echo" => self.echo(&command[1..]),
            "exec" => self.exec(&command[1..]).await,
            "quit" => self.quit(),
            "wait" => self.wait(),
            "version" => self.version(),
            _ => Ok((&[], quake_traits::ControlFlow::Poll)),
        }
    }
}
