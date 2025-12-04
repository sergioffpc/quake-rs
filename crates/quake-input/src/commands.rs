use crate::InputManager;
use std::sync::Arc;

#[derive(Clone)]
pub struct InputCommands {
    input_manager: Arc<InputManager>,
}

impl InputCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["bind", "unbind", "unbindall"];

    pub fn new(inner: Arc<InputManager>) -> Self {
        Self {
            input_manager: inner,
        }
    }

    async fn bind(&self, args: &[&str]) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        let bind = args[0];
        if args.len() > 1 {
            let s = args[1..].join(" ");
            let command_text = s
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&s)
                .replace(";", "\n");
            self.input_manager.bindings.bind(bind, &command_text).await;
        } else {
            self.input_manager.bindings.unbind(bind).await;
        }
        Ok((&[], quake_traits::ControlFlow::Poll))
    }

    async fn unbind(&self, args: &[&str]) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        self.input_manager.bindings.unbind(args[0]).await;
        Ok((&[], quake_traits::ControlFlow::Poll))
    }

    async fn unbindall(&mut self) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        self.input_manager.bindings.clear().await;
        Ok((&[], quake_traits::ControlFlow::Poll))
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for InputCommands {
    async fn handle_command(
        &mut self,
        command: &[&str],
    ) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        match command[0] {
            "bind" => self.bind(&command[1..]).await,
            "unbind" => self.unbind(&command[1..]).await,
            "unbindall" => self.unbindall().await,
            _ => Ok((&[], quake_traits::ControlFlow::Poll)),
        }
    }
}
