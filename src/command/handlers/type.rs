use crate::{
    command::{path::{find_in_path, is_builtin}, CommandType},
    util::RushError,
};

pub(crate) fn handle_type(args: &[String]) -> Result<(), RushError> {
    let Some(cmd_name) = args.get(1) else {
        return Err(RushError::CommandError {
            type_: CommandType::Type,
            msg: "missing argument".into(),
            status: Some(1),
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
            status: Some(1),
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::command::Command;
    use crate::util::RushError;
    use std::{env, io};

    use crate::command::CommandType;

    // Test helper to simplify command creation
    fn parse_cmd(input: &str) -> Result<Command, RushError> {
        Command::new(io::Cursor::new(input))
    }

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
                msg: _m,
                status: Some(1)
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
