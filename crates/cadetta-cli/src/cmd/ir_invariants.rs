//! frob:ticket 01M4YMHQ5BQW8T67PVT5ZFM9K6
//! The checkable invariants of ir.md 6 that the JSON Schema cannot express, run on the raw
//! document of `ir-validate`. Each function appends `<json pointer>: <message>` lines.

use std::collections::BTreeSet;

use serde_json::Value;

/// Violations of every ir.md 6 invariant in `doc`, in document order.
pub fn check(doc: &Value) -> Vec<String> {
    let mut out = Vec::new();
    walk(doc, "", &mut out);
    sorted(doc, "/modules", "modules", false, &mut out);
    sorted(doc, "/dependencies", "dependencies", true, &mut out);
    for (i, c) in arr(&doc["components"]).iter().enumerate() {
        component(doc, c, &format!("/components/{i}"), &mut out);
    }
    for (i, c) in arr(&doc["checks"]).iter().enumerate() {
        let at = format!("/checks/{i}/verdicts/per_alternate");
        for key in c["verdicts"]["per_alternate"].as_object().into_iter().flatten().map(|(k, _)| k) {
            variant_key(key, &at, &mut out);
        }
    }
    if doc["flavour"] == "observed" {
        observed(doc, &mut out);
    }
    tracing::debug!(count = out.len(), "invariant violations");
    out
}

fn arr(v: &Value) -> &[Value] {
    v.as_array().map_or(&[], Vec::as_slice)
}

/// A string-or-name list is sorted ascending (by `name` when `by_name`).
fn sorted(doc: &Value, at: &str, key: &str, by_name: bool, out: &mut Vec<String>) {
    let names: Vec<&str> = arr(&doc[key])
        .iter()
        .filter_map(|v| if by_name { v["name"].as_str() } else { v.as_str() })
        .collect();
    if names.windows(2).any(|w| w[0] > w[1]) {
        out.push(format!("{at}: not sorted"));
    }
}

/// Document-wide: exact magnitudes, values and dimensions; `[*]` only in bindings.
fn walk(v: &Value, at: &str, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            if at != "/bindings" && !at.starts_with("/bindings/") {
                for (k, x) in map {
                    if x.as_str().is_some_and(|s| s.contains("[*]")) {
                        out.push(format!("{at}/{k}: `[*]` never appears outside bindings"));
                    }
                }
            }
            if map.contains_key("r") && map.contains_key("pi") {
                exact(map, at, out);
            }
            if map.contains_key("nominal") && map.contains_key("authored") {
                if map.get("exact") == Some(&Value::Bool(false)) && !map.contains_key("reason") {
                    out.push(format!("{at}: exact = false needs a reason"));
                }
                if map.contains_key("designed") {
                    out.push(format!("{at}/designed: a Value has no designed field"));
                }
            }
            if let Some(Value::Object(cur)) = map.get("currency") {
                for (code, e) in cur {
                    let iso = code.len() == 3 && code.bytes().all(|b| b.is_ascii_uppercase());
                    if !iso || !(e.is_i64() || e.is_u64()) {
                        out.push(format!("{at}/currency/{code}: needs an uppercase ISO code and an integer exponent"));
                    }
                }
            }
            for (k, x) in map {
                walk(x, &format!("{at}/{}", k.replace('~', "~0").replace('/', "~1")), out);
            }
        }
        Value::Array(items) => items.iter().enumerate().for_each(|(i, x)| walk(x, &format!("{at}/{i}"), out)),
        _ => {}
    }
}

fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Parse `num/den` into `(num, den)`.
fn ratio(text: &str) -> Option<(i128, i128)> {
    let (n, d) = text.split_once('/')?;
    Some((n.parse().ok()?, d.parse().ok()?))
}

fn exact(map: &serde_json::Map<String, Value>, at: &str, out: &mut Vec<String>) {
    let Some((n, d)) = map["r"].as_str().and_then(ratio) else {
        out.push(format!("{at}/r: must be `num/den`"));
        return;
    };
    if d <= 0 {
        out.push(format!("{at}/r: denominator must be positive"));
    } else if gcd(n.unsigned_abs(), d.unsigned_abs()) != 1 {
        out.push(format!("{at}/r: not reduced"));
    }
}

/// A full-form path: `package::Item...` with an uppercase item name.
fn full_path(p: &str) -> bool {
    p.rsplit_once("::")
        .is_some_and(|(pkg, tail)| !pkg.is_empty() && tail.starts_with(|c: char| c.is_ascii_uppercase()))
}

fn path_at(v: &Value, at: &str, out: &mut Vec<String>) {
    match v.as_str() {
        Some(p) if full_path(p) => {}
        _ => out.push(format!("{at}: not a full-form canonical path")),
    }
}

/// `SLOT=ALT`: both sides full form, the slot side with a `/slot` step.
fn variant_key(key: &str, at: &str, out: &mut Vec<String>) {
    let ok = key.split_once('=').is_some_and(|(slot, alt)| {
        full_path(slot) && full_path(alt) && slot.rsplit_once("::").is_some_and(|(_, t)| t.contains('/'))
    });
    if !ok {
        out.push(format!("{at}/{key}: variant key must be full-form SLOT=ALT with a /slot step"));
    }
}

fn component(doc: &Value, c: &Value, at: &str, out: &mut Vec<String>) {
    path_at(&c["path"], &format!("{at}/path"), out);
    if let Some(p) = c["path"].as_str() {
        let item = p.rsplit_once("::").map_or(p, |(_, t)| t);
        if item.contains(['.', '/', '[']) {
            out.push(format!("{at}/path: a component path has no member tail"));
        }
    }
    let sig = c.get("params_sig").is_some();
    for (i, m) in arr(&c["members"]).iter().enumerate() {
        member(doc, m, sig, &format!("{at}/members/{i}"), out);
    }
}

