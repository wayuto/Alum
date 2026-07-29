use crate::compiler::{
    Span,
    lexer::{LexerError, Token},
};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum ParserError {
    UnexpectedToken {
        expected: Option<Token>,
        found: Token,
        span: Span,
    },
    LexerError(LexerError),
}

impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            ParserError::UnexpectedToken {
                expected,
                found,
                span,
            } => {
                if let Some(exp) = expected {
                    write!(
                        f,
                        "at {}:{}: Expected '{:?}', found '{:?}'",
                        span.line, span.col, exp, found
                    )
                } else {
                    write!(
                        f,
                        "at {}:{}: Unexpected token: '{:?}'",
                        span.line, span.col, found
                    )
                }
            }
            ParserError::LexerError(le) => write!(f, "{}", le),
        }
    }
}

impl std::error::Error for ParserError {}

impl ParserError {
    pub fn span(&self) -> Option<Span> {
        match self {
            ParserError::UnexpectedToken { span, .. } => Some(*span),
            ParserError::LexerError(e) => Some(e.span()),
        }
    }
}

impl From<LexerError> for ParserError {
    fn from(value: LexerError) -> Self {
        Self::LexerError(value)
    }
}
