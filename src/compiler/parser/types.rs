use super::error::ParserError;
use crate::compiler::{
    lexer::Token,
    modules::DeclKind,
    parser::{Parser, Primitive, Type},
};
use std::collections::HashMap;

impl<'a> Parser<'a> {
    pub(super) fn lookup_type_param(&self, name: &str) -> Option<usize> {
        self.type_param_scopes
            .last()
            .and_then(|scope| scope.get(name).copied())
    }

    pub(super) fn push_type_params(&mut self, params: &[String]) {
        let mut scope = HashMap::new();
        for (i, name) in params.iter().enumerate() {
            scope.insert(name.clone(), i);
        }
        self.type_param_scopes.push(scope);
    }

    pub(super) fn get_type_params_list(&mut self) -> Result<Vec<String>, ParserError> {
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
                        found: self.found_token_or_eof(),
                        span: self.last_span,
                    });
                }
            }
        }
        Ok(params)
    }

    pub(super) fn get_type_args_list(&mut self) -> Result<Vec<Type>, ParserError> {
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
                        found: self.found_token_or_eof(),
                        span: self.last_span,
                    });
                }
            }
        }
        Ok(args)
    }

    pub(super) fn parse_type(&mut self) -> Result<Type, ParserError> {
        if let Some(Ok((Token::STAR, _))) = self.peek() {
            self.next()?;
            let inner_type = self.parse_type()?;
            return Ok(Type::Pointer(Box::new(inner_type)));
        }

        let (first_token, span) = self.next()?;

        let base_type = match first_token {
            Token::TYPE(t) => {
                if matches!(self.peek(), Some(Ok((Token::COLONCOLON, _)))) {
                    self.next()?;
                    let (member, _) = self.parse_ident()?;
                    let mod_name = self.alias_map.get(&t).cloned().unwrap_or_else(|| t.clone());
                    let target = self.resolve_module_name(&mod_name, &member)?;
                    let kind = self.resolve_module_kind(&mod_name, &member);
                    let mut args = Vec::new();
                    if matches!(self.peek(), Some(Ok((Token::LT, _)))) {
                        self.next()?;
                        args = self.get_type_args_list()?;
                        self.expect(Token::GT)?;
                    }
                    match kind {
                        DeclKind::Union => Type::Union(target, args),
                        DeclKind::Enum => Type::Primitive(Primitive::Int),
                        _ => Type::Struct(target, args),
                    }
                } else {
                    match t.as_str() {
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
                    }
                }
            }
            Token::IDENT(s) => {
                if matches!(self.peek(), Some(Ok((Token::COLONCOLON, _)))) {
                    self.next()?;
                    let (member, _) = self.parse_ident()?;
                    let mod_name = self.alias_map.get(&s).cloned().unwrap_or_else(|| s.clone());
                    let target = self.resolve_module_name(&mod_name, &member)?;
                    let kind = self.resolve_module_kind(&mod_name, &member);
                    let mut args = Vec::new();
                    if matches!(self.peek(), Some(Ok((Token::LT, _)))) {
                        self.next()?;
                        args = self.get_type_args_list()?;
                        self.expect(Token::GT)?;
                    }
                    match kind {
                        DeclKind::Union => Type::Union(target, args),
                        DeclKind::Enum => Type::Primitive(Primitive::Int),
                        _ => Type::Struct(target, args),
                    }
                } else if let Some((target, kind)) = self.from_alias.get(&s).cloned() {
                    let mut args = Vec::new();
                    if matches!(self.peek(), Some(Ok((Token::LT, _)))) {
                        self.next()?;
                        args = self.get_type_args_list()?;
                        self.expect(Token::GT)?;
                    }
                    match kind {
                        DeclKind::Union => Type::Union(target, args),
                        DeclKind::Enum => Type::Primitive(Primitive::Int),
                        _ => Type::Struct(target, args),
                    }
                } else if let Some(idx) = self.lookup_type_param(&s) {
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
                                    found: self.found_token_or_eof(),
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

            while matches!(self.peek(), Some(Ok((Token::INT(_), _)))) {
                self.next()?;
            }
            self.expect(Token::RBRACKET)?;
            return Ok(Type::Array(Box::new(base_type)));
        }

        Ok(base_type)
    }

}
