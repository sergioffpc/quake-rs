use crate::console::Console;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

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
    command_buffer: VecDeque<String>,
}

impl CommandBuffer {
    pub fn pop_front(&mut self) -> Option<String> {
        self.command_buffer.pop_front()
    }

    pub fn push_back(&mut self, text: &str) {
        self.process_lines(text).for_each(|line| {
            self.command_buffer.push_back(line);
        });
    }

    pub fn push_front(&mut self, text: &str) {
        self.process_lines(text)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .for_each(|line| {
                self.command_buffer.push_front(line);
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

type Command = Rc<dyn Fn(&mut Console, &[&str])>;

#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    pub fn register_command<C>(&mut self, name: &str, command: C)
    where
        C: Fn(&mut Console, &[&str]) + 'static,
    {
        self.commands.insert(name.to_string(), Rc::new(command));
    }

    pub fn unregister_command(&mut self, name: &str) {
        self.commands.remove(name);
    }

    pub fn get(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }
}
