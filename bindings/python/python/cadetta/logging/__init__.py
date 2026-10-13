"""Logging setup: module loggers, with a stderr-only mode for protocol processes."""

from cadetta.logging.logger import get_logger, use_protocol_logging

__all__ = ["get_logger", "use_protocol_logging"]
