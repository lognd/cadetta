//! `std/src/fasteners/screwjoint.cdt` carries the ScrewJoint derived members (stdlib.md 3).
//!
//! The evaluator does not yet instantiate parameterized components or read table
//! rows, so the M5 hand case replays the member formulas over the std table rows
//! in exact integer micrometres and compares the result with hand-computed values.

// frob:ticket 01M4STAPCEF5EPEYX0MZDSBA7J

use cadetta_syntax::ast::{Component, FileEntry, ItemKind, MemberKind};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/fasteners/screwjoint.cdt");
const TABLES: &str = include_str!("../../../std/src/fasteners/tables.cdt");
const POLICY: &str = include_str!("../../../std/src/fasteners/policy.cdt");

/// The public members of stdlib.md 3, in that table's order.
const PUBLIC: &[&str] = &[
    "clearance_hole",
    "counterbore_dia",
    "counterbore_depth",
    "min_engagement",
    "grip",
    "screw_length",
    "screw",
    "float",
    "tap_drill",
    "thread_depth",
    "drill_depth",
    "callout",
];

/// Parses the module, asserting no diagnostics, and returns the ScrewJoint component.
fn joint() -> Component {
    let parsed = parse(SRC);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    for e in parsed.file.entries {
        if let FileEntry::Item(i) = e
            && let ItemKind::Component(c) = i.kind
            && c.name.name == "ScrewJoint"
        {
            return c;
        }
    }
    panic!("no ScrewJoint");
}

/// The name a member declares, if it declares one.
fn name_of(k: &MemberKind) -> Option<String> {
    match k {
        MemberKind::Param(p) => Some(p.name.name.clone()),
        MemberKind::Let(l) => Some(l.name.name.clone()),
        MemberKind::Part(p) => Some(p.name.name.clone()),
        _ => None,
    }
}

/// The row of `pub table <name>` in `src` whose line starts with `key =>`, split on commas.
fn row(src: &str, table: &str, key: &str) -> Vec<String> {
    let start = src.find(&format!("pub table {table}")).expect("table");
    let line = src[start..]
        .lines()
        .skip(1)
        .take_while(|l| l.trim() != "}")
        .map(str::trim)
        .find(|l| l.starts_with(key))
        .expect("row");
    line.split_once("=>")
        .unwrap()
        .1
        .split(',')
        .map(|c| c.split('[').next().unwrap().trim().to_owned())
        .collect()
}

