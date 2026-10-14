//! Bridge from Rust `tracing` events to a Python callback, which the
//! `cadetta._logging` module turns into `logging` records
//! (docs/spec/bindings.md 3).
//!
//! Each event is forwarded as `(level, target, message)` with the level
//! already mapped to the stdlib `logging` numbers. Filtering is left to
//! Python (`Logger.isEnabledFor`), so the Python logger configuration is
//! the single source of truth. Subscriber setup lives in `cadetta-log`.

// frob:ticket 01M4SV2P2ZX7BMWKB98CJ14FY0

use pyo3::prelude::*;
use tracing::{Level, debug};

/// The stdlib `logging` number for a tracing level (TRACE maps below DEBUG).
fn py_level(level: &Level) -> u8 {
    match *level {
        Level::TRACE => 5,
        Level::DEBUG => 10,
        Level::INFO => 20,
        Level::WARN => 30,
        Level::ERROR => 40,
    }
}

/// Install the tracing-to-callback bridge as the global subscriber.
///
/// `callback(level: int, target: str, message: str)` receives each event.
/// Returns `False` when a global subscriber already exists (nothing is
/// changed), `True` when the bridge was installed.
#[pyfunction]
pub fn install_log_bridge(callback: Py<PyAny>) -> bool {
    let sink = Box::new(move |level: &Level, target: &str, message: &str| {
        Python::attach(|py| {
            // A failing handler must not unwind into core code, and logging
            // the failure here would recurse into this sink.
            let _ = callback.call1(py, (py_level(level), target, message));
        });
    });
    let installed = cadetta_log::init_sink(sink).is_ok();
    if installed {
        debug!("tracing bridged to Python logging");
    }
    installed
}

/// Register the logging bridge in the `_core` module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(install_log_bridge, m)?)
}
