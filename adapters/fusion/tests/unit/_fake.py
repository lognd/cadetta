"""A fake Fusion session that records a transcript, for CI without Fusion."""

from __future__ import annotations

from typani import Err, Ok, Result

from cadetta_fusion.session import DocState, NativeParam, ParamEdit


def param(name: str, expression: str, unit: str = "mm") -> NativeParam:
    """Build a user parameter."""
    return NativeParam(name=name, expression=expression, unit=unit, value=0.0)


class FakeSession:
    """In-memory Fusion: documents hold parameters; every call is logged."""

    def __init__(self, docs: dict[str, list[NativeParam]] | None = None) -> None:
        self.docs = docs or {}
        self.state: dict[str, DocState] = {}
        self.transcript: list[tuple[str, ...]] = []
        self.fail_apply = False

    def version(self) -> Result[str, str]:
        self.transcript.append(("version",))
        return Ok("2.0.fake")

    def open_document(self, path: str) -> Result[DocState, str]:
        self.transcript.append(("open", path))
        if path not in self.docs:
            return Err(f"no such design {path}")
        doc = self.state.setdefault(
            path, DocState(path=path, dirty=False, readonly=False, attached=False)
        )
        return Ok(doc)

    def parameters(self, path: str) -> Result[list[NativeParam], str]:
        self.transcript.append(("parameters", path))
        return Ok(list(self.docs[path]))

    def apply(self, path: str, edits: list[ParamEdit]) -> Result[None, str]:
        self.transcript.append(("apply", path, *[e.name for e in edits]))
        if self.fail_apply:
            return Err("expression error")
        by_name = {e.name: e.expression for e in edits}
        self.docs[path] = [
            p.model_copy(update={"expression": by_name.get(p.name, p.expression)})
            for p in self.docs[path]
        ]
        return Ok(None)

    def save_document(self, path: str) -> Result[None, str]:
        self.transcript.append(("save", path))
        return Ok(None)

    def close_document(self, path: str) -> Result[None, str]:
        self.transcript.append(("close", path))
        return Ok(None)
