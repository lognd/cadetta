//! The JSON-RPC 2.0 client that drives one external adapter over stdio.

use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::types::*;

/// Failure of a protocol call; adapter-side refusals carry their JSON-RPC error.
#[derive(Debug)]
pub enum RpcError {
    /// The adapter process could not be started.
    Spawn(std::io::Error),
    /// Reading or writing the pipes failed.
    Io(std::io::Error),
    /// The peer sent something that is not a valid protocol message.
    Protocol(String),
    /// The adapter answered with a JSON-RPC error object.
    Remote {
        /// JSON-RPC error code.
        code: i64,
        /// Error message.
        message: String,
        /// Optional structured data.
        data: Option<Value>,
    },
    /// `initialize` reported a version other than the configured one (R008, exit 2).
    VersionMismatch {
        /// Version required by `cadetta.toml`.
        expected: String,
        /// Version the adapter reported.
        found: String,
    },
    /// The method needs a capability the adapter did not declare.
    Unsupported(&'static str),
}

impl RpcError {
    /// The diagnostic code this error maps to, if any.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::VersionMismatch { .. } => Some(R008),
            _ => None,
        }
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(e) => write!(f, "cannot start adapter: {e}"),
            Self::Io(e) => write!(f, "adapter i/o failed: {e}"),
            Self::Protocol(m) => write!(f, "adapter protocol error: {m}"),
            Self::Remote { code, message, .. } => write!(f, "adapter error {code}: {message}"),
            Self::VersionMismatch { expected, found } => write!(
                f,
                "{R008}: adapter version {found} does not match configured {expected}"
            ),
            Self::Unsupported(m) => write!(f, "adapter does not declare the capability for `{m}`"),
        }
    }
}

impl std::error::Error for RpcError {}

/// A client bound to a pipe pair, already initialized.
pub struct Client<R: BufRead, W: Write> {
    reader: R,
    writer: W,
    next_id: u64,
    info: InitializeResult,
    child: Option<Child>,
    /// Set once a timed-out call killed the adapter; `shutdown` then only reaps it.
    killed: bool,
}

/// The adapter's stdout, pumped by a thread so a read can give up after a timeout.
///
/// Reading the pipe directly would block for as long as any grandchild of the adapter holds
/// it open, even after the adapter itself was killed.
// frob:ticket 01M4YG7D22XEEK9M3X678B88BT
pub struct ChannelReader {
    rx: Receiver<std::io::Result<Vec<u8>>>,
    chunk: Vec<u8>,
    pos: usize,
    timeout: Option<Duration>,
}

impl ChannelReader {
    /// Starts the pump thread over `source`; each read waits at most `timeout` for data.
    fn new<S: Read + Send + 'static>(mut source: S, timeout: Option<Duration>) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = [0_u8; 8192];
            loop {
                let msg = match source.read(&mut buf) {
                    Ok(0) => return,
                    Ok(n) => Ok(buf[..n].to_vec()),
                    Err(e) => Err(e),
                };
                let failed = msg.is_err();
                if tx.send(msg).is_err() || failed {
                    return;
                }
            }
        });
        Self {
            rx,
            chunk: Vec::new(),
            pos: 0,
            timeout,
        }
    }
}

