---
id: SPR-001
title: "Repository bootstrap and CI"
type: how-to
status: ARCHIVED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001, RUN-001, GOV-008]
created: 2026-10-16
updated: 2026-10-16
version: 1.2.0
---
> [!NOTE]
> **ARCHIVED BASELINE:** Traditional sprint-based tracking has been superseded by Specification-Driven Development (SDD v2.0). All future deliverables are tracked as Spec Waves in `.spec/ARCHITECT_BACKLOG.md`.

> **BLUF:** Create the Cargo workspace, the Python packages and CI, so that every later sprint lands in a checked, lint-guarded repository. Developer: AGT-002-CORE (with AGT-002-TOOLS for Python). Testers: AGT-003-IF. Scope-bounded; at most about 3 agent-days.

# SPR-001 — Repository bootstrap and CI

- **Roadmap item:** `docs/11` P0.1 · **Phase:** P0 (gate G0)
- **Depends on:** none. **Approved by the director 2026-10-16 (C2).** GitHub plan: Free (Q-INFRA-1)
- **Binding contracts:** `docs/05_architecture/05.0` §6 (crate layout); GOV-003; GOV-008 §1–2, §4; `docs/13` §3–4; RUN-001
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

### T-00: The machine
- **Deliverable:** The director's laptop set up per RUN-001
- **Acceptance:** `bin/check_all.sh` passes on the laptop and the report is attached
- **Status:** [ ] open: the director's action (RUN-001); carried forward in VER-SPR-001

### T-01: Cargo workspace
- **Deliverable:** `Cargo.toml` workspace; crate stubs per `05.0` §6 that P0/P1 need (`rh-ids`, `rh-core`, `rh-events`, `rh-registry`, `rh-contracts`, `rh-snapshot`, `rh-cli`); `rust-toolchain.toml` pinned
- **Acceptance:** `cargo build --workspace` and `cargo nextest run` pass on an empty workspace
- **Status:** [x] done (VER-SPR-001)

### T-02: Python packaging
- **Deliverable:** `python/` with `pyproject.toml` and a committed `uv.lock` (Python 3.12); `ruff` and `mypy --strict` configured; RUN-001 §4 switched from the ad-hoc `uv pip install` line to `uv sync`
- **Acceptance:** `ruff check` and `mypy` pass; `uv sync --frozen` on a fresh clone reproduces the environment
- **Status:** [x] done (VER-SPR-001)

### T-03: Static gates
- **Deliverable:** `rustfmt`, `clippy -D warnings`, `cargo-deny` (licence allow-list from ADR-0021, including the xxhash-rust trap in `docs/14`)
- **Acceptance:** A deliberately GPL test dependency fails `cargo-deny` (positive control)
- **Amended by D19 (2026-10-16):** personal-use mode. Licences go to a non-blocking ledger, and the GPL control now proves a GPL dependency is recorded there. `bans` and `sources` stay blocking, each with a positive control (ADR-0021 as amended)
- **Status:** [x] done (VER-SPR-001)

### T-04: Architecture lints
- **Deliverable:** Dependency-graph lint (system crates may depend only on `rh-core`, `rh-ids`, `rh-events`, `rh-registry`, contract crates); plane-isolation lint; forbidden-API grep gate (`HashMap` iteration, `thread_rng`, `StdRng`, `mul_add`, wall clock)
- **Acceptance:** Each lint has a failing fixture that it catches (V-INSP-03, V-PLN-01, V-DET-19)
- **Status:** [x] done (VER-SPR-001)

### T-05: CI wiring
- **Deliverable:** GitHub Actions on GitHub-hosted Ubuntu runners (GOV-008 §1) runs `bin/check_all.sh` plus the Rust and Python gates, with the Cargo cache; the check report is uploaded as an artifact. Merge protection per `docs/13` §4.4: the merge queue if the plan allows it; otherwise required checks with "branch must be up to date", plus the serialised-merge rule. A `pre-push` hook (installed by a `bin/` script) runs the fast tier locally
- **Acceptance:** A PR with a lint violation cannot be merged; a clean PR passes in ≤ 15 min on the hosted runner. If the GitHub plan cannot enforce required checks (Q-INFRA-1), the sprint records that in `VER-SPR-001` and the Architect enforces "no green report, no merge" by hand until it can
- **Status:** [x] done (VER-SPR-001)

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-IF | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-001` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| B-1 | Required checks and the merge queue depend on the GitHub plan for a private repository (Q-INFRA-1) | `decisions_pending.md` | **Resolved:** Free. CI reports; the Architect enforces "no green report, no merge" by hand (`docs/13` §4.4) |

## Completion

- [x] All tasks meet their acceptance criteria (T-00, the director's laptop run, carried forward).
- [x] `bin/check_all.sh` passes (71 checks); the report is attached in CI.
- [x] Tester verdicts are PASS (`VER-SPR-001-TEST`).
- [x] Architect audit `VER-SPR-001` is PASS.
- [x] No open DEF- against this sprint (DEF-1 to DEF-5 fixed).
- [x] Merge route matches the CI-detected change class (C2, Architect merges).
- **Status:** [x] done (VER-SPR-001)