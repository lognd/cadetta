//! frob:ticket 01M4SWN8MPH5ARCRJG4QB9ANVQ
//! `cadetta lsp`: a Language Server Protocol 3.17 server on stdio, a thin layer over `check`
//! and `fmt` (config-cli.md 3.8). It adds no rule and no diagnostic of its own.
//!
//! Transport is Content-Length framed JSON-RPC on stdin/stdout (the `lsp-server` crate);
//! stdout carries protocol only and logs go to stderr through `cadetta-log`. Buffer parse
//! errors (E001) are published at once on open and change; the full `check` pipeline runs on
//! open and save, on a worker thread per package, against the files on disk, and a result
//! whose package was re-checked meanwhile is dropped.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use crossbeam_channel::{Receiver, Sender, select, unbounded};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CodeLensOptions, CodeLensParams, Diagnostic as LspDiagnostic, DiagnosticSeverity,
    DocumentFormattingParams, GotoDefinitionParams, HoverParams, HoverProviderCapability,
    InitializeParams, NumberOrString, PositionEncodingKind, PublishDiagnosticsParams,
    Range as LspRange, SaveOptions, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions, TextEdit, Url,
};
use serde_json::{Value, json};

use cadetta_lint::diag::{Diagnostic, Location, Severity};
use cadetta_syntax::format::format_source;
use cadetta_syntax::parser::parse;
use cadetta_syntax::span::SourceFile;

use super::check::{no_drift, run_check_with};
use super::lsp_query::{self, FileEntries};
use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::{CONFIG_FILE, Project, SOURCE_EXT, find_config};
use crate::{EXIT_FAIL, EXIT_OK, EXIT_USAGE, Globals};

/// The lock file name, where lock locations are published.
const LOCK_FILE: &str = "cadetta.lock";

/// The `source` every published diagnostic carries.
const SOURCE: &str = "cadetta";

/// The negotiated position encoding (config-cli.md 3.8): `utf-32` when offered, else `utf-16`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-16 code units (the LSP default).
    Utf16,
    /// Unicode scalar values, which is what `cadetta.diag/1` columns already count.
    Utf32,
}

impl Encoding {
    /// Pick the encoding from the client's offered list (absent means only UTF-16).
    pub fn negotiate(offered: &[PositionEncodingKind]) -> Self {
        if offered.contains(&PositionEncodingKind::UTF32) {
            Self::Utf32
        } else {
            Self::Utf16
        }
    }

    /// The LSP name advertised in `positionEncoding`.
    pub fn kind(self) -> PositionEncodingKind {
        match self {
            Self::Utf16 => PositionEncodingKind::UTF16,
            Self::Utf32 => PositionEncodingKind::UTF32,
        }
    }

    /// How many units `ch` takes in this encoding.
    pub(super) fn units(self, ch: char) -> u32 {
        match self {
            Self::Utf16 => ch.len_utf16() as u32,
            Self::Utf32 => 1,
        }
    }
}

/// Convert a 1-based line and 1-based character column (as `cadetta.diag/1` reports them) to
/// a 0-based LSP position in `enc`, using `text` to measure the line.
pub fn to_lsp_position(text: &str, line: u32, col: u32, enc: Encoding) -> lsp_types::Position {
    let line0 = line.saturating_sub(1);
    let line_text = text.split('\n').nth(line0 as usize).unwrap_or("");
    let want = col.saturating_sub(1) as usize;
    let mut character = 0u32;
    let mut seen = 0usize;
    for ch in line_text.chars().take(want) {
        character += enc.units(ch);
        seen += 1;
    }
    // A column past the end of the line counts one unit per missing character.
    character += (want - seen) as u32;
    lsp_types::Position::new(line0, character)
}

/// The position just past the last character of `text` in `enc`.
pub fn end_position(text: &str, enc: Encoding) -> lsp_types::Position {
    let line = text.matches('\n').count() as u32;
    let last = text.rsplit('\n').next().unwrap_or("");
    let character = last.chars().map(|c| enc.units(c)).sum();
    lsp_types::Position::new(line, character)
}

/// The LSP severity number for a `cadetta.diag/1` severity (error 1 .. hint 4).
fn severity_of(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
        Severity::Hint => DiagnosticSeverity::HINT,
    }
}

