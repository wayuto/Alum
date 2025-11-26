use crate::token::{Literal, Token, TokenType};

#[derive(Debug)]
pub struct Lexer {
    pub tok: Token,
    pos: usize,
    src: Vec<char>,
}

impl Lexer {
    pub fn new(src: String) -> Self {
        Lexer {
            pos: 0,
            tok: Token {
                token: TokenType::EOF,
                value: None,
            },
            src: src.chars().collect(),
        }
    }

    fn current(&self) -> char {
        *self.src.get(self.pos).unwrap_or(&'\0')
    }

    fn bump(&mut self) -> () {
        self.pos += 1;
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

    fn is_prefix(&self) -> bool {
        let prev = *self.src.get(self.pos - 1).unwrap_or(&' ');
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
                        token: TokenType::FUNCDECL,
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
                _ => {}
            }
            return;
        } else if self.current() == '"' {
        }
    }
}
