use std::io::{self, BufRead};

pub fn process_input<R: BufRead>(mut reader: R) -> io::Result<String> {
    let mut input = String::new();
    reader.read_line(&mut input).unwrap();

    Ok(input.trim().into())
}

pub fn read_command<R: BufRead>(reader: R) -> String {
    match process_input(reader) {
        Ok(input) => input,
        Err(_) => unreachable!(),
    }
}

pub fn find_command(name: &str) -> Option<()> {
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
        let cmd_name = read_command(reader);

        assert_eq!(input, cmd_name);
    }

    #[test]
    fn should_find_command() {
        let cmd_name = "go";
        let cmd = find_command(cmd_name);

        // All commands are unknown for now
        assert!(cmd.is_none());
    }
}
