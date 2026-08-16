mod ast;
mod display;
mod error;
mod parser;

pub use ast::*;
pub use error::ParserError;

use crate::compiler::{
    Span,
    lexer::{Lexer, LexerError, Token},
    modules::{DeclKind, ModuleLoader},
};
use std::{cell::RefCell, collections::HashMap, iter::Peekable, rc::Rc};

pub struct Parser<'a> {
    lex: Peekable<Lexer<'a>>,
    lookahead: Vec<Result<(Token, Span), LexerError>>,
    last_span: Span,
    typedefs: HashMap<String, Type>,
    structs: HashMap<String, (Vec<String>, Vec<(String, Type)>)>,
    unions: HashMap<String, (Vec<String>, Vec<(String, Type)>)>,
    enums: HashMap<String, Vec<(String, isize)>>,
    type_param_scopes: Vec<HashMap<String, usize>>,
    has_fstring: bool,
    scope_depth: usize,
    deref_depth: usize,
    modules: Rc<RefCell<ModuleLoader>>,
    base_path: String,
    alias_map: HashMap<String, String>,
    from_alias: HashMap<String, (String, DeclKind)>,
    own_decls: Vec<(String, DeclKind, bool)>,

    decl_pub: bool,
}

impl<'a> Parser<'a> {
    pub fn loaded_module_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.modules.borrow().loaded.keys().cloned().collect();
        names.extend(self.alias_map.keys().cloned());
        names.sort();
        names.dedup();
        names
    }

    pub fn module_members(&self, name: &str) -> Option<Vec<(String, DeclKind)>> {
        let real = self
            .alias_map
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string());
        let m = self.modules.borrow();
        let lm = m.loaded.get(&real)?;
        let mut out: Vec<(String, DeclKind)> = lm
            .names
            .keys()
            .filter(|n| lm.pub_names.contains(*n))
            .map(|n| (n.clone(), lm.kinds.get(n).copied().unwrap_or(DeclKind::Fn)))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Some(out)
    }
}
