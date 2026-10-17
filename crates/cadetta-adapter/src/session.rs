//! Drive-session lifecycle over a protocol client: open, push, save, close, shutdown.
//!
//! Specification: `docs/spec/adapters.md` 2.2 (documents, sessions, saving) and 5.

use std::fmt;
use std::io::{BufRead, Write};

use crate::availability::{Busy, partition_busy};
use crate::rpc::{
    ChangedParam, Client, LocatorStatus, OpenParams, PushParams, PushResult, PushStatus,
    ReadParams, ReadResult, ReadSource, RegenParams, RegenResult, RpcError, SaveResult,
    SessionParams,
};

/// Diagnostic code for a dirty, locked or read-only document (adapters.md 2.3).
pub const R009: &str = "R009";

/// `[[adapter]] save`: when the adapter saves the documents it modified.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SaveMode {
    /// Save after a successful sync (the default).
    #[default]
    AfterSync,
    /// Leave documents modified; the sync obligation stays open as "unsaved".
    Never,
}

/// Failure of a session step.
#[derive(Debug)]
pub enum SessionError {
    /// The protocol call failed.
    Rpc(RpcError),
    /// A document is dirty or read-only and is refused before any push (R009).
    Refused {
        /// The refused file.
        file: String,
        /// Why it was refused.
        reason: &'static str,
    },
}

impl SessionError {
    /// The diagnostic code this error maps to, if any.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::Refused { .. } => Some(R009),
            Self::Rpc(e) => e.code(),
        }
    }
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rpc(e) => write!(f, "{e}"),
            Self::Refused { file, reason } => write!(f, "{R009}: `{file}` is {reason}"),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<RpcError> for SessionError {
    fn from(e: RpcError) -> Self {
        Self::Rpc(e)
    }
}

/// Changed parameters destined for one document.
#[derive(Debug, Clone, PartialEq)]
pub struct FilePush {
    /// The document to push into.
    pub file: String,
    /// The changed parameters.
    pub params: Vec<ChangedParam>,
}

/// What a full drive sync produced.
#[derive(Debug, Clone, PartialEq)]
pub struct SyncReport {
    /// Per-file, per-locator push statuses, in push order.
    pub statuses: Vec<(String, Vec<LocatorStatus>)>,
    /// Saved files, or `None` when saving was skipped.
    pub saved: Option<SaveResult>,
}

/// An open adapter session; closes only what it opened.
pub struct DriveSession<R: BufRead, W: Write> {
    client: Client<R, W>,
    handle: String,
    busy: Vec<Busy>,
}

impl<R: BufRead, W: Write> DriveSession<R, W> {
    /// Opens `files`; dirty or read-only files are refused per file (R009) and the rest proceed.
    ///
    /// The session is closed, the adapter shut down and `Refused` returned only when every file
    /// is refused.
    // frob:ticket 01M4Y3WHS6G3MXMECY8ZMVJ6CV
    pub fn open(mut client: Client<R, W>, files: &[String]) -> Result<Self, SessionError> {
        let res = client.open(&OpenParams {
            files: files.to_vec(),
        })?;
        let session = res.session;
        let handle = session.handle.clone();
        let (ready, busy) = partition_busy(&session.files);
        let this = Self {
            client,
            handle,
            busy,
        };
        if ready.is_empty()
            && let Some(first) = this.busy.first()
        {
            let refused = SessionError::Refused {
                file: first.file.clone(),
                reason: first.reason,
            };
            tracing::warn!(refused = this.busy.len(), "every document refused (R009)");
            this.abandon();
            return Err(refused);
        }
        tracing::info!(handle = %this.handle, files = files.len(), busy = this.busy.len(), "session opened");
        Ok(this)
    }

    /// The files refused at open (R009); the host must not push to, save or close them itself.
    // frob:ticket 01M4Y3WHS6G3MXMECY8ZMVJ6CV
    pub fn busy(&self) -> &[Busy] {
        &self.busy
    }

    /// The session handle the adapter issued.
    pub fn handle(&self) -> &str {
        &self.handle
    }

    fn params(&self) -> SessionParams {
        SessionParams {
            session: self.handle.clone(),
        }
    }

    /// Pushes changed parameters into one file; each locator carries its own status.
    pub fn push(
        &mut self,
        file: &str,
        params: Vec<ChangedParam>,
    ) -> Result<PushResult, SessionError> {
        let res = self.client.push(&PushParams {
            session: self.handle.clone(),
            file: file.to_string(),
            params,
        })?;
        for s in &res.statuses {
            tracing::debug!(file, locator = %s.locator, status = ?s.status, "locator pushed");
        }
        Ok(res)
    }

