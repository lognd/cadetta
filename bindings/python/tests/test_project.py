"""Project.open, evaluate and design indexing (bindings.md 3.1, 3.2)."""

# frob:ticket 01M4SV2NMPSM42YZT9MXT4YVMW

import json
import subprocess
import sys
from fractions import Fraction
from pathlib import Path

from cadetta import PathNotFound, Project, ProjectError
from cadetta.models import ir
from typani import Err, Ok

AXLE = "acme.rover::Axle.end_diameter"
SRC = (
    "model Axle {\n"
    "  end_diameter: Length = 12mm [+-0.1mm]\n"
    "  od: Length = 8mm @measured\n"
    "  l: Length = end_diameter * 2\n"
    "}\n"
)


def make_pkg(root: Path, name: str = "acme.rover") -> Path:
    """Write a one-package fixture project under root."""
    root.mkdir(parents=True, exist_ok=True)
    (root / "cadetta.toml").write_text(
        f'[package]\nname = "{name}"\nversion = "1.2.3"\n', encoding="utf-8"
    )
    (root / "src").mkdir(exist_ok=True)
    (root / "src" / "lib.cdt").write_text(SRC, encoding="utf-8")
    return root


def cli_ir(root: Path) -> dict:
    """The design IR as the CLI prints it."""
    out = subprocess.run(
        [sys.executable, "-m", "cadetta", "--format", "json", "--cwd", str(root), "ir"],
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(out.stdout)


def member_json(doc: dict, path: str) -> dict:
    """Find one member in the CLI JSON."""
    return next(m for c in doc["components"] for m in c["members"] if m["path"] == path)


def test_open_evaluate_wraps_generated_models(tmp_path: Path) -> None:
    """Open then evaluate yields Ok(Design) over generated pydantic models."""
    root = make_pkg(tmp_path / "p")
    opened = Project.open(root)
    assert isinstance(opened, Ok)
    design = opened.unwrap().evaluate()
    assert isinstance(design, Ok)
    assert isinstance(design.unwrap().model, ir.Project)


def test_member_matches_cli_json(tmp_path: Path) -> None:
    """Nominal, max, mid, kind and citation agree with the CLI ir JSON."""
    root = make_pkg(tmp_path / "p")
    doc = cli_ir(root)
    design = Project.open(root).unwrap().evaluate().unwrap()
    m = design.get(AXLE).unwrap()
    raw = member_json(doc, AXLE)
    assert m.kind == raw["kind"] == "param"
    assert m.citation is None and raw.get("citation") is None
    q = m.value
    assert q is not None and q.kind == "Length"
    assert q.nominal.to_exact("mm") == Fraction(12)
    assert q.max.to_exact("mm") == Fraction(121, 10)
    assert q.mid.to_exact("mm") == Fraction(12)
    cited = design.get("acme.rover::Axle.od").unwrap()
    assert cited.citation is not None
    raw_cite = member_json(doc, "acme.rover::Axle.od")["citation"]
    assert cited.citation.model_dump(exclude_none=True, mode="json") == raw_cite


def test_unknown_path_is_err_not_keyerror(tmp_path: Path) -> None:
    """An unknown path returns Err(PathNotFound)."""
    design = Project.open(make_pkg(tmp_path / "p")).unwrap().evaluate().unwrap()
    got = design.get("acme.rover::Axle.nope")
    assert isinstance(got, Err)
    assert isinstance(got.err, PathNotFound)


def test_root_discovery_walks_up(tmp_path: Path) -> None:
    """A path inside src finds the package root."""
    root = make_pkg(tmp_path / "p")
    got = Project.open(root / "src").unwrap()
    assert got.root == root.resolve()


def test_root_discovery_config_file(tmp_path: Path) -> None:
    """config= starts the walk at the file's directory."""
    root = make_pkg(tmp_path / "p")
    got = Project.open(None, root / "cadetta.toml").unwrap()
    assert got.root == root.resolve()


def test_root_discovery_none_found(tmp_path: Path) -> None:
    """No cadetta.toml project gives Err(ProjectError)."""
    bare = tmp_path / "bare"
    bare.mkdir()
    got = Project.open(bare)
    assert isinstance(got, Err)
    assert isinstance(got.err, ProjectError)


def test_root_discovery_workspace(tmp_path: Path) -> None:
    """A workspace root has no package; a member scopes to itself."""
    ws = tmp_path / "ws"
    ws.mkdir()
    (ws / "cadetta.toml").write_text('[workspace]\nmembers = ["a"]\n', encoding="utf-8")
    make_pkg(ws / "a", "acme.a")
    top = Project.open(ws).unwrap()
    assert top.is_workspace and top.package is None and top.modules == []
    member = Project.open(ws / "a" / "src").unwrap()
    assert member.root == (ws / "a").resolve()
    assert member.package.name == "acme.a"


def test_package_and_modules(tmp_path: Path) -> None:
    """Package id and sorted module paths come from the manifest and src."""
    root = make_pkg(tmp_path / "p")
    (root / "src" / "frame.cdt").write_text("", encoding="utf-8")
    (root / "src" / "drive").mkdir()
    (root / "src" / "drive" / "motor.cdt").write_text("", encoding="utf-8")
    proj = Project.open(root).unwrap()
    assert proj.package.name == "acme.rover"
    assert proj.package.version == "1.2.3"
    assert proj.modules == [
        "acme.rover",
        "acme.rover::drive",
        "acme.rover::drive::motor",
        "acme.rover::frame",
    ]
