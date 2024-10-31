use std::{
    any::Any,
    collections::{vec_deque::Iter, HashMap, HashSet, VecDeque},
};

use legion::system;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while1},
    character::complete::{line_ending, not_line_ending, space0},
    combinator::{opt, recognize},
    multi::{many0, many1},
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};

use crate::ResourceFiles;

pub type ConsoleCmd = Vec<String>;
pub type ConsoleVar = Box<dyn Any + Send + Sync>;

#[derive(Default)]
pub struct Console {
    command_registry: HashSet<String>,
    command_queue: VecDeque<ConsoleCmd>,
    variables: HashMap<String, ConsoleVar>,
    alias: HashMap<String, ConsoleCmd>,
}

impl Console {
    pub fn push_command(&mut self, cmd: &str) {
        let cmd = format!("{}\r\n", cmd.trim().to_lowercase());
        let (_remaining, command) = Self::command(cmd.as_str()).unwrap();
        self.command_queue
            .push_back(command.iter().map(|s| s.to_string()).collect());
    }

    pub fn commands(&self) -> Iter<'_, Vec<String>> {
        self.command_queue.iter()
    }

    pub fn set_var<T: Send + Sync + 'static>(&mut self, var_name: &str, var_value: T) {
        self.variables
            .insert(var_name.to_string(), Box::new(var_value));
    }

    pub fn get_var<T: Send + Sync + 'static>(&self, var_name: &str) -> Option<&T> {
        self.variables.get(var_name)?.downcast_ref::<T>()
    }

    pub fn remove_var(&mut self, var_name: &str) {
        self.variables.remove(var_name);
    }

    pub fn set_alias(&mut self, alias: &str, cmd: String) {
        let cmd = format!("{}\r\n", cmd.trim().to_lowercase());
        let (_remaining, command) = Self::command(cmd.as_str()).unwrap();
        self.alias.insert(
            alias.to_string(),
            command.iter().map(|s| s.to_string()).collect(),
        );
    }

    pub fn get_alias(&mut self, alias: &str) -> Option<&ConsoleCmd> {
        self.alias.get(alias)
    }

    pub fn remove_alias(&mut self, alias: &str) {
        self.alias.remove(alias);
    }

    pub fn register_command(&mut self, cmd_name: &str) {
        let cmd_name = cmd_name.trim().to_lowercase();
        self.command_registry.insert(cmd_name.to_owned());
    }

    pub fn unregister_command(&mut self, cmd_name: &str) {
        let cmd_name = cmd_name.trim().to_lowercase();
        self.command_registry.remove(cmd_name.as_str());
    }

    fn execute_command(
        &mut self,
        command: &ConsoleCmd,
        resource_files: &mut ResourceFiles,
    ) -> Option<VecDeque<ConsoleCmd>> {
        match &command[..] {
            // Execute a script file.
            [ref cmd, filename] if cmd == "exec" => {
                let mut buf = String::new();
                let mut queue = VecDeque::new();

                if let Ok(mut reader) = resource_files.take(filename) {
                    reader.read_to_string(&mut buf).unwrap();
                    buf.push_str("\r\n");
                }

                if let Ok((_remaining, commands)) = Self::many_commands(buf.as_str()) {
                    for cmd in commands {
                        let cmd: Vec<String> =
                            cmd.iter().map(|s| s.trim().to_lowercase()).collect();

                        if let Some(mut result) = self.execute_command(&cmd, resource_files) {
                            queue.append(&mut result);
                        }
                    }
                }

                Some(queue)
            }
            // The alias command is used to create a reference to a command or list of commands.  When aliasing multiple commands,
            // or commands that contain multiple words (such as "fraglimit 50"), you must enclose all the commands in quotation
            // marks and separate each command with a semi-colon.
            //
            // Using alias without the [command] option will erase the alias specified in <name>
            [ref cmd, name, command @ ..] if cmd == "alias" => {
                if command.is_empty() {
                    self.remove_alias(name);
                } else {
                    self.set_alias(name, command[0].to_owned());
                }

                None
            }
            [ref cvar, value] if !self.command_registry.contains(cvar) => {
                self.set_var(cvar, value.to_owned());

                None
            }
            [alias, args @ ..] if self.alias.contains_key(alias) => {
                if let Some(cmd) = self.get_alias(alias) {
                    Some(VecDeque::from([cmd
                        .iter()
                        .chain(args.iter())
                        .cloned()
                        .collect()]))
                } else {
                    None
                }
            }
            _ => Some(VecDeque::from([command.to_owned()])),
        }
    }

    fn many_commands(input: &str) -> IResult<&str, Vec<Vec<&str>>> {
        delimited(
            many0(Self::empty_line),
            many0(terminated(Self::command, many0(Self::empty_line))),
            many0(Self::empty_line),
        )(input)
    }

    fn command(input: &str) -> IResult<&str, Vec<&str>> {
        terminated(
            many1(preceded(space0, Self::argument)),
            Self::command_terminator,
        )(input)
    }

    fn argument(input: &str) -> IResult<&str, &str> {
        alt((Self::quoted_argument, Self::basic_argument))(input)
    }

    fn basic_argument(input: &str) -> IResult<&str, &str> {
        take_while1(|c: char| !c.is_whitespace() && c != ';' && c != '/' && c != '\n' && c != '\r')(
            input,
        )
    }

    fn quoted_argument(input: &str) -> IResult<&str, &str> {
        alt((
            delimited(tag("\""), is_not("\""), tag("\"")),
            delimited(tag("'"), is_not("'"), tag("'")),
        ))(input)
    }

    fn command_terminator(input: &str) -> IResult<&str, &str> {
        alt((tag(";"), Self::empty_line))(input)
    }

    fn empty_line(input: &str) -> IResult<&str, &str> {
        recognize(tuple((space0, opt(Self::line_comment), line_ending)))(input)
    }

    fn line_comment(input: &str) -> IResult<&str, &str> {
        recognize(preceded(tag("//"), not_line_ending))(input)
    }
}

#[system]
pub fn console_command_preprocessor(
    #[resource] console: &mut Console,
    #[resource] resource_files: &mut ResourceFiles,
) {
    let mut command_queue = VecDeque::new();
    command_queue.extend(console.command_queue.drain(..));
    for command in command_queue {
        if let Some(commands) = console.execute_command(&command, resource_files) {
            console.command_queue.extend(commands);
        }
    }
}

#[system]
pub fn console_command_postprocessor(#[resource] console: &mut Console) {
    console.command_queue.clear();
}
