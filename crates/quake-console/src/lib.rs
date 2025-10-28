use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

#[derive(Default)]
pub struct ConsoleVariables {
    console: HashMap<String, String>,
}

impl ConsoleVariables {
    pub fn get<T>(&self, name: &str) -> Option<T>
    where
        T: std::str::FromStr,
    {
        self.console.get(name).and_then(|value| value.parse().ok())
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.console.insert(name.to_string(), value.to_string());
    }
}

#[derive(Default)]
pub struct Console {
    console_variables: ConsoleVariables,
    command_aliases: Rc<RefCell<CommandAliases>>,
    command_buffer: CommandBuffer,
    command_registry: CommandRegistry,
}

impl Console {
    pub fn new(resources: &quake_resource::Resources) -> Self {
        let console_variables = ConsoleVariables::default();

        let command_aliases = Rc::new(RefCell::new(CommandAliases::default()));
        let command_buffer = CommandBuffer::default();

        let mut command_registry = CommandRegistry::default();
        Self::register_alias_command(&mut command_registry, Rc::clone(&command_aliases));
        Self::register_exec_command(&mut command_registry);

        Self {
            console_variables,
            command_aliases,
            command_buffer,
            command_registry,
        }
    }

    pub fn add_alias(&mut self, alias: &str, command: &str) {
        self.command_aliases
            .borrow_mut()
            .register_alias(alias, command);
    }

    pub fn add_command(&mut self, name: &str, command: Command) {
        self.command_registry.register_command(name, command);
    }

    pub fn add_text(&mut self, text: &str) {
        self.command_buffer.push_back(text);
    }

    pub fn execute(&mut self, resources: &quake_resource::Resources) {
        self.command_buffer.execute(
            resources,
            &mut self.console_variables,
            Rc::clone(&self.command_aliases),
            &mut self.command_registry,
        );
    }

    fn register_alias_command(
        command_registry: &mut CommandRegistry,
        command_aliases: Rc<RefCell<CommandAliases>>,
    ) {
        command_registry.register_command("alias", move |_, _, args| {
            let alias = args[0];
            if args.len() > 1 {
                let s = args[1..].join(" ");
                let command_text = s
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(&s)
                    .replace(";", "\n");
                command_aliases
                    .borrow_mut()
                    .register_alias(alias, &command_text);
            } else {
                command_aliases.borrow_mut().unregister_alias(alias);
            }
            None
        });
    }

    fn register_exec_command(command_registry: &mut CommandRegistry) {
        command_registry.register_command("exec", |resources, _, args| {
            let file_name = args[0];
            if let Ok(file_contents) = resources.by_name::<String>(file_name) {
                Some(file_contents)
            } else {
                None
            }
        });
    }
}

#[derive(Default)]
struct CommandAliases {
    aliases: HashMap<String, String>,
}

impl CommandAliases {
    fn register_alias(&mut self, alias: &str, command: &str) {
        self.aliases.insert(alias.to_string(), command.to_string());
    }

    fn unregister_alias(&mut self, alias: &str) {
        self.aliases.remove(alias);
    }

    fn get_alias(&self, alias: &str) -> Option<&str> {
        self.aliases.get(alias).map(|command| command.as_str())
    }
}

#[derive(Default)]
struct CommandBuffer {
    command_buffer: VecDeque<String>,
}

impl CommandBuffer {
    fn push_back(&mut self, text: &str) {
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .for_each(|line| {
                self.command_buffer.push_back(line.to_string());
            });
    }

    fn push_front(&mut self, text: &str) {
        let lines = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with("//"));
        lines.rev().for_each(|line| {
            self.command_buffer.push_front(line.to_string());
        })
    }

    fn execute(
        &mut self,
        resources: &quake_resource::Resources,
        variables: &mut ConsoleVariables,
        aliases: Rc<RefCell<CommandAliases>>,
        registry: &CommandRegistry,
    ) {
        while let Some(command_line) = self.command_buffer.pop_front() {
            let mut args = command_line.split_whitespace();
            let command_name = args.next().unwrap();
            let command_args = args.collect::<Vec<_>>();

            if let Some(command_text) = aliases.borrow().get_alias(command_name) {
                self.push_front(command_text);
                continue;
            }

            if let Some(command_handler) = registry.get_command(command_name) {
                if let Some(additional_commands) =
                    command_handler(resources, variables, &command_args)
                {
                    self.push_front(&additional_commands);
                }

                continue;
            }

            let variable_name = command_name;
            let variable_arg = command_args.join(" ");
            variables.set(variable_name, &variable_arg);
        }
    }
}

pub type Command =
    Box<dyn Fn(&quake_resource::Resources, &ConsoleVariables, &[&str]) -> Option<String>>;

#[derive(Default)]
struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    fn register_command<C>(&mut self, name: &str, command: C)
    where
        C: Fn(&quake_resource::Resources, &ConsoleVariables, &[&str]) -> Option<String> + 'static,
    {
        self.commands.insert(name.to_string(), Box::new(command));
    }

    fn unregister_command(&mut self, name: &str) {
        self.commands.remove(name);
    }

    fn get_command(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }
}
