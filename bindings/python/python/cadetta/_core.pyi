"""Type stubs for the Rust extension module (bindings/python)."""

def run_cli(argv: list[str]) -> int:
    """Run the cadetta command line; returns the exit code."""

def core_version() -> str:
    """The Rust core's version string, e.g. `cadetta 0.1.0`."""
