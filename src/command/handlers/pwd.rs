use std::env;

use crate::{command::CommandType, util::RushError};

pub(crate) fn handle_pwd(_args: &[String]) -> Result<(), RushError> {
    let cwd = env::current_dir().map_err(|error| RushError::CommandError {
        type_: CommandType::Pwd,
        msg: error.to_string(),
        status: error.raw_os_error(),
    })?;
    println!("{}", cwd.display());
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
    fn executes_successfully() {
        let cmd = parse_cmd("pwd").unwrap();
        assert!(cmd.run().is_ok());
    }
}
