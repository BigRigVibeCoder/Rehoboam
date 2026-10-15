# ADR-0022 — Spec-first, contract-tested, agent-sized work packages

**Status:** Proposed · **Date:** 2026-10-15

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