    // frob:ticket 01M4SPWTDD069AXXQBXD4T6ATZ
    /// Rebuilds `targets` (all files when empty) so dependent geometry recomputes (R004 on failure).
    pub fn regen(&mut self, targets: &[String]) -> Result<RegenResult, SessionError> {
        let res = self.client.regen(&RegenParams {
            session: self.handle.clone(),
            targets: targets.to_vec(),
        })?;
        tracing::info!(targets = targets.len(), "session regenerated");
        Ok(res)
    }

    // frob:ticket 01M4SPWTDD069AXXQBXD4T6ATZ
    /// Reads the session's documents as they are now (the read-back after a push).
    pub fn read(&mut self) -> Result<ReadResult, SessionError> {
        let res = self.client.read(&ReadParams {
            source: ReadSource::Session(self.handle.clone()),
        })?;
        tracing::debug!("session read back");
        Ok(res)
    }

    /// Saves the documents of the session.
    pub fn save(&mut self) -> Result<SaveResult, SessionError> {
        let res = self.client.save(&self.params())?;
        tracing::info!(files = res.files.len(), "session saved");
        Ok(res)
    }

    /// Closes the session and shuts the adapter down, logging (not returning) failures (R009 path).
    fn abandon(mut self) {
        if let Err(e) = self.client.close(&self.params()) {
            tracing::warn!(error = %e, "closing session failed");
        }
        if let Err(e) = self.client.shutdown() {
            tracing::warn!(error = %e, "shutting the adapter down failed");
        }
    }

    /// Closes the session and shuts the adapter down.
    pub fn finish(mut self) -> Result<(), SessionError> {
        self.client.close(&self.params())?;
        tracing::info!(handle = %self.handle, "session closed");
        self.client.shutdown()?;
        Ok(())
    }
}

