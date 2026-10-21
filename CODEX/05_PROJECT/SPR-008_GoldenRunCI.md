---
id: SPR-008
title: "Golden-run CI, the smoke world and the bisect tool"
type: how-to
status: ARCHIVED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001, RUN-002, GOV-008, VER-SPR-008, VER-SPR-008-TEST]
created: 2026-10-16
updated: 2026-10-16
version: 1.2.1
---
> [!NOTE]
> **ARCHIVED BASELINE:** Traditional sprint-based tracking has been superseded by Specification-Driven Development (SDD v2.0). All future deliverables are tracked as Spec Waves in `.spec/ARCHITECT_BACKLOG.md`.

> **BLUF:** Prove determinism continuously: a smoke world with dummy systems runs a thousand years in CI with hash ladders, and a bisect tool names any divergence. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-008 — Golden-run CI, the smoke world and the bisect tool

- **Roadmap item:** `docs/11` P0.6 · **Phase:** P0 (gate G0)
- **Depends on:** SPR-006, SPR-007 · **Plan approved by the director 2026-10-16** (P0 batch)
- **Binding contracts:** `docs/13` §4.3–4.5; ADR-0006
- **Requirements covered:** DET-4, DET-5

## Architect rulings, 2026-10-16 (binding; written before the sprint starts)
1. **The first system crate, and BCK-002 A-27 first (T-00).**
   - The smoke world's dummy systems live in a new crate, `rh-sys-smoke` (plane A, kind `system`), under the same dependency rules as every system crate.
   - Their skeletons are generated from ODD-R specs with `lint_odd.py --rust` and brought in with `include!`, as ADR-0033 §3 intends. That is why A-27 is task T-00: `lint_forbidden` follows an `include!` whose argument is a string literal resolving to a file inside the repository, and scans that file as source. A non-literal or out-of-repository path stays forbidden. Controls both ways. SPR-005's counted `LINT-ALLOW` in `rh-registry/tests/real_specs.rs` then goes.
2. **Dummy systems never enter the real Registry.**
   - The smoke world carries its own spec set (`worlds/smoke/systems/*.odd.toml`, X-prefixed ids) and whatever Registry overlay it needs, under `worlds/smoke/`.
   - `specs/systems/` and `rh-cli ownership` stay limited to real systems.
   - `bin/check_all.sh` runs `lint_odd.py` and `gen_skeletons --check` over the smoke specs as well.
   - Registration compares the dummy code with these specs (rule `spec`), as for real systems.
3. **What the smoke world exercises.**
   - Every phase of `05.1` §4.2 and every pulse.
   - Two bindings of one system in one phase.
   - `same_instant` and `lagged` reads.
   - Timers: scheduled, fired, cancelled, and re-sampled by a synthetic material input that goes through SPR-007's journal.
   - Lifecycle commands sent and applied.
   - Events with causal edges.
   - An invariant per system.
   - At least one `rh-sample` draw, so the float zone is inside the golden run.
   - All arithmetic is fixed point, with keyed draws only.
4. **The runner and the ladder.**
   - `rh-cli run smoke --years N --threads T --ladder yearly|monthly --out <file>` writes a hash ladder: per rung, the sim-time, the plane-A state hash (SPR-007, including engine-held state) and the per-table roots.
   - The ladder format is versioned and documented, and holds no build id or wall time, so ladders from two machines or two builds compare byte for byte.
   - `rh-cli ladder compare <a> <b>` reports the first differing rung.
5. **Tiers.**
   - PR tier: `smoke` for 1,000 years, yearly, at 1 thread and at N threads, compared with the committed baseline. If this pushes the PR tier past the 15-minute budget (V-PERF-04), measure it, propose a shorter PR horizon, and keep the full length nightly.
   - Nightly: monthly for 1,000 years at 1, 2, 4 and N threads × 3 repeats (V-DET-01, V-DET-02; the G0 criterion).
