pub mod checker;
mod error;
mod expr;
mod unify;

use crate::compiler::parser::Type;
pub use error::CheckerError;
use std::collections::HashMap;

pub struct TypeChecker {
    pub(super) type_stack: Vec<HashMap<String, Type>>,
    pub(super) functions: HashMap<String, (Vec<String>, Vec<Type>, Type)>,
    pub(super) structs: HashMap<String, (Vec<String>, Vec<(String, Type)>)>,
    pub(super) unions: HashMap<String, (Vec<String>, Vec<(String, Type)>)>,
    pub(super) enums: HashMap<String, Vec<(String, isize)>>,
    pub(super) type_var_counter: usize,
    pub(super) type_bindings: HashMap<usize, Type>,
    pub(super) return_types: Vec<Type>,
    pub(super) generic_params: Vec<HashMap<usize, Type>>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            type_stack: vec![HashMap::new()],
            functions: HashMap::new(),
            structs: HashMap::new(),
            unions: HashMap::new(),
            enums: HashMap::new(),
            type_var_counter: 0,
            type_bindings: HashMap::new(),
            return_types: Vec::new(),
            generic_params: Vec::new(),
        }
    }
}
