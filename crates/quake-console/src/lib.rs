use crate::command::Command;
use rustyline::completion::Completer;
use rustyline::{Context, Helper, Highlighter, Hinter, Validator};
use std::rc::Rc;

mod builtins;
pub mod command;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum ControlFlow {
    #[default]
    Poll,
    Wait,
}

pub struct Console {
    command_buffer: command::CommandBuffer,
    command_aliases: command::CommandAliases,
    command_variables: command::CommandVariables,
    command_registry: command::CommandRegistry,
    command_executor: command::CommandExecutor,
}

impl Console {
    pub fn new(resources: Rc<quake_resources::Resources>) -> Self {
        let command_buffer = command::CommandBuffer::default();
        let command_aliases = command::CommandAliases::default();
        let command_variables = command::CommandVariables::default();

        let mut command_registry = command::CommandRegistry::default();
        command_registry.register_command("alias", builtins::alias());
        command_registry.register_command("echo", builtins::echo());
        command_registry.register_command("exec", builtins::exec(resources.clone()));
        command_registry.register_command("quit", builtins::quit());
        command_registry.register_command("wait", builtins::wait());

        let command_executor = command::CommandExecutor::default();

        Self {
            command_buffer,
            command_aliases,
            command_variables,
            command_registry,
            command_executor,
        }
    }

    pub fn register_command(&mut self, name: &str, command: Command) {
        self.command_registry.register_command(name, command);
    }

    pub fn unregister_command(&mut self, name: &str) {
        self.command_registry.unregister_command(name);
    }

    pub fn prepend_text(&mut self, text: &str) {
        self.command_buffer.push_front(text);
    }

    pub fn append_text(&mut self, text: &str) {
        self.command_buffer.push_back(text);
    }

    pub fn execute(&mut self) {
        let mut command_context = command::CommandContext {
            buffer: &mut self.command_buffer,
            aliases: &mut self.command_aliases,
            variables: &mut self.command_variables,
            writer: Box::new(std::io::stdout()),
        };

        self.command_executor
            .execute(&mut command_context, &self.command_registry);
    }

    pub fn repl(&mut self) -> anyhow::Result<()> {
        let config = rustyline::Config::builder()
            .auto_add_history(true)
            .history_ignore_space(true)
            .completion_type(rustyline::CompletionType::List)
            .edit_mode(rustyline::EditMode::Emacs)
            .build();

        let mut rl = rustyline::Editor::with_config(config)?;
        let commands = self.command_registry.commands().map(String::from).collect();
        rl.set_helper(Some(ConsoleHelper { commands }));

        let history_file = ".quake_history";
        let _ = rl.load_history(history_file);

        loop {
            let readline = rl.readline(">>> ");
            match readline {
                Ok(line) => {
                    self.append_text(line.as_str());
                    self.execute();
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
