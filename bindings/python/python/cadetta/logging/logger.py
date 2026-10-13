"""Lazy dictConfig initialisation from a packaged TOML config."""

from __future__ import annotations

import logging
import logging.config
import tomllib
from pathlib import Path

_CONFIG_DIR = Path(__file__).parent
# Default: DEBUG/INFO to stdout, WARNING+ to stderr.
_DEFAULT_CONFIG = _CONFIG_DIR / "config.toml"
# Protocol processes (adapters, solvers) own stdout, so every record goes to stderr.
_PROTOCOL_CONFIG = _CONFIG_DIR / "protocol.toml"

_config_path = _DEFAULT_CONFIG
_initialized = False


def _init() -> None:
    """Load the selected logging config once."""
    global _initialized
    if _initialized:
        return
    with _config_path.open("rb") as f:
        logging.config.dictConfig(tomllib.load(f))
    _initialized = True


def use_protocol_logging() -> None:
    """Route every log record to stderr (for processes whose stdout is a protocol)."""
    global _config_path, _initialized
    _config_path = _PROTOCOL_CONFIG
    _initialized = False
    _init()


def get_logger(name: str) -> logging.Logger:
    """Return the logger for `name`, initialising logging on first use."""
    _init()
    return logging.getLogger(name)