6. **Baseline integrity (T-04).** CI recomputes the ladder on every PR head.
   - If the baseline files are unchanged, they must equal the recomputation.
   - If they changed, they must equal the recomputation byte for byte, so a baseline can only be regenerated, never typed, and the PR title must carry `[C3]`.
   - One gate enforces both, with controls: a hand-edited baseline fails, and a regenerated baseline in a PR not marked C3 fails.
7. **Bisect v0 (T-03).**
   - `rh-cli bisect` runs the baseline and candidate builds (two binaries, or two revisions built in CI) to the first divergent rung, then to the first divergent month.
   - It compares per-table roots, then chunks (SPR-007's `first_divergence`), then rows, and reports the month, the table, the row and the first differing column.
   - V-DET-11: a one-row perturbation injected at a random month and table is named exactly.
   - In CI, a golden mismatch runs the bisect automatically and attaches its report as a job artifact (`docs/13` §4.5).
8. **Cross-machine (part of T-02).**
   - This sprint builds the mechanism: portable ladders, `ladder compare`, and the nightly ladder uploaded as an artifact.
   - The documented procedure for the director's laptop (RUN-001) and `adjutant` (RUN-002) runs when those machines are available. The G0 criterion is thread-count and repeat invariance (V-DET-01/02); cross-machine evidence is recorded when it exists.
9. **Order.** This sprint starts after SPR-007 merges: it needs the real plane hashes and the keyframes, which let a bisect skip ahead instead of re-running from year 0.

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C3 (determinism machinery; creates the golden-world corpus) |
| Planes touched (ADR-0024) | A |
| Owned columns written | dummy systems only |
| RNG purposes | dummy purposes |
| Golden-hash effect | records the first golden baselines (through the merge queue) |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-DET-01, V-DET-02 (1/2/4/N threads × 3 repeats, monthly for 1,000 years), V-DET-11 (bisect names an injected month, table and row) |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-00: Scan included source (BCK-002 A-27)
- **Deliverable:** `lint_forbidden` follows a literal in-repository `include!` and scans the included file; non-literal and out-of-repository paths stay forbidden (ruling 1). Then the counted `LINT-ALLOW` in `rh-registry/tests/real_specs.rs` is removed
- **Acceptance:** Controls both ways: a forbidden API inside an included skeleton FAILS; a clean included skeleton passes; `include!(concat!(…))` and an out-of-repository path FAIL
- **Status:** [ ] not started

### T-01: Smoke world
- **Deliverable:** `worlds/smoke` with dummy systems exercising every phase, timers and lifecycle commands
- **Acceptance:** Runs 1,000 sim-years headless
- **Status:** [ ] not started

### T-02: Hash ladders in CI
- **Deliverable:** Yearly ladders on PRs (shorter horizons); monthly full length nightly; a cross-machine check nightly between a GitHub-hosted Ubuntu runner and the director's laptop (Intel), and on `adjutant` (AMD) in each VM session (RUN-002 §4); different CPU vendors replace the cross-platform matrix per D5 (GOV-008 §1)
- **Acceptance:** V-DET-01/02 green
- **Status:** [ ] not started

### T-03: Bisect tool v0
- **Deliverable:** Compares per-table Merkle roots between baseline and candidate; reports first divergent month, table, row
- **Acceptance:** V-DET-11 on injected perturbations
- **Status:** [ ] not started

### T-04: Baseline management
- **Deliverable:** Baselines recorded only at merge of approved C3 PRs (by the merge queue, or by the fallback in `docs/13` §4.4)
- **Acceptance:** A hand-edited baseline is rejected
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-008` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [x] All tasks meet their acceptance criteria.
- [x] `bin/check_all.sh` passes; the report is attached.
- [x] Tester verdicts are PASS.
- [x] Architect audit `VER-SPR-008` is PASS.
- [x] No open DEF- against this sprint.
- [x] Merge route matches the CI-detected change class.
