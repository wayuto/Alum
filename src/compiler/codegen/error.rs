use crate::compiler::Span;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone)]
pub enum CodeGenError {
    UndefinedVariable {
        name: String,
        span: Span,
    },
    UndefinedFunction {
        name: String,
        span: Span,
    },
    UseAfterMove {
        name: String,
        moved_at: Span,
        span: Span,
    },
    NameError {
        message: String,
    },
    TypeError {
        message: String,
    },
    ScopeError {
        message: String,
    },
    SyntaxError {
        message: String,
    },
    MissingOperand {
        message: String,
    },
    InvalidOperand {
        message: String,
    },
    UnsupportedOperation {
        message: String,
    },
    AssemblyError(String),
}

impl Display for CodeGenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            CodeGenError::UndefinedVariable { name, .. } => {
                write!(f, "Undefined variable: '{}'", name)
            }
            CodeGenError::UndefinedFunction { name, .. } => {
                write!(f, "Undefined function: '{}'", name)
            }
            CodeGenError::UseAfterMove { name, moved_at, .. } => write!(
                f,
                "Use of moved value: '{}' (moved at {}:{}) no longer owns its data; \
                 assign it a new value or use '$' to copy before moving",
                name, moved_at.line, moved_at.col
            ),
            CodeGenError::NameError { message } => write!(f, "Name error: {}", message),
            CodeGenError::TypeError { message } => write!(f, "Type error: {}", message),
            CodeGenError::ScopeError { message } => write!(f, "Scope error: {}", message),
            CodeGenError::SyntaxError { message } => write!(f, "Syntax error: {}", message),
            CodeGenError::MissingOperand { message } => write!(f, "Missing operand: {}", message),
            CodeGenError::InvalidOperand { message } => write!(f, "Invalid operand: {}", message),
            CodeGenError::UnsupportedOperation { message } => {
                write!(f, "Unsupported operation: {}", message)
            }
            CodeGenError::AssemblyError(msg) => write!(f, "Assembly error: {}", msg),
        }
    }
}

impl std::error::Error for CodeGenError {}

impl CodeGenError {
    pub fn span(&self) -> Span {
        match self {
            CodeGenError::UndefinedVariable { span, .. }
            | CodeGenError::UndefinedFunction { span, .. }
            | CodeGenError::UseAfterMove { span, .. } => *span,
            _ => Span::new(0, 0),
        }
    }
}
