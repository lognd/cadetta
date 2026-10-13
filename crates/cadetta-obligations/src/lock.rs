//! `cadetta.lock`: the committed, line-oriented record of everything not derivable from source.
//!
//! Specification: `docs/spec/obligations.md` section 3 (and 2.4).
//! frob:ticket 01M4ST8EPKBWRWHF51Y5NV3VK0

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io;
use std::path::Path;

use thiserror::Error;
use tracing::{debug, trace, warn};

use crate::graph::{IdError, ObligationId, Verdict};

/// Errors reading, parsing or writing the lock.
#[derive(Debug, Error)]
pub enum LockError {
    /// A line of the lock is not a well formed record.
    #[error("cadetta.lock line {line}: {reason}")]
    Syntax {
        /// 1-based line number.
        line: usize,
        /// What is wrong.
        reason: String,
    },
    /// The lock file could not be read or written.
    #[error("cadetta.lock i/o: {0}")]
    Io(#[from] io::Error),
}

fn bad(line: usize, reason: impl Into<String>) -> LockError {
    LockError::Syntax {
        line,
        reason: reason.into(),
    }
}

/// A calendar date `YYYY-MM-DD`; lexical order is chronological order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date(String);

impl Date {
    /// Validate and wrap a `YYYY-MM-DD` date (no time component is ever stored).
    pub fn new(s: &str) -> Result<Self, String> {
        let b = s.as_bytes();
        let ok = b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
            && (1..=12).contains(&s[5..7].parse::<u32>().unwrap_or(0))
            && (1..=31).contains(&s[8..10].parse::<u32>().unwrap_or(0));
        if ok {
            Ok(Self(s.to_owned()))
        } else {
            Err(format!("`{s}` is not a YYYY-MM-DD date"))
        }
    }

    /// The date text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A lowercase hexadecimal digest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hex(String);

impl Hex {
    /// Validate and wrap a non-empty lowercase hex string.
    pub fn new(s: &str) -> Result<Self, String> {
        if !s.is_empty() && s.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) {
            Ok(Self(s.to_owned()))
        } else {
            Err(format!("`{s}` is not a lowercase hex digest"))
        }
    }

    /// The digest text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `attest`: a review of one obligation in one role (key: id, role).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attest {
    /// The obligation attested. Cited-file acknowledgements are `citation:` ids.
    pub id: ObligationId,
    /// Role that attested.
    pub role: String,
    /// Who (`git config user.email` or `--as`).
    pub by: String,
    /// When (date only).
    pub at: Date,
    /// Closure digest of the covered set at ack time.
    pub digest: Hex,
    /// Verdict at ack time.
    pub verdict: Verdict,
    /// Margin at ack time (quantity text, opaque here).
    pub margin: String,
    /// Own digests of every transitive dep; kept sorted and unique.
    pub deps: Vec<Hex>,
    /// Free text; `baseline` for `ack --baseline`.
    pub reason: String,
}

/// A `--alt SLOT=ALT` restriction of a waiver.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AltBinding {
    /// Part slot.
    pub slot: String,
    /// Alternate chosen for the slot.
    pub alternate: String,
}

/// `waiver`: closure digest of a waiver's targets at review time (key: code, glob, alt).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiver {
    /// Diagnostic code.
    pub code: String,
    /// Target glob.
    pub glob: String,
    /// Optional alternate restriction.
    pub alt: Option<AltBinding>,
    /// Closure digest of the targets.
    pub digest: Hex,
}

/// `synced ADAPTER PATH`: last emitted value of one locator (key: adapter, path, locator).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedValue {
    /// Adapter name.
    pub adapter: String,
    /// Parameter path.
    pub path: String,
    /// Native locator within the CAD file.
    pub locator: String,
    /// Last emitted value text.
    pub value: String,
}

/// `synced ADAPTER file=`: saved digest of one native file (key: adapter, file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedFile {
    /// Adapter name.
    pub adapter: String,
    /// Native file path.
    pub file: String,
    /// Digest of the file as last synced.
    pub digest: Hex,
}

