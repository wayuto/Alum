use super::error::ParserError;
use crate::compiler::{
    Span,
    lexer::{FstringSeg, Lexer, LexerError, Token},
    parser::{Expr, FuncAttrs, Parser, Primitive, Program, Type},
};
use std::collections::HashMap;

#[derive(Clone, Copy)]
enum CompoundOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

fn op_line(expr: &Expr) -> usize {
    match expr {
        Expr::Index(base, _, _) | Expr::MemberAccess(base, _, _) | Expr::Call(base, _, _, _) => {
            op_line(base)
        }
        Expr::Deref(inner, _)
        | Expr::AddressOf(inner, _)
        | Expr::Neg(inner, _)
        | Expr::BNot(inner, _) => op_line(inner),
        _ => expr.span().line,
    }
}

fn make_compound_assign(target: Expr, op: CompoundOp, rhs: Expr, span: Span) -> Expr {
    let bin = |l: Expr, r: Expr| {
        let sp = Span::new(0, 0);
        match op {
            CompoundOp::Add => Expr::Add(Box::new(l), Box::new(r), sp),
            CompoundOp::Sub => Expr::Sub(Box::new(l), Box::new(r), sp),
            CompoundOp::Mul => Expr::Mul(Box::new(l), Box::new(r), sp),
            CompoundOp::Div => Expr::Div(Box::new(l), Box::new(r), sp),
            CompoundOp::Mod => Expr::Mod(Box::new(l), Box::new(r), sp),
            CompoundOp::And => Expr::LAnd(Box::new(l), Box::new(r), sp),
            CompoundOp::Or => Expr::LOr(Box::new(l), Box::new(r), sp),
            CompoundOp::Xor => Expr::Xor(Box::new(l), Box::new(r), sp),
            CompoundOp::Shl => Expr::Shl(Box::new(l), Box::new(r), sp),
            CompoundOp::Shr => Expr::Shr(Box::new(l), Box::new(r), sp),
        }
    };
    match &target {
        Expr::Index(arr, idx, _) => {
            let t = Expr::Index(arr.clone(), idx.clone(), Span::new(0, 0));
            let v = bin(t.clone(), rhs);
            Expr::IndexAssign(Box::new(t), Box::new(v), span)
        }
        Expr::MemberAccess(obj, field, _) => {
            let t = Expr::MemberAccess(obj.clone(), field.clone(), Span::new(0, 0));
            let v = bin(t, rhs);
            Expr::MemberAssign(obj.clone(), field.clone(), Box::new(v), span)
        }
        Expr::Deref(ptr, _) => {
            let t = Expr::Deref(ptr.clone(), Span::new(0, 0));
            let v = bin(t, rhs);
            Expr::DerefAssign(ptr.clone(), Box::new(v), span)
        }
        _ => {
            let t = target.clone();
            let v = bin(t.clone(), rhs);
            Expr::IndexAssign(Box::new(t), Box::new(v), span)
        }
    }
}

fn make_inc_dec(target: Expr, is_inc: bool, span: Span) -> Expr {
    let one = Expr::Int(1, Span::new(0, 0));
    make_compound_assign(
        target,
        if is_inc {
            CompoundOp::Add
        } else {
            CompoundOp::Sub
        },
        one,
        span,
    )
}

