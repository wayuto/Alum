use std::{collections::HashMap, fs, iter::Peekable, path::Path, str::Chars};

#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub params: Vec<String>,
    pub body: String,
}

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
    MacroError {
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
            PreprocessorError::MacroError { message, row, col } => {
                write!(f, "Macro error at {}:{}: {}", row, col, message)
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

            for (name, macro_def) in &self.defines {
                let macro_pattern = format!("{}(", name);
                let mut search_start = 0;

                while let Some(pos) = result[search_start..].find(&macro_pattern) {
                    let abs_pos = search_start + pos;
                    let args_start = abs_pos + name.len();

                    let (args_str, args_end) = match self.extract_args(&result, args_start) {
                        Some(args) => args,
                        None => {
                            search_start = abs_pos + 1;
                            continue;
                        }
                    };

                    let args: Vec<String> =
                        args_str.split(',').map(|s| s.trim().to_string()).collect();

                    if args.len() != macro_def.params.len() {
                        search_start = abs_pos + 1;
                        continue;
                    }

                    let expanded_body =
                        self.substitute_params(&macro_def.body, &macro_def.params, &args);

                    let before = &result[..abs_pos];
                    let after = &result[args_end..];
                    result = format!("{}{}{}", before, expanded_body, after);

                    changed = true;
                    search_start = abs_pos + expanded_body.len();
                }
            }

            for (name, macro_def) in &self.defines {
                if macro_def.params.is_empty() {
                    let mut new_result = String::new();
                    let mut i = 0;
                    while i < result.len() {
                        if result[i..].starts_with(name) {
                            let before = if i > 0 {
                                Some(result.chars().nth(i - 1).unwrap())
                            } else {
                                None
                            };
                            let after = if i + name.len() < result.len() {
                                Some(result.chars().nth(i + name.len()).unwrap())
                            } else {
                                None
                            };

                            let is_ident_before =
                                before.map_or(false, |c| c.is_alphanumeric() || c == '_');
                            let is_ident_after =
                                after.map_or(false, |c| c.is_alphanumeric() || c == '_');

                            if !is_ident_before && !is_ident_after {
                                new_result.push_str(&macro_def.body);
                                i += name.len();
                                changed = true;
                                continue;
                            }
                        }
                        new_result.push(result.chars().nth(i).unwrap());
                        i += 1;
                    }
                    result = new_result;
                }
            }

            iterations += 1;
        }

        result
    }

    fn extract_args(&self, text: &str, start: usize) -> Option<(String, usize)> {
        if start >= text.len() || text.as_bytes()[start] != b'(' {
            return None;
        }

        let mut depth = 1;
        let args_start = start + 1;
        let mut args_end = start;
        let mut in_string = false;
        let mut string_char = '\0';

        for (i, c) in text[start + 1..].char_indices() {
            let pos = start + 1 + i;

            if c == '"' || c == '\'' {
                if !in_string {
                    in_string = true;
                    string_char = c;
                } else if c == string_char {
                    in_string = false;
                }
            }

            if !in_string {
                if c == '(' {
                    depth += 1;
                } else if c == ')' {
                    if depth == 1 {
                        args_end = pos;
                        break;
                    }
                    depth -= 1;
                }
            }
        }

        if depth != 1 {
            return None;
        }

        let args_str = text[args_start..args_end].trim().to_string();
        Some((args_str, args_end + 1))
    }

    fn substitute_params(&self, body: &str, params: &[String], args: &[String]) -> String {
        let mut result = body.to_string();

        for (param, arg) in params.iter().zip(args.iter()) {
            result = result.replace(param, arg);
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

                        let (params, value) = if self.current() == '(' {
                            self.bump();
                            let mut params = Vec::new();
                            let mut current_param = String::new();

                            while self.current() != ')' && self.current() != '\0' {
                                if self.current() == ',' {
                                    if !current_param.trim().is_empty() {
                                        params.push(current_param.trim().to_string());
                                    }
                                    current_param.clear();
                                    self.bump();
                                    self.skip_spaces();
                                } else if self.current().is_alphanumeric() || self.current() == '_'
                                {
                                    current_param.push(self.current());
                                    self.bump();
                                } else if !self.current().is_whitespace() {
                                    return Err(PreprocessorError::MacroError {
                                        message: format!(
                                            "Invalid parameter name: '{}'",
                                            self.current()
                                        ),
                                        row: self.row,
                                        col: self.col,
                                    });
                                } else {
                                    self.bump();
                                }
                            }

                            if !current_param.trim().is_empty() {
                                params.push(current_param.trim().to_string());
                            }

                            if self.current() != ')' {
                                return Err(PreprocessorError::MacroError {
                                    message: "Unclosed parameter list in macro definition"
                                        .to_string(),
                                    row: self.row,
                                    col: self.col,
                                });
                            }
                            self.bump();
                            self.skip_spaces();

                            let mut macro_body = String::new();
                            while self.current() != '\n' && self.current() != '\0' {
                                macro_body.push(self.current());
                                self.bump();
                            }

                            (params, macro_body.trim().to_string())
                        } else {
                            let mut simple_value = String::new();
                            while self.current() != '\n' && self.current() != '\0' {
                                simple_value.push(self.current());
                                self.bump();
                            }
                            (Vec::new(), simple_value.trim().to_string())
                        };

                        let expanded_value = self.expand_macros(&value);

                        self.defines.insert(
                            name,
                            MacroDefinition {
                                params,
                                body: expanded_value,
                            },
                        );
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

                        if let Some(macro_def) = self.defines.get(&cmd) {
                            if macro_def.params.is_empty() {
                                output.push_str(&macro_def.body);
                            } else {
                                output.push('$');
                                output.push_str(&cmd);
                            }
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
                    output.push_str(&ident);
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

        let expanded_output = self.expand_macros(&output);
        Ok(expanded_output)
    }
}
