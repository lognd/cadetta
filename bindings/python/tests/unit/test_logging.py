"""Tests for cadetta.logging: formatter, filter, and the two configs."""

import logging
import tomllib
from pathlib import Path

import cadetta.logging.logger as logger_module
from cadetta.logging import get_logger, use_protocol_logging
from cadetta.logging.filter import BelowLevelFilter
from cadetta.logging.formatter import MalmbergFormatter

_DIR = Path(logger_module.__file__).parent


def _record(level: int) -> logging.LogRecord:
    return logging.LogRecord("t", level, __file__, 1, "hello", None, None)


# frob:tests bindings/python/python/cadetta/logging/formatter.py::MalmbergFormatter.format kind=unit
def test_info_has_no_level_prefix():
    assert MalmbergFormatter().format(_record(logging.INFO)) == "hello"


# frob:tests bindings/python/python/cadetta/logging/formatter.py::MalmbergFormatter.__init__ kind=unit
def test_warning_and_show_level_add_prefix():
    assert MalmbergFormatter().format(_record(logging.WARNING)) == "WARNING: hello"
    assert (
        MalmbergFormatter(show_level=True).format(_record(logging.DEBUG))
        == "DEBUG: hello"
    )


# frob:tests bindings/python/python/cadetta/logging/filter.py::BelowLevelFilter.filter kind=unit
# frob:tests bindings/python/python/cadetta/logging/filter.py::BelowLevelFilter.__init__ kind=unit
def test_below_level_filter_keeps_only_lower_levels():
    keep = BelowLevelFilter("WARNING")
    assert keep.filter(_record(logging.INFO))
    assert not keep.filter(_record(logging.WARNING))


# frob:tests bindings/python/python/cadetta/logging/logger.py::get_logger kind=unit
def test_get_logger_returns_named_logger():
    assert get_logger("cadetta.test").name == "cadetta.test"


def test_default_config_splits_stdout_and_stderr():
    handlers = tomllib.loads((_DIR / "config.toml").read_text())["handlers"]
    assert handlers["stdout"]["stream"] == "ext://sys.stdout"
    assert handlers["stderr"]["stream"] == "ext://sys.stderr"


def test_protocol_config_never_writes_stdout():
    # stdout is the protocol channel; no handler may write to it.
    handlers = tomllib.loads((_DIR / "protocol.toml").read_text())["handlers"]
    assert handlers and all(
        h["stream"] == "ext://sys.stderr" for h in handlers.values()
    )


# frob:tests bindings/python/python/cadetta/logging/logger.py::use_protocol_logging kind=unit
def test_use_protocol_logging_installs_stderr_only_root():
    use_protocol_logging()
    assert [h.name for h in logging.getLogger().handlers] == ["stderr"]
