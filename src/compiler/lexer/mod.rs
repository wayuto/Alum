mod error;
mod lexer;
mod tokens;

pub use error::LexerError;
pub use tokens::*;

use std::str::Chars;

pub struct Lexer<'a> {
    chars: Chars<'a>,
    current: Option<char>,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut chars = src.chars();
        let current = chars.next();
        Self {
            chars,
            current,
            line: 1,
            col: 1,
        }
    }
}
