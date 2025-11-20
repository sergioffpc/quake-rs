use parking_lot::{Mutex, RwLock};
use quake_traits::ControlFlow;
use rustyline::completion::Completer;
use rustyline::{Context, Helper, Highlighter, Hinter, Validator};

pub mod builtins;
pub mod command;

#[derive(Default)]
pub struct Console {
    command_buffer: Mutex<command::CommandBuffer>,
    command_aliases: RwLock<command::CommandAliases>,
    command_variables: RwLock<command::CommandVariables>,
    command_registry: RwLock<command::CommandRegistry>,
}

impl Console {
    pub fn register_commands_handler<H>(&self, commands: &[&str], handler: H) -> anyhow::Result<()>
    where
        H: quake_traits::CommandHandler + Clone + 'static,
    {
        self.command_registry
            .write()
            .register_commands_handler(commands, handler);
        Ok(())
    }

    pub fn unregister_command(&self, name: &str) -> anyhow::Result<()> {
        self.command_registry.write().unregister_command(name);
        Ok(())
    }

    pub fn prepend_text(&mut self, text: &str) {
        self.command_buffer.lock().push_front(text);
    }

    pub fn append_text(&self, text: &str) {
        self.command_buffer.lock().push_back(text);
    }

    pub fn execute(&self) -> anyhow::Result<()> {
        while let Some(command_line) = self.fetch_next_command() {
            let (name, args) = self.parse_command_line(command_line.as_str());

            if let Some(command_line) = self.command_aliases.read().get(name) {
                self.command_buffer.lock().push_front(command_line);
                continue;
            }

            if let Some(command_handler) = self.command_registry.write().get_mut(name) {
                match command_handler.handle_command(
                    &std::iter::once(name)
                        .chain(args.iter().copied())
                        .collect::<Vec<_>>(),
                )? {
                    ControlFlow::Wait => break,
                    ControlFlow::Poll => continue,
                }
            }

            let value = args.join(" ");
            self.command_variables.write().set(name, &value);
        }

        Ok(())
    }

    fn fetch_next_command(&self) -> Option<String> {
        self.command_buffer.lock().pop_front()
    }

    fn parse_command_line<'a>(&self, command_line: &'a str) -> (&'a str, Vec<&'a str>) {
        let mut args = command_line.split_whitespace();
        let name = args.next().unwrap_or("");
        let args = args.collect::<Vec<_>>();
        (name, args)
    }

    pub fn repl(&self) -> anyhow::Result<()> {
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
                    self.append_text(line.as_str());
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
