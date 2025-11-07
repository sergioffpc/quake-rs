use rustyline::completion::Completer;
use rustyline::{Context, Helper, Highlighter, Hinter, Validator};
use std::rc::Rc;

mod command;

pub struct Console {
    command_buffer: command::CommandBuffer,
    command_aliases: command::CommandAliases,
    command_variables: command::CommandVariables,
    command_registry: command::CommandRegistry,
    command_executor: command::CommandExecutor,

    resources: Rc<quake_resources::Resources>,
}

impl Console {
    pub fn new(resources: Rc<quake_resources::Resources>) -> Self {
        let command_buffer = command::CommandBuffer::default();
        let command_aliases = command::CommandAliases::default();
        let command_variables = command::CommandVariables::default();

        let mut command_registry = command::CommandRegistry::default();
        command_registry.register_command("alias", move |ctx, args| {
            let alias = args[0];
            if args.len() > 1 {
                let s = args[1..].join(" ");
                let command_text = s
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(&s)
                    .replace(";", "\n");
                ctx.aliases.register_alias(alias, &command_text);
            } else {
                ctx.aliases.unregister_alias(alias);
            }

            command::ControlFlow::Poll
        });

        command_registry.register_command("exec", move |ctx, args| {
            let file_name = args[0];
            if let Ok(file_contents) = ctx.resources.by_name::<String>(file_name) {
                ctx.buffer.push_front(&file_contents);
            }

            command::ControlFlow::Poll
        });

        command_registry.register_command("quit", move |_, _| std::process::exit(0));
        command_registry.register_command("wait", move |_, _| command::ControlFlow::Wait);

        let command_executor = command::CommandExecutor::default();

        Self {
            command_buffer,
            command_aliases,
            command_variables,
            command_registry,
            command_executor,

            resources,
        }
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
            resources: &self.resources,
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
