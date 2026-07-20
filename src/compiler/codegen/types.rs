use cranelift::codegen::ir;
use crate::compiler::parser::Type;
use crate::compiler::Span;
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
        span: Span,
    },
    UndefinedVariable {
        name: String,
        span: Span,
    },
    #[allow(dead_code)]
    UndefinedFunction {
        name: String,
        span: Span,
    },
    ModuleError(String, Span),
}

impl Display for CodeGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodeGenError::UnexpectedExpression { found, span } => {
                write!(f, "Unexpected expression: '{:?}' at {:?}, expected FuncDecl", found, span)
            }
            CodeGenError::UndefinedVariable { name, span } => {
                write!(f, "Undefined variable: '{:?}' at {:?}", name, span)
            }
            CodeGenError::UndefinedFunction { name, span } => {
                write!(f, "Undefined function: '{:?}' at {:?}", name, span)
            }
            CodeGenError::ModuleError(msg, span) => {
                write!(f, "Module error: '{}' at {:?}", msg, span)
            }
        }
    }
}

impl std::error::Error for CodeGenError {}

impl CodeGenError {
    pub fn span(&self) -> crate::compiler::Span {
        match self {
            CodeGenError::UnexpectedExpression { span, .. }
            | CodeGenError::UndefinedVariable { span, .. }
            | CodeGenError::UndefinedFunction { span, .. }
            | CodeGenError::ModuleError(_, span) => *span,
        }
    }
}

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
