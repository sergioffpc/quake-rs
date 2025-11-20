use crate::Console;
use quake_resources::Resources;
use quake_traits::ControlFlow;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConsoleBuiltins {
    inner: Arc<Console>,
    resources: Arc<Resources>,
}

impl ConsoleBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] =
        &["alias", "echo", "exec", "quit", "wait", "version"];

    pub fn new(inner: Arc<Console>, resources: Arc<Resources>) -> Self {
        Self { inner, resources }
    }

    fn builtin_alias(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        let alias = args[0];
        if args.len() > 1 {
            let s = args[1..].join(" ");
            let command_text = s
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&s)
                .replace(";", "\n");
            self.inner
                .command_aliases
                .write()
                .register_alias(alias, &command_text);
        } else {
            self.inner.command_aliases.write().unregister_alias(alias);
        }
        Ok(ControlFlow::Poll)
    }

    fn builtin_echo(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        use std::io::Write;
        writeln!(std::io::stdout(), "{}", args.join(" "))?;
        Ok(ControlFlow::Poll)
    }

    fn builtin_exec(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        if let Ok(text) = self.resources.by_name::<String>(args[0]) {
            self.inner.command_buffer.lock().push_front(&text);
        }
        Ok(ControlFlow::Poll)
    }

    fn builtin_quit(&mut self) -> anyhow::Result<ControlFlow> {
        std::process::exit(0);
    }

    fn builtin_wait(&mut self) -> anyhow::Result<ControlFlow> {
        Ok(ControlFlow::Wait)
    }

    fn builtin_version(&mut self) -> anyhow::Result<ControlFlow> {
        use std::io::Write;
        writeln!(
            std::io::stdout(),
            "Quake Version: {}",
            env!("CARGO_PKG_VERSION")
        )?;

        Ok(ControlFlow::Poll)
    }
}

impl quake_traits::CommandHandler for ConsoleBuiltins {
    fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "alias" => self.builtin_alias(&command[1..]),
            "echo" => self.builtin_echo(&command[1..]),
            "exec" => self.builtin_exec(&command[1..]),
            "quit" => self.builtin_quit(),
            "wait" => self.builtin_wait(),
            "version" => self.builtin_version(),
            _ => Ok(ControlFlow::Poll),
        }
    }
}
