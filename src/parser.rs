use std::{fmt::Display, iter::Peekable};

use cranelift_object::object::macho::N_EXT;

use crate::{
    ast::{Expr, Program, Type},
    lexer::{Lexer, LexerError, Token},
};

#[derive(Debug)]
pub enum ParserError {
    UnexpectedToken {
        expected: Option<Token>,
        found: Token,
    },
    LexerError(LexerError),
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::UnexpectedToken { expected, found } => {
                if let Some(exp) = expected {
                    write!(f, "Expected '{:?}', found '{:?}'", exp, found)
                } else {
                    write!(f, "Unexpected token: '{:?}'", found)
                }
            }
            ParserError::LexerError(le) => write!(f, "{}", le),
        }
    }
}

impl std::error::Error for ParserError {}

pub struct Parser<'a> {
    lex: Peekable<Lexer<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(lex: Lexer<'a>) -> Self {
        Self {
            lex: lex.peekable(),
        }
    }

    pub fn parse(&mut self) -> Result<Program, ParserError> {
        let mut body = Vec::new();
        loop {
            match self.peek() {
                Some(Ok(Token::EOF)) | None => break,
                _ => body.push(self.expr()?),
            }
        }
        Ok(Program { body })
    }

    fn expr(&mut self) -> Result<Expr, ParserError> {
        if let Some(token) = self.peek() {
            match token {
                Ok(Token::LET) => {
                    self.next()?;
                    let name = match self.next()? {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("NAME".to_string())),
                                found: token,
                            });
                        }
                    };
                    let token = self.next()?;
                    if token != Token::COLON {
                        return Err(ParserError::UnexpectedToken {
                            expected: Some(Token::COLON),
                            found: token,
                        });
                    }
                    let token = self.next()?;
                    let ty = match token {
                        Token::Type(t) => t,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::Type(Type::Int)),
                                found: token,
                            });
                        }
                    };
                    let token = self.next()?;
                    if token != Token::EQ {
                        return Err(ParserError::UnexpectedToken {
                            expected: Some(Token::EQ),
                            found: token,
                        });
                    }

                    Ok(Expr::VarDecl(name, ty, Box::new(self.expr()?)))
                }
                Ok(Token::FUN) => {
                    self.next()?;
                    let name = match self.next()? {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("FUNC NAME".to_string())),
                                found: token,
                            });
                        }
                    };
                    let token = self.next()?;
                    if token != Token::LPAREN {
                        return Err(ParserError::UnexpectedToken {
                            expected: Some(Token::LPAREN),
                            found: token,
                        });
                    }
                    let mut params: Vec<(String, Type)> = Vec::new();
                    loop {
                        let peeked = self
                            .peek()
                            .ok_or(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("PARAM NAME".to_string())),
                                found: Token::EOF,
                            })?
                            .clone();
                        match peeked {
                            Ok(Token::IDENT(s)) => {
                                self.next()?; // consume IDENT token
                                let token = self.next()?;
                                if token != Token::COLON {
                                    return Err(ParserError::UnexpectedToken {
                                        expected: Some(Token::COLON),
                                        found: token,
                                    });
                                }
                                let ty = match self.next()? {
                                    Token::Type(t) => t,
                                    token => {
                                        return Err(ParserError::UnexpectedToken {
                                            expected: Some(Token::Type(Type::Void)),
                                            found: token,
                                        });
                                    }
                                };
                                params.push((s.clone(), ty));

                                let token = self.next()?;
                                if token == Token::RPAREN {
                                    break;
                                } else if token != Token::COMMA {
                                    return Err(ParserError::UnexpectedToken {
                                        expected: Some(Token::COMMA),
                                        found: token,
                                    });
                                }
                            }
                            Ok(Token::RPAREN) => {
                                self.next()?;
                                break;
                            }
                            Ok(token) => {
                                return Err(ParserError::UnexpectedToken {
                                    expected: Some(Token::IDENT("PARAM NAME".to_string())),
                                    found: Token::EOF,
                                });
                            }
                            Err(e) => return Err(ParserError::from(e.clone())),
                        }
                    }
                    if let token = self.next()?
                        && token != Token::COLON
                    {
                        return Err(ParserError::UnexpectedToken {
                            expected: Some(Token::COLON),
                            found: token,
                        });
                    }
                    let ret_type = match self.next()? {
                        Token::Type(t) => t,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::Type(Type::Void)),
                                found: token,
                            });
                        }
                    };
                    Ok(Expr::FuncDecl(
                        name,
                        params,
                        ret_type,
                        Box::new(self.expr()?),
                    ))
                }
                Ok(Token::RET) => {
                    self.next()?;
                    Ok(Expr::Return(Box::new(self.expr()?)))
                }
                Ok(Token::BOOL(b)) => {
                    let bool_val = *b;
                    self.next()?;
                    Ok(Expr::Bool(bool_val))
                }
                Ok(Token::IF) => {
                    self.next()?;
                    let cond = self.expr()?;
                    let then_branch = self.expr()?;
                    let else_branch = match self.peek() {
                        Some(Ok(Token::ELSE)) => {
                            self.next()?; // consume ELSE
                            Some(Box::new(self.expr()?))
                        }
                        _ => None,
                    };
                    Ok(Expr::If(Box::new(cond), Box::new(then_branch), else_branch))
                }
                Ok(Token::WHILE) => {
                    self.next()?;
                    let cond = self.expr()?;
                    let body = self.expr()?;
                    Ok(Expr::While(Box::new(cond), Box::new(body)))
                }
                Ok(_) => self.additive(),
                Err(e) => Err(ParserError::LexerError(e.to_owned())),
            }
        } else {
            unreachable!()
        }
    }

    fn additive(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.term()?;
        while let Some(Ok(op)) = self.peek().cloned() {
            match op {
                Token::PLUS | Token::MINUS => {
                    self.next()?;
                    let right = self.term()?;
                    left = match op {
                        Token::PLUS => Expr::Add(Box::new(left), Box::new(right)),
                        Token::MINUS => Expr::Sub(Box::new(left), Box::new(right)),
                        _ => unreachable!(),
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.factor()?;
        while let Some(Ok(op)) = self.peek().cloned() {
            match op {
                Token::STAR | Token::SLASH => {
                    self.next()?;
                    let right = self.factor()?;
                    left = match op {
                        Token::STAR => Expr::Mul(Box::new(left), Box::new(right)),
                        Token::SLASH => Expr::Div(Box::new(left), Box::new(right)),
                        _ => unreachable!(),
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<Expr, ParserError> {
        if let Some(peeked) = self.peek().cloned() {
            match peeked {
                Ok(Token::INT(n)) => {
                    self.next()?;
                    return Ok(Expr::Int(n));
                }
                Ok(Token::BOOL(b)) => {
                    self.next()?;
                    return Ok(Expr::Bool(b));
                }
                Ok(Token::LPAREN) => {
                    self.next()?;
                    let expr = self.expr()?;
                    match self.peek().cloned() {
                        Some(Ok(Token::RPAREN)) => {
                            self.next()?;
                            return Ok(expr);
                        }
                        Some(Ok(token)) => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::RPAREN),
                                found: token,
                            });
                        }
                        Some(Err(e)) => return Err(e.into()),
                        None => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::RPAREN),
                                found: Token::EOF,
                            });
                        }
                    }
                }
                Ok(Token::LBRACE) => {
                    let mut exprs: Vec<Expr> = Vec::new();
                    self.next()?;
                    loop {
                        match self.peek().ok_or(ParserError::UnexpectedToken {
                            expected: Some(Token::RBRACE),
                            found: Token::EOF,
                        })? {
                            Ok(Token::RBRACE) => {
                                self.next()?;
                                break;
                            }
                            Ok(Token::EOF) => {
                                return Err(ParserError::UnexpectedToken {
                                    expected: Some(Token::RBRACE),
                                    found: Token::EOF,
                                });
                            }
                            Err(e) => return Err(ParserError::LexerError(e.to_owned())),
                            _ => {
                                exprs.push(self.expr()?);
                            }
                        }
                    }
                    return Ok(Expr::Stmt(exprs));
                }
                Ok(Token::IDENT(s)) => {
                    let name = s.clone();
                    self.next()?;

                    if let Some(Ok(Token::LPAREN)) = self.peek() {
                        self.next()?;
                        let mut args: Vec<Expr> = Vec::new();
                        loop {
                            let peeked = self
                                .peek()
                                .ok_or(ParserError::UnexpectedToken {
                                    expected: Some(Token::IDENT("PARAM NAME".to_string())),
                                    found: Token::EOF,
                                })?
                                .clone();
                            match peeked {
                                Ok(Token::RPAREN) => {
                                    self.next()?;
                                    return Ok(Expr::FuncCall(name, args));
                                }
                                Ok(_) => {
                                    args.push(self.expr()?);
                                    let token = self.next()?;
                                    match token {
                                        Token::COMMA => {}
                                        Token::RPAREN => {
                                            return Ok(Expr::FuncCall(name, args));
                                        }
                                        token => {
                                            return Err(ParserError::UnexpectedToken {
                                                expected: Some(Token::COMMA),
                                                found: token,
                                            });
                                        }
                                    }
                                }
                                Err(e) => return Err(e.into()),
                            }
                        }
                    }
                    return Ok(Expr::Var(name));
                }
                Ok(token) => {
                    return Err(ParserError::UnexpectedToken {
                        expected: None,
                        found: token,
                    });
                }
                Err(e) => return Err(ParserError::LexerError(e.to_owned())),
            }
        }
        Err(ParserError::UnexpectedToken {
            expected: None,
            found: Token::EOF,
        })
    }

    fn peek(&mut self) -> Option<&Result<Token, LexerError>> {
        self.lex.peek()
    }

    fn next(&mut self) -> Result<Token, ParserError> {
        if let Some(token) = self.lex.next() {
            match token {
                Ok(token) => Ok(token),
                Err(e) => Err(ParserError::LexerError(e)),
            }
        } else {
            Err(ParserError::UnexpectedToken {
                expected: None,
                found: Token::EOF,
            })
        }
    }
}

impl From<LexerError> for ParserError {
    fn from(value: LexerError) -> Self {
        Self::LexerError(value)
    }
}
