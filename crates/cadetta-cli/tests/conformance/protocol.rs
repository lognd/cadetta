//! frob:ticket 01M4VQXM5M08SGAEJQTQ6AKDVQ
//! Executor for the `binding` entry `protocol`: adapter-protocol conformance (adapters.md 5).
//!
//! The fixture JSON describes a scripted adapter (capabilities, replies or raw wire lines) and an
//! operation. The scenario runs on the real host layer ([`Client`], [`DriveSession`], [`sync`]
//! and the `availability` mapping) over an in-memory wire, then records what the host sent and
//! how it classified the outcome. Nothing is judged here: a check the host lacks leaves the
//! observation missing and the fixture failing (see `known-gaps.toml`).

use std::cell::{Cell, RefCell};
use std::fs;
use std::io::{self, BufRead, Cursor, Read, Write};
use std::path::Path;
use std::rc::Rc;

use cadetta_adapter::Capability;
use cadetta_adapter::availability::{self, EXIT_UNAVAILABLE, Verb};
use cadetta_adapter::builtin;
use cadetta_adapter::rpc::{
    ChangedParam, Client, EmitParams, OpenParams, ProcessClient, PushMode, PushParams, ReadParams,
    ReadSource, RegenParams, RegenState, RpcError, SessionParams,
};
use cadetta_adapter::session::{self, FilePush, SaveMode, SessionError};
use serde_json::{Value, json};

use crate::compare::Produced;
use crate::exec::Job;
use crate::run::{Facts, Scratch, run_cli};

/// Exit status of a run whose diagnostics include an error.
const EXIT_FAIL: i32 = 1;
/// Exit status of a usage or adapter failure.
const EXIT_USAGE: i32 = 2;

/// Run one `binding protocol` fixture and record what the host did.
pub fn run(job: &Job) -> Result<Facts, String> {
    let text =
        fs::read_to_string(job.fixture).map_err(|e| format!("{}: {e}", job.fixture.display()))?;
    let spec: Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", job.fixture.display()))?;
    if spec.get("initialize_result").is_some() {
        return Ok(features(&spec));
    }
    if spec.get("request").is_some() {
        return builtin_request(&spec);
    }
    match spec["op"].as_str() {
        Some("launch") => launch(&spec),
        Some(op) => scripted(job, &spec, op),
        None => refusals(&spec),
    }
}

/// A request to a built-in adapter: refused with a config error (exit 2) when the host's
/// capability set for it lacks the capability the request needs.
fn builtin_request(spec: &Value) -> Result<Facts, String> {
    let name = spec["adapter"].as_str().unwrap_or("?");
    let cap = match spec["request"].as_str() {
        Some("emit") => Capability::Emit,
        Some("read") => Capability::Read,
        Some("drive") => Capability::Drive,
        other => return Err(format!("adapter `{name}`: unknown request {other:?}")),
    };
    let mut facts = Facts::default();
    match builtin::require(name, cap) {
        Ok(()) => {}
        Err(e) => {
            tracing::debug!(adapter = name, error = %e, "built-in request refused");
            facts.messages.push(e.to_string());
            facts.exit = EXIT_USAGE;
        }
    }
    Ok(facts)
}

/// Scripted adapter output that reports how far the host has read.
struct Wire {
    inner: Cursor<Vec<u8>>,
    pos: Rc<Cell<usize>>,
}

impl Read for Wire {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.pos.set(self.pos.get() + n);
        Ok(n)
    }
}

impl BufRead for Wire {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        self.pos.set(self.pos.get() + amt);
        self.inner.consume(amt);
    }
}