/// `adopted`: an imported file awaiting its first sync (key: adapter, file; R006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adopted {
    /// Adapter name.
    pub adapter: String,
    /// Native file path.
    pub file: String,
}

/// `package`: one pinned dependency (key: name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Package name.
    pub name: String,
    /// Resolved version.
    pub version: String,
    /// Source (registry, path or git locator).
    pub source: String,
    /// Content digest.
    pub digest: Hex,
}

/// `swap`: audit trail of a part swap (key: the whole line; never tidied away).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Swap {
    /// Part slot.
    pub slot: String,
    /// Alternate swapped out.
    pub from: String,
    /// Alternate swapped in.
    pub to: String,
    /// Who.
    pub by: String,
    /// When.
    pub at: Date,
    /// Why.
    pub reason: String,
}

/// One lock record: the six kinds of obligations.md 3 (`synced` has two forms).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    /// See [`Attest`].
    Attest(Attest),
    /// See [`Waiver`].
    Waiver(Waiver),
    /// See [`SyncedValue`].
    SyncedValue(SyncedValue),
    /// See [`SyncedFile`].
    SyncedFile(SyncedFile),
    /// See [`Adopted`].
    Adopted(Adopted),
    /// See [`Package`].
    Package(Package),
    /// See [`Swap`].
    Swap(Swap),
}

/// Sort key of a record: (kind word, key fields). Kinds sort bytewise by their word.
pub type RecordKey = (&'static str, Vec<String>);

impl Record {
    /// The record's (kind, key) sort key (obligations.md 3 table).
    pub fn key(&self) -> RecordKey {
        match self {
            Record::Attest(a) => ("attest", vec![a.id.to_string(), a.role.clone()]),
            Record::Waiver(w) => {
                let alt = w
                    .alt
                    .as_ref()
                    .map(|a| format!("{}={}", a.slot, a.alternate))
                    .unwrap_or_default();
                ("waiver", vec![w.code.clone(), w.glob.clone(), alt])
            }
            Record::SyncedValue(s) => (
                "synced",
                vec![
                    s.adapter.clone(),
                    "0".into(),
                    s.path.clone(),
                    s.locator.clone(),
                ],
            ),
            Record::SyncedFile(s) => (
                "synced",
                vec![s.adapter.clone(), "1".into(), s.file.clone()],
            ),
            Record::Adopted(a) => ("adopted", vec![a.adapter.clone(), a.file.clone()]),
            Record::Package(p) => ("package", vec![p.name.clone()]),
            Record::Swap(_) => ("swap", vec![self.render()]),
        }
    }

    /// The record's date, for the tie rule (only `attest` and `swap` carry one).
    pub fn at(&self) -> Option<&Date> {
        match self {
            Record::Attest(a) => Some(&a.at),
            Record::Swap(s) => Some(&s.at),
            _ => None,
        }
    }

    /// Render the record as its canonical single line (no newline).
    pub fn render(&self) -> String {
        let mut s = String::new();
        match self {
            Record::Attest(a) => {
                let mut deps: Vec<&str> = a.deps.iter().map(Hex::as_str).collect();
                deps.sort_unstable();
                deps.dedup();
                let _ = write!(
                    s,
                    "attest {} role={} by={} at={} digest={} verdict={} margin={} deps={} reason={}",
                    word(&a.id.to_string(), true),
                    word(&a.role, false),
                    word(&a.by, false),
                    a.at.as_str(),
                    a.digest.as_str(),
                    verdict_name(a.verdict),
                    word(&a.margin, false),
                    deps.join(","),
                    quoted(&a.reason),
                );
            }
            Record::Waiver(w) => {
                let _ = write!(s, "waiver {} {}", word(&w.code, true), word(&w.glob, true));
                if let Some(a) = &w.alt {
                    let _ = write!(
                        s,
                        " alt={}={}",
                        word(&a.slot, false),
                        word(&a.alternate, false)
                    );
                }
                let _ = write!(s, " digest={}", w.digest.as_str());
            }
            Record::SyncedValue(v) => {
                let _ = write!(
                    s,
                    "synced {} {} locator={} value={}",
                    word(&v.adapter, true),
                    word(&v.path, true),
                    word(&v.locator, false),
                    word(&v.value, false),
                );
            }
            Record::SyncedFile(f) => {
                let _ = write!(
                    s,
                    "synced {} file={} digest={}",
                    word(&f.adapter, true),
                    word(&f.file, false),
                    f.digest.as_str(),
                );
            }
            Record::Adopted(a) => {
                let _ = write!(
                    s,
                    "adopted {} {}",
                    word(&a.adapter, true),
                    word(&a.file, true)
                );
            }
            Record::Package(p) => {
                let _ = write!(
                    s,
                    "package {} version={} source={} digest={}",
                    word(&p.name, true),
                    word(&p.version, false),
                    word(&p.source, false),
                    p.digest.as_str(),
                );
            }
            Record::Swap(w) => {
                let _ = write!(
                    s,
                    "swap {} from={} to={} by={} at={} reason={}",
                    word(&w.slot, true),
                    word(&w.from, false),
                    word(&w.to, false),
                    word(&w.by, false),
                    w.at.as_str(),
                    quoted(&w.reason),
                );
            }
        }
        s
    }
}

fn verdict_name(v: Verdict) -> &'static str {
    match v {
        Verdict::Holds => "holds",
        Verdict::Violated => "violated",
        Verdict::Undecided => "undecided",
    }
}

