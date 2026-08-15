use crate::compiler::Span;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone)]
pub enum LexerError {
    InvalidNumber {
        line: usize,
        col: usize,
    },
    UnexpectedChar {
        expected: Option<String>,
        found: char,
        line: usize,
        col: usize,
    },
    UnclosedQuote {
        line: usize,
        col: usize,
    },
}

impl Display for LexerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            LexerError::UnexpectedChar {
                expected,
                found,
                line,
                col,
            } => {
                if let Some(exp) = expected {
                    write!(
                        f,
                        "Unexpected char at {}:{}: expected '{}', found '{}'",
                        line, col, exp, found
                    )
                } else {
                    write!(f, "Unexpected char at {}:{}: '{}'", line, col, found)
                }
            }
            LexerError::InvalidNumber { line, col } => {
                write!(f, "Invalid number at {}:{}", line, col)
            }
            LexerError::UnclosedQuote { line, col } => {
                write!(f, "Unclosed quote at {}:{}", line, col)
            }
        }
    }
}

impl std::error::Error for LexerError {}

impl LexerError {
    pub fn span(&self) -> Span {
        match self {
            LexerError::InvalidNumber { line, col }
            | LexerError::UnexpectedChar { line, col, .. }
            | LexerError::UnclosedQuote { line, col } => Span::new(*line, *col),
        }
    }
}