fn member(doc: &Value, m: &Value, parameterized: bool, at: &str, out: &mut Vec<String>) {
    path_at(&m["path"], &format!("{at}/path"), out);
    for (i, d) in arr(&m["deps"]).iter().enumerate() {
        path_at(d, &format!("{at}/deps/{i}"), out);
    }
    let has_deps = !arr(&m["deps"]).is_empty();
    if doc["flavour"] != "observed" {
        let derived = m["provenance"] == "derived";
        if derived != has_deps {
            out.push(format!("{at}/provenance: derived exactly when deps is non-empty"));
        }
    }
    if m["analysis"] != Value::Bool(m["kind"] == "let") {
        out.push(format!("{at}/analysis: true exactly for a let"));
    }
    if m["provenance"] == "derived" && (m.get("tol").is_some() || m["value"].get("tol").is_some()) {
        out.push(format!("{at}/tol: a derived member has no tol"));
    }
    if parameterized && m.get("value").is_some() {
        out.push(format!("{at}/value: a parameterized component's members have no value"));
    }
    if let Some(vs) = m["variants"].as_object() {
        for (key, v) in vs {
            variant_key(key, &format!("{at}/variants"), out);
            if m.get("value") == Some(v) {
                out.push(format!("{at}/variants/{key}: equals the primary value"));
            }
        }
    }
    bound(&m["bound"], &format!("{at}/bound"), out);
    tol(&m["value"]["tol"], &format!("{at}/value/tol"), out);
    tol(&m["tol"], &format!("{at}/tol"), out);
}

fn bound(b: &Value, at: &str, out: &mut Vec<String>) {
    let Some(op) = b["op"].as_str() else { return };
    let (lo, hi) = (b.get("lo").is_some(), b.get("hi").is_some());
    let ok = match op {
        "<=" => hi && !lo,
        ">=" => lo && !hi,
        _ => lo && hi,
    };
    if !ok {
        out.push(format!("{at}: `{op}` carries {}", match op { "<=" => "hi only", ">=" => "lo only", _ => "both lo and hi" }));
    }
}

fn rat(v: &Value) -> Option<(i128, i128)> {
    v["r"].as_str().and_then(ratio)
}

fn tol(t: &Value, at: &str, out: &mut Vec<String>) {
    let terms = arr(&t["terms"]);
    for (i, term) in terms.iter().enumerate() {
        let at = format!("{at}/terms/{i}");
        path_at(&term["var"], &format!("{at}/var"), out);
        if let (Some((ln, ld)), Some((hn, hd))) = (rat(&term["lo"]), rat(&term["hi"])) {
            if ld > 0 && hd > 0 && ln * hd > hn * ld {
                out.push(format!("{at}: lo must not exceed hi"));
            }
        }
    }
    for (i, term) in terms.iter().enumerate() {
        let Some(stem) = term["var"].as_str().and_then(|v| v.strip_suffix(".u")) else { continue };
        let partner = terms.iter().find(|o| o["var"].as_str() == Some(&format!("{stem}.v")));
        match partner {
            Some(p) if p["lo"] == term["lo"] && p["hi"] == term["hi"] => {}
            _ => out.push(format!("{at}/terms/{i}: affine `.u` needs a `.v` partner with equal lo and hi")),
        }
    }
}

/// Observed flavour: one `<package>::Observed` component of imported, dependency-free members
/// with unique path-safe names and a binding each; no citations, parts, fits, checks, solvers.
fn observed(doc: &Value, out: &mut Vec<String>) {
    let want = format!("{}::Observed", doc["package"]["name"].as_str().unwrap_or_default());
    let comps = arr(&doc["components"]);
    if comps.len() != 1 || comps[0]["path"].as_str() != Some(want.as_str()) {
        out.push(format!("/components: observed IR has exactly one component `{want}`"));
    }
    for key in ["citations", "checks", "solvers"] {
        if !arr(&doc[key]).is_empty() {
            out.push(format!("/{key}: observed IR has none"));
        }
    }
    let bound_paths: BTreeSet<&str> = arr(&doc["bindings"])
        .iter()
        .filter(|b| !arr(&b["locators"]).is_empty())
        .filter_map(|b| b["path"].as_str())
        .collect();
    let mut seen = BTreeSet::new();
    for (ci, c) in comps.iter().enumerate() {
        for key in ["parts", "fits"] {
            if !arr(&c[key]).is_empty() {
                out.push(format!("/components/{ci}/{key}: observed IR has none"));
            }
        }
        for (i, m) in arr(&c["members"]).iter().enumerate() {
            let at = format!("/components/{ci}/members/{i}");
            if m["provenance"].get("imported").is_none() {
                out.push(format!("{at}/provenance: observed members are imported"));
            }
            if !arr(&m["deps"]).is_empty() {
                out.push(format!("{at}/deps: observed members have no deps"));
            }
            let path = m["path"].as_str().unwrap_or_default();
            let name = path.rsplit_once('.').map_or("", |(_, n)| n);
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                out.push(format!("{at}/path: member name `{name}` is not path-safe"));
            }
            if !seen.insert(path) {
                out.push(format!("{at}/path: duplicate `{path}` (collisions take `_2`, `_3`)"));
            }
            if !bound_paths.contains(path) {
                out.push(format!("{at}/path: no binding with a locator for `{path}`"));
            }
        }
    }
}
