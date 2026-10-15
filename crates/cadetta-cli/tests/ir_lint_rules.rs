//! frob:ticket 01M4YNMDMKRAP06VPQP71ZWJXV
//! `cadetta lint FILE` runs the value-reading IR-only rules over a decoded document: each case
//! edits a clean schema-valid document so exactly one family fires, and checks the clean one is
//! quiet.

use std::path::Path;

use serde_json::{Value, json};

fn base() -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/ir/d003-ir.must-not-fire.json");
    serde_json::from_str(&std::fs::read_to_string(p).expect("base")).expect("json")
}

/// Lint `doc` through the CLI; returns the exit code and the JSON output.
fn lint(name: &str, doc: &Value) -> (u8, String) {
    let dir = std::env::temp_dir().join(format!("cadetta-ir-lint-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp");
    let file = dir.join(format!("{name}.json"));
    std::fs::write(&file, doc.to_string()).expect("write");
    let args: Vec<String> = ["lint", file.to_str().unwrap(), "--format", "json"].map(str::to_owned).to_vec();
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
    let (code, out) = lint("q006", &with_members(vec![param("acme.demo::Axle.r", "sqrt(2) mm", inexact)], "model"));
    assert!(out.contains("Q006"), "{code} {out}");
}

// frob:tests crates/cadetta-ir/src/decode.rs::decode_project kind=unit
#[test]
fn uncited_ots_value_fires_s001() {
    let exact = json!({"pi": 0, "r": "1/100"});
    let (_, out) = lint("s001", &with_members(vec![param("acme.demo::Axle.r", "10mm", exact)], "ots"));
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
    let (_, out) = lint("m001", &with_members(vec![param("acme.demo::Axle.a", "0.123456789mm", exact)], "model"));
    assert!(out.contains("M001"), "{out}");
}