/// The message followed by its `note:` and `help:` lines (config-cli.md 3.8).
fn message_of(d: &Diagnostic) -> String {
    let mut message = d.message.clone();
    for note in &d.notes {
        message.push_str("\nnote: ");
        message.push_str(&note.message);
    }
    for fix in &d.fixes {
        message.push_str("\nhelp: ");
        message.push_str(&fix.title);
    }
    message
}

/// The `data` object: the canonical `path` and `variant` when present.
fn data_of(d: &Diagnostic) -> Option<Value> {
    let mut data = serde_json::Map::new();
    if let Some(path) = &d.path {
        data.insert("path".to_owned(), json!(path));
    }
    if let Some(alt) = &d.alt {
        data.insert("variant".to_owned(), json!(alt));
    }
    (!data.is_empty()).then_some(Value::Object(data))
}

/// Map a located `cadetta.diag/1` record to an LSP diagnostic; `text` is the text of the file
/// the location is in (used to convert columns into `enc`). `None` for a native or absent
/// location, which is logged rather than published.
pub fn map_diagnostic(d: &Diagnostic, text: &str, enc: Encoding) -> Option<LspDiagnostic> {
    let range = match d.location.as_ref()? {
        Location::Source { start, end, .. } => LspRange::new(
            to_lsp_position(text, start.line, start.col, enc),
            to_lsp_position(text, end.line, end.col, enc),
        ),
        Location::Lock { line } => {
            let start = to_lsp_position(text, *line, 1, enc);
            let end = lsp_types::Position::new(
                start.line,
                text.split('\n')
                    .nth(start.line as usize)
                    .unwrap_or("")
                    .chars()
                    .map(|c| enc.units(c))
                    .sum(),
            );
            LspRange::new(start, end)
        }
        Location::Native { .. } => return None,
    };
    Some(LspDiagnostic {
        range,
        severity: Some(severity_of(d.severity)),
        code: Some(NumberOrString::String(d.code.clone())),
        code_description: None,
        source: Some(SOURCE.to_owned()),
        message: message_of(d),
        related_information: None,
        tags: None,
        data: data_of(d),
    })
}

/// The E001 diagnostics of a buffer, mapped into `enc` (published on open and change).
pub fn syntax_diagnostics(text: &str, enc: Encoding) -> Vec<LspDiagnostic> {
    let source = SourceFile::new("", text.to_owned());
    parse(text)
        .diags
        .iter()
        .filter(|d| d.code.code() == "E001")
        .map(|d| {
            let (a, b) = (source.line_col(d.span.start), source.line_col(d.span.end));
            LspDiagnostic {
                range: LspRange::new(
                    to_lsp_position(text, a.line as u32, a.col as u32, enc),
                    to_lsp_position(text, b.line as u32, b.col as u32, enc),
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("E001".to_owned())),
                source: Some(SOURCE.to_owned()),
                message: d.message.clone(),
                ..LspDiagnostic::default()
            }
        })
        .collect()
}

/// The server capabilities: sync, formatting, hover, code lens, definition and the encoding.
fn capabilities(enc: Encoding) -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: Some(enc.kind()),
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: None,
                will_save_wait_until: None,
                save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                    include_text: Some(false),
                })),
            },
        )),
        document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: Some(false),
        }),
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        ..ServerCapabilities::default()
    }
}

/// The outcome of one package check, sent from a worker thread to the server loop.
struct PackageResult {
    /// The package root (key of the supersede counter).
    root: PathBuf,
    /// The run number this result belongs to.
    generation: u64,
    /// Diagnostics per file, empty lists included so stale ones clear.
    files: BTreeMap<PathBuf, Vec<LspDiagnostic>>,
    /// The hover, lens and definition index of the package.
    index: FileEntries,
}

