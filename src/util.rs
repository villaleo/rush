use std::io::BufRead;
use std::vec::Vec;

use anyhow::bail;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RushError {
    #[error("{0}: command not found")]
    CommandNotFound(String),
    #[error("error reading input: unexpected EOF")]
    UnexpectedEOF,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CommandType {
    Echo,
    Exit,
    Unknown,
}

pub struct Command {
    pub name: String,
    pub type_: CommandType,
    pub args: Vec<String>,
}

impl Command {
    pub fn new<R: BufRead>(reader: R) -> anyhow::Result<Command> {
        let args = process_input(reader)?;
        let name = args.first().unwrap().to_string();
        let type_ = Self::find_type(&name);

        Ok(Command { name, type_, args })
    }

    pub fn find_type(name: &str) -> CommandType {
        match name.trim() {
            "exit" => CommandType::Exit,
            "echo" => CommandType::Echo,
            _ => CommandType::Unknown,
        }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        match self.type_ {
            CommandType::Echo => {
                // Skip the first argument (command name)
                let args = &self.args[1..];

                if let Some((last, args)) = args.split_last() {
                    println!("{} {}", args.join(" "), last);
                }

                Ok(())
            }
            CommandType::Exit => Ok(()),
            CommandType::Unknown => bail!(RushError::CommandNotFound(self.name.to_owned())),
        }
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.type_ {
            CommandType::Echo => write!(f, "echo"),
            CommandType::Exit => write!(f, "exit"),
            CommandType::Unknown => write!(f, "{}", self.name),
        }
    }
}

fn process_input<R: BufRead>(mut reader: R) -> anyhow::Result<Vec<String>> {
    let mut input = String::new();
    reader
        .read_line(&mut input)
        .map_err(|_| RushError::UnexpectedEOF)?;

    let tokens = input
        .clone()
        .trim()
        .split(" ")
        .map(String::from)
        .collect::<Vec<_>>();

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self};

    #[test]
    fn should_exit_on_find_exit_cmd() {
        let input = "exit";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader);

        assert!(cmd.as_ref().is_ok());
        assert!(matches!(cmd.as_ref().unwrap().type_, CommandType::Exit));
        assert!(
            cmd.as_ref().unwrap().args.len() == 1,
            "command name should be the only arg"
        );
    }

    #[test]
    fn should_read_command() {
        let input = "go";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader);

        assert!(cmd.as_ref().is_ok());
        assert!(matches!(cmd.as_ref().unwrap().type_, CommandType::Unknown));
        assert!(
            cmd.as_ref().unwrap().args.len() == 1,
            "command name should be the only arg"
        );
    }

    #[test]
    fn should_find_command_type() {
        let cmd_name = String::from("go");
        let cmd = Command::find_type(&cmd_name);

        assert!(matches!(cmd, CommandType::Unknown));
    }
}
