use crate::ControlFlow;
use std::collections::{HashMap, VecDeque};

#[derive(Default)]
pub struct CommandAliases {
    aliases: HashMap<String, String>,
}

impl CommandAliases {
    pub fn register_alias(&mut self, alias: &str, command: &str) {
        self.aliases.insert(alias.to_string(), command.to_string());
    }

    pub fn unregister_alias(&mut self, alias: &str) {
        self.aliases.remove(alias);
    }

    pub fn get(&self, alias: &str) -> Option<&str> {
        self.aliases.get(alias).map(|command| command.as_str())
    }
}

#[derive(Default)]
pub struct CommandBuffer {
    buffer: VecDeque<String>,
}

impl CommandBuffer {
    pub fn pop_front(&mut self) -> Option<String> {
        self.buffer.pop_front()
    }

    pub fn push_back(&mut self, text: &str) {
        self.process_lines(text).for_each(|line| {
            self.buffer.push_back(line);
        });
    }

    pub fn push_front(&mut self, text: &str) {
        self.process_lines(text)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .for_each(|line| {
                self.buffer.push_front(line);
            });
    }

    fn process_lines<'a>(&self, text: &'a str) -> impl Iterator<Item = String> + 'a {
        text.lines()
            .map(|line| {
                let trimmed = line.trim();
                if let Some(comment_position) = trimmed.find("//") {
                    line[..comment_position].trim().to_string()
                } else {
                    trimmed.to_string()
                }
            })
            .filter(|line| !line.is_empty())
    }
}

pub struct CommandContext<'a> {
    pub buffer: &'a mut CommandBuffer,
    pub aliases: &'a mut CommandAliases,
    pub variables: &'a mut CommandVariables,
    pub writer: Box<dyn std::io::Write>,
}

pub type Command = Box<dyn Fn(&mut CommandContext, &[&str]) -> ControlFlow>;

#[derive(Default)]
pub struct CommandRegistry {
    registry: HashMap<String, Command>,
}

impl CommandRegistry {
    pub fn register_command(&mut self, name: &str, command: Command) {
        self.registry.insert(name.to_string(), command);
    }

    pub fn unregister_command(&mut self, name: &str) {
        self.registry.remove(name);
    }

    pub fn get(&self, name: &str) -> Option<&Command> {
        self.registry.get(name)
    }

    pub fn commands(&self) -> impl Iterator<Item = &String> {
        self.registry.keys()
    }
}

#[derive(Default)]
pub struct CommandVariables {
    variables: HashMap<String, String>,
}

impl CommandVariables {
    pub fn get<T>(&self, name: &str) -> Option<T>
    where
        T: std::str::FromStr,
    {
        self.variables
            .get(name)
            .and_then(|value| value.parse().ok())
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }
}

#[derive(Default)]
pub struct CommandExecutor {
    control_flow: ControlFlow,
}

impl CommandExecutor {
    pub fn execute(&mut self, context: &mut CommandContext, registry: &CommandRegistry) {
        self.control_flow = ControlFlow::Poll;
        while let Some(command_line) = context.buffer.pop_front() {
            let (name, args) = self.parse_command_line(&command_line);

            if self.try_execute_alias(context, &name) {
                continue;
            }

            if self.try_execute_command(context, registry, &name, &args) {
                if self.control_flow == ControlFlow::Wait {
                    break;
                }
                continue;
            }

            self.assign_variable(context, &name, &args);
        }
    }

    fn parse_command_line<'a>(&self, command_line: &'a str) -> (&'a str, Vec<&'a str>) {
        let mut args = command_line.split_whitespace();
        let name = args.next().unwrap_or("");
        let args = args.collect::<Vec<_>>();
        (name, args)
    }

    fn try_execute_alias(&self, context: &mut CommandContext, name: &str) -> bool {
        if let Some(command_line) = context.aliases.get(name) {
            context.buffer.push_front(command_line);
            true
        } else {
            false
        }
    }

    fn try_execute_command(
        &self,
        context: &mut CommandContext,
        registry: &CommandRegistry,
        name: &str,
        args: &[&str],
    ) -> bool {
        if let Some(command_fn) = registry.get(name) {
            command_fn(context, args);
            true
        } else {
            false
        }
    }

    fn assign_variable(&self, context: &mut CommandContext, name: &str, args: &[&str]) {
        let value = args.join(" ");
        context.variables.set(name, &value);
    }
}
