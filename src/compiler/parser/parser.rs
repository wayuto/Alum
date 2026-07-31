use super::error::ParserError;
use crate::compiler::{
    Span,
    lexer::{Lexer, LexerError, Token},
    parser::{Expr, Parser, Program, Type},
};
use std::collections::HashMap;

impl<'a> Parser<'a> {
    pub fn new(lex: Lexer<'a>) -> Self {
        Self {
            lex: lex.peekable(),
            lookahead: Vec::new(),
            last_span: Span::new(1, 1),
            typedefs: HashMap::new(),
            structs: HashMap::new(),
            type_param_scopes: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Result<Program, ParserError> {
        let mut body = Vec::new();
        loop {
            match self.peek() {
                Some(Ok((Token::EOF, _))) | None => break,
                _ => body.push(self.expr()?),
            }
        }
        Ok(Program { body })
    }

    fn expr(&mut self) -> Result<Expr, ParserError> {
        if let Some(token) = self.peek().cloned() {
            match token {
                Ok((Token::LET, _)) => {
                    self.next()?;
                    let (token, span) = self.next()?;
                    let name = match token {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("NAME".to_string())),
                                found: token,
                                span,
                            });
                        }
                    };
                    let ty = if matches!(self.peek(), Some(Ok((Token::COLON, _)))) {
                        self.next()?;
                        self.parse_type()?
                    } else {
                        Type::Unknown
                    };
                    let (token, span) = self.next()?;
                    if token != Token::EQ {
                        return Err(ParserError::UnexpectedToken {
                            expected: Some(Token::EQ),
                            found: token,
                            span,
                        });
                    }

                    Ok(Expr::VarDecl(name, ty, Box::new(self.expr()?), span))
                }
                Ok((Token::FUN, _)) => {
                    self.next()?;
                    let (token, span) = self.next()?;
                    let name = match token {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("FUNC NAME".to_string())),
                                found: token,
                                span,
                            });
                        }
                    };
                    let type_params = if matches!(self.peek(), Some(Ok((Token::LT, _)))) {
                        self.next()?;
                        let params = self.get_type_params_list()?;
                        self.expect(Token::GT)?;
                        params
                    } else {
                        Vec::new()
                    };
                    if !type_params.is_empty() {
                        self.push_type_params(&type_params);
                    }
                    self.expect(Token::LPAREN)?;
                    let params = self.get_params_list()?;
                    self.expect(Token::RPAREN)?;
                    self.expect(Token::COLON)?;
                    let ret_type = self.parse_type()?;
                    let body = self.expr()?;
                    if !type_params.is_empty() {
                        self.type_param_scopes.pop();
                    }
                    Ok(Expr::FuncDecl(
                        name,
                        type_params,
                        params,
                        ret_type,
                        Box::new(body),
                        span,
                    ))
                }
                Ok((Token::EXTERN, _)) => {
                    self.next()?;
                    let (token, span) = self.next()?;
                    let name = match token {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("EXTERN NAME".to_string())),
                                found: token,
                                span,
                            });
                        }
                    };
                    self.expect(Token::LPAREN)?;
                    let params = self.get_params_list()?;
                    self.expect(Token::RPAREN)?;
                    self.expect(Token::COLON)?;
                    let ret_type = self.parse_type()?;
                    Ok(Expr::Extern(name, params, ret_type, span))
                }
                Ok((Token::TYPEDEF, _)) => {
                    self.next()?;
                    let (token, span) = self.next()?;
                    let alias_name = match token {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("TYPEDEF NAME".to_string())),
                                found: token,
                                span,
                            });
                        }
                    };
                    self.expect(Token::EQ)?;
                    let target_type = self.parse_type()?;

                    self.typedefs
                        .insert(alias_name.clone(), target_type.clone());
                    Ok(Expr::TypeDef(span))
                }
                Ok((Token::RET, span)) => {
                    self.next()?;
                    Ok(Expr::Return(Box::new(self.expr()?), span))
                }
                Ok((Token::IF, _)) => {
                    self.next()?;
                    let cond = self.expr()?;
                    let then_branch = self.expr()?;
                    let else_branch = match self.peek() {
                        Some(Ok((Token::ELSE, _))) => {
                            self.next()?;
                            Some(Box::new(self.expr()?))
                        }
                        _ => None,
                    };
                    Ok(Expr::If(
                        Box::new(cond),
                        Box::new(then_branch),
                        else_branch,
                        Span::new(0, 0),
                    ))
                }
                Ok((Token::WHILE, _)) => {
                    self.next()?;
                    let cond = self.expr()?;
                    let body = self.expr()?;
                    Ok(Expr::While(Box::new(cond), Box::new(body), Span::new(0, 0)))
                }
                Ok((Token::BREAK, span)) => {
                    self.next()?;
                    Ok(Expr::Break(span))
                }
                Ok((Token::CONTINUE, span)) => {
                    self.next()?;
                    Ok(Expr::Continue(span))
                }
                Ok((Token::FOR, _)) => {
                    self.next()?;

                    let (token, span) = self.next()?;
                    let var_name = match token {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("VAR_NAME".to_string())),
                                found: token,
                                span,
                            });
                        }
                    };

                    self.expect(Token::IN)?;

                    let array_expr = self.expr()?;

                    let body = self.expr()?;

                    Ok(Expr::For(
                        var_name,
                        Box::new(array_expr),
                        Box::new(body),
                        span,
                    ))
                }
                Ok((Token::STRUCT, _)) => {
                    self.next()?;
                    let (token, span) = self.next()?;
                    let name = match token {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("STRUCT NAME".to_string())),
                                found: token,
                                span,
                            });
                        }
                    };
                    let type_params = if matches!(self.peek(), Some(Ok((Token::LT, _)))) {
                        self.next()?;
                        let params = self.get_type_params_list()?;
                        self.expect(Token::GT)?;
                        params
                    } else {
                        Vec::new()
                    };
                    if !type_params.is_empty() {
                        self.push_type_params(&type_params);
                    }
                    self.expect(Token::LBRACE)?;
                    let mut fields = Vec::new();
                    loop {
                        match self.peek().cloned() {
                            Some(Ok((Token::RBRACE, _))) => {
                                self.next()?;
                                break;
                            }
                            Some(Ok((Token::IDENT(field_name), _))) => {
                                self.next()?;
                                self.expect(Token::COLON)?;
                                let field_type = self.parse_type()?;
                                fields.push((field_name, field_type));
                                match self.peek().cloned() {
                                    Some(Ok((Token::COMMA, _))) => {
                                        self.next()?;
                                    }
                                    Some(Ok((Token::RBRACE, _))) => {
                                        self.next()?;
                                        break;
                                    }
                                    _ => {
                                        return Err(ParserError::UnexpectedToken {
                                            expected: Some(Token::COMMA),
                                            found: Token::EOF,
                                            span: self.last_span,
                                        });
                                    }
                                }
                            }
                            _ => {
                                return Err(ParserError::UnexpectedToken {
                                    expected: Some(Token::IDENT("FIELD NAME".to_string())),
                                    found: Token::EOF,
                                    span: self.last_span,
                                });
                            }
                        }
                    }
                    if !type_params.is_empty() {
                        self.type_param_scopes.pop();
                    }
                    self.structs
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                    Ok(Expr::Struct(name, type_params, fields, span))
                }

                Ok((_, _)) => {
                    let expr = self.logical()?;
                    if let Some(Ok((Token::EQ, _))) = self.peek() {
                        self.next()?;
                        let val = self.expr()?;
                        return match &expr {
                            Expr::Index(_, _, _) => {
                                if let Expr::Index(arr, idx, _) = expr {
                                    Ok(Expr::IndexAssign(
                                        Box::new(Expr::Index(arr, idx, Span::new(0, 0))),
                                        Box::new(val),
                                        Span::new(0, 0),
                                    ))
                                } else {
                                    unreachable!()
                                }
                            }
                            Expr::MemberAccess(_, _, _) => {
                                if let Expr::MemberAccess(obj, field, _) = expr {
                                    Ok(Expr::MemberAssign(
                                        obj,
                                        field,
                                        Box::new(val),
                                        Span::new(0, 0),
                                    ))
                                } else {
                                    unreachable!()
                                }
                            }
                            Expr::Deref(_, _) => {
                                if let Expr::Deref(ptr, _) = expr {
                                    Ok(Expr::DerefAssign(ptr, Box::new(val), Span::new(0, 0)))
                                } else {
                                    unreachable!()
                                }
                            }
                            _ => Ok(Expr::IndexAssign(
                                Box::new(expr),
                                Box::new(val),
                                Span::new(0, 0),
                            )),
                        };
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
        let mut left = self.bitwise()?;
        while let Some(Ok((op, _))) = self.peek().cloned() {
            match op {
                Token::AND | Token::OR => {
                    self.next()?;
                    let right = self.bitwise()?;
                    let span = left.span();
                    left = match op {
                        Token::AND => Expr::LAnd(Box::new(left), Box::new(right), span),
                        Token::OR => Expr::LOr(Box::new(left), Box::new(right), span),
                        _ => unreachable!(),
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn bitwise(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.comparison()?;
        while let Some(Ok((op, _))) = self.peek().cloned() {
            match op {
                Token::XOR => {
                    self.next()?;
                    let right = self.comparison()?;
                    let span = left.span();
                    left = Expr::Xor(Box::new(left), Box::new(right), span);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn call(&mut self) -> Result<Expr, ParserError> {
        let mut callee = self.factor()?;

        loop {
            match self.peek().cloned() {
                Some(Ok((Token::LPAREN, _))) => {
                    self.next()?;
                    let mut args = Vec::new();
                    loop {
                        match self.peek().cloned() {
                            Some(Ok((Token::RPAREN, _))) => {
                                self.next()?;
                                break;
                            }
                            Some(Ok((_, _))) => {
                                args.push(self.expr()?);
                                let (token, span) = self.next()?;
                                match token {
                                    Token::COMMA => {}
                                    Token::RPAREN => break,
                                    token => {
                                        return Err(ParserError::UnexpectedToken {
                                            expected: Some(Token::COMMA),
                                            found: token,
                                            span,
                                        });
                                    }
                                }
                            }
                            _ => {
                                return Err(ParserError::UnexpectedToken {
                                    expected: Some(Token::RPAREN),
                                    found: Token::EOF,
                                    span: self.last_span,
                                });
                            }
                        }
                    }
                    callee = Expr::Call(Box::new(callee), Vec::new(), args, Span::new(0, 0));
                }
                Some(Ok((Token::LBRACKET, _))) => {
                    self.next()?;
                    let index = self.expr()?;
                    let (token, span) = self.next()?;
                    match token {
                        Token::RBRACKET => {}
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::RBRACKET),
                                found: token,
                                span,
                            });
                        }
                    }
                    callee = Expr::Index(Box::new(callee), Box::new(index), Span::new(0, 0));
                }
                Some(Ok((Token::DOT, _))) => {
                    self.next()?;
                    let (token, span) = self.next()?;
                    let field_name = match token {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("FIELD NAME".to_string())),
                                found: token,
                                span,
                            });
                        }
                    };
                    callee = Expr::MemberAccess(Box::new(callee), field_name, Span::new(0, 0));
                }
                _ => break,
            }
        }

        Ok(callee)
    }

    fn comparison(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.additive()?;
        while let Some(Ok((op, _))) = self.peek().cloned() {
            match op {
                Token::CEQ | Token::NE | Token::LT | Token::LE | Token::GT | Token::GE => {
                    self.next()?;
                    let right = self.additive()?;
                    let span = left.span();
                    left = match op {
                        Token::CEQ => Expr::Eq(Box::new(left), Box::new(right), span),
                        Token::NE => Expr::Ne(Box::new(left), Box::new(right), span),
                        Token::LT => Expr::Lt(Box::new(left), Box::new(right), span),
                        Token::LE => Expr::Le(Box::new(left), Box::new(right), span),
                        Token::GT => Expr::Gt(Box::new(left), Box::new(right), span),
                        Token::GE => Expr::Ge(Box::new(left), Box::new(right), span),
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

        if let Some(Ok((Token::DOTDOT, _))) = self.peek() {
            self.next()?;
            let right = self.term()?;
            let span = left.span();
            return Ok(Expr::Range(Box::new(left), Box::new(right), span));
        }

        while let Some(Ok((op, _))) = self.peek().cloned() {
            match op {
                Token::PLUS | Token::MINUS => {
                    self.next()?;
                    let right = self.term()?;
                    let span = left.span();
                    left = match op {
                        Token::PLUS => Expr::Add(Box::new(left), Box::new(right), span),
                        Token::MINUS => Expr::Sub(Box::new(left), Box::new(right), span),
                        _ => unreachable!(),
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.prefix()?;
        while let Some(Ok((op, _))) = self.peek().cloned() {
            match op {
                Token::STAR | Token::SLASH | Token::PERCENT => {
                    self.next()?;
                    let right = self.prefix()?;
                    let span = left.span();
                    left = match op {
                        Token::STAR => Expr::Mul(Box::new(left), Box::new(right), span),
                        Token::SLASH => Expr::Div(Box::new(left), Box::new(right), span),
                        Token::PERCENT => Expr::Mod(Box::new(left), Box::new(right), span),
                        _ => unreachable!(),
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Result<Expr, ParserError> {
        match self.peek().cloned() {
            Some(Ok((Token::STAR, span))) => {
                self.next()?;
                let operand = self.prefix()?;
                Ok(Expr::Deref(Box::new(operand), span))
            }
            Some(Ok((Token::LAND, span))) => {
                self.next()?;
                let operand = self.prefix()?;
                Ok(Expr::AddressOf(Box::new(operand), span))
            }
            _ => self.call(),
        }
    }

    fn factor(&mut self) -> Result<Expr, ParserError> {
        if let Some(peeked) = self.peek().cloned() {
            match peeked {
                Ok((Token::INT(n), span)) => {
                    self.next()?;
                    return Ok(Expr::Int(n, span));
                }
                Ok((Token::FLOAT(f), span)) => {
                    self.next()?;
                    return Ok(Expr::Float(f, span));
                }
                Ok((Token::BOOL(b), span)) => {
                    self.next()?;
                    return Ok(Expr::Bool(b, span));
                }
                Ok((Token::STRING(s), span)) => {
                    self.next()?;
                    return Ok(Expr::String(s, span));
                }
                Ok((Token::NIL, span)) => {
                    self.next()?;
                    return Ok(Expr::Nil(span));
                }
                Ok((Token::LAMBDA, span)) => {
                    self.next()?;
                    self.expect(Token::LPAREN)?;
                    let params = self.get_params_list()?;
                    self.expect(Token::RPAREN)?;
                    self.expect(Token::COLON)?;
                    let ret_type = self.parse_type()?;
                    return Ok(Expr::Lambda(params, Box::new(self.expr()?), ret_type, span));
                }
                Ok((Token::LPAREN, _)) => {
                    self.next()?;
                    let expr = self.logical()?;
                    match self.peek().cloned() {
                        Some(Ok((Token::RPAREN, _))) => {
                            self.next()?;
                            return Ok(expr);
                        }
                        Some(Ok((token, span))) => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::RPAREN),
                                found: token,
                                span,
                            });
                        }
                        Some(Err(e)) => return Err(e.into()),
                        None => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::RPAREN),
                                found: Token::EOF,
                                span: self.last_span,
                            });
                        }
                    }
                }
                Ok((Token::LBRACE, span)) => {
                    let mut exprs: Vec<Expr> = Vec::new();
                    self.next()?;
                    let last_span = self.last_span;
                    loop {
                        match self.peek().ok_or(ParserError::UnexpectedToken {
                            expected: Some(Token::RBRACE),
                            found: Token::EOF,
                            span: last_span,
                        })? {
                            Ok((Token::RBRACE, _)) => {
                                self.next()?;
                                break;
                            }
                            Ok((Token::SEMICOLON, _)) => {
                                self.next()?;
                                continue;
                            }
                            Ok((Token::EOF, _)) => {
                                return Err(ParserError::UnexpectedToken {
                                    expected: Some(Token::RBRACE),
                                    found: Token::EOF,
                                    span: self.last_span,
                                });
                            }
                            Err(e) => return Err(ParserError::LexerError(e.to_owned())),
                            _ => {
                                exprs.push(self.expr()?);
                                if let Some(Ok((Token::SEMICOLON, _))) = self.peek() {
                                    self.next()?;
                                }
                            }
                        }
                    }
                    return Ok(Expr::Block(exprs, span));
                }
                Ok((Token::LBRACKET, span)) => {
                    self.next()?;

                    let is_fill_syntax = match self.peek() {
                        Some(Ok((Token::TYPE(_), _))) => true,
                        Some(Ok((Token::IDENT(_), _))) => {
                            matches!(self.peek_n(1), Some(Ok((Token::SEMICOLON, _))))
                        }
                        _ => false,
                    };

                    if is_fill_syntax {
                        let elem_type = self.parse_type()?;
                        self.expect(Token::SEMICOLON)?;
                        let len = self.expr()?;
                        self.expect(Token::RBRACKET)?;
                        return Ok(Expr::ArrayFill(elem_type, Box::new(len), span));
                    } else {
                        let mut elements = Vec::new();
                        loop {
                            match self.peek().cloned() {
                                Some(Ok((Token::RBRACKET, _))) => {
                                    self.next()?;
                                    break;
                                }
                                Some(Ok((_, _))) => {
                                    elements.push(self.expr()?);
                                    let (token, span) = self.next()?;
                                    match token {
                                        Token::COMMA => {}
                                        Token::RBRACKET => break,
                                        token => {
                                            return Err(ParserError::UnexpectedToken {
                                                expected: Some(Token::COMMA),
                                                found: token,
                                                span,
                                            });
                                        }
                                    }
                                }
                                _ => {
                                    return Err(ParserError::UnexpectedToken {
                                        expected: Some(Token::RBRACKET),
                                        found: Token::EOF,
                                        span: self.last_span,
                                    });
                                }
                            }
                        }
                        return Ok(Expr::ArrayLiteral(elements, span));
                    }
                }
                Ok((Token::IDENT(s), span)) => {
                    let name = s.clone();
                    self.next()?;

                    if self.structs.contains_key(&name) {
                        if let Some(Ok((Token::LT, _))) = self.peek() {
                            self.next()?;
                            let type_args = self.get_type_args_list()?;
                            self.expect(Token::GT)?;
                            if let Some(Ok((Token::LBRACE, _))) = self.peek() {
                                self.next()?;
                                let mut fields = Vec::new();
                                loop {
                                    match self.peek().cloned() {
                                        Some(Ok((Token::RBRACE, _))) => {
                                            self.next()?;
                                            break;
                                        }
                                        Some(Ok((Token::IDENT(field_name), _))) => {
                                            self.next()?;
                                            self.expect(Token::COLON)?;
                                            let field_value = self.expr()?;
                                            fields.push((field_name, field_value));
                                            match self.peek().cloned() {
                                                Some(Ok((Token::COMMA, _))) => {
                                                    self.next()?;
                                                }
                                                Some(Ok((Token::RBRACE, _))) => {
                                                    self.next()?;
                                                    break;
                                                }
                                                _ => {
                                                    return Err(ParserError::UnexpectedToken {
                                                        expected: Some(Token::COMMA),
                                                        found: Token::EOF,
                                                        span: self.last_span,
                                                    });
                                                }
                                            }
                                        }
                                        _ => {
                                            return Err(ParserError::UnexpectedToken {
                                                expected: Some(Token::IDENT(
                                                    "FIELD NAME".to_string(),
                                                )),
                                                found: Token::EOF,
                                                span: self.last_span,
                                            });
                                        }
                                    }
                                }
                                return Ok(Expr::StructLiteral(name, type_args, fields, span));
                            }
                        } else if let Some(Ok((Token::LBRACE, _))) = self.peek() {
                            self.next()?;
                            let mut fields = Vec::new();
                            loop {
                                match self.peek().cloned() {
                                    Some(Ok((Token::RBRACE, _))) => {
                                        self.next()?;
                                        break;
                                    }
                                    Some(Ok((Token::IDENT(field_name), _))) => {
                                        self.next()?;
                                        self.expect(Token::COLON)?;
                                        let field_value = self.expr()?;
                                        fields.push((field_name, field_value));
                                        match self.peek().cloned() {
                                            Some(Ok((Token::COMMA, _))) => {
                                                self.next()?;
                                            }
                                            Some(Ok((Token::RBRACE, _))) => {
                                                self.next()?;
                                                break;
                                            }
                                            _ => {
                                                return Err(ParserError::UnexpectedToken {
                                                    expected: Some(Token::COMMA),
                                                    found: Token::EOF,
                                                    span: self.last_span,
                                                });
                                            }
                                        }
                                    }
                                    _ => {
                                        return Err(ParserError::UnexpectedToken {
                                            expected: Some(Token::IDENT("FIELD NAME".to_string())),
                                            found: Token::EOF,
                                            span: self.last_span,
                                        });
                                    }
                                }
                            }
                            return Ok(Expr::StructLiteral(name, Vec::new(), fields, span));
                        }
                    }

                    if let Some(Ok((Token::EQ, _))) = self.peek() {
                        self.next()?;
                        let val = self.expr()?;
                        return Ok(Expr::VarAssign(name, Box::new(val), span));
                    }
                    if let Some(Ok((Token::PLUSEQ, _))) = self.peek() {
                        self.next()?;
                        let val = self.expr()?;
                        return Ok(Expr::AddAssign(name, Box::new(val), span));
                    }
                    if let Some(Ok((Token::MINUSEQ, _))) = self.peek() {
                        self.next()?;
                        let val = self.expr()?;
                        return Ok(Expr::SubAssign(name, Box::new(val), span));
                    }
                    return Ok(Expr::Var(name, span));
                }
                Ok((Token::MINUS, span)) => {
                    self.next()?;
                    let operand = self.factor()?;
                    return Ok(Expr::Neg(Box::new(operand), span));
                }
                Ok((Token::PLUS, _span)) => {
                    self.next()?;
                    let operand = self.factor()?;
                    return Ok(operand);
                }
                Ok((Token::PLUSPLUS, span)) => {
                    self.next()?;
                    match self.peek().cloned() {
                        Some(Ok((Token::IDENT(name), _))) => {
                            self.next()?;
                            return Ok(Expr::Inc(name, span));
                        }
                        _ => {
                            return Err(ParserError::UnexpectedToken {
                                expected: None,
                                found: self.peek().cloned().unwrap().unwrap().0,
                                span,
                            });
                        }
                    }
                }
                Ok((Token::MINUSMINUS, span)) => {
                    self.next()?;
                    match self.peek().cloned() {
                        Some(Ok((Token::IDENT(name), _))) => {
                            self.next()?;
                            return Ok(Expr::Dec(name, span));
                        }
                        _ => {
                            return Err(ParserError::UnexpectedToken {
                                expected: None,
                                found: self.peek().cloned().unwrap().unwrap().0,
                                span,
                            });
                        }
                    }
                }
                Ok((Token::NOT, span)) => {
                    self.next()?;
                    let operand = self.factor()?;
                    return Ok(Expr::Not(Box::new(operand), span));
                }
                Ok((token, span)) => {
                    return Err(ParserError::UnexpectedToken {
                        expected: None,
                        found: token,
                        span,
                    });
                }
                Err(e) => return Err(ParserError::LexerError(e.to_owned())),
            }
        }
        Err(ParserError::UnexpectedToken {
            expected: None,
            found: Token::EOF,
            span: self.last_span,
        })
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParserError> {
        let (token, span) = self.next()?;
        if token == expected {
            Ok(())
        } else {
            Err(ParserError::UnexpectedToken {
                expected: Some(expected),
                found: token,
                span,
            })
        }
    }

    fn get_params_list(&mut self) -> Result<Vec<(String, Type)>, ParserError> {
        let mut params = Vec::new();
        loop {
            match self.peek().cloned() {
                Some(Ok((Token::IDENT(s), _))) => {
                    self.next()?;

                    let next_is_colon = matches!(self.peek(), Some(Ok((Token::COLON, _))));
                    if next_is_colon {
                        self.expect(Token::COLON)?;
                        let ty = self.parse_type()?;
                        params.push((s, ty));
                    } else {
                        let ty = if let Some(idx) = self.lookup_type_param(&s) {
                            Type::Param(idx)
                        } else if let Some(ty) = self.typedefs.get(&s) {
                            ty.clone()
                        } else {
                            let mut args = Vec::new();
                            if matches!(self.peek(), Some(Ok((Token::LT, _)))) {
                                self.next()?;
                                args = self.get_type_args_list()?;
                                self.expect(Token::GT)?;
                            }
                            Type::Struct(s, args)
                        };

                        if matches!(self.peek(), Some(Ok((Token::LBRACKET, _)))) {
                            self.next()?;
                            if matches!(self.peek(), Some(Ok((Token::INT(_), _)))) {
                                self.next()?;
                            }
                            self.expect(Token::RBRACKET)?;
                            params.push(("_anon".to_string(), Type::Array(Box::new(ty))));
                        } else {
                            params.push(("_anon".to_string(), ty));
                        }
                    }

                    match self.peek().cloned() {
                        Some(Ok((Token::COMMA, _))) => {
                            self.next()?;
                        }
                        Some(Ok((Token::RPAREN, _))) => {
                            break;
                        }
                        _ => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::COMMA),
                                found: Token::EOF,
                                span: self.last_span,
                            });
                        }
                    }
                }
                Some(Ok((Token::TYPE(_), _))) => {
                    let ty = self.parse_type()?;
                    params.push(("_anon".to_string(), ty));

                    match self.peek().cloned() {
                        Some(Ok((Token::COMMA, _))) => {
                            self.next()?;
                        }
                        Some(Ok((Token::RPAREN, _))) => {
                            break;
                        }
                        _ => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::COMMA),
                                found: Token::EOF,
                                span: self.last_span,
                            });
                        }
                    }
                }
                Some(Ok((Token::STAR, _))) => {
                    let ty = self.parse_type()?;
                    params.push(("_anon".to_string(), ty));

                    match self.peek().cloned() {
                        Some(Ok((Token::COMMA, _))) => {
                            self.next()?;
                        }
                        Some(Ok((Token::RPAREN, _))) => {
                            break;
                        }
                        _ => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::COMMA),
                                found: Token::EOF,
                                span: self.last_span,
                            });
                        }
                    }
                }
                Some(Ok((Token::RPAREN, _))) => {
                    break;
                }
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        expected: Some(Token::IDENT("PARAM NAME".to_string())),
                        found: Token::EOF,
                        span: self.last_span,
                    });
                }
            }
        }
        Ok(params)
    }

    fn peek(&mut self) -> Option<&Result<(Token, Span), LexerError>> {
        self.peek_n(0)
    }

    fn peek_n(&mut self, n: usize) -> Option<&Result<(Token, Span), LexerError>> {
        while self.lookahead.len() <= n {
            match self.lex.next() {
                Some(tok) => self.lookahead.push(tok),
                None => break,
            }
        }
        self.lookahead.get(n)
    }

    fn lookup_type_param(&self, name: &str) -> Option<usize> {
        self.type_param_scopes
            .last()
            .and_then(|scope| scope.get(name).copied())
    }

    fn push_type_params(&mut self, params: &[String]) {
        let mut scope = HashMap::new();
        for (i, name) in params.iter().enumerate() {
            scope.insert(name.clone(), i);
        }
        self.type_param_scopes.push(scope);
    }

    fn get_type_params_list(&mut self) -> Result<Vec<String>, ParserError> {
        let mut params = Vec::new();
        loop {
            let (token, span) = self.next()?;
            match token {
                Token::IDENT(s) => params.push(s),
                token => {
                    return Err(ParserError::UnexpectedToken {
                        expected: Some(Token::IDENT("TYPE PARAM".to_string())),
                        found: token,
                        span,
                    });
                }
            }
            match self.peek().cloned() {
                Some(Ok((Token::COMMA, _))) => {
                    self.next()?;
                }
                Some(Ok((Token::GT, _))) => break,
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        expected: Some(Token::GT),
                        found: Token::EOF,
                        span: self.last_span,
                    });
                }
            }
        }
        Ok(params)
    }

    fn get_type_args_list(&mut self) -> Result<Vec<Type>, ParserError> {
        let mut args = Vec::new();
        loop {
            args.push(self.parse_type()?);
            match self.peek().cloned() {
                Some(Ok((Token::COMMA, _))) => {
                    self.next()?;
                }
                Some(Ok((Token::GT, _))) => break,
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        expected: Some(Token::GT),
                        found: Token::EOF,
                        span: self.last_span,
                    });
                }
            }
        }
        Ok(args)
    }

    fn parse_type(&mut self) -> Result<Type, ParserError> {
        if let Some(Ok((Token::STAR, _))) = self.peek() {
            self.next()?;
            let inner_type = self.parse_type()?;
            return Ok(Type::Pointer(Box::new(inner_type)));
        }

        let (first_token, span) = self.next()?;

        let base_type = match first_token {
            Token::TYPE(t) => match t.as_str() {
                "int" => Type::Primitive(crate::compiler::parser::Primitive::Int),
                "float" => Type::Primitive(crate::compiler::parser::Primitive::Float),
                "bool" => Type::Primitive(crate::compiler::parser::Primitive::Boolean),
                "string" => Type::Primitive(crate::compiler::parser::Primitive::String),
                "void" => Type::Primitive(crate::compiler::parser::Primitive::Void),
                name => {
                    let mut args = Vec::new();
                    if matches!(self.peek(), Some(Ok((Token::LT, _)))) {
                        self.next()?;
                        args = self.get_type_args_list()?;
                        self.expect(Token::GT)?;
                    }
                    Type::Struct(name.to_string(), args)
                }
            },
            Token::IDENT(s) => {
                if let Some(idx) = self.lookup_type_param(&s) {
                    Type::Param(idx)
                } else if let Some(ty) = self.typedefs.get(&s) {
                    ty.clone()
                } else {
                    let mut args = Vec::new();
                    if matches!(self.peek(), Some(Ok((Token::LT, _)))) {
                        self.next()?;
                        args = self.get_type_args_list()?;
                        self.expect(Token::GT)?;
                    }
                    Type::Struct(s, args)
                }
            }
            token => {
                return Err(ParserError::UnexpectedToken {
                    expected: Some(Token::TYPE("int".to_string())),
                    found: token,
                    span,
                });
            }
        };

        if let Some(Ok((Token::LPAREN, _))) = self.peek() {
            self.next()?;
            let mut params = Vec::new();
            loop {
                match self.peek() {
                    Some(Ok((Token::RPAREN, _))) => {
                        self.next()?;
                        break;
                    }
                    Some(Ok((_, _))) => {
                        params.push(self.parse_type()?);
                        match self.peek() {
                            Some(Ok((Token::COMMA, _))) => {
                                self.next()?;
                            }
                            Some(Ok((Token::RPAREN, _))) => {
                                self.next()?;
                                break;
                            }
                            _ => {
                                return Err(ParserError::UnexpectedToken {
                                    expected: Some(Token::COMMA),
                                    found: Token::EOF,
                                    span: self.last_span,
                                });
                            }
                        }
                    }
                    _ => break,
                }
            }
            return Ok(Type::Function(params, Box::new(base_type)));
        }

        if let Some(Ok((Token::LBRACKET, _))) = self.peek() {
            self.next()?;
            if matches!(self.peek(), Some(Ok((Token::INT(_), _)))) {
                self.next()?;
            }
            self.expect(Token::RBRACKET)?;
            return Ok(Type::Array(Box::new(base_type)));
        }

        Ok(base_type)
    }

    fn next(&mut self) -> Result<(Token, Span), ParserError> {
        let result = if !self.lookahead.is_empty() {
            Some(self.lookahead.remove(0))
        } else {
            self.lex.next()
        };
        if let Some(result) = result {
            match result {
                Ok((token, span)) => {
                    self.last_span = span;
                    Ok((token, span))
                }
                Err(e) => Err(ParserError::LexerError(e)),
            }
        } else {
            Err(ParserError::UnexpectedToken {
                expected: None,
                found: Token::EOF,
                span: self.last_span,
            })
        }
    }
}
