"""Mutation and IO verbs as typed Results over the CLI (bindings.md 3.2)."""

# frob:ticket 01M4SWP896V9Y8K5H3EJKWR1T0

from __future__ import annotations

from typing import TYPE_CHECKING

from pydantic import BaseModel, ConfigDict
from typani import Err, Ok, Result

from cadetta.logging import get_logger
from cadetta.models import ack, emit, import_, sync, update

if TYPE_CHECKING:
    from cadetta.project import Project, ProjectError

__all__ = [
    "SolveReport",
    "SyncReport",
    "ack_obligations",
    "emit_adapters",
    "import_adapter",
    "solve_project",
    "sync_project",
    "update_lock",
]

_log = get_logger(__name__)


class SolveReport(BaseModel):
    """What `solve` printed: solver paths run, found fresh, and the raw text.

    The CLI has no solve-report.json yet (see the solve JSON ticket), so this
    is parsed from the text lines `solved PATH [VARIANTS]` and `fresh PATH`.
    """

    model_config = ConfigDict(frozen=True)

    solved: list[str]
    fresh: list[str]
    text: str


SyncReport = sync.SyncReport
"""The generated `sync.json` model: records, diagnostics, regen, manual."""


def update_lock(
    project: Project, name: str | None = None
) -> Result[update.Update, ProjectError]:
    """Run `update [NAME]`: re-resolve dependencies and rewrite the lock."""
    verb = ["update"] + ([name] if name is not None else [])
    _log.info("update %s", name or "(all)")
    return project._model(update.Update, verb)


def emit_adapters(
    project: Project, adapters: list[str] | None = None
) -> Result[emit.EmitReport, ProjectError]:
    """Run `emit [ADAPTER...]`: write the configured adapters' outputs."""
    verb = ["emit", *(adapters or [])]
    _log.info("emit %s", adapters or "(all)")
    return project._model(emit.EmitReport, verb)


def import_adapter(
    project: Project,
    adapter: str,
    files: list[str],
    accept: bool = False,
    update: bool = False,
    write: bool = False,
) -> Result[import_.ImportReport, ProjectError]:
    """Run `import`: propose, `--accept`, or `--update [--write]` from CAD files."""
    if accept:
        verb = ["import", "--accept"]
    else:
        verb = ["import", adapter, *files]
        if update:
            verb.append("--update")
        if write:
            verb.append("--write")
    _log.info("import %s accept=%s update=%s write=%s", adapter, accept, update, write)
    return project._model(import_.ImportReport, verb)


def ack_obligations(
    project: Project,
    selector: str,
    role: str,
    reason: str | None = None,
    baseline: bool = False,
    release: bool = False,
    dry_run: bool = False,
    as_: str | None = None,
) -> Result[ack.AckOutput, ProjectError]:
    """Run `ack SELECTOR --role R`: attest obligations at their current digests."""
    verb = ["ack", selector, "--role", role]
    if reason is not None:
        verb += ["--reason", reason]
    if baseline:
        verb.append("--baseline")
    if release:
        verb.append("--release")
    if dry_run:
        verb.append("--dry-run")
    if as_ is not None:
        verb += ["--as", as_]
    _log.info("ack %s role=%s dry_run=%s", selector, role, dry_run)
    return project._model(ack.AckOutput, verb)


def solve_project(
    project: Project, names: list[str] | None = None, variants: bool = False
) -> Result[SolveReport, ProjectError]:
    """Run `solve [GLOB...] [--variants]` and parse its text report."""
    verb = ["solve", *(names or [])]
    if variants:
        verb.append("--variants")
    ran = project._run(verb, global_format=False)
    if isinstance(ran, Err):
        return ran
    text = ran.unwrap()
    solved, fresh = [], []
    for line in text.splitlines():
        head, _, rest = line.partition(" ")
        if head == "solved":
            solved.append(rest.split(" [")[0])
        elif head == "fresh":
            fresh.append(rest)
    _log.info("solve: %d solved, %d fresh", len(solved), len(fresh))
    return Ok(SolveReport(solved=solved, fresh=fresh, text=text))


def sync_project(
    project: Project,
    adapters: list[str] | None = None,
    dry_run: bool = False,
    only: str | None = None,
    force_design: bool = False,
    no_regen: bool = False,
    module: str | None = None,
) -> Result[SyncReport, ProjectError]:
    """Run `sync [flags] [ADAPTER...]` and parse its `sync.json` report."""
    verb = ["sync"]
    if dry_run:
        verb.append("--dry-run")
    if only is not None:
        verb += ["--only", only]
    if force_design:
        verb.append("--force-design")
    if no_regen:
        verb.append("--no-regen")
    if module is not None:
        verb += ["--module", module]
    verb += adapters or []
    _log.info("sync %s", verb[1:])
    return project._model(sync.SyncReport, verb)
