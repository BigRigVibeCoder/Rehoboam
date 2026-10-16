# python/

The uv project behind every Python check in `bin/check_all.sh` (SPR-001 T-02).

- `pyproject.toml`: dependencies, plus the ruff, mypy and pytest configuration (run from the repository root).
- `uv.lock`: the committed lock; `uv sync --frozen --project python` reproduces `python/.venv`.
- `tests/`: unit tests for the repository lints in `bin/`.

Python packages (`rehoboam_compiler`, `rehoboam_llm`, …; `docs/05_architecture/05.0` §6) arrive here in later sprints.
