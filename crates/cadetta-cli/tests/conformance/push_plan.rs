//! frob:ticket 01M4VQXMBWSFHWEVGFBBC7S5KT
//! `binding push-plan`: what the adapter host would push for a design, without a tool.
//!
//! Tolerance fixtures run `plan_tolerance` per locator and print one line per written
//! tolerance (`<locator> tolerance <form> <value>`) on `stdout` for `output` to check.

use std::fs;

use cadetta_adapter::locator::{DesignForm, DesignTolerance, TolerancePlan, plan_tolerance};
use cadetta_ir::model::{Locator, LocatorKind, LocatorMode, TolerancePolicy};
use serde_json::Value;

use super::Job;
use crate::run::Facts;

/// Run the push-plan executor on the fixture's JSON and collect the plan lines.
pub fn run(job: &Job) -> Result<Facts, String> {
    let text =
        fs::read_to_string(job.fixture).map_err(|e| format!("{}: {e}", job.fixture.display()))?;
    let spec: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let adapter = &spec["adapter"];
    let has_feature = adapter["features"]
        .as_array()
        .is_some_and(|f| f.iter().any(|v| v == "tolerances"));
    let adapter_default = policy(adapter["tolerances"].as_str())?.unwrap_or(TolerancePolicy::Drive);
    let unit = adapter["native_unit"].as_str().unwrap_or("mm");
    let locators = spec["locators"]
        .as_object()
        .ok_or("push-plan fixture lacks `locators`")?;
    let mut lines = Vec::new();
    for (path, l) in locators {
        let design_text = spec["design"][path]
            .as_str()
            .ok_or_else(|| format!("no design value for `{path}`"))?;
        let locator = Locator {
            locator: l["locator"].as_str().unwrap_or(path).to_owned(),
            kind: match l["kind"].as_str() {
                Some("dimension") => LocatorKind::Dimension,
                other => return Err(format!("push-plan: unsupported locator kind {other:?}")),
            },
            mode: if l["mode"] == "ref" {
                LocatorMode::Ref
            } else {
                LocatorMode::Value
            },
            native_unit: unit.to_owned(),
            ref_to: l["ref_to"].as_str().map(str::to_owned),
            for_alternate: None,
            config: None,
            sign: None,
            component: None,
            implicit: None,
            tolerance: policy(l["tolerance"].as_str())?,
        };
        let plan = TolerancePlan {
            adapter_default,
            has_feature,
            nominal_emit: l["emit"].as_str() != Some("max"),
            tool_has_fit: true,
        };
        let design = parse_design(design_text, unit)?;
        if let Some(tol) = plan_tolerance(&locator, &plan, &design) {
            let word = match tol {
                cadetta_adapter::rpc::DimTolerance::Symmetric { .. } => "symmetric",
                cadetta_adapter::rpc::DimTolerance::Bilateral { .. } => "bilateral",
                cadetta_adapter::rpc::DimTolerance::Limits { .. } => "limits",
                cadetta_adapter::rpc::DimTolerance::Fit { .. } => "fit",
            };
            lines.push(format!("{} tolerance {word} {tol}", locator.locator));
        }
    }
    tracing::debug!(lines = lines.len(), "push-plan: tolerance lines planned");
    Ok(Facts {
        stdout: lines.join("\n"),
        ..Facts::default()
    })
}

/// A `drive`/`ignore` word, or `None` when absent.
fn policy(word: Option<&str>) -> Result<Option<TolerancePolicy>, String> {
    match word {
        None => Ok(None),
        Some("drive") => Ok(Some(TolerancePolicy::Drive)),
        Some("ignore") => Ok(Some(TolerancePolicy::Ignore)),
        Some(other) => Err(format!("push-plan: unknown tolerance policy `{other}`")),
    }
}

/// A decimal in `unit`, printed without trailing zeros (`11.9mm`).
fn lit(v: f64, unit: &str) -> String {
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format!("{s}{unit}")
}

/// The number in a `<n><unit>` literal.
fn num(text: &str, unit: &str) -> Result<f64, String> {
    text.trim()
        .trim_start_matches('+')
        .trim_end_matches(unit)
        .parse()
        .map_err(|_| format!("push-plan: cannot read `{text}` as a `{unit}` number"))
}

/// The authored design forms the fixtures use: `N +-T`, `N [+U, -L]`, `[MIN, MAX]`, `N CLASS`.
fn parse_design(text: &str, unit: &str) -> Result<DesignTolerance, String> {
    let text = text.trim();
    if let Some(body) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        let (min, max) = body.split_once(',').ok_or("interval needs `min, max`")?;
        return Ok(DesignTolerance {
            form: DesignForm::Interval,
            min: min.trim().to_owned(),
            max: max.trim().to_owned(),
        });
    }
    let (nominal, rest) = text.split_once(' ').ok_or("design needs a tolerance")?;
    let n = num(nominal, unit)?;
    let (form, lo, hi) = if let Some(t) = rest.strip_prefix("+-") {
        let t = num(t, unit)?;
        (DesignForm::Symmetric(lit(t, unit)), n - t, n + t)
    } else if let Some(body) = rest.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let (u, l) = body.split_once(',').ok_or("bilateral needs `+u, -l`")?;
        let (u, l) = (num(u, unit)?, num(l, unit)?);
        (
            DesignForm::Bilateral(lit(u, unit), lit(l, unit)),
            n + l,
            n + u,
        )
    } else {
        let form = DesignForm::Class {
            class: rest.to_owned(),
            upper: String::new(),
            lower: String::new(),
        };
        (form, n, n)
    };
    Ok(DesignTolerance {
        form,
        min: lit(lo, unit),
        max: lit(hi, unit),
    })
}
