use std::{iter::Peekable, str::Chars};

use crate::{
    error::GosError,
    token::{Literal, Token, TokenType, VarType},
};

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    tok: Token,
    src: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer {
            tok: Token {
                token: TokenType::EOF,
                value: None,
                row: 1,
                col: 1,
            },
            src: src.chars().peekable(),
        }
    }

    fn current(&mut self) -> char {
        *self.src.peek().unwrap_or(&'\0')
    }

    fn bump(&mut self) -> () {
        self.src.next();
        self.tok.col += 1;
    }

    fn skip_spaces(&mut self) -> () {
        while self.current() == ' ' || self.current() == '\t' || self.current() == '\n' {
            if self.current() == '\n' {
                self.tok.row += 1;
                self.tok.col = 0;
            }
            self.bump();
        }
    }

    fn parse_number(&mut self) -> u64 {
        let mut int_part = 0u64;

        while self.current().is_numeric() {
            int_part = int_part * 10 + self.current().to_digit(10).unwrap() as u64;
            self.bump();
        }

        if self.current() == '.' {
            let mut err = GosError::new(self.tok.row, self.tok.col);
            err.unimplemented("float number");
            err.panic();
        }

        int_part
    }

    fn parse_ident(&mut self) -> String {
        let mut ident = String::new();

        if self.current().is_ascii_alphabetic() {
            ident.push(self.current());
            self.bump();
        }

        while self.current().is_alphanumeric() {
            ident.push(self.current());
            self.bump();
        }
        ident
    }

    fn is_prefix(&mut self) -> bool {
        let prev = *self.src.peek().unwrap_or(&' ');
        self.tok.token == TokenType::EOF
            || self.tok.token == TokenType::LPAREN
            || self.tok.token == TokenType::EQ
            || prev == '='
            || prev == '('
    }

    pub fn next_token(&mut self) -> () {
        self.skip_spaces();
        if self.current() == '\0' {
            self.tok = Token {
                token: TokenType::EOF,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current().is_numeric() {
            let val = self.parse_number();
            self.tok = Token {
                token: TokenType::LITERAL(VarType::Number),
                value: Some(Literal::Number(val as i64)),
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current().is_alphabetic() {
            let ident: String = self.parse_ident();
            match ident.as_str() {
                "true" => {
                    self.tok = Token {
                        token: TokenType::LITERAL(VarType::Bool),
                        value: Some(Literal::Bool(true)),
                        row: self.tok.row,
                        col: self.tok.col,
                    };
                }
                "false" => {
                    self.tok = Token {
                        token: TokenType::LITERAL(VarType::Bool),
                        value: Some(Literal::Bool(false)),
                        row: self.tok.row,
                        col: self.tok.col,
                    };
                }
                "null" => {
                    self.tok = Token {
                        token: TokenType::LITERAL(VarType::Void),
                        value: Some(Literal::Void),
                        row: self.tok.row,
                        col: self.tok.col,
                    };
                }
                "let" => {
                    self.tok = Token {
                        token: TokenType::VARDECL,
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    };
                }
                "fun" => {
                    self.tok = Token {
                        token: TokenType::FUNCDECL,
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
                "return" => {
                    self.tok = Token {
                        token: TokenType::RETURN,
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
                "if" => {
                    self.tok = Token {
                        token: TokenType::IF,
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
                "else" => {
                    self.tok = Token {
                        token: TokenType::ELSE,
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
                "while" => {
                    self.tok = Token {
                        token: TokenType::WHILE,
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
                "goto" => {
                    self.tok = Token {
                        token: TokenType::GOTO,
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
                "extern" => {
                    self.tok = Token {
                        token: TokenType::EXTERN,
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
                "pub" => {
                    self.tok = Token {
                        token: TokenType::PUB,
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
                "num" => {
                    self.tok = Token {
                        token: TokenType::Type(VarType::Number),
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
                "arr" => {
                    if self.current() != '<' {
                        let mut err = GosError::new(self.tok.row, self.tok.col);
                        err.unexpected_char(Some('<'), self.current());
                        err.panic();
                    }
                    self.bump();
                    let len = self.parse_number();
                    if self.current() != '>' {
                        let mut err = GosError::new(self.tok.row, self.tok.col);
                        err.unexpected_char(Some('>'), self.current());
                        err.panic();
                    }
                    self.bump();
                    self.tok = Token {
                        token: TokenType::Type(VarType::Array(len as usize)),
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
                "sizeof" => {
                    self.tok = Token {
                        token: TokenType::SIZEOF,
                        value: None,
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
                _ => {
                    self.tok = Token {
                        token: TokenType::IDENT,
                        value: Some(Literal::Str(ident)),
                        row: self.tok.row,
                        col: self.tok.col,
                    }
                }
            }
            return;
        } else if self.current() == '"' {
            self.bump();
            let mut s = String::new();
            loop {
                match self.current() {
                    '"' => {
                        self.bump();
                        break;
                    }
                    '\0' => {}
                    '\\' => {
                        self.bump();
                        match self.current() {
                            'n' => {
                                s.push('\n');
                                self.bump();
                            }
                            't' => {
                                s.push('\t');
                                self.bump();
                            }
                            'r' => {
                                s.push('\r');
                                self.bump();
                            }
                            '\\' => {
                                s.push('\\');
                                self.bump();
                            }
                            '"' => {
                                s.push('"');
                                self.bump();
                            }
                            _ => self.bump(),
                        }
                    }
                    c => {
                        s.push(c);
                        self.bump();
                    }
                }
            }
            self.tok = Token {
                token: TokenType::LITERAL(VarType::Str),
                value: Some(Literal::Str(s)),
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current() == '\'' {
            self.bump();
            let mut s = String::new();
            while self.current() != '\'' {
                if self.current() == '\0' {
                    let mut err = GosError::new(self.tok.row, self.tok.col);
                    err.unexpected_char(Some('\\'), self.current());
                    err.panic();
                }
                s.push(self.current());
                self.bump();
            }
            self.bump();
            self.tok = Token {
                token: TokenType::LITERAL(VarType::Str),
                value: Some(Literal::Str(s)),
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current() == '+' {
            if self.is_prefix() {
                self.bump();
                return;
            }
            self.bump();
            if self.current() == '+' {
                self.tok = Token {
                    token: TokenType::INC,
                    value: None,
                    row: self.tok.row,
                    col: self.tok.col,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::ADD,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current() == '-' {
            if self.is_prefix() {
                self.tok = Token {
                    token: TokenType::NEG,
                    value: None,
                    row: self.tok.row,
                    col: self.tok.col,
                };
                self.bump();
                return;
            }
            self.bump();
            if self.current() == '-' {
                self.tok = Token {
                    token: TokenType::DEC,
                    value: None,
                    row: self.tok.row,
                    col: self.tok.col,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::SUB,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current() == '*' {
            self.tok = Token {
                token: TokenType::MUL,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            self.bump();
            return;
        } else if self.current() == '/' {
            self.tok = Token {
                token: TokenType::DIV,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            self.bump();
            return;
        } else if self.current() == '(' {
            self.tok = Token {
                token: TokenType::LPAREN,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            self.bump();
            return;
        } else if self.current() == ')' {
            self.tok = Token {
                token: TokenType::RPAREN,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            self.bump();
            return;
        } else if self.current() == '{' {
            self.tok = Token {
                token: TokenType::LBRACE,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            self.bump();
            return;
        } else if self.current() == '}' {
            self.tok = Token {
                token: TokenType::RBRACE,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            self.bump();
            return;
        } else if self.current() == '=' {
            self.bump();
            if self.current() == '=' {
                self.tok = Token {
                    token: TokenType::COMPEQ,
                    value: None,
                    row: self.tok.row,
                    col: self.tok.col,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::EQ,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current() == '!' {
            self.bump();
            if self.current() == '=' {
                self.tok = Token {
                    token: TokenType::COMPNE,
                    value: None,
                    row: self.tok.row,
                    col: self.tok.col,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::LOGNOT,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current() == '>' {
            self.bump();
            if self.current() == '=' {
                self.tok = Token {
                    token: TokenType::COMPGE,
                    value: None,
                    row: self.tok.row,
                    col: self.tok.col,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::COMPGT,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current() == '<' {
            self.bump();
            if self.current() == '=' {
                self.tok = Token {
                    token: TokenType::COMPLE,
                    value: None,
                    row: self.tok.row,
                    col: self.tok.col,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::COMPLT,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current() == '&' {
            self.bump();
            if self.current() == '&' {
                self.tok = Token {
                    token: TokenType::COMPAND,
                    value: None,
                    row: self.tok.row,
                    col: self.tok.col,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::LOGAND,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current() == '|' {
            self.bump();
            if self.current() == '|' {
                self.tok = Token {
                    token: TokenType::COMPOR,
                    value: None,
                    row: self.tok.row,
                    col: self.tok.col,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::LOGOR,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            return;
        } else if self.current() == '^' {
            self.tok = Token {
                token: TokenType::LOGXOR,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            self.bump();
            return;
        } else if self.current() == ':' {
            self.tok = Token {
                token: TokenType::COLON,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            self.bump();
            return;
        } else if self.current() == '[' {
            self.tok = Token {
                token: TokenType::LBRACKET,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            self.bump();
            return;
        } else if self.current() == ']' {
            self.tok = Token {
                token: TokenType::RBRACKET,
                value: None,
                row: self.tok.row,
                col: self.tok.col,
            };
            self.bump();
            return;
        } else if self.current() == '#' {
            while self.current() != '\n' && self.current() != '\0' {
                self.bump();
            }
            return;
        } else {
            let mut err = GosError::new(self.tok.row, self.tok.col);
            err.unexpected_char(None, self.current());
            err.panic();
        }
    }

    pub fn curr_tok(&self) -> Token {
        self.tok.clone()
    }

    pub fn curr_ch(&mut self) -> char {
        self.current()
    }
}
