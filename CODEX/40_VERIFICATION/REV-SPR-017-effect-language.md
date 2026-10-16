---
id: REV-SPR-017
title: "Independent review of the Registry effect language v0 spec (PR #41)"
type: record
status: DRAFT
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, REV-P1a, BCK-002]
created: 2026-10-16
updated: 2026-10-16
version: 0.3.0
---

> **BLUF:** Not ready at round 1 (PR #41 head `35aa53a`): **2 blockers, 8 majors, 7 minors.** The reference reproduces all 456 golden cases, its positive control fails as it should, and the 12 succession cases agree with the spec text when checked by hand. The genealogy relations read only the life-course log and reach through the dead. Nothing depends on iteration order. D1 rounding matches `rh-core::fixed` and D9 matches `draw_hits`. The blockers: ticks are written in days where the engine counts minutes, and a 4-degree candidate cut breaks representation. Two choices go to the director at the SPR-017 plan gate: whether collateral heirs must be of the title's blood (M8), and the sex rules (D4). The reviewer expects no objection after one revision round (8/10).

# REV-SPR-017: the effect language v0, round 1

Reviewed: `specs/registry/effect_language_v0.md`, `effect_language_golden_v0.toml` (456 cases) and `effect_ref.py` at PR #41 head `35aa53a`, against `05.5` §2.1, `05.1` §2 and §6, `rh-core::fixed`, REV-P1a §7–§8 and PR #32 at `770811a`. The review ran in a session on a different model from the drafter's, using scratch copies; nothing in the repository was edited.

## 1. Checks that pass
- `effect_ref.py --check` reproduces all 456 cases. Its positive control fails as intended. ruff and mypy are clean.
- All 12 succession cases were checked by hand against the spec text, and all agree. They include a grandson through a dead elder son ahead of living younger sons, and a dead daughter's son after every son's line and before the living younger daughter, under male preference.
- Every relation reads only `life_course.*` and reaches through the dead.
- No result depends on iteration or hash order, and no floats are used beyond reading Registry bounds.
- D1 matches `round.rs` and `arith.rs`, including `MIN / -1 = MAX`.
- D9 matches `draw_hits`.

## 2. Findings and the Architect's rulings

| # | Finding | Ruling |
|---|---|---|
| B1 | Ticks are written in days. `05.1` §4.1, `SimTime` and M2's `birth_tick` count minutes. In the engine, tanistry's age test would compare minutes with 5,760, so a male counts as adult at 4 days old. The draw key `t` also disagrees with the hosts' `[[rng]]` declarations. The fixture's ticks are in days, so the golden file cannot catch either problem | Ticks are SimTime minutes. Add `calendar.minutes_per_day`, do age tests in `int`, and convert the fixture |
| B2 | `candidates = kin(holder, 4)` breaks representation: a first cousin's son is degree 5. `succession_primogeniture_for_a_vassal` omits four living representatives. The 256 cap truncates by ascending id, which can silently drop a late-born heir | Candidates come from a lineage relation: the descendants, to `DESCENT_MAX`, of each ancestor `A_h(holder)` with h ≤ 3, truncated in (up, line) order. Truncating `candidates` is an error, never a diagnostic |
| M1 | `q8(1) / 0.001` passes `E_DIV` because the literal's text is non-zero, but its bits round to 0. The reference then raises `ZeroDivisionError` | Check the divisor's bits after rounding; add rejection cases |
| M2 | D8 puts the relations in a contract crate, against `05.1` §2 rule 1 and the NEW-3 ruling. A contract crate cannot hold the children-by-parent index either | The prelude lives in `rh-registry`, reads through `SystemView`, and uses the host's derived index |
| M3 | A degree limited to a literal ≤ 4 cannot express M7's `dormancy_degrees` ∈ [1, 6] or the Registry's `kin(6)` | The maximum degree is 6, as a literal or as a `params` constant bounded in range |
| M4 | The language cannot express T1's draw keys (`break_outcome`, `attack_outcome`, keyed by target id). The spec's `MURDER_ATTEMPT` with `k = 0` lets two attempts share a draw, which undoes REV-P1a MIN-3 | `k` may be an `int` expression or an entity id; lint that each call site is unique per system; align the golden file's T1 slice with T1's spec |
| M5 | `E_OWNER_NOT_REGISTERED` rejects the load, which conflicts with NEW-4's drop-and-report rule. A read whose owner is absent returns the type's zero, not the neutral value | The program compiles; registration drops the action and reports it; an absent owner's read gives the coupling's neutral value |
| M6 | The `[commands]` signatures omit `tick` from `ScheduleDeath` and `Marry`, which M2's `[[applies]]` declares | Lint the signatures against `[[applies]]`; add the tick arguments |
| M7 | The reference looks up the life-course log by row id and never reads `person_id`. Renumbering the rows changes 60 cases, so an evaluator with the same mistake would still pass | Index by `person_id`; give the fixture row ids that differ from the person ids |
| M8 | The cognatic `common` puts a maternal half-sister, not of the grantor's blood, ahead of the grantor's son | **Director, at the SPR-017 gate.** The Architect recommends a title-blood rule as the default (collaterals must descend from a previous holder of the title), with the unrestricted variant kept as a Registry variant. 75% |
| m1–m7 | Keys are untyped. An enum's zero is its first value. `missing_reads` is undefined, so a short-circuiting evaluator passes. The spec's own examples fail its checker. Some rejection cases have two faults. Some cases are vacuous: `vow_avenge_kin_*` passes on missing reads, the twins cases do not have equal keys, and `effects_two_in_order` emits two identical commands. Smaller points: the draw key's entity kind, the `ancestors` bound (126), relation-argument wording, and expression depth | All accepted, each as the reviewer proposes |

The reviewer's positions on D1–D13: agree with D1, D3, D6, D7, D9, D10, D11 and D13; partly agree with D2 and D4 (enum defaults, absent owners, title blood); disagree with D8 (M2) and D12 (B2, M3). On Q1–Q11: Q2's premise is false (B2); Q5's key is `["person", "tick"]`; Q7 is evaluable after the death instant through `life_course.death_cause`; the rest are agreed.

## 3. Follow-through
- **PR #41 round 2:** the rulings above, sent to the drafter on 2026-10-16.
- **PR #32 round 4d** (head `d8bbddb`): M7 declares `sortition`; T1 declares its `OpinionModifier` self-send; the relation names follow §5.3; M7 no longer restates the candidate set.

## 4. Round 2 (PR #41 head `1245d95`, v0.2.0, 487 cases)

**Verdict:** not ready. **1 blocker, 4 majors, 6 minors.** The reference reproduces all 487 cases. Ten laws were checked by hand and agree with the text. The mutants fail as they should: a row-id-keyed log fails 70 cases, an absent-owner read that returns zero fails 3, and a short-circuiting evaluator fails 2. B1, M1–M3, M5, M7 and the round-1 minors are resolved. B2 is resolved but brought two regressions (N1, N2). M4 is not resolved (N4), and M6 only partly. M8 is implemented but wrong (B3).

| # | Finding | Ruling |
|---|---|---|
| B3 | `descends(x, title.founder)` looks only 6 generations up, and the founder never changes. Seven generations below the founder, `primogeniture` passes over the dead holder's sons for his mother and grandmother, and `seniority` and agnatic primogeniture return empty lines | Title blood is "descends from the holder, or from one of the title's last 8 holders". `holders_of(title, n)` is read from a new M7 tenure log (`tenure.title`, `holder`, `start`, `end`; append-only; written by `rh-worldgen` at world creation). `title.founder` goes. Add a golden case seven or more generations deep |
| N1 | `lineage` includes ancestors at heights 1–2, and the `common(x, holder) != x` guard was dropped, so a father can rank ahead of his grandson | Restore the guard; pin it with a case |
| N2 | The 1,024 bound, with a hard failure, is reached by ordinary fertility (about 1,300) | Prune dead members with no living descendant before applying the bound; size the bound in D12; add an `E_TRUNCATED` case and a case with known great-grandparents |
| N3 | `event: int` lets an EventId be computed with, or used as a draw key, before it exists, and the merge rewrites only `Command::cause` | `event` becomes an opaque type, allowed only in event-typed payload fields. The `rh-core` merge rewrites those fields (a plan requirement) |
| N4 | `MURDER_ATTEMPT` draws `attack_outcome` itself, so the event can record a failure while the program schedules a death | Bind `success` from the attempt event that T1 drew (`break_outcome` or `attack_outcome`); `when success` |
| m | Purpose names and reads out of step with PR #32 (`succession_tie`, T1's reads, the `OpinionModifier` self-send, S7's `death_cause` read). A five-value `death_cause` enum against PR #37's six. `E_RNG_REUSE` is only syntactic. The sortition key has no title. `E_NO_NEUTRAL` depends on which systems are registered. `calendar.minutes_per_day` is an engine constant | Align with PR #32 at `9d883b5` and PR #37 at `83b9654`. `E_RNG_REUSE` gets both a static lint (the entity kind in k, folded constants, aggregates keyed on the loop variable) and a run-time check per step. Sortition is keyed by title. A machine-readable `neutral` value on `[[reads]]`. `MINUTES_PER_DAY` |

**Follow-through:** PR #41 round 3, and PR #32 round 4g (the tenure log, sortition keyed by title, S7's `death_cause` read). Plan items for SPR-014: the merge rewrites event-typed payload fields, and the machine-readable `neutral` value comes with the absent-owner rule.

## 5. Round 3 (PR #41 head `08deb53`, v0.3.0, 538 cases)

**Verdict:** not ready. **0 blockers, 2 majors, 6 minors.** All 538 cases reproduce. The reviewer checked by hand: the deep world, both N1 cases, the title-blood exclusion, tanistry and the fan counts. The mutants fail as they should: a row-id-keyed log fails 86 cases, short-circuit evaluation 2, stopping at height 2 fails 5, and each pruning mutant fails 1 to 4. B3, N1–N4 and M6 are resolved. The reviewer has no objection to the language itself (grammar, types, semantics, totality), but objects to D14 option (a) until N5 is recorded, and to D19 until N6 is fixed.

| # | Finding | Ruling |
|---|---|---|
| N5 | Title blood at world start and under rotas. With a one-row tenure log, a childless holder's death gives an empty line, and seniority and tanistry return only the son. A rota of 9 tenures excludes a cousin whose grandfather held. Genesis founders have no parent links at rung 0 | A worldgen requirement: for a dynastic world, `rh-worldgen` back-fills pre-epoch life-course rows with parent links, and tenure rows for past holders (SPR-019). The spec states this precondition. The window widens to `holders_of(title, 16)`. Pin a one-row-log case and a rota case |
| N6 | The run-time `E_RNG_REUSE` ledger is scoped to a host step, so evaluating one sortition line twice in a step aborts the run | Scope the ledger to one evaluation of one program for one binding. A repeat evaluation is idempotent. Pin a case |
| m1–m6 | The D5 guard leaks under uncle–niece marriage. A redraw in a verb whose payload carries an outcome is accepted. Neutral values are untested beyond the type's zero. Enum values named `none` cannot be written. The static lint spans a system's mutually exclusive programs. Assorted text and pins, including that the `neutral` field must exist before SPR-017's loader | `not descends(holder, x)`. A draw in an outcome-carrying verb is rejected. A neutral case that is not zero. Allow a keyword after the dot. Uniqueness within a program and across programs that can run in one step. Fix the text. The `neutral` field becomes a spec prerequisite of SPR-017 |
