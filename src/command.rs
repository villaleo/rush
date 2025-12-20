use std::{
    env::{self, split_paths},
    fmt,
    io::BufRead,
    path::Path,
    str::FromStr,
};

use crate::util::{RushError, tokenize};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum CommandType {
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
pub(crate) struct Command {
    pub type_: CommandType,
    pub args: Vec<String>,
}

impl Command {
    pub(crate) fn new<R: BufRead>(reader: R) -> Result<Command, RushError> {
        let args = tokenize(reader)?;

        // Read the name of the command from the tokenized args
        let Some(name) = args.first() else {
            return Err(RushError::Nop);
        };

        let type_ = CommandType::from_str(name)?;
        match type_ {
            CommandType::Unknown(cmd) => Err(RushError::CommandNotFound(cmd)),
            _ => Ok(Command { type_, args }),
        }
    }

    pub(crate) fn run(&self) -> Result<(), RushError> {
        match self.type_ {
            CommandType::Echo => self.handle_echo(),
            CommandType::Exit => Ok(()),
            CommandType::Type => self.handle_type(),
            CommandType::Unknown(ref cmd_name) => self.handle_unknown_cmd(cmd_name),
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

    fn handle_type(&self) -> Result<(), RushError> {
        let Some(cmd_name) = self.args.get(1) else {
            return Err(RushError::CommandError {
                type_: CommandType::Type,
                msg: "missing argument".into(),
            });
        };

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
    matches!(
        CommandType::from_str(cmd_name),
        Ok(CommandType::Echo | CommandType::Exit | CommandType::Type)
    )
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
            let result = CommandType::from_str("echo");
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), CommandType::Echo));
        }

        #[test]
        fn parse_exit() {
            let result = CommandType::from_str("exit");
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), CommandType::Exit));
        }

        #[test]
        fn parse_type() {
            let result = CommandType::from_str("type");
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), CommandType::Type));
        }

        #[test]
        fn parse_unknown_wraps_in_variant() {
            let result = CommandType::from_str("nonexistent");
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), CommandType::Unknown(_)));
        }

        #[test]
        fn display_formatting() {
            assert_eq!(CommandType::Echo.to_string(), "echo");
            assert_eq!(CommandType::Exit.to_string(), "exit");
            assert_eq!(CommandType::Type.to_string(), "type");
            assert_eq!(CommandType::Unknown("custom".into()).to_string(), "custom");
        }

        #[test]
        fn whitespace_trimmed() {
            assert!(matches!(
                CommandType::from_str("  echo  ").unwrap(),
                CommandType::Echo
            ));
            assert!(matches!(
                CommandType::from_str("\texit\n").unwrap(),
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
            let cmd = parse_cmd("echo \"hello world\"").unwrap();
            assert_eq!(cmd.args, vec!["echo", "hello world"]);
        }

        #[test]
        fn multiple_spaces_handled() {
            let cmd = parse_cmd("echo    hello    world").unwrap();
            assert_eq!(cmd.args, vec!["echo", "hello", "world"]);
        }
    }

    mod echo_command {
        use super::*;

        #[test]
        fn no_args() {
            let cmd = parse_cmd("echo").unwrap();
            assert!(cmd.run().is_ok());
        }

        #[test]
        fn single_arg() {
            let cmd = parse_cmd("echo hello").unwrap();
            assert!(cmd.run().is_ok());
        }

        #[test]
        fn multiple_args() {
            let cmd = parse_cmd("echo hello world test").unwrap();
            assert!(cmd.run().is_ok());
        }

        #[test]
        fn quoted_args() {
            let cmd = parse_cmd("echo \"hello world\" test").unwrap();
            assert!(cmd.run().is_ok());
            assert_eq!(cmd.args, vec!["echo", "hello world", "test"]);
        }

        #[test]
        fn empty_quoted_string() {
            let cmd = parse_cmd("echo \"\"").unwrap();
            assert!(cmd.run().is_ok());
            assert_eq!(cmd.args, vec!["echo", ""]);
        }

        #[test]
        fn special_characters() {
            let cmd = parse_cmd("echo !@#$%^&*()").unwrap();
            assert!(cmd.run().is_ok());
        }

        #[test]
        fn numbers() {
            let cmd = parse_cmd("echo 123 456").unwrap();
            assert!(cmd.run().is_ok());
            assert_eq!(cmd.args, vec!["echo", "123", "456"]);
        }

        #[test]
        fn with_leading_trailing_spaces() {
            let cmd = parse_cmd("   echo   hello   ").unwrap();
            assert!(cmd.run().is_ok());
            assert_eq!(cmd.args, vec!["echo", "hello"]);
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

    mod type_command {
        use super::*;

        #[test]
        fn builtin_echo() {
            let cmd = parse_cmd("type echo").unwrap();
            assert!(cmd.run().is_ok());
        }

        #[test]
        fn builtin_exit() {
            let cmd = parse_cmd("type exit").unwrap();
            assert!(cmd.run().is_ok());
        }

        #[test]
        fn builtin_type_itself() {
            let cmd = parse_cmd("type type").unwrap();
            assert!(cmd.run().is_ok());
        }

        #[test]
        fn no_args_fails() {
            let cmd = parse_cmd("type").unwrap();
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
        fn no_args_error_message() {
            let cmd = parse_cmd("type").unwrap();
            let error = cmd.run().unwrap_err();
            assert!(error.to_string().contains("missing argument"));
        }

        #[test]
        fn unknown_command_fails() {
            let cmd = parse_cmd("type nonexistent").unwrap();
            let result = cmd.run();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[test]
        fn unknown_command_error_contains_name() {
            let cmd = parse_cmd("type nonexistent123").unwrap();
            let error = cmd.run().unwrap_err();
            let error_msg = error.to_string();
            assert!(error_msg.contains("nonexistent123"));
        }

        #[test]
        fn path_command_ls_found_when_path_set() {
            // Test with 'ls' which should exist on macOS/Unix
            if env::var_os("PATH").is_some() {
                let cmd = parse_cmd("type ls").unwrap();
                let result = cmd.run();
                assert!(result.is_ok());
            }
        }

        #[test]
        fn multiple_args_uses_first() {
            let cmd = parse_cmd("type echo exit").unwrap();
            assert!(cmd.run().is_ok());
            assert_eq!(cmd.args, vec!["type", "echo", "exit"]);
        }
    }

    mod path_utilities {
        use super::*;

        #[test]
        fn is_builtin_recognizes_commands() {
            assert!(is_builtin("echo"));
            assert!(is_builtin("exit"));
            assert!(is_builtin("type"));
            assert!(!is_builtin("nonexistent"));
            assert!(!is_builtin("ls"));
            assert!(!is_builtin("grep"));
        }

        #[test]
        fn is_builtin_with_whitespace() {
            assert!(is_builtin(" echo "));
            assert!(is_builtin("\texit"));
        }

        #[test]
        fn find_in_path_returns_none_for_nonexistent() {
            let result = find_in_path("definitely_does_not_exist_12345");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
        }

        #[test]
        fn find_in_path_finds_ls_on_unix() {
            if env::var_os("PATH").is_some() {
                let result = find_in_path("ls");
                assert!(result.is_ok());
                assert!(result.unwrap().is_some());
            }
        }
    }
}
