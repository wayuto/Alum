use super::error::ParserError;
use crate::compiler::{
    Span,
    lexer::{Lexer, Token},
    modules::{DeclKind, ModuleLoader},
    parser::{Expr, Parser, Primitive, Program, Type},
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub(super) enum CompoundOp {
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

pub(super) fn make_compound_assign(
    target: Expr,
    op: CompoundOp,
    rhs: Expr,
    span: Span,
) -> Result<Expr, ParserError> {
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
            Ok(Expr::IndexAssign(Box::new(t), Box::new(v), span))
        }
        Expr::MemberAccess(obj, field, _) => {
            let t = Expr::MemberAccess(obj.clone(), field.clone(), Span::new(0, 0));
            let v = bin(t, rhs);
            Ok(Expr::MemberAssign(
                obj.clone(),
                field.clone(),
                Box::new(v),
                span,
            ))
        }
        Expr::Deref(ptr, _) => {
            let t = Expr::Deref(ptr.clone(), Span::new(0, 0));
            let v = bin(t, rhs);
            Ok(Expr::DerefAssign(ptr.clone(), Box::new(v), span))
        }
        _ => Err(ParserError::UnexpectedToken {
            expected: Some(Token::IDENT(
                "assignable target (variable, index, member or deref)".to_string(),
            )),
            found: Token::EQ,
            span,
        }),
    }
}

