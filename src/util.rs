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

#[derive(Debug)]
enum TokenKind {
    Literal(String),
    Quoted(String),
    Space,
}

#[derive(Debug)]
pub struct Tokenizer {
    input: String,
    tokens: Vec<TokenKind>,
}

impl Tokenizer {
    pub fn from<R>(mut reader: R) -> Result<Self, RushError>
    where
        R: io::BufRead,
    {
        let mut input = String::new();
        reader
            .read_line(&mut input)
            .map_err(|_| RushError::UnexpectedEOF)?;

        Ok(Self {
            input: input.trim().to_owned(),
            tokens: Vec::new(),
        })
    }

    pub fn tokenize(&mut self) -> Result<Vec<String>, RushError> {
        let buf = &mut String::new();
        let mut quote_count = 0;
        let mut has_seen_literal = false;

        for (i, char) in self.input.chars().enumerate() {
            match char {
                '\'' => {
                    quote_count += 1;

                    if quote_count == 1 {
                        // If there's content in buf, push it as a Literal before
                        // starting the quoted string
                        if !buf.trim().is_empty() {
                            has_seen_literal = true;
                            self.tokens.push(TokenKind::Literal(buf.trim().into()));
                        }
                        buf.clear();
                        continue;
                    }

                    if quote_count == 2 {
                        // Ignore empty quoted tokens
                        if buf.trim().len() == 0 {
                            buf.clear();
                            quote_count = 0;
                            continue;
                        }

                        // Concatenate consecutive tokens (only if last token is NOT Space)
                        if !matches!(self.tokens.last(), Some(TokenKind::Space)) {
                            match self.tokens.last_mut() {
                                Some(TokenKind::Quoted(last_token)) => {
                                    last_token.push_str(&buf.clone());
                                    buf.clear();
                                    quote_count = 0;
                                    continue;
                                }
                                Some(TokenKind::Literal(last_token)) => {
                                    last_token.push_str(&buf.clone());
                                    // Convert the Literal to a Quoted since it now contains quoted content
                                    let combined = last_token.clone();
                                    self.tokens.pop();
                                    self.tokens.push(TokenKind::Quoted(combined));
                                    buf.clear();
                                    quote_count = 0;
                                    continue;
                                }
                                _ => {}
                            }
                        } else {
                            // There's a Space before this quoted string, so pop it before adding the new token
                            self.tokens.pop();
                        }

                        self.tokens.push(TokenKind::Quoted(buf.clone()));

                        buf.clear();
                        quote_count = 0;
                    }
                }
                ' ' => {
                    if quote_count == 0 {
                        // Skip over empty tokens
                        if buf.trim().is_empty() {
                            buf.clear();
                            // Push Space token after Literals, OR after Quoted if we've seen a literal before
                            // This allows pure quoted strings to concatenate, but separates tokens when literals are involved
                            if matches!(self.tokens.last(), Some(TokenKind::Literal(_))) {
                                self.tokens.push(TokenKind::Space);
                            } else if has_seen_literal
                                && matches!(self.tokens.last(), Some(TokenKind::Quoted(_)))
                            {
                                self.tokens.push(TokenKind::Space);
                            }
                            continue;
                        }

                        // Since we aren't processing a quoted string, push the buf into
                        // self.tokens as a Literal token
                        has_seen_literal = true;
                        self.tokens.push(TokenKind::Literal(buf.trim().into()));
                        // Push a Space token after the Literal token to help the state machine
                        // determine whether to concatenate or not
                        self.tokens.push(TokenKind::Space);

                        buf.clear();
                        continue;
                    }

                    // We push a space into buf if we're processing a quoted string
                    buf.push(' ');
                }
                char => {
                    // At the end, an odd num of quotes means a quote wasn't terminated
                    if i == self.input.len() - 1 && quote_count % 2 == 1 {
                        return Err(RushError::UnterminatedQuote);
                    }

                    // Push the current char into buf
                    buf.push(char);
                }
            }
        }

        // Push remaining chars into self.tokens
        if buf.len() > 0 {
            // Concatenate with the last token if it's a Literal or Quoted (no Space between)
            match self.tokens.last_mut() {
                Some(TokenKind::Literal(last_token)) => {
                    last_token.push_str(buf.trim());
                }
                Some(TokenKind::Quoted(last_token)) => {
                    last_token.push_str(buf.trim());
                }
                _ => {
                    self.tokens.push(TokenKind::Literal(buf.trim().into()));
                }
            }
        }

        let mut tokens = Vec::<String>::new();

        for token in &self.tokens {
            match token {
                TokenKind::Literal(literal) => tokens.push(literal.to_owned()),
                TokenKind::Quoted(quoted) => tokens.push(quoted.to_owned()),
                TokenKind::Space => { /* state machine hint */ }
            }
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, BufRead};

    // Shared test helper
    fn parse(input: &str) -> Result<Vec<String>, RushError> {
        let mut state_machine = Tokenizer::from(io::Cursor::new(input))?;
        state_machine.tokenize()
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
            assert_eq!(
                parse("echo \'world     shell\' \'script\'\'test\' example\'\'hello").unwrap(),
                vec!["echo", "world     shell", "scripttest", "examplehello"]
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
            let err = Tokenizer::from(reader).unwrap_err();
            assert!(matches!(err, RushError::UnexpectedEOF));
        }
    }
}