/// Requests the host wrote, shared so they can be inspected after the client is consumed.
#[derive(Clone, Default)]
struct Sink(Rc<RefCell<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Wire and sink for a scripted adapter reply stream, plus the read-position handle.
fn pipes(text: String) -> (Wire, Sink, Rc<Cell<usize>>) {
    let pos = Rc::new(Cell::new(0));
    let wire = Wire {
        inner: Cursor::new(text.into_bytes()),
        pos: Rc::clone(&pos),
    };
    (wire, Sink::default(), pos)
}

/// One JSON-RPC result line.
pub(super) fn rpc(id: usize, result: &Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string()
}

/// The `initialize` result the scripted adapter answers with.
fn init_result(spec: &Value, op: &str) -> Value {
    let adapter = &spec["adapter"];
    let mut result = json!({
        "name": "mock",
        "version": adapter["version"],
        "capabilities": adapter["capabilities"],
        "native_units": "mm",
        "precision": 6,
        "platforms": ["linux"],
    });
    if op == "initialize"
        && let Some(extra) = spec["reply"].as_object()
    {
        for (k, v) in extra {
            result[k] = v.clone();
        }
    }
    result
}

/// Shorthand replies become protocol shapes: `diagnostics` defaults to `[]`, a regen
/// `{target: state}` map becomes a status list, a push `{locator: status}` map a status list.
fn normalize(op: &str, reply: &Value, locators: &[String]) -> Value {
    let mut reply = reply.clone();
    let Some(obj) = reply.as_object_mut() else {
        return reply;
    };
    if op == "regen"
        && let Some(map) = obj.get("targets").and_then(Value::as_object)
    {
        let list: Vec<Value> = map
            .iter()
            .map(|(t, s)| json!({"target": t, "status": s}))
            .collect();
        obj.insert("targets".into(), Value::Array(list));
    }
    if op == "push" && !obj.is_empty() && obj.keys().all(|k| locators.contains(k)) {
        let list: Vec<Value> = obj
            .iter()
            .map(|(l, s)| json!({"locator": l, "status": s, "old": null, "new": null}))
            .collect();
        *obj = serde_json::Map::new();
        obj.insert("statuses".into(), Value::Array(list));
    }
    let shaped = ["observed", "targets", "statuses"]
        .iter()
        .any(|k| obj.contains_key(*k));
    if shaped && !obj.contains_key("diagnostics") {
        obj.insert("diagnostics".into(), json!([]));
    }
    reply
}

/// Fixture wire lines: split on embedded newlines, ids shifted past `initialize` (which takes id 1),
/// replies completed like [`normalize`]. Returns the lines and the end offset (in the whole wire)
/// of the first id-bearing line, given `prefix_len` bytes already written.
fn wire_lines(spec: &Value, op: &str) -> Vec<String> {
    let raw: Vec<&str> = spec["wire"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .flat_map(str::lines)
        .collect();
    raw.into_iter()
        .map(|line| match serde_json::from_str::<Value>(line) {
            Ok(mut msg) if msg.is_object() => {
                if let Some(id) = msg.get("id").and_then(Value::as_u64) {
                    msg["id"] = json!(id + 1);
                }
                if let Some(result) = msg.get("result") {
                    let fixed = normalize(op, result, &[]);
                    msg["result"] = fixed;
                }
                msg.to_string()
            }
            _ => line.to_owned(),
        })
        .collect()
}

/// The `method` of every request the host wrote, in order.
fn methods(out: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(out)
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|m| m["method"].as_str().map(str::to_owned))
        .collect()
}

/// The params of the first written request named `method`.
fn sent_params(out: &[u8], method: &str) -> Option<Value> {
    String::from_utf8_lossy(out)
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|m| m["method"] == method)
        .map(|m| m["params"].clone())
}

/// Fold a transport failure into the facts: R008 for unavailability, else a protocol error.
fn fail(facts: &mut Facts, adapter: &str, err: &RpcError, init: bool) {
    if let Some(un) = availability::unavailable(adapter, err, Verb::Sync) {
        facts.produced.push(Produced {
            code: un.diagnostic.code.clone(),
            severity: "error".into(),
            path: None,
        });
        facts.messages.push(un.diagnostic.message);
        facts.exit = i32::from(un.exit.unwrap_or(EXIT_UNAVAILABLE));
        facts.observed.insert("protocol_error".into(), json!(false));
    } else {
        tracing::debug!(error = %err, "protocol error observed");
        facts.messages.push(err.to_string());
        // A bad `initialize` stops the run; a bad later reply is only reported as a protocol error.
        if init {
            facts.exit = EXIT_USAGE;
        }
        facts.observed.insert("protocol_error".into(), json!(true));
    }
}

/// Fold the diagnostics of a reply into the facts; an error-severity one makes the run fail.
fn take_diagnostics(facts: &mut Facts, diagnostics: &[cadetta_adapter::rpc::DiagRecord]) {
    for d in diagnostics {
        let code = d.0["code"].as_str().unwrap_or_default().to_owned();
        let severity = d.0["severity"].as_str().unwrap_or_default().to_owned();
        if severity == "error" {
            facts.exit = EXIT_FAIL;
        }
        facts.produced.push(Produced {
            code,
            severity,
            path: None,
        });
    }
}

