"""Drive: sessions, push, regen with R004 restore, save (adapters.md 2.2, 2.3)."""

# frob:ticket 01M4SPWSMZASH05AGR95PGAZKD

from __future__ import annotations

import re
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

from cadetta_freecad.backend import DocState, FreeCadBackend, NativeItem

__all__ = ["DriveSessions", "SessionState"]

_NUMBER_RE = re.compile(r"^=?\s*(-?\d+(?:\.\d+)?)\s*([A-Za-z]+)?\s*$")

_log = get_logger(__name__)


def _diag(
    code: str, severity: str, message: str, path: str | None = None
) -> Diagnostic:
    """Build a diagnostic record."""
    return Diagnostic(code=code, severity=severity, message=message, path=path)


@dataclass
class SessionState:
    """One open session: its documents, snapshots and which files were modified."""

    docs: dict[str, DocState]
    snapshots: dict[str, dict[str, NativeItem]] = field(default_factory=dict)
    modified: set[str] = field(default_factory=set)
    failed: set[str] = field(default_factory=set)


def _decimal(value: float | int) -> str:
    """Plain decimal text of a number, no exponent."""
    return format(Decimal(repr(value)), "f")


def _new_item(change: ChangedParam, old: NativeItem) -> NativeItem | str:
    """Return the item to write for `change`, or why the value is unusable."""
    ref = change.mode is PushMode.REF and change.ref_to
    if old.kind == "alias":
        if ref:
            return old.model_copy(update={"content": f"={change.ref_to}"})
        raw = change.value
        if isinstance(raw, bool) or not isinstance(raw, (int, float, str)):
            return f"value {raw!r} is not a number or text"
        text = _decimal(raw) if not isinstance(raw, str) else raw.strip()
        old_unit = _NUMBER_RE.match(old.content or "")
        m = _NUMBER_RE.match(text)
        if m is not None:
            unit = m.group(2) or (old_unit.group(2) if old_unit else None)
            equals = text.startswith("=") or (old.content or "").startswith("=")
            text = ("=" if equals else "") + m.group(1) + (f" {unit}" if unit else "")
        return old.model_copy(update={"content": text})
    if ref:
        return old.model_copy(update={"expression": change.ref_to})
    raw = change.value
    if isinstance(raw, str):
        m = _NUMBER_RE.match(raw)
        if m is None:
            return f"value {raw!r} is not numeric"
        number = float(m.group(1))
    elif isinstance(raw, bool) or not isinstance(raw, (int, float)):
        return f"value {raw!r} is not numeric"
    else:
        number = float(raw)
    return old.model_copy(update={"value": number, "expression": None})


def _same(a: NativeItem, b: NativeItem) -> bool:
    """True when writing `b` over `a` would change nothing."""
    return (a.content, a.expression, a.value) == (b.content, b.expression, b.value)


