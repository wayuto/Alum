pub mod checker;
mod error;
mod expr;
mod unify;

use crate::compiler::parser::Type;
pub use error::CheckerError;
use std::collections::HashMap;

pub struct TypeChecker {
    pub(super) type_stack: Vec<HashMap<String, Type>>,
    pub(super) functions: HashMap<String, (Vec<Type>, Type)>,
    pub(super) structs: HashMap<String, Vec<(String, Type)>>,
    pub(super) typedefs: HashMap<String, Type>,
    pub(super) type_var_counter: usize,
    pub(super) type_bindings: HashMap<usize, Type>,
}
