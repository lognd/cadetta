"""Drive: sessions, atomic push, regen check and save (fusion.md 5, 6)."""

# frob:ticket 01M555TVRF47KEJ406NPQMSJDX

from __future__ import annotations

from dataclasses import dataclass, field
from decimal import Decimal
from pathlib import Path

from blake3 import blake3
from cadetta.adapter import (
    ChangedParam,
    Diagnostic,
    FileDigest,
    FileState,
    LocatorStatus,
    PushMode,
    PushStatus,
    RegenState,
    Session,
    TargetStatus,
)
from cadetta.logging import get_logger
from typani import Err, Ok

from cadetta_fusion.read import LITERAL_RE
from cadetta_fusion.session import DocState, FusionSession, NativeParam, ParamEdit

__all__ = ["DriveSessions", "SessionState"]

_log = get_logger(__name__)


def _diag(
    code: str, severity: str, message: str, path: str | None = None
) -> Diagnostic:
    """Build a diagnostic record."""
    return Diagnostic(code=code, severity=severity, message=message, path=path)


@dataclass
class SessionState:
    """One open session: its documents and which files were modified or failed."""

    docs: dict[str, DocState]
    modified: set[str] = field(default_factory=set)
    failed: set[str] = field(default_factory=set)


def _expression(change: ChangedParam, old: NativeParam, names: set[str]) -> str:
    """Return the expression to assign for `change`, or raise ValueError why not."""
    if change.mode is PushMode.REF and change.ref_to:
        if change.ref_to not in names:
            raise ValueError(f"ref target {change.ref_to!r} is not a user parameter")
        return change.ref_to
    raw = change.value
    if isinstance(raw, bool) or not isinstance(raw, (int, float, str)):
        raise ValueError(f"value {raw!r} is not a number or text")
    text = raw.strip() if isinstance(raw, str) else format(Decimal(repr(raw)), "f")
    m = LITERAL_RE.match(text)
    if m is None:
        raise ValueError(f"value {raw!r} is not a literal number with optional unit")
    unit = m.group(2) or old.unit
    return f"{m.group(1)} {unit}".strip()


def _canon(expression: str) -> str:
    """Whitespace-free form of an expression, for no-change detection."""
    return "".join(expression.split())


