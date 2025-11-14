use crate::InputManager;
use quake_traits::ControlFlow;
use std::sync::Arc;

#[derive(Clone)]
pub struct InputBuiltins {
    inner: Arc<InputManager>,
}

impl InputBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["bind", "unbind", "unbindall"];

    pub fn new(inner: Arc<InputManager>) -> Self {
        Self { inner }
    }

    fn builtin_bind(&self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        let bind = args[0];
        if args.len() > 1 {
            let s = args[1..].join(" ");
            let command_text = s
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&s)
                .replace(";", "\n");
            self.inner.bindings.bind(bind, &command_text)?;
        } else {
            self.inner.bindings.unbind(bind)?;
        }
        Ok(ControlFlow::Poll)
    }

    fn builtin_unbind(&self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        self.inner.bindings.unbind(args[0])?;
        Ok(ControlFlow::Poll)
    }

    fn builtin_unbindall(&mut self) -> anyhow::Result<ControlFlow> {
        self.inner.bindings.clear()?;
        Ok(ControlFlow::Poll)
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for InputBuiltins {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "bind" => self.builtin_bind(&command[1..]),
            "unbind" => self.builtin_unbind(&command[1..]),
            "unbindall" => self.builtin_unbindall(),
            _ => Ok(ControlFlow::Poll),
        }
    }
}
