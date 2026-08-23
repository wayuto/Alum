use super::error::ParserError;
use crate::compiler::{
    Span,
    lexer::{Lexer, Token},
    modules::{DeclKind, LoadedModule, ModuleLoader},
    parser::{Expr, FuncAttrs, Parser, Primitive, Type},
    preprocessor::Preprocessor,
};
use std::collections::HashMap;

impl<'a> Parser<'a> {
    pub(super) fn parse_ident(&mut self) -> Result<(String, Span), ParserError> {
        let (tok, span) = self.next()?;
        match tok {
            Token::IDENT(s) | Token::TYPE(s) => Ok((s, span)),
            token => Err(ParserError::UnexpectedToken {
                expected: Some(Token::IDENT("NAME".to_string())),
                found: token,
                span,
            }),
        }
    }

    pub(super) fn parse_module_path(&mut self) -> Result<Vec<String>, ParserError> {
        let (first, _) = self.parse_ident()?;
        let mut segs = vec![first];
        while matches!(self.peek(), Some(Ok((Token::COLONCOLON, _)))) {
            if !matches!(self.peek_n(1), Some(Ok((Token::IDENT(_), _)))) {
                break;
            }
            self.next()?;
            let (seg, _) = self.parse_ident()?;
            segs.push(seg);
        }
        Ok(segs)
    }

    pub(super) fn register_path_alias(
        &mut self,
        alias: &str,
        mod_name: &str,
    ) -> Result<(), ParserError> {
        if let Some(prev) = self.alias_map.get(alias) {
            if prev != mod_name {
                return Err(ParserError::ModuleError {
                    message: format!(
                        "conflicting import: '{alias}' already refers to module '{prev}'"
                    ),
                    span: Some(self.last_span),
                });
            }
        } else {
            self.alias_map
                .insert(alias.to_string(), mod_name.to_string());
        }
        Ok(())
    }

    pub(super) fn resolve_path_chain(&mut self, segs: &[String]) -> Result<String, ParserError> {
        for i in (1..segs.len()).rev() {
            let p = segs[0..=i].join("/");
            let hit = self.modules.borrow().loaded.contains_key(&p)
                || self
                    .modules
                    .borrow()
                    .find_file(&p, &self.base_path)
                    .is_some();
            if hit {
                if i + 1 >= segs.len() {
                    return Err(ParserError::ModuleError {
                        message: format!("'{p}' is a module, not a member"),
                        span: Some(self.last_span),
                    });
                }
                let decls = self.load_module(&p)?;
                self.deferred_module_decls.extend(decls);
                return self.resolve_module_name(&p, &segs[i + 1]);
            }
        }
        for i in (1..segs.len()).rev() {
            let p = segs[0..i].join("/");
            if self.modules.borrow().loaded.contains_key(&p) {
                return self.resolve_module_name(&p, &segs[i]);
            }
        }
        Err(ParserError::ModuleError {
            message: format!(
                "'{}' has no member '{}' (module not imported?)",
                segs[0],
                segs.get(1).map(|s| s.as_str()).unwrap_or("?")
            ),
            span: Some(self.last_span),
        })
    }

    pub(super) fn parse_import_stmt(&mut self, body: &mut Vec<Expr>) -> Result<(), ParserError> {
        self.next()?;
        let mut segs = self.parse_module_path()?;
        let mod_name = segs.join("/");
        let multi = segs.len() > 1;
        let last_seg = segs.pop().unwrap();
        let decls = match self.load_module(&mod_name) {
            Ok(d) => d,
            Err(e) => {
                let is_dir = self.modules.borrow().dir_exists(&mod_name, &self.base_path);
                if is_dir {
                    self.register_path_alias(&last_seg, &mod_name)?;
                    Vec::new()
                } else {
                    return Err(e);
                }
            }
        };
        body.extend(decls);
        if matches!(self.peek(), Some(Ok((Token::AS, _)))) {
            self.next()?;
            let (a, _) = self.parse_ident()?;
            self.register_path_alias(&a, &mod_name)?;
        } else if multi {
            self.register_path_alias(&last_seg, &mod_name)?;
        }
        Ok(())
    }

    pub(super) fn insert_from_alias(
        &mut self,
        key: String,
        target: String,
        kind: DeclKind,
    ) -> Result<(), ParserError> {
        if let Some((prev, _)) = self.from_alias.get(&key) {
            if prev != &target {
                return Err(ParserError::ModuleError {
                    message: format!(
                        "conflicting import: '{key}' is already imported (points to '{prev}')"
                    ),
                    span: Some(self.last_span),
                });
            }
        }
        self.from_alias.insert(key, (target, kind));
        Ok(())
    }

