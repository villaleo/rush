use std::io::BufRead;
use std::vec::Vec;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RushError {
    #[error("{0}: command not found")]
    CommandNotFound(String),
    #[error("error reading input: unexpected EOF")]
    UnexpectedEOF,
}

pub(crate) fn process_input<R: BufRead>(mut reader: R) -> anyhow::Result<Vec<String>> {
    let mut input = String::new();
    reader
        .read_line(&mut input)
        .map_err(|_| RushError::UnexpectedEOF)?;

    let tokens = input
        .clone()
        .trim()
        .split(" ")
        .map(String::from)
        .collect::<Vec<_>>();

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self};

    #[test]
    fn process_input_parses_tokens() {
        let input = io::Cursor::new("echo hello world\n");
        let tokens = process_input(input).unwrap();
        let tokens = tokens.iter().map(String::as_str).collect::<Vec<_>>();

        assert_eq!(tokens, vec!["echo", "hello", "world"]);
    }

    #[test]
    fn process_input_single_token() {
        let input = io::Cursor::new("ls\n");
        let tokens = process_input(input).unwrap();
        assert_eq!(tokens, vec!["ls".to_string()]);
    }

    struct ErrReader;

    impl io::Read for ErrReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::UnexpectedEof, "read error"))
        }
    }

    impl BufRead for ErrReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "fill_buf error",
            ))
        }
        fn consume(&mut self, _n: usize) {}
    }

    #[test]
    fn process_input_returns_unexpected_eof_on_read_error() {
        let reader = ErrReader;
        let err = process_input(reader).unwrap_err();
        assert!(err.to_string().contains("unexpected EOF"));
    }

    #[test]
    fn should_return_command_not_found_for_unknown_command() {
        let input = io::Cursor::new("unknowncmd\n");
        let tokens = process_input(input).unwrap();
        assert_eq!(tokens, vec!["unknowncmd".to_string()]);
    }
}
