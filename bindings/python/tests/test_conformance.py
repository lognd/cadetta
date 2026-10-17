"""Replay the conformance corpus through the binding and assert CLI parity.

Every `source` fixture in tests/conformance is staged twice (as the Rust
runner stages it): one copy is driven through the Python binding, the other
through the CLI with `--format json`. The binding result must equal the CLI
document (bindings.md 1, 3) and leave the same files behind. The `binding`
fixtures with `entry = python` are replayed against the binding API itself.
"""

# frob:ticket 01M4SV2P5W45J454JP96ZMPC6J

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tomllib
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import pytest
from cadetta import Project
from cadetta.logging import get_logger
from typani import Err

_log = get_logger(__name__)

CORPUS = Path(__file__).resolve().parents[3] / "tests" / "conformance"
SINGLE_FILE_TOML = (
    '[package]\nname = "acme.demo"\nversion = "0.1.0"\nas_of = "2026-10-13"\n\n'
    '[lint]\nfail_on = "error"\n'
)
# Entries replayed as binding-vs-CLI parity (the rest of the corpus is
# adapter-protocol or IR-schema material the binding does not wrap).
PARITY_ENTRIES = {"check", "config", "eval", "emit", "solve", "ack"}
JSON_EXITS = (0, 1)


def _cli_argv() -> list[str]:
    """The oracle CLI: `$CADETTA_CLI` when set, else the same module the binding spawns."""
    override = os.environ.get("CADETTA_CLI")
    return [override] if override else [sys.executable, "-m", "cadetta"]


def _known_gaps() -> dict[str, str]:
    """Fixtures allowed to fail their expectation: `<doc>/<NAME>` to ticket and reason."""
    doc = tomllib.loads((CORPUS / "known-gaps.toml").read_text(encoding="utf-8"))
    return {g["fixture"]: f"{g['ticket']}: {g['reason']}" for g in doc.get("gap", [])}


KNOWN_GAPS = _known_gaps()


@dataclass(frozen=True)
class Case:
    """One side (must_fire or must_not_fire) of one source fixture."""

    name: str
    side: str
    entry: str
    fixture: Path
    expect: dict[str, Any]
    lists_u002: bool = False

    @property
    def id(self) -> str:
        """The pytest id: `<doc>/<NAME>:<side>`."""
        return f"{self.name}:{self.side}"

    @property
    def gap(self) -> str | None:
        """The known-gaps reason for this fixture, if any."""
        return KNOWN_GAPS.get(self.name)


@dataclass
class Replay:
    """What the binding and the CLI each produced for one case."""

    binding: Any = None
    cli: Any = None
    binding_files: dict[str, str] = field(default_factory=dict)
    cli_files: dict[str, str] = field(default_factory=dict)
    exit: int = 0
    diagnostics: list[dict[str, Any]] = field(default_factory=list)


def _cases() -> list[Case]:
    """Every parity-replayable source fixture side in the corpus."""
    out: list[Case] = []
    for expect_file in sorted(CORPUS.rglob("*.expect.json")):
        spec = json.loads(expect_file.read_text(encoding="utf-8"))
        entry = spec.get("entry", "check")
        head = entry.split()[0]
        if spec["kind"] != "source" or head not in PARITY_ENTRIES:
            continue
        stem = expect_file.name.removesuffix(".expect.json")
        rel = expect_file.parent.relative_to(CORPUS)
        for side in ("must-fire", "must-not-fire"):
            fixture = expect_file.parent / f"{stem}.{side}"
            if not fixture.is_dir():
                fixture = fixture.with_name(fixture.name + ".cdt")
            if not fixture.exists():
                continue
            out.append(
                Case(
                    name=f"{rel.as_posix()}/{stem}",
                    side=side,
                    entry=entry,
                    fixture=fixture,
                    expect=spec[side.replace("-", "_")],
                    lists_u002=any(
                        d["code"] == "U002"
                        for blk in ("must_fire", "must_not_fire")
                        for d in spec[blk].get("diagnostics", [])
                    ),
                )
            )
    return out


CASES = _cases()


def _stage(case: Case, root: Path) -> Path:
    """Stage a fixture as a package at `root`, as the Rust runner does."""
    if case.fixture.is_dir():
        shutil.copytree(case.fixture, root)
    else:
        (root / "src").mkdir(parents=True)
        (root / "cadetta.toml").write_text(SINGLE_FILE_TOML, encoding="utf-8")
        shutil.copy(case.fixture, root / "src" / "lib.cdt")
    return root


def _snapshot(root: Path) -> dict[str, str]:
    """Every file under `root` as relative path to text."""
    return {
        p.relative_to(root).as_posix(): p.read_text(encoding="utf-8", errors="replace")
        for p in sorted(root.rglob("*"))
        if p.is_file()
    }