class DriveSessions:
    """Holds sessions and runs open, push, regen, save and close on a session."""

    def __init__(self, fusion: FusionSession) -> None:
        """Drive Fusion through `fusion`."""
        self._fusion = fusion
        self._sessions: dict[str, SessionState] = {}
        self._next = 1

    def files(self, handle: str) -> list[str] | None:
        """Return the file paths of session `handle`, or None when unknown."""
        state = self._sessions.get(handle)
        return None if state is None else list(state.docs)

    def open(self, files: list[str]) -> tuple[Session, list[Diagnostic]] | str:
        """Open `files`; R009 for a dirty or read-only one; Err text on failure."""
        docs: dict[str, DocState] = {}
        diags: list[Diagnostic] = []
        for path in files:
            match self._fusion.open_document(path):
                case Err(why):
                    self._release(docs)
                    return why
                case Ok(state):
                    docs[path] = state
            if state.dirty or state.readonly:
                why = "unsaved changes" if state.dirty else "read-only"
                _log.warning("refusing %s: %s", path, why)
                diags.append(
                    _diag("R009", "error", f"{path} is {why}; refused before any push")
                )
        handle = f"s{self._next}"
        self._next += 1
        self._sessions[handle] = SessionState(docs=docs)
        _log.info("session %s opened with %d files", handle, len(docs))
        session = Session(
            handle=handle,
            files=[
                FileState(path=p, dirty=d.dirty, readonly=d.readonly)
                for p, d in docs.items()
            ],
        )
        return session, diags

    def push(
        self, handle: str, file: str, params: list[ChangedParam]
    ) -> tuple[list[LocatorStatus], list[Diagnostic]] | str:
        """Validate every change, then apply the valid ones atomically."""
        state = self._sessions.get(handle)
        if state is None or file not in state.docs:
            return f"{file} is not in session {handle}"
        doc = state.docs[file]
        if doc.dirty or doc.readonly:
            why = "unsaved changes" if doc.dirty else "read-only"
            _log.warning("push refused for %s: %s", file, why)
            statuses = [
                LocatorStatus(locator=c.locator, status=PushStatus.REJECTED)
                for c in params
            ]
            return statuses, [_diag("R009", "error", f"{file} is {why}; not pushed")]
        match self._fusion.parameters(file):
            case Err(why):
                return why
            case Ok(listed):
                native = {p.name: p for p in listed}
        statuses: dict[str, LocatorStatus] = {}
        diags: list[Diagnostic] = []
        edits: list[ParamEdit] = []
        olds: dict[str, NativeParam] = {}
        for change in params:
            loc = change.locator
            old = native.get(loc)
            if old is None:
                _log.warning("locator %s missing in %s", loc, file)
                diags.append(
                    _diag("R003", "error", f"{loc} not found in {file}", change.path)
                )
                statuses[loc] = LocatorStatus(locator=loc, status=PushStatus.SKIPPED)
                continue
            try:
                new = _expression(change, old, set(native))
            except ValueError as e:
                diags.append(_diag("R010", "error", f"{loc}: {e}", change.path))
                statuses[loc] = LocatorStatus(locator=loc, status=PushStatus.REJECTED)
                continue
            if _canon(new) == _canon(old.expression):
                _log.debug("locator %s unchanged", loc)
                statuses[loc] = LocatorStatus(locator=loc, status=PushStatus.SKIPPED)
                continue
            if LITERAL_RE.match(old.expression) is None:
                diags.append(
                    _diag(
                        "R006",
                        "warning",
                        f"{loc} replaces native expression {old.expression!r}",
                        change.path,
                    )
                )
            edits.append(ParamEdit(name=loc, expression=new))
            olds[loc] = old
        if edits:
            self._apply(state, file, edits, olds, statuses, diags)
        return [statuses[c.locator] for c in params], diags

    def _apply(
        self,
        state: SessionState,
        file: str,
        edits: list[ParamEdit],
        olds: dict[str, NativeParam],
        statuses: dict[str, LocatorStatus],
        diags: list[Diagnostic],
    ) -> None:
        """Apply `edits` in one atomic call and record every outcome."""
        match self._fusion.apply(file, edits):
            case Err(why):
                _log.error("atomic apply to %s failed: %s", file, why)
                diags.append(
                    _diag(
                        "R004",
                        "error",
                        f"Fusion refused the push to {file}: {why}; "
                        "no parameter was changed",
                    )
                )
                state.failed.add(file)
                for e in edits:
                    statuses[e.name] = LocatorStatus(
                        locator=e.name, status=PushStatus.FAILED
                    )
            case Ok(_):
                state.modified.add(file)
                state.failed.discard(file)
                for e in edits:
                    _log.info("pushed %s in %s", e.name, file)
                    statuses[e.name] = LocatorStatus(
                        locator=e.name,
                        status=PushStatus.PUSHED,
                        old=olds[e.name].expression,
                        new=e.expression,
                    )

    def regen(
        self, handle: str, targets: list[str]
    ) -> tuple[list[TargetStatus], list[Diagnostic]] | str:
        """Confirm each target still evaluates; Fusion recomputes on every apply."""
        state = self._sessions.get(handle)
        if state is None:
            return f"unknown session {handle}"
        statuses: list[TargetStatus] = []
        diags: list[Diagnostic] = []
        for target in targets or list(state.docs):
            if target not in state.docs:
                return f"{target} is not in session {handle}"
            match self._fusion.parameters(target):
                case Ok(_):
                    statuses.append(TargetStatus(target=target, status=RegenState.OK))
                case Err(why):
                    state.failed.add(target)
                    diags.append(
                        _diag("R004", "error", f"rebuild of {target} failed: {why}")
                    )
                    statuses.append(
                        TargetStatus(target=target, status=RegenState.FAILED)
                    )
        return statuses, diags

    def save(self, handle: str) -> tuple[list[FileDigest], list[Diagnostic]] | str:
        """Save modified files that did not fail; return their digests."""
        state = self._sessions.get(handle)
        if state is None:
            return f"unknown session {handle}"
        files: list[FileDigest] = []
        for path in sorted(state.modified - state.failed):
            match self._fusion.save_document(path):
                case Err(why):
                    return why
            digest = "blake3:" + blake3(Path(path).read_bytes()).hexdigest()
            files.append(FileDigest(path=path, digest=digest))
        state.modified -= {f.path for f in files}
        _log.info("saved %d files for session %s", len(files), handle)
        return files, []

    def close(self, handle: str) -> str | None:
        """Close what the session opened and forget it; return an error text."""
        state = self._sessions.pop(handle, None)
        if state is None:
            return f"unknown session {handle}"
        return self._release(state.docs)

    def _release(self, docs: dict[str, DocState]) -> str | None:
        """Close the documents we opened (not attached ones); first error text."""
        first: str | None = None
        for path, doc in docs.items():
            if doc.attached:
                continue
            match self._fusion.close_document(path):
                case Err(why):
                    first = first or why
        return first