/// Run the full `check` for the package containing `dir` against the files on disk and map
/// the unwaived records per file; the error is a message to log.
fn check_package(
    dir: &Path,
    enc: Encoding,
) -> Result<(BTreeMap<PathBuf, Vec<LspDiagnostic>>, FileEntries), String> {
    let (project, scope) = Project::open(Some(dir), None).map_err(|e| e.to_string())?;
    let globals = Globals {
        format: Format::Json,
        cwd: Some(dir.to_owned()),
        config: None,
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run_check_with(&[], &globals, &mut out, &mut err, &no_drift);
    let report: Value = serde_json::from_slice(&out).map_err(|_| {
        format!(
            "check produced no report (exit {code}): {}",
            String::from_utf8_lossy(&err).trim()
        )
    })?;
    let records: Vec<Diagnostic> =
        serde_json::from_value(report["diagnostics"].clone()).map_err(|e| e.to_string())?;
    tracing::info!(root = %project.root.display(), records = records.len(), code, "lsp: check finished");

    let mut files: BTreeMap<PathBuf, Vec<LspDiagnostic>> = BTreeMap::new();
    for &i in &scope {
        for module in &project.packages[i].modules {
            files.entry(module.file.clone()).or_default();
        }
    }
    let mut texts: HashMap<PathBuf, String> = HashMap::new();
    for d in &records {
        if d.waived.is_some() {
            tracing::debug!(code = %d.code, "lsp: waived diagnostic not published");
            continue;
        }
        let file = match &d.location {
            Some(Location::Source { file, .. }) => project.root.join(file),
            Some(Location::Lock { .. }) => project.root.join(LOCK_FILE),
            Some(Location::Native { .. }) | None => {
                tracing::info!(code = %d.code, message = %d.message, "lsp: no publishable location, logged only");
                continue;
            }
        };
        let text = texts
            .entry(file.clone())
            .or_insert_with(|| std::fs::read_to_string(&file).unwrap_or_default());
        if let Some(mapped) = map_diagnostic(d, text, enc) {
            files.entry(file).or_default().push(mapped);
        }
    }
    let analysis = analyze(&project, &scope);
    let index = lsp_query::build_index(&project, &analysis);
    Ok((files, index))
}

/// The server state behind the message loop.
struct Server<'a> {
    conn: &'a Connection,
    enc: Encoding,
    /// Open buffers by URI.
    docs: HashMap<Url, String>,
    /// Latest check run number per package root.
    generation: HashMap<PathBuf, u64>,
    /// URIs last published by a package check, per root, to clear the ones that vanish.
    published: HashMap<PathBuf, HashSet<Url>>,
    /// Query index per package root, replaced on each finished check.
    index: HashMap<PathBuf, FileEntries>,
    results: Sender<PackageResult>,
    shutdown: bool,
}

