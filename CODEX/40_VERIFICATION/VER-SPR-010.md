---
id: VER-SPR-010
title: "Audit of SPR-010"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-010, VER-SPR-010-TEST]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** PASS. `rh-core::fixed` delivers `Q8_8`, `Q16_16` and `Money` with saturating arithmetic, one rounding rule (half to even), guarded division, an exact draw-to-probability mapping, and the V-DET-19 float gate. The independent Tester found no defects; its 906,078-vector exact rational reference agrees with the code. Of the five contract gaps it raised, three are fixed and two are carried forward. The Architect probed the fix round with two mutations, and both were caught. Change class C3 (determinism machinery); no golden world exists yet. The Architect merges (GOV-005 §6).

# VER-SPR-010 — Audit of SPR-010

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh` | PASS (93 checks) | The Developer and the Architect, both at `9d28de2`, rebased onto `main` `8873fb3` |
| CI on GitHub-hosted runners | PASS at `9d28de2` (run 38858556458); runs again on the audit commit | PR #5 |
| Change class | C3: determinism machinery (the numeric types every later hash covers); no golden world exists | — |

## 2. Tester verdicts
| Tester | Contract verified | Verdict | Defects |
|---|---|---|---|
| AGT-003-DET, on a different model from the Developer | T-01 to T-04 at `a9962fd`, from the contracts first. 906,078 vectors from its own `fractions.Fraction` reference, independent of `bin/gen_fixed_vectors.py`. The 14,329 committed golden rows re-derived. Exhaustive draw blocks for every probability in `[-3, 65540]`. 99 float-lint cases. 26 `check_all` mutations, 25 caught | PASS | None. OBS-1 and OBS-2, GAP-1 to GAP-5 (`VER-SPR-010-TEST` §3–4) |

The fix round came after the Tester's run. It changed no behaviour the Tester verified: GAP-1 is wording plus pinned rows, GAP-3 adds lints, GAP-4 adds a test, OBS-2 changes a header string. The Architect verified it with mutations instead of a second Tester pass (§3).

## 3. Defects, gaps and observations: disposition
| Item | Disposition | Commit |
|---|---|---|
| GAP-1: `checked_div` saturates on overflow, though T-02 said "returns `Option`" | **Ruling:** keep the code. `None` means a zero divisor only, and an overflowing quotient saturates like every other result. T-02's wording is amended, `MIN / -1` golden rows are pinned for `Q8_8`, `Q16_16` and money, and the rule is recorded in the decisions log | `23d16d0` |
| GAP-3: a float never named in plane-A code passes both lint and clippy | **Ruling:** `clippy::float_arithmetic` and `clippy::float_cmp` are denied workspace-wide, with a rendered positive control that tests the real lint tables. **Architect probe:** removing `float_arithmetic` from the root `Cargo.toml` makes the control compile, so the gate goes red. **Residual:** no clippy lint covers float `<` and `>`. Such a float can only enter through an external dependency, so the gap closes at the dependency edge (BCK-002 A-20, the Developer's proposal) | `bb32742` |
| GAP-4: deleting the golden `--check` line left `check_all` green | Fixed beyond the recommendation: a gate-inventory test lists all 57 gates by exact name. **Architect probe:** deleting the line turns `test_every_required_gate_runs_in_the_chain` red. **Known limit:** the inventory runs inside the pytest gate, so deleting that one line silences it as well. Accepted: deleting a gate is a visible diff in review | `ffb8fbc` |
| OBS-2: the golden header said "None" while the rows say `none` | Fixed in the generator | `9d28de2` |
| GAP-2: an escape excuses a line, not an item | Carried forward to the first sprint that declares an `f32_field` (BCK-002 A-22). No spec declares one yet | — |
| GAP-5: no mixed-precision products and no `Q8_8` draw | Recorded so later sprints do not assume them; added when a system needs them | — |
| OBS-1: identifiers ending `_f32`/`_f64` are flagged | Accepted: fails closed, and the workaround is a rename | — |
| `UniformDraw` was a placeholder until SPR-002 | SPR-002 has landed. Implementing the trait for `rh_ids::RngStream` needs a crate edge, so it goes to the first sprint that draws against a probability (BCK-002 A-21). The doc comments now give layout v2 of H | Audit commit |
| Escape from SPR-002: the doc comment on `rh_ids::hash::h` gave H without `epoch` | Fixed here; recorded in `VER-SPR-002` §3 as that sprint's first escape | Audit commit |
| Rebase fix-up: the workspace float lints rejected `rh-ids`'s statistical test, which computes its bands in `f64` | A reasoned `#![allow(clippy::float_arithmetic)]` in that test file. `tests/` never links into the engine, the same exemption `lint_forbidden` gives | `bb32742` |

**Escapes found later:** none so far.

## 4. Acceptance criteria
| Task | Criterion | Met |
|---|---|---|
| T-01 | Compile-fail tests: no mixing of the three types, no float conversion from a plane-A crate | Yes. Five fixtures and a compiling twin, run by `check_all`; the Tester ran 32 compile probes. No float conversion exists under any feature |
| T-02 | Properties (saturation, identities, commutativity, ordering), rounding against an exact rational reference, golden vectors committed | Yes. 14,332 golden rows from `fractions` (the Tester's 14,329 plus the three GAP-1 rows), replayed by nextest and re-derived by `--check` |
| T-03 | Exhaustive edge cases; frequency against the exact probability | Yes. `P(hit) = clamp(p, 0, 1)` exactly; one draw per call whatever `p` is |
| T-04 | A float threshold planted in a plane-A fixture fails `check_all` | Yes. Six planted float controls, plus workspace and declared-field controls |

## 5. Contract conformance
- **`05.1` §6 item 4, DET-3:** `q8_8` is i16, `q16_16` is i32, money is i64 minor units; every operation is saturating.
- **`05.5` §2.1:** no floats, saturating arithmetic, and guarded division (`None` for a zero divisor).
- **ADR-0006, keyed RNG:** `chance` takes exactly one draw per call, so the counter discipline holds on every branch.
- **GOV-003 §2:** no `unsafe`; every cast carries a reason and an `INVARIANT`; no floats in `src/`.
- **Contract edits in this sprint**, authorised by the Architect:
  - T-02's wording (GAP-1);
  - the V-DET-19 method text in `docs/09`;
  - the workspace clippy float lints;
  - the formula wording of two doc comments.

## 6. Verdict
| Field | Value |
|---|---|
| Verdict | **PASS** |
| Merge route | The Architect merges (GOV-005 §6); no golden world exists, so no out-of-tolerance announcement applies |
| Carried forward | BCK-002 A-20 (plane-A dependency allow-list), A-21 (`UniformDraw` for `RngStream`), A-22 (escape per item) |
| Escapes found later | — |
