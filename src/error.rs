use crate::token::Literal;
use std::process::exit;

enum ErrorType {
    Unknown,
    SyntaxError(String),
    UnimplementedError(String),
    NameError(String),
    ImportError(String),
    TypeError(String),
}

pub struct GosError {
    row: usize,
    col: usize,
    err_type: ErrorType,
}

impl GosError {
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            row,
            col,
            err_type: ErrorType::Unknown,
        }
    }

    pub fn unexpected_char(&mut self, expected: Option<char>, found: char) -> () {
        match expected {
            Some(ch) => {
                self.err_type =
                    ErrorType::SyntaxError(format!("expected {:?}, found: {:?}", ch, found));
            }
            None => {
                self.err_type = ErrorType::SyntaxError(format!("unexpected '{}'", found));
            }
        }
    }

    pub fn import_error(&mut self, file: String) -> () {
        self.err_type = ErrorType::ImportError(format!("cannot import {:?}", file));
    }

    pub fn unimplemented(&mut self, unimplemented: &str) -> () {
        self.err_type =
            ErrorType::UnimplementedError(format!("{} hasn't been implemented", unimplemented));
    }

    pub fn invalid_name(&mut self, name: Literal) -> () {
        self.err_type = ErrorType::NameError(format!("invalid name: {:?}", name));
    }

    pub fn unknown_type(&mut self) -> () {
        self.err_type = ErrorType::TypeError("unknown type".to_string());
    }

    pub fn panic(&self) -> () {
        match &self.err_type {
            ErrorType::SyntaxError(e) => {
                eprintln!(
                    "SyntaxError: {} (line: {}, column: {})",
                    e, self.row, self.col
                );
            }
            ErrorType::UnimplementedError(e) => {
                eprintln!(
                    "UnimplementedError: {} (line: {}, column: {})",
                    e, self.row, self.col
                );
            }
            ErrorType::ImportError(e) => {
                eprintln!(
                    "ImportError: {} (line: {}, column: {})",
                    e, self.row, self.col
                );
            }
            ErrorType::NameError(e) => {
                eprintln!(
                    "NameError: {} (line: {}, column: {})",
                    e, self.row, self.col
                );
            }
            ErrorType::TypeError(e) => {
                eprintln!(
                    "TypeError: {} (line: {}, column: {})",
                    e, self.row, self.col
                );
            }
            ErrorType::Unknown => {
                eprintln!("UnknownError (line: {}, column: {})", self.row, self.col);
            }
        }
        exit(1);
    }
}