impl Server<'_> {
    /// Send one message to the client; a closed pipe is logged, not fatal.
    fn send(&self, message: Message) {
        if let Err(error) = self.conn.sender.send(message) {
            tracing::warn!(%error, "lsp: client connection closed");
        }
    }

    /// Publish `diagnostics` for `uri` (an empty list clears).
    fn publish(&self, uri: Url, diagnostics: Vec<LspDiagnostic>) {
        tracing::debug!(%uri, count = diagnostics.len(), "lsp: publishDiagnostics");
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        };
        self.send(Message::Notification(Notification::new(
            "textDocument/publishDiagnostics".to_owned(),
            params,
        )));
    }

    /// Publish the buffer's E001 diagnostics for `uri`.
    fn publish_syntax(&self, uri: &Url) {
        if let Some(text) = self.docs.get(uri) {
            self.publish(uri.clone(), syntax_diagnostics(text, self.enc));
        }
    }

    /// Start a full check of the package that holds `uri`; a run still going for it is
    /// superseded (its result will be dropped). Documents outside a package get none.
    fn schedule_check(&mut self, uri: &Url) {
        let Ok(path) = uri.to_file_path() else {
            tracing::debug!(%uri, "lsp: not a file uri, no package check");
            return;
        };
        let dir = path.parent().unwrap_or(Path::new(".")).to_owned();
        let root = match find_config(&dir) {
            Ok(config) => config.parent().unwrap_or(Path::new(".")).to_owned(),
            Err(error) => {
                tracing::info!(%error, file = %path.display(), "lsp: no package, syntax diagnostics only");
                return;
            }
        };
        let generation = self.generation.entry(root.clone()).or_insert(0);
        *generation += 1;
        let generation = *generation;
        tracing::info!(root = %root.display(), generation, "lsp: package check scheduled");
        let (tx, enc) = (self.results.clone(), self.enc);
        std::thread::spawn(move || match check_package(&dir, enc) {
            Ok((files, index)) => {
                let _ = tx.send(PackageResult {
                    root,
                    generation,
                    files,
                    index,
                });
            }
            Err(message) => tracing::error!(%message, "lsp: package check failed"),
        });
    }

    /// Publish a finished check unless a newer run superseded it.
    fn apply_result(&mut self, result: PackageResult) {
        if self.generation.get(&result.root) != Some(&result.generation) {
            tracing::info!(root = %result.root.display(), generation = result.generation, "lsp: superseded check dropped");
            return;
        }
        self.index.insert(result.root.clone(), result.index);
        let mut now = HashSet::new();
        for (path, diagnostics) in result.files {
            let Ok(uri) = Url::from_file_path(&path) else {
                tracing::warn!(file = %path.display(), "lsp: cannot make a uri, not published");
                continue;
            };
            self.publish(uri.clone(), diagnostics);
            now.insert(uri);
        }
        let before = self
            .published
            .insert(result.root, now.clone())
            .unwrap_or_default();
        for gone in before.difference(&now) {
            self.publish(gone.clone(), Vec::new());
        }
    }

    /// Whether `uri` names a `.cdt` source (the only documents this server handles).
    fn is_source(uri: &Url) -> bool {
        uri.path().ends_with(&format!(".{SOURCE_EXT}"))
    }

    /// Handle one notification; `Some(code)` means exit with that status.
    fn notification(&mut self, n: Notification) -> Option<u8> {
        tracing::debug!(method = %n.method, "lsp: notification");
        match n.method.as_str() {
            "exit" => {
                let code = if self.shutdown { EXIT_OK } else { EXIT_FAIL };
                tracing::info!(code, "lsp: exit");
                return Some(code);
            }
            "textDocument/didOpen" => {
                if let Ok(p) =
                    serde_json::from_value::<lsp_types::DidOpenTextDocumentParams>(n.params)
                {
                    let uri = p.text_document.uri;
                    if Self::is_source(&uri) {
                        self.docs.insert(uri.clone(), p.text_document.text);
                        self.publish_syntax(&uri);
                        self.schedule_check(&uri);
                    }
                }
            }
            "textDocument/didChange" => {
                if let Ok(p) =
                    serde_json::from_value::<lsp_types::DidChangeTextDocumentParams>(n.params)
                {
                    let uri = p.text_document.uri;
                    // Full sync: the last change carries the whole text.
                    if let (true, Some(change)) = (
                        self.docs.contains_key(&uri),
                        p.content_changes.into_iter().last(),
                    ) {
                        self.docs.insert(uri.clone(), change.text);
                        self.publish_syntax(&uri);
                    }
                }
            }
            "textDocument/didSave" => {
                if let Ok(p) =
                    serde_json::from_value::<lsp_types::DidSaveTextDocumentParams>(n.params)
                    && Self::is_source(&p.text_document.uri)
                {
                    self.schedule_check(&p.text_document.uri);
                }
            }
            "textDocument/didClose" => {
                if let Ok(p) =
                    serde_json::from_value::<lsp_types::DidCloseTextDocumentParams>(n.params)
                {
                    tracing::debug!(uri = %p.text_document.uri, "lsp: document closed");
                    self.docs.remove(&p.text_document.uri);
                }
            }
            other => tracing::debug!(method = other, "lsp: notification ignored"),
        }
        None
    }

    /// Answer `textDocument/formatting`: one whole-document edit, none when canonical, an
    /// error response when the buffer has syntax errors.
    fn formatting(&self, id: RequestId, params: Value) -> Response {
        let Ok(p) = serde_json::from_value::<DocumentFormattingParams>(params) else {
            return Response::new_err(
                id,
                ErrorCode::InvalidParams as i32,
                "bad formatting params".to_owned(),
            );
        };
        let Some(text) = self.docs.get(&p.text_document.uri) else {
            return Response::new_err(
                id,
                ErrorCode::InvalidParams as i32,
                "document is not open".to_owned(),
            );
        };
        match format_source(text) {
            Ok(formatted) if formatted == *text => {
                tracing::debug!("lsp: already canonical");
                Response::new_ok(id, Value::Null)
            }
            Ok(formatted) => {
                let range =
                    LspRange::new(lsp_types::Position::new(0, 0), end_position(text, self.enc));
                Response::new_ok(id, vec![TextEdit::new(range, formatted)])
            }
            Err(error) => {
                tracing::info!(%error, "lsp: formatting refused");
                Response::new_err(id, ErrorCode::RequestFailed as i32, error.to_string())
            }
        }
    }

    /// The text of `uri`: the open buffer, else the file on disk.
    fn text_of(&self, uri: &Url) -> String {
        self.docs.get(uri).cloned().unwrap_or_else(|| {
            uri.to_file_path()
                .ok()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default()
        })
    }

    /// The indexed entries of the file `uri` names (empty before the first check).
    fn entries_of(&self, uri: &Url) -> Vec<lsp_query::Entry> {
        let Ok(path) = uri.to_file_path() else {
            return Vec::new();
        };
        self.index
            .values()
            .find_map(|files| files.get(&path))
            .cloned()
            .unwrap_or_default()
    }

    /// Parse `params` as `P` and answer with `f`; bad params are an error response.
    fn query<P: serde::de::DeserializeOwned>(
        &self,
        id: RequestId,
        params: Value,
        f: fn(&Self, P) -> Value,
    ) -> Response {
        match serde_json::from_value::<P>(params) {
            Ok(p) => Response::new_ok(id, f(self, p)),
            Err(error) => {
                tracing::info!(%error, "lsp: bad query params");
                Response::new_err(id, ErrorCode::InvalidParams as i32, error.to_string())
            }
        }
    }

    /// `textDocument/hover`: the `why` chain of the declaration under the cursor.
    fn hover(&self, p: HoverParams) -> Value {
        let at = p.text_document_position_params;
        let text = self.text_of(&at.text_document.uri);
        let entries = self.entries_of(&at.text_document.uri);
        serde_json::to_value(lsp_query::hover(&entries, &text, at.position, self.enc))
            .unwrap_or(Value::Null)
    }

    /// `textDocument/codeLens`: one `impact` lens per declaration of the file.
    fn code_lens(&self, p: CodeLensParams) -> Value {
        let uri = p.text_document.uri;
        let lenses = lsp_query::lenses(&self.entries_of(&uri), &self.text_of(&uri), self.enc);
        serde_json::to_value(lenses).unwrap_or(Value::Null)
    }

    /// `textDocument/definition`: the declaration a canonical path or read name points at.
    fn definition(&self, p: GotoDefinitionParams) -> Value {
        let at = p.text_document_position_params;
        let uri = at.text_document.uri;
        let here = self.entries_of(&uri);
        let mut all = Vec::new();
        for files in self.index.values() {
            for (file, entries) in files {
                all.extend(entries.iter().map(|e| (file.as_path(), e)));
            }
        }
        let found = lsp_query::definition(
            &all,
            &here,
            &self.text_of(&uri),
            at.position,
            self.enc,
            |file| std::fs::read_to_string(file).unwrap_or_default(),
        );
        serde_json::to_value(found).unwrap_or(Value::Null)
    }

    /// Handle one request.
    fn request(&mut self, req: Request) {
        tracing::debug!(method = %req.method, "lsp: request");
        let response = if req.method == "shutdown" {
            self.shutdown = true;
            Response::new_ok(req.id, Value::Null)
        } else if self.shutdown {
            Response::new_err(
                req.id,
                ErrorCode::InvalidRequest as i32,
                "server is shut down".to_owned(),
            )
        } else if req.method == "textDocument/formatting" {
            self.formatting(req.id, req.params)
        } else if req.method == "textDocument/hover" {
            self.query(req.id, req.params, Self::hover)
        } else if req.method == "textDocument/codeLens" {
            self.query(req.id, req.params, Self::code_lens)
        } else if req.method == "textDocument/definition" {
            self.query(req.id, req.params, Self::definition)
        } else {
            tracing::info!(method = %req.method, "lsp: unsupported request");
            Response::new_err(
                req.id,
                ErrorCode::MethodNotFound as i32,
                format!("unsupported method `{}`", req.method),
            )
        };
        self.send(Message::Response(response));
    }

    /// The message loop; returns the process exit status.
    fn run(&mut self, results: &Receiver<PackageResult>) -> u8 {
        loop {
            select! {
                recv(self.conn.receiver) -> message => match message {
                    Err(_) => {
                        tracing::warn!("lsp: client disconnected without exit");
                        return EXIT_FAIL;
                    }
                    Ok(Message::Request(req)) => self.request(req),
                    Ok(Message::Notification(n)) => {
                        if let Some(code) = self.notification(n) {
                            return code;
                        }
                    }
                    Ok(Message::Response(r)) => tracing::debug!(id = ?r.id, "lsp: client response ignored"),
                },
                recv(results) -> result => {
                    if let Ok(result) = result {
                        self.apply_result(result);
                    }
                }
            }
        }
    }
}

