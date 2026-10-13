//! Python bindings: the `cadetta._core` extension module behind the
//! importable `cadetta` package (docs/spec/bindings.md).

use pyo3::prelude::*;

mod quantity;

/// Run the `cadetta` command line with `argv` (arguments after the
/// program name); returns the exit code. Backs the `cadetta` console script.
#[pyfunction]
fn run_cli(argv: Vec<String>) -> u8 {
    tracing::debug!(?argv, "run_cli from Python");
    cadetta::run_with_logging(
        &argv,
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    )
}

/// The version of the Rust core, e.g. `cadetta 0.1.0`.
#[pyfunction]
fn core_version() -> &'static str {
    cadetta::VERSION
}

/// The `cadetta._core` module.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;
    m.add_function(wrap_pyfunction!(core_version, m)?)?;
    quantity::register(m)?;
    Ok(())
}