impl Read for ChannelReader {
    /// Copies pumped bytes; EOF when the pump ends, `TimedOut` when none arrive in time.
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.chunk.len() {
            let next = match self.timeout {
                Some(t) => match self.rx.recv_timeout(t) {
                    Ok(m) => m,
                    Err(RecvTimeoutError::Timeout) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            format!("no answer from the adapter within {t:?}"),
                        ));
                    }
                    Err(RecvTimeoutError::Disconnected) => return Ok(0),
                },
                None => match self.rx.recv() {
                    Ok(m) => m,
                    Err(_) => return Ok(0),
                },
            };
            self.chunk = next?;
            self.pos = 0;
        }
        let n = out.len().min(self.chunk.len() - self.pos);
        out[..n].copy_from_slice(&self.chunk[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// A client talking to a spawned adapter process.
pub type ProcessClient = Client<BufReader<ChannelReader>, ChildStdin>;

impl ProcessClient {
    /// Starts `command` (no PATH search) and runs `initialize`; stderr is inherited.
    ///
    /// No call is time-bounded; see [`ProcessClient::spawn_with_timeout`].
    // frob:ticket 01M4YG7D22XEEK9M3X678B88BT
    pub fn spawn(
        command: &Path,
        args: &[String],
        expected_version: Option<&str>,
    ) -> Result<Self, RpcError> {
        Self::spawn_with_timeout(command, args, expected_version, None)
    }

    /// Like [`ProcessClient::spawn`], but a call (including `initialize`) with no answer within
    /// `timeout` kills the adapter and fails with an `Io` error of kind `TimedOut`.
    // frob:ticket 01M4YG7D22XEEK9M3X678B88BT
    pub fn spawn_with_timeout(
        command: &Path,
        args: &[String],
        expected_version: Option<&str>,
        timeout: Option<Duration>,
    ) -> Result<Self, RpcError> {
        tracing::info!(command = %command.display(), ?timeout, "spawning adapter");
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| {
                tracing::error!(error = %e, "adapter spawn failed");
                RpcError::Spawn(e)
            })?;
        let stdin = child.stdin.take().expect("stdin was piped");
        let stdout = child.stdout.take().expect("stdout was piped");
        let reader = BufReader::new(ChannelReader::new(stdout, timeout));
        // Hand the child to the client first so a failed `initialize` drops (kills) it.
        Self::connect_with(reader, stdin, expected_version, Some(child))
    }
}

impl<R: BufRead, W: Write> Client<R, W> {
    /// Runs `initialize` over the pipes and refuses a version mismatch (R008).
    pub fn connect(reader: R, writer: W, expected_version: Option<&str>) -> Result<Self, RpcError> {
        Self::connect_with(reader, writer, expected_version, None)
    }

    fn connect_with(
        reader: R,
        writer: W,
        expected_version: Option<&str>,
        child: Option<Child>,
    ) -> Result<Self, RpcError> {
        let mut client = Self {
            reader,
            writer,
            next_id: 1,
            info: InitializeResult {
                name: String::new(),
                version: String::new(),
                capabilities: Vec::new(),
                native_units: String::new(),
                precision: 0,
                platforms: Vec::new(),
                features: Vec::new(),
            },
            child,
            killed: false,
        };
        let params = InitializeParams {
            protocol: PROTOCOL.to_string(),
            client: ClientInfo {
                name: "cadetta".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        let info: InitializeResult = client.call("initialize", &params)?;
        if let Some(want) = expected_version
            && want != info.version
        {
            tracing::error!(expected = want, found = %info.version, "adapter version mismatch (R008)");
            return Err(RpcError::VersionMismatch {
                expected: want.to_string(),
                found: info.version,
            });
        }
        tracing::info!(name = %info.name, version = %info.version, "adapter initialized");
        client.info = info;
        Ok(client)
    }

    /// What the adapter reported at `initialize`.
    pub fn info(&self) -> &InitializeResult {
        &self.info
    }

    fn has(&self, cap: CapabilityName) -> bool {
        self.info.capabilities.contains(&cap)
    }

    fn require(&self, cap: CapabilityName, method: &'static str) -> Result<(), RpcError> {
        if self.has(cap) {
            Ok(())
        } else {
            tracing::warn!(method, ?cap, "capability not declared");
            Err(RpcError::Unsupported(method))
        }
    }

    fn call<P: Serialize, T: DeserializeOwned>(
        &mut self,
        method: &str,
        params: &P,
    ) -> Result<T, RpcError> {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        tracing::debug!(method, id, "rpc request");
        let mut line = serde_json::to_string(&req).expect("a request is always serializable");
        line.push('\n');
        self.writer
            .write_all(line.as_bytes())
            .and_then(|()| self.writer.flush())
            .map_err(RpcError::Io)?;
        loop {
            let mut buf = String::new();
            let read = self.reader.read_line(&mut buf).map_err(|e| {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    tracing::error!(method, error = %e, "adapter did not answer; killing it");
                    self.killed = true;
                    if let Some(child) = self.child.as_mut() {
                        let _ = child.kill();
                    }
                }
                RpcError::Io(e)
            })?;
            if read == 0 {
                return Err(RpcError::Protocol("adapter closed stdout".into()));
            }
            if buf.trim().is_empty() {
                continue;
            }
            let msg: Value = serde_json::from_str(&buf)
                .map_err(|e| RpcError::Protocol(format!("invalid JSON: {e}")))?;
            if msg.get("id").is_none() {
                tracing::debug!(method, "ignoring adapter notification");
                continue;
            }
            if msg.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
                return Err(RpcError::Protocol("missing jsonrpc 2.0 marker".into()));
            }
            if msg["id"] != json!(id) {
                return Err(RpcError::Protocol(format!(
                    "response id {} does not match request {id}",
                    msg["id"]
                )));
            }
            if let Some(err) = msg.get("error") {
                let code = err.get("code").and_then(Value::as_i64).unwrap_or(0);
                let message = err
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                tracing::warn!(method, code, %message, "adapter returned an error");
                return Err(RpcError::Remote {
                    code,
                    message,
                    data: err.get("data").cloned(),
                });
            }
            let result = msg.get("result").cloned().unwrap_or(Value::Null);
            return serde_json::from_value(result).map_err(|e| {
                tracing::error!(method, error = %e, "result does not match the protocol schema");
                RpcError::Protocol(format!("`{method}` result: {e}"))
            });
        }
    }

    /// Asks the adapter to write its files; requires `emit`.
    pub fn emit(&mut self, params: &EmitParams) -> Result<EmitResult, RpcError> {
        self.require(CapabilityName::Emit, "emit")?;
        self.call("emit", params)
    }

    /// Opens documents in the live tool; requires `drive`.
    pub fn open(&mut self, params: &OpenParams) -> Result<OpenResult, RpcError> {
        self.require(CapabilityName::Drive, "open")?;
        self.call("open", params)
    }

    /// Reads observed IR from files or a session; requires `read` or `drive`.
    pub fn read(&mut self, params: &ReadParams) -> Result<ReadResult, RpcError> {
        if !self.has(CapabilityName::Drive) {
            self.require(CapabilityName::Read, "read")?;
        }
        self.call("read", params)
    }

    /// Pushes changed parameters into a session; requires `drive`.
    pub fn push(&mut self, params: &PushParams) -> Result<PushResult, RpcError> {
        self.require(CapabilityName::Drive, "push")?;
        // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
        if !self.has_feature(Feature::Tolerances)
            && params.params.iter().any(|p| p.tolerance.is_some())
        {
            tracing::info!("adapter lacks the tolerances feature; tolerances not sent");
            let mut stripped = params.clone();
            for p in &mut stripped.params {
                p.tolerance = None;
            }
            return self.call("push", &stripped);
        }
        self.call("push", params)
    }

    /// Whether the adapter reported `feature` at `initialize`.
    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    pub fn has_feature(&self, feature: Feature) -> bool {
        self.info.features.contains(&feature)
    }

    /// Regenerates targets in a session; requires `drive`.
    pub fn regen(&mut self, params: &RegenParams) -> Result<RegenResult, RpcError> {
        self.require(CapabilityName::Drive, "regen")?;
        self.call("regen", params)
    }

    /// Saves the documents of a session; requires `drive`.
    pub fn save(&mut self, params: &SessionParams) -> Result<SaveResult, RpcError> {
        self.require(CapabilityName::Drive, "save")?;
        self.call("save", params)
    }

    /// Closes a session; requires `drive`.
    pub fn close(&mut self, params: &SessionParams) -> Result<(), RpcError> {
        self.require(CapabilityName::Drive, "close")?;
        self.call("close", params)
    }

    /// Sends `shutdown` and reaps the process, if one was spawned.
    pub fn shutdown(mut self) -> Result<(), RpcError> {
        let res: Result<(), RpcError> = if self.killed {
            tracing::info!("adapter was killed after a timeout; skipping shutdown");
            Ok(())
        } else {
            self.call("shutdown", &json!({}))
        };
        if let Some(mut child) = self.child.take() {
            match child.wait() {
                Ok(status) => tracing::info!(%status, "adapter exited"),
                Err(e) => tracing::warn!(error = %e, "waiting for adapter failed"),
            }
        }
        res
    }
}

impl<R: BufRead, W: Write> Drop for Client<R, W> {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            tracing::warn!("adapter dropped without shutdown; killing it");
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn init(version: &str, caps: &[&str]) -> Value {
        json!({"name": "mock", "version": version, "capabilities": caps,
               "native_units": "mm", "precision": 6, "platforms": ["linux"]})
    }

    fn diag() -> Value {
        json!({"schema": "cadetta.diag/1", "code": "R004", "severity": "error",
               "message": "feature failed"})
    }

    /// Builds the scripted stdout of a mock adapter: one response per result.
    fn script(results: &[Value]) -> Cursor<Vec<u8>> {
        let mut out = String::new();
        for (i, r) in results.iter().enumerate() {
            out.push_str(&json!({"jsonrpc": "2.0", "id": i + 1, "result": r}).to_string());
            out.push('\n');
        }
        Cursor::new(out.into_bytes())
    }

    fn init_features(features: Value) -> Value {
        let mut v = init("1.0.0", &["drive"]);
        v["features"] = features;
        v
    }

    fn tol_push() -> PushParams {
        PushParams {
            session: "s1".into(),
            file: "a.FCStd".into(),
            params: vec![ChangedParam {
                path: "p.x".into(),
                value: json!("12 mm"),
                locator: "D1@Sketch".into(),
                mode: PushMode::Value,
                ref_to: None,
                tolerance: Some(DimTolerance::Symmetric { t: "0.1 mm".into() }),
            }],
        }
    }

    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    // frob:tests initialize_features_accepted_and_validated
    #[test]
    fn initialize_features_accepted_and_validated() {
        let c = Client::connect(
            script(&[init_features(json!(["frames", "tolerances"]))]),
            Vec::new(),
            None,
        )
        .unwrap();
        assert!(c.has_feature(Feature::Tolerances));
        assert!(c.has_feature(Feature::Frames));
        assert!(!c.has_feature(Feature::RenameGlobals));
        let bare = Client::connect(script(&[init("1.0.0", &["drive"])]), Vec::new(), None).unwrap();
        assert!(!bare.has_feature(Feature::Tolerances));
        assert!(
            Client::connect(script(&[init_features(json!(["bogus"]))]), Vec::new(), None).is_err()
        );
    }

    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    // frob:tests tolerance_sent_only_with_feature
    #[test]
    fn tolerance_sent_only_with_feature() {
        let pushed = json!({"statuses": [], "diagnostics": []});
        let sent = |features: Value| {
            let mut out = Vec::new();
            let mut c = Client::connect(
                script(&[init_features(features), pushed.clone()]),
                &mut out,
                None,
            )
            .unwrap();
            c.push(&tol_push()).unwrap();
            drop(c);
            String::from_utf8(out).unwrap().contains("\"tolerance\"")
        };
        assert!(sent(json!(["tolerances"])));
        assert!(!sent(json!([])));
    }

    // frob:ticket 01M4SPWRS0CDJT4GYC92AB1HCD
    // frob:tests crates/cadetta-adapter/src/rpc/client.rs::Client kind=unit
    #[test]
    fn initialize_returns_adapter_facts() {
        let c = Client::connect(
            script(&[init("1.2.0", &["emit", "read"])]),
            Vec::new(),
            None,
        )
        .unwrap();
        let i = c.info();
        assert_eq!(i.name, "mock");
        assert_eq!(i.version, "1.2.0");
        assert_eq!(i.capabilities, [CapabilityName::Emit, CapabilityName::Read]);
        assert_eq!(i.native_units, "mm");
        assert_eq!(i.precision, 6);
        assert_eq!(i.platforms, ["linux"]);
    }

    // frob:ticket 01M4SPWRS0CDJT4GYC92AB1HCD
    // frob:tests crates/cadetta-adapter/src/rpc/client.rs::Client kind=unit
    #[test]
    fn full_sequence_results_match_schema_types() {
        let results = [
            init("1.0.0", &["emit", "read", "drive"]),
            json!({"files": [{"path": "a.txt", "digest": "blake3:00"}], "diagnostics": [diag()]}),
            json!({"session": {"handle": "s1",
                   "files": [{"path": "a.FCStd", "dirty": false, "readonly": false}]},
                   "diagnostics": []}),
            json!({"observed": {"x": 1}, "diagnostics": []}),
            json!({"statuses": [{"locator": "Sketch.d1", "status": "pushed",
                   "old": "1 mm", "new": "2 mm"}], "diagnostics": []}),
            json!({"targets": [{"target": "Part", "status": "failed"}],
                   "diagnostics": [diag()]}),
            json!({"files": [{"path": "a.FCStd", "digest": "blake3:01"}], "diagnostics": []}),
            Value::Null,
            Value::Null,
        ];
        let mut out = Vec::new();
        let mut c = Client::connect(script(&results), &mut out, Some("1.0.0")).unwrap();
        let e = c
            .emit(&EmitParams {
                design: json!({}),
                bindings: json!({}),
                policy: json!({}),
            })
            .unwrap();
        assert_eq!(e.files[0].path, "a.txt");
        let o = c
            .open(&OpenParams {
                files: vec!["a.FCStd".into()],
            })
            .unwrap();
        assert_eq!(o.session.handle, "s1");
        let r = c
            .read(&ReadParams {
                source: ReadSource::Session("s1".into()),
            })
            .unwrap();
        assert_eq!(r.observed["x"], 1);
        let p = c
            .push(&PushParams {
                session: "s1".into(),
                file: "a.FCStd".into(),
                params: vec![ChangedParam {
                    path: "p.x".into(),
                    value: json!("2 mm"),
                    locator: "Sketch.d1".into(),
                    mode: PushMode::Value,
                    ref_to: None,
                    tolerance: None,
                }],
            })
            .unwrap();
        assert_eq!(p.statuses[0].status, PushStatus::Pushed);
        let g = c
            .regen(&RegenParams {
                session: "s1".into(),
                targets: vec!["Part".into()],
            })
            .unwrap();
        assert_eq!(g.targets[0].status, RegenState::Failed);
        let s = SessionParams {
            session: "s1".into(),
        };
        assert_eq!(c.save(&s).unwrap().files.len(), 1);
        c.close(&s).unwrap();
        c.shutdown().unwrap();
        let sent = String::from_utf8(out).unwrap();
        let methods: Vec<String> = sent
            .lines()
            .map(|l| {
                serde_json::from_str::<Value>(l).unwrap()["method"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(
            methods,
            [
                "initialize",
                "emit",
                "open",
                "read",
                "push",
                "regen",
                "save",
                "close",
                "shutdown"
            ]
        );
    }

    // frob:ticket 01M4SPWRS0CDJT4GYC92AB1HCD
    // frob:tests crates/cadetta-adapter/src/rpc/client.rs::Client kind=unit
    #[test]
    fn malformed_result_is_a_protocol_error() {
        let results = [
            init("1.0.0", &["emit"]),
            json!({"files": [], "diagnostics": [{"code": "R004"}], "extra": 1}),
        ];
        let mut c = Client::connect(script(&results), Vec::new(), None).unwrap();
        let r = c.emit(&EmitParams {
            design: json!({}),
            bindings: json!({}),
            policy: json!({}),
        });
        assert!(matches!(r, Err(RpcError::Protocol(_))));
    }

    // frob:ticket 01M4SPWRS0CDJT4GYC92AB1HCD
    // frob:tests crates/cadetta-adapter/src/rpc/client.rs::Client kind=unit
    #[test]
    fn version_mismatch_is_refused_r008() {
        let r = Client::connect(
            script(&[init("1.0.0", &["emit"])]),
            Vec::new(),
            Some("2.0.0"),
        );
        match r {
            Err(e @ RpcError::VersionMismatch { .. }) => assert_eq!(e.code(), Some("R008")),
            other => panic!("unexpected {:?}", other.err()),
        }
    }

    // frob:ticket 01M4SPWRS0CDJT4GYC92AB1HCD
    // frob:tests crates/cadetta-adapter/src/rpc/client.rs::Client kind=unit
    #[test]
    fn session_methods_need_drive() {
        let mut c = Client::connect(script(&[init("1.0.0", &["emit"])]), Vec::new(), None).unwrap();
        let s = SessionParams {
            session: "s".into(),
        };
        assert!(matches!(
            c.open(&OpenParams { files: vec![] }),
            Err(RpcError::Unsupported("open"))
        ));
        assert!(matches!(c.save(&s), Err(RpcError::Unsupported("save"))));
        assert!(matches!(c.close(&s), Err(RpcError::Unsupported("close"))));
    }

    // frob:ticket 01M4SPWRS0CDJT4GYC92AB1HCD
    // frob:tests crates/cadetta-adapter/src/rpc/client.rs::Client kind=unit
    #[test]
    fn remote_error_is_surfaced() {
        let mut data =
            json!({"jsonrpc": "2.0", "id": 1, "result": init("1.0.0", &["emit"])}).to_string();
        data.push('\n');
        data.push_str(
            &json!({"jsonrpc": "2.0", "id": 2, "error": {"code": -32000, "message": "boom"}})
                .to_string(),
        );
        data.push('\n');
        let mut c = Client::connect(Cursor::new(data.into_bytes()), Vec::new(), None).unwrap();
        let r = c.emit(&EmitParams {
            design: json!({}),
            bindings: json!({}),
            policy: json!({}),
        });
        assert!(matches!(r, Err(RpcError::Remote { code: -32000, .. })));
    }

    // frob:ticket 01M4SPWRS0CDJT4GYC92AB1HCD
    // frob:tests crates/cadetta-adapter/src/rpc/client.rs::ProcessClient kind=unit
    #[cfg(unix)]
    #[test]
    fn spawned_process_speaks_the_protocol() {
        let reply1 = json!({"jsonrpc": "2.0", "id": 1, "result": init("0.1.0", &["emit"])});
        let reply2 = json!({"jsonrpc": "2.0", "id": 2, "result": null});
        let script = format!("read a; echo '{reply1}'; read b; echo '{reply2}'");
        let c = ProcessClient::spawn(
            Path::new("/bin/sh"),
            &["-c".to_string(), script],
            Some("0.1.0"),
        )
        .unwrap();
        assert_eq!(c.info().name, "mock");
        c.shutdown().unwrap();
    }

    // frob:ticket 01M4YG7D22XEEK9M3X678B88BT
    // frob:tests crates/cadetta-adapter/src/rpc/client.rs::spawn_with_timeout kind=unit
    #[cfg(unix)]
    #[test]
    fn silent_process_times_out_even_when_a_grandchild_holds_the_pipe() {
        let start = std::time::Instant::now();
        let r = ProcessClient::spawn_with_timeout(
            Path::new("/bin/sh"),
            &["-c".to_string(), "sleep 20; echo done".to_string()],
            None,
            Some(Duration::from_millis(300)),
        );
        assert!(
            matches!(&r, Err(RpcError::Io(e)) if e.kind() == std::io::ErrorKind::TimedOut),
            "{:?}",
            r.err()
        );
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "{:?}",
            start.elapsed()
        );
    }

    // frob:ticket 01M4YG7D22XEEK9M3X678B88BT
    // frob:tests crates/cadetta-adapter/src/rpc/client.rs::spawn_with_timeout kind=unit
    #[cfg(unix)]
    #[test]
    fn prompt_process_is_not_killed_by_a_generous_timeout() {
        let reply1 = json!({"jsonrpc": "2.0", "id": 1, "result": init("0.1.0", &["emit"])});
        let reply2 = json!({"jsonrpc": "2.0", "id": 2, "result": null});
        let script = format!("read a; echo '{reply1}'; read b; echo '{reply2}'");
        let c = ProcessClient::spawn_with_timeout(
            Path::new("/bin/sh"),
            &["-c".to_string(), script],
            Some("0.1.0"),
            Some(Duration::from_secs(30)),
        )
        .unwrap();
        c.shutdown().unwrap();
    }
}
