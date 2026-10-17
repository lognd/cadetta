# FreeCAD live round trip on nova (ticket 9KSAN8K)

Host: nova (Windows ARM64, x64 emulation). FreeCAD 1.1.4 (Revision 20260928),
`freecadcmd.exe`, embedded Python 3.11.14 (AMD64).

The cadetta wheel was built for `x86_64-pc-windows-msvc` (abi3-py311) with
maturin, installed with `adapters/freecad` into a venv made from FreeCAD's
Python, and `tests/live/run_under_freecadcmd.py` was run under freecadcmd
(`CADETTA_SITE=<venv site-packages>`, `QT_QPA_PLATFORM=offscreen`).

`live-run.log` is the transcript: both live tests pass. The round-trip test
drives emit, read, open, push, regen, save and close against a real FCStd
file and re-reads the saved file to confirm the pushed dimension changed while
the untouched one did not.

Note: the x64 build scripts fail with STATUS_DLL_INIT_FAILED in a headless
console session; run the build in a visible interactive session.
