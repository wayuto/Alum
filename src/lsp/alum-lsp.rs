use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use alc::compiler::{
    Span, lexer::Lexer, modules::DeclKind, parser::Parser, preprocessor::Preprocessor,
    visitor::TypeChecker,
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
            let docs = lock_docs(&self.docs);
            docs.get(&uri).cloned()
        };
        let Some(source) = source else {
            return Ok(None);
        };

        let line = source.lines().nth(pos.line as usize).unwrap_or("");
        let prefix = word_prefix(line, pos.character as usize);

        let base_path = uri
            .to_file_path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();

        if let Some((mod_name, member_prefix)) = module_member_context(line, pos.character as usize)
        {
            if let Some(members) =
                parse_loaded_modules(&source, &base_path).and_then(|m| m.get(&mod_name).cloned())
            {
                let items = members
                    .iter()
                    .filter(|(n, _)| n.starts_with(&member_prefix))
                    .map(|(n, kind)| CompletionItem {
                        label: n.clone(),
                        kind: Some(item_kind(*kind)),
                        insert_text: Some(n.clone()),
                        ..Default::default()
                    })
                    .collect();
                return Ok(Some(CompletionResponse::Array(items)));
            }
        }

        if let Some(mod_prefix) = import_module_prefix(line, pos.character as usize) {
            let items = available_modules(&base_path)
                .iter()
                .filter(|n| n.starts_with(&mod_prefix))
                .map(|n| CompletionItem {
                    label: n.clone(),
                    kind: Some(CompletionItemKind::MODULE),
                    insert_text: Some(n.clone()),
                    ..Default::default()
                })
                .collect();
            return Ok(Some(CompletionResponse::Array(items)));
        }

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
        lock_docs(&self.docs).insert(uri.clone(), text);
        self.publish_diagnostics(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.last() {
            lock_docs(&self.docs).insert(uri.clone(), change.text.clone());
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
            let docs = lock_docs(&self.docs);
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

        let mut published_current = false;
        for (file, diagnostics) in by_file {
            if let Ok(file_url) = Url::from_file_path(&file) {
                if &file_url == uri {
                    published_current = true;
                }
                self.client
                    .publish_diagnostics(file_url, diagnostics, None)
                    .await;
            }
        }

        if !published_current {
            self.client
                .publish_diagnostics(uri.clone(), Vec::new(), None)
                .await;
        }
    }
}

fn lock_docs(
    docs: &Mutex<HashMap<Url, String>>,
) -> std::sync::MutexGuard<'_, HashMap<Url, String>> {
    docs.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn analyze(source: &str, base_path: &str, out: &mut HashMap<String, Vec<Diagnostic>>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        analyze_inner(source, base_path)
    }));
    if let Ok(by_file) = result {
        for (file, diags) in by_file {
            out.entry(file).or_default().extend(diags);
        }
    }
}

fn analyze_inner(source: &str, base_path: &str) -> HashMap<String, Vec<Diagnostic>> {
    let mut out: HashMap<String, Vec<Diagnostic>> = HashMap::new();
    let mut preprocessor = Preprocessor::new(source, base_path.to_string(), Vec::new());

    let (processed, source_map) = match preprocessor.preprocess() {
        Ok(res) => res,
        Err(e) => {
            push_diag(source, None, base_path, e.span(), e.to_string(), &mut out);
            return out;
        }
    };

    let lexer = Lexer::new(&processed);
    let mut parser = Parser::new(lexer, base_path.to_string(), Vec::new());
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
                    &mut out,
                );
            }
        }
        return out;
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
            &mut out,
        );
    }
    out
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
    "as", "bool", "break", "continue", "cst", "else", "enum", "extern", "false", "float", "for",
    "fun", "if", "import", "in", "int", "match", "nil", "return", "string", "struct", "true",
    "typedef", "union", "using", "var", "void", "while",
];

fn split_at_utf16(line: &str, utf16_col: usize) -> (&str, &str) {
    let mut units = 0;
    for (i, c) in line.char_indices() {
        if units >= utf16_col {
            return (&line[..i], &line[i..]);
        }
        units += c.len_utf16();
    }
    (line, "")
}

fn module_member_context(line: &str, utf16_col: usize) -> Option<(String, String)> {
    let (before, _) = split_at_utf16(line, utf16_col);
    let idx = before.rfind("::")?;
    let head = &before[..idx];
    let mod_name = head
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next_back()
        .unwrap_or("")
        .to_string();
    if mod_name.is_empty() {
        return None;
    }

    let prefix: String = before[idx + 2..]
        .chars()
        .skip_while(|c| *c == '{')
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    Some((mod_name, prefix))
}

fn import_module_prefix(line: &str, utf16_col: usize) -> Option<String> {
    let (before, _) = split_at_utf16(line, utf16_col);
    let words: Vec<&str> = before
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .collect();
    let len = words.len();
    if len >= 1 && (words[len - 1] == "import" || words[len - 1] == "using") {
        Some(String::new())
    } else if len >= 2 && (words[len - 2] == "import" || words[len - 2] == "using") {
        Some(words[len - 1].to_string())
    } else {
        None
    }
}

fn parse_loaded_modules(
    source: &str,
    base_path: &str,
) -> Option<HashMap<String, Vec<(String, DeclKind)>>> {
    let mut preprocessor = Preprocessor::new(source, base_path.to_string(), Vec::new());
    let (processed, _) = preprocessor.preprocess().ok()?;
    let lexer = Lexer::new(&processed);
    let mut parser = Parser::new(lexer, base_path.to_string(), Vec::new());
    let _ = parser.parse_collect();
    let mut members = HashMap::new();
    for name in parser.loaded_module_names() {
        if let Some(m) = parser.module_members(&name) {
            members.insert(name, m);
        }
    }
    Some(members)
}

fn available_modules(base_path: &str) -> Vec<String> {
    let mut dirs: Vec<String> = Vec::new();
    if let Some(dir) = Path::new(base_path)
        .parent()
        .and_then(|p| p.to_str())
        .filter(|s| !s.is_empty())
    {
        dirs.push(dir.to_string());
    }
    dirs.push("/usr/local/include/alum".to_string());
    dirs.push("/usr/local/alum".to_string());

    let mut names = Vec::new();
    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                for ext in [".al", ".ah"] {
                    if let Some(stem) = name.strip_suffix(ext) {
                        names.push(stem.to_string());
                    }
                }
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

fn item_kind(kind: DeclKind) -> CompletionItemKind {
    match kind {
        DeclKind::Fn | DeclKind::ExternFn => CompletionItemKind::FUNCTION,
        DeclKind::Struct => CompletionItemKind::STRUCT,
        DeclKind::Union => CompletionItemKind::STRUCT,
        DeclKind::Enum => CompletionItemKind::ENUM,
        DeclKind::Const | DeclKind::GlobalVar | DeclKind::ExternVar => CompletionItemKind::VARIABLE,
    }
}

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
