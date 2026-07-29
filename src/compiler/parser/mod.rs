mod ast;
mod display;
mod error;
mod parser;

pub use ast::*;
pub use error::ParserError;

use crate::compiler::{Span, lexer::Lexer};
use std::{collections::HashMap, iter::Peekable};

pub struct Parser<'a> {
    lex: Peekable<Lexer<'a>>,
    last_span: Span,
    typedefs: HashMap<String, Type>,
    structs: HashMap<String, Vec<(String, Type)>>,
}
