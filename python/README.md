# python/

The uv project behind every Python check in `bin/check_all.sh` (SPR-001 T-02), and the home of the repository's Python packages (PY-PKG).

- `pyproject.toml`: dependencies, the build backend and the package list, plus the ruff, mypy and pytest configuration (run from the repository root).
- `uv.lock`: the committed lock; `uv sync --frozen --project python` reproduces `python/.venv` and installs the packages in editable mode.
- `rehoboam_*/`: the packages, flat and top level (`docs/05_architecture/05.0_overview.md` §6). `rehoboam_tools/` holds only the distribution's version; code goes in a package named for its purpose.
- `tests/`: unit tests for the repository lints in `bin/` and for the packages.

## How the packages are installed and run

`uv sync` builds the project with `uv_build` (pinned exactly to the uv release CI installs, so uv uses the copy bundled in its own binary and downloads nothing) and installs it in editable mode: `python/.venv` gets a `rehoboam_tools.pth` file holding the absolute path of `python/`. The interpreter's `site` module reads that file at start-up, so from the repository root:

```bash
python/.venv/bin/python -I -m rehoboam_tools        # prints "rehoboam_tools 0.0.0"
python/.venv/bin/python -I -c "import rehoboam_tools"
```

`-I` (isolated mode) still processes `.pth` files; `-S` skips `site` and with it the packages and every dependency, so never run them with `-S`. `-I` implies `-E`, so `PYTHONHASHSEED` and every other `PYTHON*` variable is ignored: a subprocess that must set one (a two-process determinism check) runs `python -P -s -m rehoboam_<name>` instead. An edit to a package's code needs no new sync. A clone moved to another path, or a git worktree, needs `uv sync --frozen --project python` again, since the `.pth` file holds an absolute path: another checkout's `python/.venv` imports that checkout's packages.

## Adding a package (SPR-027 T-05, SPR-028 T-02, SPR-029 T-02)

1. Create `python/rehoboam_<name>/` with an `__init__.py` (module docstring and `Refs:`) and an empty `py.typed` (mypy needs it when a file that imports the package is checked on its own). Add a `__main__.py` if the package is run as `python -I -m rehoboam_<name>`; it must answer `--help` with status 0, as argparse does.
2. Add one line, `"rehoboam_<name>",`, to `module-name` in `[tool.uv.build-backend]` of `pyproject.toml`, in alphabetical order. If two branches add packages at once, keep both lines when you merge.
3. Run `uv lock --check --offline --project python`. It still passes, because `uv.lock` records the project as `source = { editable = "." }` and not its modules, so the lock does not change. The lock changes only when you add a dependency (`textual` for SPR-027 T-05): edit `[project] dependencies`, run `uv lock --project python`, and commit both files.
4. Put the package's tests in `python/tests/`. `test_packaging.py` needs no edit: it is parametrized over `module-name`, and it fails if the list and the `python/rehoboam_*/` directories differ in either direction, or if a package has no `__init__.py` or `py.typed`. It checks that each package imports under `python -I -c` from this checkout, that each `__main__` module runs under `python -I -m <module> --help`, and that ruff and mypy include every module in their file lists, with no mypy override relaxing `--strict` (inline suppressions are not looked for). A `python/rehoboam_*/` directory holding only `__pycache__`, which a branch switch leaves behind, is reported as stale: delete it.

Do not put a module directly in `python/`. The editable install puts all of `python/` on `sys.path`, so such a module would be importable everywhere; `test_packaging.py` refuses it.