    pub(super) fn parse_using_stmt(&mut self, body: &mut Vec<Expr>) -> Result<(), ParserError> {
        self.next()?;
        let segs = self.parse_module_path()?;
        if segs.len() > 1 {
            let path = segs.join("/");
            let is_file = self
                .modules
                .borrow()
                .find_file(&path, &self.base_path)
                .is_some();
            let mod_loaded = self.modules.borrow().loaded.contains_key(&path);
            if is_file || mod_loaded {
                let decls = self.load_module(&path)?;
                body.extend(decls);
                let last_seg = segs.last().unwrap().clone();
                self.register_path_alias(&last_seg, &path)?;
                return Ok(());
            }
            if segs.len() == 2 {
                let mod_name = &segs[0];
                let name = &segs[1];
                let real_name = self
                    .alias_map
                    .get(mod_name)
                    .cloned()
                    .unwrap_or_else(|| mod_name.clone());
                let decls = self.load_module(&real_name)?;
                body.extend(decls);
                let target = self.resolve_module_name(&real_name, name)?;
                let kind = self.resolve_module_kind(&real_name, name);
                if matches!(self.peek(), Some(Ok((Token::AS, _)))) {
                    self.next()?;
                    let (a, _) = self.parse_ident()?;
                    self.insert_from_alias(a, target, kind)?;
                } else {
                    self.insert_from_alias(name.clone(), target, kind)?;
                }
                return Ok(());
            }
            let mod_path = segs[..segs.len() - 1].join("/");
            let name = segs.last().unwrap();
            let hit = self
                .modules
                .borrow()
                .find_file(&mod_path, &self.base_path)
                .is_some()
                || self.modules.borrow().loaded.contains_key(&mod_path);
            if hit {
                let decls = self.load_module(&mod_path)?;
                body.extend(decls);
                let target = self.resolve_module_name(&mod_path, name)?;
                let kind = self.resolve_module_kind(&mod_path, name);
                if matches!(self.peek(), Some(Ok((Token::AS, _)))) {
                    self.next()?;
                    let (a, _) = self.parse_ident()?;
                    self.insert_from_alias(a, target, kind)?;
                } else {
                    self.insert_from_alias(name.clone(), target, kind)?;
                }
                return Ok(());
            }
            return Err(ParserError::ModuleError {
                message: format!("module '{}' not found", path),
                span: Some(self.last_span),
            });
        }
        let mod_name = segs[0].clone();
        self.expect(Token::COLONCOLON)?;
        let real_name = self
            .alias_map
            .get(&mod_name)
            .cloned()
            .unwrap_or_else(|| mod_name.clone());
        let decls = self.load_module(&real_name)?;
        body.extend(decls);

        if matches!(self.peek(), Some(Ok((Token::LBRACE, _)))) {
            self.next()?;
            loop {
                let (name, _) = self.parse_ident()?;
                let target = self.resolve_module_name(&real_name, &name)?;
                let kind = self.resolve_module_kind(&real_name, &name);
                if matches!(self.peek(), Some(Ok((Token::AS, _)))) {
                    self.next()?;
                    let (a, _) = self.parse_ident()?;
                    self.insert_from_alias(a, target, kind)?;
                } else {
                    self.insert_from_alias(name, target, kind)?;
                }
                match self.peek() {
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
                            found: match self.peek().cloned() {
                                Some(Ok((t, _))) => t,
                                _ => Token::EOF,
                            },
                            span: self.last_span,
                        });
                    }
                }
            }
        } else {
            let (name, _) = self.parse_ident()?;
            let target = self.resolve_module_name(&real_name, &name)?;
            let kind = self.resolve_module_kind(&real_name, &name);
            if matches!(self.peek(), Some(Ok((Token::AS, _)))) {
                self.next()?;
                let (a, _) = self.parse_ident()?;
                self.insert_from_alias(a, target, kind)?;
            } else {
                self.insert_from_alias(name, target, kind)?;
            }
        }
        Ok(())
    }

    pub(super) fn load_module(&mut self, mod_name: &str) -> Result<Vec<Expr>, ParserError> {
        {
            let m = self.modules.borrow();
            if m.loading.iter().any(|n| n == mod_name) {
                return Err(ParserError::ModuleError {
                    message: format!("circular import of module '{}'", mod_name),
                    span: Some(self.last_span),
                });
            }
            let already = m.loaded.get(mod_name).cloned();
            if let Some(lm) = already {
                drop(m);
                self.merge_module_types(&lm);
                return Ok(Vec::new());
            }
        }

        let file_path = {
            let m = self.modules.borrow();
            m.find_file(mod_name, &self.base_path)
        };
        let Some(file_path) = file_path else {
            return Err(ParserError::ModuleError {
                message: format!("module '{}' not found", mod_name),
                span: Some(self.last_span),
            });
        };

        let content =
            std::fs::read_to_string(&file_path).map_err(|e| ParserError::ModuleError {
                message: format!("failed to read '{}': {}", file_path, e),
                span: Some(self.last_span),
            })?;
        let include_paths = self.modules.borrow().include_paths.clone();
        let mut pp = Preprocessor::new(&content, file_path.clone(), include_paths);
        let (processed, _) = pp.preprocess().map_err(|e| ParserError::ModuleError {
            message: format!("preprocessing module '{}': {}", mod_name, e),
            span: Some(self.last_span),
        })?;

        let lex = Lexer::new(&processed);
        let rc = self.modules.clone();
        self.modules.borrow_mut().loading.push(mod_name.to_string());
        let mut sub = Parser::new_loader(lex, file_path, rc);
        let parse_result = sub.parse();
        self.modules.borrow_mut().loading.pop();
        let mut sub_program = parse_result.map_err(|e| ParserError::ModuleError {
            message: format!("parsing module '{}': {}", mod_name, e),
            span: Some(self.last_span),
        })?;

        let own_decls = std::mem::take(&mut sub.own_decls);
        let names = ModuleLoader::build_names_map(mod_name, &own_decls);
        ModuleLoader::rename_module(&mut sub_program.body, &names);
        ModuleLoader::strip_module_pub(&mut sub_program.body);

        let renamed = |n: &String| names.get(n).cloned().unwrap_or_else(|| n.clone());
        let mut structs = HashMap::new();
        let mut unions = HashMap::new();
        let mut enums = HashMap::new();
        let mut typedefs = HashMap::new();
        for (k, v) in sub.structs.iter() {
            let k = renamed(k);
            structs.insert(k.clone(), v.clone());
            self.structs.insert(k, v.clone());
        }
        for (k, v) in sub.unions.iter() {
            let k = renamed(k);
            unions.insert(k.clone(), v.clone());
            self.unions.insert(k, v.clone());
        }
        for (k, v) in sub.enums.iter() {
            let k = renamed(k);
            enums.insert(k.clone(), v.clone());
            self.enums.insert(k, v.clone());
        }
        for (k, v) in sub.typedefs.iter() {
            let k = renamed(k);
            typedefs.insert(k.clone(), v.clone());
            self.typedefs.insert(k, v.clone());
        }

        let kinds = own_decls.iter().map(|(n, k, _)| (n.clone(), *k)).collect();
        let pub_names = own_decls
            .iter()
            .filter(|(_, _, p)| *p)
            .map(|(n, _, _)| n.clone())
            .collect();
        self.modules.borrow_mut().loaded.insert(
            mod_name.to_string(),
            LoadedModule {
                names,
                kinds,
                pub_names,
                structs,
                unions,
                enums,
                typedefs,
            },
        );

        Ok(sub_program.body)
    }

    pub(super) fn merge_module_types(&mut self, lm: &LoadedModule) {
        for (k, v) in lm.structs.iter() {
            self.structs.insert(k.clone(), v.clone());
        }
        for (k, v) in lm.unions.iter() {
            self.unions.insert(k.clone(), v.clone());
        }
        for (k, v) in lm.enums.iter() {
            self.enums.insert(k.clone(), v.clone());
        }
        for (k, v) in lm.typedefs.iter() {
            self.typedefs.insert(k.clone(), v.clone());
        }
    }

    pub(super) fn resolve_module_name(
        &self,
        mod_name: &str,
        name: &str,
    ) -> Result<String, ParserError> {
        let m = self.modules.borrow();
        if let Some(lm) = m.loaded.get(mod_name) {
            if let Some(target) = lm.names.get(name) {
                if !lm.pub_names.contains(name) {
                    return Err(ParserError::ModuleError {
                        message: format!(
                            "member '{name}' of module '{mod_name}' is private (not marked pub)"
                        ),
                        span: Some(self.last_span),
                    });
                }
                return Ok(target.clone());
            }
        }
        Err(ParserError::ModuleError {
            message: format!(
                "'{}' has no member '{}' (module not imported?)",
                mod_name, name
            ),
            span: Some(self.last_span),
        })
    }

    pub(super) fn resolve_module_kind(&self, mod_name: &str, name: &str) -> DeclKind {
        let m = self.modules.borrow();
        if let Some(lm) = m.loaded.get(mod_name) {
            if let Some(kind) = lm.kinds.get(name) {
                return *kind;
            }
        }
        DeclKind::Fn
    }

    pub(super) fn append_fstring_helpers(&mut self, body: &mut Vec<Expr>) {
        if self.has_fstring {
            let mut itoa_attrs = FuncAttrs::default();
            itoa_attrs.is_external = true;
            itoa_attrs.link_name = Some("itoa".to_string());
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
            ftoa_attrs.link_name = Some("ftoa".to_string());
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
}
