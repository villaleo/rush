use std::io::BufRead;

use std::str::FromStr;
use std::vec::Vec;

use thiserror::Error;

use crate::command::CommandType;

#[derive(Error, Debug)]
pub enum RushError {
    #[error("{type_}: {msg}")]
    CommandError { type_: CommandType, msg: String },
    #[error("{0}: command not found")]
    CommandNotFound(String),
    #[error("")]
    Nop,
    #[error("error reading input: unexpected EOF")]
    UnexpectedEOF,
    #[error("error: unterminated quote")]
    UnterminatedQuote,
}

pub fn tokenize<R: BufRead>(mut reader: R) -> Result<Vec<String>, RushError> {
    let mut input = String::new();
    reader
        .read_line(&mut input)
        .map_err(|_| RushError::UnexpectedEOF)?;

    let Ok(ref input_tokens) = String::from_str(input.trim());

    let buf = &mut String::new();
    let mut tokens = Vec::new();
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
                    if buf.trim().is_empty() {
                        continue;
                    }

                    tokens.push(buf.trim().to_string());
                    buf.clear();
                    continue;
                }

                // Push the current char into buf
                buf.push(char);

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
    use std::io::{self, BufRead};

    // Test helper to simplify test cases
    fn parse(input: &str) -> Result<Vec<String>, RushError> {
        tokenize(io::Cursor::new(input))
    }

    #[test]
    fn test_basic_tokenization() {
        assert_eq!(
            parse("echo hello world\n").unwrap(),
            vec!["echo", "hello", "world"]
        );
        assert_eq!(parse("ls\n").unwrap(), vec!["ls"]);
        assert_eq!(parse("unknowncmd\n").unwrap(), vec!["unknowncmd"]);
    }

    #[test]
    fn test_quoted_strings() {
        assert_eq!(
            parse("echo \"hello world\"\n").unwrap(),
            vec!["echo", "hello world"]
        );
        assert_eq!(
            parse("echo \"two  spaces\"   and  some \"mo re\"\n").unwrap(),
            vec!["echo", "two  spaces", "and", "some", "mo re"]
        );
        assert_eq!(
            parse("\"single quoted token\"\n").unwrap(),
            vec!["single quoted token"]
        );
        assert_eq!(
            parse("\"first\" \"second\" \"third\"\n").unwrap(),
            vec!["first", "second", "third"]
        );
    }

    #[test]
    fn test_unterminated_quotes() {
        assert!(matches!(
            parse("echo \"hello world\n").unwrap_err(),
            RushError::UnterminatedQuote
        ));
        assert!(matches!(
            parse("\"unterminated\n").unwrap_err(),
            RushError::UnterminatedQuote
        ));
        assert!(matches!(
            parse("cmd \"arg1\" \"unterminated\n").unwrap_err(),
            RushError::UnterminatedQuote
        ));
    }

    #[test]
    fn test_whitespace_handling() {
        // Multiple spaces between tokens
        assert_eq!(
            parse("echo    hello    world\n").unwrap(),
            vec!["echo", "hello", "world"]
        );

        // Leading spaces
        assert_eq!(parse("   echo hello\n").unwrap(), vec!["echo", "hello"]);

        // Trailing spaces
        assert_eq!(parse("echo hello   \n").unwrap(), vec!["echo", "hello"]);

        // Multiple spaces everywhere
        assert_eq!(
            parse("   echo    hello    \n").unwrap(),
            vec!["echo", "hello"]
        );

        // Only spaces (edge case)
        assert_eq!(parse("     \n").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(parse("\n").unwrap(), Vec::<String>::new());
        assert_eq!(parse("").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_quotes_with_special_chars() {
        assert_eq!(
            parse("echo \"hello!@#$%^&*()world\"\n").unwrap(),
            vec!["echo", "hello!@#$%^&*()world"]
        );
        assert_eq!(
            parse("echo \"path/to/file\"\n").unwrap(),
            vec!["echo", "path/to/file"]
        );
        assert_eq!(
            parse("grep \"pattern\" file.txt\n").unwrap(),
            vec!["grep", "pattern", "file.txt"]
        );
    }

    #[test]
    fn test_empty_quoted_strings() {
        assert_eq!(parse("\"\"\n").unwrap(), vec![""]);
        assert_eq!(parse("echo \"\"\n").unwrap(), vec!["echo", ""]);
        assert_eq!(parse("\"\" \"\" \"\"\n").unwrap(), vec!["", "", ""]);
    }

    #[test]
    fn test_mixed_quotes_and_tokens() {
        assert_eq!(
            parse("cp \"source file\" dest\n").unwrap(),
            vec!["cp", "source file", "dest"]
        );
        assert_eq!(
            parse("command arg1 \"quoted arg\" arg2 \"another quoted\"\n").unwrap(),
            vec!["command", "arg1", "quoted arg", "arg2", "another quoted"]
        );
    }

    #[test]
    fn test_consecutive_quotes() {
        assert_eq!(parse("\"\"\"\" \n").unwrap(), vec!["", ""]);
        assert_eq!(parse("\"a\"\"b\"\n").unwrap(), vec!["a", "b"]);
    }

    #[test]
    fn test_quotes_at_boundaries() {
        // Quote at start
        assert_eq!(
            parse("\"start\" middle end\n").unwrap(),
            vec!["start", "middle", "end"]
        );

        // Quote at end
        assert_eq!(
            parse("start middle \"end\"\n").unwrap(),
            vec!["start", "middle", "end"]
        );

        // Only quoted token
        assert_eq!(parse("\"only\"\n").unwrap(), vec!["only"]);
    }

    #[test]
    fn test_single_char_tokens() {
        assert_eq!(parse("a b c\n").unwrap(), vec!["a", "b", "c"]);
        assert_eq!(parse("\"a\" \"b\" \"c\"\n").unwrap(), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_long_inputs() {
        let long_token = "a".repeat(1000);
        let input = format!("echo {}\n", long_token);
        assert_eq!(parse(&input).unwrap(), vec!["echo", &long_token]);

        let long_quoted = format!("echo \"{}\"\n", long_token);
        assert_eq!(parse(&long_quoted).unwrap(), vec!["echo", &long_token]);
    }

    #[test]
    fn test_numbers_and_symbols() {
        assert_eq!(
            parse("command 123 456\n").unwrap(),
            vec!["command", "123", "456"]
        );
        assert_eq!(parse("echo $VAR\n").unwrap(), vec!["echo", "$VAR"]);
        assert_eq!(
            parse("test -f file.txt\n").unwrap(),
            vec!["test", "-f", "file.txt"]
        );
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
    fn test_read_error_handling() {
        let reader = ErrReader;
        let err = tokenize(reader).unwrap_err();
        assert!(matches!(err, RushError::UnexpectedEOF));
    }
}
