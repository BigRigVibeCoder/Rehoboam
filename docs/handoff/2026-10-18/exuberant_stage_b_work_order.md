# Work Order: SPR-015 CI remediation and Stage B completion (revision 2)

- **Assignee:** Developer Exuberant (`AGT-002-CORE`)
- **Assigner:** Architect (`AGT-001`)
- **Date:** 2026-10-18
- **Branch:** `sprint/SPR-015-births-f1` (PR #107), head `cda5d16d` at writing
- **Change class:** C3 `[perf-baseline]`
- **Record:** `CODEX/40_VERIFICATION/VER-SPR-015-INTERIM.md` (the evidence for everything below)

Revision 2 replaces revision 1, which listed three fixes and was wrong about the second. CI is fail-fast: the failures below are the ones seen so far, not necessarily all.

## 1. Do these in order

### 1. Finish T-01: the promotion was a copy, not a move
`specs/systems/next/M2.demography.odd.toml` must not exist once M2 0.3.0 is frozen in `specs/systems/`. Today both exist, and the next-version lint reports `spec_version 0.3.0 must be a semver above the frozen 0.3.0`.
- Remove `next/M2.demography.odd.toml` (`git rm`).
- Remove M2's section and the `BIRTH` and `MARRIAGE` verbs from `next/registry_delta.toml`. Keep M7's section for SPR-021a.
- Update `next/README.md` where it still calls M2 a draft ("Current contents", step 4). SPR-016 starts the next M2 version, 0.3.1.
- Regenerate `next/amendments.lock.toml` with `python -I specs/systems/lint_odd.py --next specs/systems/next --overlay specs/systems/next/registry_delta.toml --write-amendments-lock`, and read the diff. Never type it.
- Re-run `specs/annals/fixtures/gen_fixtures.py` and `selftest_annals_v0.py --write` as the original work order says.
- **Check:** `lint_odd.py --next … --overlay …` exits 0.

### 2. The registry fixture
`tests/registry_effect/registry_absolute_primogeniture.toml` must equal `registry_v0.toml` plus the one law (`python/tests/test_registry_effect_extension.py`). Find how the file was produced (SPR-017 T-09) and regenerate it the same way. If no tool exists, say so in the PR and tell the Architect; do not hand-edit silently.
- **Check:** `pytest python/tests/test_registry_effect_extension.py` passes.

### 3. Explain the quiet-instant cost before touching a baseline
V-PERF-04 reports `instant/quiet` 14,306 to 23,230 instructions (+62%). A quiet instant has no births. Find where about 8,900 instructions go, with `cargo bench -p rh-bench --features arena-alloc --bench instant` and the counts under valgrind.
- If it is work proportional to N or L that SPR-038a removed, fix it. A baseline move does not excuse it.
- If it is a fixed cost (a new column, a new index hook), write the cause and the size in the PR body.
- Only after this, re-record.

### 4. Snapshot and baselines, in the final `[C3] [perf-baseline]` commit only
- `RH_UPDATE_SNAPSHOTS=1 cargo nextest run -p rh-golden --release --test tail`. Read the diff: the state hash on the keyframe line is expected to change, and **nothing else may change**. Report any other changed line.
- `python -I bin/perf_gate.py record`, with a `Reason:` line in the commit body.
- `bin/regen_baseline.sh` for the golden ladders, as the class requires.

### 5. Run everything locally
`bin/check_all.sh` (all three parts) and `python -I bin/check_all_selftest.py inventory`. Attach the report path to the PR. No report, no pass.

## 2. Process corrections

- The Stage A checkpoint is binding (GOV-005 §4.2): after T-01 to T-03 you stop for the Tester. T-04 was pushed an hour later with no Tester pass. Do not start Stage C or any task beyond T-04 until the Tester has passed Stage A and B.
- Commit subjects follow GOV-009 §3 (`type(SPR-015): T-NN description`, a `Why:`, `What:`, `Refs:` body). `merge(SPR-015): …` is not a type in use; use `Merge origin/main …`, by gate name.
- Keep PR #107 a draft until the checks above pass.

## 3. Then

The Architect requests the Tester (a different model from yours), audits (`VER-SPR-015`) and merges. SPR-016 stage 2 waits for that merge.
