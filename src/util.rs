use std::io::{self};
use std::vec::Vec;

use crate::command::CommandType;

#[derive(thiserror::Error, Debug)]
pub enum RushError {
    #[error("{type_}: {msg}")]
    CommandError {
        type_: CommandType,
        msg: String,
        status: Option<i32>,
    },
    #[error("{0}: command not found")]
    CommandNotFound(String),
    #[error("")]
    Nop,
    #[error("error reading input: unexpected EOF")]
    UnexpectedEOF,
    #[error("error: unterminated quote")]
    UnterminatedQuote,
}

pub fn tokenize<R: io::BufRead>(mut reader: R) -> Result<Vec<String>, RushError> {
    let mut input = String::new();
    reader
        .read_line(&mut input)
        .map_err(|_| RushError::UnexpectedEOF)?;

    let input_tokens = input.trim();
    let buf = &mut String::new();

    enum TokenKind {
        Literal(String),
        Quoted(String),
    }

    let mut tokens = Vec::<TokenKind>::new();
    let mut quote_count = 0;

    for (i, char) in input_tokens.chars().enumerate() {
        match char {
            '\'' => {
                quote_count += 1;

                // Push buf to tokens when more than 1 quote is found
                if quote_count > 1 {
                    // Ignore empty quoted tokens
                    if buf.len() == 0 {
                        quote_count = 0;
                        continue;
                    }

                    // Concatenate consecutive quoted tokens
                    if let Some(TokenKind::Quoted(last_token)) = tokens.last_mut() {
                        last_token.push_str(&buf.clone());
                    } else {
                        tokens.push(TokenKind::Quoted(buf.clone()));
                    }

                    buf.clear();
                    quote_count = 0;
                }
            }
            ' ' => {
                // If we haven't seen a quote yet and we encounter a space, push buf
                // into tokens and clear buf
                if quote_count == 0 {
                    // Skip over empty tokens
                    if buf.trim().is_empty() {
                        continue;
                    }

                    tokens.push(TokenKind::Literal(buf.trim().into()));
                    buf.clear();
                    continue;
                }

                buf.push(' ');
            }
            char => {
                // At the end, an odd num of quotes means a quote wasn't terminated
                if i == input_tokens.len() - 1 && quote_count % 2 == 1 {
                    return Err(RushError::UnterminatedQuote);
                }

                // Push the current char into buf
                buf.push(char);

                // At the end, push any remaining chars into tokens
                if i == input_tokens.len() - 1 && buf.len() > 0 {
                    tokens.push(TokenKind::Literal(buf.trim().into()));
                }
            }
        }
    }

    Ok(tokens
        .iter()
        .map(|token| match token {
            TokenKind::Literal(literal) => literal.to_owned(),
            TokenKind::Quoted(quoted) => quoted.to_owned(),
        })
        .collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, BufRead};

    // Shared test helper
    fn parse(input: &str) -> Result<Vec<String>, RushError> {
        tokenize(io::Cursor::new(input))
    }

    mod basic_tokenization {
        use super::*;

        #[test]
        fn single_command() {
            assert_eq!(parse("ls\n").unwrap(), vec!["ls"]);
        }

        #[test]
        fn command_with_arguments() {
            assert_eq!(
                parse("echo hello world\n").unwrap(),
                vec!["echo", "hello", "world"]
            );
        }

        #[test]
        fn unknown_command() {
            assert_eq!(parse("unknowncmd\n").unwrap(), vec!["unknowncmd"]);
        }

        #[test]
        fn single_character_tokens() {
            assert_eq!(parse("a b c\n").unwrap(), vec!["a", "b", "c"]);
        }

        #[test]
        fn numbers_and_flags() {
            assert_eq!(
                parse("command 123 456\n").unwrap(),
                vec!["command", "123", "456"]
            );
            assert_eq!(
                parse("test -f file.txt\n").unwrap(),
                vec!["test", "-f", "file.txt"]
            );
        }

        #[test]
        fn shell_variables() {
            assert_eq!(parse("echo $VAR\n").unwrap(), vec!["echo", "$VAR"]);
        }
    }

    mod quoted_strings {
        use super::*;

        #[test]
        fn simple_quoted_string() {
            assert_eq!(
                parse("echo \'hello world\'\n").unwrap(),
                vec!["echo", "hello world"]
            );
        }

        #[test]
        fn consecutive_quoted_strings_are_concatenated() {
            assert_eq!(
                parse("\'first\' \'second\' \'third\'\n").unwrap(),
                vec!["firstsecondthird"]
            );
        }

        #[test]
        fn preserves_spaces_in_quotes() {
            assert_eq!(
                parse("echo \'two  spaces\'   and  some \'mo re\'\n").unwrap(),
                vec!["echo", "two  spaces", "and", "some", "mo re"]
            );
        }

        #[test]
        fn single_quoted_token() {
            assert_eq!(
                parse("\'single quoted token\'\n").unwrap(),
                vec!["single quoted token"]
            );
        }

        #[test]
        fn empty_quoted_strings() {
            assert_eq!(parse("\'\'\n").unwrap(), Vec::<&str>::new());
            assert_eq!(parse("echo \'\'\n").unwrap(), vec!["echo"]);
            assert_eq!(parse("\'\' \'\' \'\'\n").unwrap(), Vec::<&str>::new());
        }

        #[test]
        fn quotes_with_special_characters() {
            assert_eq!(
                parse("echo \'hello!@#$%^&*()world\'\n").unwrap(),
                vec!["echo", "hello!@#$%^&*()world"]
            );
            assert_eq!(
                parse("echo \'path/to/file\'\n").unwrap(),
                vec!["echo", "path/to/file"]
            );
        }

        #[test]
        fn mixed_quoted_and_unquoted() {
            assert_eq!(
                parse("cp \'source file\' dest\n").unwrap(),
                vec!["cp", "source file", "dest"]
            );
            assert_eq!(
                parse("command arg1 \'quoted arg\' arg2 \'another quoted\'\n").unwrap(),
                vec!["command", "arg1", "quoted arg", "arg2", "another quoted"]
            );
        }

        #[test]
        fn consecutive_quotes() {
            assert_eq!(parse("\'\'\'\' \n").unwrap(), Vec::<&str>::new());
            assert_eq!(parse("\'a\'\'b\'\n").unwrap(), vec!["ab"]);
        }

        #[test]
        fn quotes_at_start() {
            assert_eq!(
                parse("\'start\' middle end\n").unwrap(),
                vec!["start", "middle", "end"]
            );
        }

        #[test]
        fn quotes_at_end() {
            assert_eq!(
                parse("start middle \'end\'\n").unwrap(),
                vec!["start", "middle", "end"]
            );
        }

        #[test]
        fn only_quoted_token() {
            assert_eq!(parse("\'only\'\n").unwrap(), vec!["only"]);
        }

        #[test]
        fn single_char_quoted() {
            assert_eq!(parse("\'a\' \'b\' \'c\'\n").unwrap(), vec!["abc"]);
        }

        #[test]
        fn quoted_pattern_for_grep() {
            assert_eq!(
                parse("grep \'pattern\' file.txt\n").unwrap(),
                vec!["grep", "pattern", "file.txt"]
            );
        }
    }

    mod whitespace_handling {
        use super::*;

        #[test]
        fn multiple_spaces_between_tokens() {
            assert_eq!(
                parse("echo    hello    world\n").unwrap(),
                vec!["echo", "hello", "world"]
            );
        }

        #[test]
        fn leading_spaces() {
            assert_eq!(parse("   echo hello\n").unwrap(), vec!["echo", "hello"]);
        }

        #[test]
        fn trailing_spaces() {
            assert_eq!(parse("echo hello   \n").unwrap(), vec!["echo", "hello"]);
        }

        #[test]
        fn mixed_leading_and_trailing_spaces() {
            assert_eq!(
                parse("   echo    hello    \n").unwrap(),
                vec!["echo", "hello"]
            );
        }

        #[test]
        fn only_spaces() {
            assert_eq!(parse("     \n").unwrap(), Vec::<String>::new());
        }

        #[test]
        fn empty_input() {
            assert_eq!(parse("\n").unwrap(), Vec::<String>::new());
            assert_eq!(parse("").unwrap(), Vec::<String>::new());
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn very_long_unquoted_token() {
            let long_token = "a".repeat(1000);
            let input = format!("echo {}\n", long_token);
            assert_eq!(parse(&input).unwrap(), vec!["echo", &long_token]);
        }

        #[test]
        fn very_long_quoted_token() {
            let long_token = "a".repeat(1000);
            let long_quoted = format!("echo \'{}\'\n", long_token);
            assert_eq!(parse(&long_quoted).unwrap(), vec!["echo", &long_token]);
        }
    }

    mod error_handling {
        use super::*;

        #[test]
        fn unterminated_quote_at_end() {
            assert!(matches!(
                parse("echo \'hello world\n").unwrap_err(),
                RushError::UnterminatedQuote
            ));
        }

        #[test]
        fn unterminated_quote_at_start() {
            assert!(matches!(
                parse("\'unterminated\n").unwrap_err(),
                RushError::UnterminatedQuote
            ));
        }

        #[test]
        fn unterminated_quote_after_valid_quotes() {
            assert!(matches!(
                parse("cmd \'arg1\' \'unterminated\n").unwrap_err(),
                RushError::UnterminatedQuote
            ));
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
        fn io_read_error_returns_unexpected_eof() {
            let reader = ErrReader;
            let err = tokenize(reader).unwrap_err();
            assert!(matches!(err, RushError::UnexpectedEOF));
        }
    }
}