impl<'a> Parser<'a> {
    pub fn new(lex: Lexer<'a>) -> Self {
        Self {
            lex: lex.peekable(),
            lookahead: Vec::new(),
            last_span: Span::new(1, 1),
            typedefs: HashMap::new(),
            structs: HashMap::new(),
            unions: HashMap::new(),
            enums: HashMap::new(),
            type_param_scopes: Vec::new(),
            has_fstring: false,
            scope_depth: 0,
            deref_depth: 0,
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
        self.append_fstring_helpers(&mut body);
        Ok(Program { body })
    }

    /// Like [`Parser::parse`], but recovers from errors and returns all of
    /// them instead of stopping at the first. On an error, the token stream is
    /// synchronized to the next statement boundary (a `;` or a `}` at brace
    /// depth 0), so parsing resumes cleanly at the next top-level item.
    pub fn parse_collect(&mut self) -> (Program, Vec<ParserError>) {
        let mut body = Vec::new();
        let mut errors = Vec::new();
        loop {
            match self.peek().cloned() {
                None | Some(Ok((Token::EOF, _))) => break,
                Some(Err(le)) => {
                    errors.push(ParserError::LexerError(le));
                    let _ = self.next();
                }
                Some(Ok(_)) => match self.expr() {
                    Ok(expr) => body.push(expr),
                    Err(e) => {
                        errors.push(e);
                        self.synchronize();
                    }
                },
            }
        }
        self.append_fstring_helpers(&mut body);
        (Program { body }, errors)
    }

    /// Skips tokens until a likely restart point for the next statement: a `;`
    /// at brace depth 0, a `}` that closes the enclosing block, or a top-level
    /// declaration keyword (these cannot appear inside a block, so they are
    /// safe to restart from). Used for error recovery after a failed statement.
    fn synchronize(&mut self) {
        let mut depth: i32 = 0;
        loop {
            match self.peek().cloned() {
                None | Some(Err(_)) => {
                    let _ = self.next();
                }
                Some(Ok((Token::EOF, _))) => break,
                Some(Ok((Token::SEMICOLON, _))) if depth == 0 => {
                    let _ = self.next();
                    break;
                }
                Some(Ok((
                    Token::FUN
                    | Token::STRUCT
                    | Token::UNION
                    | Token::ENUM
                    | Token::TYPEDEF
                    | Token::EXTERN,
                    _,
                ))) if depth == 0 => break,
                Some(Ok((Token::LBRACE, _))) => {
                    depth += 1;
                    let _ = self.next();
                }
                Some(Ok((Token::RBRACE, _))) => {
                    depth -= 1;
                    let _ = self.next();
                    if depth < 0 {
                        break;
                    }
                }
                Some(Ok((_, _))) => {
                    let _ = self.next();
                }
            }
        }
    }

    fn append_fstring_helpers(&mut self, body: &mut Vec<Expr>) {
        if self.has_fstring {
            let mut itoa_attrs = FuncAttrs::default();
            itoa_attrs.is_external = true;
            body.push(Expr::FuncDecl(
                "itoa".to_string(),
                itoa_attrs,
                Vec::new(),
                vec![("n".to_string(), Type::Primitive(Primitive::Int))],
                Type::Primitive(Primitive::String),
                Box::new(Expr::Nil(Span::new(0, 0))),
                Span::new(0, 0),
            ));
            let mut ftoa_attrs = FuncAttrs::default();
            ftoa_attrs.is_external = true;
            body.push(Expr::FuncDecl(
                "ftoa".to_string(),
                ftoa_attrs,
                Vec::new(),
                vec![("n".to_string(), Type::Primitive(Primitive::Float))],
                Type::Primitive(Primitive::String),
                Box::new(Expr::Nil(Span::new(0, 0))),
                Span::new(0, 0),
            ));
        }
    }

    fn parse_sub_expr(&self, src: &str, span: Span) -> Result<Expr, ParserError> {
        let src = src.trim();
        if src.is_empty() {
            return Err(ParserError::UnexpectedToken {
                expected: None,
                found: Token::EOF,
                span,
            });
        }
        let lex = Lexer::new(src);
        let mut sub = Parser {
            lex: lex.peekable(),
            lookahead: Vec::new(),
            last_span: span,
            typedefs: self.typedefs.clone(),
            structs: self.structs.clone(),
            unions: self.unions.clone(),
            enums: self.enums.clone(),
            type_param_scopes: self.type_param_scopes.clone(),
            has_fstring: false,
            deref_depth: 0,
            scope_depth: self.scope_depth,
        };
        sub.expr()
    }

    fn expr(&mut self) -> Result<Expr, ParserError> {
        if let Some(token) = self.peek().cloned() {
            match token {
                Ok((Token::VAR, _)) => {
                    self.next()?;
                    let is_pub = if self.scope_depth == 0 {
                        self.parse_global_annotation("var")?
                    } else {
                        if matches!(self.peek(), Some(Ok((Token::LPAREN, _)))) {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("NAME".to_string())),
                                found: Token::LPAREN,
                                span: self.last_span,
                            });
                        }
                        false
                    };
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
                    let init = if matches!(self.peek(), Some(Ok((Token::EQ, _)))) {
                        self.next()?;
                        Some(Box::new(self.expr()?))
                    } else {
                        None
                    };
                    if self.scope_depth == 0 {
                        Ok(Expr::GlobalVar(name, is_pub, ty, init, span))
                    } else {
                        let init = match init {
                            Some(init) => init,
                            None => {
                                return Err(ParserError::UnexpectedToken {
                                    expected: Some(Token::EQ),
                                    found: match self.peek().cloned() {
                                        Some(Ok((t, _))) => t,
                                        _ => Token::EOF,
                                    },
                                    span,
                                });
                            }
                        };
                        Ok(Expr::VarDecl(name, ty, init, span))
                    }
                }
                Ok((Token::CST, _)) => {
                    self.next()?;
                    let is_pub = self.parse_global_annotation("cst")?;
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

                    Ok(Expr::ConstDecl(
                        name,
                        ty,
                        Box::new(self.expr()?),
                        is_pub,
                        span,
                    ))
                }
                Ok((Token::FUN, _)) => {
                    self.next()?;
                    let mut attrs = FuncAttrs::default();
                    if matches!(self.peek(), Some(Ok((Token::LPAREN, _)))) {
                        self.next()?;
                        loop {
                            let (token, span) = self.next()?;
                            match token {
                                Token::IDENT(s) if s == "pub" => attrs.is_pub = true,
                                Token::IDENT(s) if s == "pure" => attrs.is_pure = true,
                                Token::EXTERN => attrs.is_external = true,
                                token => {
                                    return Err(ParserError::UnexpectedToken {
                                        expected: Some(Token::IDENT(
                                            "ANNOTATION (pub|extern|pure)".to_string(),
                                        )),
                                        found: token,
                                        span,
                                    });
                                }
                            }
                            match self.peek().cloned() {
                                Some(Ok((Token::COMMA, _))) => {
                                    self.next()?;
                                }
                                Some(Ok((Token::RPAREN, _))) => {
                                    self.next()?;
                                    break;
                                }
                                Some(Ok((token, span))) => {
                                    return Err(ParserError::UnexpectedToken {
                                        expected: Some(Token::COMMA),
                                        found: token,
                                        span,
                                    });
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
                    }
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
                    let body = if attrs.is_external {
                        if !type_params.is_empty() {
                            self.type_param_scopes.pop();
                        }
                        Box::new(Expr::Nil(Span::new(0, 0)))
                    } else {
                        self.scope_depth += 1;
                        let body = self.expr()?;
                        self.scope_depth -= 1;
                        if !type_params.is_empty() {
                            self.type_param_scopes.pop();
                        }
                        Box::new(body)
                    };
                    Ok(Expr::FuncDecl(
                        name,
                        attrs,
                        type_params,
                        params,
                        ret_type,
                        body,
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
                    match self.peek() {
                        Some(Ok((Token::COLON, _))) => {
                            self.next()?;
                            let ty = self.parse_type()?;
                            Ok(Expr::ExternVar(name, ty, span))
                        }
                        Some(Ok((Token::LPAREN, _))) => Err(ParserError::UnexpectedToken {
                            expected: Some(Token::IDENT(
                                "': ' for an extern variable (functions use fun(extern) ...)"
                                    .to_string(),
                            )),
                            found: Token::LPAREN,
                            span,
                        }),
                        Some(Ok((token, found_span))) => Err(ParserError::UnexpectedToken {
                            expected: Some(Token::COLON),
                            found: token.clone(),
                            span: *found_span,
                        }),
                        _ => Err(ParserError::UnexpectedToken {
                            expected: Some(Token::COLON),
                            found: Token::EOF,
                            span,
                        }),
                    }
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
                Ok((Token::MATCH, span_)) => {
                    self.next()?;
                    let var = self.expr()?;
                    self.expect(Token::LBRACE)?;
                    let mut cases: Vec<(Expr, Expr)> = Vec::new();
                    let mut default: Option<Box<Expr>> = None;
                    loop {
                        match self.peek().cloned() {
                            Some(Ok((Token::RBRACE, _))) => {
                                self.next()?;
                                break;
                            }
                            Some(Ok((Token::IDENT(s), _))) if s == "_".to_string() => {
                                self.next()?;
                                self.expect(Token::COLON)?;
                                default = Some(Box::new(self.expr()?));
                                self.expect(Token::RBRACE)?;
                                break;
                            }
                            Some(Ok((_, _))) => {
                                let case = self.expr()?;
                                self.expect(Token::COLON)?;
                                let ret = self.expr()?;
                                cases.push((case, ret));
                            }
                            Some(Err(e)) => return Err(ParserError::LexerError(e.clone())),
                            None => {
                                return Err(ParserError::UnexpectedToken {
                                    expected: Some(Token::RBRACE),
                                    found: Token::EOF,
                                    span: self.last_span,
                                });
                            }
                        }
                    }
                    Ok(Expr::Match(Box::new(var), cases, default, span_))
                }
                Ok((Token::RET, span)) => {
                    self.next()?;
                    let val = match self.peek() {
                        Some(Ok((Token::SEMICOLON, _))) | Some(Ok((Token::RBRACE, _))) | None => {
                            Box::new(Expr::Nil(span))
                        }
                        Some(Ok((_, next_span))) if next_span.line > span.line => {
                            Box::new(Expr::Nil(span))
                        }
                        _ => Box::new(self.expr()?),
                    };
                    Ok(Expr::Return(val, span))
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

                Ok((Token::UNION, _)) => {
                    self.next()?;
                    let (token, span) = self.next()?;
                    let name = match token {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("UNION NAME".to_string())),
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
                    self.unions
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                    Ok(Expr::Union(name, type_params, fields, span))
                }

                Ok((Token::ENUM, span)) => {
                    self.next()?;
                    let (token, name_span) = self.next()?;
                    let name = match token {
                        Token::IDENT(s) => s,
                        token => {
                            return Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT("ENUM NAME".to_string())),
                                found: token,
                                span: name_span,
                            });
                        }
                    };
                    self.expect(Token::LBRACE)?;
                    let mut members = Vec::new();
                    let mut next_value: isize = 0;
                    loop {
                        match self.peek().cloned() {
                            Some(Ok((Token::RBRACE, _))) => {
                                self.next()?;
                                break;
                            }
                            Some(Ok((Token::IDENT(member_name), _))) => {
                                self.next()?;
                                let value = if matches!(self.peek(), Some(Ok((Token::EQ, _)))) {
                                    self.next()?;
                                    let (token, val_span) = self.next()?;
                                    match token {
                                        Token::INT(n) => n,
                                        Token::MINUS => {
                                            let (tok2, sp2) = self.next()?;
                                            match tok2 {
                                                Token::INT(n) => -n,
                                                token => {
                                                    return Err(ParserError::UnexpectedToken {
                                                        expected: Some(Token::INT(0)),
                                                        found: token,
                                                        span: sp2,
                                                    });
                                                }
                                            }
                                        }
                                        token => {
                                            return Err(ParserError::UnexpectedToken {
                                                expected: Some(Token::INT(0)),
                                                found: token,
                                                span: val_span,
                                            });
                                        }
                                    }
                                } else {
                                    next_value
                                };
                                next_value = value + 1;
                                members.push((member_name, value));
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
                                    expected: Some(Token::IDENT("ENUM MEMBER".to_string())),
                                    found: Token::EOF,
                                    span: self.last_span,
                                });
                            }
                        }
                    }
                    self.enums.insert(name.clone(), members.clone());
                    Ok(Expr::Enum(name, members, span))
                }

                Ok((_, span)) => {
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
                    if let Some(e) = self.try_compound_assign(&expr, span)? {
                        return Ok(e);
                    }
                    Ok(expr)
                }
                Err(e) => Err(ParserError::LexerError(e.clone())),
            }
        } else {
            unreachable!()
        }
    }

