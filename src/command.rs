use std::io::BufRead;

use anyhow::bail;

use crate::util::{RushError, process_input};

#[derive(Debug, Eq, PartialEq)]
pub enum CommandType {
    Echo,
    Exit,
    Type,
    Unknown,
}

#[derive(Debug)]
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
            "type" => CommandType::Type,
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
            CommandType::Type => match &self.args[1..].first() {
                Some(cmd_name) => match Self::find_type(&cmd_name) {
                    CommandType::Unknown => bail!("{}: not found", cmd_name),
                    _ => {
                        println!("{} is a shell builtin", cmd_name);
                        Ok(())
                    }
                },
                None => Ok(()),
            },
            CommandType::Unknown => bail!(RushError::CommandNotFound(self.name.to_owned())),
        }
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.type_ {
            CommandType::Echo => write!(f, "echo"),
            CommandType::Exit => write!(f, "exit"),
            CommandType::Type => write!(f, "type"),
            CommandType::Unknown => write!(f, "{}", self.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::RushError;

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

    #[test]
    fn should_return_command_not_found_for_unknown_command() {
        let input = "nonexistent";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_err());

        let err = result.unwrap_err();
        let rush_err = err.downcast_ref::<RushError>();
        assert!(rush_err.is_some());
        assert!(matches!(rush_err.unwrap(), RushError::CommandNotFound(_)));
    }

    #[test]
    fn should_return_unexpected_eof_on_read_failure() {
        struct FailingReader;

        impl io::Read for FailingReader {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"))
            }
        }

        impl io::BufRead for FailingReader {
            fn fill_buf(&mut self) -> io::Result<&[u8]> {
                Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"))
            }

            fn consume(&mut self, _amt: usize) {}
        }

        let reader = FailingReader;
        let result = Command::new(reader);

        assert!(result.is_err());
        let err = result.unwrap_err();
        let rush_err = err.downcast_ref::<RushError>();
        assert!(rush_err.is_some());
        assert!(matches!(rush_err.unwrap(), RushError::UnexpectedEOF));
    }

    #[test]
    fn should_parse_command_with_multiple_arguments() {
        let input = "echo hello world foo";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["echo", "hello", "world", "foo"]);
        assert!(matches!(cmd.type_, CommandType::Echo));
    }

    #[test]
    fn should_identify_echo_command_type() {
        let cmd_type = Command::find_type("echo");
        assert!(matches!(cmd_type, CommandType::Echo));
    }

    #[test]
    fn should_identify_exit_command_type() {
        let cmd_type = Command::find_type("exit");
        assert!(matches!(cmd_type, CommandType::Exit));
    }

    #[test]
    fn should_print_all_arguments_for_echo_command() {
        let input = "echo hello world";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        // note: can't easily capture stdout in a unit test,
        // but we can verify the command runs without error
        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_exit_command_should_succeed() {
        let input = "exit";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_echo_with_no_args_should_succeed() {
        let input = "echo";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_echo_with_single_arg_should_succeed() {
        let input = "echo hello";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_echo_with_multiple_args_should_succeed() {
        let input = "echo hello world test";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_type_with_builtin_should_succeed() {
        let input = "type echo";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_type_with_exit_builtin_should_succeed() {
        let input = "type exit";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_type_with_type_builtin_should_succeed() {
        let input = "type type";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_type_with_unknown_command_should_fail() {
        let input = "type nonexistent";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn run_type_with_no_args_should_succeed() {
        let input = "type";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_unknown_command_should_fail_with_command_not_found() {
        let input = "unknowncmd";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_err());

        let err = result.unwrap_err();
        let rush_err = err.downcast_ref::<RushError>();
        assert!(rush_err.is_some());
        assert!(matches!(rush_err.unwrap(), RushError::CommandNotFound(_)));
    }

    #[test]
    fn run_unknown_command_error_contains_command_name() {
        let input = "mycustomcmd";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        let result = cmd.run();
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_string = err.to_string();
        assert!(err_string.contains("mycustomcmd"));
        assert!(err_string.contains("command not found"));
    }
}