/// Serve one LSP session on `conn`: handshake, then the message loop. Returns the exit status
/// (0 for `exit` after `shutdown`, 1 otherwise).
pub fn serve(conn: &Connection) -> u8 {
    let (id, params) = match conn.initialize_start() {
        Ok(started) => started,
        Err(error) => {
            tracing::error!(%error, "lsp: initialize failed");
            return EXIT_FAIL;
        }
    };
    let offered = serde_json::from_value::<InitializeParams>(params)
        .ok()
        .and_then(|p| p.capabilities.general)
        .and_then(|g| g.position_encodings)
        .unwrap_or_default();
    let enc = Encoding::negotiate(&offered);
    tracing::info!(?enc, "lsp: initialize");
    let result = json!({
        "capabilities": capabilities(enc),
        "serverInfo": { "name": "cadetta", "version": env!("CARGO_PKG_VERSION") },
    });
    if let Err(error) = conn.initialize_finish(id, result) {
        tracing::error!(%error, "lsp: initialized notification missing");
        return EXIT_FAIL;
    }
    let (tx, rx) = unbounded();
    let mut server = Server {
        conn,
        enc,
        docs: HashMap::new(),
        generation: HashMap::new(),
        published: HashMap::new(),
        index: HashMap::new(),
        results: tx,
        shutdown: false,
    };
    server.run(&rx)
}

