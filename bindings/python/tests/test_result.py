"""Result contract tests (bindings.md 3)."""

# frob:ticket 01M4SWP8D2254TD7A8558R6ZY3

from cadetta.result import report_from_exit
from pydantic import BaseModel
from typani import Err, Ok


class _Report(BaseModel):
    """A stand-in check report."""

    diagnostics: list[str]


def test_error_diagnostics_are_ok_report() -> None:
    """Given error diagnostics (exit 1), when check runs, then Ok(report)."""
    # frob:tests bindings/python/python/cadetta/result.py::report_from_exit kind=unit
    out = report_from_exit(_Report, "check", 1, '{"diagnostics": ["E0001"]}', "", str)
    assert isinstance(out, Ok)
    assert out.danger_ok.diagnostics == ["E0001"]


def test_usage_exit_is_err() -> None:
    """Exit 2 (usage) is Err carrying stderr."""
    # frob:tests bindings/python/python/cadetta/result.py::report_from_exit kind=unit
    out = report_from_exit(_Report, "check", 2, "", "bad flag\n", str)
    assert isinstance(out, Err)
    assert out.danger_err == "bad flag"


def test_unreadable_output_is_err() -> None:
    """Non-JSON stdout on a result exit is Err."""
    # frob:tests bindings/python/python/cadetta/result.py::report_from_exit kind=unit
    assert isinstance(report_from_exit(_Report, "check", 0, "nope", "", str), Err)
