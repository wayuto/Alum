use crate::compiler::{Span, parser::Type};
use std::fmt;

#[derive(Debug)]
pub enum CheckerError {
    TypeMismatch {
        expected: Type,
        found: Type,
        context: String,
        span: Span,
    },
    UndefinedVariable(String, Span),
    #[allow(dead_code)]
    UndefinedFunction(String, Span),
    UndefinedStruct(String, Span),
    UndefinedUnion(String, Span),
    UndefinedField {
        struct_name: String,
        field: String,
        span: Span,
    },
    UndefinedEnumMember {
        enum_name: String,
        member: String,
        span: Span,
    },
    ArgCountMismatch {
        expected: usize,
        found: usize,
        func: String,
        span: Span,
    },
    NonStructMemberAccess(String, Span),
    InvalidOperation {
        op: String,
        type_name: String,
        span: Span,
    },
}

impl fmt::Display for CheckerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckerError::TypeMismatch {
                expected,
                found,
                context,
                ..
            } => {
                write!(
                    f,
                    "Type mismatch in {}: expected {}, found {}",
                    context, expected, found
                )
            }
            CheckerError::UndefinedVariable(name, _) => {
                write!(f, "Undefined variable: {}", name)
            }
            CheckerError::UndefinedFunction(name, _) => {
                write!(f, "Undefined function: {}", name)
            }
            CheckerError::UndefinedStruct(name, _) => {
                write!(f, "Undefined struct: {}", name)
            }
            CheckerError::UndefinedUnion(name, _) => {
                write!(f, "Undefined union: {}", name)
            }
            CheckerError::UndefinedField {
                struct_name, field, ..
            } => {
                write!(f, "Struct {} has no field {}", struct_name, field)
            }
            CheckerError::UndefinedEnumMember {
                enum_name, member, ..
            } => {
                write!(f, "Enum {} has no member {}", enum_name, member)
            }
            CheckerError::ArgCountMismatch {
                expected,
                found,
                func,
                ..
            } => {
                write!(
                    f,
                    "Function {} expects {} arguments, found {}",
                    func, expected, found
                )
            }
            CheckerError::NonStructMemberAccess(type_name, _) => {
                write!(f, "Cannot access member on non-struct type: {}", type_name)
            }
            CheckerError::InvalidOperation { op, type_name, .. } => {
                write!(f, "Invalid operation '{}' on type {}", op, type_name)
            }
        }
    }
}

impl std::error::Error for CheckerError {}

impl CheckerError {
    pub fn span(&self) -> Span {
        match self {
            CheckerError::TypeMismatch { span, .. }
            | CheckerError::UndefinedField { span, .. }
            | CheckerError::UndefinedEnumMember { span, .. }
            | CheckerError::ArgCountMismatch { span, .. }
            | CheckerError::InvalidOperation { span, .. } => *span,
            CheckerError::UndefinedVariable(_, s)
            | CheckerError::UndefinedFunction(_, s)
            | CheckerError::UndefinedStruct(_, s)
            | CheckerError::UndefinedUnion(_, s)
            | CheckerError::NonStructMemberAccess(_, s) => *s,
        }
    }
}
