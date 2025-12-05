use crate::ConsoleManager;
use std::fmt::Write;
use std::sync::Arc;
use tabled::settings::{Padding, Style};
use tabled::{Table, Tabled};

#[derive(Clone)]
pub struct ConsoleCommands {
    console_manager: Arc<ConsoleManager>,
    resources_manager: Arc<quake_resources::ResourcesManager>,
}

impl ConsoleCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] =
        &["alias", "cvars", "echo", "exec", "quit", "wait", "version"];

    pub fn new(
        console_manager: Arc<ConsoleManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
    ) -> Self {
        Self {
            console_manager,
            resources_manager,
        }
    }

    async fn alias(
        &mut self,
        args: &[&str],
    ) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        let mut buffer = String::new();

        match args.len() {
            0 => {
                #[derive(Tabled, Clone)]
                struct AliasEntry {
                    #[tabled(rename = "Alias")]
                    name: String,
                    #[tabled(rename = "Command")]
                    command: String,
                }
                let alias_data: Vec<AliasEntry> = self
                    .console_manager
                    .command_aliases
                    .read()
                    .await
                    .iter()
                    .map(|(k, v)| AliasEntry {
                        name: k.clone(),
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

    fn echo(&mut self, args: &[&str]) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        let mut buffer = String::new();
        writeln!(&mut buffer, "{}", args.join(" "))?;
        Ok((buffer, quake_traits::ControlFlow::Poll))
    }

    async fn exec(&mut self, args: &[&str]) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        if let Ok(text) = self.resources_manager.by_name::<String>(args[0]).await {
            self.console_manager
                .command_buffer
                .lock()
                .await
                .push_front(&text);
        }
        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }

    async fn cvars(&self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        #[derive(Tabled, Clone)]
        struct VariableEntry {
            #[tabled(rename = "Variable")]
            name: String,
            #[tabled(rename = "Value")]
            value: String,
        }
        let cvars_data: Vec<VariableEntry> = self
            .console_manager
            .command_variables
            .read()
            .await
            .iter()
            .map(|(k, v)| VariableEntry {
                name: k.clone(),
                value: v.clone(),
            })
            .collect();

        let buffer = if !cvars_data.is_empty() {
            Table::new(cvars_data)
                .with(Style::re_structured_text())
                .with(Padding::new(1, 1, 0, 0))
                .to_string()
        } else {
            "No variables defined".to_string()
        };
        Ok((buffer, quake_traits::ControlFlow::Poll))
    }

    fn quit(&mut self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        std::process::exit(0);
    }

    fn wait(&mut self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        Ok((String::default(), quake_traits::ControlFlow::Wait))
    }

    fn version(&mut self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
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
            "cvars" => self.cvars().await,
            "echo" => self.echo(&command[1..]),
            "exec" => self.exec(&command[1..]).await,
            "quit" => self.quit(),
            "wait" => self.wait(),
            "version" => self.version(),
            _ => Ok((String::default(), quake_traits::ControlFlow::Poll)),
        }
    }
}
