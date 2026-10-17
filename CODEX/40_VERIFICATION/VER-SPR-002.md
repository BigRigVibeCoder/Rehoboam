---
id: VER-SPR-002
title: "Audit of SPR-002"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-002, VER-SPR-002-TEST]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** PASS. `rh-ids` delivers typed IDs, the keyed hash H, counter RNG streams, branch salts and genealogical PersonIds. The independent Tester passed all six tasks twice: first at layout v1, then **re-verified layout v2** (the epoch field added on the Architect's ruling) from the normative table in `05.1` §6 alone. Every defect is fixed. Change class C3 (determinism machinery); no golden world exists yet. The Architect merges (GOV-005 §6).

# VER-SPR-002 — Audit of SPR-002

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh` | PASS (77 checks) | Developer at `4c59386`; Tester at `185c7a6`; Architect at the audit commit |
| CI on GitHub-hosted runners | PASS at `4c59386` (run 38856354088); runs again on the audit commit | PR #4 |
| Nightly ignored-test run | Added (`.github/workflows/nightly.yml`): the full 10⁶-draw V-DET-20 sweep | `787abec` |
| Change class | C3: determinism machinery (`rh-ids`); no golden world exists | — |

## 2. Tester verdicts
| Tester | Contract verified | Verdict | Defects |
|---|---|---|---|
| AGT-003-DET, different model from the Developer | All six tasks at `10173bc` (layout v1): its own ChaCha8/SplitMix64 reference, validated against RFC 8439 and published SplitMix64 sequences, agreed on 3,207 known answers; 17 of 17 implementation mutations caught; 300 random fork trees | PASS | DEF-1 (doctest label), DEF-2 (doctest gate bypass) |
| Same Tester, re-verification at `4c59386` (layout v2) | A v2 reference written **from the normative table in `05.1` §6 item 2 only**: all 227 golden lines and 5,000 random keys agree (4,569 with a non-zero epoch; edge epochs; swapped and adjacent system/epoch pairs); 17 of 17 H and light-stream mutations caught; DEF-1 and DEF-2 confirmed closed | PASS | DEF-3 (formula wording) |

## 3. Defects and gaps: disposition
| Item | Fix | Commit |
|---|---|---|
| DEF-1: a compile-fail doctest carried the wrong error code | Label corrected to E0308, with the reason | `a56fea7` |
| DEF-2: demoting compile-fail doctests left `check_all` green | Gate counts the expected compile-fail doctests and their compiling twins per file, with a positive control | `3cfe30f` |
| GAP-2: ADR-0025's re-sample `epoch` had no field in the key | **Ruling:** an explicit `epoch: u32`, entering at K1 apart from `k`; H layout v2; golden vectors regenerated; V-DET-20 covers adjacent epochs | `dcabe9d` |
| GAP-1: H's byte layout lived only in crate docs | **Ruling:** the normative layout table goes into `05.1` §6 item 2; ADR-0006 points to it | `ed5c2d9` |
| GAP-7: `docs/09` did not state the PR/nightly split of V-DET-20 | Method text updated | `4c59386` |
| DEF-3: the headline formula without `epoch` survived in nine places | Formula now `H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)` in `05.1` §6, ADR-0006, CLAUDE.md, GOV-003, `docs/03` DET-2, the sprint and three crate doc comments. `05.1` also records that H is ChaCha8 (the "Philox or ChaCha8" choice is made) | Audit commit |
| GAP-3: `Purpose` has no entity kind; `PurposeId::of` is public, so the registry is advisory | Carried forward to SPR-005, where systems declare purposes from their specs and `lint_odd.py` checks `[[rng]]` against the code (BCK-002 A-18) | — |
| GAP-4: salt uniqueness relies on the Timeline Manager allocating unique ids | Carried forward to the timeline manager (P2.6) as an invariant to test | — |
| GAP-5: light-stream overlap odds understated in docs | Documented trade-off; light streams are opt-in and RngStream is the default. Docs to be corrected when light streams gain a caller | — |
| GAP-6: a collision rehash changes one person's id across branches | Accepted: collisions are about 2⁻⁶⁴ per pair; the rehash is deterministic within a branch. Noted for ADR-0026's next revision | — |
| Light-stream v2 layout is not in the normative table (the Tester verified it against crate docs only) | Accepted for now: it is not the canonical H, and the golden file pins it. It becomes normative if a system adopts light streams | — |
| OBS-4: a compile-fail doctest that fails for the wrong reason still passes | Known rustdoc limit (codes unchecked). Mitigated by the compiling twins and the per-file counts | — |

**Escapes found later:** one. DEF-3's sweep missed the doc comment on `rh_ids::hash::h`, which still gave the formula without `epoch`; found in the SPR-010 audit and fixed there (`VER-SPR-010` §3). Documentation only; the code and the golden vectors were already layout v2.

## 4. Acceptance criteria
| Task | Criterion | Met |
|---|---|---|
| T-01 | Compile-fail tests for mixing ID types | Yes (8 cases with twins; counted) |
| T-02 | Golden vectors: fixed keys to fixed outputs, committed | Yes (layout v2; independent generator and an independent Tester reference) |
| T-03 | Streams independent of call order; known-answer vectors | Yes |
| T-04 | V-DET-16 property test over random fork trees | Yes (300 trees, Tester's own path model) |
| T-05 | Forced collisions; ids stable across runs | Yes |
| T-06 | V-DET-20 within its band; PR tier under 30 s | Yes (PR tier 1.5–2 s; nightly sweep) |

## 5. Contract conformance
- **ADR-0006, `05.1` §6 item 2:** conforms to the normative layout v2 (Tester, independently).
- **ADR-0025 item 3:** the re-sample epoch is now part of the key.
- **ADR-0026:** genealogical ids `H64(mother, parity)` with a deterministic rehash.
- **GOV-003 §2 and `lint_arch`:** `rand_core` and `rand_chacha` only in `rh-ids`; no floats in `src/`; no hash-map iteration; no wall clock.
- **Contract edits in this sprint**, authorised by the Architect: the `05.1` §6 layout table, the ADR-0006 pointer, the `docs/09` V-DET-20 split, and the formula wording (DEF-3).

## 6. Verdict
| Field | Value |
|---|---|
| Verdict | **PASS** |
| Merge route | The Architect merges (GOV-005 §6); no golden world exists, so no out-of-tolerance announcement applies |
| Escapes found later | 1 (documentation only; `VER-SPR-010` §3) |
