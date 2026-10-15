//! frob:ticket 01M4YNMDMKRAP06VPQP71ZWJXV
//! `cadetta lint FILE` runs the value-reading IR-only rules over a decoded document: each case
//! edits a clean schema-valid document so exactly one family fires, and checks the clean one is
//! quiet.

use std::path::Path;

use serde_json::{Value, json};

fn base() -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/ir/d003-ir.must-not-fire.json");
    serde_json::from_str(&std::fs::read_to_string(p).expect("base")).expect("json")
}

/// Lint `doc` through the CLI; returns the exit code and the JSON output.
fn lint(name: &str, doc: &Value) -> (u8, String) {
    let dir = std::env::temp_dir().join(format!("cadetta-ir-lint-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp");
    let file = dir.join(format!("{name}.json"));
    std::fs::write(&file, doc.to_string()).expect("write");
    let args: Vec<String> = ["lint", file.to_str().unwrap(), "--format", "json"]
        .map(str::to_owned)
        .to_vec();
    let mut out = Vec::new();
    let code = cadetta::run(&args, &mut out);
    (code, String::from_utf8(out).expect("utf8"))
}

fn dim_length() -> Value {
    json!({"A":0,"K":0,"angle":0,"cd":0,"currency":{},"kg":0,"m":1,"mol":0,"s":0})
}

fn param(path: &str, authored: &str, nominal: Value) -> Value {
    json!({"analysis":false,"attrs":{},"deps":[],"kind":"param","path":path,"priv":false,
        "provenance":"authored","was":[],
        "value":{"angle":0,"authored":authored,"conservative":false,"dim":dim_length(),
                 "exact":true,"nominal":nominal}})
}

fn with_members(members: Vec<Value>, class: &str) -> Value {
    let mut doc = base();
    let c = &mut doc["components"][0];
    c["members"] = Value::Array(members);
    c["class"] = json!(class);
    doc
}

// frob:tests crates/cadetta-ir/src/decode.rs::decode_project kind=unit
#[test]
fn clean_document_is_quiet_after_full_decode() {
    let (code, out) = lint("clean", &base());
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("\"code\""), "{out}");
}

// frob:tests crates/cadetta-ir/src/decode.rs::decode_project kind=unit
#[test]
fn inexact_nominal_fires_q006() {
    let inexact = json!({"f64": 0.5, "root": "acme.demo::Axle.r"});
    let (code, out) = lint(
        "q006",
        &with_members(
            vec![param("acme.demo::Axle.r", "sqrt(2) mm", inexact)],
            "model",
        ),
    );
    assert!(out.contains("Q006"), "{code} {out}");
}

// frob:tests crates/cadetta-ir/src/decode.rs::decode_project kind=unit
#[test]
fn uncited_ots_value_fires_s001() {
    let exact = json!({"pi": 0, "r": "1/100"});
    let (_, out) = lint(
        "s001",
        &with_members(vec![param("acme.demo::Axle.r", "10mm", exact)], "ots"),
    );
    assert!(out.contains("S001"), "{out}");
}

// frob:tests crates/cadetta-ir/src/decode.rs::decode_project kind=unit
#[test]
fn equal_literals_fire_d001() {
    let exact = json!({"pi": 0, "r": "1/100"});
    let (_, out) = lint(
        "d001",
        &with_members(
            vec![
                param("acme.demo::Axle.a", "10mm", exact.clone()),
                param("acme.demo::Axle.b", "10mm", exact),
            ],
            "model",
        ),
    );
    assert!(out.contains("D001"), "{out}");
}

// frob:tests crates/cadetta-ir/src/decode.rs::decode_project kind=unit
#[test]
fn long_literal_fires_m001() {
    let exact = json!({"pi": 0, "r": "123456789/1000000000000"});
    let (_, out) = lint(
        "m001",
        &with_members(
            vec![param("acme.demo::Axle.a", "0.123456789mm", exact)],
            "model",
        ),
    );
    assert!(out.contains("M001"), "{out}");
}

fn table(cell_r: &str) -> Value {
    let cell = json!({"angle":0,"authored":"10mm","conservative":false,"dim":dim_length(),
        "exact":true,"nominal":{"pi":0,"r":cell_r}});
    json!({"path":"acme.demo::Bearing","key":{"name":"id","type":"Length"},
        "cols":[{"name":"bore","type":"Length"}],
        "rows":[{"key":cell.clone(),"cells":[cell]}]})
}

