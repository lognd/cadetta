//! Built-in CSV adapter: emits and reads `name,value,unit,note` rows (emitters/csv.md).
//!
//! RFC 4180 quoting, LF endings, a `#` header comment that is skipped on read
//! and excluded from change detection. The canonical path is not in the file;
//! the caller resolves it through the bindings.

use crate::policy::{EmitRequest, PolicyError, to_decimal};
use crate::read_units::{NativeValue, ObservedValue, UnitPolicy};

/// Adapter name.
pub const ADAPTER: &str = "csv";
/// Default decimal places (adapters.md 5).
pub const DEFAULT_PRECISION: u32 = 6;
/// The fixed column row.
pub const COLUMNS: &str = "name,value,unit,note";

/// The header comment fields: adapter version and the two digests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// Adapter name and version, e.g. `csv 0.1.0`.
    pub adapter: String,
    /// IR digest, `blake3:<hex>`.
    pub ir: String,
    /// Bindings digest, `blake3:<hex>`.
    pub bindings: String,
}

/// One emitted row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Bound locator (kind `global`) or mangled path.
    pub name: String,
    /// Decimal string, or text for Text parameters.
    pub value: String,
    /// Unit; empty for Counts, Ratios and Text.
    pub unit: String,
    /// Trailing-comment equivalent; may be empty.
    pub note: String,
}

/// A failure emitting or reading a CSV file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsvError {
    /// Malformed CSV (unterminated quote, bad column row, wrong field count).
    Parse(String),
    /// A value failed emit policy (Q004).
    Policy(PolicyError),
    /// A row whose `name` matches no binding of this adapter (R016).
    UnboundRow {
        /// The unbound `name` column value.
        name: String,
        /// The file the row came from.
        file: String,
    },
}

impl std::fmt::Display for CsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "csv file: {e}"),
            Self::Policy(e) => write!(f, "{e}"),
            Self::UnboundRow { name, file } => {
                write!(f, "R016: row `{name}` in `{file}` matches no binding")
            }
        }
    }
}

impl std::error::Error for CsvError {}

impl From<PolicyError> for CsvError {
    fn from(e: PolicyError) -> Self {
        Self::Policy(e)
    }
}

/// Builds a [`Row`] from a decided request at `precision` decimals (Q004 if inexact).
pub fn row_from_request(
    name: &str,
    req: &EmitRequest,
    precision: u32,
    note: &str,
) -> Result<Row, CsvError> {
    Ok(Row {
        name: name.to_string(),
        value: to_decimal(req.value, precision)?,
        unit: req.unit.clone(),
        note: note.to_string(),
    })
}

fn quote(field: &str, out: &mut String) {
    if field.contains([',', '"', '\r', '\n']) {
        out.push('"');
        out.push_str(&field.replace('"', "\"\""));
        out.push('"');
    } else {
        out.push_str(field);
    }
}

/// Serialises a CSV file: header comment, column row, one row each, LF, final newline.
pub fn emit(header: &Header, rows: &[Row]) -> String {
    let mut s = format!(
        "# cadetta emit {} ir={} bindings={}\n{COLUMNS}\n",
        header.adapter, header.ir, header.bindings
    );
    for r in rows {
        for (i, f) in [&r.name, &r.value, &r.unit, &r.note]
            .into_iter()
            .enumerate()
        {
            if i > 0 {
                s.push(',');
            }
            quote(f, &mut s);
        }
        s.push('\n');
    }
    tracing::debug!(rows = rows.len(), bytes = s.len(), "csv emitted");
    s
}

/// Splits RFC 4180 text into records, skipping records that begin with `#`.
fn records(text: &str) -> Result<Vec<Vec<String>>, CsvError> {
    let mut out = Vec::new();
    let mut rec: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut at_start = true;
    let mut skipping = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if skipping {
            if c == '\n' {
                skipping = false;
                at_start = true;
            }
            continue;
        }
        if at_start && !in_quotes && c == '#' {
            skipping = true;
            continue;
        }
        at_start = false;
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => in_quotes = true,
            ',' => rec.push(std::mem::take(&mut field)),
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' => {
                rec.push(std::mem::take(&mut field));
                if !(rec.len() == 1 && rec[0].is_empty()) {
                    out.push(std::mem::take(&mut rec));
                } else {
                    rec.clear();
                }
                at_start = true;
            }
            _ => field.push(c),
        }
    }
    if in_quotes {
        return Err(CsvError::Parse("unterminated quoted field".into()));
    }
    if !skipping && (!field.is_empty() || !rec.is_empty()) {
        rec.push(field);
        out.push(rec);
    }
    Ok(out)
}

