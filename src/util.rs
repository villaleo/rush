use std::io::{self, BufRead};

#[derive(Debug, Eq, PartialEq)]
pub enum CommandType {
    Exit,
    Unknown,
}

pub struct Command {
    pub type_: CommandType,
    pub cmd_str: String,
}

impl Command {
    pub fn new<R: BufRead>(reader: R) -> anyhow::Result<Command> {
        let cmd = match process_input(reader) {
            Ok(cmd_line) => match Self::find(&cmd_line) {
                Some(cmd) => cmd,
                None => {
                    return Ok(Command {
                        type_: CommandType::Unknown,
                        cmd_str: cmd_line,
                    });
                }
            },
            Err(_) => unreachable!(),
        };

        Ok(cmd)
    }

    pub fn find(name: &str) -> Option<Command> {
        if name.trim() == "exit" {
            return Some(Command {
                type_: CommandType::Exit,
                cmd_str: name.trim().into(),
            });
        }

        // All commands are unknown for now
        None
    }
}

fn process_input<R: BufRead>(mut reader: R) -> io::Result<String> {
    let mut input = String::new();
    reader.read_line(&mut input).unwrap();

    Ok(input.trim().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_exit_on_find_exit_cmd() {
        let input = "exit";
        let cmd = Command::find(input);

        assert!(cmd.as_ref().is_some());
        assert_eq!(cmd.as_ref().unwrap().cmd_str, input);
        assert_eq!(cmd.as_ref().unwrap().type_, CommandType::Exit);
    }

    #[test]
    fn should_read_command() {
        let input = "go";
        let reader = io::Cursor::new(input);
        let cmd = Command::new(reader);

        assert!(&cmd.is_ok());
        assert_eq!(CommandType::Unknown, cmd.as_ref().unwrap().type_);
        assert_eq!(input, cmd.as_ref().unwrap().cmd_str);
    }

    #[test]
    fn should_find_command() {
        let cmd_name = "go";
        let cmd = Command::find(cmd_name);

        // All commands are unknown for now
        assert!(cmd.is_none());
    }
}
