"""Type stubs for the Rust extension module (bindings/python)."""

from collections.abc import Callable
from fractions import Fraction

def run_cli(argv: list[str]) -> int:
    """Run the cadetta command line; returns the exit code."""

def core_version() -> str:
    """The Rust core's version string, e.g. `cadetta 0.1.0`."""

def install_log_bridge(callback: Callable[[int, str, str], object]) -> bool:
    """Bridge tracing events to callback(level, target, message); False if taken."""

class QuantityError(Exception):
    """A quantity operation violated the kind, dimension or parse rules."""

    code: str | None

class Quantity:
    """An exact magnitude with dimension, kind and optional tolerance."""

    @staticmethod
    def _parse(text: str) -> Quantity:
        """Parse source syntax; raises QuantityError."""
    @staticmethod
    def _from_ir(text: str) -> Quantity:
        """Build from an IR Value JSON object; raises QuantityError."""
    @property
    def exact(self) -> bool:
        """True when the magnitude is exact (not marked inexact)."""
    @property
    def kind(self) -> str | None:
        """The kind name, or None for an anonymous compound kind."""
    @property
    def dim(self) -> list[int]:
        """SI base-dimension exponents."""
    @property
    def angle(self) -> int:
        """The angle exponent."""
    @property
    def role(self) -> str | None:
        """The mmc/lmc role, if set."""
    @property
    def nominal(self) -> Quantity:
        """Untoleranced nominal."""
    @property
    def min(self) -> Quantity:
        """Untoleranced lower limit."""
    @property
    def max(self) -> Quantity:
        """Untoleranced upper limit."""
    @property
    def mid(self) -> Quantity:
        """Untoleranced midpoint of min and max."""
    @property
    def mmc(self) -> Quantity:
        """Maximum-material limit (needs a role)."""
    @property
    def lmc(self) -> Quantity:
        """Least-material limit (needs a role)."""
    def with_role(self, role: str) -> Quantity:
        """A copy tagged "hole" or "shaft"."""
    def as_kind(self, kind: str) -> Quantity:
        """Mirror `as`; raises QuantityError on mismatch."""
    def to_exact(self, unit: str) -> Fraction | None:
        """Nominal in unit as a Fraction, or None when inexact."""
    def to_float(self, unit: str) -> float:
        """Nominal in unit as a float."""
    def sqrt(self) -> Quantity:
        """Square root (Q011 when the dimension does not halve)."""
    def __add__(self, other: Quantity | int) -> Quantity: ...
    def __radd__(self, other: Quantity | int) -> Quantity: ...
    def __sub__(self, other: Quantity | int) -> Quantity: ...
    def __rsub__(self, other: Quantity | int) -> Quantity: ...
    def __mul__(self, other: Quantity | int) -> Quantity: ...
    def __rmul__(self, other: Quantity | int) -> Quantity: ...
    def __truediv__(self, other: Quantity | int) -> Quantity: ...
    def __rtruediv__(self, other: Quantity | int) -> Quantity: ...
    def __neg__(self) -> Quantity: ...
    def __pow__(self, exp: int) -> Quantity: ...
