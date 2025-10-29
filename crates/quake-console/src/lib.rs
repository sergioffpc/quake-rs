use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

#[derive(Default)]
struct ConsoleVariables {
    console: HashMap<String, String>,
}

impl ConsoleVariables {
    fn get<T>(&self, name: &str) -> Option<T>
    where
        T: std::str::FromStr,
    {
        self.console.get(name).and_then(|value| value.parse().ok())
    }

    fn set(&mut self, name: &str, value: &str) {
        self.console.insert(name.to_string(), value.to_string());
    }
}

#[derive(Default)]
pub struct Console {
    console_variables: ConsoleVariables,
    command_aliases: CommandAliases,
    command_buffer: CommandBuffer,
    command_registry: CommandRegistry,
}

impl Console {
    pub fn prepend_script(&mut self, text: &str) {
        self.command_buffer.push_front(text);
    }

    pub fn append_script(&mut self, text: &str) {
        self.command_buffer.push_back(text);
    }

    pub fn execute(&mut self, resources: &quake_resource::Resources) {
        while let Some(command_line) = self.command_buffer.pop_front() {
            let mut args = command_line.split_whitespace();
            let command_name = args.next().unwrap();
            let command_args = args.collect::<Vec<_>>();

            if let Some(command_text) = self.command_aliases.get_alias(command_name) {
                self.command_buffer.push_front(command_text);
                continue;
            }

            if let Some(command_handler) = self.command_registry.get_command(command_name).cloned()
            {
                command_handler(self, resources, &command_args);
                continue;
            }

            let variable_name = command_name;
            let variable_arg = command_args.join(" ");
            self.console_variables.set(variable_name, &variable_arg);
        }
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
    fn pop_front(&mut self) -> Option<String> {
        self.command_buffer.pop_front()
    }

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
}

type Command = Rc<dyn Fn(&mut Console, &quake_resource::Resources, &[&str])>;

#[derive(Default)]
struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    fn register_command<C>(&mut self, name: &str, command: C)
    where
        C: Fn(&mut Console, &quake_resource::Resources, &[&str]) + 'static,
    {
        self.commands.insert(name.to_string(), Rc::new(command));
    }

    fn unregister_command(&mut self, name: &str) {
        self.commands.remove(name);
    }

    fn get_command(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }
}

pub fn register_builtin_commands(console: &mut Console, resources: &quake_resource::Resources) {
    register_alias_command(console);
    register_exec_command(console, resources);
}

fn register_alias_command(console: &mut Console) {
    console
        .command_registry
        .register_command("alias", move |console, _, args| {
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
        });
}

fn register_exec_command(console: &mut Console, resources: &quake_resource::Resources) {
    console
        .command_registry
        .register_command("exec", move |console, resources, args| {
            let file_name = args[0];
            if let Ok(file_contents) = resources.by_name::<String>(file_name) {
                console.prepend_script(&file_contents);
            }
        });
}
