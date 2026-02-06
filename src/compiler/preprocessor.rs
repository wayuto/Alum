use std::{collections::HashMap, fs, iter::Peekable, path::Path, str::Chars};

#[derive(Debug, Clone)]
pub enum PreprocessorError {
    ImportError {
        file: String,
        row: usize,
        col: usize,
    },
    IoError {
        message: String,
        row: usize,
        col: usize,
    },
    ConditionError {
        message: String,
        row: usize,
        col: usize,
    },
}

impl std::error::Error for PreprocessorError {}

impl std::fmt::Display for PreprocessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreprocessorError::ImportError { file, row, col } => {
                write!(
                    f,
                    "Import error at {}:{}: cannot import '{}'",
                    row, col, file
                )
            }
            PreprocessorError::IoError { message, row, col } => {
                write!(f, "IO error at {}:{}: {}", row, col, message)
            }
            PreprocessorError::ConditionError { message, row, col } => {
                write!(f, "Condition error at {}:{}: {}", row, col, message)
            }
        }
    }
}

pub struct Preprocessor<'a> {
    src: Peekable<Chars<'a>>,
    base_path: String,
    include_paths: Vec<String>,
    row: usize,
    col: usize,
    defines: HashMap<String, String>,
    condition_stack: Vec<bool>,
    skipping: bool,
}

impl<'a> Preprocessor<'a> {
    pub fn new(src: &'a str, base_path: String, include_paths: Vec<String>) -> Self {
        let mut default_paths = vec![
            "/usr/local/include/alum".to_string(),
            "/usr/local/alum".to_string(),
        ];

        default_paths.extend(include_paths);

        Self {
            src: src.chars().peekable(),
            base_path,
            include_paths: default_paths,
            row: 1,
            col: 0,
            defines: HashMap::new(),
            condition_stack: Vec::new(),
            skipping: false,
        }
    }

    fn current(&mut self) -> char {
        *self.src.peek().unwrap_or(&'\0')
    }

    fn bump(&mut self) {
        if let Some(c) = self.src.next() {
            if c == '\n' {
                self.row += 1;
                self.col = 0;
            } else {
                self.col += 1;
            }
        }
    }

    fn skip_spaces(&mut self) {
        while self.current() == ' ' || self.current() == '\t' {
            self.bump();
        }
    }

    fn skip_until_newline(&mut self) {
        while self.current() != '\n' && self.current() != '\0' {
            self.bump();
        }
    }

    fn parse_ident(&mut self) -> String {
        let mut ident = String::new();
        if self.current().is_ascii_alphabetic() || self.current() == '_' {
            ident.push(self.current());
            self.bump();
        }
        while self.current().is_alphanumeric() || self.current() == '_' {
            ident.push(self.current());
            self.bump();
        }
        ident
    }

    fn parse_file_path(&mut self) -> Option<String> {
        self.skip_spaces();
        if self.current() != '"' {
            return None;
        }
        self.bump();
        let mut file = String::new();
        while self.current() != '"' && self.current() != '\0' {
            file.push(self.current());
            self.bump();
        }
        if self.current() == '"' {
            self.bump();
            return Some(file);
        }
        None
    }

    fn expand_macros(&self, value: &str) -> String {
        let mut result = value.to_string();
        let mut changed = true;
        let max_iterations = 100;
        let mut iterations = 0;

        while changed && iterations < max_iterations {
            changed = false;
            for (name, val) in &self.defines {
                let pattern = format!("${}", name);
                if result.contains(&pattern) {
                    result = result.replace(&pattern, val);
                    changed = true;
                }
            }
            iterations += 1;
        }

        result
    }

    fn check_condition(&mut self, negated: bool) -> bool {
        self.skip_spaces();
        let ident = self.parse_ident();
        let defined = self.defines.contains_key(&ident);

        if negated { !defined } else { defined }
    }

