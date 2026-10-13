"""cadetta: typed, toleranced design parameters for CAD (Python interop API).

The core is Rust (`cadetta._core`); this package is the Python face of it.
The interop API is specified in docs/spec/bindings.md.
"""

from cadetta._core import core_version
from cadetta.quantity import Quantity, QuantityError, quantity

__all__ = ["Quantity", "QuantityError", "core_version", "quantity"]
