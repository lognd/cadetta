"""Rust tracing events reach Python logging with level and target kept (bindings.md 3)."""

# frob:ticket 01M4SV2P2ZX7BMWKB98CJ14FY0

import logging

import pytest
from cadetta import _core, _logging


# frob:tests bindings/python/python/cadetta/_logging.py::logger_name kind=unit
def test_logger_name_maps_rust_paths() -> None:
    """Rust `a::b` targets become dotted logger names."""
    assert _logging.logger_name("cadetta_units::dim") == "cadetta_units.dim"
    assert _logging.logger_name("_core") == "_core"


# frob:tests bindings/python/python/cadetta/_logging.py::emit kind=unit
def test_emit_keeps_level_and_target(caplog: pytest.LogCaptureFixture) -> None:
    """emit() produces a record at the given level on the target's logger."""
    with caplog.at_level(5):
        _logging.emit(30, "a::b", "careful")
        _logging.emit(5, "a::b", "fine detail")
    got = [(r.name, r.levelno, r.getMessage(), r.target) for r in caplog.records]
    assert got == [("a.b", 30, "careful", "a::b"), ("a.b", 5, "fine detail", "a::b")]


# frob:tests bindings/python/python/cadetta/_logging.py::emit kind=unit
def test_emit_respects_logger_level(caplog: pytest.LogCaptureFixture) -> None:
    """Records below the Python logger's level are dropped."""
    with caplog.at_level(logging.WARNING):
        _logging.emit(10, "a::b", "quiet")
    assert caplog.records == []


# frob:tests bindings/python/python/cadetta/_logging.py::install kind=integration
# frob:tests bindings/python/src/logging.rs::install_log_bridge kind=integration
def test_core_call_emits_debug_records(caplog: pytest.LogCaptureFixture) -> None:
    """A core call logs its tracing debug event through logging at DEBUG."""
    assert _core.install_log_bridge(_logging.emit) is False  # import installed it
    with caplog.at_level(logging.DEBUG):
        _core.run_cli(["--version"])
    hits = [r for r in caplog.records if "run_cli from Python" in r.getMessage()]
    assert hits, [r.getMessage() for r in caplog.records]
    assert hits[0].levelno == logging.DEBUG
    assert hits[0].name == "_core"
    assert hits[0].target == "_core"
    assert "argv=" in hits[0].getMessage()
