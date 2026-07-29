use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(line: usize, col: usize) -> Self {
        Span { line, col }
    }
}

#[derive(Debug, Clone)]
pub struct SourceMap {
    pub files: HashMap<String, String>,
    out_to_src: Vec<(String, usize)>,
}

impl SourceMap {
    pub fn new() -> Self {
        SourceMap {
            files: HashMap::new(),
            out_to_src: vec![],
        }
    }

    pub fn add_file(&mut self, path: String, source: String) {
        self.files.entry(path).or_insert(source);
    }

    pub fn record_line(&mut self, file_path: &str, src_line: usize) {
        self.out_to_src.push((file_path.to_string(), src_line));
    }

    pub fn merge_child(&mut self, child: SourceMap) {
        self.files.extend(child.files);
        self.out_to_src.extend(child.out_to_src);
    }

    pub fn resolve(&self, out_line: usize) -> Option<(&str, usize, &str)> {
        if out_line == 0 {
            return None;
        }
        let (file, src_line) = self.out_to_src.get(out_line - 1)?;
        let source = self.files.get(file.as_str())?;
        Some((file, *src_line, source))
    }
}
