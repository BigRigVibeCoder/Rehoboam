---
id: SPR-010
title: "rh-core::fixed: fixed-point and money types"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001, ADR-0006, VER-SPR-001]
created: 2026-10-16
updated: 2026-10-16
version: 1.1.0
---

> **BLUF:** Build the fixed-point number types that every threshold quantity in plane A must use (DET-3): `Q8_8`, `Q16_16` and money in i64 minor units, with saturating arithmetic and one documented rounding rule. Found missing from the P0 plan by the SPR-001 audit; no other sprint delivered it. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 2 agent-days.

# SPR-010 — rh-core::fixed

- **Roadmap item:** `docs/11` P0.4 (core numerics; DET-3 is a P0 requirement) · **Phase:** P0 (gate G0)
- **Depends on:** SPR-001. Independent of SPR-002 and SPR-004, so it can run beside them · **Plan approved by the director 2026-10-16** (C3; the director also merges it)
- **Binding contracts:** `05.1` §6 item 4 (numerics); `05.5` §2.1 (the effect language: fixed point only, saturating, no floats, guarded division); ADR-0006; GOV-003 §2; `specs/systems/_template.odd.toml` (the `q8_8` and `q16_16` column types)
- **Requirements covered:** DET-3

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C3 (determinism machinery: the director approves the spec and the merge) |
| Planes touched (ADR-0024) | A (machinery) |
| Owned columns written | none |
| RNG purposes | none |
| Golden-hash effect | defines the numeric types later hashes cover; no golden world exists yet |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-DET-19 (the float-in-threshold part, now checkable), golden vectors for every operation, property tests below |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` V-DET-19 row if the check's form changes; `CLAUDE.md` checks |

## Tasks

### T-01: The types
- **Deliverable:** `Q8_8` (i16 storage) and `Q16_16` (i32 storage) newtypes and `Money` (i64 minor units) in `rh-core::fixed`; constructors from integers and from exact ratios; no conversion from or to `f32`/`f64` in plane A (a test-only or tool-only conversion is allowed behind a feature that plane-A crates may not enable)
- **Acceptance:** Compile-fail tests: no implicit mixing of the three types, and no float conversion from a plane-A crate
- **Status:** [ ] not started

### T-02: Arithmetic and rounding
- **Deliverable:** Saturating add, subtract and multiply; division only through a checked function that returns `Option` (the effect language's "guarded division"); one rounding rule for multiply and divide, documented with a `DECISION:` comment (round half to even is the default proposal); comparisons and ordering
- **Acceptance:** Property tests: saturation at both bounds, commutativity and the identity elements, monotone ordering, rounding matches an exact rational reference (Python `fractions`, run as a golden-vector generator); golden vectors committed
- **Status:** [ ] not started

### T-03: Thresholds and draws
- **Deliverable:** A comparison of a uniform RNG draw (a `u64` from SPR-002's API, or a placeholder trait until SPR-002 lands) against a `Q16_16` probability, with a documented exact mapping
- **Acceptance:** Exhaustive test over the edge cases (probability 0, 1, the smallest step); frequency test against the exact probability
- **Status:** [ ] not started

### T-04: The float gate
- **Deliverable:** `bin/lint_forbidden.py` gains the V-DET-19 float rule for plane A: `f32`/`f64` only in items marked as declared float fields (`f32_field` in the system spec), with the same escape syntax and a positive control
- **Acceptance:** A float threshold planted in a plane-A fixture fails `bin/check_all.sh`
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation; the rounding rule against an exact rational reference | `VER-SPR-010` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| B-1 | The director's approval of this spec (C3) | `decisions_pending.md` | **Resolved:** approved 2026-10-16 |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-010` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
