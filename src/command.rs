use std::{
    env::{self, split_paths},
    fmt,
    io::BufRead,
    path::Path,
    str::FromStr,
};

use crate::util::{RushError, tokenize};

#[derive(Debug, Eq, PartialEq)]
pub enum CommandType {
    Echo,
    Exit,
    Type,
    Unknown(String),
}

impl fmt::Display for CommandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandType::Echo => write!(f, "echo"),
            CommandType::Exit => write!(f, "exit"),
            CommandType::Type => write!(f, "type"),
            CommandType::Unknown(cmd) => write!(f, "{}", cmd),
        }
    }
}

#[derive(Debug)]
pub struct Command {
    pub type_: CommandType,
    pub args: Vec<String>,
}

impl Command {
    pub fn new<R: BufRead>(reader: R) -> Result<Command, RushError> {
        let args = tokenize(reader)?;

        // Read the name of the command from the tokenized args
        let Some(name) = args.first() else {
            return Err(RushError::Nop);
        };

        let type_ = CommandType::from_str(&name)?;
        match type_ {
            CommandType::Unknown(cmd) => Err(RushError::CommandNotFound(cmd)),
            _ => Ok(Command { type_, args }),
        }
    }

    pub fn run(&self) -> Result<(), RushError> {
        match self.type_ {
            CommandType::Echo => self.handle_echo(),
            CommandType::Exit => Ok(()),
            CommandType::Type => {
                if self.args.len() == 1 {
                    return Err(RushError::CommandError {
                        type_: CommandType::Type,
                        msg: "missing argument".into(),
                    });
                }

                let Some(cmd_name) = self.args.get(1) else {
                    unreachable!();
                };

                self.handle_type(cmd_name)
            }
            CommandType::Unknown(ref unknown) => self.handle_unknown_cmd(unknown),
        }
    }

    fn handle_echo(&self) -> Result<(), RushError> {
        // Skip the first argument (command name)
        let tokens = &self.args[1..];

        if tokens.is_empty() {
            return Ok(());
        }

        println!("{}", tokens.join(" "));
        Ok(())
    }

    fn handle_type(&self, cmd_name: &str) -> Result<(), RushError> {
        if is_builtin(cmd_name) {
            println!("{cmd_name} is a shell builtin");
            return Ok(());
        }

        match find_in_path(cmd_name)? {
            Some(path) => {
                println!("{} is {}", cmd_name, path);
                Ok(())
            }
            None => Err(RushError::CommandError {
                type_: CommandType::Unknown(cmd_name.into()),
                msg: "not found".into(),
            }),
        }
    }

    fn handle_unknown_cmd(&self, cmd: &str) -> Result<(), RushError> {
        Err(RushError::CommandNotFound(cmd.into()))
    }
}

impl FromStr for CommandType {
    type Err = RushError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "exit" => Ok(CommandType::Exit),
            "echo" => Ok(CommandType::Echo),
            "type" => Ok(CommandType::Type),
            unknown => Ok(CommandType::Unknown(unknown.to_string())),
        }
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    true // On non-Unix, just check existence
}

fn is_builtin(cmd_name: &str) -> bool {
    let Ok(type_) = CommandType::from_str(cmd_name) else {
        unreachable!()
    };

    match type_ {
        CommandType::Unknown(_) => false,
        _ => true,
    }
}

fn find_in_path(cmd_name: &str) -> Result<Option<String>, RushError> {
    let path_env = match env::var_os("PATH") {
        Some(path) => path,
        None => return Ok(None),
    };

    for dir in split_paths(&path_env) {
        let full_path = Path::new(&dir).join(cmd_name);

        // Check if file exists and is executable
        if full_path.exists() && is_executable(&full_path) {
            return Ok(Some(full_path.to_string_lossy().to_string()));
        }
    }

    Ok(None)
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
    fn should_find_command_type() {
        let cmd_name = String::from("go");
        let cmd = CommandType::from_str(&cmd_name);

        assert!(cmd.is_ok());
        assert!(matches!(cmd.unwrap(), CommandType::Unknown(_)));
    }

    #[test]
    fn should_return_command_not_found_for_unknown_command() {
        let input = "nonexistent";
        let reader = io::Cursor::new(input);

        let cmd = Command::new(reader);
        assert!(cmd.is_err());
        assert!(matches!(cmd.unwrap_err(), RushError::CommandNotFound(_)));
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
        assert!(matches!(result.unwrap_err(), RushError::UnexpectedEOF));
    }

    #[test]
    fn should_parse_command_with_multiple_arguments() {
        let input = "echo hello world foo";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).unwrap();

        assert_eq!(cmd.args, vec!["echo", "hello", "world", "foo"]);
        assert!(matches!(cmd.type_, CommandType::Echo));
    }

    #[test]
    fn should_identify_echo_command_type() {
        let result = CommandType::from_str("echo");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), CommandType::Echo));
    }

    #[test]
    fn should_identify_exit_command_type() {
        let result = CommandType::from_str("exit");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), CommandType::Exit));
    }

    #[test]
    fn should_identify_type_command_type() {
        let result = CommandType::from_str("type");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), CommandType::Type));
    }

    #[test]
    fn should_print_all_arguments_for_echo_command() {
        let input = "echo hello world";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).expect("expected command 'echo'");

        // note: can't easily capture stdout in a unit test,
        // but we can verify the command runs without error
        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_exit_command_should_succeed() {
        let input = "exit";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).expect("expected command 'exit'");

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_echo_with_no_args_should_succeed() {
        let input = "echo";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).expect("expected command 'echo'");

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_echo_with_single_arg_should_succeed() {
        let input = "echo hello";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).expect("expected command 'echo'");

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_echo_with_multiple_args_should_succeed() {
        let input = "echo hello world test";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).expect("expected command 'echo'");

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_type_with_builtin_should_succeed() {
        let input = "type echo";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).expect("expected command 'type'");

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_type_with_exit_builtin_should_succeed() {
        let input = "type exit";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).expect("expected command 'type'");

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_type_with_type_builtin_should_succeed() {
        let input = "type type";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).expect("expected command 'type'");

        let result = cmd.run();
        assert!(result.is_ok());
    }

    #[test]
    fn run_type_with_unknown_command_should_fail() {
        let input = "type nonexistent";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).expect("expected command 'type'");

        let result = cmd.run();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn run_type_with_no_args_should_fail() {
        let input = "type";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader).expect("expected command 'type'");

        let result = cmd.run();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RushError::CommandError {
                type_: CommandType::Type,
                msg: _
            }
        ));
    }

    #[test]
    fn run_unknown_command_should_fail_with_command_not_found() {
        let input = "unknowncmd";
        let reader = io::Cursor::new(input);

        let cmd = Command::new(reader);
        assert!(cmd.is_err());
        assert!(matches!(cmd.unwrap_err(), RushError::CommandNotFound(_)));
    }

    #[test]
    fn run_unknown_command_error_contains_command_name() {
        let input = "mycustomcmd";
        let reader = io::Cursor::new(input);

        let cmd = Command::new(reader);
        assert!(cmd.is_err());

        let error_str = cmd.unwrap_err().to_string();
        assert!(error_str.contains(input));
        assert!(error_str.contains("command not found"));
    }
}
