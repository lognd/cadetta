# cadetta-fusion

The Fusion adapter for cadetta (`docs/spec/fusion.md`). An external adapter
speaking JSON-RPC 2.0 on stdio (stdout is protocol only; logs go to stderr),
subclassing `cadetta.adapter.Adapter`.

Fusion's Python runs only inside Fusion, so `addin/CadettaFusion` (copy it into
Fusion's AddIns folder) serves the Fusion API on `127.0.0.1:47360`
(`CADETTA_FUSION_ADDR`). The adapter reads and drives Fusion user parameters
through the `FusionSession` seam (`session.py`); tests use a fake session and
need no Fusion.

Develop: `uv sync && uv run ruff check && uv run pytest`.
`cadetta-fusion --version` prints the version.