fn with_table(cell_r: &str) -> Value {
    let exact = json!({"pi": 0, "r": "1/100"});
    let mut doc = with_members(vec![param("acme.demo::Axle.a", "10mm", exact)], "model");
    doc["tables"] = json!([table(cell_r)]);
    doc
}

// frob:tests crates/cadetta-ir/src/decode.rs::decode_project kind=unit
#[test]
fn literal_equal_to_a_table_cell_fires_d002() {
    let (_, out) = lint("d002-fires", &with_table("1/100"));
    assert!(out.contains("D002"), "{out}");
    assert!(out.contains("Bearing[10mm].bore"), "{out}");
}

// frob:tests crates/cadetta-ir/src/decode.rs::decode_project kind=unit
#[test]
fn literal_unlike_every_table_cell_is_quiet_for_d002() {
    let (_, out) = lint("d002-quiet", &with_table("3/100"));
    assert!(!out.contains("D002"), "{out}");
}

fn with_fit(clearance_r: &str) -> Value {
    let mut doc = base();
    let value = json!({"angle":0,"authored":"c","conservative":false,"dim":dim_length(),
        "exact":true,"nominal":{"pi":0,"r":clearance_r}});
    doc["components"][0]["fits"] = json!([{
        "path":"acme.demo::Axle.fit","hole":"acme.demo::Axle.d","shaft":"acme.demo::Axle.r",
        "clearance":value,"type":"clearance",
        "required":{"op":"<=","hi":{"pi":0,"r":"1/1000"}}}]);
    doc
}

// frob:tests crates/cadetta-ir/src/decode.rs::decode_project kind=unit
#[test]
fn clearance_outside_its_required_range_fires_f004() {
    let (_, out) = lint("f004-fires", &with_fit("1/100"));
    assert!(out.contains("F004"), "{out}");
}

// frob:tests crates/cadetta-ir/src/decode.rs::decode_project kind=unit
#[test]
fn clearance_inside_its_required_range_is_quiet_for_f004() {
    let (_, out) = lint("f004-quiet", &with_fit("1/2000"));
    assert!(!out.contains("F004"), "{out}");
}

// frob:tests crates/cadetta-ir/src/decode.rs::decode_project kind=unit
#[test]
fn badly_cased_member_fires_n001() {
    let exact = json!({"pi": 0, "r": "1/100"});
    let (_, out) = lint(
        "n001",
        &with_members(
            vec![param("acme.demo::Axle.BadName", "10mm", exact)],
            "model",
        ),
    );
    assert!(out.contains("N001"), "{out}");
}

// frob:tests crates/cadetta-cli/src/cmd/ir_lint.rs::lint_document kind=unit
#[test]
fn span_offsets_become_line_and_column_of_the_source_beside_the_document() {
    let dir = std::env::temp_dir().join(format!("cadetta-ir-lint-span-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp");
    let src = "component Axle {\n    param BadName = 10mm\n}\n";
    std::fs::write(dir.join("axle.cdt"), src).expect("src");
    let start = src.find("BadName").expect("offset");
    let exact = json!({"pi": 0, "r": "1/100"});
    let mut member = param("acme.demo::Axle.BadName", "10mm", exact);
    member["span"] = json!({"file":"axle.cdt","start":start,"end":start + 7});
    let doc = with_members(vec![member], "model");
    let file = dir.join("doc.json");
    std::fs::write(&file, doc.to_string()).expect("write");
    let args: Vec<String> = ["lint", file.to_str().unwrap(), "--format", "json"]
        .map(str::to_owned)
        .to_vec();
    let mut out = Vec::new();
    cadetta::run(&args, &mut out);
    let out = String::from_utf8(out).expect("utf8");
    let report: Value = serde_json::from_str(&out).expect("json");
    let text = report.to_string();
    assert!(text.contains("N001"), "{out}");
    assert!(text.contains("axle.cdt"), "{out}");
    assert!(
        !text.contains("\"line\":1,"),
        "line 1 means the offset was not resolved: {out}"
    );
    assert!(text.contains("\"line\":2"), "{out}");
    assert!(text.contains("\"col\":11"), "{out}");
}
