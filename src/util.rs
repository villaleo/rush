use std::io::BufRead;

use std::str::FromStr;
use std::vec::Vec;

use anyhow::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RushError {
    #[error("{0}: command not found")]
    CommandNotFound(String),
    #[error("error reading input: unexpected EOF")]
    UnexpectedEOF,
    #[error("{0}")]
    InternalError(Error),
    #[error("error: unterminated quote")]
    UnterminatedQuote,
}

pub(crate) fn process_input<R: BufRead>(mut reader: R) -> Result<Vec<String>, RushError> {
    let mut input = String::new();
    reader
        .read_line(&mut input)
        .map_err(|_| RushError::UnexpectedEOF)?;

    let Ok(ref input_tokens) = String::from_str(input.trim());

    let buf = &mut String::new();
    let mut tokens = Vec::<String>::new();
    let mut quote_count = 0;

    for (i, char) in input_tokens.chars().enumerate() {
        match char {
            '"' => {
                quote_count += 1;

                // Push buf to tokens when more than 1 quote is found
                if quote_count > 1 {
                    tokens.push(buf.clone());
                    buf.clear();
                    quote_count = 0;
                }
            }
            char => {
                // At the end, an odd num of quotes means a quote wasn't terminated
                if i == input_tokens.len() - 1 && quote_count % 2 == 1 {
                    return Err(RushError::UnterminatedQuote);
                }

                // If we haven't seen a quote yet and we encounter a space, push buf
                // into tokens and clear buf
                if quote_count == 0 && char == ' ' {
                    // Skip over empty tokens
                    if buf.trim().len() == 0 {
                        continue;
                    }

                    tokens.push(buf.trim().to_string());
                    buf.clear();
                    continue;
                }

                // Push the current char into buf
                buf.push_str(&format!("{}", char));

                // At the end, push any remaining chars into tokens
                if i == input_tokens.len() - 1 && buf.len() > 0 {
                    tokens.push(buf.clone());
                }
            }
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self};

    #[test]
    fn process_input_parses_tokens() {
        let input = io::Cursor::new("echo hello world\n");

        let tokens = process_input(input);
        assert!(tokens.is_ok());
        assert_eq!(tokens.unwrap(), vec!["echo", "hello", "world"]);
    }

    #[test]
    fn process_input_parses_quoted_tokens() {
        let input = io::Cursor::new("echo \"hello world\"\n");

        let tokens = process_input(input);
        assert!(tokens.is_ok());
        assert_eq!(tokens.unwrap(), vec!["echo", "hello world"]);
    }

    #[test]
    fn process_input_err_on_unterminated_quoted() {
        let input = io::Cursor::new("echo \"hello world\n");

        let tokens = process_input(input);
        assert!(tokens.is_err());
        assert!(matches!(tokens.unwrap_err(), RushError::UnterminatedQuote));
    }

    #[test]
    fn process_input_parses_repeated_spaces_ok() {
        let input = io::Cursor::new("echo \"two  spaces\"   and  some \"mo re\"\n");

        let tokens = process_input(input);
        assert!(tokens.is_ok());
        assert_eq!(
            tokens.unwrap(),
            vec!["echo", "two  spaces", "and", "some", "mo re"]
        );
    }

    #[test]
    fn process_input_single_token() {
        let input = io::Cursor::new("ls\n");
        let tokens = process_input(input);
        assert!(tokens.is_ok());
        assert_eq!(tokens.unwrap(), vec!["ls".to_string()]);
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
        assert!(matches!(err, RushError::UnexpectedEOF));
    }

    #[test]
    fn should_return_command_not_found_for_unknown_command() {
        let input = io::Cursor::new("unknowncmd\n");
        let tokens = process_input(input).unwrap();
        assert_eq!(tokens, vec!["unknowncmd".to_string()]);
    }
}
