use std::{env, path::Path};

use crate::{command::CommandType, util::RushError};

pub(crate) fn handle_cd(args: &[String]) -> Result<(), RushError> {
    // A helper function that attempts to cd to the HOME directory
    fn cd_home_dir() -> Result<(), RushError> {
        let home_dir = env::home_dir().ok_or_else(|| RushError::CommandError {
            type_: CommandType::Cd,
            msg: "failed to locate home directory".into(),
            status: Some(1),
        })?;

        env::set_current_dir(&Path::new(&home_dir)).map_err(|error| RushError::CommandError {
            type_: CommandType::Cd,
            msg: error.to_string(),
            status: error.raw_os_error(),
        })
    }

    if let Some(target_dir) = &args.get(1) {
        return match target_dir.as_str() {
            "~" => cd_home_dir(),
            target_dir => {
                return env::set_current_dir(&Path::new(target_dir)).map_err(|error| {
                    RushError::CommandError {
                        type_: CommandType::Cd,
                        msg: format!("{}: No such file or directory", target_dir),
                        status: error.raw_os_error(),
                    }
                });
            }
        };
    }

    cd_home_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use serial_test::serial;
    use std::io;

    // Test helper to simplify command creation
    fn parse_cmd(input: &str) -> Result<Command, RushError> {
        Command::new(io::Cursor::new(input))
    }

    #[test]
    fn parse_cd_command() {
        let cmd = parse_cmd("cd /tmp").unwrap();
        assert!(matches!(cmd.type_, CommandType::Cd));
        assert_eq!(cmd.args, vec!["cd", "/tmp"]);
    }

    #[test]
    #[serial]
    fn cd_to_absolute_path() {
        let original_dir = env::current_dir().unwrap();

        let cmd = parse_cmd("cd /tmp").unwrap();
        let result = cmd.run();
        let current = env::current_dir().unwrap();

        // Restore original directory before assertions
        env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        assert!(
            // On macOS, /tmp is a symlink to /private/tmp
            current == Path::new("/tmp") || current == Path::new("/private/tmp"),
            "Expected /tmp or /private/tmp, got {:?}",
            current
        );
    }

    #[test]
    #[serial]
    fn cd_to_root() {
        let original_dir = env::current_dir().unwrap();

        let cmd = parse_cmd("cd /").unwrap();
        let result = cmd.run();
        let current = env::current_dir().unwrap();

        // Restore original directory before assertions
        env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(current, Path::new("/"));
    }

    #[test]
    fn cd_to_nonexistent_directory() {
        let cmd = parse_cmd("cd /nonexistent_directory_12345").unwrap();
        let result = cmd.run();
        assert!(result.is_err());

        if let Err(RushError::CommandError { type_, msg, .. }) = result {
            assert!(matches!(type_, CommandType::Cd));
            assert!(msg.contains("No such file") || msg.contains("cannot find"));
        } else {
            panic!("Expected CommandError");
        }
    }

    #[test]
    fn cd_to_file_not_directory() {
        // Try to cd to /etc/hosts which is a file
        let cmd = parse_cmd("cd /etc/hosts").unwrap();
        let result = cmd.run();
        assert!(result.is_err());

        if let Err(RushError::CommandError { type_, .. }) = result {
            assert!(matches!(type_, CommandType::Cd));
        } else {
            panic!("Expected CommandError");
        }
    }

    #[test]
    #[serial]
    fn cd_with_no_arguments() {
        let original_dir = env::current_dir().unwrap();

        let cmd = parse_cmd("cd").unwrap();
        let result = cmd.run();
        let _current = env::current_dir().unwrap();

        // Restore original directory before assertions
        env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn cd_with_multiple_path_segments() {
        let original_dir = env::current_dir().unwrap();

        // Test with a path that has multiple segments
        let cmd = parse_cmd("cd /usr/local").unwrap();
        let result = cmd.run();

        // This might fail on some systems if /usr/local doesn't exist
        let current = if result.is_ok() {
            Some(env::current_dir().unwrap())
        } else {
            None
        };

        env::set_current_dir(&original_dir).unwrap();

        if let Some(current) = current {
            assert_eq!(current, Path::new("/usr/local"));
        }
    }

    #[test]
    #[serial]
    fn cd_preserves_trailing_slash() {
        let original_dir = env::current_dir().unwrap();

        let cmd = parse_cmd("cd /tmp/").unwrap();
        let result = cmd.run();

        // Should still change to /tmp even with trailing slash
        let current = env::current_dir().unwrap();

        env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        assert!(
            // On macOS, /tmp is a symlink to /private/tmp
            current == Path::new("/tmp") || current == Path::new("/private/tmp"),
            "Expected /tmp or /private/tmp, got {:?}",
            current
        );
    }

    #[test]
    fn cd_is_recognized_as_builtin() {
        use crate::command::path::is_builtin;
        assert!(is_builtin("cd"));
    }

    #[test]
    fn cd_command_type_display() {
        assert_eq!(CommandType::Cd.to_string(), "cd");
    }

    #[test]
    #[serial]
    fn cd_to_current_directory() {
        let original_dir = env::current_dir().unwrap();

        let cmd = parse_cmd("cd .").unwrap();
        let result = cmd.run();
        let current = env::current_dir().unwrap();

        env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(current, original_dir);
    }

    #[test]
    #[serial]
    fn cd_to_parent_directory() {
        let original_dir = env::current_dir().unwrap();

        // First cd to /tmp to have a known starting point
        env::set_current_dir("/tmp").unwrap();

        let cmd = parse_cmd("cd ..").unwrap();
        let result = cmd.run();
        let current = env::current_dir().unwrap();

        env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        // On macOS, /tmp is /private/tmp, so .. should be /private
        // On Linux, /tmp/.. should be /
        assert!(
            current == Path::new("/") || current == Path::new("/private"),
            "Expected / or /private, got {:?}",
            current
        );
    }

    #[test]
    #[serial]
    fn cd_to_grandparent_directory() {
        let original_dir = env::current_dir().unwrap();

        // Start from a known deep path
        if Path::new("/usr/local/bin").exists() {
            env::set_current_dir("/usr/local/bin").unwrap();

            let cmd = parse_cmd("cd ../..").unwrap();
            let result = cmd.run();
            let current = env::current_dir().unwrap();

            env::set_current_dir(&original_dir).unwrap();

            assert!(result.is_ok());
            assert_eq!(current, Path::new("/usr"));
        } else {
            env::set_current_dir(&original_dir).unwrap();
        }
    }

    #[test]
    #[serial]
    fn cd_to_relative_subdirectory() {
        let original_dir = env::current_dir().unwrap();

        // Change to /usr which should have a 'local' subdirectory
        if Path::new("/usr/local").exists() {
            env::set_current_dir("/usr").unwrap();

            let cmd = parse_cmd("cd local").unwrap();
            let result = cmd.run();
            let current = env::current_dir().unwrap();

            env::set_current_dir(&original_dir).unwrap();

            assert!(result.is_ok());
            assert_eq!(current, Path::new("/usr/local"));
        } else {
            env::set_current_dir(&original_dir).unwrap();
        }
    }

    #[test]
    #[serial]
    fn cd_to_relative_path_with_current_dir() {
        let original_dir = env::current_dir().unwrap();

        // Change to /usr which should have a 'local' subdirectory
        if Path::new("/usr/local").exists() {
            env::set_current_dir("/usr").unwrap();

            let cmd = parse_cmd("cd ./local").unwrap();
            let result = cmd.run();
            let current = env::current_dir().unwrap();

            env::set_current_dir(&original_dir).unwrap();

            assert!(result.is_ok());
            assert_eq!(current, Path::new("/usr/local"));
        } else {
            env::set_current_dir(&original_dir).unwrap();
        }
    }

    #[test]
    #[serial]
    fn cd_to_complex_relative_path() {
        let original_dir = env::current_dir().unwrap();

        // Test navigating up and then down: ../sibling pattern
        if Path::new("/usr/local").exists() && Path::new("/usr/bin").exists() {
            env::set_current_dir("/usr/local").unwrap();

            let cmd = parse_cmd("cd ../bin").unwrap();
            let result = cmd.run();
            let current = env::current_dir().unwrap();

            env::set_current_dir(&original_dir).unwrap();

            assert!(result.is_ok());
            assert_eq!(current, Path::new("/usr/bin"));
        } else {
            env::set_current_dir(&original_dir).unwrap();
        }
    }

    #[test]
    #[serial]
    fn cd_to_nonexistent_relative_path() {
        let original_dir = env::current_dir().unwrap();

        let cmd = parse_cmd("cd ./nonexistent_subdir_12345").unwrap();
        let result = cmd.run();

        env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_err());
        if let Err(RushError::CommandError { type_, msg, .. }) = result {
            assert!(matches!(type_, CommandType::Cd));
            assert!(msg.contains("No such file") || msg.contains("cannot find"));
        } else {
            panic!("Expected CommandError");
        }
    }

    #[test]
    #[serial]
    fn cd_parent_from_root() {
        let original_dir = env::current_dir().unwrap();

        // cd to root first
        env::set_current_dir("/").unwrap();

        // Try to go to parent of root (should stay at root)
        let cmd = parse_cmd("cd ..").unwrap();
        let result = cmd.run();
        let current = env::current_dir().unwrap();

        env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(current, Path::new("/"));
    }

    #[test]
    #[serial]
    fn cd_to_deeply_nested_relative_path() {
        let original_dir = env::current_dir().unwrap();

        // Test ../../.. navigation
        if Path::new("/usr/local/bin").exists() {
            env::set_current_dir("/usr/local/bin").unwrap();

            let cmd = parse_cmd("cd ../../..").unwrap();
            let result = cmd.run();
            let current = env::current_dir().unwrap();

            env::set_current_dir(&original_dir).unwrap();

            assert!(result.is_ok());
            assert_eq!(current, Path::new("/"));
        } else {
            env::set_current_dir(&original_dir).unwrap();
        }
    }

    #[test]
    #[serial]
    fn cd_to_relative_path_multiple_segments() {
        let original_dir = env::current_dir().unwrap();

        // Navigate to a multi-segment relative path
        if Path::new("/usr").exists() {
            env::set_current_dir("/usr").unwrap();

            if Path::new("/usr/local/bin").exists() {
                let cmd = parse_cmd("cd local/bin").unwrap();
                let result = cmd.run();
                let current = env::current_dir().unwrap();

                env::set_current_dir(&original_dir).unwrap();

                assert!(result.is_ok());
                assert_eq!(current, Path::new("/usr/local/bin"));
            } else {
                env::set_current_dir(&original_dir).unwrap();
            }
        } else {
            env::set_current_dir(&original_dir).unwrap();
        }
    }

    #[test]
    #[serial]
    fn cd_to_home_with_tilde() {
        let original_dir = env::current_dir().unwrap();

        let cmd = parse_cmd("cd ~").unwrap();
        let result = cmd.run();
        let current = env::current_dir().unwrap();

        env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());

        // Verify we're in the home directory
        if let Some(home) = env::home_dir() {
            assert_eq!(current, home);
        }
    }

    #[test]
    #[serial]
    fn cd_with_no_args_goes_to_home() {
        let original_dir = env::current_dir().unwrap();

        // First cd somewhere else
        env::set_current_dir("/tmp").unwrap();

        let cmd = parse_cmd("cd").unwrap();
        let result = cmd.run();
        let current = env::current_dir().unwrap();

        env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());

        // Verify we're in the home directory
        if let Some(home) = env::home_dir() {
            assert_eq!(current, home);
        }
    }

    #[test]
    fn cd_tilde_parsing() {
        let cmd = parse_cmd("cd ~").unwrap();
        assert!(matches!(cmd.type_, CommandType::Cd));
        assert_eq!(cmd.args, vec!["cd", "~"]);
    }

    #[test]
    #[serial]
    fn cd_to_home_from_different_directory() {
        let original_dir = env::current_dir().unwrap();

        // Start from a known directory
        env::set_current_dir("/").unwrap();

        let cmd = parse_cmd("cd ~").unwrap();
        let result = cmd.run();
        let current = env::current_dir().unwrap();

        env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());

        // Verify we changed from / to home
        if let Some(home) = env::home_dir() {
            assert_eq!(current, home);
            assert_ne!(current, Path::new("/"));
        }
    }

    #[test]
    #[serial]
    fn cd_tilde_multiple_times() {
        let original_dir = env::current_dir().unwrap();

        // cd ~ should work multiple times
        for _ in 0..3 {
            let cmd = parse_cmd("cd ~").unwrap();
            let result = cmd.run();
            assert!(result.is_ok());

            if let Some(home) = env::home_dir() {
                assert_eq!(env::current_dir().unwrap(), home);
            }
        }

        env::set_current_dir(&original_dir).unwrap();
    }
}
