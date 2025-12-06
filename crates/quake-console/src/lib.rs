use rustyline::completion::Completer;
use rustyline::{Context, Helper, Highlighter, Hinter, Validator};
use std::fmt::Write;
use tokio::sync::{Mutex, RwLock};
use tracing::log::info;

pub mod command;
pub mod commands;

#[derive(Default)]
pub struct ConsoleManager {
    command_buffer: Mutex<command::CommandBuffer>,
    command_aliases: RwLock<command::CommandAliases>,
    command_variables: RwLock<command::CommandVariables>,
    command_registry: RwLock<command::CommandRegistry>,
}

impl ConsoleManager {
    pub async fn register_commands_handler<H>(
        &self,
        commands: &[&str],
        handler: H,
    ) -> anyhow::Result<()>
    where
        H: quake_traits::CommandHandler + Clone + 'static,
    {
        self.command_registry
            .write()
            .await
            .register_commands_handler(commands, handler);
        Ok(())
    }

    pub async fn unregister_command(&self, name: &str) -> anyhow::Result<()> {
        self.command_registry.write().await.unregister_command(name);
        Ok(())
    }

    pub async fn prepend_text(&mut self, text: &str) {
        self.command_buffer.lock().await.push_front(text);
    }

    pub async fn append_text(&self, text: &str) {
        self.command_buffer.lock().await.push_back(text);
    }

    pub async fn execute(&self) -> anyhow::Result<String> {
        let mut buffer = String::new();
        while let Some(command_line) = self.fetch_next_command().await {
            info!("Executing command: {}", command_line);

            let (name, args) = self.parse_command_line(command_line.as_str());

            if let Some(command_line) = self.command_aliases.read().await.get(name) {
                self.command_buffer.lock().await.push_front(command_line);
                continue;
            }

            if let Some(command_handler) = self.command_registry.write().await.get_mut(name) {
                match command_handler
                    .handle_command(
                        &std::iter::once(name)
                            .chain(args.iter().copied())
                            .collect::<Vec<_>>(),
                    )
                    .await?
                {
                    (output, control_flow) => {
                        if !output.is_empty() {
                            writeln!(&mut buffer, "{}", output)?;
                        }
                        match control_flow {
                            quake_traits::ControlFlow::Wait => break,
                            quake_traits::ControlFlow::Poll => continue,
                        }
                    }
                }
            }

            if args.is_empty() {
                self.command_variables
                    .read()
                    .await
                    .get::<String>(name)
                    .map(async |value| {
                        writeln!(&mut buffer, "{}", value).unwrap();
                    });
            } else {
                let value = args.join(" ");
                self.command_variables.write().await.set(name, &value);
            }
        }

        Ok(buffer)
    }

    pub async fn get<T>(&self, name: &str) -> Option<T>
    where
        T: std::str::FromStr,
    {
        self.command_variables.read().await.get::<T>(name)
    }

    pub async fn repl(&self) -> anyhow::Result<()> {
        let config = rustyline::Config::builder()
            .auto_add_history(true)
            .history_ignore_space(true)
            .completion_type(rustyline::CompletionType::List)
            .edit_mode(rustyline::EditMode::Emacs)
            .build();

        let mut rl = rustyline::Editor::with_config(config)?;
        rl.set_helper(Some(ConsoleHelper {
            commands: self
                .command_registry
                .read()
                .await
                .commands()
                .map(String::from)
                .collect(),
        }));

        let history_file = ".quake_history";
        let _ = rl.load_history(history_file);

        loop {
            let readline = rl.readline(">>> ");
            match readline {
                Ok(line) => {
                    self.append_text(line.as_str()).await;
                    let output = self.execute().await?;
                    if !output.is_empty() {
                        print!("{}", output);
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted)
                | Err(rustyline::error::ReadlineError::Eof) => {
                    break;
                }
                Err(err) => {
                    anyhow::bail!("Error: {}", err);
                }
            }
        }

        rl.save_history(history_file)
            .map_err(|err| anyhow::anyhow!(err))
    }

    async fn fetch_next_command(&self) -> Option<String> {
        self.command_buffer.lock().await.pop_front()
    }

    fn parse_command_line<'a>(&self, command_line: &'a str) -> (&'a str, Vec<&'a str>) {
        let mut args = command_line.split_whitespace();
        let name = args.next().unwrap_or("");
        let args = args.collect::<Vec<_>>();
        (name, args)
    }
}

#[derive(Helper, Highlighter, Hinter, Validator)]
struct ConsoleHelper {
    commands: Vec<String>,
}

impl Completer for ConsoleHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let start = line[..pos]
            .rfind(char::is_whitespace)
            .map(|i| i + 1)
            .unwrap_or(0);
        let word = &line[start..pos];

        let mut candidates = Vec::new();
        for command in &self.commands {
            if command.starts_with(word) {
                candidates.push(command.clone());
            }
        }
        Ok((start, candidates))
    }
}
