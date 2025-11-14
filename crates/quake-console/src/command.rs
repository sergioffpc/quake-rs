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

pub struct CommandRegistry {
    registry: multimap::MultiMap<String, Box<dyn quake_traits::CommandHandler>>,
}

impl CommandRegistry {
    pub fn register_commands_handler<H>(&mut self, commands: &[&str], handler: H)
    where
        H: quake_traits::CommandHandler + Clone + 'static,
    {
        for command in commands {
            self.registry
                .insert(command.to_string(), Box::new(handler.clone()));
        }
    }

    pub fn unregister_command(&mut self, name: &str) {
        self.registry.remove(name);
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Box<dyn quake_traits::CommandHandler>> {
        self.registry.get_mut(name)
    }

    pub fn commands(&self) -> impl Iterator<Item = &String> {
        self.registry.keys()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self {
            registry: multimap::MultiMap::new(),
        }
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

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum ControlFlow {
    #[default]
    Poll,
    Wait,
}

#[derive(Default)]
pub struct CommandExecutor {
    control_flow: ControlFlow,
}

impl CommandExecutor {
    pub async fn execute(
        &mut self,
        buffer: &mut CommandBuffer,
        aliases: &CommandAliases,
        variables: &mut CommandVariables,
        registry: &mut CommandRegistry,
    ) -> anyhow::Result<()> {
        self.control_flow = ControlFlow::Poll;
        while let Some(command_line) = buffer.pop_front() {
            let (name, args) = self.parse_command_line(&command_line);

            if self.try_execute_alias(aliases, buffer, &name) {
                continue;
            }

            if self.try_execute_command(registry, &name, &args).await? {
                if self.control_flow == ControlFlow::Wait {
                    break;
                }
                continue;
            }

            self.assign_variable(variables, &name, &args);
        }

        Ok(())
    }

    pub fn get_control_flow(&self) -> ControlFlow {
        self.control_flow
    }

    pub fn set_control_flow(&mut self, control_flow: ControlFlow) {
        self.control_flow = control_flow;
    }

    fn parse_command_line<'a>(&self, command_line: &'a str) -> (&'a str, Vec<&'a str>) {
        let mut args = command_line.split_whitespace();
        let name = args.next().unwrap_or("");
        let args = args.collect::<Vec<_>>();
        (name, args)
    }

    fn try_execute_alias(
        &self,
        aliases: &CommandAliases,
        buffer: &mut CommandBuffer,
        name: &str,
    ) -> bool {
        if let Some(command_line) = aliases.get(name) {
            buffer.push_front(command_line);
            true
        } else {
            false
        }
    }

    async fn try_execute_command(
        &self,
        registry: &mut CommandRegistry,
        name: &str,
        args: &[&str],
    ) -> anyhow::Result<bool> {
        if let Some(command_handler) = registry.get_mut(name) {
            command_handler
                .handle_command(
                    &std::iter::once(name)
                        .chain(args.iter().copied())
                        .collect::<Vec<_>>(),
                )
                .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn assign_variable(&self, variables: &mut CommandVariables, name: &str, args: &[&str]) {
        let value = args.join(" ");
        variables.set(name, &value);
    }
}
