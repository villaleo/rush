use std::{io, process};

use crate::{command::CommandType, util::RushError};

pub(crate) fn handle_executable(
    path: &str,
    name: &str,
    args: &[String],
) -> Result<Option<i32>, RushError> {
    let into_rush_err = |error: io::Error| RushError::CommandError {
        type_: CommandType::Executable {
            path: path.into(),
            name: name.into(),
        },
        msg: error.to_string(),
        status: error.raw_os_error(),
    };

    let mut child = process::Command::new(name)
        .args(&args[1..])
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()
        .map_err(into_rush_err)?;

    // Take ownership of stdout and stderr
    let mut child_stdout = child.stdout.take().expect("stdout was piped");
    let mut child_stderr = child.stderr.take().expect("stderr was piped");

    // Spawn threads to copy output in parallel
    use std::thread;
    let stdout_thread = thread::spawn(move || io::copy(&mut child_stdout, &mut io::stdout()));
    let stderr_thread = thread::spawn(move || io::copy(&mut child_stderr, &mut io::stderr()));

    let status = child.wait().map_err(into_rush_err)?;

    // Wait for output threads to finish
    stdout_thread
        .join()
        .expect("stdout thread panicked")
        .map_err(into_rush_err)?;
    stderr_thread
        .join()
        .expect("stderr thread panicked")
        .map_err(into_rush_err)?;

    if status.success() {
        return Ok(status.code());
    }

    Err(RushError::CommandError {
        type_: CommandType::Executable {
            path: path.into(),
            name: name.into(),
        },
        msg: match status.code() {
            Some(code) => format!("process exited with code {}", code),
            None => "process terminated by signal".into(),
        },
        status: status.code(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::util::RushError;
    use std::{env, io};

    use crate::command::path::find_in_path;

    // Test helper to simplify command creation
    fn parse_cmd(input: &str) -> Result<Command, RushError> {
        Command::new(io::Cursor::new(input))
    }

    // Helper to create a Command with an executable type
    fn create_executable_command(path: &str, args: Vec<String>) -> Command {
        Command {
            type_: CommandType::Executable {
                path: path.to_string(),
                name: args[0].clone(),
            },
            args,
        }
    }

    #[test]
    fn test_successful_execution() {
        // Use 'true' command which always exits with 0
        let cmd = create_executable_command("/usr/bin/true", vec!["true".to_string()]);

        let result = cmd.handle_executable("/usr/bin/true", "true");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(0));
    }

    #[test]
    fn test_failed_execution() {
        // Use 'false' command which always exits with 1
        let cmd = create_executable_command("/usr/bin/false", vec!["false".to_string()]);

        let result = cmd.handle_executable("/usr/bin/false", "false");
        assert!(result.is_err());

        if let Err(RushError::CommandError { status, .. }) = result {
            assert_eq!(status, Some(1));
        } else {
            panic!("Expected CommandError");
        }
    }

    #[test]
    fn test_nonexistent_executable() {
        let cmd =
            create_executable_command("/nonexistent/path/to/binary", vec!["binary".to_string()]);

        let result = cmd.handle_executable("/nonexistent/path/to/binary", "binary");
        assert!(result.is_err());

        if let Err(RushError::CommandError { msg, .. }) = result {
            assert!(msg.contains("No such file") || msg.contains("cannot find"));
        } else {
            panic!("Expected CommandError");
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_permission_denied() {
        // Try to execute a file without execute permissions
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let temp_file = "/tmp/rush_test_no_exec";
        fs::write(temp_file, "#!/bin/sh\necho test").unwrap();

        // Set permissions to read-only
        let mut perms = fs::metadata(temp_file).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(temp_file, perms).unwrap();

        let cmd = create_executable_command(temp_file, vec!["rush_test_no_exec".to_string()]);

        let result = cmd.handle_executable(temp_file, "rush_test_no_exec");
        assert!(result.is_err());

        // Cleanup
        fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_exit_code_propagation() {
        // Use sh to exit with a specific code
        let cmd = create_executable_command(
            "/bin/sh",
            vec!["sh".to_string(), "-c".to_string(), "exit 42".to_string()],
        );

        let result = cmd.handle_executable("/bin/sh", "sh");
        assert!(result.is_err());

        if let Err(RushError::CommandError { status, .. }) = result {
            assert_eq!(status, Some(42));
        } else {
            panic!("Expected CommandError with exit code 42");
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_signal_termination() {
        if env::var_os("PATH").is_some() {
            if let Ok(Some(ref shell_path)) = find_in_path("sh") {
                let cmd = create_executable_command(
                    shell_path,
                    vec!["sh".to_string(), "-c".to_string(), "kill -9 $$".to_string()],
                );

                let result = cmd.handle_executable(shell_path, "sh");
                assert!(result.is_err());

                if let Err(RushError::CommandError { status, msg, .. }) = result {
                    // When killed by signal, exit code is None
                    assert_eq!(status, None);
                    assert!(msg.contains("signal") || msg.contains("terminated"));
                } else {
                    panic!("Expected CommandError from signal");
                }
            }
        }
    }

    #[test]
    fn test_integration_parse_and_run_executable() {
        if env::var_os("PATH").is_some() {
            let cmd = parse_cmd("true").unwrap();
            assert!(matches!(cmd.type_, CommandType::Executable { .. }));

            let result = cmd.run();
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_integration_executable_with_arguments() {
        if env::var_os("PATH").is_some() {
            // Use 'echo' from PATH (not the builtin, but /bin/echo)
            if let Ok(Some(echo_path)) = find_in_path("echo") {
                // Skip this test if echo is not found as a separate executable
                if echo_path.starts_with("/") {
                    let input = format!("{} hello world", echo_path);
                    let cmd = parse_cmd(&input).unwrap();

                    if let CommandType::Executable { ref name, .. } = cmd.type_ {
                        assert!(name.contains("echo"));
                        assert_eq!(cmd.args.len(), 3); // echo, hello, world
                    } else {
                        panic!("Expected Executable type");
                    }

                    let result = cmd.run();
                    assert!(result.is_ok());
                }
            }
        }
    }

    #[test]
    fn test_integration_executable_not_in_path() {
        let result = parse_cmd("definitely_nonexistent_command_831");
        assert!(result.is_err());

        if let Err(RushError::CommandNotFound(name)) = result {
            assert_eq!(name, "definitely_nonexistent_command_831");
        } else {
            panic!("Expected CommandNotFound error");
        }
    }
}