fn parse(text: &str) -> Result<Vec<Row>, CsvError> {
    let recs = records(text).inspect_err(|e| tracing::warn!(error = %e, "csv rejected"))?;
    let mut it = recs.into_iter();
    match it.next() {
        Some(h) if h.join(",") == COLUMNS => {}
        _ => {
            tracing::warn!("csv missing column row");
            return Err(CsvError::Parse(format!("expected column row `{COLUMNS}`")));
        }
    }
    it.enumerate()
        .map(|(i, r)| {
            let [name, value, unit, note]: [String; 4] =
                r.try_into().map_err(|r: Vec<String>| {
                    tracing::warn!(
                        row = i + 1,
                        fields = r.len(),
                        "csv row has wrong field count"
                    );
                    CsvError::Parse(format!("row {} has {} fields, expected 4", i + 1, r.len()))
                })?;
            Ok(Row {
                name,
                value,
                unit,
                note,
            })
        })
        .collect()
}

/// One parameter as read back, with its resolved unit; the path comes from the bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadParam {
    /// Native name (the `name` column).
    pub name: String,
    /// Value with unit resolution applied.
    pub observed: ObservedValue,
}

/// Reads a CSV file; an empty `unit` takes the policy's native or assumed unit.
pub fn read(text: &str, units: &UnitPolicy) -> Result<Vec<ReadParam>, CsvError> {
    let out: Vec<ReadParam> = parse(text)?
        .into_iter()
        .map(|r| ReadParam {
            observed: units.read(&NativeValue {
                literal: r.value,
                unit: (!r.unit.is_empty()).then_some(r.unit),
            }),
            name: r.name,
        })
        .collect();
    tracing::debug!(params = out.len(), "csv read");
    Ok(out)
}

/// Reads a CSV file against the adapter's bound names; the first row whose `name`
/// is not in `bound` is R016 and no parameters are produced (emitters/csv.md 3).
pub fn read_bound(
    text: &str,
    units: &UnitPolicy,
    file: &str,
    bound: &[&str],
) -> Result<Vec<ReadParam>, CsvError> {
    let params = read(text, units)?;
    if let Some(p) = params.iter().find(|p| !bound.contains(&p.name.as_str())) {
        tracing::error!(row = %p.name, file, "R016 unbound csv row");
        return Err(CsvError::UnboundRow {
            name: p.name.clone(),
            file: file.into(),
        });
    }
    Ok(params)
}

