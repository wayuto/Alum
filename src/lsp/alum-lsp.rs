use std::collections::HashMap;
use std::sync::Mutex;

use alc::compiler::{
    Span, lexer::Lexer, parser::Parser, preprocessor::Preprocessor, visitor::TypeChecker,
};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    docs: Mutex<HashMap<Url, String>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions::default()),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "alum-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let source = {
            let docs = self.docs.lock().unwrap();
            docs.get(&uri).cloned()
        };
        let Some(source) = source else {
            return Ok(None);
        };

        let line = source.lines().nth(pos.line as usize).unwrap_or("");
        let prefix = word_prefix(line, pos.character as usize);

        let items = ALUM_KEYWORDS
            .iter()
            .filter(|kw| kw.starts_with(&prefix))
            .map(|kw| CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                insert_text: Some(kw.to_string()),
                ..Default::default()
            })
            .collect();

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.docs.lock().unwrap().insert(uri.clone(), text);
        self.publish_diagnostics(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.last() {
            self.docs
                .lock()
                .unwrap()
                .insert(uri.clone(), change.text.clone());
        }
        self.publish_diagnostics(&uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        self.publish_diagnostics(&uri).await;
    }
}

impl Backend {
    async fn publish_diagnostics(&self, uri: &Url) {
        let source = {
            let docs = self.docs.lock().unwrap();
            docs.get(uri).cloned()
        };
        let Some(source) = source else {
            return;
        };

        let base_path = uri
            .to_file_path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();

        let mut by_file: HashMap<String, Vec<Diagnostic>> = HashMap::new();
        analyze(&source, &base_path, &mut by_file);

        let diagnostics = by_file.remove(&base_path).unwrap_or_default();

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

fn analyze(source: &str, base_path: &str, out: &mut HashMap<String, Vec<Diagnostic>>) {
    let mut preprocessor = Preprocessor::new(source, base_path.to_string(), Vec::new());

    let (processed, source_map) = match preprocessor.preprocess() {
        Ok(res) => res,
        Err(e) => {
            push_diag(source, None, base_path, e.span(), e.to_string(), out);
            return;
        }
    };

    let lexer = Lexer::new(&processed);
    let mut parser = Parser::new(lexer);
    let (mut ast, parse_errors) = parser.parse_collect();

    if !parse_errors.is_empty() {
        for e in parse_errors {
            if let Some(span) = e.span() {
                push_diag(
                    source,
                    Some(&source_map),
                    base_path,
                    span,
                    e.to_string(),
                    out,
                );
            }
        }
        return;
    }

    let checker = TypeChecker::new();
    let check_errors = checker.check_collect(&mut ast);
    for e in check_errors {
        push_diag(
            source,
            Some(&source_map),
            base_path,
            e.span(),
            e.to_string(),
            out,
        );
    }
}

fn push_diag(
    base_source: &str,
    source_map: Option<&alc::compiler::SourceMap>,
    base_path: &str,
    span: Span,
    message: String,
    out: &mut HashMap<String, Vec<Diagnostic>>,
) {
    let (file, src_line, source_text) = match source_map.and_then(|m| m.resolve(span.line)) {
        Some((f, line, src)) => (f.to_string(), line, src.to_string()),
        None => (base_path.to_string(), span.line, base_source.to_string()),
    };

    if file != base_path {
        return;
    }

    let line_text = source_text
        .lines()
        .nth(src_line.saturating_sub(1))
        .unwrap_or("");

    let start = to_position(line_text, src_line, span.col);

    let diagnostics = out.entry(file).or_default();
    diagnostics.push(Diagnostic {
        range: Range {
            start,
            end: Position {
                line: start.line,
                character: start.character + 1,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        message,
        ..Default::default()
    });
}

const ALUM_KEYWORDS: &[&str] = &[
    "bool", "break", "continue", "cst", "else", "enum", "extern", "false", "float", "for", "fun",
    "if", "in", "int", "match", "nil", "return", "string", "struct", "true", "typedef", "union",
    "var", "void", "while",
];

fn word_prefix(line: &str, utf16_col: usize) -> String {
    let mut units = 0;
    let mut byte_idx = line.len();
    for (i, c) in line.char_indices() {
        if units >= utf16_col {
            byte_idx = i;
            break;
        }
        units += c.len_utf16();
    }
    let before = &line[..byte_idx];
    before
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn to_position(line_text: &str, line: usize, col: usize) -> Position {
    let prefix: String = line_text.chars().take(col.saturating_sub(1)).collect();
    Position {
        line: line.saturating_sub(1) as u32,
        character: prefix.encode_utf16().count() as u32,
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        docs: Mutex::new(HashMap::new()),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
