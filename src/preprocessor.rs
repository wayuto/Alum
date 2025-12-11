use std::{fs, iter::Peekable, str::Chars};

pub struct Preprocessor<'a> {
    pos: usize,
    src: Peekable<Chars<'a>>,
    path: String,
}

impl<'a> Preprocessor<'a> {
    pub fn new(src: &'a str, path: String) -> Self {
        Self {
            pos: 0,
            src: src.chars().peekable(),
            path,
        }
    }

    fn current(&mut self) -> char {
        *self.src.peek().unwrap_or(&'\0')
    }

    fn bump(&mut self) -> () {
        self.src.next();
    }

    fn skip_spaces(&mut self) -> () {
        while self.current() == ' ' || self.current() == '\t' || self.current() == '\n' {
            self.bump();
        }
    }

    fn parse_ident(&mut self) -> String {
        let mut ident = String::new();

        if self.current().is_ascii_alphabetic() {
            ident.push(self.current());
            self.bump();
        }

        while self.current().is_alphanumeric() {
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

    pub fn preprocess(&mut self) -> String {
        let mut output = String::new();
        while self.current() != '\0' {
            output.push_str(
                (|| {
                    let mut chunk = String::new();
                    while self.current() != '\0' && self.current() != '$' {
                        chunk.push(self.current());
                        self.bump();
                    }
                    chunk
                })()
                .as_str(),
            );
            if self.current() == '\0' {
                break;
            }
            if self.current() == '$' {
                let start_pos = self.pos;
                self.bump();
                let cmd = self.parse_ident();

                match cmd.as_str() {
                    "import" => {
                        let file = self.parser_file_path();
                        if let Some(file) = file {
                            if self.path.is_empty() {
                                self.path = '.'.to_string();
                            }
                            let raw =
                                fs::read_to_string(format!("{}/{}", self.path, file)).unwrap();
                            let mut pp = Preprocessor::new(raw.as_str(), self.path.clone());
                            let conent = pp.preprocess();
                            output.push_str(&conent);
                        } else {
                            self.pos = start_pos;
                            output.push(self.current());
                            self.bump();
                        }
                    }
                    _ => {
                        self.pos = start_pos;
                        output.push(self.current());
                        self.bump();
                    }
                }
            }
        }
        output
    }
}
