use crate::compiler::{
    SourceMap, Span, codegen::CodeGenError, lexer::LexerError, parser::ParserError,
    preprocessor::PreprocessorError, visitor::checker::CheckerError,
};
use std::{error, fmt, io};

#[derive(Debug)]
pub struct CompilerError {
    source_map: SourceMap,
    kind: CompilerErrorKind,
}

#[derive(Debug)]
enum CompilerErrorKind {
    Io(io::Error),
    Preprocessor(PreprocessorError),
    Lexer(LexerError),
    Parser(ParserError),
    Checker(CheckerError),
    CodeGen(CodeGenError),
}

impl CompilerError {
    pub fn new<E: Into<CompilerError>>(e: E, source_map: SourceMap) -> Self {
        let mut ce = e.into();
        ce.source_map = source_map;
        ce
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            CompilerErrorKind::Io(e) => write!(f, "IO error: {}", e),
            CompilerErrorKind::Preprocessor(e) => write!(f, "Preprocessor error: {}", e),
            CompilerErrorKind::Lexer(e) => write!(f, "Lexer error: {}", e),
            CompilerErrorKind::Parser(e) => write!(f, "Parser error: {}", e),
            CompilerErrorKind::Checker(e) => write!(f, "Type check error: {}", e),
            CompilerErrorKind::CodeGen(e) => write!(f, "Code generation error: {}", e),
        }
    }
}

impl error::Error for CompilerError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            CompilerErrorKind::Io(e) => Some(e),
            CompilerErrorKind::Preprocessor(e) => Some(e),
            CompilerErrorKind::Lexer(e) => Some(e),
            CompilerErrorKind::Parser(e) => Some(e),
            CompilerErrorKind::Checker(e) => Some(e),
            CompilerErrorKind::CodeGen(e) => Some(e),
        }
    }
}

impl From<io::Error> for CompilerError {
    fn from(e: io::Error) -> Self {
        CompilerError {
            source_map: SourceMap::new(),
            kind: CompilerErrorKind::Io(e),
        }
    }
}

impl From<PreprocessorError> for CompilerError {
    fn from(e: PreprocessorError) -> Self {
        CompilerError {
            source_map: SourceMap::new(),
            kind: CompilerErrorKind::Preprocessor(e),
        }
    }
}

impl From<LexerError> for CompilerError {
    fn from(e: LexerError) -> Self {
        CompilerError {
            source_map: SourceMap::new(),
            kind: CompilerErrorKind::Lexer(e),
        }
    }
}

impl From<ParserError> for CompilerError {
    fn from(e: ParserError) -> Self {
        CompilerError {
            source_map: SourceMap::new(),
            kind: CompilerErrorKind::Parser(e),
        }
    }
}

impl From<CheckerError> for CompilerError {
    fn from(e: CheckerError) -> Self {
        CompilerError {
            source_map: SourceMap::new(),
            kind: CompilerErrorKind::Checker(e),
        }
    }
}

impl From<CodeGenError> for CompilerError {
    fn from(e: CodeGenError) -> Self {
        CompilerError {
            source_map: SourceMap::new(),
            kind: CompilerErrorKind::CodeGen(e),
        }
    }
}

impl CompilerError {
    pub fn span(&self) -> Option<Span> {
        match &self.kind {
            CompilerErrorKind::Io(_) => None,
            CompilerErrorKind::Preprocessor(e) => Some(e.span()),
            CompilerErrorKind::Lexer(e) => Some(e.span()),
            CompilerErrorKind::Parser(e) => e.span(),
            CompilerErrorKind::Checker(e) => Some(e.span()),
            CompilerErrorKind::CodeGen(e) => Some(e.span()),
        }
    }

    pub fn diagnose(&self) -> String {
        let mut out = format!("error: {}\n", self);

        if let Some(span) = self.span() {
            if let Some((file_path, src_line, source)) = self.source_map.resolve(span.line) {
                out.push_str(&format!("  --> {}:{}:{}\n", file_path, src_line, span.col));
                out.push_str("   |\n");

                if let Some(line) = source.lines().nth(src_line.saturating_sub(1)) {
                    let prefix = format!("{:>3} | ", src_line);
                    out.push_str(&format!("{}{}\n", prefix, line));
                    out.push_str(&format!(
                        "{} {}{}\n",
                        " ".repeat(prefix.len()),
                        " ".repeat(span.col.saturating_sub(1)),
                        "^---",
                    ));
                }
            } else {
                out.push_str(&format!("  --> at line:{} col:{}\n", span.line, span.col));
            }
        }

        out
    }
}
