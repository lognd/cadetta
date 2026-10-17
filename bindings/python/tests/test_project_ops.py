"""Mutation and IO verbs return typed Results (bindings.md 3.2)."""

# frob:ticket 01M4SWP896V9Y8K5H3EJKWR1T0

import json
import subprocess
from pathlib import Path

import pytest
from cadetta import Project, ProjectError, project_ops
from cadetta.models import ack, emit, import_, sync, update
from typani import Err, Ok


def make_pkg(root: Path) -> Project:
    """Write a minimal package and open it."""
    (root / "src").mkdir(parents=True)
    (root / "cadetta.toml").write_text(
        '[package]\nname = "acme.rover"\nversion = "1.2.3"\n', encoding="utf-8"
    )
    (root / "src" / "lib.cdt").write_text("model A {\n  x: Length = 1mm\n}\n", "utf-8")
    return Project.open(root).unwrap()


class Spy:
    """Record the verb argv and answer with canned output."""

    def __init__(self, stdout: str, code: int = 0) -> None:
        """Hold the canned stdout and exit code."""
        self.stdout, self.code, self.verbs = stdout, code, []

    def __call__(self, project, verb, *, global_format):  # noqa: ANN001
        """Stand in for Project._spawn."""
        self.verbs.append(verb)
        return subprocess.CompletedProcess(verb, self.code, self.stdout, "")


def spy(monkeypatch: pytest.MonkeyPatch, payload: object, code: int = 0) -> Spy:
    """Patch Project._spawn with a Spy answering `payload` (JSON or text)."""
    out = payload if isinstance(payload, str) else json.dumps(payload)
    s = Spy(out, code)
    monkeypatch.setattr(Project, "_spawn", lambda self, v, **kw: s(self, v, **kw))
    return s


# frob:tests bindings/python/python/cadetta/project_ops.py::update_lock
def test_update_all_and_named(tmp_path: Path, monkeypatch) -> None:
    """update(name=None) runs bare `update` and returns the update model."""
    p = make_pkg(tmp_path)
    s = spy(
        monkeypatch,
        {"schema": "cadetta.update/1", "packages": [], "removed": [], "wrote": False},
    )
    got = p.update()
    assert isinstance(got, Ok) and isinstance(got.unwrap(), update.Update)
    assert p.update("dep").is_ok
    assert s.verbs == [["update"], ["update", "dep"]]


def test_update_real_cli_runs(tmp_path: Path) -> None:
    """The real CLI answers `update` on a dependency-free package."""
    got = make_pkg(tmp_path).update()
    assert isinstance(got, Ok), got
    assert got.unwrap().packages == []


# frob:tests bindings/python/python/cadetta/project_ops.py::emit_adapters
def test_emit_passes_adapters(tmp_path: Path, monkeypatch) -> None:
    """emit(adapters) forwards the adapter ids and validates the emit model."""
    p = make_pkg(tmp_path)
    s = spy(monkeypatch, {"files": [], "diagnostics": []})
    got = p.emit(["freecad"])
    assert isinstance(got.unwrap(), emit.EmitReport)
    assert s.verbs == [["emit", "freecad"]]


# frob:tests bindings/python/python/cadetta/project_ops.py::import_adapter
def test_import_modes(tmp_path: Path, monkeypatch) -> None:
    """import_ builds propose, --accept and --update --write argv."""
    p = make_pkg(tmp_path)
    s = spy(
        monkeypatch,
        {"mode": "propose", "written": [], "items": [], "unchanged": [], "notes": []},
    )
    assert isinstance(p.import_("freecad", ["a.FCStd"]).unwrap(), import_.ImportReport)
    p.import_("freecad", [], accept=True)
    p.import_("freecad", ["a.FCStd"], update=True, write=True)
    assert s.verbs == [
        ["import", "freecad", "a.FCStd"],
        ["import", "--accept"],
        ["import", "freecad", "a.FCStd", "--update", "--write"],
    ]