    fn on_same_line(op_line: usize, left_line: usize) -> bool {
        left_line == 0 || op_line == left_line
    }

    fn compound_op_for_token(tok: &Token) -> Option<CompoundOp> {
        match tok {
            Token::PLUSEQ => Some(CompoundOp::Add),
            Token::MINUSEQ => Some(CompoundOp::Sub),
            Token::STAREQ => Some(CompoundOp::Mul),
            Token::SLASHEQ => Some(CompoundOp::Div),
            Token::PERCENTEQ => Some(CompoundOp::Mod),
            Token::ANDEQ => Some(CompoundOp::And),
            Token::OREQ => Some(CompoundOp::Or),
            Token::XOREQ => Some(CompoundOp::Xor),
            Token::SHLEQ => Some(CompoundOp::Shl),
            Token::SHREQ => Some(CompoundOp::Shr),
            _ => None,
        }
    }

    fn try_compound_assign(
        &mut self,
        expr: &Expr,
        span: Span,
    ) -> Result<Option<Expr>, ParserError> {
        let op = match self.peek().cloned() {
            Some(Ok((tok, _))) => Self::compound_op_for_token(&tok),
            _ => None,
        };
        if let Some(op) = op {
            self.next()?;
            let val = self.expr()?;
            return Ok(Some(make_compound_assign(expr.clone(), op, val, span)));
        }
        Ok(None)
    }

