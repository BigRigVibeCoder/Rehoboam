# Work Order: Agent 1 (AGT-002-TOOLS) — SPR-028 Task T-03 (Ensembles & G1a Demography Gates)

**Role:** Developer, Tools (`AGT-002-TOOLS`)  
**Assigned Agent:** `Agent 1` / `Adjutant Root`  
**Working Directory:** `/home/bdavidriggins/Documents/Rehoboam`  
**Target Branch:** `sprint/SPR-028-ensembles` (branched from `origin/main`)  
**Date:** 2026-10-19 / 2026-10-20  
**Author:** Lead Architect (`AGT-001`)  

---

## 1. Executive Briefing & Context

### 1.1 PR #124 Status: APPROVED & MERGED to `main`
- **PR #124 (`spec(SPR-028): T-01 validation harness contract and reference`)** has passed 100% of CI checks (`core`, `golden`, `stream`) and has been merged into `main` at commit `775f939105aa1867a0f5338aefd74f75574acc16`.
- **Delivered & Ratified:**
  - T-01: Validation harness contract, specification [`specs/validation/harness_v0.md`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/harness_v0.md), TOML schema, and reference runner passing all 16 statistical validation tests.
  - T-02: Production calibration library [`python/rehoboam_calib/`](file:///home/bdavidriggins/Documents/Rehoboam/python/rehoboam_calib/) (modules: `stats`, `read`, `bands`, `report`, `__main__`) and comprehensive unit/integration test suite [`python/tests/test_calib.py`](file:///home/bdavidriggins/Documents/Rehoboam/python/tests/test_calib.py) passing cleanly.
  - Review: [`CODEX/40_VERIFICATION/REV-SPR-028-harness.md`](file:///home/bdavidriggins/Documents/Rehoboam/CODEX/40_VERIFICATION/REV-SPR-028-harness.md) ratified **APPROVED 1.0.0**.

### 1.2 Concurrency Isolation Reminder (GOV-011 §2)
- You are operating in the **main repository root** (`/home/bdavidriggins/Documents/Rehoboam`).
- Sibling agent **Agent 2 (`AGT-002-CORE`)** is operating strictly inside Git worktree `.claude/worktrees/agent-2-spr016`.
- Do not run heavy release builds concurrently without checking system memory.

---

## 2. Work Order: Task T-03 — Ensembles, Demography Gates & V-GOLD-03

Per [`CODEX/05_PROJECT/SPR-028_ValidationHarness.md`](file:///home/bdavidriggins/Documents/Rehoboam/CODEX/05_PROJECT/SPR-028_ValidationHarness.md#t-03-ensembles-the-g1a-demography-gates-v-gold-03s-comparison-f-8-f-12-f-14-g10-g12-075-day), you are authorized to begin **Task T-03**.

### 2.1 Branch Initialization
Fetch `origin/main` and create your new sprint branch:
```bash
git fetch origin
git checkout -b sprint/SPR-028-ensembles origin/main
```

### 2.2 Task Deliverables

#### 1. CLI Ensemble Runner (`rehoboam_calib ensemble`)
Extend `python/rehoboam_calib/__main__.py` (and supporting module `python/rehoboam_calib/ensemble.py` if modular):
```bash
python -m rehoboam_calib ensemble --world <w> --seeds 1-32 --years Y --jobs J --out DIR
```
- Orchestrates `rh-cli run --seed s --threads 1 --years Y --run-dir DIR/seed-s --vitals` per seed, running `J` jobs concurrently.
- Upon completion of all seeds, automatically invokes `rehoboam_calib report` over `DIR/seed-*`.
- Fail-fast rule: Any single failed seed run must immediately abort the ensemble and name the failing seed and error.

#### 2. Golden Test World: `fixture-demography_1000`
- Create `tests/worlds/demography_1000/world.ron` (world name `fixture-demography_1000` per `crates/rh-golden/src/world.rs:124–135`):
  - 1,000 synthesized persons.
  - 1 region (`flat`, `region_count: 1`, so M1 F0 gives one settlement and N4's pool is the world).
  - Registry M2 defaults; M2, M1, and S3 registered.

#### 3. The G1a Demography Gates
- Implement scripts / configs for evaluating:
  - **V-SF-01 ($r$):** Two-sided 99.9% Student-t interval of the seed-mean annual intrinsic growth rate $r$ must lie within $[-0.1, +0.3]\%$ per year (S1; widowhood chain gives $r \approx +0.11\%$).
  - **V-SF-02 ($q(15)$):** Seed-mean $q(15)$ and two-sided 99.9% t-interval must lie in $[35\%, 55\%]$ ($e_0=30, \beta=1$ gives $q(15)=0.4443$ compiled).
  - **V-SF-02b ($l(65)/l(15)$):** Ratio of survivors at age 65 to age 15 lies within $\pm 0.03$ of compiled schedule ($0.3146$).
  - Population range per seed reported with `expected_spread` and seeds outside $[0.3, 5] \cdot N_0$ (informative diagnostic, not a hard pass/fail verdict; G10).
  - Provide runner script `bin/g1a_ensembles.sh` (32 seeds × 1,000 years for session-level, 32 seeds × 300 years for weekly).

#### 4. Plant Calibration Mutant: `mutant-m2-e0-plus-15`
- Add cargo feature `mutant-m2-e0-plus-15` in `crates/rh-sys-demography/Cargo.toml` and wire into schedule compilation (`e0 + 15`).
- Non-vacuous check:
  - `compile_schedule` at $e_0=45$ gives $q(15)=0.2463$ (falls 10 points below band $[35\%, 55\%]$) and $l(65)/l(15)=0.4530$ (shifts 0.14 from target, violating $\pm 0.03$).
  - Fails V-SF-02 and V-SF-02b by name!

#### 5. V-GOLD-03 Comparison Tool & GitHub Actions Workflow
- Implement `python -m rehoboam_calib compare --before DIR --after DIR`:
  - Enforce zero-variance rule: identical before/after runs must return `EQUIVALENT` with paired differences identically 0.
  - Correct build vs `mutant-m2-fertility-x2` must return `NOT EQUIVALENT` on V-SF-01 ($r$) with the failing statistic named.
- Add `.github/workflows/ensemble.yml`:
  - `workflow_dispatch` trigger: base and head refs, worlds, 32 seeds × 200 years each.
  - `pull_request` trigger: runs smoke test (2 seeds × 10 years of `household`, base = head, verifying `EQUIVALENT`).

---

## 3. Execution & Verification Checklist

1. [x] Check out `sprint/SPR-028-ensembles` from `origin/main`.
2. [x] Implement `rehoboam_calib ensemble` and `compare`.
3. [x] Construct `tests/worlds/demography_1000/world.ron` and `world.toml`.
4. [x] Plant `mutant-m2-e0-plus-15` and `mutant-m2-fertility-x2` in `rh-sys-demography`.
5. [x] Wire `.github/workflows/ensemble.yml` and `bin/g1a_ensembles.sh`.
6. [x] Verify all checks locally:
   - `python/.venv/bin/pytest python/tests/test_calib.py`
   - `python/.venv/bin/ruff check python/`
   - `python/.venv/bin/ruff format --check python/`
   - `python/.venv/bin/mypy --strict python/rehoboam_calib/`
   - `bin/check_all.sh --part core`
7. [ ] Commit with standard GOV-009 commit messages (`feat(SPR-028): T-03 ... [C1]`).
8. [ ] Push branch and open draft PR:
   ```bash
   git push -u origin sprint/SPR-028-ensembles
   gh pr create --draft --title "feat(SPR-028): T-03 ensembles and G1a demography gates [C1]" \
     --body "Why: ...\nWhat: ...\nRefs: SPR-028 T-03\nAgent: Adjutant (AGT-002-TOOLS)"
   ```