fn verdict_parse(s: &str) -> Option<Verdict> {
    [Verdict::Holds, Verdict::Violated, Verdict::Undecided]
        .into_iter()
        .find(|v| verdict_name(*v) == s)
}

/// Always-quoted string with `\\ \" \n \r \t \u{..}` escapes.
fn quoted(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(o, "\\u{{{:x}}}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Bare when safe, quoted otherwise; positional words also quote `=` so they never read as `key=value`.
fn word(s: &str, positional: bool) -> String {
    let needs = s.is_empty()
        || s.chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '"' || c == '\\')
        || (positional && s.contains('='));
    if needs { quoted(s) } else { s.to_owned() }
}

/// A lexed token: `key=value` (key present) or a positional word.
struct Token {
    key: Option<String>,
    value: String,
}

fn lex_quoted(
    it: &mut std::iter::Peekable<std::str::Chars<'_>>,
    line: usize,
) -> Result<String, LockError> {
    // The opening quote is already consumed.
    let mut o = String::new();
    loop {
        match it.next() {
            None => return Err(bad(line, "unterminated quoted string")),
            Some('"') => return Ok(o),
            Some('\\') => match it.next() {
                Some('"') => o.push('"'),
                Some('\\') => o.push('\\'),
                Some('n') => o.push('\n'),
                Some('r') => o.push('\r'),
                Some('t') => o.push('\t'),
                Some('u') => {
                    if it.next() != Some('{') {
                        return Err(bad(line, "bad \\u escape"));
                    }
                    let mut h = String::new();
                    loop {
                        match it.next() {
                            Some('}') => break,
                            Some(c) if c.is_ascii_hexdigit() => h.push(c),
                            _ => return Err(bad(line, "bad \\u escape")),
                        }
                    }
                    let c = u32::from_str_radix(&h, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .ok_or_else(|| bad(line, "bad \\u escape"))?;
                    o.push(c);
                }
                _ => return Err(bad(line, "unknown escape")),
            },
            Some(c) => o.push(c),
        }
    }
}

fn lex(text: &str, line: usize) -> Result<Vec<Token>, LockError> {
    let mut toks = Vec::new();
    let mut it = text.chars().peekable();
    loop {
        while it.peek().is_some_and(|c| *c == ' ') {
            it.next();
        }
        let Some(&first) = it.peek() else { break };
        if first == '"' {
            it.next();
            toks.push(Token {
                key: None,
                value: lex_quoted(&mut it, line)?,
            });
            continue;
        }
        let mut bare = String::new();
        while it.peek().is_some_and(|c| *c != ' ') {
            bare.push(it.next().unwrap_or(' '));
        }
        let key_len = bare
            .find(|c: char| !(c.is_ascii_lowercase() || c == '_'))
            .filter(|&i| i > 0 && bare.as_bytes()[i] == b'=');
        match key_len {
            Some(i) => {
                let key = bare[..i].to_owned();
                let rest = &bare[i + 1..];
                let value = if let Some(q) = rest.strip_prefix('"') {
                    // Re-lex the quoted value, which may contain spaces: splice back the remaining input.
                    let mut tail: String = q.to_owned();
                    tail.extend(it.by_ref());
                    let mut ti = tail.chars().peekable();
                    let v = lex_quoted(&mut ti, line)?;
                    let remaining: String = ti.collect();
                    toks.push(Token {
                        key: Some(key),
                        value: v,
                    });
                    return {
                        toks.extend(lex(&remaining, line)?);
                        Ok(toks)
                    };
                } else {
                    rest.to_owned()
                };
                toks.push(Token {
                    key: Some(key),
                    value,
                });
            }
            None => toks.push(Token {
                key: None,
                value: bare,
            }),
        }
    }
    Ok(toks)
}

/// Positional and keyed fields of one line, consumed field by field.
struct Fields {
    line: usize,
    pos: std::vec::IntoIter<String>,
    kv: BTreeMap<String, String>,
}

impl Fields {
    fn new(toks: Vec<Token>, line: usize) -> Result<Self, LockError> {
        let mut pos = Vec::new();
        let mut kv = BTreeMap::new();
        for t in toks {
            match t.key {
                None => pos.push(t.value),
                Some(k) => {
                    if kv.insert(k.clone(), t.value).is_some() {
                        return Err(bad(line, format!("duplicate field `{k}`")));
                    }
                }
            }
        }
        Ok(Self {
            line,
            pos: pos.into_iter(),
            kv,
        })
    }

    fn pos(&mut self, what: &str) -> Result<String, LockError> {
        self.pos
            .next()
            .ok_or_else(|| bad(self.line, format!("missing {what}")))
    }

    fn take(&mut self, key: &str) -> Result<String, LockError> {
        self.kv
            .remove(key)
            .ok_or_else(|| bad(self.line, format!("missing field `{key}`")))
    }

    fn opt(&mut self, key: &str) -> Option<String> {
        self.kv.remove(key)
    }

    fn date(&mut self, key: &str) -> Result<Date, LockError> {
        let v = self.take(key)?;
        Date::new(&v).map_err(|e| bad(self.line, e))
    }

    fn hex(&mut self, key: &str) -> Result<Hex, LockError> {
        let v = self.take(key)?;
        Hex::new(&v).map_err(|e| bad(self.line, e))
    }

    fn finish(mut self) -> Result<(), LockError> {
        if let Some(p) = self.pos.next() {
            return Err(bad(self.line, format!("unexpected word `{p}`")));
        }
        if let Some(k) = self.kv.keys().next() {
            return Err(bad(self.line, format!("unexpected field `{k}`")));
        }
        Ok(())
    }
}

fn parse_record(text: &str, line: usize) -> Result<Record, LockError> {
    let mut toks = lex(text, line)?;
    if toks.is_empty() || toks[0].key.is_some() {
        return Err(bad(line, "expected a record kind"));
    }
    let kind = toks.remove(0).value;
    let mut f = Fields::new(toks, line)?;
    let rec = match kind.as_str() {
        "attest" => {
            let id = f.pos("obligation id")?;
            let id = ObligationId::parse(&id).map_err(|e: IdError| bad(line, e.to_string()))?;
            let role = f.take("role")?;
            let by = f.take("by")?;
            let at = f.date("at")?;
            let digest = f.hex("digest")?;
            let v = f.take("verdict")?;
            let verdict =
                verdict_parse(&v).ok_or_else(|| bad(line, format!("unknown verdict `{v}`")))?;
            let margin = f.take("margin")?;
            let deps_raw = f.take("deps")?;
            let mut deps = Vec::new();
            if !deps_raw.is_empty() {
                for d in deps_raw.split(',') {
                    deps.push(Hex::new(d).map_err(|e| bad(line, e))?);
                }
            }
            deps.sort();
            deps.dedup();
            let reason = f.take("reason")?;
            Record::Attest(Attest {
                id,
                role,
                by,
                at,
                digest,
                verdict,
                margin,
                deps,
                reason,
            })
        }
        "waiver" => {
            let code = f.pos("code")?;
            let glob = f.pos("glob")?;
            let alt = match f.opt("alt") {
                None => None,
                Some(a) => {
                    let (slot, alternate) = a
                        .split_once('=')
                        .ok_or_else(|| bad(line, "alt must be SLOT=ALT"))?;
                    Some(AltBinding {
                        slot: slot.to_owned(),
                        alternate: alternate.to_owned(),
                    })
                }
            };
            let digest = f.hex("digest")?;
            Record::Waiver(Waiver {
                code,
                glob,
                alt,
                digest,
            })
        }
        "synced" => {
            let adapter = f.pos("adapter")?;
            if f.kv.contains_key("file") {
                let file = f.take("file")?;
                let digest = f.hex("digest")?;
                Record::SyncedFile(SyncedFile {
                    adapter,
                    file,
                    digest,
                })
            } else {
                let path = f.pos("path")?;
                let locator = f.take("locator")?;
                let value = f.take("value")?;
                Record::SyncedValue(SyncedValue {
                    adapter,
                    path,
                    locator,
                    value,
                })
            }
        }
        "adopted" => {
            let adapter = f.pos("adapter")?;
            let file = f.pos("file")?;
            Record::Adopted(Adopted { adapter, file })
        }
        "package" => {
            let name = f.pos("name")?;
            let version = f.take("version")?;
            let source = f.take("source")?;
            let digest = f.hex("digest")?;
            Record::Package(Package {
                name,
                version,
                source,
                digest,
            })
        }
        "swap" => {
            let slot = f.pos("slot")?;
            let from = f.take("from")?;
            let to = f.take("to")?;
            let by = f.take("by")?;
            let at = f.date("at")?;
            let reason = f.take("reason")?;
            Record::Swap(Swap {
                slot,
                from,
                to,
                by,
                at,
                reason,
            })
        }
        other => return Err(bad(line, format!("unknown record kind `{other}`"))),
    };
    f.finish()?;
    Ok(rec)
}

/// The whole lock: records keyed and sorted by (kind, key).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lock {
    records: BTreeMap<RecordKey, Record>,
}

impl Lock {
    /// An empty lock.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a record applying the tie rule: newest `at` wins, else the bytewise greater line.
    /// Returns whether the record is now the stored one.
    pub fn insert(&mut self, mut rec: Record) -> bool {
        if let Record::Attest(a) = &mut rec {
            a.deps.sort();
            a.deps.dedup();
        }
        let key = rec.key();
        let win = match self.records.get(&key) {
            None => true,
            Some(old) => {
                let ord = rec
                    .at()
                    .cmp(&old.at())
                    .then_with(|| rec.render().cmp(&old.render()));
                ord.is_gt()
            }
        };
        if win {
            trace!(kind = key.0, key = ?key.1, "lock record stored");
            self.records.insert(key, rec);
        } else {
            debug!(kind = key.0, key = ?key.1, "lock record lost the tie rule");
        }
        win
    }

    /// Remove and return the record with this key.
    pub fn remove(&mut self, key: &RecordKey) -> Option<Record> {
        self.records.remove(key)
    }

    /// Look a record up by key.
    pub fn get(&self, key: &RecordKey) -> Option<&Record> {
        self.records.get(key)
    }

    /// The attestation of `id` by `role`, if any.
    pub fn attestation(&self, id: &ObligationId, role: &str) -> Option<&Attest> {
        match self
            .records
            .get(&("attest", vec![id.to_string(), role.to_owned()]))
        {
            Some(Record::Attest(a)) => Some(a),
            _ => None,
        }
    }

    /// Records in canonical (kind, key) order.
    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.records.values()
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the lock has no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Parse lock text. Order and duplicates are tolerated (tie rule applies, with a warning);
    /// see [`Lock::is_canonical`] for the strict check `lock verify` needs.
    pub fn parse(text: &str) -> Result<Self, LockError> {
        let mut lock = Self::new();
        let body = text.strip_suffix('\n').unwrap_or(text);
        if !body.is_empty() || text.contains('\n') {
            for (i, l) in body.split('\n').enumerate() {
                let rec = parse_record(l, i + 1)?;
                if !lock.insert(rec) {
                    warn!(line = i + 1, "duplicate lock key; the other record won");
                }
            }
        }
        debug!(records = lock.len(), "lock parsed");
        Ok(lock)
    }

    /// Render the canonical bytes: sorted lines, each ending in `\n`; an empty lock is empty.
    pub fn render(&self) -> String {
        let mut s = String::new();
        for r in self.records.values() {
            s.push_str(&r.render());
            s.push('\n');
        }
        s
    }

    /// Whether `text` is exactly the canonical rendering of what it parses to (sorted, unique).
    pub fn is_canonical(text: &str) -> Result<bool, LockError> {
        Ok(Self::parse(text)?.render() == text)
    }

    /// Load from a reader closure (injected in tests); a missing file is an empty lock.
    pub fn load_with(read: impl FnOnce() -> io::Result<String>) -> Result<Self, LockError> {
        match read() {
            Ok(t) => Self::parse(&t),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                debug!("no cadetta.lock; starting empty");
                Ok(Self::new())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Load `cadetta.lock` from `path` (the only file the obligations node reads).
    pub fn load(path: &Path) -> Result<Self, LockError> {
        Self::load_with(|| std::fs::read_to_string(path))
    }

    /// Write the canonical bytes through injected `read` (current bytes) and `write` closures,
    /// skipping the write when unchanged. Returns whether `write` ran.
    pub fn save_with(
        &self,
        read: impl FnOnce() -> io::Result<String>,
        write: impl FnOnce(&str) -> io::Result<()>,
    ) -> Result<bool, LockError> {
        let bytes = self.render();
        if read().is_ok_and(|old| old == bytes) {
            debug!("cadetta.lock unchanged; not written");
            return Ok(false);
        }
        write(&bytes)?;
        debug!(records = self.len(), "cadetta.lock written");
        Ok(true)
    }

    /// Write the canonical bytes to `path` atomically (temp file then rename), skipping the
    /// write when unchanged. Returns whether the file was written.
    pub fn save(&self, path: &Path) -> Result<bool, LockError> {
        self.save_with(
            || std::fs::read_to_string(path),
            |bytes| {
                let mut tmp = path.as_os_str().to_owned();
                tmp.push(".tmp");
                std::fs::write(&tmp, bytes.as_bytes())?;
                std::fs::rename(&tmp, path)
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(s: &str) -> Hex {
        Hex::new(s).unwrap()
    }

    fn attest(id: &str, role: &str, at: &str, reason: &str) -> Record {
        Record::Attest(Attest {
            id: ObligationId::parse(id).unwrap(),
            role: role.into(),
            by: "ana@acme.example".into(),
            at: Date::new(at).unwrap(),
            digest: h("ab12"),
            verdict: Verdict::Holds,
            margin: "0.12 mm".into(),
            deps: vec![h("ff"), h("0a"), h("ff")],
            reason: reason.into(),
        })
    }

    fn sample() -> Lock {
        let mut l = Lock::new();
        l.insert(attest(
            "fit:acme::shaft_fit",
            "tolerance_review",
            "2026-10-01",
            "ok \"fine\"\nline2",
        ));
        l.insert(attest(
            "citation:docs/ds.pdf",
            "source",
            "2026-09-01",
            "baseline",
        ));
        l.insert(Record::Waiver(Waiver {
            code: "W001".into(),
            glob: "acme::*".into(),
            alt: Some(AltBinding {
                slot: "wheel".into(),
                alternate: "b".into(),
            }),
            digest: h("01"),
        }));
        l.insert(Record::SyncedValue(SyncedValue {
            adapter: "freecad".into(),
            path: "acme::shaft.dia".into(),
            locator: "Sketch.d1".into(),
            value: "12.5 mm".into(),
        }));
        l.insert(Record::SyncedFile(SyncedFile {
            adapter: "freecad".into(),
            file: "cad/shaft.FCStd".into(),
            digest: h("de"),
        }));
        l.insert(Record::Adopted(Adopted {
            adapter: "freecad".into(),
            file: "cad/x.FCStd".into(),
        }));
        l.insert(Record::Package(Package {
            name: "std".into(),
            version: "0.1.0".into(),
            source: "registry".into(),
            digest: h("99"),
        }));
        l.insert(Record::Swap(Swap {
            slot: "wheel".into(),
            from: "a".into(),
            to: "b".into(),
            by: "bo@acme.example".into(),
            at: Date::new("2026-10-02").unwrap(),
            reason: "stock out".into(),
        }));
        l
    }

    // frob:tests crates/cadetta-obligations/src/lock.rs::Lock.render
    #[test]
    fn same_state_same_bytes_and_roundtrip() {
        let a = sample().render();
        let b = sample().render();
        assert_eq!(a, b);
        let back = Lock::parse(&a).unwrap();
        assert_eq!(back.render(), a);
        assert_eq!(back, sample());
        assert!(Lock::is_canonical(&a).unwrap());
        assert!(a.is_ascii());
    }

    // frob:tests crates/cadetta-obligations/src/lock.rs::Lock.insert
    #[test]
    fn independent_attestations_merge_as_line_union() {
        let x = attest("check:a", "r", "2026-10-01", "x");
        let y = attest("check:b", "r", "2026-10-01", "y");
        let mut l1 = Lock::new();
        l1.insert(x.clone());
        l1.insert(y.clone());
        let mut l2 = Lock::new();
        l2.insert(y);
        l2.insert(x);
        assert_eq!(l1.render(), l2.render());
        let lines: Vec<_> = l1.render().lines().map(str::to_owned).collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0] < lines[1]);
    }

    // frob:tests crates/cadetta-obligations/src/lock.rs::Record.render
    #[test]
    fn attest_holds_who_role_when_reason_and_digest_set() {
        let line = attest("check:a", "tolerance_review", "2026-10-01", "baseline").render();
        assert_eq!(
            line,
            "attest check:a role=tolerance_review by=ana@acme.example at=2026-10-01 \
             digest=ab12 verdict=holds margin=\"0.12 mm\" deps=0a,ff reason=\"baseline\""
        );
        let Record::Attest(a) = parse_record(&line, 1).unwrap() else {
            panic!()
        };
        assert_eq!(a.by, "ana@acme.example");
        assert_eq!(a.role, "tolerance_review");
        assert_eq!(a.at.as_str(), "2026-10-01");
        assert_eq!(a.reason, "baseline");
        assert_eq!(a.digest, h("ab12"));
        assert_eq!(a.deps, vec![h("0a"), h("ff")]);
    }

    // frob:tests crates/cadetta-obligations/src/lock.rs::Lock.insert
    #[test]
    fn tie_rule_newest_at_then_greater_line() {
        let mut l = Lock::new();
        assert!(l.insert(attest("check:a", "r", "2026-10-01", "old")));
        assert!(l.insert(attest("check:a", "r", "2026-10-02", "new")));
        assert!(!l.insert(attest("check:a", "r", "2026-10-01", "zzz")));
        assert_eq!(l.len(), 1);
        // equal at: greater line wins
        assert!(l.insert(attest("check:a", "r", "2026-10-02", "z")));
        assert!(!l.insert(attest("check:a", "r", "2026-10-02", "a")));
        let id = ObligationId::parse("check:a").unwrap();
        assert_eq!(l.attestation(&id, "r").unwrap().reason, "z");
    }

    // frob:tests crates/cadetta-obligations/src/lock.rs::Lock.render
    #[test]
    fn six_kinds_sorted_by_kind_then_key_line_oriented() {
        let out = sample().render();
        let kinds: Vec<&str> = out.lines().map(|l| l.split(' ').next().unwrap()).collect();
        assert_eq!(
            kinds,
            [
                "adopted", "attest", "attest", "package", "swap", "synced", "synced", "waiver"
            ]
        );
        let lines: Vec<&str> = out.lines().collect();
        assert!(
            lines[1].starts_with("attest citation:"),
            "keys sorted within kind"
        );
        assert!(
            lines[5].contains("locator="),
            "synced values before synced files"
        );
        assert!(lines[6].contains("file="));
        assert_eq!(out.lines().count(), out.matches('\n').count());
    }

    // frob:tests crates/cadetta-obligations/src/lock.rs::Lock.parse
    #[test]
    fn lock_has_no_solver_records_and_rejects_garbage() {
        assert!(Lock::parse("solver foo digest=aa\n").is_err());
        assert!(Lock::parse("<<<<<<< ours\n").is_err());
        assert!(Lock::parse("adopted a\n").is_err());
        assert!(Lock::parse("package p version=1 source=s digest=XYZ\n").is_err());
        assert!(Lock::parse("swap s from=a to=b by=u at=2026-13-01 reason=\"x\"\n").is_err());
        assert!(Lock::parse("\n").is_err());
        assert!(Lock::parse("").unwrap().is_empty());
    }

    // frob:tests crates/cadetta-obligations/src/lock.rs::Lock.is_canonical
    #[test]
    fn unsorted_or_duplicate_text_is_not_canonical() {
        let text =
            "package b version=1 source=s digest=aa\npackage a version=1 source=s digest=aa\n";
        assert!(!Lock::is_canonical(text).unwrap());
        let dup = "adopted a f\nadopted a f\n";
        assert!(!Lock::is_canonical(dup).unwrap());
    }

    // frob:tests crates/cadetta-obligations/src/lock.rs::Lock.load_with
    #[test]
    fn load_rebuilds_state_and_missing_file_is_empty() {
        let text = sample().render();
        let l = Lock::load_with(|| Ok(text.clone())).unwrap();
        assert_eq!(l, sample());
        let e = Lock::load_with(|| Err(io::Error::from(io::ErrorKind::NotFound))).unwrap();
        assert!(e.is_empty());
        assert!(Lock::load_with(|| Err(io::Error::from(io::ErrorKind::PermissionDenied))).is_err());
    }

    // frob:tests crates/cadetta-obligations/src/lock.rs::Lock.save_with
    #[test]
    fn save_is_byte_exact_and_skips_unchanged() {
        let disk = std::cell::RefCell::new(String::new());
        let lock = sample();
        assert!(
            lock.save_with(
                || Ok(disk.borrow().clone()),
                |b| {
                    *disk.borrow_mut() = b.to_owned();
                    Ok(())
                }
            )
            .unwrap()
        );
        let disk = disk.into_inner();
        assert_eq!(disk, lock.render());
        let d2 = disk.clone();
        assert!(
            !lock
                .save_with(|| Ok(d2.clone()), |_| panic!("must not write"))
                .unwrap()
        );
        assert_eq!(Lock::parse(&disk).unwrap(), lock);
    }
}
