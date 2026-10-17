"""FusionAdapter: read and drive Fusion user parameters (adapters.md 5, fusion.md)."""

# frob:ticket 01M555TVRF47KEJ406NPQMSJDX

from __future__ import annotations

from typing import ClassVar

from cadetta.adapter import (
    PROTOCOL,
    Adapter,
    Capability,
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
from typani import Err, Ok, Result

from cadetta_fusion import __version__
from cadetta_fusion.drive import DriveSessions
from cadetta_fusion.read import observed_from_parameters
from cadetta_fusion.session import FusionSession, NativeParam, connect

__all__ = ["NATIVE_UNITS", "PRECISION", "FusionAdapter"]

#: The unit the adapter reports; every pushed expression carries its own unit.
NATIVE_UNITS = "mm"
#: Fractional decimal digits Fusion keeps in a parameter expression.
PRECISION = 6

INVALID_PARAMS = -32602
INTERNAL_ERROR = -32000

_log = get_logger(__name__)


class FusionAdapter(Adapter):
    """Serves read and drive for Fusion behind a session seam."""

    capabilities: ClassVar[frozenset[Capability]] = frozenset(
        {Capability.READ, Capability.DRIVE}
    )

    def __init__(self, session: FusionSession | None = None) -> None:
        """Use `session` for Fusion; connect to the add-in on first use if omitted."""
        self._session: FusionSession | None = session
        self._drive: DriveSessions | None = None
        if session is not None:
            self._drive = DriveSessions(session)

    def _connected(self) -> Result[DriveSessions, RpcError]:
        """Return the drive state, connecting to the add-in the first time."""
        if self._drive is None:
            match connect():
                case Err(why):
                    return Err(RpcError(code=INTERNAL_ERROR, message=why))
                case Ok(rpc):
                    self._session = rpc
                    self._drive = DriveSessions(rpc)
        return Ok(self._drive)

    def initialize(
        self, params: InitializeParams
    ) -> Result[InitializeResult, RpcError]:
        """Report identity, read and drive, mm and the expression precision."""
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
        match self._connected():
            case Ok(_) if self._session is not None:
                match self._session.version():
                    case Ok(v):
                        _log.info("Fusion %s", v)
                    case Err(why):
                        _log.warning("continuing without Fusion: %s", why)
            case Err(e):
                # initialize still succeeds; tool-touching methods report the failure.
                _log.warning("continuing without Fusion: %s", e.message)
        return Ok(
            InitializeResult(
                name="fusion",
                version=__version__,
                capabilities=sorted(self.capabilities),
                native_units=NATIVE_UNITS,
                precision=PRECISION,
                platforms=["windows", "macos"],
            )
        )

    def read(self, params: ReadParams) -> Result[ReadResult, RpcError]:
        """Read the user parameters of the given designs into observed IR."""
        match self._connected():
            case Err(e):
                return Err(e)
            case Ok(drive):
                pass
        assert self._session is not None
        files = params.source.files
        if files is None:
            handle = params.source.session
            files = drive.files(handle) if handle is not None else None
            if files is None:
                _log.error("read: no files and unknown session %r", handle)
                return Err(
                    RpcError(
                        code=INVALID_PARAMS,
                        message=f"read needs files or a known session: {handle!r}",
                    )
                )
        designs: list[tuple[str, list[NativeParam]]] = []
        opened: list[str] = []
        try:
            for path in files:
                if params.source.files is not None:
                    match self._session.open_document(path):
                        case Err(why):
                            return Err(RpcError(code=INTERNAL_ERROR, message=why))
                        case Ok(state) if not state.attached:
                            opened.append(path)
                match self._session.parameters(path):
                    case Err(why):
                        return Err(RpcError(code=INTERNAL_ERROR, message=why))
                    case Ok(listed):
                        designs.append((path, listed))
        finally:
            # A read without a session closes what it opened, saving nothing.
            for path in opened:
                self._session.close_document(path)
        observed, diags = observed_from_parameters(designs)
        _log.info("read %d designs into %s", len(designs), observed.package.name)
        return Ok(ReadResult(observed=observed, diagnostics=diags))

    def open(self, params: OpenParams) -> Result[OpenResult, RpcError]:
        """Open the designs (or attach); R009 for dirty or read-only ones."""
        match self._connected():
            case Err(e):
                return Err(e)
            case Ok(drive):
                pass
        match drive.open(params.files):
            case str() as why:
                return Err(RpcError(code=INTERNAL_ERROR, message=why))
            case (session, diags):
                return Ok(OpenResult(session=session, diagnostics=diags))

    def push(self, params: PushParams) -> Result[PushResult, RpcError]:
        """Write the changed parameters of one design in one atomic apply."""
        match self._connected():
            case Err(e):
                return Err(e)
            case Ok(drive):
                pass
        match drive.push(params.session, params.file, params.params):
            case str() as why:
                return Err(RpcError(code=INVALID_PARAMS, message=why))
            case (statuses, diags):
                return Ok(PushResult(statuses=statuses, diagnostics=diags))

    def regen(self, params: RegenParams) -> Result[RegenResult, RpcError]:
        """Confirm the designs still evaluate after a push."""
        match self._connected():
            case Err(e):
                return Err(e)
            case Ok(drive):
                pass
        match drive.regen(params.session, params.targets):
            case str() as why:
                return Err(RpcError(code=INVALID_PARAMS, message=why))
            case (targets, diags):
                return Ok(RegenResult(targets=targets, diagnostics=diags))

    def save(self, params: SessionParams) -> Result[SaveResult, RpcError]:
        """Write modified designs whose push succeeded back to their archives."""
        match self._connected():
            case Err(e):
                return Err(e)
            case Ok(drive):
                pass
        match drive.save(params.session):
            case str() as why:
                return Err(RpcError(code=INTERNAL_ERROR, message=why))
            case (files, diags):
                return Ok(SaveResult(files=files, diagnostics=diags))

    def close(self, params: SessionParams) -> Result[None, RpcError]:
        """Close the session, closing only designs the adapter opened itself."""
        match self._connected():
            case Err(e):
                return Err(e)
            case Ok(drive):
                pass
        why = drive.close(params.session)
        if why is not None:
            return Err(RpcError(code=INTERNAL_ERROR, message=why))
        return Ok(None)
