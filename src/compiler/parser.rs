use crate::compiler::{
    ast::{Expr, Program, Type},
    lexer::{Lexer, LexerError, Token},
};
use std::collections::HashMap;
use std::{fmt::Display, iter::Peekable};

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
    typedefs: HashMap<String, Type>,
    structs: HashMap<String, Vec<(String, Type)>>,
}

impl<'a> Parser<'a> {
    pub fn new(lex: Lexer<'a>) -> Self {
        Self {
            lex: lex.peekable(),
            typedefs: HashMap::new(),
            structs: HashMap::new(),
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
                    let ty = self.parse_type()?;
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
                    self.expect(Token::LPAREN)?;
                    let params = self.get_params_list()?;
                    self.expect(Token::RPAREN)?;
                    self.expect(Token::COLON)?;
                    let ret_type = self.parse_type()?;
                    Ok(Expr::FuncDecl(
                        name,
                        params,
                        ret_type,
                        Box::new(self.expr()?),
                    ))
                }
                Ok(Token::EXTERN) => {
                    self.next()?;
                    let name = match self.next()? {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("EXTERN NAME".to_string())),
                                found: token,
                            });
                        }
                    };
                    self.expect(Token::LPAREN)?;
                    let params = self.get_params_list()?;
                    self.expect(Token::RPAREN)?;
                    self.expect(Token::COLON)?;
                    let ret_type = self.parse_type()?;
                    Ok(Expr::Extern(name, params, ret_type))
                }
                Ok(Token::TYPEDEF) => {
                    self.next()?;
                    let alias_name = match self.next()? {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("TYPEDEF NAME".to_string())),
                                found: token,
                            });
                        }
                    };
                    self.expect(Token::EQ)?;
                    let target_type = self.parse_type()?;

                    self.typedefs
                        .insert(alias_name.clone(), target_type.clone());
                    Ok(Expr::TypeDef)
                }
                Ok(Token::RET) => {
                    self.next()?;
                    Ok(Expr::Return(Box::new(self.expr()?)))
                }
                Ok(Token::IF) => {
                    self.next()?;
                    let cond = self.expr()?;
                    let then_branch = self.expr()?;
                    let else_branch = match self.peek() {
                        Some(Ok(Token::ELSE)) => {
                            self.next()?;
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
                Ok(Token::BREAK) => {
                    self.next()?;
                    Ok(Expr::Break)
                }
                Ok(Token::CONTINUE) => {
                    self.next()?;
                    Ok(Expr::Continue)
                }
                Ok(Token::FOR) => {
                    self.next()?;

                    let var_name = match self.next()? {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("VAR_NAME".to_string())),
                                found: token,
                            });
                        }
                    };

                    self.expect(Token::IN)?;

                    let start = self.expr()?;

                    self.expect(Token::DOTDOT)?;

                    let end = self.expr()?;

                    let body = self.expr()?;

                    Ok(Expr::For(
                        var_name,
                        Box::new(start),
                        Box::new(end),
                        Box::new(body),
                    ))
                }
                Ok(Token::STRUCT) => {
                    self.next()?;
                    let name = match self.next()? {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("STRUCT NAME".to_string())),
                                found: token,
                            });
                        }
                    };
                    self.expect(Token::LBRACE)?;
                    let mut fields = Vec::new();
                    loop {
                        match self.peek().cloned() {
                            Some(Ok(Token::RBRACE)) => {
                                self.next()?;
                                break;
                            }
                            Some(Ok(Token::IDENT(field_name))) => {
                                self.next()?;
                                self.expect(Token::COLON)?;
                                let field_type = self.parse_type()?;
                                fields.push((field_name, field_type));
                                match self.peek().cloned() {
                                    Some(Ok(Token::COMMA)) => {
                                        self.next()?;
                                    }
                                    Some(Ok(Token::RBRACE)) => {
                                        self.next()?;
                                        break;
                                    }
                                    _ => {
                                        return Err(ParserError::UnexpectedToken {
                                            expected: Some(Token::COMMA),
                                            found: Token::EOF,
                                        });
                                    }
                                }
                            }
                            _ => {
                                return Err(ParserError::UnexpectedToken {
                                    expected: Some(Token::IDENT("FIELD NAME".to_string())),
                                    found: Token::EOF,
                                });
                            }
                        }
                    }
                    self.structs.insert(name.clone(), fields.clone());
                    Ok(Expr::Struct(name, fields))
                }

                Ok(_) => {
                    let expr = self.logical()?;
                    if let Some(Ok(Token::EQ)) = self.peek() {
                        self.next()?;
                        let val = self.expr()?;
                        return Ok(Expr::IndexAssign(Box::new(expr), Box::new(val)));
                    }
                    Ok(expr)
                }
                Err(e) => Err(ParserError::LexerError(e.clone())),
            }
        } else {
            unreachable!()
        }
    }

    fn logical(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.comparison()?;
        while let Some(Ok(op)) = self.peek().cloned() {
            match op {
                Token::AND | Token::OR => {
                    self.next()?;
                    let right = self.comparison()?;
                    left = match op {
                        Token::AND => Expr::And(Box::new(left), Box::new(right)),
                        Token::OR => Expr::Or(Box::new(left), Box::new(right)),
                        _ => unreachable!(),
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn call(&mut self) -> Result<Expr, ParserError> {
        let mut callee = self.factor()?;
        while let Some(Ok(Token::LPAREN)) = self.peek() {
            self.next()?;
            let mut args = Vec::new();
            loop {
                match self.peek().cloned() {
                    Some(Ok(Token::RPAREN)) => {
                        self.next()?;
                        break;
                    }
                    Some(Ok(_)) => {
                        args.push(self.expr()?);
                        match self.next()? {
                            Token::COMMA => {}
                            Token::RPAREN => break,
                            token => {
                                return Err(ParserError::UnexpectedToken {
                                    expected: Some(Token::COMMA),
                                    found: token,
                                });
                            }
                        }
                    }
                    _ => {
                        return Err(ParserError::UnexpectedToken {
                            expected: Some(Token::RPAREN),
                            found: Token::EOF,
                        });
                    }
                }
            }
            callee = Expr::Call(Box::new(callee), args);
        }

        while let Some(Ok(Token::LBRACKET)) = self.peek() {
            self.next()?;
            let index = self.expr()?;
            match self.next()? {
                Token::RBRACKET => {}
                token => {
                    return Err(ParserError::UnexpectedToken {
                        expected: Some(Token::RBRACKET),
                        found: token,
                    });
                }
            }
            callee = Expr::Index(Box::new(callee), Box::new(index));
        }

        while let Some(Ok(Token::DOT)) = self.peek() {
            self.next()?;
            let field_name = match self.next()? {
                Token::IDENT(s) => s,
                token => {
                    return Err(ParserError::UnexpectedToken {
                        expected: Some(Token::IDENT("FIELD NAME".to_string())),
                        found: token,
                    });
                }
            };
            callee = Expr::MemberAccess(Box::new(callee), field_name);
        }

        Ok(callee)
    }

    fn comparison(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.additive()?;
        while let Some(Ok(op)) = self.peek().cloned() {
            match op {
                Token::CEQ | Token::NE | Token::LT | Token::LE | Token::GT | Token::GE => {
                    self.next()?;
                    let right = self.additive()?;
                    left = match op {
                        Token::CEQ => Expr::Eq(Box::new(left), Box::new(right)),
                        Token::NE => Expr::Ne(Box::new(left), Box::new(right)),
                        Token::LT => Expr::Lt(Box::new(left), Box::new(right)),
                        Token::LE => Expr::Le(Box::new(left), Box::new(right)),
                        Token::GT => Expr::Gt(Box::new(left), Box::new(right)),
                        Token::GE => Expr::Ge(Box::new(left), Box::new(right)),
                        _ => unreachable!(),
                    };
                }
                _ => break,
            }
        }
        Ok(left)
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
        let mut left = self.call()?;
        while let Some(Ok(op)) = self.peek().cloned() {
            match op {
                Token::STAR | Token::SLASH | Token::PERCENT => {
                    self.next()?;
                    let right = self.call()?;
                    left = match op {
                        Token::STAR => Expr::Mul(Box::new(left), Box::new(right)),
                        Token::SLASH => Expr::Div(Box::new(left), Box::new(right)),
                        Token::PERCENT => Expr::Mod(Box::new(left), Box::new(right)),
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
                Ok(Token::FLOAT(f)) => {
                    self.next()?;
                    return Ok(Expr::Float(f));
                }
                Ok(Token::BOOL(b)) => {
                    self.next()?;
                    return Ok(Expr::Bool(b));
                }
                Ok(Token::STRING(s)) => {
                    self.next()?;
                    return Ok(Expr::String(s));
                }
                Ok(Token::NIL) => {
                    self.next()?;
                    return Ok(Expr::Nil);
                }
                Ok(Token::LAMBDA) => {
                    self.next()?;
                    self.expect(Token::LPAREN)?;
                    let params = self.get_params_list()?;
                    self.expect(Token::RPAREN)?;
                    self.expect(Token::COLON)?;
                    let ret_type = self.parse_type()?;
                    return Ok(Expr::Lambda(params, Box::new(self.expr()?), ret_type));
                }
                Ok(Token::LPAREN) => {
                    self.next()?;
                    let expr = self.logical()?;
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
                            Ok(Token::SEMICOLON) => {
                                self.next()?;
                                continue;
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
                                if let Some(Ok(Token::SEMICOLON)) = self.peek() {
                                    self.next()?;
                                }
                            }
                        }
                    }
                    return Ok(Expr::Stmt(exprs));
                }
                Ok(Token::LBRACKET) => {
                    self.next()?;

                    let is_fill_syntax = match self.peek() {
                        Some(Ok(Token::TYPE(_))) => true,
                        Some(Ok(Token::IDENT(s))) if s == "arr" => true,
                        _ => false,
                    };

                    if is_fill_syntax {
                        let elem_type = self.parse_type()?;
                        self.expect(Token::SEMICOLON)?;
                        let length = self.expr()?;
                        self.expect(Token::RBRACKET)?;
                        return Ok(Expr::ArrayFill(elem_type, Box::new(length)));
                    } else {
                        let mut elements = Vec::new();
                        loop {
                            match self.peek().cloned() {
                                Some(Ok(Token::RBRACKET)) => {
                                    self.next()?;
                                    break;
                                }
                                Some(Ok(_)) => {
                                    elements.push(self.expr()?);
                                    match self.next()? {
                                        Token::COMMA => {}
                                        Token::RBRACKET => break,
                                        token => {
                                            return Err(ParserError::UnexpectedToken {
                                                expected: Some(Token::COMMA),
                                                found: token,
                                            });
                                        }
                                    }
                                }
                                _ => {
                                    return Err(ParserError::UnexpectedToken {
                                        expected: Some(Token::RBRACKET),
                                        found: Token::EOF,
                                    });
                                }
                            }
                        }
                        return Ok(Expr::ArrayLiteral(elements));
                    }
                }
                Ok(Token::IDENT(s)) => {
                    let name = s.clone();
                    self.next()?;

                    if let Some(Ok(Token::LBRACE)) = self.peek() {
                        if self.structs.contains_key(&name) {
                            self.next()?;
                            let mut fields = Vec::new();
                            loop {
                                match self.peek().cloned() {
                                    Some(Ok(Token::RBRACE)) => {
                                        self.next()?;
                                        break;
                                    }
                                    Some(Ok(Token::IDENT(field_name))) => {
                                        self.next()?;
                                        self.expect(Token::COLON)?;
                                        let field_value = self.expr()?;
                                        fields.push((field_name, field_value));
                                        match self.peek().cloned() {
                                            Some(Ok(Token::COMMA)) => {
                                                self.next()?;
                                            }
                                            Some(Ok(Token::RBRACE)) => {
                                                self.next()?;
                                                break;
                                            }
                                            _ => {
                                                return Err(ParserError::UnexpectedToken {
                                                    expected: Some(Token::COMMA),
                                                    found: Token::EOF,
                                                });
                                            }
                                        }
                                    }
                                    _ => {
                                        return Err(ParserError::UnexpectedToken {
                                            expected: Some(Token::IDENT("FIELD NAME".to_string())),
                                            found: Token::EOF,
                                        });
                                    }
                                }
                            }
                            return Ok(Expr::StructLiteral(name, fields));
                        }
                    }

                    if let Some(Ok(Token::EQ)) = self.peek() {
                        self.next()?;
                        let val = self.expr()?;
                        return Ok(Expr::VarAssign(name, Box::new(val)));
                    }
                    return Ok(Expr::Var(name));
                }
                Ok(Token::MINUS) => {
                    self.next()?;
                    let operand = self.factor()?;
                    return Ok(Expr::Sub(Box::new(Expr::Int(0)), Box::new(operand)));
                }
                Ok(Token::NOT) => {
                    self.next()?;
                    let operand = self.factor()?;
                    return Ok(Expr::Not(Box::new(operand)));
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

    fn expect(&mut self, expected: Token) -> Result<(), ParserError> {
        match self.next()? {
            token if token == expected => Ok(()),
            found => Err(ParserError::UnexpectedToken {
                expected: Some(expected),
                found,
            }),
        }
    }

    fn get_params_list(&mut self) -> Result<Vec<(String, Type)>, ParserError> {
        let mut params = Vec::new();
        loop {
            match self.peek().cloned() {
                Some(Ok(Token::IDENT(s))) => {
                    self.next()?;

                    let next_is_colon = matches!(self.peek(), Some(Ok(Token::COLON)));
                    if next_is_colon {
                        self.expect(Token::COLON)?;
                        let ty = self.parse_type()?;
                        params.push((s, ty));
                    } else if s == "arr" && matches!(self.peek(), Some(Ok(Token::LBRACKET))) {
                        self.expect(Token::LBRACKET)?;
                        let inner_type = self.parse_type()?;
                        self.expect(Token::RBRACKET)?;
                        params.push(("_anon".to_string(), Type::Array(Box::new(inner_type))));
                    } else {
                        params.push(("_anon".to_string(), Type::Named(s)));
                    }

                    match self.peek().cloned() {
                        Some(Ok(Token::COMMA)) => {
                            self.next()?;
                        }
                        Some(Ok(Token::RPAREN)) => {
                            break;
                        }
                        _ => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::COMMA),
                                found: Token::EOF,
                            });
                        }
                    }
                }
                Some(Ok(Token::TYPE(t))) => {
                    self.next()?;
                    params.push(("_anon".to_string(), Type::Named(t)));

                    match self.peek().cloned() {
                        Some(Ok(Token::COMMA)) => {
                            self.next()?;
                        }
                        Some(Ok(Token::RPAREN)) => {
                            break;
                        }
                        _ => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::COMMA),
                                found: Token::EOF,
                            });
                        }
                    }
                }
                Some(Ok(Token::RPAREN)) => {
                    break;
                }
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        expected: Some(Token::IDENT("PARAM NAME".to_string())),
                        found: Token::EOF,
                    });
                }
            }
        }
        Ok(params)
    }

    fn peek(&mut self) -> Option<&Result<Token, LexerError>> {
        self.lex.peek()
    }

    fn parse_type(&mut self) -> Result<Type, ParserError> {
        let first_token = self.next()?;

        match first_token {
            Token::TYPE(t) => {
                if let Some(Ok(Token::LPAREN)) = self.peek() {
                    let mut params = Vec::new();
                    self.expect(Token::LPAREN)?;
                    loop {
                        match self.peek() {
                            Some(Ok(Token::RPAREN)) => {
                                self.next()?;
                                break;
                            }
                            Some(Ok(_)) => {
                                params.push(Box::new(self.parse_type()?));
                                match self.peek() {
                                    Some(Ok(Token::COMMA)) => {
                                        self.next()?;
                                    }
                                    Some(Ok(Token::RPAREN)) => {
                                        self.next()?;
                                        break;
                                    }
                                    _ => {
                                        return Err(ParserError::UnexpectedToken {
                                            expected: Some(Token::COMMA),
                                            found: Token::EOF,
                                        });
                                    }
                                }
                            }
                            _ => break,
                        }
                    }

                    return Ok(Type::Function(params, Box::new(Type::Named(t))));
                }

                Ok(Type::Named(t))
            }
            Token::IDENT(s) if s == "arr" => {
                self.expect(Token::LBRACKET)?;
                let inner_type = self.parse_type()?;
                self.expect(Token::RBRACKET)?;
                Ok(Type::Array(Box::new(inner_type)))
            }
            Token::IDENT(s) => {
                if let Some(ty) = self.typedefs.get(&s) {
                    Ok(ty.clone())
                } else {
                    Ok(Type::Named(s))
                }
            }
            token => Err(ParserError::UnexpectedToken {
                expected: Some(Token::TYPE("int".to_string())),
                found: token,
            }),
        }
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
