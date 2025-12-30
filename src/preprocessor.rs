use std::{collections::HashMap, fs, iter::Peekable, str::Chars};

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
        }
    }
}

pub struct Preprocessor<'a> {
    src: Peekable<Chars<'a>>,
    path: String,
    row: usize,
    col: usize,

    defines: HashMap<String, String>,
}

impl<'a> Preprocessor<'a> {
    pub fn new(src: &'a str, path: String) -> Self {
        Self {
            src: src.chars().peekable(),
            path,
            row: 1,
            col: 0,
            defines: HashMap::new(),
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

    fn parser_file_path(&mut self) -> Option<String> {
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

    pub fn preprocess(&mut self) -> Result<String, PreprocessorError> {
        let mut output = String::new();

        while self.current() != '\0' {
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
                        self.defines.insert(name, value.trim().to_string());
                    }
                    "import" => {
                        let file_name =
                            self.parser_file_path().ok_or(PreprocessorError::IoError {
                                message: "Invalid import path".to_string(),
                                row: self.row,
                                col: self.col,
                            })?;

                        let paths_to_try = [
                            format!(
                                "{}/{}",
                                if self.path.is_empty() {
                                    "."
                                } else {
                                    &self.path
                                },
                                file_name
                            ),
                            format!(
                                "{}/{}.al",
                                if self.path.is_empty() {
                                    "."
                                } else {
                                    &self.path
                                },
                                file_name
                            ),
                            format!("/usr/local/alum/{}", file_name),
                            format!("/usr/local/alum/{}.al", file_name),
                        ];

                        let mut raw_content = None;
                        for p in &paths_to_try {
                            if let Ok(c) = fs::read_to_string(p) {
                                raw_content = Some(c);
                                break;
                            }
                        }

                        if let Some(content) = raw_content {
                            let mut child_pp = Preprocessor::new(&content, self.path.clone());

                            child_pp.defines = self.defines.clone();

                            let processed_sub = child_pp.preprocess()?;
                            output.push_str(&processed_sub);

                            self.defines = child_pp.defines;
                        } else {
                            return Err(PreprocessorError::ImportError {
                                file: file_name,
                                row: self.row,
                                col: self.col,
                            });
                        }
                    }
                    _ => {
                        if let Some(val) = self.defines.get(&cmd) {
                            output.push_str(val);
                        } else {
                            output.push('$');
                            output.push_str(&cmd);
                        }
                    }
                }
            } else if self.current().is_ascii_alphabetic() || self.current() == '_' {
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
        Ok(output)
    }
}
