use super::error::ParserError;
use super::parser::{CompoundOp, make_compound_assign, make_inc_dec};
use crate::compiler::{
    Span,
    lexer::{FstringSeg, Lexer, Token},
    parser::{Expr, FuncAttrs, Parser, Type},
};

impl<'a> Parser<'a> {
    pub(super) fn parse_sub_expr(&mut self, src: &str, span: Span) -> Result<Expr, ParserError> {
        let src = src.trim();
        if src.is_empty() {
            return Err(ParserError::UnexpectedToken {
                expected: None,
                found: self.found_token_or_eof(),
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
            modules: self.modules.clone(),
            base_path: self.base_path.clone(),
            alias_map: self.alias_map.clone(),
            from_alias: self.from_alias.clone(),
            own_decls: Vec::new(),
            deferred_module_decls: Vec::new(),
            decl_pub: false,
            expr_depth: 0,
        };
        let r = sub.expr();
        self.deferred_module_decls.extend(sub.deferred_module_decls);
        self.has_fstring |= sub.has_fstring;
        r
    }

    pub(super) fn expr(&mut self) -> Result<Expr, ParserError> {
        const MAX_EXPR_DEPTH: usize = 2000;
        if self.expr_depth > MAX_EXPR_DEPTH {
            return Err(ParserError::ModuleError {
                message: format!("expression nesting exceeds {} levels", MAX_EXPR_DEPTH),
                span: Some(self.last_span),
            });
        }
        self.expr_depth += 1;
        let result = self.expr_inner();
        self.expr_depth -= 1;
        result
    }

    pub(super) fn expr_inner(&mut self) -> Result<Expr, ParserError> {
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
                    if self.scope_depth > 0 && matches!(self.peek(), Some(Ok((Token::LPAREN, _)))) {
                        return Err(ParserError::UnexpectedToken {
                            expected: Some(Token::IDENT("NAME".to_string())),
                            found: Token::LPAREN,
                            span: self.last_span,
                        });
                    }
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
                                        found: self.found_token_or_eof(),
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
                    if attrs.is_external {
                        attrs.link_name = Some(name.clone());
                    }
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
                    let signature = self.parse_fun_signature(attrs.is_external);
                    if !type_params.is_empty() {
                        self.type_param_scopes.pop();
                    }
                    let (params, ret_type, body) = signature?;
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
                            found: self.found_token_or_eof(),
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
                                    found: self.found_token_or_eof(),
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
                    self.decl_pub = self.parse_global_annotation("struct")?;
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
                    let parsed_fields = self.parse_field_list();
                    if !type_params.is_empty() {
                        self.type_param_scopes.pop();
                    }
                    let fields = parsed_fields?;
                    self.structs
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                    Ok(Expr::Struct(name, type_params, fields, span))
                }

                Ok((Token::UNION, _)) => {
                    self.next()?;
                    self.decl_pub = self.parse_global_annotation("union")?;
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
                    let parsed_fields = self.parse_field_list();
                    if !type_params.is_empty() {
                        self.type_param_scopes.pop();
                    }
                    let fields = parsed_fields?;
                    self.unions
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                    Ok(Expr::Union(name, type_params, fields, span))
                }

                Ok((Token::ENUM, span)) => {
                    self.next()?;
                    self.decl_pub = self.parse_global_annotation("enum")?;
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
                                next_value =
                                    value.checked_add(1).ok_or(ParserError::UnexpectedToken {
                                        expected: None,
                                        found: Token::INT(value),
                                        span: self.last_span,
                                    })?;
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
                                            found: self.found_token_or_eof(),
                                            span: self.last_span,
                                        });
                                    }
                                }
                            }
                            _ => {
                                return Err(ParserError::UnexpectedToken {
                                    expected: Some(Token::IDENT("ENUM MEMBER".to_string())),
                                    found: self.found_token_or_eof(),
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
                            _ => Err(ParserError::UnexpectedToken {
                                expected: Some(Token::IDENT(
                                    "assignable target (variable, index, member or deref)"
                                        .to_string(),
                                )),
                                found: Token::EQ,
                                span,
                            }),
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
            Err(ParserError::UnexpectedToken {
                expected: Some(Token::IDENT("expression".to_string())),
                found: self.found_token_or_eof(),
                span: self.last_span,
            })
        }
    }

    pub(super) fn compound_op_for_token(tok: &Token) -> Option<CompoundOp> {
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

    pub(super) fn try_compound_assign(
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
            return Ok(Some(make_compound_assign(expr.clone(), op, val, span)?));
        }
        Ok(None)
    }

    pub(super) fn try_compound_assign_name(
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

    pub(super) fn logical(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.logical_and()?;
        while let Some(Ok((op, _))) = self.peek().cloned() {
            match op {
                Token::OR | Token::LOR => {
                    self.next()?;
                    let right = self.logical_and()?;
                    let span = left.span();
                    left = Expr::LOr(Box::new(left), Box::new(right), span);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    pub(super) fn logical_and(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.comparison()?;
        while let Some(Ok((op, _))) = self.peek().cloned() {
            match op {
                Token::AND | Token::LAND => {
                    self.next()?;
                    let right = self.comparison()?;
                    let span = left.span();
                    left = Expr::LAnd(Box::new(left), Box::new(right), span);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    pub(super) fn bitwise(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.shift()?;
        while let Some(Ok((op, _))) = self.peek().cloned() {
            match op {
                Token::XOR => {
                    self.next()?;
                    let right = self.shift()?;
                    let span = left.span();
                    left = Expr::Xor(Box::new(left), Box::new(right), span);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    pub(super) fn call(&mut self) -> Result<Expr, ParserError> {
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
                                    found: self.found_token_or_eof(),
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
                Some(Ok((Token::AT, _))) => {
                    self.next()?;
                    let target_type = self.parse_type()?;
                    callee = Expr::Cast(Box::new(callee), target_type, Span::new(0, 0));
                }
                Some(Ok((Token::PLUSPLUS, span))) => {
                    self.next()?;
                    callee = match callee {
                        Expr::Var(name, _) => Expr::Inc(name, span),
                        other => make_inc_dec(other, true, span)?,
                    };
                }
                Some(Ok((Token::MINUSMINUS, span))) => {
                    self.next()?;
                    callee = match callee {
                        Expr::Var(name, _) => Expr::Dec(name, span),
                        other => make_inc_dec(other, false, span)?,
                    };
                }
                _ => break,
            }
        }

        Ok(callee)
    }

    pub(super) fn comparison(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.bitwise()?;
        while let Some(Ok((op, _))) = self.peek().cloned() {
            match op {
                Token::CEQ | Token::NE | Token::LT | Token::LE | Token::GT | Token::GE => {
                    self.next()?;
                    let right = self.bitwise()?;
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

    pub(super) fn shift(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.additive()?;
        while let Some(Ok((op, _))) = self.peek().cloned() {
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

    pub(super) fn additive(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.term()?;

        if self
            .peek()
            .cloned()
            .map(|r| matches!(r, Ok((Token::DOTDOT, _))))
            .unwrap_or(false)
        {
            self.next()?;
            let right = self.additive()?;
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

    pub(super) fn term(&mut self) -> Result<Expr, ParserError> {
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

    pub(super) fn prefix(&mut self) -> Result<Expr, ParserError> {
        const MAX_EXPR_DEPTH: usize = 4000;
        if self.expr_depth > MAX_EXPR_DEPTH {
            return Err(ParserError::ModuleError {
                message: format!("expression nesting exceeds {} levels", MAX_EXPR_DEPTH),
                span: Some(self.last_span),
            });
        }
        self.expr_depth += 1;
        let result = self.prefix_inner();
        self.expr_depth -= 1;
        result
    }

    pub(super) fn prefix_inner(&mut self) -> Result<Expr, ParserError> {
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

    pub(super) fn factor(&mut self) -> Result<Expr, ParserError> {
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
                    let body = self.expr();
                    self.scope_depth -= 1;
                    return Ok(Expr::Lambda(params, Box::new(body?), ret_type, span));
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
                                found: self.found_token_or_eof(),
                                span: self.last_span,
                            });
                        }
                    }
                }
                Ok((Token::LBRACE, span)) => {
                    self.next()?;
                    self.scope_depth += 1;
                    let block = self.parse_block_stmts(span);
                    self.scope_depth -= 1;
                    return block;
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
                                        found: self.found_token_or_eof(),
                                        span: self.last_span,
                                    });
                                }
                            }
                        }
                        return Ok(Expr::ArrayLiteral(elements, span));
                    }
                }
                Ok((Token::IDENT(s), span)) | Ok((Token::TYPE(s), span)) => {
                    let is_type_kw = matches!(self.peek(), Some(Ok((Token::TYPE(_), _))));
                    let mut name = s.clone();
                    self.next()?;

                    if matches!(self.peek(), Some(Ok((Token::COLONCOLON, _)))) {
                        let mut segs = vec![name.clone()];
                        while matches!(self.peek(), Some(Ok((Token::COLONCOLON, _))))
                            && matches!(self.peek_n(1), Some(Ok((Token::IDENT(_), _))))
                        {
                            self.next()?;
                            let (s, _) = self.parse_ident()?;
                            segs.push(s);
                        }
                        if segs.len() > 1 {
                            if let Some(t) = self.alias_map.get(&segs[0]).cloned() {
                                segs[0] = t;
                            }
                            name = self.resolve_path_chain(&segs)?;
                        } else if let Some((target, _)) = self.from_alias.get(&name).cloned() {
                            name = target;
                        }
                    } else if let Some((target, _)) = self.from_alias.get(&name).cloned() {
                        name = target;
                    } else if is_type_kw {
                        return Err(ParserError::UnexpectedToken {
                            expected: None,
                            found: Token::TYPE(name.clone()),
                            span,
                        });
                    }

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
                                                        found: self.found_token_or_eof(),
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
                                                found: self.found_token_or_eof(),
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
                                                    found: self.found_token_or_eof(),
                                                    span: self.last_span,
                                                });
                                            }
                                        }
                                    }
                                    _ => {
                                        return Err(ParserError::UnexpectedToken {
                                            expected: Some(Token::IDENT("FIELD NAME".to_string())),
                                            found: self.found_token_or_eof(),
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
                        if let Some(Ok((Token::PLUSPLUS, _))) = self.peek() {
                            self.next()?;
                            return Ok(Expr::Inc(name, span));
                        }
                        if let Some(Ok((Token::MINUSMINUS, _))) = self.peek() {
                            self.next()?;
                            return Ok(Expr::Dec(name, span));
                        }
                    }
                    return Ok(Expr::Var(name, span));
                }
                Ok((Token::MINUS, span)) => {
                    self.next()?;
                    let operand = self.call();
                    return Ok(Expr::Neg(Box::new(operand?), span));
                }
                Ok((Token::PLUS, _span)) => {
                    self.next()?;
                    let operand = self.call();
                    return Ok(operand?);
                }
                Ok((Token::PLUSPLUS, span)) => {
                    self.next()?;
                    let operand = self.prefix()?;
                    return Ok(match operand {
                        Expr::Var(name, _) => Expr::Inc(name, span),
                        other => make_inc_dec(other, true, span)?,
                    });
                }
                Ok((Token::MINUSMINUS, span)) => {
                    self.next()?;
                    let operand = self.prefix()?;
                    return Ok(match operand {
                        Expr::Var(name, _) => Expr::Dec(name, span),
                        other => make_inc_dec(other, false, span)?,
                    });
                }
                Ok((Token::NOT, span)) => {
                    self.next()?;
                    let operand = self.call();
                    return Ok(Expr::Not(Box::new(operand?), span));
                }
                Ok((Token::BNOT, span)) => {
                    self.next()?;
                    let operand = self.call();
                    return Ok(Expr::BNot(Box::new(operand?), span));
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
            found: self.found_token_or_eof(),
            span: self.last_span,
        })
    }
}
