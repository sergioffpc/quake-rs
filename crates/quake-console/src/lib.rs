use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ExecutionControlFlow {
    Running,
    Suspended,
    Stopped,
}

pub struct Console {
    resources: Rc<RefCell<quake_resource::Resources>>,
    console_variables: ConsoleVariables,
    command_aliases: CommandAliases,
    command_buffer: CommandBuffer,
    command_registry: CommandRegistry,
    execution_control_flow: ExecutionControlFlow,
}

impl Console {
    pub fn new(resources: Rc<RefCell<quake_resource::Resources>>) -> Self {
        Console {
            resources,
            console_variables: ConsoleVariables::default(),
            command_aliases: CommandAliases::default(),
            command_buffer: CommandBuffer::default(),
            command_registry: CommandRegistry::default(),
            execution_control_flow: ExecutionControlFlow::Stopped,
        }
    }

    pub fn register_command<C>(&mut self, name: &str, command: C)
    where
        C: Fn(&mut Console, &[&str]) + 'static,
    {
        self.command_registry.register_command(name, command);
    }

    pub fn unregister_command(&mut self, name: &str) {
        self.command_registry.unregister_command(name);
    }

    pub fn get_variable<T>(&self, name: &str) -> Option<T>
    where
        T: std::str::FromStr,
    {
        self.console_variables.get(name)
    }

    pub fn set_variable(&mut self, name: &str, value: &str) {
        self.console_variables.set(name, value);
    }

    pub fn prepend_script(&mut self, text: &str) {
        self.command_buffer.push_front(text);
    }

    pub fn append_script(&mut self, text: &str) {
        self.command_buffer.push_back(text);
    }

    pub fn execute(&mut self) {
        self.execution_control_flow = ExecutionControlFlow::Running;
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
                command_handler(self, &command_args);
                if self.execution_control_flow == ExecutionControlFlow::Suspended {
                    break;
                }
                continue;
            }

            let variable_name = command_name;
            let variable_arg = command_args.join(" ");
            self.console_variables.set(variable_name, &variable_arg);
        }
        self.execution_control_flow = ExecutionControlFlow::Stopped;
    }

    pub fn register_builtin_commands(&mut self) {
        self.register_alias_command();
        self.register_exec_command();
        self.register_wait_command();
    }

    fn register_alias_command(&mut self) {
        self.command_registry
            .register_command("alias", move |console, args| {
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

    fn register_exec_command(&mut self) {
        let resources = self.resources.clone();
        self.command_registry
            .register_command("exec", move |console, args| {
                let file_name = args[0];
                if let Ok(file_contents) = resources.borrow_mut().by_name::<String>(file_name) {
                    console.prepend_script(&file_contents);
                }
            });
    }

    fn register_wait_command(&mut self) {
        self.command_registry
            .register_command("wait", move |console, _| {
                console.execution_control_flow = ExecutionControlFlow::Suspended;
            });
    }
}

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
        self.process_lines(text).for_each(|line| {
            self.command_buffer.push_back(line);
        });
    }

    fn push_front(&mut self, text: &str) {
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
struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    fn register_command<C>(&mut self, name: &str, command: C)
    where
        C: Fn(&mut Console, &[&str]) + 'static,
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
