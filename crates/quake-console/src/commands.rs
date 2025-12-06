use crate::ConsoleManager;
use std::fmt::Write;
use std::sync::Arc;
use tabled::settings::{Padding, Style};
use tabled::{Table, Tabled};

#[derive(Clone)]
pub struct ConsoleCommands {
    console_manager: Arc<ConsoleManager>,
    resources_manager: Arc<quake_resources::ResourcesManager>,
    stuffcmds: String,
}

impl ConsoleCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &[
        "alias",
        "vars",
        "echo",
        "exec",
        "quit",
        "stuffcmds",
        "wait",
        "version",
    ];

    pub fn new(
        console_manager: Arc<ConsoleManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
    ) -> Self {
        Self {
            console_manager,
            resources_manager,
            stuffcmds: String::default(),
        }
    }

    pub fn extend_stuffcmds(&mut self, stuffcmds: Vec<String>) {
        let mut result = Vec::new();

        let stuffcmds_str = stuffcmds.join(" ");
        let mut iter = stuffcmds_str.split_whitespace().peekable();

        while let Some(token) = iter.next() {
            if let Some(cmd) = token.strip_prefix('+') {
                let mut line = String::new();
                line.push_str(cmd);

                while let Some(&next) = iter.peek() {
                    if next.starts_with('+') {
                        break;
                    }
                    line.push(' ');
                    line.push_str(iter.next().unwrap());
                }

                result.push(line);
            }
        }

        self.stuffcmds = format!("{}\n{}", self.stuffcmds, result.join("\n"));
    }

    async fn alias(&self, args: &[&str]) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        let mut buffer = String::new();

        match args.len() {
            0 => {
                #[derive(Tabled, Clone, Debug)]
                struct AliasEntry {
                    alias: String,
                    command: String,
                }
                let alias_data: Vec<AliasEntry> = self
                    .console_manager
                    .command_aliases
                    .read()
                    .await
                    .iter()
                    .map(|(k, v)| AliasEntry {
                        alias: k.clone(),
                        command: v
                            .clone()
                            .split_whitespace()
                            .collect::<Vec<&str>>()
                            .join("; "),
                    })
                    .collect();

                buffer = if !alias_data.is_empty() {
                    Table::new(alias_data)
                        .with(Style::re_structured_text())
                        .with(Padding::new(1, 1, 0, 0))
                        .to_string()
                } else {
                    "No aliases defined".to_string()
                };
            }
            1 => {
                self.console_manager
                    .command_aliases
                    .write()
                    .await
                    .unregister_alias(args[0]);
            }
            _ => {
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
                    .register_alias(args[0], &command_text);
            }
        }
        Ok((buffer, quake_traits::ControlFlow::Poll))
    }

    fn echo(&self, args: &[&str]) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        let mut buffer = String::new();
        writeln!(&mut buffer, "{}", args.join(" "))?;
        Ok((buffer, quake_traits::ControlFlow::Poll))
    }

    async fn exec(&self, args: &[&str]) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        if let Ok(text) = self.resources_manager.by_name::<String>(args[0]).await {
            self.console_manager
                .command_buffer
                .lock()
                .await
                .push_front(&text);
        }
        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }

    async fn vars(&self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        #[derive(Tabled, Clone)]
        struct VariableEntry {
            variable: String,
            value: String,
        }
        let vars_data: Vec<VariableEntry> = self
            .console_manager
            .command_variables
            .read()
            .await
            .iter()
            .map(|(k, v)| VariableEntry {
                variable: k.clone(),
                value: v.clone(),
            })
            .collect();

        let buffer = if !vars_data.is_empty() {
            Table::new(vars_data)
                .with(Style::re_structured_text())
                .with(Padding::new(1, 1, 0, 0))
                .to_string()
        } else {
            "No variables defined".to_string()
        };
        Ok((buffer, quake_traits::ControlFlow::Poll))
    }

    fn quit(&self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        std::process::exit(0);
    }

    async fn stuffcmds(&mut self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        self.console_manager
            .command_buffer
            .lock()
            .await
            .push_front(&self.stuffcmds);
        Ok((String::new(), quake_traits::ControlFlow::Poll))
    }

    fn wait(&self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        Ok((String::default(), quake_traits::ControlFlow::Wait))
    }

    fn version(&self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        let mut buffer = String::new();
        writeln!(&mut buffer, "Quake Version: {}", env!("CARGO_PKG_VERSION"))?;
        Ok((buffer, quake_traits::ControlFlow::Poll))
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ConsoleCommands {
    async fn handle_command(
        &mut self,
        command: &[&str],
    ) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        match command[0] {
            "alias" => self.alias(&command[1..]).await,
            "vars" => self.vars().await,
            "echo" => self.echo(&command[1..]),
            "exec" => self.exec(&command[1..]).await,
            "quit" => self.quit(),
            "stuffcmds" => self.stuffcmds().await,
            "wait" => self.wait(),
            "version" => self.version(),
            _ => Ok((String::default(), quake_traits::ControlFlow::Poll)),
        }
    }
}
