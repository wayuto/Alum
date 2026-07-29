mod context;
mod expr;
mod func;
pub mod ir;
mod irgen;
mod lambda;

use crate::compiler::{
    irgen::ir::{IRConst, IRFunction},
    parser::Type,
};
use std::collections::HashMap;

pub struct IRGen {
    pub(super) functions: Vec<IRFunction>,
    pub(super) constants: Vec<IRConst>,
    constant_pool: HashMap<IRConst, usize>,
    pub(super) structs: HashMap<String, Vec<(String, Type)>>,
    pub(super) lambda_counter: u32,
}

impl IRGen {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            constants: Vec::new(),
            constant_pool: HashMap::new(),
            structs: HashMap::new(),
            lambda_counter: 0,
        }
    }
}
