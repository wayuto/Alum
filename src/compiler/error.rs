use crate::compiler::{
    SourceMap, Span, codegen::CodeGenError, lexer::LexerError, parser::ParserError,
    preprocessor::PreprocessorError, visitor::checker::CheckerError,
};
use std::{error, fmt, io};

#[derive(Debug)]
pub struct CompilerError {
    source_map: SourceMap,
    errors: Vec<CompilerErrorKind>,
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

    /// Builds a report from several errors of the same stage, sharing one
    /// `SourceMap` for line resolution.
    pub fn report<E: Into<CompilerError>>(
        source_map: SourceMap,
        errors: impl IntoIterator<Item = E>,
    ) -> Self {
        let mut ce = CompilerError {
            source_map,
            errors: Vec::new(),
        };
        for e in errors {
            let single: CompilerError = e.into();
            ce.errors.extend(single.errors);
        }
        ce
    }

    fn kind_string(kind: &CompilerErrorKind) -> String {
        match kind {
            CompilerErrorKind::Io(e) => format!("IO error: {}", e),
            CompilerErrorKind::Preprocessor(e) => format!("Preprocessor error: {}", e),
            CompilerErrorKind::Lexer(e) => format!("Lexer error: {}", e),
            CompilerErrorKind::Parser(e) => format!("Parser error: {}", e),
            CompilerErrorKind::Checker(e) => format!("Type check error: {}", e),
            CompilerErrorKind::CodeGen(e) => format!("Code generation error: {}", e),
        }
    }

    fn span_of(kind: &CompilerErrorKind) -> Option<Span> {
        match kind {
            CompilerErrorKind::Io(_) => None,
            CompilerErrorKind::Preprocessor(e) => Some(e.span()),
            CompilerErrorKind::Lexer(e) => Some(e.span()),
            CompilerErrorKind::Parser(e) => e.span(),
            CompilerErrorKind::Checker(e) => Some(e.span()),
            CompilerErrorKind::CodeGen(e) => Some(e.span()),
        }
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let joined: Vec<String> = self.errors.iter().map(Self::kind_string).collect();
        f.write_str(&joined.join("\n"))
    }
}

impl error::Error for CompilerError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.errors.first()? {
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
            errors: vec![CompilerErrorKind::Io(e)],
        }
    }
}

impl From<PreprocessorError> for CompilerError {
    fn from(e: PreprocessorError) -> Self {
        CompilerError {
            source_map: SourceMap::new(),
            errors: vec![CompilerErrorKind::Preprocessor(e)],
        }
    }
}

impl From<LexerError> for CompilerError {
    fn from(e: LexerError) -> Self {
        CompilerError {
            source_map: SourceMap::new(),
            errors: vec![CompilerErrorKind::Lexer(e)],
        }
    }
}

impl From<ParserError> for CompilerError {
    fn from(e: ParserError) -> Self {
        CompilerError {
            source_map: SourceMap::new(),
            errors: vec![CompilerErrorKind::Parser(e)],
        }
    }
}

impl From<CheckerError> for CompilerError {
    fn from(e: CheckerError) -> Self {
        CompilerError {
            source_map: SourceMap::new(),
            errors: vec![CompilerErrorKind::Checker(e)],
        }
    }
}

impl From<CodeGenError> for CompilerError {
    fn from(e: CodeGenError) -> Self {
        CompilerError {
            source_map: SourceMap::new(),
            errors: vec![CompilerErrorKind::CodeGen(e)],
        }
    }
}

impl CompilerError {
    /// Returns the span of the first error, if it has one.
    pub fn span(&self) -> Option<Span> {
        Self::span_of(self.errors.first()?)
    }

    pub fn diagnose(&self) -> String {
        let mut out = String::new();
        for (i, kind) in self.errors.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&format!("error: {}\n", Self::kind_string(kind)));

            if let Some(span) = Self::span_of(kind) {
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
        }
        out
    }
}
