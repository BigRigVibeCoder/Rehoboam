---
id: VER-SPR-002-TEST
title: "Contract tester evidence for SPR-002"
type: reference
status: CLOSED
owner: tester
agents: [tester]
tags: [verification, testing, determinism]
related: [SPR-002, VER-SPR-002, ADR-0006, ADR-0026]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** T-01 to T-06 pass every case the contracts name, and `bin/check_all.sh` (75 checks) is green on clean code and turns red on 8 of 9 clean gate mutations. All 3,207 known-answer checks agree with my own ChaCha8/SplitMix64 reference, which I validated against RFC 8439 and Vigna's published sequences. All 17 mutations of the implementation are caught, including every mutation of H that V-DET-20 was meant to see. Two low defects remain, none blocking: a wrong error code on one compile-fail doctest (DEF-1), and a gate that stays green when every typed-ID compile-fail case is removed (DEF-2). Six contract gaps need an Architect ruling. The two that matter most: ADR-0025's `epoch` has no place in the RNG key (GAP-2), and the byte layout of H lives only in the code docs, not in an ADR (GAP-1).

Tested commit: `10173bc` on `sprint/SPR-002-ids-rng` (implementation `182f9ca`, plus the Developer's nightly-workflow commit). Probes were written from the contracts before the implementation was read (plan written first; it named the layout as a likely gap). All mutation work ran in throwaway copies of an exported tree; the repository's code was never modified. Model and tester independence: this tester ran on a different model from the Developer (GOV-005 §4.2); the Architect records which in `VER-SPR-002`.

## 1. Verdicts

| Gate | Verdict | Basis |
|---|---|---|
| T-01 typed IDs (compile-fail) | **PASS, DEF-1 and DEF-2 open** | 284 compile cases built with `rustc` (every ordered pair of the six ID types × argument, `==`, `<`, `From`, `.into()`, tuple constructor; every type × integer arg/`.into()`/arithmetic/`+=`/index/`as`/deref/`default`/private field; seven compiling controls). 248 failed with exactly the predicted code; 36 differed only because `T::from(x)` resolves to the reflexive impl and reports E0308, not E0277 (my expectation; still a type error for the stated reason). Controls compile. |
| T-02 keyed hash H, V-DET-09 | **PASS** (GAP-1) | 207 committed vectors, plus 3,000 random keys incl. `k` ∈ {7, 8, 2⁴⁰, u64::MAX}, `t` = i64::MIN, `system` = u32::MAX, agree with my reference. Output is byte-identical across processes and across debug/release. |
| T-03 counter RNG | **PASS** (GAP-3, GAP-5) | Order independence holds for 50 shuffled interleavings mixing `h`, `RngStream::draw` and `StreamKey::draw`, and across 8 threads. Every bit of every key field changes the draw, and no two of 4,096 small-value keys collide (no field aliasing). Duplicate purposes are rejected at compile time (E0080) and at run time. |
| T-04 salts, V-DET-16 | **PASS** (GAP-4) | 300 random fork trees (equal-tick siblings, forks before a timeline's own fork point, what-if and reroll forks, 17 probe ticks each) checked against my own path model: equal path ⇔ equal salt, and the empty path equals `CANONICAL_SALT`. `resample_on_fork` purposes redraw across a fork, others do not, siblings differ, a what-if matches its parent's draws exactly. |
| T-05 genealogical PersonId | **PASS** (GAP-6) | Stable across runs; the n-th child of a mother has the same id in a branch with extra births; forced collisions rehash deterministically, independent of load order; chain walk and gap behaviour correct; no id is ever reissued. |
| T-06 V-DET-20 | **PASS** | PR tier: 1.5 s, 8 exceedances of 6,000 (expected 6.0, allowed 15). Full 10⁶ sweep (run once by me): 23 s release, 6 exceedances (expected 6.0). Sensitivity: 7 of 7 mutations of H and 2 of 2 of the light stream fail the harness. |
| V-DET-19 on rh-ids | **PASS** | Clean in `check_all` (clippy `-D warnings`, `lint_forbidden`, `lint_arch` rng-crate rule). Not re-probed; SPR-001 evidence covers the gate. |
| `bin/check_all.sh` | **PASS** | 75 checks green on a clean export after `uv sync --frozen`; see section 3 for gate mutations. |

## 2. Results

| Probe | Clause | Expected | Actual | Verdict |
|---|---|---|---|---|
| Reference self-check | V-DET-09 | RFC 8439 §2.1.1, §2.3.2, §2.4.2; SplitMix64 seeds 0 and 1234567 | all match | PASS |
| Golden file vs reference | V-DET-09 | 207 lines agree | 0 mismatches (12 kinds) | PASS |
| 3,000 random keys | V-DET-09, T-03 | H, `RngStream` seek + 3 sequential, `LightStream` equal reference | 0 mismatches | PASS |
| Compile-fail matrix (284) | T-01 | each fails with predicted code | 248 exact; 36 expectation errors (reflexive `From`); 0 unexpected successes | PASS |
| Developer's 8 compile-fail doctests built individually | T-01 "fails for the stated reason" | stated code = actual code | 7 of 8; `TimelineId::from(PersonId)` is labelled E0277, actual E0308 | DEF-1 |
| Order independence, threads | T-03 | identical | identical | PASS |
| One-bit flips (6 fields, 64 bits) | T-03 | every bit changes the draw | all do; no cross-key collision | PASS |
| Random fork trees ×300 | V-DET-16 | injective on distinct paths, canonical empty | holds | PASS |
| `resample_on_fork`, what-if, reroll, grandchild | V-DET-16 | as 05.1 §6.9 | holds | PASS |
| PersonId stability, collisions | ADR-0026 | deterministic rehash | holds | PASS |
| Golden regeneration ×2 | T-02 | byte-identical | identical SHA-256 | PASS |
| Golden tamper | T-02 | `--check` and Rust golden test fail | both fail | PASS |
| Golden file with CRLF line ends | T-02 | not byte-identical | `--check` accepts it | OBS-1 |
| PR-tier time | T-06 | < 30 s | 1.5 s (dev profile) | PASS |
| Implementation mutations (17) | all | each caught | 17 of 17 caught (list below) | PASS |

Implementation mutations, all caught by at least the unit tests and the golden or probe suites. For H and the light stream the V-DET-20 harness also went red: entity low bit, branch salt, `t` low bit, purpose low bit, seed low bit, system low bit, draw index wrapping at 2¹⁶, light-stream entity bit, light-stream salt. Salt and person mutations went red in `fork_trees`: salt ignores timeline id, `<` for `<=`, `resample_on_fork` ignored, what-if extends the salt, fork before own fork point keeps later steps, rehash counter stuck, parity ignored, duplicate-name check removed.

## 3. `check_all.sh` gate mutations

| Mutation | Result | Red at |
|---|---|---|
| Golden vector tampered | red | nextest (golden test) |
| H ignores entity low bit | red | nextest |
| A typed-ID compile-fail case made to compile | red | doctests |
| V-DET-20 test dropped from the named gate | red | named rh-ids gate |
| Stale-golden positive control replaced by a fresh file | red | positive control |
| `resample_on_fork` ignored | red | nextest |
| V-DET-20 PR test `#[ignore]`d | red | named rh-ids gate |
| H mutated and harness threshold neutered | red | nextest (golden) |
| **All typed-ID compile-fail doctests demoted to plain text** | **green** | **DEF-2** |
| Harness threshold neutered alone | green | expected: test weakening is a review matter |

## 4. Defects

**DEF-1 (low): wrong error code on one compile-fail doctest.**
- Clause: T-01 acceptance, "each compile-fail case fails for the stated reason".
- Reproduction: build the doctest in `crates/rh-ids/src/ids.rs` (`let id: TimelineId = TimelineId::from(PersonId::from_raw(1));`) as a binary with `rh-ids` as a dependency.
- Expected: E0277 (as labelled). Actual: E0308 (`expected TimelineId, found PersonId`).
- It does fail for a type-mismatch reason and the twin compiles, so the safety property holds. Stable rustdoc ignores the code, so nothing checks the label. The module docs say the twin is what proves the reason; the label should be corrected or dropped.

**DEF-2 (low to medium): the doctest gate cannot see the typed-ID cases disappear.**
- Clause: SPR-002 T-01 acceptance; GOV-002 law on gates that can go silent.
- Reproduction: in a copy, replace every `compile_fail,E…` in `ids.rs` with `text` and run `bin/check_all.sh`.
- Expected: red. Actual: green, because the check only looks for the string `compile fail ... ok`, which the one case in `rng.rs` still prints.
- Suggested fix: count the typed-ID cases (8), or run them under a named test target.

**Closed during the sprint:** the full V-DET-20 sweep was documented as nightly but no workflow ran it. The Developer's commit `10173bc` adds `.github/workflows/nightly.yml` (daily, `--run-ignored ignored-only`). I could not observe a GitHub Actions run from here, so the schedule itself is untested.

## 5. Observations

- **OBS-1:** `gen_rng_golden.py --check` accepts a CRLF copy of the golden file (text-mode read), so "byte-identical" is checked by content, not bytes. Harmless today; a Windows nightly with autocrlf would not break on it.
- **OBS-2:** the PR tier draws 2¹⁶ per stream, not the 10⁶ in `docs/09`; the full sweep takes 23 s in release, so the PR tier has room for more. The sprint allows the split; `docs/09` does not state it (GAP-7).
- **OBS-3:** V-DET-20 as written tests cross-stream dependence only. No mutation of within-stream structure (for example a repeated half-word) was tried, and the harness has no serial test; I did not verify either way.

## 6. Contract gaps (for the Architect)

1. **GAP-1, medium.** ADR-0006 and `05.1` §6.2 fix H's inputs but not its byte layout, the ChaCha key/nonce/counter mapping or the salt composition. They exist only in `crates/rh-ids` docs. My reference is therefore independent of the Developer's *code* and of the primitives, not of the layout. Either amend the ADR to cite layout v1 as normative or say the crate docs are the contract.
2. **GAP-2, medium.** ADR-0025 item 3 and ADR-0006 item 8 put the entity `epoch` in the re-sample key. `RngKey` and H have no epoch field and `EntityKey` is a lossless `u64`, so there is no home for it. Decide whether it folds into `entity`, `t` or a new field before SPR-003 sampling.
3. **GAP-3, low to medium.** `Purpose` carries no entity kind, although the `ids.rs` docs and the `[[rng]] key` spec field say it does. `RngKey.entity` is a raw `u64`, so typed IDs end at the key; and `PurposeId::of` and `from_raw` are public, so a draw can use an undeclared purpose. The registry is advisory unless a lint enforces membership.
4. **GAP-4, low.** V-DET-16 "no two rollouts share a salt" holds only if the Timeline Manager allocates unique `TimelineId`s. `fork` checks only its own lineage (by inspection: two siblings given the same id get the same salt).
5. **GAP-5, low.** `LightStream` draw `k` is `mix(base + (k+1)·γ)`, so two streams whose bases differ by `j·γ` are shifted copies. The doc quotes a 2⁻⁶⁴ per-pair collision; the chance of *overlap* is about 2L times larger for L-draw streams (analysis, not run). Still harmless for scalar uses; the doc should say so.
6. **GAP-6, low.** A collision rehash changes a PersonId only in the branch that has the collision, so that one person differs across branches (CRN pairing loses it). The ADR accepts rarity but does not say this.
7. **GAP-7, low.** `docs/09` V-DET-20 states 10⁶ draws with no PR/nightly split; the sprint splits it (see OBS-2).

## 7. Reproduction notes

Scratch material (reference implementation, probe plan, 284-case compile matrix, 17-mutation script, gate script) was kept outside the repository. The reference is about 90 lines of Python with no dependency on `bin/gen_rng_golden.py`. The independence claim: ChaCha is checked against RFC 8439 at 20 rounds and then run at 8 rounds, which is the same code path; the 8-round output is confirmed against `rand_chacha` through the 3,000-key comparison, not by an external 8-round vector.