/// Parses `5.5mm` into integer micrometres.
fn um(s: &str) -> i64 {
    let n = s.strip_suffix("mm").unwrap_or_else(|| panic!("{s}"));
    (n.parse::<f64>().unwrap() * 1000.0).round() as i64
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_screwjoint.rs::public_members_are_exactly_the_spec_list kind=unit
#[test]
fn public_members_are_exactly_the_spec_list() {
    let c = joint();
    let mut public = vec![];
    let mut private = vec![];
    for m in &c.members {
        if let Some(n) = name_of(&m.kind) {
            if m.private {
                private.push(n);
            } else {
                public.push(n);
            }
        }
    }
    let mut want: Vec<_> = PUBLIC.iter().map(|s| s.to_string()).collect();
    public.sort();
    want.sort();
    assert_eq!(public, want);
    assert!(!private.is_empty());
    for p in [
        "major",
        "pitch",
        "pocket_dia",
        "pocket_depth",
        "column",
        "drill_nominal",
    ] {
        assert!(private.contains(&p.to_owned()), "{p}");
    }
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_screwjoint.rs::derived_formulas_match_the_spec kind=unit
#[test]
fn derived_formulas_match_the_spec() {
    for line in [
        "min_engagement = policy.engagement[material].factor * major",
        "grip = clamp - counterbore_depth",
        "screw_length = policy.lengths.at_least(grip + min_engagement).key",
        "part screw[n]: Iso4762(thread, screw_length, screw_cost)",
        "priv half = (clearance_hole - major) / 2",
        "float = if nut { half + half } else { half }",
        "tap_drill = policy.drills.at_least(drill_nominal).key",
        "thread_depth = screw_length - grip + policy.bottom_clearance",
        "drill_depth = thread_depth + policy.tap_lead_in * pitch + policy.drill_point * tap_drill",
        "fmt(\"{} - 6H, {} DP; drill {} DP {}\", thread, thread_depth, tap_drill, drill_depth)",
        "Iso273[thread].medium",
        "HeadKind::ButtonHead => Iso7380Head[thread].counterbore_depth",
        "HeadKind::FlatHead => Iso10642Head[thread].counterbore_dia",
        "HeadKind::SocketHead => Iso4762Head[thread].counterbore_depth",
    ] {
        assert!(SRC.contains(line), "{line}");
    }
}

/// The M5 joint of one case, replayed over the std rows in micrometres.
struct Case {
    clearance: i64,
    cb_depth: i64,
    min_eng: i64,
    grip: i64,
    screw_len: i64,
    float: i64,
    tap_drill: i64,
    thread_depth: i64,
    drill_depth_x1000: i64,
}

/// Replays the ScrewJoint formulas for M5 with the given clamp, counterbore, factor and tap column.
fn m5(clamp: i64, counterbore: bool, factor_x10: i64, col: &str) -> Case {
    let major = um(&row(TABLES, "Thread", "MetricThread::M5")[0]);
    let pitch = um(&row(TABLES, "Thread", "MetricThread::M5")[1]);
    let clearance = um(&row(TABLES, "Iso273", "MetricThread::M5")[1]);
    let head = row(TABLES, "Iso4762Head", "MetricThread::M5");
    let cb_depth = if counterbore { um(&head[4]) } else { 0 };
    let min_eng = factor_x10 * major / 10;
    let grip = clamp - cb_depth;
    let lengths: Vec<i64> = {
        let start = TABLES.find("pub table Iso4762Len").unwrap();
        TABLES[start..]
            .lines()
            .skip(3)
            .take_while(|l| l.trim() != "}")
            .map(|l| um(l.trim()))
            .collect()
    };
    let screw_len = *lengths.iter().find(|l| **l >= grip + min_eng).unwrap();
    let idx = match col {
        "cut_75" => 0,
        "cut_65" => 1,
        _ => 2,
    };
    let tap_drill = um(&row(TABLES, "TapDrill", "MetricThread::M5")[idx]);
    let thread_depth = screw_len - grip + 1000;
    // tap_lead_in 3 pitches, drill_point 0.3 of the drill: micrometres x 1000.
    let drill_depth_x1000 = thread_depth * 1000 + 3 * pitch * 1000 + 3 * tap_drill * 100;
    Case {
        clearance,
        cb_depth,
        min_eng,
        grip,
        screw_len,
        float: (clearance - major) / 2,
        tap_drill,
        thread_depth,
        drill_depth_x1000,
    }
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_screwjoint.rs::m5_hand_cases_match_exactly kind=unit
#[test]
fn m5_hand_cases_match_exactly() {
    // The policy figures the replay hard-codes are the ones in IsoDefault.
    assert!(POLICY.contains("bottom_clearance = 1mm"));
    assert!(POLICY.contains("tap_lead_in      = 3"));
    assert!(POLICY.contains("drill_point      = 0.3"));
    // Aluminium (factor 2.0, 75% drill), 6 mm clamp, plain hole:
    // engagement 10, grip 6, need 16 -> 16 mm screw, depth 11, drill 4.2,
    // drill depth 11 + 3 * 0.8 + 0.3 * 4.2 = 14.66, float (5.5 - 5) / 2 = 0.25.
    let a = m5(6000, false, 20, "cut_75");
    assert_eq!(
        (a.clearance, a.cb_depth, a.min_eng, a.grip, a.screw_len),
        (5500, 0, 10_000, 6000, 16_000)
    );
    assert_eq!((a.float, a.tap_drill, a.thread_depth), (250, 4200, 11_000));
    assert_eq!(a.drill_depth_x1000, 14_660_000);
    // Stainless (factor 1.0, 65% drill), 10 mm clamp, counterbored (depth 5.4):
    // grip 4.6, engagement 5, need 9.6 -> 10 mm screw, depth 6.4, drill 4.3,
    // drill depth 6.4 + 2.4 + 0.3 * 4.3 = 10.09.
    let s = m5(10_000, true, 10, "cut_65");
    assert_eq!(
        (s.cb_depth, s.min_eng, s.grip, s.screw_len),
        (5400, 5000, 4600, 10_000)
    );
    assert_eq!((s.tap_drill, s.thread_depth), (4300, 6400));
    assert_eq!(s.drill_depth_x1000, 10_090_000);
}

// frob:ticket 01M4VSSDQZJ13JHVPXB3CHC33P

// frob:tests crates/cadetta-syntax/tests/std_fasteners_screwjoint.rs::pocket_members_select_the_table_by_head kind=unit
#[test]
fn pocket_members_select_the_table_by_head() {
    let c = joint();
    let when_counterbore = |name: &str| {
        c.members.iter().any(|m| {
            name_of(&m.kind).as_deref() == Some(name)
                && m.attrs.iter().any(|a| a.name.name == "when")
        })
    };
    // Rows are looked up only under counterbore = true, so a size a head standard lacks is not an error otherwise.
    assert!(when_counterbore("pocket_dia") && when_counterbore("pocket_depth"));
    for (kind, table) in [
        ("SocketHead", "Iso4762Head"),
        ("ButtonHead", "Iso7380Head"),
        ("FlatHead", "Iso10642Head"),
    ] {
        for col in ["counterbore_dia", "counterbore_depth"] {
            let arm = format!("HeadKind::{kind} => {table}[thread].{col}");
            assert_eq!(SRC.matches(&arm).count(), 1, "{arm}");
        }
    }
    assert!(SRC.contains("counterbore_dia = if counterbore { pocket_dia } else { 0mm }"));
}
