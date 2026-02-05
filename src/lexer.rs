use std::{fmt::Display, str::Chars};

use crate::ast::Type;

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
}

impl Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Token {
    INT(isize),
    BOOL(bool),
    NIL,
    PLUS,
    MINUS,
    STAR,
    SLASH,
    CEQ,
    NE,
    LT,
    LE,
    GT,
    GE,
    AND,
    OR,
    LAND,
    LOR,
    XOR,
    NOT,
    LPAREN,
    RPAREN,
    LBRACE,
    RBRACE,
    EQ,
    COLON,
    COMMA,
    LET,
    FUN,
    RET,
    IF,
    ELSE,
    WHILE,
    Type(Type),
    IDENT(String),
    EOF,
}

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

    fn bump(&mut self) {
        self.current = self.chars.next();
        self.col += 1;
    }

    fn sw(&mut self) {
        while let Some(c) = self.current {
            if c.is_whitespace() {
                if c == '\n' {
                    self.line += 1;
                    self.col = 1;
                }
                self.bump()
            } else {
                break;
            }
        }
    }

    fn lex_int(&mut self) -> Result<isize, LexerError> {
        let mut buf = String::new();
        while let Some(c) = self.current
            && c.is_ascii_digit()
        {
            buf.push(c);
            self.bump();
        }
        buf.parse::<isize>().map_err(|_| LexerError::InvalidNumber {
            line: self.line,
            col: self.col,
        })
    }

    fn lex_ident(&mut self) -> Result<String, LexerError> {
        let mut ident = String::new();

        if let Some(c) = self.current
            && (c.is_alphabetic() || c == '_')
        {
            ident.push(c);
            self.bump();
        } else {
            return Err(LexerError::UnexpectedChar {
                expected: None,
                found: self.current.unwrap(),
                line: self.line,
                col: self.col,
            });
        }

        while let Some(c) = self.current
            && (c.is_alphanumeric() || c == '_')
        {
            ident.push(c);
            self.bump();
        }

        Ok(ident)
    }

    fn next_token(&mut self) -> Result<Token, LexerError> {
        self.sw();
        let c = match self.current {
            None => return Ok(Token::EOF),
            Some(ch) => ch,
        };

        let tok = match c {
            '+' => {
                self.bump();
                Token::PLUS
            }
            '-' => {
                self.bump();
                Token::MINUS
            }
            '*' => {
                self.bump();
                Token::STAR
            }
            '/' => {
                self.bump();
                if self.current == Some('/') {
                    while self.current == Some('\n') {
                        self.bump();
                    }
                    return self.next_token();
                }
                Token::SLASH
            }
            '(' => {
                self.bump();
                Token::LPAREN
            }
            ')' => {
                self.bump();
                Token::RPAREN
            }
            '{' => {
                self.bump();
                Token::LBRACE
            }
            '}' => {
                self.bump();
                Token::RBRACE
            }
            '=' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::CEQ
                } else {
                    Token::EQ
                }
            }
            '!' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::NE
                } else {
                    Token::NOT
                }
            }
            '>' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::GE
                } else {
                    Token::GT
                }
            }
            '<' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::LE
                } else {
                    Token::LT
                }
            }
            '&' => {
                self.bump();
                if self.current == Some('&') {
                    self.bump();
                    Token::AND
                } else {
                    Token::LAND
                }
            }
            '|' => {
                self.bump();
                if self.current == Some('|') {
                    self.bump();
                    Token::OR
                } else {
                    Token::LOR
                }
            }
            '^' => {
                self.bump();
                Token::XOR
            }
            ':' => {
                self.bump();
                Token::COLON
            }
            ',' => {
                self.bump();
                Token::COMMA
            }
            _ if c.is_ascii_digit() => self.lex_int().map(Token::INT)?,
            _ if c.is_ascii_alphabetic() => {
                let ident = self.lex_ident()?;
                match ident.as_str() {
                    "let" => Token::LET,
                    "fun" => Token::FUN,
                    "int" => Token::Type(Type::Int),
                    "bool" => Token::Type(Type::Bool),
                    "void" => Token::Type(Type::Void),
                    "true" => Token::BOOL(true),
                    "false" => Token::BOOL(false),
                    "nil" => Token::NIL,
                    "return" => Token::RET,
                    "if" => Token::IF,
                    "else" => Token::ELSE,
                    "while" => Token::WHILE,
                    _ => Token::IDENT(ident),
                }
            }
            _ => {
                let err = LexerError::UnexpectedChar {
                    expected: None,
                    found: c,
                    line: self.line,
                    col: self.col,
                };
                self.bump();
                return Err(err);
            }
        };

        Ok(tok)
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexerError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.sw();
        match self.current {
            None => None,
            _ => Some(self.next_token()),
        }
    }
}