    fn try_compound_assign_name(
        &mut self,
        name: &str,
        span: Span,
    ) -> Result<Option<Expr>, ParserError> {
        let op = match self.peek().cloned() {
            Some(Ok((tok, _))) => Self::compound_op_for_token(&tok),
            _ => None,
        };
        if let Some(op) = op {
            self.next()?;
            let val = self.expr()?;
            let assign = match op {
                CompoundOp::Add => Expr::AddAssign(name.to_string(), Box::new(val), span),
                CompoundOp::Sub => Expr::SubAssign(name.to_string(), Box::new(val), span),
                CompoundOp::Mul => Expr::MulAssign(name.to_string(), Box::new(val), span),
                CompoundOp::Div => Expr::DivAssign(name.to_string(), Box::new(val), span),
                CompoundOp::Mod => Expr::ModAssign(name.to_string(), Box::new(val), span),
                CompoundOp::And => Expr::AndAssign(name.to_string(), Box::new(val), span),
                CompoundOp::Or => Expr::OrAssign(name.to_string(), Box::new(val), span),
                CompoundOp::Xor => Expr::XorAssign(name.to_string(), Box::new(val), span),
                CompoundOp::Shl => Expr::ShlAssign(name.to_string(), Box::new(val), span),
                CompoundOp::Shr => Expr::ShrAssign(name.to_string(), Box::new(val), span),
            };
            return Ok(Some(assign));
        }
        Ok(None)
    }