def _scrub(value: Any, root: Path) -> Any:
    """Replace the scratch root in every string so two roots compare equal."""
    text = json.dumps(value, sort_keys=True).replace(str(root), "<ROOT>")
    return json.loads(text)


def _adapter_id(root: Path, token: str) -> str:
    """Map an `emit TOKEN` adapter name to its configured id (as the Rust runner)."""
    table = tomllib.loads((root / "cadetta.toml").read_text(encoding="utf-8"))
    for adapter in table.get("adapter", []):
        if adapter.get("name") == token:
            return adapter.get("id", token)
    return token


def _flag(args: list[str], name: str) -> str | None:
    """The value following `name` in `args`, if present."""
    return args[args.index(name) + 1] if name in args else None


def _tokens(case: Case, root: Path) -> list[str]:
    """The CLI verb tokens for a case: the entry words plus the fixture args."""
    tokens = case.entry.split()
    if tokens[0] == "config":
        tokens = ["check"]
    if tokens[0] == "eval":
        tokens = ["ir"]
    if tokens[0] == "emit" and len(tokens) > 1:
        tokens[1] = _adapter_id(root, tokens[1])
    return tokens + list(case.expect.get("args", []))


def _binding_call(case: Case, project: Project, tokens: list[str]) -> Any:
    """Drive the binding for one case; return its Result."""
    args = [*tokens[1:]] if tokens[0] == "check" else list(case.expect.get("args", []))
    head = tokens[0]
    if head == "check":
        return project.check(
            release="--release" in args,
            as_of=_flag(args, "--as-of"),
            module=_flag(args, "--module"),
        )
    if head == "ir":
        return project.evaluate()
    if head == "emit":
        names = [t for t in tokens[1:] if not t.startswith("--")]
        names = [n for n in names if n != _flag(args, "--module")]
        module = _flag(args, "--module")
        if module is not None:
            return project._model(
                __import__("cadetta.models.emit", fromlist=["EmitReport"]).EmitReport,
                ["emit", *tokens[1:]],
            )
        return project.emit(names)
    if head == "solve":
        module = _flag(args, "--module")
        if module is not None or "--format" in args:
            # Option values (e.g. `--format text`) are not solver names; pass raw.
            return project._run(["solve", *args], global_format=False)
        names = [a for a in args if not a.startswith("--")]
        return project.solve(names or None, variants="--variants" in args)
    if head == "ack":
        return project.ack(
            args[0],
            _flag(args, "--role") or "",
            reason=_flag(args, "--reason"),
            as_=_flag(args, "--as"),
        )
    raise AssertionError(f"no binding call for {head}")


def _normalise_binding(result: Any, head: str) -> Any:
    """Reduce a binding Result to `("err", None)` or `("ok", document-or-text)`."""
    if isinstance(result, Err):
        return ("err", None)
    value = result.unwrap()
    if head == "solve":
        return ("ok", value if isinstance(value, str) else value.text)
    return ("ok", value_dump(value))


def value_dump(value: Any) -> Any:
    """A binding model as the JSON document it was parsed from."""
    target = getattr(value, "model", value)
    return target.model_dump(mode="json", by_alias=True, exclude_unset=True)


def _run_cli(
    argv: list[str], root: Path, tokens: list[str], head: str
) -> tuple[int, str]:
    """Run the CLI oracle on `root`; return exit and stdout."""
    cmd = [*argv, *([] if head == "solve" else ["--format", "json"])]
    cmd += ["--cwd", str(root), *tokens]
    done = subprocess.run(cmd, capture_output=True, text=True, check=False)
    return done.returncode, done.stdout


def _replay(case: Case, scratch: Path) -> Replay:
    """Run one case through the binding and the CLI on separate staged copies."""
    key = f"{abs(hash(case.id))}"
    b_root = _stage(case, scratch / "binding" / key)
    c_root = _stage(case, scratch / "cli" / key)
    tokens = _tokens(case, b_root)
    head = tokens[0]
    opened = Project.open(b_root)
    result = (
        opened
        if isinstance(opened, Err)
        else _binding_call(case, opened.unwrap(), tokens)
    )
    rep = Replay(exit=0)
    rep.binding = _scrub(_normalise_binding(result, head), b_root)
    code, stdout = _run_cli(_cli_argv(), c_root, tokens, head)
    rep.exit = code
    if head == "solve":
        rep.cli = _scrub(
            ("ok", stdout) if code in JSON_EXITS else ("err", None), c_root
        )
    elif code in JSON_EXITS:
        rep.cli = _scrub(("ok", json.loads(stdout)), c_root)
    else:
        rep.cli = ["err", None]
    rep.binding_files = {
        k: v.replace(str(b_root), "<ROOT>") for k, v in _snapshot(b_root).items()
    }
    rep.cli_files = {
        k: v.replace(str(c_root), "<ROOT>") for k, v in _snapshot(c_root).items()
    }
    if rep.binding[0] == "ok" and head == "check":
        # A waived diagnostic is not produced (as in the Rust runner).
        rep.diagnostics = [
            d for d in rep.binding[1].get("diagnostics", []) if not d.get("waived")
        ]
    return rep