/// `initialize_result` scenarios: does the host accept the declared features?
fn features(spec: &Value) -> Facts {
    let r = &spec["initialize_result"];
    let result = json!({
        "name": r["adapter"],
        "version": r["version"],
        "capabilities": r["capabilities"],
        "native_units": "mm",
        "precision": 6,
        "platforms": ["linux"],
        "features": r["features"],
    });
    let (input, out, _) = pipes(format!("{}\n", rpc(1, &result)));
    let mut facts = Facts::default();
    match Client::connect_for(input, out.clone(), None, "linux") {
        Ok(_) => {
            facts.observed.insert("protocol_error".into(), json!(false));
        }
        Err(e) => fail(&mut facts, "mock", &e, true),
    }
    facts
}

/// Calls made on a client with no session, to see which the host refuses before sending.
fn refusals(spec: &Value) -> Result<Facts, String> {
    let (input, out, _) = pipes(format!("{}\n", rpc(1, &init_result(spec, "calls"))));
    let mut facts = Facts::default();
    let mut refused = Vec::new();
    match Client::connect_for(input, out.clone(), None, "linux") {
        Err(e) => fail(&mut facts, "mock", &e, true),
        Ok(mut client) => {
            let session = || SessionParams {
                session: "h1".into(),
            };
            for call in spec["calls"].as_array().into_iter().flatten() {
                let name = call.as_str().unwrap_or_default();
                let res = match name {
                    "emit" => client
                        .emit(&EmitParams {
                            design: json!(null),
                            bindings: json!(null),
                            policy: json!(null),
                        })
                        .map(drop),
                    "open" => client
                        .open(&OpenParams {
                            files: vec!["a".into()],
                        })
                        .map(drop),
                    "push" => client
                        .push(&PushParams {
                            session: "h1".into(),
                            file: "a".into(),
                            params: Vec::new(),
                        })
                        .map(drop),
                    "regen" => client
                        .regen(&RegenParams {
                            session: "h1".into(),
                            targets: Vec::new(),
                        })
                        .map(drop),
                    "save" => client.save(&session()).map(drop),
                    "close" => client.close(&session()),
                    other => return Err(format!("unknown call `{other}`")),
                };
                if matches!(res, Err(RpcError::Unsupported(_))) {
                    refused.push(name.to_owned());
                }
            }
        }
    }
    let mut sent = methods(&out.0.borrow());
    sent.retain(|m| m != "initialize");
    tracing::debug!(?refused, ?sent, "refusals observed");
    facts.observed.insert("refused".into(), json!(refused));
    facts.observed.insert("sent".into(), json!(sent));
    Ok(facts)
}

/// The `launch` scenario: a package adapter started through `cadetta sync`, so the real
/// command resolution (package-relative, never searched on PATH) decides the outcome.
fn launch(spec: &Value) -> Result<Facts, String> {
    let scratch = Scratch::new()?;
    let root = &scratch.0;
    let command = spec["command"]
        .as_str()
        .ok_or("launch: missing `command`")?;
    let version = spec["adapter"]["version"].as_str().unwrap_or("1.0");
    put(
        root,
        "cadetta.toml",
        &format!(
            "[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n[[adapter]]\nname = \"freecad\"\nversion = \"{version}\"\ncommand = [\"{command}\"]\n"
        ),
    )?;
    put(
        root,
        "src/lib.cdt",
        "model CoverPlate {\n  thickness: Length = 6mm\n}\n",
    )?;
    if spec["in_package"].as_bool().unwrap_or(false) {
        let init = rpc(
            1,
            &json!({
                "name": "freecad", "version": version, "capabilities": ["emit", "read"],
                "native_units": "mm", "precision": 6, "platforms": ["linux"],
            }),
        );
        let script = format!(
            "#!/bin/sh\nread line\nprintf '%s\\n' '{init}'\nread line\nprintf '%s\\n' '{}'\nread line\nprintf '%s\\n' '{}'\n",
            rpc(2, &json!({"files": [], "diagnostics": []})),
            rpc(3, &Value::Null)
        );
        put_exec(root, command, &script)?;
    }
    let mut facts = run_cli(root, &["sync".to_owned()])?;
    if facts.exit == EXIT_USAGE && facts.stderr.contains("R008") && facts.produced.is_empty() {
        facts.produced.push(Produced {
            code: "R008".into(),
            severity: "error".into(),
            path: None,
        });
    }
    Ok(facts)
}

