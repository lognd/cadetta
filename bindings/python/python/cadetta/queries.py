"""Query and mutation verbs as typed Results over the CLI's JSON (bindings.md 3.2)."""

# frob:ticket 01M4SV2NRTBH99MYAY6EWBDP66

from __future__ import annotations

from typing import TYPE_CHECKING

from pydantic import BaseModel, ConfigDict
from typani import Result

from cadetta.logging import get_logger
from cadetta.models import compare, diag, impact, obligations, swap, why
from cadetta.models import impact_since as since_model

if TYPE_CHECKING:
    from cadetta.project import Project, ProjectError

__all__ = [
    "CheckReport",
    "check",
    "compare_interface",
    "impact_of",
    "impact_swap",
    "obligations_of",
    "swap_part",
    "why_path",
    "why_stale",
]

_log = get_logger(__name__)


class CheckReport(BaseModel):
    """The `check` report with each diagnostic fully typed (diag.json)."""

    model_config = ConfigDict(extra="forbid")

    diagnostics: list[diag.Diagnostic]


def check(
    project: Project,
    release: bool = False,
    as_of: str | None = None,
    module: str | None = None,
) -> Result[CheckReport, ProjectError]:
    """Run `check`; error-severity findings (exit 1) are still an Ok report."""
    verb = ["check"]
    if release:
        verb.append("--release")
    if as_of is not None:
        verb += ["--as-of", as_of]
    if module is not None:
        verb += ["--module", module]
    _log.debug("check: %s", verb)
    return project._model(CheckReport, verb)


def why_path(project: Project, path: str) -> Result[why.Why, ProjectError]:
    """Run `why PATH`: the derivation of one member."""
    return project._model(why.Why, ["why", path])


def why_stale(
    project: Project, obligation: str, role: str | None = None
) -> Result[why.Why, ProjectError]:
    """Run `why --stale OBLIGATION`: why an obligation is stale."""
    verb = ["why", "--stale", obligation]
    if role is not None:
        verb += ["--role", role]
    return project._model(why.Why, verb)


def impact_of(project: Project, path: str) -> Result[impact.ImpactOutput, ProjectError]:
    """Run `impact PATH`: the downstream dependents of a member."""
    return project._model(impact.ImpactOutput, ["impact", path])


def impact_since(
    project: Project, since: str | None = None
) -> Result[since_model.ImpactSince, ProjectError]:
    """Run `impact --since REV` (`--worktree` when None): diff two evaluations."""
    verb = ["impact", "--worktree"] if since is None else ["impact", "--since", since]
    return project._model(since_model.ImpactSince, verb)


def impact_swap(
    project: Project, slot: str, alternate: str
) -> Result[swap.Swap, ProjectError]:
    """Run `impact --swap SLOT ALT`: preview a part swap without writing."""
    return project._model(swap.Swap, ["impact", "--swap", slot, alternate])


def swap_part(
    project: Project, slot: str, alternate: str, reason: str, as_: str | None = None
) -> Result[swap.Swap, ProjectError]:
    """Run `swap SLOT ALT --reason R`: apply a part swap and report it."""
    verb = ["swap", slot, alternate, "--reason", reason]
    if as_ is not None:
        verb += ["--as", as_]
    _log.info("swap %s -> %s", slot, alternate)
    return project._model(swap.Swap, verb)


def compare_interface(
    project: Project, interface: str
) -> Result[compare.Compare, ProjectError]:
    """Run `compare IFACE`: the alternates of an interface side by side."""
    return project._model(compare.Compare, ["compare", interface])


def obligations_of(
    project: Project,
    stale: bool = False,
    role: str | None = None,
    release: bool = False,
    as_of: str | None = None,
) -> Result[obligations.ObligationsOutput, ProjectError]:
    """Run `obligations`: the obligation list, optionally only stale ones."""
    verb = ["obligations"]
    if stale:
        verb.append("--stale")
    if role is not None:
        verb += ["--role", role]
    if release:
        verb.append("--release")
    if as_of is not None:
        verb += ["--as-of", as_of]
    return project._model(obligations.ObligationsOutput, verb)