pub(super) fn make_inc_dec(target: Expr, is_inc: bool, span: Span) -> Result<Expr, ParserError> {
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
    pub fn new(lex: Lexer<'a>, base_path: String, include_paths: Vec<String>) -> Self {
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
            modules: Rc::new(RefCell::new(ModuleLoader::new(include_paths))),
            base_path,
            alias_map: HashMap::new(),
            from_alias: HashMap::new(),
            own_decls: Vec::new(),
            deferred_module_decls: Vec::new(),
            decl_pub: false,
            expr_depth: 0,
        }
    }

    pub fn new_loader(
        lex: Lexer<'a>,
        base_path: String,
        modules: Rc<RefCell<ModuleLoader>>,
    ) -> Self {
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
            modules,
            base_path,
            alias_map: HashMap::new(),
            from_alias: HashMap::new(),
            own_decls: Vec::new(),
            deferred_module_decls: Vec::new(),
            decl_pub: false,
            expr_depth: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Program, ParserError> {
        let mut body = Vec::new();
        loop {
            match self.peek() {
                Some(Ok((Token::EOF, _))) | None => break,
                Some(Ok((Token::IMPORT, _))) => self.parse_import(&mut body)?,
                Some(Ok((Token::USING, _))) => self.parse_using(&mut body)?,
                _ => {
                    let expr = self.expr()?;
                    self.record_decl(&expr);
                    body.push(expr);
                }
            }
        }
        body.extend(std::mem::take(&mut self.deferred_module_decls));
        self.append_fstring_helpers(&mut body);
        Ok(Program { body })
    }

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
                Some(Ok((Token::IMPORT, _))) => {
                    if let Err(e) = self.parse_import(&mut body) {
                        errors.push(e);
                        self.synchronize();
                    }
                }
                Some(Ok((Token::USING, _))) => {
                    if let Err(e) = self.parse_using(&mut body) {
                        errors.push(e);
                        self.synchronize();
                    }
                }
                Some(Ok(_)) => match self.expr() {
                    Ok(expr) => {
                        self.record_decl(&expr);
                        body.push(expr);
                    }
                    Err(e) => {
                        errors.push(e);
                        self.synchronize();
                    }
                },
            }
        }
        body.extend(std::mem::take(&mut self.deferred_module_decls));
        self.append_fstring_helpers(&mut body);
        (Program { body }, errors)
    }

    fn synchronize(&mut self) {
        let mut depth: i32 = 0;
        loop {
            match self.peek().cloned() {
                None => break,
                Some(Err(_)) => {
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
                    | Token::EXTERN
                    | Token::IMPORT
                    | Token::USING,
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

    fn record_decl(&mut self, expr: &Expr) {
        match expr {
            Expr::FuncDecl(name, attrs, ..) => {
                let kind = if attrs.is_external {
                    DeclKind::ExternFn
                } else {
                    DeclKind::Fn
                };
                let is_pub = attrs.is_pub || attrs.is_external;
                self.own_decls.push((name.clone(), kind, is_pub));
            }
            Expr::Struct(name, ..) => {
                self.own_decls
                    .push((name.clone(), DeclKind::Struct, self.decl_pub))
            }
            Expr::Union(name, ..) => {
                self.own_decls
                    .push((name.clone(), DeclKind::Union, self.decl_pub))
            }
            Expr::Enum(name, ..) => {
                self.own_decls
                    .push((name.clone(), DeclKind::Enum, self.decl_pub))
            }
            Expr::ConstDecl(name, _, _, is_pub, _) => {
                self.own_decls
                    .push((name.clone(), DeclKind::Const, *is_pub))
            }
            Expr::GlobalVar(name, is_pub, ..) => {
                self.own_decls
                    .push((name.clone(), DeclKind::GlobalVar, *is_pub))
            }
            Expr::ExternVar(name, ..) => {
                self.own_decls
                    .push((name.clone(), DeclKind::ExternVar, true))
            }
            _ => {}
        }
        self.decl_pub = false;
    }

    pub(super) fn parse_fun_signature(
        &mut self,
        is_external: bool,
    ) -> Result<(Vec<(String, Type)>, Type, Box<Expr>), ParserError> {
        self.expect(Token::LPAREN)?;
        let params = self.get_params_list()?;
        self.expect(Token::RPAREN)?;

        let ret_type = if matches!(self.peek(), Some(Ok((Token::COLON, _)))) {
            self.next()?;
            self.parse_type()?
        } else {
            Type::Primitive(Primitive::Void)
        };
        let body = if is_external {
            Box::new(Expr::Nil(Span::new(0, 0)))
        } else {
            self.scope_depth += 1;
            let body = self.expr();
            self.scope_depth -= 1;
            Box::new(body?)
        };
        Ok((params, ret_type, body))
    }

    pub(super) fn parse_field_list(&mut self) -> Result<Vec<(String, Type)>, ParserError> {
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
        Ok(fields)
    }

    pub(super) fn parse_block_exprs(&mut self, span: Span) -> Result<Expr, ParserError> {
        let mut exprs: Vec<Expr> = Vec::new();
        let last_span = self.last_span;
        loop {
            let Some(peeked) = self.peek().cloned() else {
                return Err(ParserError::UnexpectedToken {
                    expected: Some(Token::RBRACE),
                    found: Token::EOF,
                    span: last_span,
                });
            };
            match peeked {
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
                        found: self.found_token_or_eof(),
                        span: self.last_span,
                    });
                }
                Err(e) => return Err(ParserError::LexerError(e.to_owned())),
                _ => {
                    let e = self.expr()?;
                    if matches!(self.peek(), Some(Ok((Token::SEMICOLON, _)))) {
                        self.next()?;
                        if !matches!(
                            e,
                            Expr::Return(..)
                                | Expr::Break(..)
                                | Expr::Continue(_)
                                | Expr::VarDecl(..)
                                | Expr::ConstDecl(..)
                                | Expr::FuncDecl(..)
                                | Expr::Struct(..)
                                | Expr::Union(..)
                                | Expr::Enum(..)
                                | Expr::TypeDef(_)
                                | Expr::ExternVar(..)
                                | Expr::GlobalVar(..)
                        ) {
                            let span = e.span();
                            exprs.push(Expr::Cast(
                                Box::new(e),
                                Type::Primitive(Primitive::Void),
                                span,
                            ));
                            continue;
                        }
                    }
                    exprs.push(e);
                }
            }
        }
        Ok(Expr::Block(exprs, span))
    }

    pub(super) fn parse_global_annotation(&mut self, kw: &str) -> Result<bool, ParserError> {
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

    pub(super) fn get_params_list(&mut self) -> Result<Vec<(String, Type)>, ParserError> {
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
                                found: self.found_token_or_eof(),
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
                                found: self.found_token_or_eof(),
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
                                found: self.found_token_or_eof(),
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
                        found: self.found_token_or_eof(),
                        span: self.last_span,
                    });
                }
            }
        }
        Ok(params)
    }
}
