# Independent review of PY-PKG (arch/PY-PKG 14c2633)

The review ran on a model different from the drafter's; nothing in the repository was changed. The Architect recorded this report from the reviewer's returned text.

## BLUF
**READY**, with two small fixes recommended in this PR (P1, P2). BLK 0, MAJ 0, MIN 3, NIT 4. The backend pin works as claimed (offline direct build on uv 0.11.32); adding a package is a directory plus one list line with `uv.lock` byte-identical; every planted real mistake turned a test or an existing gate red except inline suppressions (P5); the branch merges cleanly into main 3d26090. Corrections to the brief: the branch's parent is 17ca24f (13 commits behind main), not 99a5b3d.

## Verified
- `uv sync -v`: "Performing direct build for rehoboam-tools"; no uv_build in the cache; UV_OFFLINE=1 on a clean venv works, also after a forced rebuild (~7 ms). With uv_build==0.11.31 an offline sync fails; a range >=0.11.32,<0.12 also builds directly.
- PyPI: uv-build 0.11.32 first file 2026-07-23T23:05:01Z; MIT OR Apache-2.0; requires_dist empty. `uv lock --offline` rewrites nothing; the diff is one line (virtual -> editable).
- A second package (rehoboam_calib) leaves uv.lock byte-identical; `uv lock --check --offline` passes; `-I -m` works. `module-name` restricts only the wheel/sdist; the editable `.pth` (one plain absolute path, appended after site-packages, no import hook) makes anything in python/ importable; only the test guards it. A declared package with no directory fails `uv sync` itself.
- `-I -m`/`-I -c` work from the repo root, /tmp and /; `-I -S` fails. mypy needs py.typed for a file checked alone. A nested `__main__` (rehoboam_narrative/annals) is discovered.

## Planted mistakes (29): all caught except
- #8 ruff per-file-ignores = ALL on the package (passes), #29 inline `# ruff: noqa` / `# mypy: ignore-errors` (passes): P5; #28 a subpackage dir without `__init__.py` (harmless namespace package); #21 `package = false` passes pytest (stale venv) but the existing `uv lock --check` gate catches it; #25 a malformed `module-name` aborts collection (P4); #27 a stray dir holding only `__pycache__` fails wrongly (P1).

## Gates
uv lock --check rc 0 (20 packages); drift control rc 1 as expected; ruff pass; format 58 files; mypy 67 files; pytest 734 (and with hostile-CI variables); gen_manifest 221; inventory OK 485 (--plant-unlisted fails); lint_arch 0; lint_forbidden 0; workflows parse; ci_scope trace passes. All ci.yml parts, nightly.yml and adjutant.yml run `uv sync --frozen`; adjutant uses the VM's uv (one wheel download if different). Merged tree: ruff, format, mypy 70 files, gen_manifest 224, inventory 519, pytest 744.

## Findings
- P1 (MIN) `test_packaging.py:79-85`: a dir holding only `__pycache__` (left after a branch switch) gives a false "not declared"; skip or report as stale; add a planted control.
- P2 (MIN) `pyproject.toml`: isort's classification of `rehoboam_*` depends on the cwd; add `known-first-party = ["rehoboam_*"]` and move the import into its own block.
- P3 (MIN) README, SPR-028 T-02, SPR-029 T-02: `python -I` ignores PYTHONHASHSEED (-I implies -E); the two-process determinism checks need `python -P -s -m rehoboam_<name>`.
- P4 (NIT) `test_packaging.py:72-77, 124-130`: a malformed module-name aborts the whole session; make the parametrising helpers tolerant.
- P5 (NIT) the coverage tests overclaim (file lists, not suppressions); soften the wording.
- P6 (NIT) README: a git worktree needs its own sync too.
- P7 (NIT) CLAUDE.md's python/.venv line is one run-on sentence.

## Views on the drafter's F1-F8
F1 agree 8/10 (T-05 = directory + one line + textual pin; `__main__ --help` exit 0 without starting Textual; textual in every CI env is acceptable). F2 agree 8/10, plus P3; a workflow running `uv sync` without setup-uv is not checked. F3 agree 8/10, plus P3. F4 disagree that it belongs here 8/10 (6 of 19 under 14 days by the reviewer's count; a cooldown is its own decision). F5 agree 8/10. F6 accept 7/10 (a range would also work; exact pin is defensible). F7 agree 7/10. F8 agree 7/10 (process note).

## Merge test
`git merge-tree --write-tree origin/main origin/arch/PY-PKG` (main 3d26090): clean, tree 2d4712eb17ccced03fb62451dafe055a535259e6.
