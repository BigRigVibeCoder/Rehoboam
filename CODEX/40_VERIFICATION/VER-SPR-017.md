---
id: VER-SPR-017
title: "Audit of SPR-017"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-017, VER-SPR-017-TEST, BCK-002, SPR-014, SPR-020, SPR-021, SPR-024, ADR-0006, ADR-0010, ADR-0033, ADR-0034]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: PASS.** SPR-017 delivers the Registry Effect Language v0.5.1 into `rh-registry` and Rehoboam:
> - **T-01:** Typed, total front end and parser with 16 compile-time error classes and totality invariants;
> - **T-02:** Kinship prelude walked over the life-course log (`children`, `kin`, `ancestors`, `descendants`, `lineage`, `line`, `up`, `common`, and agnatic variants), `holders_of` over tenure, and court relations;
> - **T-03:** Evaluator with fixed-point arithmetic, total semantics, deterministic sorting, and diagnostic tracing;
> - **T-04:** Succession protocol (7 laws, eligibility, tie-breaking) and verb effects yielding queued commands;
> - **T-05:** Golden test suite reproducing all 610 cases in debug and release builds; all 12 planted mutants killed;
> - **T-06:** Registry schema update `[effect_language]` and `[commands]`, loader verification, and real checking environment;
> - **T-07:** Host seam (`SystemViewHost`, `StepDraws`) over `SystemView` and `StepCtx`, converting actions to typed commands;
> - **T-08:** Differential fuzz driver (`bin/effect_fuzz.py`) verifying SplitMix64 seeds 1–16 with 100% coverage assertion against `effect_ref.py`;
> - **T-09:** Data-only extension test (`tests/registry_effect/`) verifying absolute primogeniture in reference and Rust.
>
> Verification & Rulings:
> - Tester findings in `VER-SPR-017-TEST` (Pass 1 & Pass 2) are ratified. DEF-1, DEF-2, and GAP-1..GAP-5 are verified closed.
> - Deviations 1–10 from the done report are audited and ACCEPTED.
> - All automated checks pass cleanly: `bin/check_all.sh` 100% across core, golden, and stream parts (586 gates chained and listed both ways, zero failures).
>
> Change class C2 (Registry schema & tooling). Merged with a merge commit pinned to the passing head commit (GOV-005 §6, GOV-009 §6).

# VER-SPR-017: audit of SPR-017

PR #90 merges with a merge commit, integrating Effect Language v0.5.1 into `main`.

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh --part core` | **PASS**: 0 FAIL rows. All unit tests, linters, and controls pass. | Architect local run on `3e03ed9` |
| `bin/check_all.sh --part golden` | **PASS**: 0 FAIL rows. Release golden tests, 12 mutants killed, differential fuzz, golden runs pass. | Architect local run on `3e03ed9` |
| `bin/check_all.sh --part stream` | **PASS**: 0 FAIL rows. All 37 stream checks pass. | Architect local run on `3e03ed9` |
| `python -I bin/check_all_selftest.py inventory` | **PASS**: 586 gates chained and listed, each once, both ways. | Clean inventory pass |
| `cargo fmt --all --check` | **PASS**: All formatting clean across workspace. | Rustfmt check |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**: Zero warnings across all workspace crates and targets. | Clippy check |
| `bin/gen_manifest.py --check` | **PASS**: Validates cleanly. | Manifest check |
| Change class | C2 (Registry schema, effect language engine, tooling) | — |

## 2. Tester verdicts (VER-SPR-017-TEST)
| Pass | Result | Focus |
|---|---|---|
| Pass 1 (`eab84d5`) | **PASS with DEF-1, DEF-2** | 558/558 golden cases bit-equal; identified DEF-1/DEF-2 on integer literal division/modulo under fixed-point context. |
| Erratum 0.5.1 (`5f0312c`) | **PASS** | Evaluator and reference updated; relations are not values; `all` where semantics fixed; 52 golden cases added (610 total). |
| Pass 2 (`34a7f56` / `3e03ed9`) | **PASS** | T-06 commands lint, T-07 host seam probes, T-08 differential fuzz and mutant power verified across all 12 mutants. |

## 3. Findings and defects disposition
| ID | Finding | Ruling | Status in Head |
|---|---|---|---|
| DEF-1 | `div`/`mod` on integer literals taking fixed-point type caused internal division by zero | FIX. Strict rule §4.1 rule 7: operands demand `int`; literals resolve to `int`. | Closed in `5f0312c` |
| DEF-2 | Python reference accepted `div`/`mod` fixed-point quotient | FIX. Synchronized with §4.1 rule 7. | Closed in `5f0312c` |
| GAP-1 | Truncated count tracking on nested evaluation | FIX. Golden test cases added (`own_trunc_*`). | Closed in `5f0312c` |
| GAP-2 | No-break space rejected as syntax error | FIX. Added golden test row. | Closed in `5f0312c` |
| GAP-3 | Parentheses depth boundary and RNG key collision | FIX. Bound at 63/64 depth and RNG reuse test added. | Closed in `5f0312c` |
| GAP-5 | Parameter syntax errors in command signatures | FIX. Parameter validation and duplicate checks verified. | Closed in `661a558` |

## 4. Audit of Deviations 1–10
| # | Deviation | Architect Ruling | Rationale |
|---|---|---|---|
| 1 | `effect_env` takes Registry text rather than file path | **ACCEPTED** | Enables fast in-memory compilation and synthetic fixture testing without disk IO. |
| 2 | `rh-registry` depends on `rh-events` | **ACCEPTED** | Essential for shared `EventRef` causality definitions and command origins. |
| 3 | `StepDraws` keyed on persons and regions only | **ACCEPTED** | Satisfies all 7 succession laws; expansion to arbitrary entity types tracked for SPR-021a. |
| 4 | `SystemViewHost` never answers `Neutral` | **ACCEPTED** | Neutral is statically compiled into reads (§4.5); absent owner fallback verified with SPR-014. |
| 5 | M7's `person.id` read declared in stub | **ACCEPTED** | Required for identity resolution through the seam; frozen spec updated in SPR-021. |
| 6 | Sortition checked against step's draw order | **ACCEPTED** | Draw values originate from the engine's deterministic stream; permute tests confirm invariance. |
| 7 | FLED's test-only office 299 | **ACCEPTED** | Isolated mock entity ID for resignation command unit tests. |
| 8 | Nightly fuzz timeout of 60 minutes | **ACCEPTED** | Sufficient headroom for 1,024 seeds at ~0.8s per seed. |
| 9 | Mutant build `--features rh-manifest/mutant-effect` | **ACCEPTED** | Fully integrated with BCK-002 A-52 build guards and automated gate detection. |
| 10 | Golden release run and mutants placed in part golden | **ACCEPTED** | Conforms to three-part CI partition policy to keep part core under 12 minutes. |

## 5. Audit conclusion
SPR-017 meets all requirements of `specs/registry/effect_language_v0.md` v0.5.1 and CODEX governance. Status is **CLOSED**.
