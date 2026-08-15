mod error;
mod preprocessor;
pub use error::PreprocessorError;
use std::{collections::HashMap, iter::Peekable, str::Chars};

#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub params: Vec<String>,
    pub body: String,
}

pub struct Preprocessor<'a> {
    src: Peekable<Chars<'a>>,
    source_text: &'a str,
    base_path: String,
    include_paths: Vec<String>,
    row: usize,
    col: usize,
    defines: HashMap<String, MacroDefinition>,
    condition_stack: Vec<bool>,
    skipping: bool,
}

impl<'a> Preprocessor<'a> {
    pub fn new(src: &'a str, base_path: String, include_paths: Vec<String>) -> Self {
        let mut default_paths = Vec::new();

        default_paths.push("/usr/local/include/alum".to_string());
        default_paths.push("/usr/local/alum".to_string());

        default_paths.extend(include_paths);

        Self {
            src: src.chars().peekable(),
            source_text: src,
            base_path,
            include_paths: default_paths,
            row: 1,
            col: 0,
            defines: HashMap::new(),
            condition_stack: Vec::new(),
            skipping: false,
        }
    }
}