class DriveSessions:
    """Holds sessions and runs open, push, regen, save and close on a backend."""

    def __init__(self, backend: FreeCadBackend) -> None:
        """Drive FreeCAD through `backend`."""
        self._backend = backend
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
            match self._backend.open_document(path):
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
        """Write changed params to `file`; snapshot first; per-locator statuses."""
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
        statuses: list[LocatorStatus] = []
        diags: list[Diagnostic] = []
        snap = state.snapshots.setdefault(file, {})
        for change in params:
            status, found = self._push_one(file, change, snap, diags)
            statuses.append(status)
            if found:
                state.modified.add(file)
        return statuses, diags

    def _push_one(
        self,
        file: str,
        change: ChangedParam,
        snap: dict[str, NativeItem],
        diags: list[Diagnostic],
    ) -> tuple[LocatorStatus, bool]:
        """Push one locator; return its status and whether the file was changed."""
        loc = change.locator
        match self._backend.get_item(file, loc):
            case Err(why):
                diags.append(_diag("R010", "error", why, change.path))
                return LocatorStatus(locator=loc, status=PushStatus.FAILED), False
            case Ok(None):
                _log.warning("locator %s missing in %s", loc, file)
                diags.append(
                    _diag("R003", "error", f"{loc} not found in {file}", change.path)
                )
                return LocatorStatus(locator=loc, status=PushStatus.SKIPPED), False
            case Ok(old):
                pass
        if old.driven:
            diags.append(
                _diag("R010", "error", f"{loc} is a driven dimension", change.path)
            )
            return LocatorStatus(locator=loc, status=PushStatus.REJECTED), False
        new = _new_item(change, old)
        if isinstance(new, str):
            diags.append(_diag("R010", "error", f"{loc}: {new}", change.path))
            return LocatorStatus(locator=loc, status=PushStatus.REJECTED), False
        if _same(old, new):
            _log.debug("locator %s unchanged", loc)
            return LocatorStatus(locator=loc, status=PushStatus.SKIPPED), False
        snap.setdefault(loc, old)
        if old.expression is not None and new.expression != old.expression:
            diags.append(
                _diag(
                    "R006",
                    "warning",
                    f"{loc} replaces native expression {old.expression!r}",
                    change.path,
                )
            )
        match self._backend.set_item(file, new):
            case Err(why):
                diags.append(_diag("R010", "error", why, change.path))
                return LocatorStatus(locator=loc, status=PushStatus.FAILED), True
        _log.info("pushed %s in %s", loc, file)
        status = LocatorStatus(
            locator=loc,
            status=PushStatus.PUSHED,
            old=_shown(old),
            new=_shown(new),
        )
        return status, True

    def regen(
        self, handle: str, targets: list[str]
    ) -> tuple[list[TargetStatus], list[Diagnostic]] | str:
        """Recompute targets (all files when empty); R004 and restore on failure."""
        state = self._sessions.get(handle)
        if state is None:
            return f"unknown session {handle}"
        names = targets or list(state.docs)
        statuses: list[TargetStatus] = []
        diags: list[Diagnostic] = []
        for target in names:
            if target not in state.docs:
                return f"{target} is not in session {handle}"
            match self._backend.recompute(target):
                case Ok(failed):
                    pass
                case Err(why):
                    failed = [why]
            if not failed:
                state.snapshots.pop(target, None)
                statuses.append(TargetStatus(target=target, status=RegenState.OK))
                continue
            _log.error("rebuild of %s failed: %s", target, failed)
            detail = "; ".join(failed)
            outcome = self._restore(state, target)
            state.failed.add(target)
            diags.append(
                _diag(
                    "R004", "error", f"rebuild of {target} failed: {detail}; {outcome}"
                )
            )
            statuses.append(TargetStatus(target=target, status=RegenState.FAILED))
        return statuses, diags

    def _restore(self, state: SessionState, target: str) -> str:
        """Put the snapshotted values back and rebuild; say how it went."""
        snap = state.snapshots.pop(target, {})
        for item in snap.values():
            match self._backend.set_item(target, item):
                case Err(why):
                    _log.error("restore of %s failed: %s", item.name, why)
                    return f"restore FAILED, document left modified: {why}"
        match self._backend.recompute(target):
            case Err(why):
                return f"restore FAILED, document left modified: {why}"
            case Ok(still) if still:
                return "restore FAILED, rebuild still fails after restoring"
        state.modified.discard(target)
        _log.info("restored %s from snapshot", target)
        return "document restored from snapshot"

    def save(self, handle: str) -> tuple[list[FileDigest], list[Diagnostic]] | str:
        """Save modified files whose rebuild succeeded; return their digests."""
        state = self._sessions.get(handle)
        if state is None:
            return f"unknown session {handle}"
        files: list[FileDigest] = []
        for path in sorted(state.modified - state.failed):
            match self._backend.save_document(path):
                case Err(why):
                    return why
            digest = "blake3:" + blake3(Path(path).read_bytes()).hexdigest()
            files.append(FileDigest(path=path, digest=digest))
            state.snapshots.pop(path, None)
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
            match self._backend.close_document(path):
                case Err(why):
                    first = first or why
        return first


def _shown(item: NativeItem) -> str | float | None:
    """The old or new value of an item as reported in a push status."""
    return item.expression or item.content if item.value is None else item.value
