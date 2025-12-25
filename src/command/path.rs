use std::{env, path::Path};

use crate::{command::CommandType, util::RushError};

#[cfg(unix)]
pub(crate) fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
pub(crate) fn is_executable(_path: &Path) -> bool {
    true // On non-Unix, just check existence
}

pub(crate) fn is_builtin(cmd_name: &str) -> bool {
    matches!(
        CommandType::from_str(cmd_name),
        CommandType::Cd
            | CommandType::Echo
            | CommandType::Exit
            | CommandType::Pwd
            | CommandType::Type
    )
}

pub(crate) fn find_in_path(cmd_name: &str) -> Result<Option<String>, RushError> {
    let path_env = match env::var_os("PATH") {
        Some(path) => path,
        None => return Ok(None),
    };

    for dir in env::split_paths(&path_env) {
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

    #[test]
    fn is_builtin_recognizes_commands() {
        assert!(is_builtin("cd"));
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
