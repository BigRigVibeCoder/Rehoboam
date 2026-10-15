# ADR-0022 — Spec-first, contract-tested, agent-sized work packages

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

## Context

AI coding agents are the workforce. Agents perform best on small, well-specified tasks with fast, objective feedback, and degrade on cross-cutting changes and vague goals. Success criterion S4 (evolvability) depends on this working.

## Decision

1. **Every module has a spec** (`docs/` and `specs/`) and **contract tests** before implementation.
2. **Work packages**:
   - Each is sized for 1–3 agent-days.
   - Each carries a template: goal, interfaces touched, invariants, fixtures, acceptance gates, out-of-scope.
3. **Objective gates on every PR**:
   - Compile, clippy and fmt.
   - Unit and property tests.
   - Golden runs, with any divergence declared.
   - Performance against baseline.
   - The stylized-fact ensemble, nightly or labelled.
   - Licence check.
   - Determinism hash across thread counts.
4. **A repo-level `CLAUDE.md`/`AGENTS.md`** holds conventions. **ADRs** record cross-cutting decisions. Agents may not change an ADR without your approval.
5. **The human (you) gates** ADRs, Registry semantics, and stylized-fact or MOE judgements.

## Consequences

- Specs come first, which slows the first commit. Rework is much rarer.
- Evaluation infrastructure (golden runs, the stylized-fact harness, the rating tool) is built in P0–P2, before features.

## Evidence

`13_agentic_development_playbook.md`; R03 §2 lesson (d), benchmarks in CI.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-M11). Sources of truth: 05.1 §2, §6.6, §6.10–6.11, 05.0 §6, 05.5 §2.1, §6, `13` §3–4 (v1.1). No inline corrections.*

**Changes:**
1. **Single-writer column ownership.** Every column and relation has exactly one owning system, declared in the Registry with units and meaning. A lint checks every write set against the Registry. Moving ownership is a contract change (C2).
2. **Contract crates** (`rh-contracts`) hold, per owning system, column types and read accessors, with no implementation. System crates depend only on `rh-core`, `rh-ids`, `rh-events`, `rh-registry` and `rh-contracts`. Systems never call each other.
3. **Lifecycle command queues.** Births, deaths, migrations, transfers of office or property, and infection-state changes are typed commands. `rh-core` applies them in `(phase, PersonId)` order. Each consequence chain has one code path, whichever system caused it.
4. **Derived scheduler.** Within a phase, system order is a topological sort of the declared read and write sets, with ties broken by system ID. Reverse dependencies are declared as **lags**. Work packages cannot create hidden ordering.
5. **Change classes** (`13` §4.2). CI detects the class independently, and the higher class wins.
   - **C0, docs and tests.** No hash effect.
   - **C1, internal.** `state_hash` unchanged on every golden world. Plane-B and plane-C hashes may change.
   - **C2, contract change** (contract crate, ownership, Registry, event, interface, wire, keyframe or manifest schema). `state_hash` unchanged. Breaking changes need a migration and director approval.
   - **C3, determinism-affecting.** Changes `state_hash`. The 32-seed CRN-paired stylized-fact ensemble runs on every C3 PR. It is **auto-approved within tolerance**. The director approves out-of-tolerance changes, changes to the determinism machinery, and changes to the golden-world corpus.

   This refines Decision 5: the director does not review every baseline change. It also replaces "nightly or labelled" in Decision 3 for C3 PRs.
6. **Merge queue** (`13` §4.4). Each PR is rebased on the queue head and re-runs the per-PR gates before it lands. C3 PRs are serialised, one at a time. The queue writes new golden baselines at merge; nobody commits baselines by hand. A nightly failure on `main` freezes the queue for C1–C3.
7. **Divergence bisect tool** (`13` §4.5). On a golden-hash mismatch, CI compares per-table Merkle roots and reports the first divergent month, table and row. The report is attached to the PR. A divergence on `main` without a code change is a determinism leak and freezes the queue.
8. **Review-hours budget.** The director is the only human reviewer, so review time is the binding constraint on throughput.
   - The director sets a weekly **review-hours budget** and records it in `docs/decisions-log.md`.
   - The architect agent plans work-package intake so that director-gated items fit the budget: ADR changes, Registry semantics, breaking C2 changes, director-gated C3 changes, phase gates and [H] ratings.
   - Items over budget wait in the queue. They are never auto-approved to save time.
   - Change classes keep in-tolerance work off the director's desk.
9. **Data-only extensions.** A `data_only` gap report is closed by a Registry edit plus tests, without a code work package (ADR-0010 v1.1).
10. **Golden baselines cover plane A only.** Pattern, significance and compiler iteration (plane C) never churn them (ADR-0024).
11. **Records.** `docs/decisions-log.md` and the repo-level `CLAUDE.md` exist. The log records director rulings, phase-gate outcomes, director-approved C3 changes and approved ADR changes, including approval of these v1.1 amendments.
