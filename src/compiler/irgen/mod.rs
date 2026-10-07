mod array;
mod call;
mod const_eval;
mod context;
mod control;
mod expr;
mod expr_memory;
mod expr_resource;
mod func;
mod globals;
pub mod ir;
mod irgen;
mod lambda;
mod match_expr;
pub(super) mod optimizer;
mod purity;
mod type_info;
mod vm_safety;

use crate::compiler::{
    bytecode::NativeTable,
    irgen::ir::{IRConst, IRFunction, IRGlobalVar, IRType},
    parser::{Expr, Type},
};
use std::collections::{HashMap, HashSet};
pub struct IRGen {
    pub(super) functions: Vec<IRFunction>,
    pub(super) constants: Vec<IRConst>,
    constant_pool: HashMap<IRConst, usize>,
    pub(super) globals: HashMap<String, (IRConst, IRType)>,
    pub(super) global_storage: HashMap<String, (IRType, Option<IRConst>, bool)>,
    pub(super) global_emits: Vec<IRGlobalVar>,
    pub(super) structs: HashMap<String, (Vec<String>, Vec<(String, Type)>)>,
    pub(super) unions: HashMap<String, (Vec<String>, Vec<(String, Type)>)>,
    pub(super) enums: HashMap<String, Vec<(String, isize)>>,
    pub(super) generic_funcs: HashMap<String, (Vec<String>, Vec<(String, Type)>, Type, Box<Expr>)>,
    pub(super) func_high_returns: HashMap<String, Type>,
    pub(super) lambda_counter: u32,
    pub(super) mono_depth: usize,
    pub(super) pending_fn_bodies: Vec<(String, Vec<(String, Type)>, Expr)>,
    pub(super) extern_vars: HashMap<String, Type>,
    pub(super) program_body: Vec<Expr>,
    pub(super) natives: Option<NativeTable>,
    pub(super) cte_step_limit: u64,
    pub(super) cte_force: bool,
    pub(super) cte_error: Option<String>,
    cte_pure_fns: Option<HashSet<String>>,
    cte_vm_program_cache: Option<Vec<Expr>>,

    expr_depth: usize,
}

impl IRGen {
    pub fn new(cte_libs: &[String]) -> Self {
        let natives = if cte_libs.is_empty() {
            None
        } else {
            NativeTable::open(cte_libs)
                .map_err(|e| eprintln!("warning: {e}"))
                .ok()
        };
        Self {
            functions: Vec::new(),
            constants: Vec::new(),
            constant_pool: HashMap::new(),
            expr_depth: 0,
            globals: HashMap::new(),
            global_storage: HashMap::new(),
            global_emits: Vec::new(),
            structs: HashMap::new(),
            unions: HashMap::new(),
            enums: HashMap::new(),
            generic_funcs: HashMap::new(),
            func_high_returns: HashMap::new(),
            lambda_counter: 0,
            mono_depth: 0,
            pending_fn_bodies: Vec::new(),
            extern_vars: HashMap::new(),
            program_body: Vec::new(),
            natives,
            cte_step_limit: crate::compiler::bytecode::DEFAULT_CTE_STEP_LIMIT,
            cte_force: false,
            cte_error: None,
            cte_pure_fns: None,
            cte_vm_program_cache: None,
        }
    }

    pub fn with_cte_options(mut self, step_limit: u64, force: bool) -> Self {
        self.cte_step_limit = step_limit;
        self.cte_force = force;
        self
    }
}