/// Compares two files ignoring the header comment (the `sync` no-op comparison, adapters.md 2).
pub fn same_modulo_header(a: &str, b: &str) -> Result<bool, CsvError> {
    Ok(parse(a)? == parse(b)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{EmitPoint, Ratio};

    fn header(ir: &str) -> Header {
        Header {
            adapter: "csv 0.1.0".into(),
            ir: ir.into(),
            bindings: "blake3:bb".into(),
        }
    }

    fn req(num: i128, den: i128, unit: &str) -> EmitRequest {
        EmitRequest {
            path: "p".into(),
            point: EmitPoint::Nominal,
            value: Ratio::new(num, den),
            unit: unit.into(),
        }
    }

    fn rows() -> Vec<Row> {
        vec![
            row_from_request("Plate_Thk", &req(127, 20, "mm"), 6, "1/4in @\"ASTM B209\"").unwrap(),
            row_from_request("PilotBore", &req(12009, 1000, "mm"), 6, "12mm H7; emit mid").unwrap(),
            row_from_request("NumBolts", &req(4, 1, ""), 6, "").unwrap(),
        ]
    }

    // frob:tests crates/cadetta-adapter/src/builtin/csv.rs::emit kind=unit
    #[test]
    fn emit_is_rfc4180_golden() {
        let a = emit(&header("blake3:aa"), &rows());
        assert_eq!(a, emit(&header("blake3:aa"), &rows()));
        let golden = "# cadetta emit csv 0.1.0 ir=blake3:aa bindings=blake3:bb\nname,value,unit,note\nPlate_Thk,6.35,mm,\"1/4in @\"\"ASTM B209\"\"\"\nPilotBore,12.009,mm,12mm H7; emit mid\nNumBolts,4,,\n";
        assert_eq!(a, golden);
    }

    // frob:tests crates/cadetta-adapter/src/builtin/csv.rs::read kind=unit
    #[test]
    fn read_round_trips_values_and_quoting() {
        let text = emit(&header("blake3:aa"), &rows());
        let units = UnitPolicy {
            native_unit: Some("mm".into()),
            assume_unit: None,
        };
        let got = read(&text, &units).unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].observed.literal, "6.35");
        assert_eq!(got[0].observed.unit.as_deref(), Some("mm"));
        assert!(!got[0].observed.unit_assumed);
        assert_eq!(got[1].observed.literal, "12.009");
        assert_eq!(got[2].observed.literal, "4");
        assert!(got[2].observed.unit_assumed);
        assert_eq!(parse(&text).unwrap(), rows());
    }

    // frob:tests crates/cadetta-adapter/src/builtin/csv.rs::same_modulo_header kind=unit
    #[test]
    fn header_is_excluded_from_comparison() {
        let a = emit(&header("blake3:aa"), &rows());
        let b = emit(&header("blake3:cc"), &rows());
        assert_ne!(a, b);
        assert!(same_modulo_header(&a, &b).unwrap());
        let mut changed = rows();
        changed[0].value = "7".into();
        assert!(!same_modulo_header(&a, &emit(&header("blake3:aa"), &changed)).unwrap());
    }

    // frob:tests crates/cadetta-adapter/src/builtin/csv.rs::row_from_request kind=unit
    #[test]
    fn inexact_is_q004() {
        let e = row_from_request("x", &req(1, 3, "mm"), 6, "").unwrap_err();
        assert!(matches!(e, CsvError::Policy(PolicyError::Inexact { .. })));
    }

    // frob:tests crates/cadetta-adapter/src/builtin/csv.rs::read kind=unit
    #[test]
    fn malformed_input_is_rejected() {
        let u = UnitPolicy::default();
        assert!(matches!(read("a,b\n1,2\n", &u), Err(CsvError::Parse(_))));
        assert!(matches!(
            read("name,value,unit,note\nx,1\n", &u),
            Err(CsvError::Parse(_))
        ));
        assert!(matches!(
            read("name,value,unit,note\n\"x,1,mm,\n", &u),
            Err(CsvError::Parse(_))
        ));
        let crlf = read("name,value,unit,note\r\nx,1,mm,\r\n", &u).unwrap();
        assert_eq!(crlf[0].observed.literal, "1");
    }

    // frob:tests crates/cadetta-adapter/src/builtin/csv.rs::read_bound kind=unit
    #[test]
    fn unbound_row_is_r016_and_yields_nothing() {
        let u = UnitPolicy::default();
        let text = "name,value,unit,note\nB,5,mm,\nZ,7,mm,\n";
        let ok = read_bound(text, &u, "f.csv", &["B", "Z"]).unwrap();
        assert_eq!(ok.len(), 2);
        let e = read_bound(text, &u, "f.csv", &["B"]).unwrap_err();
        assert_eq!(
            e,
            CsvError::UnboundRow {
                name: "Z".into(),
                file: "f.csv".into()
            }
        );
        assert!(e.to_string().starts_with("R016"));
    }

    // frob:tests crates/cadetta-adapter/src/builtin/csv.rs::read kind=unit
    #[test]
    fn hash_comment_only_at_record_start() {
        let u = UnitPolicy::default();
        let text = "# hdr\nname,value,unit,note\n# skipped,1,mm,\nA#b,1,mm,\"x\n#kept\"\n";
        let got = read(text, &u).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "A#b");
        assert_eq!(parse(text).unwrap()[0].note, "x\n#kept");
    }
}
