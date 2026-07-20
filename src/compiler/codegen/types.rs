use cranelift::codegen::ir;
use crate::compiler::parser::Type;
use std::{
    collections::HashMap,
    fmt::Display,
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Slot {
    StackSlot(ir::StackSlot),
}

#[derive(Debug)]
pub enum CodeGenError {
    UnexpectedExpression {
        found: crate::compiler::parser::Expr,
    },
    UndefinedVariable {
        name: String,
    },
    #[allow(dead_code)]
    UndefinedFunction {
        name: String,
    },
    ModuleError(String),
}

impl Display for CodeGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodeGenError::UnexpectedExpression { found } => {
                write!(f, "Unexpected expression: '{:?}', expected FuncDecl", found)
            }
            CodeGenError::UndefinedVariable { name } => {
                write!(f, "Undefined variable: '{:?}'", name)
            }
            CodeGenError::UndefinedFunction { name } => {
                write!(f, "Undefined function: '{:?}'", name)
            }
            CodeGenError::ModuleError(msg) => {
                write!(f, "Module error: '{}'", msg)
            }
        }
    }
}

impl std::error::Error for CodeGenError {}

#[derive(Clone)]
pub(crate) struct LoopContext {
    pub header_block: ir::Block,
    pub exit_block: ir::Block,
    pub increment_block: Option<ir::Block>,
    #[allow(dead_code)]
    pub loop_params: Vec<(String, Slot)>,
}

#[derive(Clone, Debug)]
pub(crate) struct StructField {
    pub name: String,
    pub ty: Type,
    pub offset: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct StructDef {
    pub fields: Vec<StructField>,
    pub size: i64,
    #[allow(dead_code)]
    pub align: i64,
}

pub(crate) fn get_type(t: &Type, type_map: &HashMap<String, ir::Type>) -> ir::Type {
    match t {
        Type::Named(name) => *type_map.get(name).unwrap_or(&ir::types::I64),
        Type::Array(_, _) => ir::types::I64,
        Type::Pointer(_) => ir::types::I64,
        Type::Function(_, _) => ir::types::I64,
        Type::TypeVar(_) => ir::types::I64,
        Type::Auto => ir::types::I64,
        Type::Gen => ir::types::I64,
    }
}
