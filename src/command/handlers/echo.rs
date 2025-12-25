use crate::util::RushError;

pub(crate) fn handle_echo(args: &[String]) -> Result<(), RushError> {
    // Skip the first argument (command name)
    let tokens = &args[1..];

    if tokens.is_empty() {
        return Ok(());
    }

    println!("{}", tokens.join(" "));
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::command::Command;
    use crate::util::RushError;
    use std::io;

    // Test helper to simplify command creation
    fn parse_cmd(input: &str) -> Result<Command, RushError> {
        Command::new(io::Cursor::new(input))
    }

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
