use std::io::{self, BufRead};

#[derive(Debug, Eq, PartialEq)]
pub enum CommandType {
    Unknown,
}

pub struct Command {
    #[allow(dead_code)]
    pub type_: CommandType,
    pub name: String,
}

pub fn process_input<R: BufRead>(mut reader: R) -> io::Result<String> {
    let mut input = String::new();
    reader.read_line(&mut input).unwrap();

    Ok(input.trim().into())
}

pub fn read_command<R: BufRead>(reader: R) -> Command {
    match process_input(reader) {
        Ok(input) => Command {
            type_: CommandType::Unknown,
            name: input,
        },
        Err(_) => unreachable!(),
    }
}

pub fn find_command(name: &str) -> Option<Command> {
    // All commands are unknown for now
    println!("{name}: command not found");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_command() {
        let input = "go";
        let reader = io::Cursor::new(input);
        let cmd = read_command(reader);

        assert_eq!(input, cmd.name);
        assert_eq!(CommandType::Unknown, cmd.type_);
    }

    #[test]
    fn should_find_command() {
        let cmd_name = "go";
        let cmd = find_command(cmd_name);

        // All commands are unknown for now
        assert!(cmd.is_none());
    }
}
