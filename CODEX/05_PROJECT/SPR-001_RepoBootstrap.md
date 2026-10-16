---
id: SPR-001
title: "Repository bootstrap and CI"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** Create the Cargo workspace, the Python packages and CI, so that every later sprint lands in a checked, lint-guarded repository. Developer: AGT-002-CORE (with AGT-002-TOOLS for Python). Testers: AGT-003-IF. Scope-bounded; at most about 3 agent-days.

# SPR-001 — Repository bootstrap and CI

- **Roadmap item:** `docs/11` P0.1 · **Phase:** P0 (gate G0)
- **Depends on:** none
- **Binding contracts:** `docs/05_architecture/05.0` §6 (crate layout); GOV-003; GOV-008 §2; `docs/13` §3–4
- **Requirements covered:** EVO-1, LIC-1

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C2 (creates the repository's contracts: layout, lints, CI) |
| Planes touched (ADR-0024) | none (no simulation code) |
| Owned columns written | none |
| RNG purposes | none |
| Golden-hash effect | none |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-INSP-03, V-PLN-01 (lint scaffolding with a compile-fail positive control), V-DET-19 (forbidden-API gate with a positive control); `bin/check_all.sh` in CI |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: Cargo workspace
- **Deliverable:** `Cargo.toml` workspace; crate stubs per `05.0` §6 that P0/P1 need (`rh-ids`, `rh-core`, `rh-events`, `rh-registry`, `rh-contracts`, `rh-snapshot`, `rh-cli`); `rust-toolchain.toml` pinned
- **Acceptance:** `cargo build --workspace` and `cargo nextest run` pass on an empty workspace
- **Status:** [ ] not started

### T-02: Python packaging
- **Deliverable:** `python/` with `pyproject.toml` and a lockfile; `ruff` and `mypy --strict` configured
- **Acceptance:** `ruff check` and `mypy` pass
- **Status:** [ ] not started

### T-03: Static gates
- **Deliverable:** `rustfmt`, `clippy -D warnings`, `cargo-deny` (licence allow-list from ADR-0021, including the xxhash-rust trap in `docs/14`)
- **Acceptance:** A deliberately GPL test dependency fails `cargo-deny` (positive control)
- **Status:** [ ] not started

### T-04: Architecture lints
- **Deliverable:** Dependency-graph lint (system crates may depend only on `rh-core`, `rh-ids`, `rh-events`, `rh-registry`, contract crates); plane-isolation lint; forbidden-API grep gate (`HashMap` iteration, `thread_rng`, `StdRng`, `mul_add`, wall clock)
- **Acceptance:** Each lint has a failing fixture that it catches (V-INSP-03, V-PLN-01, V-DET-19)
- **Status:** [ ] not started

### T-05: CI wiring
- **Deliverable:** CI on Linux runs `bin/check_all.sh` plus the Rust and Python gates; artifacts uploaded; merge queue configured (`docs/13` §4.4)
- **Acceptance:** A PR with a lint violation is blocked; a clean PR passes in ≤ 15 min
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-IF | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-001` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-001` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
