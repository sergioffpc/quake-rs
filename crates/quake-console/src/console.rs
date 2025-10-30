use crate::command::{CommandAliases, CommandBuffer, CommandRegistry};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tracing::log::info;

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

            if let Some(command_text) = self.command_aliases.get(command_name) {
                info!("Executing alias: {}", command_text);

                self.command_buffer.push_front(command_text);
                continue;
            }
            if let Some(command_handler) = self.command_registry.get(command_name).cloned() {
                info!(
                    "Executing command: {} {}",
                    command_name,
                    command_args.join(" ")
                );

                command_handler(self, &command_args);
                if self.execution_control_flow == ExecutionControlFlow::Suspended {
                    break;
                }
                continue;
            }

            let variable_name = command_name;
            let variable_arg = command_args.join(" ");
            info!("Setting variable: {} = {}", variable_name, variable_arg);

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