/// Write `text` under `root`.
pub(super) fn put(root: &Path, rel: &str, text: &str) -> Result<(), String> {
    let path = root.join(rel);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Write an executable script under `root`.
pub(super) fn put_exec(root: &Path, rel: &str, text: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    put(root, rel, text)?;
    let path = root.join(rel);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Locator names of a scenario: `locators`, else the single `locator.kind`, else `L`.
fn locators(spec: &Value) -> Vec<String> {
    if let Some(list) = spec["locators"].as_array() {
        return list
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect();
    }
    vec![spec["locator"]["kind"].as_str().unwrap_or("L").to_owned()]
}

/// One changed parameter for `locator`.
fn change(locator: &str, mode: PushMode, ref_to: Option<&str>) -> ChangedParam {
    ChangedParam {
        path: format!("p.{locator}"),
        value: json!(1),
        locator: locator.to_owned(),
        mode,
        ref_to: ref_to.map(str::to_owned),
        tolerance: None,
        kind: None,
    }
}

/// Scenarios with an `op` that run on a scripted in-memory adapter.
fn scripted(job: &Job, spec: &Value, op: &str) -> Result<Facts, String> {
    if spec["stderr_noise"].is_string() {
        return stderr_noise(spec);
    }
    let names = locators(spec);
    let files: Vec<String> = spec["files"].as_array().map_or_else(
        || {
            vec![
                spec["open_reports"]["file"]
                    .as_str()
                    .unwrap_or("a")
                    .to_owned(),
            ]
        },
        |l| {
            l.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        },
    );
    let mut lines = vec![rpc(1, &init_result(spec, op))];
    if spec["wire"].is_array() {
        lines.extend(wire_lines(spec, op));
    } else {
        if let Some(noise) = spec["stdout_noise"].as_str() {
            lines.push(noise.to_owned());
        }
        match op {
            "read" | "regen" => {
                let default = if op == "read" {
                    json!({"observed": []})
                } else {
                    json!({"targets": []})
                };
                let reply = if spec["reply"].is_null() {
                    &default
                } else {
                    &spec["reply"]
                };
                lines.push(rpc(2, &normalize(op, reply, &names)));
            }
            "push" => {
                let default = json!({"statuses": names.iter()
                    .map(|l| json!({"locator": l, "status": "pushed", "old": null, "new": null}))
                    .collect::<Vec<_>>(), "diagnostics": []});
                let reply = if spec["reply"].is_null() {
                    default
                } else {
                    normalize(op, &spec["reply"], &names)
                };
                lines.push(rpc(2, &reply));
            }
            "sync" => lines.extend(sync_script(spec, &files)),
            _ => {}
        }
    }
    let wire = format!("{}\n", lines.join("\n"));
    // End offset of the first id-bearing line after `initialize`: where a correct reader stops.
    let mut offset = 0;
    let mut reply_end = None;
    for (i, line) in wire.split_inclusive('\n').enumerate() {
        offset += line.len();
        let id = serde_json::from_str::<Value>(line)
            .ok()
            .and_then(|m| m.get("id").cloned());
        if i > 0 && reply_end.is_none() && id.is_some() {
            reply_end = id
                .and_then(|id| id.as_u64())
                .map(|id| (offset, id.saturating_sub(1)));
        }
    }
    let (input, out, pos) = pipes(wire);
    let mut facts = Facts::default();
    facts.observed.insert("protocol_error".into(), json!(false));
    let pin = spec["pinned_version"].as_str();
    let host = spec["host"].as_str().unwrap_or("linux");
    let connected = Client::connect_for(input, out.clone(), pin, host);
    match connected {
        Err(e) => fail(&mut facts, "mock", &e, true),
        Ok(client) => match op {
            "read" => {
                let source = match (
                    spec["source"]["session"].as_str(),
                    spec["source"]["files"].as_array(),
                ) {
                    (Some(h), _) => ReadSource::Session(h.to_owned()),
                    (None, Some(f)) => ReadSource::Files(
                        f.iter()
                            .filter_map(|v| v.as_str().map(str::to_owned))
                            .collect(),
                    ),
                    _ => ReadSource::Files(vec!["a".into()]),
                };
                run_read(&mut facts, client, &pos, &source, reply_end);
            }
            "push" => run_push(&mut facts, client, spec, &names),
            "regen" => run_regen(&mut facts, client),
            "sync" => run_sync(&mut facts, client, spec, &files),
            _ => {
                // `initialize`: connecting is the whole scenario; record the precision in force.
                let configured = spec["config_precision"]
                    .as_u64()
                    .and_then(|p| u32::try_from(p).ok());
                facts.observed.insert(
                    "effective_precision".into(),
                    json!(client.effective_precision(configured)),
                );
            }
        },
    }
    let written = out.0.borrow().clone();
    let mut sent = methods(&written);
    if let Some(params) = sent_params(&written, "read") {
        facts.observed.insert("request".into(), params);
        facts.observed.insert(
            "host_opens_session".into(),
            json!(sent.iter().any(|m| m == "open")),
        );
    }
    if op == "sync" {
        if job_wants_file_suffix(job) {
            suffix_pushes(&mut sent, &written);
        }
        facts.observed.insert("sent".into(), json!(sent));
    }
    Ok(facts)
}

/// Whether the block names pushes as `push:<file>` (the `drive-sync-order` convention).
fn job_wants_file_suffix(job: &Job) -> bool {
    job.block
        .extra
        .get("sent")
        .and_then(Value::as_array)
        .is_some_and(|l| {
            l.iter()
                .filter_map(Value::as_str)
                .any(|m| m.starts_with("push:"))
        })
}

/// Rename each `push` to `push:<file>` using the written params.
fn suffix_pushes(sent: &mut [String], out: &[u8]) {
    let mut files = String::from_utf8_lossy(out)
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|m| m["method"] == "push")
        .filter_map(|m| m["params"]["file"].as_str().map(str::to_owned))
        .collect::<Vec<_>>()
        .into_iter();
    for m in sent.iter_mut().filter(|m| *m == "push") {
        if let Some(f) = files.next() {
            *m = format!("push:{f}");
        }
    }
}

/// The adapter's answers to a drive sync: open, push per file, save, close, shutdown.
fn sync_script(spec: &Value, files: &[String]) -> Vec<String> {
    let dirty = spec["open_reports"]["dirty"].as_bool().unwrap_or(false);
    let status = spec["push_status"].as_str().unwrap_or("pushed");
    let opened: Vec<Value> = files
        .iter()
        .map(|f| json!({"path": f, "dirty": dirty, "readonly": false}))
        .collect();
    let mut lines = vec![rpc(
        2,
        &json!({"session": {"handle": "h1", "files": opened}, "diagnostics": []}),
    )];
    let mut id = 3;
    if dirty {
        lines.push(rpc(id, &Value::Null));
        lines.push(rpc(id + 1, &Value::Null));
        return lines;
    }
    for _ in files {
        lines.push(rpc(
            id,
            &json!({"statuses": [{"locator": "L", "status": status, "old": null, "new": null}], "diagnostics": []}),
        ));
        id += 1;
    }
    // The host saves only after a clean sync with `save: after-sync`; script the same shape.
    if spec["save"].as_str() != Some("never") && status != "failed" {
        lines.push(rpc(id, &json!({"files": [], "diagnostics": []})));
        id += 1;
    }
    lines.push(rpc(id, &Value::Null));
    lines.push(rpc(id + 1, &Value::Null));
    lines
}

/// `read`: classify the reply and, when the reader consumed an id-bearing line, which one.
fn run_read(
    facts: &mut Facts,
    mut client: Client<Wire, Sink>,
    pos: &Cell<usize>,
    source: &ReadSource,
    reply_end: Option<(usize, u64)>,
) {
    let result = client.read(&ReadParams {
        source: source.clone(),
    });
    match result {
        Ok(r) => {
            take_diagnostics(facts, &r.diagnostics);
            if let Some((end, id)) = reply_end {
                facts.observed.insert("result_id".into(), json!(id));
                let stopped_early = pos.get() < end;
                facts
                    .observed
                    .insert("treated_as_reply".into(), json!(stopped_early));
            }
        }
        Err(e) => fail(facts, "mock", &e, false),
    }
}

/// `push`: one request with the fixture's locator; observe the item it carried and the statuses.
fn run_push<R: std::io::BufRead, W: Write>(
    facts: &mut Facts,
    mut client: Client<R, W>,
    spec: &Value,
    names: &[String],
) {
    let mode = match spec["locator"]["mode"].as_str() {
        Some("ref") => PushMode::Ref,
        Some("frame") => PushMode::Frame,
        _ => PushMode::Value,
    };
    let ref_to = spec["locator"]["ref_to"].as_str();
    let params: Vec<ChangedParam> = names.iter().map(|n| change(n, mode, ref_to)).collect();
    let item = serde_json::to_value(&params[0]).unwrap_or(Value::Null);
    match client.push(&PushParams {
        session: "h1".into(),
        file: "a".into(),
        params,
    }) {
        Ok(r) => {
            let mut item_view = serde_json::Map::new();
            for k in ["mode", "ref_to"] {
                if let Some(v) = item.get(k) {
                    item_view.insert(k.into(), v.clone());
                }
            }
            facts
                .observed
                .insert("request_item".into(), Value::Object(item_view));
            let statuses: serde_json::Map<String, Value> = r
                .statuses
                .iter()
                .map(|s| {
                    (
                        s.locator.clone(),
                        serde_json::to_value(s.status).unwrap_or(Value::Null),
                    )
                })
                .collect();
            facts
                .observed
                .insert("statuses".into(), Value::Object(statuses));
            take_diagnostics(facts, &r.diagnostics);
        }
        Err(e) => {
            if matches!(e, RpcError::Nonconforming(_)) {
                // A push result without per-locator statuses (summary only) is nonconforming.
                facts
                    .observed
                    .insert("reason".into(), json!("summary-only push result"));
            }
            fail(facts, "mock", &e, false);
        }
    }
}

/// `regen`: a failed target is the host's R004.
fn run_regen<R: std::io::BufRead, W: Write>(facts: &mut Facts, mut client: Client<R, W>) {
    match client.regen(&RegenParams {
        session: "h1".into(),
        targets: vec!["t".into()],
    }) {
        Ok(r) => {
            take_diagnostics(facts, &r.diagnostics);
            if r.targets.iter().any(|t| t.status == RegenState::Failed) {
                facts.exit = EXIT_FAIL;
                facts.produced.push(Produced {
                    code: "R004".into(),
                    severity: "error".into(),
                    path: None,
                });
            }
        }
        Err(e) => fail(facts, "mock", &e, false),
    }
}

/// `sync`: the full drive session over every file.
fn run_sync<R: std::io::BufRead, W: Write>(
    facts: &mut Facts,
    client: Client<R, W>,
    spec: &Value,
    files: &[String],
) {
    let mode = match spec["save"].as_str() {
        Some("never") => SaveMode::Never,
        _ => SaveMode::AfterSync,
    };
    let pushes: Vec<FilePush> = files
        .iter()
        .map(|f| FilePush {
            file: f.clone(),
            params: vec![change("L", PushMode::Value, None)],
        })
        .collect();
    match session::sync(client, pushes, mode) {
        Ok(report) => {
            tracing::debug!(saved = report.saved.is_some(), "sync completed");
        }
        Err(SessionError::Rpc(e)) => fail(facts, "mock", &e, false),
        Err(e) => {
            let code = e.code().unwrap_or("R008").to_owned();
            facts.messages.push(e.to_string());
            facts.exit = EXIT_FAIL;
            facts.produced.push(Produced {
                code,
                severity: "error".into(),
                path: None,
            });
        }
    }
}

/// A real adapter process that writes to stderr: the host must keep stdout protocol-only.
fn stderr_noise(spec: &Value) -> Result<Facts, String> {
    let scratch = Scratch::new()?;
    let noise = spec["stderr_noise"].as_str().unwrap_or("noise");
    let init = rpc(1, &init_result(spec, "read"));
    let read = rpc(2, &json!({"observed": [], "diagnostics": []}));
    let script = format!(
        "#!/bin/sh\necho '{noise}' >&2\nread line\nprintf '%s\\n' '{init}'\nread line\nprintf '%s\\n' '{read}'\nread line\n"
    );
    put_exec(&scratch.0, "adapter.sh", &script)?;
    let mut facts = Facts::default();
    let mut client = ProcessClient::spawn(&scratch.0.join("adapter.sh"), &[], None)
        .map_err(|e| format!("spawn: {e}"))?;
    match client.read(&ReadParams {
        source: ReadSource::Files(vec!["a".into()]),
    }) {
        Ok(_) => {
            facts
                .observed
                .insert("stderr_passed_through".into(), json!(true));
            facts.observed.insert("protocol_error".into(), json!(false));
        }
        Err(e) => fail(&mut facts, "mock", &e, false),
    }
    drop(client);
    Ok(facts)
}