    fn logical(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.bitwise()?;
        while let Some(Ok((op, span))) = self.peek().cloned() {
            if !Self::on_same_line(span.line, left.span().line) {
                break;
            }
            match op {
                Token::AND | Token::OR | Token::LAND | Token::LOR => {
                    self.next()?;
                    let right = self.bitwise()?;
                    let span = left.span();
                    left = match op {
                        Token::AND | Token::LAND => {
                            Expr::LAnd(Box::new(left), Box::new(right), span)
                        }
                        Token::OR | Token::LOR => Expr::LOr(Box::new(left), Box::new(right), span),
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
        while let Some(Ok((op, span))) = self.peek().cloned() {
            if !Self::on_same_line(span.line, left.span().line) {
                break;
            }
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
                Some(Ok((Token::LPAREN, span))) => {
                    if !Self::on_same_line(span.line, op_line(&callee)) {
                        break;
                    }
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
                Some(Ok((Token::LBRACKET, span))) => {
                    if !Self::on_same_line(span.line, op_line(&callee)) {
                        break;
                    }
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
                Some(Ok((Token::DOT, span))) => {
                    if !Self::on_same_line(span.line, op_line(&callee)) {
                        break;
                    }
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
                Some(Ok((Token::AT, span))) => {
                    if !Self::on_same_line(span.line, op_line(&callee)) {
                        break;
                    }
                    self.next()?;
                    let target_type = self.parse_type()?;
                    callee = Expr::Cast(Box::new(callee), target_type, Span::new(0, 0));
                }
                Some(Ok((Token::PLUSPLUS, span))) => {
                    if Self::on_same_line(span.line, op_line(&callee)) {
                        self.next()?;
                        callee = match callee {
                            Expr::Var(name, _) => Expr::Inc(name, span),
                            other => make_inc_dec(other, true, span),
                        };
                    } else {
                        break;
                    }
                }
                Some(Ok((Token::MINUSMINUS, span))) => {
                    if Self::on_same_line(span.line, op_line(&callee)) {
                        self.next()?;
                        callee = match callee {
                            Expr::Var(name, _) => Expr::Dec(name, span),
                            other => make_inc_dec(other, false, span),
                        };
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(callee)
    }

    fn comparison(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.shift()?;
        while let Some(Ok((op, span))) = self.peek().cloned() {
            if !Self::on_same_line(span.line, left.span().line) {
                break;
            }
            match op {
                Token::CEQ | Token::NE | Token::LT | Token::LE | Token::GT | Token::GE => {
                    self.next()?;
                    let right = self.shift()?;
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

    fn shift(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.additive()?;
        while let Some(Ok((op, span))) = self.peek().cloned() {
            if !Self::on_same_line(span.line, left.span().line) {
                break;
            }
            match op {
                Token::SHL | Token::SHR => {
                    self.next()?;
                    let right = self.additive()?;
                    let span = left.span();
                    left = match op {
                        Token::SHL => Expr::Shl(Box::new(left), Box::new(right), span),
                        Token::SHR => Expr::Shr(Box::new(left), Box::new(right), span),
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

        if let Some(Ok((Token::DOTDOT, span))) = self.peek().cloned() {
            if span.line == 0 || Self::on_same_line(span.line, left.span().line) {
                self.next()?;
                let right = self.term()?;
                let span = left.span();
                return Ok(Expr::Range(Box::new(left), Box::new(right), span));
            }
        }

        while let Some(Ok((op, span))) = self.peek().cloned() {
            if !Self::on_same_line(span.line, left.span().line) {
                break;
            }
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
        while let Some(Ok((op, span))) = self.peek().cloned() {
            if !Self::on_same_line(span.line, left.span().line) {
                break;
            }
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
                self.deref_depth += 1;
                let operand = self.prefix()?;
                self.deref_depth -= 1;
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
                Ok((Token::FSTRING(segs), span)) => {
                    self.next()?;
                    self.has_fstring = true;
                    let mut parts: Vec<Expr> = Vec::new();
                    for seg in segs {
                        match seg {
                            FstringSeg::Lit(s) => parts.push(Expr::String(s, span)),
                            FstringSeg::Expr(raw) => {
                                parts.push(self.parse_sub_expr(&raw, span)?);
                            }
                        }
                    }
                    return Ok(Expr::FString(parts, span));
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
                    self.scope_depth += 1;
                    let body = self.expr()?;
                    self.scope_depth -= 1;
                    return Ok(Expr::Lambda(params, Box::new(body), ret_type, span));
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
                    self.scope_depth += 1;
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
                    self.scope_depth -= 1;
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

                    if self.structs.contains_key(&name) || self.unions.contains_key(&name) {
                        let is_union = self.unions.contains_key(&name);
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
                                if is_union {
                                    return Ok(Expr::UnionLiteral(name, type_args, fields, span));
                                } else {
                                    return Ok(Expr::StructLiteral(name, type_args, fields, span));
                                }
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
                            if is_union {
                                return Ok(Expr::UnionLiteral(name, Vec::new(), fields, span));
                            } else {
                                return Ok(Expr::StructLiteral(name, Vec::new(), fields, span));
                            }
                        }
                    }

                    if self.deref_depth == 0 {
                        if let Some(Ok((Token::EQ, _))) = self.peek() {
                            self.next()?;
                            let val = self.expr()?;
                            return Ok(Expr::VarAssign(name, Box::new(val), span));
                        }
                        if let Some(e) = self.try_compound_assign_name(&name, span)? {
                            return Ok(e);
                        }
                    }
                    if self.deref_depth == 0 {
                        if let Some(Ok((Token::PLUSPLUS, op_span))) = self.peek() {
                            if Self::on_same_line(op_span.line, span.line) {
                                self.next()?;
                                return Ok(Expr::Inc(name, span));
                            }
                        }
                        if let Some(Ok((Token::MINUSMINUS, op_span))) = self.peek() {
                            if Self::on_same_line(op_span.line, span.line) {
                                self.next()?;
                                return Ok(Expr::Dec(name, span));
                            }
                        }
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
                    let operand = self.prefix()?;
                    return Ok(match operand {
                        Expr::Var(name, _) => Expr::Inc(name, span),
                        other => make_inc_dec(other, true, span),
                    });
                }
                Ok((Token::MINUSMINUS, span)) => {
                    self.next()?;
                    let operand = self.prefix()?;
                    return Ok(match operand {
                        Expr::Var(name, _) => Expr::Dec(name, span),
                        other => make_inc_dec(other, false, span),
                    });
                }
                Ok((Token::NOT, span)) => {
                    self.next()?;
                    let operand = self.factor()?;
                    return Ok(Expr::Not(Box::new(operand), span));
                }
                Ok((Token::BNOT, span)) => {
                    self.next()?;
                    let operand = self.factor()?;
                    return Ok(Expr::BNot(Box::new(operand), span));
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

    fn parse_global_annotation(&mut self, kw: &str) -> Result<bool, ParserError> {
        let mut is_pub = false;
        if matches!(self.peek(), Some(Ok((Token::LPAREN, _)))) {
            self.next()?;
            let (token, span) = self.next()?;
            match token {
                Token::IDENT(s) if s == "pub" => is_pub = true,
                token => {
                    return Err(ParserError::UnexpectedToken {
                        expected: Some(Token::IDENT("ANNOTATION (pub)".to_string())),
                        found: token,
                        span,
                    });
                }
            }
            self.expect(Token::RPAREN)?;
        }
        if !matches!(self.peek(), Some(Ok((Token::IDENT(_), _)))) {
            return Err(ParserError::UnexpectedToken {
                expected: Some(Token::IDENT(format!("{}(pub) NAME", kw).to_string())),
                found: match self.peek().cloned() {
                    Some(Ok((t, _))) => t,
                    _ => Token::EOF,
                },
                span: self.last_span,
            });
        }
        Ok(is_pub)
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
                            if self.unions.contains_key(&s) {
                                Type::Union(s, args)
                            } else {
                                Type::Struct(s, args)
                            }
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
                "int" => Type::Primitive(Primitive::Int),
                "float" => Type::Primitive(Primitive::Float),
                "bool" => Type::Primitive(Primitive::Boolean),
                "string" => Type::Primitive(Primitive::String),
                "void" => Type::Primitive(Primitive::Void),
                name => {
                    let mut args = Vec::new();
                    if matches!(self.peek(), Some(Ok((Token::LT, _)))) {
                        self.next()?;
                        args = self.get_type_args_list()?;
                        self.expect(Token::GT)?;
                    }
                    if self.unions.contains_key(name) {
                        Type::Union(name.to_string(), args)
                    } else if self.enums.contains_key(name) {
                        Type::Primitive(Primitive::Int)
                    } else {
                        Type::Struct(name.to_string(), args)
                    }
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
                    if self.unions.contains_key(&s) {
                        Type::Union(s, args)
                    } else if self.enums.contains_key(&s) {
                        Type::Primitive(Primitive::Int)
                    } else {
                        Type::Struct(s, args)
                    }
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