    fn find_import_file(&self, file_name: &str) -> Option<String> {
        let input_dir = if self.base_path.is_empty() {
            ".".to_string()
        } else {
            Path::new(&self.base_path)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or(".")
                .to_string()
        };

        let mut search_paths = vec![
            format!("{}/{}", input_dir, file_name),
            format!("{}/{}.al", input_dir, file_name),
        ];

        for path in &self.include_paths {
            search_paths.push(format!("{}/{}", path, file_name));
            search_paths.push(format!("{}/{}.al", path, file_name));
        }

        for path in &search_paths {
            if Path::new(path).exists() {
                return Some(path.clone());
            }
        }

        None
    }

    pub fn preprocess(&mut self) -> Result<String, PreprocessorError> {
        let mut output = String::new();

        while self.current() != '\0' {
            if self.current() == '/' {
                self.bump();
                if self.current() == '/' {
                    while self.current() != '\n' && self.current() != '\0' {
                        self.bump();
                    }
                    continue;
                }
                output.push('/');
                continue;
            }

            if self.current() == '$' {
                self.bump();
                let cmd = self.parse_ident();

                match cmd.as_str() {
                    "define" => {
                        self.skip_spaces();
                        let name = self.parse_ident();
                        self.skip_spaces();

                        let mut value = String::new();
                        while self.current() != '\n' && self.current() != '\0' {
                            value.push(self.current());
                            self.bump();
                        }

                        let value = value.trim().to_string();
                        let expanded_value = self.expand_macros(&value);
                        self.defines.insert(name, expanded_value);
                    }
                    "ifdef" => {
                        let condition_met = self.check_condition(false);
                        self.condition_stack.push(condition_met);
                        self.skipping = !condition_met;
                    }
                    "ifndef" => {
                        let condition_met = self.check_condition(true);
                        self.condition_stack.push(condition_met);
                        self.skipping = !condition_met;
                    }
                    "endif" => {
                        if let Some(_) = self.condition_stack.pop() {
                            self.skipping = !self.condition_stack.is_empty()
                                && self.condition_stack.last().map(|&s| !s).unwrap_or(false);
                        } else {
                            return Err(PreprocessorError::ConditionError {
                                message: "Unexpected $endif".to_string(),
                                row: self.row,
                                col: self.col,
                            });
                        }
                    }
                    "import" => {
                        if self.skipping {
                            self.skip_until_newline();
                            continue;
                        }

                        let file_name =
                            self.parse_file_path().ok_or(PreprocessorError::IoError {
                                message: "Invalid import path".to_string(),
                                row: self.row,
                                col: self.col,
                            })?;

                        let file_path = self.find_import_file(&file_name).ok_or(
                            PreprocessorError::ImportError {
                                file: file_name,
                                row: self.row,
                                col: self.col,
                            },
                        )?;

                        let content = fs::read_to_string(&file_path).map_err(|e| {
                            PreprocessorError::IoError {
                                message: format!("Failed to read file: {}", e),
                                row: self.row,
                                col: self.col,
                            }
                        })?;

                        let mut child_pp = Preprocessor::new(
                            &content,
                            file_path.clone(),
                            self.include_paths.clone(),
                        );
                        child_pp.defines = self.defines.clone();
                        let processed_sub = child_pp.preprocess()?;
                        output.push_str(&processed_sub);
                        self.defines = child_pp.defines;
                    }
                    _ => {
                        if self.skipping {
                            self.skip_until_newline();
                            continue;
                        }

                        if let Some(val) = self.defines.get(&cmd) {
                            output.push_str(val);
                        } else {
                            output.push('$');
                            output.push_str(&cmd);
                        }
                    }
                }
            } else {
                if self.skipping {
                    self.bump();
                    continue;
                }

                if self.current().is_ascii_alphabetic() || self.current() == '_' {
                    let ident = self.parse_ident();

                    if let Some(val) = self.defines.get(&ident) {
                        output.push_str(val);
                    } else {
                        output.push_str(&ident);
                    }
                } else {
                    output.push(self.current());
                    self.bump();
                }
            }
        }

        if !self.condition_stack.is_empty() {
            return Err(PreprocessorError::ConditionError {
                message: "Unclosed $ifdef or $ifndef".to_string(),
                row: self.row,
                col: self.col,
            });
        }

        Ok(output)
    }
}
