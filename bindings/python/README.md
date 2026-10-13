# cadetta Python binding

The PyO3 binding (`import cadetta`, plus the `cadetta` console script). Build
and test with `uv` (Python 3.11+):

```sh
cd bindings/python
uv run maturin develop
uv run ruff check python tests scripts
uv run ruff format --check python tests scripts
uv run pytest
```

## Generated models

`python/cadetta/models/*.py` are generated from `docs/schemas/*.json` by
`scripts/gen_models.py`; never edit them by hand.

```sh
uv run scripts/gen_models.py          # regenerate every models module
uv run scripts/gen_models.py --check  # exit 1 naming each stale file (CI gate)
```

The script runs `datamodel-codegen` (pinned in the dev dependencies so output
is reproducible) with `--use-annotated --use-standard-collections
--use-union-operator --target-python-version 3.11 --use-subclass-enum
--formatters ruff-format --disable-timestamp` and pydantic v2 output, then
`ruff check --fix` (import order) and `ruff format`. To model another schema,
add a row to `MODELS` in the script. Bump the pinned generator version
deliberately and regenerate in the same change.

After a Rust change to `crates/cadetta-ir/src/model.rs` or a schema edit, run
the generator and commit the result; design flow `f_py_ir` in
`design/model.grmb` ties the Rust `Project` to the generated `ir.py`.
