//! frob:ticket 01M4SWN8MPH5ARCRJG4QB9ANVQ
//! In-process `cadetta lsp` session tests over in-memory streams (config-cli.md 3.8).

use std::path::PathBuf;
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use serde_json::{Value, json};

const WAIT: Duration = Duration::from_secs(60);

struct Client {
    conn: Connection,
    next: i32,
    server: Option<std::thread::JoinHandle<u8>>,
}

impl Client {
    fn start(encodings: Value) -> (Self, Value) {
        let (server_conn, conn) = Connection::memory();
        let server = std::thread::spawn(move || cadetta::cmd::lsp::serve(&server_conn));
        let mut c = Client {
            conn,
            next: 0,
            server: Some(server),
        };
        let init = c.request(
            "initialize",
            json!({"processId": null, "rootUri": null,
                   "capabilities": {"general": {"positionEncodings": encodings}}}),
        );
        c.notify("initialized", json!({}));
        (c, init)
    }

    fn send(&self, m: Message) {
        self.conn.sender.send(m).unwrap();
    }

    fn notify(&self, method: &str, params: Value) {
        self.send(Message::Notification(Notification::new(
            method.to_owned(),
            params,
        )));
    }

    /// Send a request and return the whole response as JSON (`result` or `error`).
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = RequestId::from(self.next);
        self.send(Message::Request(Request::new(
            id.clone(),
            method.to_owned(),
            params,
        )));
        loop {
            match self.conn.receiver.recv_timeout(WAIT).expect("response") {
                Message::Response(r) if r.id == id => return serde_json::to_value(r).unwrap(),
                _ => {}
            }
        }
    }

    /// Next publishDiagnostics notification.
    fn next_publish(&self) -> Value {
        loop {
            match self.conn.receiver.recv_timeout(WAIT).expect("publish") {
                Message::Notification(n) if n.method == "textDocument/publishDiagnostics" => {
                    return n.params;
                }
                _ => {}
            }
        }
    }

    fn finish(mut self, exit_first: bool) -> u8 {
        if !exit_first {
            self.request("shutdown", Value::Null);
        }
        self.notify("exit", Value::Null);
        self.server.take().unwrap().join().unwrap()
    }
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cadetta-lsp-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    dir
}

fn open(c: &Client, uri: &str, text: &str) {
    c.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": uri, "languageId": "cadetta", "version": 1, "text": text}}),
    );
}

// frob:tests crates/cadetta-cli/tests/lsp.rs::capabilities_and_exit_codes
#[test]
fn capabilities_and_exit_codes() {
    let (c, init) = Client::start(json!(["utf-16", "utf-32"]));
    let caps = &init["result"]["capabilities"];
    assert_eq!(caps["positionEncoding"], "utf-32");
    assert_eq!(caps["documentFormattingProvider"], true);
    assert_eq!(caps["textDocumentSync"]["openClose"], true);
    assert_eq!(caps["textDocumentSync"]["change"], 1);
    assert_eq!(caps.as_object().unwrap().len(), 3);
    assert_eq!(c.finish(false), 0);

    let (c, init) = Client::start(json!(["utf-16"]));
    assert_eq!(init["result"]["capabilities"]["positionEncoding"], "utf-16");
    assert_eq!(c.finish(true), 1);
}

// frob:tests crates/cadetta-cli/tests/lsp.rs::syntax_error_then_format_refused
#[test]
fn syntax_error_then_format_refused() {
    let dir = temp("syntax");
    let uri = format!("file://{}/loose.cdt", dir.display());
    let (mut c, _) = Client::start(json!(["utf-16"]));
    open(&c, &uri, "model {{{\n");
    let p = c.next_publish();
    assert_eq!(p["uri"], uri);
    let d = &p["diagnostics"][0];
    assert_eq!(d["code"], "E001");
    assert_eq!(d["severity"], 1);
    assert_eq!(d["source"], "cadetta");
    let r = c.request(
        "textDocument/formatting",
        json!({"textDocument": {"uri": uri}, "options": {"tabSize": 2, "insertSpaces": true}}),
    );
    assert_eq!(r["error"]["code"], -32803);
    assert_eq!(c.finish(false), 0);
}

// frob:tests crates/cadetta-cli/tests/lsp.rs::format_returns_one_whole_document_edit
#[test]
fn format_returns_one_whole_document_edit() {
    let dir = temp("format");
    let uri = format!("file://{}/loose.cdt", dir.display());
    let (mut c, _) = Client::start(json!(["utf-16"]));
    let messy = "model   M {\n  a: Length   = 1mm\n}\n";
    open(&c, &uri, messy);
    c.next_publish();
    let params =
        json!({"textDocument": {"uri": uri}, "options": {"tabSize": 2, "insertSpaces": true}});
    let r = c.request("textDocument/formatting", params.clone());
    let edits = r["result"].as_array().expect("edits");
    assert_eq!(edits.len(), 1);
    assert_eq!(
        edits[0]["range"]["start"],
        json!({"line": 0, "character": 0})
    );
    assert_eq!(edits[0]["range"]["end"], json!({"line": 3, "character": 0}));
    let formatted = edits[0]["newText"].as_str().unwrap().to_owned();
    // A canonical buffer yields no edit.
    c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": formatted}]}),
    );
    c.next_publish();
    let r = c.request("textDocument/formatting", params);
    assert!(r["result"].is_null());
    assert_eq!(c.finish(false), 0);
}

// frob:tests crates/cadetta-cli/tests/lsp.rs::save_publishes_package_lint_and_clears
#[test]
fn save_publishes_package_lint_and_clears() {
    let dir = temp("pkg");
    std::fs::write(
        dir.join("cadetta.toml"),
        "[package]\nname = \"acme.axle\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let bad = "pub model Axle {\n  od: Length = 20mm\n  let bore: Length = 28mm\n  check bore_ok: bore < od \"bore fits\"\n}\n";
    let file = dir.join("src/lib.cdt");
    std::fs::write(&file, bad).unwrap();
    let uri = format!("file://{}", file.display());
    let (c, _) = Client::start(json!(["utf-16"]));
    open(&c, &uri, bad);
    // Immediate buffer publish (no E001), then the package result.
    assert_eq!(c.next_publish()["diagnostics"].as_array().unwrap().len(), 0);
    let p = c.next_publish();
    assert_eq!(p["uri"], uri);
    let codes: Vec<&str> = p["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"C001"), "{codes:?}");

    // Fix on disk and save: the list clears.
    let good = bad.replace("28mm", "8mm");
    std::fs::write(&file, &good).unwrap();
    c.notify(
        "textDocument/didSave",
        json!({"textDocument": {"uri": uri}}),
    );
    let p = c.next_publish();
    assert_eq!(p["uri"], uri);
    assert!(p["diagnostics"].as_array().unwrap().is_empty(), "{p}");
    assert_eq!(c.finish(false), 0);
}
