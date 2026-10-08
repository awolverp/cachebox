# cachebox

In-memory cache for Python. The hot path is a PyO3 extension (`cachebox._core`); `cachebox/` is the public package.

## Working rules

- Keep diffs small and match the surrounding style. Do not add a new language, framework, or packaging tool.
- After a behavior change, rebuild with maturin and run pytest.
- Do not edit `target/`, `*.so`, `__pycache__`, `.venv`, or other build output. Do not edit `Cargo.lock` or `uv.lock` unless a dependency changed.
- Do not read or write secrets or `.env`.
- `src/hashbrown/` is vendored hashbrown (see `LICENSE-THIRD-PARTY`). Change it only when the table itself must change.
- v6 is the public API. A breaking change needs a note in `docs/docs/migration.md` and a matching update to `cachebox/_core.pyi` and the MkDocs pages.
- `use-small-offset` is a test-only Cargo feature. Release and publish builds must not enable it.

## Layout

- `src/policies/` — eviction (Cache, FIFO, RR, LRU, LFU, TTL, VTTL).
- `src/pyclasses/` — one PyO3 class per policy, plus key/value/item iterators.
- `src/internal/` — pickle, `OnceInit` (`__new__` / `__init__`), linked list, hash helpers.
- `cachebox/_cachebox.py` — public `TTLCache` and `VTTLCache` (subclasses of the Rust types). Other classes are re-exported from `_core`.
- `cachebox/utils.py` and `_wrappers.py` — `@cached`, key makers, stampede locks.
- `tests/` — pytest mixins shared across implementations. Rust `#[cfg(test)]` covers the vendored table only.

## Commands

```bash
uv venv .venv && uv pip install --group ci
maturin develop --features use-small-offset   # local tests; CI does this
pytest -v -n auto                             # CI also sets HYPOTHESIS_PROFILE=slow
typos --config typos.toml
zizmor .github/
mkdocs serve --config-file docs/mkdocs.yml
```

`maturin develop --release` is the documented source install. Wheels are built only by `.github/workflows/CI.yml` on a tag.

## Style

- `rustfmt.toml` sets `imports_granularity = "Item"`.
- Rust edition is whatever `Cargo.toml` says. Edition 2024 denies `unsafe_op_in_unsafe_fn`; the crate allows it in `src/lib.rs`.
- Python docstrings are Google style. MkDocs (`mkdocstrings`) renders them.
- Clippy warns on `dbg!` and `print!`. Clippy, rustfmt, and mypy are not CI jobs.
