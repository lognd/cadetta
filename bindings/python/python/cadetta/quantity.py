"""Parse source-syntax quantities into the Rust-backed `Quantity` (bindings.md 3.1)."""

# frob:ticket 01M4SV2NGV5P9KBBT7987BZCKQ

from typani import Err, Ok, Result

from cadetta._core import Quantity
from cadetta.errors import QuantityError
from cadetta.logging import get_logger

__all__ = ["Quantity", "QuantityError", "quantity"]

_log = get_logger(__name__)


def quantity(text: str) -> Result[Quantity, QuantityError]:
    """Parse `"12mm [+-0.1mm]"` into a Quantity; Err carries the Q/E code."""
    try:
        q = Quantity._parse(text)
    except QuantityError as exc:
        _log.debug("quantity parse failed: %s (code=%s)", exc, exc.code)
        return Err(exc)
    _log.debug("quantity parsed: %r", q)
    return Ok(q)