def _exit_problems(block: dict[str, Any], exit_code: int) -> list[str]:
    """The exit-status rule of the README: explicit, else nonzero iff an error is listed."""
    if "exit" in block:
        want = block["exit"]
        return [] if want == exit_code else [f"exit {exit_code}, expected {want}"]
    wants_error = any(d["severity"] == "error" for d in block.get("diagnostics", []))
    if wants_error:
        return [] if exit_code != 0 else ["exit 0, expected nonzero"]
    return [] if exit_code == 0 else [f"exit {exit_code}, expected 0"]


def _compare(
    block: dict[str, Any], produced: list[dict[str, Any]], exit_code: int
) -> list[str]:
    """Closed-list diagnostic matching (README): errors and warnings must all be listed."""
    used = [False] * len(produced)
    problems: list[str] = []
    wanted = sorted(block.get("diagnostics", []), key=lambda d: "path" not in d)
    for want in wanted:
        for i, got in enumerate(produced):
            if (
                not used[i]
                and got["code"] == want["code"]
                and got["severity"] == want["severity"]
                and ("path" not in want or got.get("path") == want["path"])
            ):
                used[i] = True
                break
        else:
            problems.append(f"missing {want['code']} ({want['severity']})")
    problems += [
        f"extra {got['code']} ({got['severity']})"
        for i, got in enumerate(produced)
        if not used[i] and got["severity"] in ("error", "warning")
    ]
    return problems + _exit_problems(block, exit_code)


@pytest.fixture(scope="module")
def replays(
    tmp_path_factory: pytest.TempPathFactory, request: pytest.FixtureRequest
) -> Callable[[Case], Replay]:
    """Replay only the cases selected this session, once, in parallel.

    Replaying the whole corpus eagerly made a single `-k` selection cost minutes.
    """
    scratch = tmp_path_factory.mktemp("conformance")
    selected = {
        item.callspec.params["case"].id
        for item in request.session.items
        if hasattr(item, "callspec") and "case" in item.callspec.params
    }
    todo = [c for c in CASES if c.id in selected]
    _log.info("replaying %d of %d corpus cases", len(todo), len(CASES))

    def one(case: Case) -> Replay | BaseException:
        try:
            return _replay(case, scratch)
        except Exception as exc:  # noqa: BLE001 - surfaced by the asserting test
            _log.error("replay of %s failed: %s", case.id, exc)
            return exc

    with ThreadPoolExecutor(max_workers=os.cpu_count() or 4) as pool:
        done = dict(zip((c.id for c in todo), pool.map(one, todo), strict=True))

    def lookup(case: Case) -> Replay:
        got = done[case.id]
        if isinstance(got, BaseException):
            raise got
        return got

    return lookup


def test_corpus_is_discovered() -> None:
    """The corpus yields a substantial set of parity cases in every entry kind."""
    heads = {c.entry.split()[0] for c in CASES}
    assert heads == PARITY_ENTRIES
    assert len(CASES) > 500


# frob:tests bindings/python/python/cadetta/project.py::Project.check
# frob:tests bindings/python/python/cadetta/project.py::Project.evaluate
# frob:tests bindings/python/python/cadetta/project.py::Project.emit
# frob:tests bindings/python/python/cadetta/project.py::Project.solve
# frob:tests bindings/python/python/cadetta/project.py::Project.ack
@pytest.mark.parametrize("case", CASES, ids=[c.id for c in CASES])
def test_binding_equals_cli_json(case: Case, replays: Callable[[Case], Replay]) -> None:
    """The binding result and files equal the CLI `--format json` run (bindings.md 1)."""
    rep = replays(case)
    assert rep.binding == rep.cli
    assert rep.binding_files == rep.cli_files


def _problems(case: Case, rep: Replay) -> list[str]:
    """Expectation mismatches for one case, via the binding's own diagnostics."""
    if case.entry.split()[0] in ("check", "config"):
        produced = rep.diagnostics
        if not case.lists_u002:
            # Fixture components are private and unused by design (as in the Rust runner).
            produced = [d for d in produced if d["code"] != "U002"]
        return _compare(case.expect, produced, rep.exit)
    return _exit_problems(case.expect, rep.exit)


@pytest.mark.parametrize("case", CASES, ids=[c.id for c in CASES])
def test_binding_meets_fixture_expectation(
    case: Case, replays: Callable[[Case], Replay]
) -> None:
    """Diagnostics and exit status the fixture expects, observed via the binding."""
    problems = _problems(case, replays(case))
    if case.gap is not None:
        if problems:
            pytest.xfail(f"known gap {case.gap}: {problems}")
        return
    assert not problems
