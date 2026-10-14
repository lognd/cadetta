"""Regenerate cadetta.models from docs/schemas, or check they are not stale.

WHY: the models are generated from the JSON schemas the Rust core owns; a schema
change that leaves them stale breaks the binding, so CI runs `--check`.

Run with `uv run scripts/gen_models.py` (write) or `... --check` (drift gate).
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from cadetta.logging import get_logger
from pydantic import BaseModel, ConfigDict
from typani import Err, Ok, Result

# frob:ticket 01M4VKKB35X383AQA0SF2KGRGH
log = get_logger(__name__)

BINDING_DIR = Path(__file__).resolve().parent.parent
REPO_ROOT = BINDING_DIR.parent.parent
SCHEMA_DIR = REPO_ROOT / "docs" / "schemas"
MODELS_DIR = BINDING_DIR / "python" / "cadetta" / "models"

# Options recorded from ~1KGXRMC; pydantic v2 output, then ruff for import order.
CODEGEN_OPTIONS = (
    "--input-file-type",
    "jsonschema",
    "--output-model-type",
    "pydantic_v2.BaseModel",
    "--use-annotated",
    "--use-standard-collections",
    "--use-union-operator",
    "--target-python-version",
    "3.11",
    "--use-subclass-enum",
    "--formatters",
    "ruff-format",
    "--disable-timestamp",
)


class ModelSpec(BaseModel):
    """One generated module: the schema file it comes from and its module name."""

    model_config = ConfigDict(frozen=True)

    schema_file: str
    module: str


# Every generated module under cadetta.models. Add a row to model a new schema.
MODELS: tuple[ModelSpec, ...] = (
    ModelSpec(schema_file="cadetta.ir.1.json", module="ir"),
    ModelSpec(schema_file="cite-list.json", module="cite_list"),
    ModelSpec(schema_file="names.json", module="names"),
    ModelSpec(schema_file="bind.json", module="bind"),
    ModelSpec(schema_file="units.json", module="units"),
    ModelSpec(schema_file="explain.json", module="explain"),
    ModelSpec(schema_file="diag.json", module="diag"),
    ModelSpec(schema_file="check.json", module="check"),
    ModelSpec(schema_file="why.json", module="why"),
    ModelSpec(schema_file="impact.json", module="impact"),
    ModelSpec(schema_file="impact-since.json", module="impact_since"),
    ModelSpec(schema_file="swap.json", module="swap"),
    ModelSpec(schema_file="compare.json", module="compare"),
    ModelSpec(schema_file="obligations.json", module="obligations"),
    ModelSpec(schema_file="update.json", module="update"),
    ModelSpec(schema_file="ack.json", module="ack"),
    ModelSpec(schema_file="emit.json", module="emit"),
    ModelSpec(schema_file="import.json", module="import_"),
    ModelSpec(schema_file="sync.json", module="sync"),
)


class GenError(BaseModel):
    """A failed generation step, with the command and its output."""

    model_config = ConfigDict(frozen=True)

    step: str
    detail: str

    def __str__(self) -> str:
        """Render as `step: detail`."""
        return f"{self.step}: {self.detail}"


def _run(step: str, cmd: list[str]) -> Result[None, GenError]:
    """Run a subprocess, mapping a non-zero exit to a GenError."""
    log.debug("running %s: %s", step, " ".join(cmd))
    proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        log.error("%s failed with exit %d", step, proc.returncode)
        return Err(GenError(step=step, detail=(proc.stdout + proc.stderr).strip()))
    return Ok(None)


def generate(spec: ModelSpec, out_dir: Path) -> Result[Path, GenError]:
    """Generate one models module into out_dir and return its path."""
    out = out_dir / f"{spec.module}.py"
    schema = SCHEMA_DIR / spec.schema_file
    log.info("generating %s from %s", out.name, schema.name)
    codegen = [sys.executable, "-m", "datamodel_code_generator"]
    codegen += ["--input", str(schema), "--output", str(out), *CODEGEN_OPTIONS]
    ruff = [sys.executable, "-m", "ruff"]
    steps = (
        (f"datamodel-codegen {spec.module}", codegen),
        (
            f"ruff check --fix {spec.module}",
            [*ruff, "check", "--fix", "--select", "I,F401", "--quiet", str(out)],
        ),
        (f"ruff format {spec.module}", [*ruff, "format", "--quiet", str(out)]),
    )
    for step, cmd in steps:
        match _run(step, cmd):
            case Err(e):
                return Err(e)
            case Ok(_):
                pass
    return Ok(out)


def _line_length_config(out_dir: Path) -> None:
    """Copy the binding's ruff config next to the output so formatting matches."""
    shutil.copy(BINDING_DIR / "pyproject.toml", out_dir / "pyproject.toml")


def run(check: bool) -> Result[list[str], GenError]:
    """Regenerate all models (or, with check, return the names of stale files)."""
    stale: list[str] = []
    with tempfile.TemporaryDirectory(prefix="cadetta-models-") as tmp:
        tmp_dir = Path(tmp)
        _line_length_config(tmp_dir)
        for spec in MODELS:
            match generate(spec, tmp_dir):
                case Err(e):
                    return Err(e)
                case Ok(generated):
                    committed = MODELS_DIR / generated.name
                    same = (
                        committed.exists()
                        and committed.read_bytes() == generated.read_bytes()
                    )
                    if same:
                        log.info("%s is up to date", committed.name)
                    elif check:
                        log.warning("%s is stale", committed.name)
                        stale.append(str(committed.relative_to(REPO_ROOT)))
                    else:
                        shutil.copyfile(generated, committed)
                        log.info("wrote %s", committed)
    return Ok(stale)


def main(argv: list[str] | None = None) -> int:
    """CLI entry: exit 0 when done or clean, 1 when stale or failed."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check",
        action="store_true",
        help="regenerate into a temp dir and exit 1 naming each stale file",
    )
    args = parser.parse_args(argv)
    match run(args.check):
        case Err(e):
            log.error("generation failed: %s", e)
            return 1
        case Ok(stale):
            for name in stale:
                log.error("stale generated model: %s", name)
            if stale:
                log.error("run `uv run scripts/gen_models.py` and commit the result")
                return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
