"""Adapter-side driver for the session conformance fixtures (adapters.md 2.2).

Usage: `session_driver.py FIXTURE.json` prints one JSON object of observed facts.

The fixture describes a tool with some documents already open (`preopened`,
plus `document.file` when `running_instance` and the adapter is not told to
open it via `opened_by_adapter`). The driver runs the real `FreeCadAdapter`
open and close over a fake tool and reports what the adapter did to it: whether
it attached, whether it had to open a document itself (hidden), which documents
it closed and which it left open. The host cannot observe these; the adapter
decides them.
"""

# frob:ticket 01M52SAFHK1694NYSMG6NNC66V

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from cadetta.adapter import OpenParams, SessionParams
from typani import Ok

from cadetta_freecad.adapter import FreeCadAdapter
from cadetta_freecad.backend import DocState

__all__ = ["FakeTool", "drive_fixture", "main"]


class FakeTool:
    """A running tool: a set of open documents, mirroring LiveBackend attach rules."""

    def __init__(self, open_docs: set[str], dirty: bool, read_only: bool) -> None:
        """Start with `open_docs` already open."""
        self.open_docs = set(open_docs)
        self.dirty, self.read_only = dirty, read_only
        self.attached: list[str] = []
        self.opened: list[str] = []
        self.closed: list[str] = []

    def open_document(self, path: str) -> Ok[DocState]:
        """Attach to `path` when open, else open it (hidden: no view is created)."""
        if path in self.open_docs:
            self.attached.append(path)
            return Ok(
                DocState(
                    path=path, dirty=self.dirty, readonly=self.read_only, attached=True
                )
            )
        self.open_docs.add(path)
        self.opened.append(path)
        return Ok(DocState(path=path, readonly=self.read_only, attached=False))

    def close_document(self, path: str) -> Ok[None]:
        """Close `path` in the tool and record it."""
        self.open_docs.discard(path)
        self.closed.append(path)
        return Ok(None)

    def freecad_version(self) -> Ok[str]:
        """Report a fixed tool version."""
        return Ok("1.0.0")


def drive_fixture(spec: dict[str, Any]) -> dict[str, Any]:
    """Run open then close of the fixture's document through the adapter; return facts."""
    doc = spec["document"]
    file = doc["file"]
    open_docs = set(spec.get("preopened", []))
    if spec.get("running_instance") and not spec.get("opened_by_adapter"):
        open_docs.add(file)
    tool = FakeTool(open_docs, bool(doc.get("dirty")), bool(doc.get("read_only")))
    adapter = FreeCadAdapter(tool)  # type: ignore[arg-type]
    opened = adapter.open(OpenParams(files=[file])).unwrap()
    closed = adapter.close(SessionParams(session=opened.session.handle))
    if closed.is_err:
        raise SystemExit(f"close failed: {closed.danger_err}")
    facts: dict[str, Any] = {
        "diagnostics": [
            {"code": d.code, "severity": d.severity, "path": d.path}
            for d in opened.diagnostics
        ],
        "attached": bool(tool.attached),
        "started": bool(tool.opened),
        "closes_tool": bool(tool.closed),
        "closed": sorted(tool.closed),
        "left_open": sorted(tool.open_docs),
    }
    if tool.opened:
        facts["hidden"] = True
    return facts


def main(argv: list[str]) -> int:
    """Read the fixture named by `argv[0]` and print the observed facts as JSON."""
    spec = json.loads(Path(argv[0]).read_text(encoding="ascii"))
    sys.stdout.write(json.dumps(drive_fixture(spec)) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
