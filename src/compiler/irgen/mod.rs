mod context;
mod expr;
mod func;
pub mod ir;
mod irgen;
mod lambda;

use crate::compiler::{
    irgen::ir::{IRConst, IRFunction, IRType},
    parser::{Expr, Type},
};
use std::collections::HashMap;

pub struct IRGen {
    pub(super) functions: Vec<IRFunction>,
    pub(super) constants: Vec<IRConst>,
    constant_pool: HashMap<IRConst, usize>,
    pub(super) globals: HashMap<String, (IRConst, IRType)>,
    pub(super) structs: HashMap<String, (Vec<String>, Vec<(String, Type)>)>,
    pub(super) unions: HashMap<String, (Vec<String>, Vec<(String, Type)>)>,
    pub(super) enums: HashMap<String, Vec<(String, isize)>>,
    pub(super) generic_funcs: HashMap<String, (Vec<String>, Vec<(String, Type)>, Type, Box<Expr>)>,
    pub(super) func_high_returns: HashMap<String, Type>,
    pub(super) mono_in_progress: Vec<String>,
    pub(super) lambda_counter: u32,
    pub(super) extern_vars: HashMap<String, Type>,
}

impl IRGen {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            constants: Vec::new(),
            constant_pool: HashMap::new(),
            globals: HashMap::new(),
            structs: HashMap::new(),
            unions: HashMap::new(),
            enums: HashMap::new(),
            generic_funcs: HashMap::new(),
            func_high_returns: HashMap::new(),
            mono_in_progress: Vec::new(),
            lambda_counter: 0,
            extern_vars: HashMap::new(),
        }
    }
}
