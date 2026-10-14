"""FreeCADAdapter: the reference external adapter (adapters.md 5, 6)."""

# frob:ticket 01M4SPWSCZ06W3EW6B1RDPEZRQ
# frob:ticket 01M4SPWSFF7Z2K6FSGSZ1N30JH
# frob:ticket 01M4SPWSJ81C9Q9BV5AYXHEWE6
# frob:ticket 01M4SPWSMZASH05AGR95PGAZKD

from __future__ import annotations

from typing import ClassVar

from cadetta.adapter import (
    PROTOCOL,
    Adapter,
    Capability,
    EmitParams,
    EmitResult,
    FileDigest,
    InitializeParams,
    InitializeResult,
    OpenParams,
    OpenResult,
    PushParams,
    PushResult,
    ReadParams,
    ReadResult,
    RegenParams,
    RegenResult,
    RpcError,
    SaveResult,
    SessionParams,
)
from cadetta.logging import get_logger
from cadetta.models import ir
from pydantic import TypeAdapter, ValidationError
from typani import Err, Ok, Result

from cadetta_freecad import __version__
from cadetta_freecad.backend import FreeCadBackend, LiveBackend, NativeDocument
from cadetta_freecad.drive import DriveSessions
from cadetta_freecad.emit import (
    SHEET_NAME,
    EmitPolicy,
    build_plan,
    digest_plan,
)
from cadetta_freecad.read import observed_from_documents

__all__ = ["NATIVE_UNITS", "PRECISION", "FreeCadAdapter"]

#: FreeCAD's internal length unit.
NATIVE_UNITS = "mm"
#: Fractional decimal digits of the Spreadsheet number format the adapter writes.
PRECISION = 6

INVALID_PARAMS = -32602
INTERNAL_ERROR = -32000

_BINDINGS = TypeAdapter(list[ir.Binding])

_log = get_logger(__name__)


class FreeCadAdapter(Adapter):
    """Serves emit, read and drive for FreeCAD behind a backend seam."""

    capabilities: ClassVar[frozenset[Capability]] = frozenset(
        {Capability.EMIT, Capability.READ, Capability.DRIVE}
    )

    def __init__(self, backend: FreeCadBackend | None = None) -> None:
        """Use `backend` for FreeCAD access; the live module when omitted."""
        self._backend: FreeCadBackend = (
            backend if backend is not None else LiveBackend()
        )
        self._drive = DriveSessions(self._backend)

    def initialize(
        self, params: InitializeParams
    ) -> Result[InitializeResult, RpcError]:
        """Report identity, the three capabilities, mm and the number precision."""
        if params.protocol != PROTOCOL:
            _log.error("unsupported protocol %r", params.protocol)
            return Err(
                RpcError(
                    code=INVALID_PARAMS,
                    message=f"unsupported protocol {params.protocol!r}; "
                    f"this adapter speaks {PROTOCOL}",
                )
            )
        _log.info("initialize from %s %s", params.client.name, params.client.version)
        match self._backend.freecad_version():
            case Ok(v):
                _log.info("FreeCAD %s", v)
            case Err(why):
                # initialize still succeeds; tool-touching methods report the failure.
                _log.warning("continuing without FreeCAD: %s", why)
        return Ok(
            InitializeResult(
                name="freecad",
                version=__version__,
                capabilities=sorted(self.capabilities),
                native_units=NATIVE_UNITS,
                precision=PRECISION,
                platforms=["linux", "windows", "macos"],
            )
        )

    def emit(self, params: EmitParams) -> Result[EmitResult, RpcError]:
        """Write the bound values as Spreadsheet aliases into the FCStd at `out`."""
        try:
            policy = EmitPolicy.model_validate(params.policy)
            bindings = _BINDINGS.validate_python(params.bindings)
        except ValidationError as e:
            _log.error("bad emit params: %s", e)
            return Err(RpcError(code=INVALID_PARAMS, message=f"bad emit params: {e}"))
        if params.design.digest is None:
            _log.error("design IR carries no digest")
            return Err(
                RpcError(code=INVALID_PARAMS, message="design IR carries no digest")
            )
        match build_plan(params.design, bindings, policy, __version__):
            case Err(diags):
                _log.error("emit withheld %s: %d diagnostics", policy.out, len(diags))
                return Ok(EmitResult(files=[], diagnostics=diags))
            case Ok(plan):
                pass
        match self._backend.write_sheet(policy.out, SHEET_NAME, plan.cells):
            case Err(why):
                return Err(RpcError(code=INTERNAL_ERROR, message=why))
            case Ok(_):
                pass
        digest = digest_plan(plan)
        _log.info("emitted %s (%s)", policy.out, digest)
        return Ok(EmitResult(files=[FileDigest(path=policy.out, digest=digest)]))

    def read(self, params: ReadParams) -> Result[ReadResult, RpcError]:
        """Read aliases and dimensions of the given FCStd files into observed IR."""
        files = params.source.files
        if files is None:
            handle = params.source.session
            files = self._drive.files(handle) if handle is not None else None
            if files is None:
                _log.error("read: no files and unknown session %r", handle)
                return Err(
                    RpcError(
                        code=INVALID_PARAMS,
                        message=f"read needs files or a known session: {handle!r}",
                    )
                )
        docs: list[NativeDocument] = []
        for path in files:
            match self._backend.read_document(path):
                case Err(why):
                    return Err(RpcError(code=INTERNAL_ERROR, message=why))
                case Ok(doc):
                    docs.append(doc)
        observed, diags = observed_from_documents(docs, native_unit=NATIVE_UNITS)
        _log.info("read %d files into %s", len(docs), observed.package.name)
        return Ok(ReadResult(observed=observed, diagnostics=diags))

    def open(self, params: OpenParams) -> Result[OpenResult, RpcError]:
        """Open the files hidden (or attach); R009 for dirty or read-only ones."""
        match self._drive.open(params.files):
            case str() as why:
                return Err(RpcError(code=INTERNAL_ERROR, message=why))
            case (session, diags):
                return Ok(OpenResult(session=session, diagnostics=diags))

    def push(self, params: PushParams) -> Result[PushResult, RpcError]:
        """Write the changed params of one file, snapshotting old values first."""
        match self._drive.push(params.session, params.file, params.params):
            case str() as why:
                return Err(RpcError(code=INVALID_PARAMS, message=why))
            case (statuses, diags):
                return Ok(PushResult(statuses=statuses, diagnostics=diags))

    def regen(self, params: RegenParams) -> Result[RegenResult, RpcError]:
        """Recompute targets; a rebuild failure is R004 and restores the snapshot."""
        match self._drive.regen(params.session, params.targets):
            case str() as why:
                return Err(RpcError(code=INVALID_PARAMS, message=why))
            case (targets, diags):
                return Ok(RegenResult(targets=targets, diagnostics=diags))

    def save(self, params: SessionParams) -> Result[SaveResult, RpcError]:
        """Save modified documents whose rebuild succeeded; report their digests."""
        match self._drive.save(params.session):
            case str() as why:
                return Err(RpcError(code=INTERNAL_ERROR, message=why))
            case (files, diags):
                return Ok(SaveResult(files=files, diagnostics=diags))

    def close(self, params: SessionParams) -> Result[None, RpcError]:
        """Close the session, closing only documents the adapter opened itself."""
        why = self._drive.close(params.session)
        if why is not None:
            return Err(RpcError(code=INTERNAL_ERROR, message=why))
        return Ok(None)
