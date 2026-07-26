use crate::compiler::{Span, lexer::Token};
use std::{fmt::Display, str::Chars};

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
            LexerError::UnclosedQuote { line, col } => {
                write!(f, "Unclosed quote at {}:{}", line, col)
            }
        }
    }
}

impl std::error::Error for LexerError {}

impl LexerError {
    pub fn span(&self) -> crate::compiler::Span {
        match self {
            LexerError::InvalidNumber { line, col }
            | LexerError::UnexpectedChar { line, col, .. }
            | LexerError::UnclosedQuote { line, col } => crate::compiler::Span::new(*line, *col),
        }
    }
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

    fn lex_float(&mut self) -> Result<f64, LexerError> {
        let mut buf = String::new();

        while let Some(c) = self.current
            && c.is_ascii_digit()
        {
            buf.push(c);
            self.bump();
        }

        if let Some('.') = self.current {
            buf.push('.');
            self.bump();
        } else {
            return Err(LexerError::InvalidNumber {
                line: self.line,
                col: self.col,
            });
        }

        while let Some(c) = self.current
            && c.is_ascii_digit()
        {
            buf.push(c);
            self.bump();
        }

        if let Some('e' | 'E') = self.current {
            buf.push(self.current.unwrap());
            self.bump();
            if let Some('+' | '-') = self.current {
                buf.push(self.current.unwrap());
                self.bump();
            }
            while let Some(c) = self.current
                && c.is_ascii_digit()
            {
                buf.push(c);
                self.bump();
            }
        }
        buf.parse::<f64>().map_err(|_| LexerError::InvalidNumber {
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

    fn lex_string(&mut self, quote: char) -> Result<String, LexerError> {
        let mut s = String::new();
        while self.current != Some(quote) {
            if self.current == Some('\0') {
                return Err(LexerError::UnclosedQuote {
                    line: self.line,
                    col: self.col,
                });
            }
            if self.current == Some('\\') {
                self.bump();
                match self.current {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('r') => s.push('\r'),
                    Some(c) => s.push(c),
                    None => {}
                }
                self.bump();
                continue;
            }
            s.push(self.current.unwrap());
            self.bump();
        }
        self.bump();
        Ok(s)
    }

    fn next_tok(&mut self) -> Result<(Token, Span), LexerError> {
        self.sw();
        let line = self.line;
        let col = self.col;
        let c = match self.current {
            None => return Ok((Token::EOF, Span::new(line, col))),
            Some(ch) => ch,
        };

        let tok = match c {
            '+' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::PLUSEQ
                } else if self.current == Some('+') {
                    self.bump();
                    Token::PLUSPLUS
                } else {
                    Token::PLUS
                }
            }
            '-' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::MINUSEQ
                } else if self.current == Some('-') {
                    self.bump();
                    Token::MINUSMINUS
                } else {
                    Token::MINUS
                }
            }
            '*' => {
                self.bump();
                Token::STAR
            }
            '/' => {
                self.bump();
                if self.current == Some('/') {
                    while self.current != Some('\n') && self.current.is_some() {
                        self.bump();
                    }
                    return self.next_tok();
                }
                Token::SLASH
            }
            '%' => {
                self.bump();
                Token::PERCENT
            }
            '.' => {
                self.bump();
                if self.current == Some('.') {
                    self.bump();
                    Token::DOTDOT
                } else {
                    Token::DOT
                }
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
            '[' => {
                self.bump();
                Token::LBRACKET
            }
            ']' => {
                self.bump();
                Token::RBRACKET
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
            ';' => {
                self.bump();
                Token::SEMICOLON
            }
            '\\' => {
                self.bump();
                Token::LAMBDA
            }
            '\'' => {
                self.bump();
                Token::STRING(self.lex_string('\'')?)
            }
            '"' => {
                self.bump();
                Token::STRING(self.lex_string('"')?)
            }
            _ if c.is_ascii_digit() => {
                let mut lookahead = self.chars.clone();
                let mut is_float = false;

                while let Some(ch) = lookahead.next() {
                    if ch == '.' {
                        if let Some(next_char) = lookahead.next() {
                            if next_char.is_ascii_digit() {
                                is_float = true;
                            }
                        }
                        break;
                    } else if !ch.is_ascii_digit() {
                        break;
                    }
                }
                if is_float {
                    self.lex_float().map(Token::FLOAT)?
                } else {
                    self.lex_int().map(Token::INT)?
                }
            }
            _ if c.is_ascii_alphabetic() => {
                let ident = self.lex_ident()?;
                match ident.as_str() {
                    "let" => Token::LET,
                    "fun" => Token::FUN,
                    "for" => Token::FOR,
                    "in" => Token::IN,
                    "extern" => Token::EXTERN,
                    "int" => Token::TYPE(ident),
                    "float" => Token::TYPE(ident),
                    "bool" => Token::TYPE(ident),
                    "string" => Token::TYPE(ident),
                    "arr" => Token::IDENT(ident),
                    "void" => Token::TYPE(ident),
                    "gen" => Token::TYPE(ident),
                    "true" => Token::BOOL(true),
                    "false" => Token::BOOL(false),
                    "nil" => Token::NIL,
                    "return" => Token::RET,
                    "if" => Token::IF,
                    "else" => Token::ELSE,
                    "while" => Token::WHILE,
                    "break" => Token::BREAK,
                    "continue" => Token::CONTINUE,
                    "typedef" => Token::TYPEDEF,
                    "struct" => Token::STRUCT,
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

        Ok((tok, Span::new(line, col)))
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<(Token, Span), LexerError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.sw();
        match self.current {
            None => None,
            _ => Some(self.next_tok()),
        }
    }
}
