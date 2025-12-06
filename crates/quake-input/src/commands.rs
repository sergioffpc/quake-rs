use crate::InputManager;
use std::sync::Arc;
use tabled::settings::{Padding, Style};
use tabled::{Table, Tabled};

#[derive(Clone)]
pub struct InputCommands {
    input_manager: Arc<InputManager>,
}

impl InputCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["bind", "unbind", "unbindall"];

    pub fn new(input_manager: Arc<InputManager>) -> Self {
        Self { input_manager }
    }

    async fn bind(&self, args: &[&str]) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        let mut buffer = String::new();

        match args.len() {
            0 => {
                #[derive(Tabled, Clone, Debug)]
                struct BindEntry {
                    key: String,
                    binding: String,
                }
                let bind_data: Vec<BindEntry> = self
                    .input_manager
                    .bindings
                    .iter()
                    .await
                    .map(|(k, v)| BindEntry {
                        key: k.clone(),
                        binding: v
                            .clone()
                            .split_whitespace()
                            .collect::<Vec<&str>>()
                            .join("; "),
                    })
                    .collect();

                buffer = if !bind_data.is_empty() {
                    Table::new(bind_data)
                        .with(Style::re_structured_text())
                        .with(Padding::new(1, 1, 0, 0))
                        .to_string()
                } else {
                    "No bindings defined".to_string()
                };
            }
            1 => {
                self.input_manager.bindings.unbind(args[0]).await;
            }
            _ => {
                let s = args[1..].join(" ");
                let command_text = s
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(&s)
                    .replace(";", "\n");
                self.input_manager
                    .bindings
                    .bind(args[0], &command_text)
                    .await;
            }
        }
        Ok((buffer, quake_traits::ControlFlow::Poll))
    }

    async fn unbind(&self, args: &[&str]) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        self.input_manager.bindings.unbind(args[0]).await;
        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }

    async fn unbindall(&mut self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        self.input_manager.bindings.clear().await;
        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for InputCommands {
    async fn handle_command(
        &mut self,
        command: &[&str],
    ) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        match command[0] {
            "bind" => self.bind(&command[1..]).await,
            "unbind" => self.unbind(&command[1..]).await,
            "unbindall" => self.unbindall().await,
            _ => Ok((String::default(), quake_traits::ControlFlow::Poll)),
        }
    }
}
