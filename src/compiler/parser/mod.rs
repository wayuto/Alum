mod ast;
mod display;
mod error;
mod parser;

pub use ast::*;
pub use error::ParserError;

use crate::compiler::{
    Span,
    lexer::{Lexer, LexerError, Token},
};
use std::{collections::HashMap, iter::Peekable};

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
}
