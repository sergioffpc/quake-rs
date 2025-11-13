use crate::InputManager;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct InputBuiltins {
    inner: Arc<Mutex<InputManager>>,
}

impl InputBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["bind", "unbind", "unbindall"];

    pub fn new(manager: Arc<Mutex<InputManager>>) -> Self {
        Self { inner: manager }
    }

    pub fn builtin_bind(&mut self, args: &[&str]) -> anyhow::Result<()> {
        let mut manager = self.lock_manager()?;

        let bind = args[0];
        if args.len() > 1 {
            let s = args[1..].join(" ");
            let command_text = s
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&s)
                .replace(";", "\n");
            manager.bindings.bind(bind, &command_text);
        } else {
            manager.bindings.unbind(bind);
        }
        Ok(())
    }

    pub fn builtin_unbind(&mut self, args: &[&str]) -> anyhow::Result<()> {
        self.lock_manager()?.bindings.unbind(args[0]);
        Ok(())
    }

    pub fn builtin_unbindall(&mut self) -> anyhow::Result<()> {
        self.lock_manager()?.bindings.clear();
        Ok(())
    }

    fn lock_manager(&self) -> anyhow::Result<std::sync::MutexGuard<InputManager>> {
        self.inner.lock().map_err(|e| anyhow::anyhow!("{}", e))
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for InputBuiltins {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<()> {
        match command[0] {
            "bind" => self.builtin_bind(&command[1..]),
            "unbind" => self.builtin_unbind(&command[1..]),
            "unbindall" => self.builtin_unbindall(),
            _ => Ok(()),
        }
    }
}
