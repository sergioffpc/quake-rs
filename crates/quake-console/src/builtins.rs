use crate::command::ControlFlow;
use crate::Console;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ConsoleBuiltins {
    inner: Arc<Mutex<Console>>,
}

impl ConsoleBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] =
        &["alias", "echo", "exec", "quit", "wait", "version"];

    pub fn new(console: Arc<Mutex<Console>>) -> Self {
        Self { inner: console }
    }

    pub fn builtin_alias(&mut self, args: &[&str]) -> anyhow::Result<()> {
        let mut console = self.lock_console()?;

        let alias = args[0];
        if args.len() > 1 {
            let s = args[1..].join(" ");
            let command_text = s
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&s)
                .replace(";", "\n");
            console.command_aliases.register_alias(alias, &command_text);
        } else {
            console.command_aliases.unregister_alias(alias);
        }
        Ok(())
    }

    pub fn builtin_echo(&mut self, args: &[&str]) -> anyhow::Result<()> {
        use std::io::Write;
        writeln!(std::io::stdout(), "{}", args.join(" "))?;
        Ok(())
    }

    pub fn builtin_exec(&mut self, args: &[&str]) -> anyhow::Result<()> {
        let mut console = self.lock_console()?;
        let text = self.load_resource_text(&console, args[0])?;
        console.command_buffer.push_front(&text);
        Ok(())
    }

    pub fn builtin_quit(&mut self) -> anyhow::Result<()> {
        std::process::exit(0);
    }

    pub fn builtin_wait(&mut self) -> anyhow::Result<()> {
        let mut console = self.lock_console()?;
        console.command_executor.set_control_flow(ControlFlow::Wait);
        Ok(())
    }

    pub fn builtin_version(&mut self) -> anyhow::Result<()> {
        use std::io::Write;
        writeln!(
            std::io::stdout(),
            "Quake Version: {}",
            env!("CARGO_PKG_VERSION")
        )?;

        Ok(())
    }

    fn lock_console(&self) -> anyhow::Result<std::sync::MutexGuard<Console>> {
        self.inner.lock().map_err(|e| anyhow::anyhow!("{}", e))
    }

    fn load_resource_text(&self, console: &Console, name: &str) -> anyhow::Result<String> {
        let resources = console
            .resources
            .read()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        resources.by_name::<String>(name)
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ConsoleBuiltins {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<()> {
        match command[0] {
            "alias" => self.builtin_alias(&command[1..]),
            "echo" => self.builtin_echo(&command[1..]),
            "exec" => self.builtin_exec(&command[1..]),
            "quit" => self.builtin_quit(),
            "wait" => self.builtin_wait(),
            "version" => self.builtin_version(),
            _ => Ok(()),
        }
    }
}