/// Run `cadetta lsp` on the process's stdin and stdout; `out` is unused because stdout is the
/// protocol channel (the `lsp-server` writer owns it).
pub fn run_lsp(
    args: &[String],
    _globals: &Globals,
    _out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    if !args.is_empty() {
        tracing::error!(?args, "lsp: takes no arguments");
        let _ = writeln!(err, "error: `lsp` takes no arguments; usage: cadetta lsp");
        return EXIT_USAGE;
    }
    tracing::info!(config = CONFIG_FILE, "lsp: starting on stdio");
    let (conn, threads) = Connection::stdio();
    let code = serve(&conn);
    // Closing our sender ends the writer thread so it can be joined.
    drop(conn);
    if let Err(error) = threads.join() {
        tracing::warn!(%error, "lsp: io threads ended with an error");
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    // Text with a 2-byte char, a BMP 3-byte char and an astral char (2 UTF-16 units).
    const TEXT: &str = "a\u{e9}\u{4e2d}\u{1d11e}z\nsecond\n";

    // frob:tests crates/cadetta-cli/src/cmd/lsp.rs::to_lsp_position
    #[test]
    fn utf32_columns_are_characters() {
        // col 6 = before `z` (5 characters precede it).
        let p = to_lsp_position(TEXT, 1, 6, Encoding::Utf32);
        assert_eq!((p.line, p.character), (0, 5));
    }

    // frob:tests crates/cadetta-cli/src/cmd/lsp.rs::to_lsp_position
    #[test]
    fn utf16_counts_astral_as_two_units() {
        let p = to_lsp_position(TEXT, 1, 6, Encoding::Utf16);
        assert_eq!((p.line, p.character), (0, 6));
        // Before the astral char: a, e-acute, CJK are one unit each.
        assert_eq!(to_lsp_position(TEXT, 1, 4, Encoding::Utf16).character, 3);
        assert_eq!(to_lsp_position(TEXT, 2, 3, Encoding::Utf16).line, 1);
    }

    // frob:tests crates/cadetta-cli/src/cmd/lsp.rs::to_lsp_position
    #[test]
    fn column_past_line_end_counts_one_unit_each() {
        let p = to_lsp_position("ab\n", 1, 6, Encoding::Utf16);
        assert_eq!(p.character, 5);
    }

    // frob:tests crates/cadetta-cli/src/cmd/lsp.rs::end_position
    #[test]
    fn end_position_measures_last_line() {
        assert_eq!(
            end_position(TEXT, Encoding::Utf16),
            lsp_types::Position::new(2, 0)
        );
        assert_eq!(
            end_position("x\u{1d11e}", Encoding::Utf16),
            lsp_types::Position::new(0, 3)
        );
        assert_eq!(
            end_position("x\u{1d11e}", Encoding::Utf32),
            lsp_types::Position::new(0, 2)
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/lsp.rs::Encoding.negotiate
    #[test]
    fn utf32_is_chosen_only_when_offered() {
        let both = [PositionEncodingKind::UTF16, PositionEncodingKind::UTF32];
        assert_eq!(Encoding::negotiate(&both), Encoding::Utf32);
        assert_eq!(
            Encoding::negotiate(&[PositionEncodingKind::UTF16]),
            Encoding::Utf16
        );
        assert_eq!(Encoding::negotiate(&[]), Encoding::Utf16);
    }

    // frob:tests crates/cadetta-cli/src/cmd/lsp.rs::Encoding.kind
    #[test]
    fn kind_names_the_encoding() {
        assert_eq!(Encoding::Utf32.kind(), PositionEncodingKind::UTF32);
        assert_eq!(Encoding::Utf16.kind(), PositionEncodingKind::UTF16);
    }

    // frob:tests crates/cadetta-cli/src/cmd/lsp.rs::map_diagnostic
    #[test]
    fn mapping_follows_the_spec() {
        use cadetta_lint::diag::{Fix, Note, Position};
        let mut d = Diagnostic::new("C001", Severity::Warning, "bore too big");
        d.path = Some("acme::Axle.bore".to_owned());
        d.alt = Some("wheel=A".to_owned());
        d.notes.push(Note {
            message: "od is 20 mm".to_owned(),
            location: None,
        });
        d.fixes.push(Fix {
            title: "shrink the bore".to_owned(),
            edits: Vec::new(),
        });
        d.location = Some(Location::Source {
            file: "src/lib.cdt".to_owned(),
            start: Position { line: 1, col: 6 },
            end: Position { line: 1, col: 7 },
        });
        let m = map_diagnostic(&d, TEXT, Encoding::Utf16).unwrap();
        assert_eq!(m.range.start.character, 6);
        assert_eq!(m.range.end.character, 7);
        assert_eq!(m.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(m.code, Some(NumberOrString::String("C001".to_owned())));
        assert_eq!(m.source.as_deref(), Some("cadetta"));
        assert_eq!(
            m.message,
            "bore too big\nnote: od is 20 mm\nhelp: shrink the bore"
        );
        assert_eq!(
            m.data,
            Some(json!({"path": "acme::Axle.bore", "variant": "wheel=A"}))
        );
        assert!(m.code_description.is_none());

        d.location = Some(Location::Native {
            adapter: "a".to_owned(),
            file: "f".to_owned(),
            locator: "l".to_owned(),
        });
        assert!(map_diagnostic(&d, "", Encoding::Utf16).is_none());
        d.location = None;
        assert!(map_diagnostic(&d, "", Encoding::Utf16).is_none());
        d.location = Some(Location::Lock { line: 2 });
        let lock = map_diagnostic(&d, "one\ntwo\n", Encoding::Utf16).unwrap();
        assert_eq!((lock.range.start.line, lock.range.end.character), (1, 3));
    }

    // frob:tests crates/cadetta-cli/src/cmd/lsp.rs::syntax_diagnostics
    #[test]
    fn buffer_errors_are_e001_in_the_negotiated_encoding() {
        let text = "model \u{1d11e} {{{\n";
        let diags = syntax_diagnostics(text, Encoding::Utf16);
        assert!(!diags.is_empty());
        assert!(
            diags
                .iter()
                .all(|d| d.code == Some(NumberOrString::String("E001".into())))
        );
        assert!(syntax_diagnostics("model M {\n}\n", Encoding::Utf16).is_empty());
    }
}
