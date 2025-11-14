use rustyline::completion::Completer;
use rustyline::{Context, Helper, Highlighter, Hinter, Validator};
use std::sync::{Arc, Mutex, RwLock};

pub mod builtins;
pub mod command;

pub struct Console {
    command_buffer: RwLock<command::CommandBuffer>,
    command_aliases: RwLock<command::CommandAliases>,
    command_variables: RwLock<command::CommandVariables>,
    command_registry: RwLock<command::CommandRegistry>,
    command_executor: Mutex<command::CommandExecutor>,
}

impl Console {
    pub fn new(resources: Arc<quake_resources::Resources>) -> Self {
        let command_buffer = RwLock::new(command::CommandBuffer::default());
        let command_aliases = RwLock::new(command::CommandAliases::default());
        let command_variables = RwLock::new(command::CommandVariables::default());
        let command_registry = RwLock::new(command::CommandRegistry::default());
        let command_executor = Mutex::new(command::CommandExecutor::default());

        Self {
            command_buffer,
            command_aliases,
            command_variables,
            command_registry,
            command_executor,
        }
    }

    pub fn register_commands_handler<H>(&self, commands: &[&str], handler: H) -> anyhow::Result<()>
    where
        H: quake_traits::CommandHandler + Clone + 'static,
    {
        let mut command_registry = self
            .command_registry
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        command_registry.register_commands_handler(commands, handler);
        Ok(())
    }

    pub fn unregister_command(&self, name: &str) -> anyhow::Result<()> {
        let mut command_registry = self
            .command_registry
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        command_registry.unregister_command(name);
        Ok(())
    }

    pub fn prepend_text(&mut self, text: &str) {
        self.command_buffer.write().unwrap().push_front(text);
    }

    pub fn append_text(&self, text: &str) {
        self.command_buffer.write().unwrap().push_back(text);
    }

    pub async fn execute(&self) -> anyhow::Result<()> {
        let mut command_executor = self
            .command_executor
            .lock()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        command_executor
            .execute(
                &mut self.command_buffer.write().unwrap(),
                &self.command_aliases.read().unwrap(),
                &mut self.command_variables.write().unwrap(),
                &mut self.command_registry.write().unwrap(),
            )
            .await
    }

    pub async fn repl(&mut self) -> anyhow::Result<()> {
        let config = rustyline::Config::builder()
            .auto_add_history(true)
            .history_ignore_space(true)
            .completion_type(rustyline::CompletionType::List)
            .edit_mode(rustyline::EditMode::Emacs)
            .build();

        let mut rl = rustyline::Editor::with_config(config)?;
        rl.set_helper(Some(ConsoleHelper {
            commands: self.list_of_registry_commands(),
        }));

        let history_file = ".quake_history";
        let _ = rl.load_history(history_file);

        loop {
            let readline = rl.readline(">>> ");
            match readline {
                Ok(line) => {
                    self.append_text(line.as_str());
                    self.execute().await?;
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

    fn list_of_registry_commands(&self) -> Vec<String> {
        self.command_registry
            .read()
            .unwrap()
            .commands()
            .map(String::from)
            .collect()
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
