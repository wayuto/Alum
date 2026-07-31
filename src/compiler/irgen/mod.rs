mod context;
mod expr;
mod func;
pub mod ir;
mod irgen;
mod lambda;

use crate::compiler::{
    irgen::ir::{IRConst, IRFunction},
    parser::{Expr, Type},
};
use std::collections::HashMap;

pub struct IRGen {
    pub(super) functions: Vec<IRFunction>,
    pub(super) constants: Vec<IRConst>,
    constant_pool: HashMap<IRConst, usize>,
    pub(super) structs: HashMap<String, (Vec<String>, Vec<(String, Type)>)>,
    pub(super) generic_funcs: HashMap<String, (Vec<String>, Vec<(String, Type)>, Type, Box<Expr>)>,
    pub(super) mono_in_progress: Vec<String>,
    pub(super) lambda_counter: u32,
}

impl IRGen {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            constants: Vec::new(),
            constant_pool: HashMap::new(),
            structs: HashMap::new(),
            generic_funcs: HashMap::new(),
            mono_in_progress: Vec::new(),
            lambda_counter: 0,
        }
    }
}
