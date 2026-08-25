use crate::compiler::Span;
use std::fmt;

#[derive(Debug, Clone)]
pub enum PreprocessorError {
    ImportError {
        file: String,
        row: usize,
        col: usize,
    },
    IoError {
        msg: String,
        row: usize,
        col: usize,
    },
    ConditionError {
        msg: String,
        row: usize,
        col: usize,
    },
    MacroError {
        msg: String,
        row: usize,
        col: usize,
    },
}

impl std::error::Error for PreprocessorError {}

impl PreprocessorError {
    pub fn span(&self) -> Span {
        match self {
            PreprocessorError::ImportError { row, col, .. }
            | PreprocessorError::IoError { row, col, .. }
            | PreprocessorError::ConditionError { row, col, .. }
            | PreprocessorError::MacroError { row, col, .. } => Span::new(*row, *col),
        }
    }
}

impl fmt::Display for PreprocessorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreprocessorError::ImportError { file, row, col } => {
                write!(
                    f,
                    "Import error at {}:{}: cannot include '{}'",
                    row, col, file
                )
            }
            PreprocessorError::IoError { msg, row, col } => {
                write!(f, "IO error at {}:{}: {}", row, col, msg)
            }
            PreprocessorError::ConditionError { msg, row, col } => {
                write!(f, "Condition error at {}:{}: {}", row, col, msg)
            }
            PreprocessorError::MacroError { msg, row, col } => {
                write!(f, "Macro error at {}:{}: {}", row, col, msg)
            }
        }
    }
}