/// Runs open, push per file, save per `mode`, close and shutdown, in that order.
///
/// Saving is skipped when `mode` is `Never` or any locator `failed`; the session is
/// always closed and the adapter shut down once it was opened.
pub fn sync<R: BufRead, W: Write>(
    client: Client<R, W>,
    pushes: Vec<FilePush>,
    mode: SaveMode,
) -> Result<SyncReport, SessionError> {
    let files: Vec<String> = pushes.iter().map(|p| p.file.clone()).collect();
    let mut session = DriveSession::open(client, &files)?;
    let mut statuses = Vec::new();
    let mut failure = None;
    let refused: Vec<String> = session.busy().iter().map(|b| b.file.clone()).collect();
    for p in pushes.into_iter().filter(|p| !refused.contains(&p.file)) {
        match session.push(&p.file, p.params) {
            Ok(r) => statuses.push((p.file, r.statuses)),
            Err(e) => {
                failure = Some(e);
                break;
            }
        }
    }
    let any_failed = statuses
        .iter()
        .flat_map(|(_, s)| s)
        .any(|s| s.status == PushStatus::Failed);
    let mut saved = None;
    if failure.is_none() && mode == SaveMode::AfterSync && !any_failed {
        match session.save() {
            Ok(r) => saved = Some(r),
            Err(e) => failure = Some(e),
        }
    } else {
        tracing::info!(?mode, any_failed, "save skipped");
    }
    let finished = session.finish();
    match (failure, finished) {
        (Some(e), _) | (None, Err(e)) => Err(e),
        (None, Ok(())) => Ok(SyncReport { statuses, saved }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::PushMode;
    use serde_json::{Value, json};
    use std::io::Cursor;

    fn script(results: &[Value]) -> Cursor<Vec<u8>> {
        let mut out = String::new();
        for (i, r) in results.iter().enumerate() {
            out.push_str(&json!({"jsonrpc": "2.0", "id": i + 1, "result": r}).to_string());
            out.push('\n');
        }
        Cursor::new(out.into_bytes())
    }

    fn init() -> Value {
        json!({"name": "mock", "version": "1.0.0", "capabilities": ["drive"],
               "native_units": "mm", "precision": 6, "platforms": ["linux"]})
    }

    fn opened(dirty: bool) -> Value {
        json!({"session": {"handle": "s1",
               "files": [{"path": "a.FCStd", "dirty": dirty, "readonly": false}]},
               "diagnostics": []})
    }

    fn change() -> ChangedParam {
        ChangedParam {
            path: "p.x".into(),
            value: json!("2 mm"),
            locator: "Sketch.d1".into(),
            mode: PushMode::Value,
            ref_to: None,
            tolerance: None,
            kind: None,
        }
    }

    fn methods(out: &[u8]) -> Vec<String> {
        String::from_utf8(out.to_vec())
            .unwrap()
            .lines()
            .map(|l| {
                serde_json::from_str::<Value>(l).unwrap()["method"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect()
    }

    fn pushes() -> Vec<FilePush> {
        vec![FilePush {
            file: "a.FCStd".into(),
            params: vec![change()],
        }]
    }

    // frob:ticket 01M4SWMCFYWCHWRQM0B6SFGK71
    // frob:tests crates/cadetta-adapter/src/session.rs::sync kind=unit
    #[test]
    fn sync_calls_open_push_save_close_shutdown_in_order() {
        let results = [
            init(),
            opened(false),
            json!({"statuses": [{"locator": "Sketch.d1", "status": "pushed",
                   "old": "1 mm", "new": "2 mm"}], "diagnostics": []}),
            json!({"files": [{"path": "a.FCStd", "digest": "blake3:01"}], "diagnostics": []}),
            Value::Null,
            Value::Null,
        ];
        let mut out = Vec::new();
        let c = Client::connect(script(&results), &mut out, None).unwrap();
        let report = sync(c, pushes(), SaveMode::AfterSync).unwrap();
        assert!(report.saved.is_some());
        assert_eq!(
            methods(&out),
            ["initialize", "open", "push", "save", "close", "shutdown"]
        );
    }

    // frob:ticket 01M4SWMCFYWCHWRQM0B6SFGK71
    // frob:tests crates/cadetta-adapter/src/session.rs::sync kind=unit
    #[test]
    fn save_never_skips_save() {
        let results = [
            init(),
            opened(false),
            json!({"statuses": [], "diagnostics": []}),
            Value::Null,
            Value::Null,
        ];
        let mut out = Vec::new();
        let c = Client::connect(script(&results), &mut out, None).unwrap();
        let report = sync(c, pushes(), SaveMode::Never).unwrap();
        assert!(report.saved.is_none());
        assert_eq!(
            methods(&out),
            ["initialize", "open", "push", "close", "shutdown"]
        );
    }

    // frob:ticket 01M4SWMCFYWCHWRQM0B6SFGK71
    // frob:tests crates/cadetta-adapter/src/session.rs::DriveSession kind=unit
    #[test]
    fn push_response_has_status_per_locator() {
        let results = [
            init(),
            opened(false),
            json!({"statuses": [
                {"locator": "A.d1", "status": "pushed", "old": "1 mm", "new": "2 mm"},
                {"locator": "A.d2", "status": "rejected", "old": null, "new": null},
                {"locator": "A.d3", "status": "failed", "old": null, "new": null}],
                "diagnostics": []}),
        ];
        let c = Client::connect(script(&results), Vec::new(), None).unwrap();
        let mut s = DriveSession::open(c, &["a.FCStd".to_string()]).unwrap();
        assert_eq!(s.handle(), "s1");
        let r = s.push("a.FCStd", vec![change()]).unwrap();
        let got: Vec<(&str, PushStatus)> = r
            .statuses
            .iter()
            .map(|l| (l.locator.as_str(), l.status))
            .collect();
        assert_eq!(
            got,
            [
                ("A.d1", PushStatus::Pushed),
                ("A.d2", PushStatus::Rejected),
                ("A.d3", PushStatus::Failed)
            ]
        );
    }

    // frob:ticket 01M4SWMCFYWCHWRQM0B6SFGK71
    // frob:tests crates/cadetta-adapter/src/session.rs::DriveSession kind=unit
    #[test]
    fn dirty_document_is_refused_r009_before_push() {
        let results = [init(), opened(true), Value::Null, Value::Null];
        let mut out = Vec::new();
        let c = Client::connect(script(&results), &mut out, None).unwrap();
        let code = DriveSession::open(c, &["a.FCStd".to_string()])
            .err()
            .and_then(|e| e.code());
        assert_eq!(code, Some("R009"));
        assert_eq!(methods(&out), ["initialize", "open", "close", "shutdown"]);
    }

    // frob:ticket 01M4Y3WHS6G3MXMECY8ZMVJ6CV
    // frob:tests crates/cadetta-adapter/src/session.rs::DriveSession kind=unit
    #[test]
    fn dirty_file_among_several_is_refused_and_the_rest_are_pushed() {
        let two = json!({"session": {"handle": "s1", "files": [
            {"path": "a.FCStd", "dirty": true, "readonly": false},
            {"path": "b.FCStd", "dirty": false, "readonly": false}]},
            "diagnostics": []});
        let push = json!({"statuses": [], "diagnostics": []});
        let results = [init(), two, push, Value::Null, Value::Null];
        let mut out = Vec::new();
        let c = Client::connect(script(&results), &mut out, None).unwrap();
        let report = sync(
            c,
            vec![
                FilePush {
                    file: "a.FCStd".into(),
                    params: vec![change()],
                },
                FilePush {
                    file: "b.FCStd".into(),
                    params: vec![change()],
                },
            ],
            SaveMode::Never,
        )
        .unwrap();
        assert_eq!(report.statuses.len(), 1);
        assert_eq!(report.statuses[0].0, "b.FCStd");
        let m = methods(&out);
        assert_eq!(m.iter().filter(|x| *x == "push").count(), 1);
    }
}