# frob:tests bindings/python/python/cadetta/project_ops.py::ack_obligations
def test_ack_flags(tmp_path: Path, monkeypatch) -> None:
    """ack maps every option to its CLI flag and returns AckOutput."""
    p = make_pkg(tmp_path)
    s = spy(
        monkeypatch,
        {
            "schema": "cadetta.ack/1",
            "role": "review",
            "dry_run": True,
            "written": False,
            "obligations": [],
        },
    )
    got = p.ack("ots:*", "review", "ok", release=True, dry_run=True, as_="a@b")
    assert isinstance(got.unwrap(), ack.AckOutput)
    assert s.verbs == [
        [
            "ack",
            "ots:*",
            "--role",
            "review",
            "--reason",
            "ok",
            "--release",
            "--dry-run",
            "--as",
            "a@b",
        ]
    ]


def test_ack_usage_error_is_err(tmp_path: Path) -> None:
    """An undeclared role is a usage error, so Err(ProjectError)."""
    got = make_pkg(tmp_path).ack("nothing:*", "nobody", "r")
    assert isinstance(got, Err) and isinstance(got.unwrap_err(), ProjectError)


# frob:tests bindings/python/python/cadetta/project_ops.py::solve_project
def test_solve_parses_text(tmp_path: Path, monkeypatch) -> None:
    """solve parses `solved` and `fresh` lines into a SolveReport."""
    p = make_pkg(tmp_path)
    s = spy(monkeypatch, "solved a::S [x=y]\nfresh a::T\n")
    got = p.solve(["a::*"], variants=True).unwrap()
    assert got.solved == ["a::S"] and got.fresh == ["a::T"]
    assert s.verbs == [["solve", "a::*", "--variants"]]


def test_solve_no_solvers(tmp_path: Path) -> None:
    """With no solvers the real CLI reports an empty SolveReport."""
    got = make_pkg(tmp_path).solve()
    assert isinstance(got, Ok), got
    assert got.unwrap().solved == []


SYNC_PAYLOAD = {
    "records": [
        {
            "adapter": "freecad",
            "file": "rover.FCStd",
            "path": "acme.rover::A.x",
            "locator": "Sketch.x",
            "verb": "push",
            "old": "1mm",
            "new": "5mm",
        }
    ],
    "diagnostics": [{"code": "R004", "severity": "error", "message": "stale"}],
    "regen": [{"file": "rover.FCStd", "status": "ok"}],
    "emitted": [],
    "manual": [],
    "lock_written": True,
}


# frob:tests bindings/python/python/cadetta/project_ops.py::sync_project
def test_sync_flags_and_report(tmp_path: Path, monkeypatch) -> None:
    """sync maps options to flags and parses the canned sync.json report."""
    p = make_pkg(tmp_path)
    s = spy(monkeypatch, SYNC_PAYLOAD)
    got = p.sync()
    assert isinstance(got, Ok) and isinstance(got.unwrap(), sync.SyncReport)
    rep = got.unwrap()
    assert rep.records[0].verb == sync.Verb.push
    assert rep.records[0].new == "5mm"
    assert rep.diagnostics[0].code == "R004"
    assert rep.regen[0].status == sync.Status.ok
    assert rep.lock_written is True
    assert p.sync(
        ["freecad"],
        dry_run=True,
        only="a::*",
        force_design=True,
        no_regen=True,
        module="m",
    ).is_ok
    assert s.verbs[0] == ["sync"]
    assert s.verbs[1:] == [
        [
            "sync",
            "--dry-run",
            "--only",
            "a::*",
            "--force-design",
            "--no-regen",
            "--module",
            "m",
            "freecad",
        ]
    ]


# frob:tests bindings/python/python/cadetta/project_ops.py::sync_project
def test_sync_bad_payload_is_err(tmp_path: Path, monkeypatch) -> None:
    """A payload that fails the sync.json schema is Err, not a loose dict."""
    p = make_pkg(tmp_path)
    spy(monkeypatch, {"records": [], "diagnostics": []})
    assert isinstance(p.sync(), Err)


def test_module_exports() -> None:
    """The ops module exposes the report types."""
    assert project_ops.SolveReport and project_ops.SyncReport


def test_cli_only_verbs_have_no_method() -> None:
    """init, cite pin and the lock verbs are CLI-only (bindings.md 3.2)."""
    for name in ("init", "cite_pin", "pin", "lock", "lock_merge", "lock_verify"):
        assert not hasattr(Project, name)
