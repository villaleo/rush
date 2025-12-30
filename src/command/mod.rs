mod handlers;
pub(crate) mod path;

use std::io;

use crate::util::{RushError, tokenize};

use self::{
    handlers::{handle_cd, handle_echo, handle_executable, handle_pwd, handle_type},
    path::find_in_path,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CommandType {
    Cd,
    Echo,
    Executable { path: String, name: String },
    Exit,
    Pwd,
    Type,
    Unknown(String),
}

impl std::fmt::Display for CommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandType::Cd => write!(f, "cd"),
            CommandType::Echo => write!(f, "echo"),
            CommandType::Executable { name, .. } => write!(f, "{}", name),
            CommandType::Exit => write!(f, "exit"),
            CommandType::Pwd => write!(f, "pwd"),
            CommandType::Type => write!(f, "type"),
            CommandType::Unknown(cmd) => write!(f, "{}", cmd),
        }
    }
}

impl CommandType {
    pub(crate) fn from_str(s: &str) -> Self {
        match s.trim() {
            "cd" => CommandType::Cd,
            "exit" => CommandType::Exit,
            "echo" => CommandType::Echo,
            "pwd" => CommandType::Pwd,
            "type" => CommandType::Type,
            unknown => CommandType::Unknown(unknown.to_string()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Command {
    pub type_: CommandType,
    pub args: Vec<String>,
}

impl Command {
    pub(crate) fn new<R: io::BufRead>(reader: R) -> Result<Command, RushError> {
        let args = tokenize(reader)?;

        // Read the name of the command from the tokenized args
        let Some(name) = args.first() else {
            return Err(RushError::Nop);
        };

        let type_ = CommandType::from_str(name);
        match type_ {
            CommandType::Unknown(cmd) => match find_in_path(&cmd)? {
                Some(path) => Ok(Command {
                    type_: CommandType::Executable { path, name: cmd },
                    args,
                }),
                None => Err(RushError::CommandNotFound(cmd)),
            },
            _ => Ok(Command { type_, args }),
        }
    }

    pub(crate) fn run(&self) -> Result<(), RushError> {
        match self.type_ {
            CommandType::Cd => handle_cd(&self.args),
            CommandType::Echo => handle_echo(&self.args),
            CommandType::Executable { ref path, ref name } => {
                match handle_executable(&path, &name, &self.args) {
                    Ok(_status) => Ok(()),
                    Err(error) => Err(error),
                }
            }
            CommandType::Exit => Ok(()),
            CommandType::Pwd => handle_pwd(&self.args),
            CommandType::Type => handle_type(&self.args),
            CommandType::Unknown(ref cmd_name) => Err(RushError::CommandNotFound(cmd_name.into())),
        }
    }

    #[cfg(test)]
    pub(crate) fn handle_executable(
        &self,
        path: &str,
        name: &str,
    ) -> Result<Option<i32>, RushError> {
        handle_executable(path, name, &self.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::RushError;
    use std::io;

    // Test helper to simplify command creation
    fn parse_cmd(input: &str) -> Result<Command, RushError> {
        Command::new(io::Cursor::new(input))
    }

    mod command_type {
        use super::*;

        #[test]
        fn parse_echo() {
            assert!(matches!(CommandType::from_str("echo"), CommandType::Echo));
        }

        #[test]
        fn parse_exit() {
            assert!(matches!(CommandType::from_str("exit"), CommandType::Exit));
        }

        #[test]
        fn parse_pwd() {
            assert!(matches!(CommandType::from_str("pwd"), CommandType::Pwd));
        }

        #[test]
        fn parse_type() {
            assert!(matches!(CommandType::from_str("type"), CommandType::Type));
        }

        #[test]
        fn parse_unknown_wraps_in_variant() {
            assert!(matches!(
                CommandType::from_str("nonexistent"),
                CommandType::Unknown(_)
            ));
        }

        #[test]
        fn display_formatting() {
            assert_eq!(CommandType::Echo.to_string(), "echo");
            assert_eq!(CommandType::Exit.to_string(), "exit");
            assert_eq!(CommandType::Pwd.to_string(), "pwd");
            assert_eq!(CommandType::Type.to_string(), "type");
            assert_eq!(CommandType::Unknown("custom".into()).to_string(), "custom");
        }

        #[test]
        fn whitespace_trimmed() {
            assert!(matches!(
                CommandType::from_str("  echo  "),
                CommandType::Echo
            ));
            assert!(matches!(
                CommandType::from_str("\texit\n"),
                CommandType::Exit
            ));
        }
    }

    mod command_parsing {
        use super::*;

        #[test]
        fn parse_exit() {
            let cmd = parse_cmd("exit").unwrap();
            assert!(matches!(cmd.type_, CommandType::Exit));
            assert_eq!(cmd.args, vec!["exit"]);
        }

        #[test]
        fn parse_echo_with_args() {
            let cmd = parse_cmd("echo hello world foo").unwrap();
            assert!(matches!(cmd.type_, CommandType::Echo));
            assert_eq!(cmd.args, vec!["echo", "hello", "world", "foo"]);
        }

        #[test]
        fn parse_pwd() {
            let cmd = parse_cmd("pwd").unwrap();
            assert!(matches!(cmd.type_, CommandType::Pwd));
            assert_eq!(cmd.args, vec!["pwd"]);
        }

        #[test]
        fn parse_type_with_arg() {
            let cmd = parse_cmd("type echo").unwrap();
            assert!(matches!(cmd.type_, CommandType::Type));
            assert_eq!(cmd.args, vec!["type", "echo"]);
        }

        #[test]
        fn unknown_command_returns_error() {
            let result = parse_cmd("nonexistent");
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), RushError::CommandNotFound(_)));
        }

        #[test]
        fn unknown_command_error_contains_name() {
            let result = parse_cmd("mycustomcmd");
            assert!(result.is_err());

            let error_str = result.unwrap_err().to_string();
            assert!(error_str.contains("mycustomcmd"));
            assert!(error_str.contains("command not found"));
        }

        #[test]
        fn empty_input_returns_nop() {
            let result = parse_cmd("");
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), RushError::Nop));
        }

        #[test]
        fn whitespace_only_returns_nop() {
            let result = parse_cmd("   ");
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), RushError::Nop));
        }

        #[test]
        fn io_error_propagates() {
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

            let result = Command::new(FailingReader);
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), RushError::UnexpectedEOF));
        }

        #[test]
        fn quoted_arguments_preserved() {
            let cmd = parse_cmd("echo \'hello world\'").unwrap();
            assert_eq!(cmd.args, vec!["echo", "hello world"]);
        }

        #[test]
        fn multiple_spaces_handled() {
            let cmd = parse_cmd("echo    hello    world").unwrap();
            assert_eq!(cmd.args, vec!["echo", "hello", "world"]);
        }
    }

    mod exit_command {
        use super::*;

        #[test]
        fn executes_successfully() {
            let cmd = parse_cmd("exit").unwrap();
            assert!(cmd.run().is_ok());
        }

        #[test]
        fn with_args_ignored() {
            let cmd = parse_cmd("exit 0").unwrap();
            assert!(cmd.run().is_ok());
            assert_eq!(cmd.args, vec!["exit", "0"]);
        }
    }
}
