use std::{iter::Peekable, str::Chars};

use crate::token::{Literal, Token, TokenType};

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
            },
            src: src.chars().peekable(),
        }
    }

    fn current(&mut self) -> char {
        *self.src.peek().unwrap_or(&'\0')
    }

    fn bump(&mut self) -> () {
        self.src.next();
    }

    fn skip_spaces(&mut self) -> () {
        while self.current() == ' ' || self.current() == '\t' || self.current() == '\n' {
            self.bump();
        }
    }

    fn parse_number(&mut self) -> f64 {
        let mut int_part = 0;
        let mut frac_part = 0;
        let mut frac_div = 1;

        while self.current().is_numeric() {
            int_part = int_part * 10 + self.current().to_digit(10).unwrap();
            self.bump();
        }

        if self.current() == '.' {
            self.bump();
            if !self.current().is_numeric() {
                panic!("Lexer: Invalid number: expected digit after '.'")
            }
            while self.current().is_numeric() {
                frac_div *= 10;
                frac_part = frac_part * 10 + self.current().to_digit(10).unwrap();
                self.bump();
            }
        }

        int_part as f64 + frac_part as f64 / frac_div as f64
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
            };
            return;
        } else if self.current().is_numeric() {
            let val = self.parse_number();
            self.tok = Token {
                token: TokenType::LITERAL,
                value: Some(Literal::Number(val)),
            };
            return;
        } else if self.current().is_alphabetic() {
            let ident = self.parse_ident();
            match ident.as_str() {
                "true" => {
                    self.tok = Token {
                        token: TokenType::LITERAL,
                        value: Some(Literal::Bool(true)),
                    };
                }
                "false" => {
                    self.tok = Token {
                        token: TokenType::LITERAL,
                        value: Some(Literal::Bool(false)),
                    };
                }
                "null" => {
                    self.tok = Token {
                        token: TokenType::LITERAL,
                        value: Some(Literal::Void),
                    };
                }
                "let" => {
                    self.tok = Token {
                        token: TokenType::VARDECL,
                        value: None,
                    };
                }
                "fun" => {
                    self.tok = Token {
                        token: TokenType::FUNCDECL,
                        value: None,
                    }
                }
                "return" => {
                    self.tok = Token {
                        token: TokenType::RETURN,
                        value: None,
                    }
                }
                "out" => {
                    self.tok = Token {
                        token: TokenType::OUT,
                        value: None,
                    }
                }
                "in" => {
                    self.tok = Token {
                        token: TokenType::IN,
                        value: None,
                    }
                }
                "if" => {
                    self.tok = Token {
                        token: TokenType::IF,
                        value: None,
                    }
                }
                "else" => {
                    self.tok = Token {
                        token: TokenType::ELSE,
                        value: None,
                    }
                }
                "while" => {
                    self.tok = Token {
                        token: TokenType::WHILE,
                        value: None,
                    }
                }
                "goto" => {
                    self.tok = Token {
                        token: TokenType::GOTO,
                        value: None,
                    }
                }
                "exit" => {
                    self.tok = Token {
                        token: TokenType::EXIT,
                        value: None,
                    }
                }
                _ => {
                    self.tok = Token {
                        token: TokenType::IDENT,
                        value: Some(Literal::Str(ident)),
                    }
                }
            }
            return;
        } else if self.current() == '"' {
            self.bump();
            let mut s = String::new();
            while self.current().is_alphabetic() {
                if self.current() == '\0' {
                    panic!("Lexer: Expected: '\"'")
                }
                s.push(self.current());
                self.bump();
            }
            self.bump();
            self.tok = Token {
                token: TokenType::LITERAL,
                value: Some(Literal::Str(s)),
            };
            return;
        } else if self.current() == '\'' {
            self.bump();
            let mut s = String::new();
            while self.current().is_alphabetic() {
                if self.current() == '\0' {
                    panic!("Lexer: Expected: \"'\"")
                }
                s.push(self.current());
                self.bump();
            }
            self.bump();
            self.tok = Token {
                token: TokenType::LITERAL,
                value: Some(Literal::Str(s)),
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
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::ADD,
                value: None,
            };
            return;
        } else if self.current() == '-' {
            if self.is_prefix() {
                self.tok = Token {
                    token: TokenType::NEG,
                    value: None,
                };
                self.bump();
                return;
            }
            self.bump();
            if self.current() == '-' {
                self.tok = Token {
                    token: TokenType::DEC,
                    value: None,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::SUB,
                value: None,
            };
            return;
        } else if self.current() == '*' {
            self.tok = Token {
                token: TokenType::MUL,
                value: None,
            };
            self.bump();
            return;
        } else if self.current() == '/' {
            self.tok = Token {
                token: TokenType::DIV,
                value: None,
            };
            self.bump();
            return;
        } else if self.current() == '(' {
            self.tok = Token {
                token: TokenType::LPAREN,
                value: None,
            };
            self.bump();
            return;
        } else if self.current() == ')' {
            self.tok = Token {
                token: TokenType::RPAREN,
                value: None,
            };
            self.bump();
            return;
        } else if self.current() == '{' {
            self.tok = Token {
                token: TokenType::LBRACE,
                value: None,
            };
            self.bump();
            return;
        } else if self.current() == '}' {
            self.tok = Token {
                token: TokenType::RBRACE,
                value: None,
            };
            self.bump();
            return;
        } else if self.current() == '=' {
            self.bump();
            if self.current() == '=' {
                self.tok = Token {
                    token: TokenType::COMPEQ,
                    value: None,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::EQ,
                value: None,
            };
            return;
        } else if self.current() == '!' {
            self.bump();
            if self.current() == '=' {
                self.tok = Token {
                    token: TokenType::COMPNE,
                    value: None,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::LOGNOT,
                value: None,
            };
            return;
        } else if self.current() == '>' {
            self.bump();
            if self.current() == '=' {
                self.tok = Token {
                    token: TokenType::COMPGE,
                    value: None,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::COMPGT,
                value: None,
            };
            return;
        } else if self.current() == '<' {
            self.bump();
            if self.current() == '=' {
                self.tok = Token {
                    token: TokenType::COMPLE,
                    value: None,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::COMPLT,
                value: None,
            };
            return;
        } else if self.current() == '&' {
            self.bump();
            if self.current() == '&' {
                self.tok = Token {
                    token: TokenType::COMPAND,
                    value: None,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::LOGAND,
                value: None,
            };
            return;
        } else if self.current() == '|' {
            self.bump();
            if self.current() == '|' {
                self.tok = Token {
                    token: TokenType::COMPOR,
                    value: None,
                };
                self.bump();
                return;
            }
            self.tok = Token {
                token: TokenType::LOGOR,
                value: None,
            };
            return;
        } else if self.current() == '^' {
            self.tok = Token {
                token: TokenType::LOGXOR,
                value: None,
            };
            self.bump();
            return;
        } else if self.current() == ':' {
            self.tok = Token {
                token: TokenType::COLON,
                value: None,
            };
            self.bump();
            return;
        } else if self.current() == '#' {
            while self.current() != '\n' || self.current() != '\0' {
                self.bump();
            }
            return;
        } else {
            panic!("Lexer: Unknown token {}", self.current())
        }
    }

    pub fn current_token(&self) -> Token {
        self.tok.clone()
    }
}
