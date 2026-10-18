---
id: VER-SPR-017-TEST
title: "Contracts and determinism tester evidence for SPR-017 (effect language v0)"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing, determinism]
related: [SPR-017, REV-SPR-017, VER-SPR-013-TEST, AGT-003-DET, AGT-003-IF, ADR-0006, ADR-0010]
created: 2026-10-18
updated: 2026-10-18
version: 0.2.0
---

> **BLUF: Checkpoint pass (T-01 to T-05) on `eab84d5`: the evaluator is sound against everything I threw at it, with one real defect. DEF-1: `div` and `mod` on integer literals that take a fixed-point type from context pass the checker, then abort evaluation with an internal "division by zero" (the reference accepts them too and returns a wrong value, DEF-2). Everything else holds: 558 of 558 golden cases bit-equal in four builds, 354 of 354 of my own cases, about 99,000 differential-fuzz programs with one mismatch (DEF-1), 14 of 14 of my mutants and 12 of 12 of theirs killed, and `bin/check_all.sh` 542 of 542. The golden file pins less than it should: it kills 4 of my 14 mutants not at all (GAP-1 to GAP-3).**

Tested head: `eab84d5cf5d373d2fc532358c2ba17e4260f7a49` on `sprint/SPR-017-effect-language` (PR #90), which holds `071edf0` (reference: ASCII whitespace, totality, chains), `c9e06cc` (spec errata) and `731ccae` (Rust). Tester model: `claude-sonnet-5-5` (configured); the Developer's commits are by a different model.

Method: clean clone of the SHA, own `CARGO_TARGET_DIR` per build, the Developer's files untouched. The only file I added to the clone was an untracked probe test, never pushed. I wrote the probe plan before any implementation existed, from the spec, the golden header, SPR-017 and REV-SPR-017. Oracles: hand derivation, `probes/oracle.py` (my exact-fraction arithmetic and kinship/lineage walks, no code shared with `effect_ref.py`), and `effect_ref.py` as third opinion. I ran the Rust evaluator on my files through the Developer's own runner (`RH_EFFECT_GOLDEN=<file> cargo test -p rh-manifest --test effect_golden`), so a file is "golden-format" and nothing else is trusted. GitHub CI ran on `eab84d5`: both jobs green.

## 1. Verdicts

| Task | Verdict | Basis |
|---|---|---|
| T-01 front end | **PASS with DEF-1, GAP-2, GAP-3** | 163 golden rejections give their code; 10^5 adversarial programs of every recursion path pass in a 2 MiB test thread in debug; no panic in about 99,000 mutated programs (§2.1) |
| T-02 values, arithmetic, reads, draws | **PASS with DEF-1, DEF-2** | exact half-even and saturation on 42 integer and 28 fixed-point cases of mine; draws and the ledger (§2.2) |
| T-03 prelude | **PASS with GAP-1** | 26 truncation cases at and over every bound; kinship, tenure and M7 relations agree with my oracle (§2.3) |
| T-04 succession and effects | **PASS** | all seven laws on a dynasty of mine, twins, a 16-year boundary to the minute, 27 effect cases, verb ledger (§2.4) |
| T-05 golden, both builds, mutants | **PASS with GAP-1 to GAP-3** | 558/558 in debug, release, native and overflow-checks; 12/12 mutants for their reasons (§2.5) |
| Determinism properties | **PASS** | row-order invariance, tenure order semantic, `Arc<Compiled>` at 1/4/16 threads (§3) |
| `bin/check_all.sh` whole | **PASS** | 542 of 542 on a clean clone (§4) |
| Ladders, baselines | **PASS** | smoke and solo at 1 = N threads; no file under `worlds/` or `baselines/` changed |

T-06 to T-09 are not claimed: Pass 2.

## 2. Contracts (AGT-003-IF)

### 2.1 T-01
| Probe | Result |
|---|---|
| 354 own cases (§1 of my plan), hand-derived, on 6 worlds, with `effect_ref.py --check` and the Rust runner | ref 354/354, Rust debug 354/354, Rust release 354/354. Before the Developer's reference fixes the reference passed 343; the 11 differences were F-1/F-2 |
| The 10^5 cases, each recursion path: `(`, `-`, `not`, `if`, `abs(`, tuple, `+`/`and` chains, 10^5 arguments, 10^5 statements, 10^5 comments, 10^6 spaces, 10^5-digit decimals on a half-ulp tie and plus-epsilon, 10^5-digit int | parens/calls/`if`/tuple: `E_BOUND`; chains and prefix runs: the reference's value (100000, true, 1, true); ties exact; the whole 354-case file runs in 4.6 s in debug, so no case is near a minute |
| Non-ASCII: NBSP, U+2003, U+3000, U+0085, VT, FF, ZWSP, BOM, NUL, accented and emoji identifiers; non-ASCII in comments | `E_SYNTAX` outside comments, accepted inside (F-1 as ruled) |
| Boundaries: 63 nested parens pass, 64 are `E_BOUND`; degrees 0/1, 6/7, 3/4, 16/17; bound products 65,536 pass, 262,144 and 1,048,576 `E_BOUND` | as the reference |
| Differential fuzz (my generator, `probes/gen_variant.py`: token and character mutants, splices, boundary numbers, non-ASCII, 70-deep prefixes), expected values from the reference, run through the Rust runner. Seeds `0x5eed0017` and `0x5eed0100 + 7919 i`, i = 1..12, on the golden and my own file | about 99,000 programs; all but one agree on result text and the four diagnostics. **Error codes agree exactly even on multi-fault programs** (0 code differences). The one mismatch is DEF-1. The reference skipped about 1 in 3,500 mutants (a draw key the world does not list; T-08's `draw_seed` ruling covers it) |
| No panic | none in any run above (a panic would fail the test with its message) |

**DEF-1 (Rust, medium).** *Clause:* §3.3 (`div`, `mod`: `int × int → int`), §4.8 (the evaluator has two failure paths only), §10 `E_TYPE` ("`int` mixed with fixed point"). *Repro* (`kind = "expr"`, system `M7.dynasty`): `q16(1) + 7 mod 2`, `q8(1) + 7 mod 2`, `0.5 + 7 mod 2`, `0.001 - 7 mod 2`, `7 mod 2 + 0.5`, `(7 mod 2) * 0.5`, `q16(1) + 7 div 2`. *Expected:* `error:E_TYPE`: `div` and `mod` fix their operands to `int`, so a literal operand cannot take the fixed type its neighbour demands. *Actual:* the checker accepts all seven; evaluation then fails with `crashed: evaluator invariant broken: a division by zero at run time` (the fixed-point arm has no `mod`/`div`). `3 mod 2.0`, `q8(1) mod 2`, `1.5 mod 2` and `q16(1) mod q16(2)` are correctly `E_TYPE`, so the hole is the literal adoption only. *Why it matters:* a program that passed the checker aborts the host, which is what totality (§4.8) forbids. Found by the fuzz; no golden row reaches it.
**DEF-2 (reference, medium).** Same programs, same cause: `effect_ref.py` accepts them and evaluates `7 mod 2` as the *quotient* in the fixed type (`0.001 - 7 mod 2` = `q16_16:-229310`, which is 0.001 − 3.5). Expected `error:E_TYPE`. Both sides must be fixed, with golden rows.

### 2.2 to 2.4 (T-02 to T-04)
Integer `/` (8 sign-tie cases), `div`/`mod` with negative divisors (7), saturation at both ends (`abs(MIN)` built without a MIN literal), decimal ties at q8_8 and q16_16 (0.5, 1.5, 2.5, 3.5 ulp), product and `narrow` ties, `round`, `floor`, `clamp` with `lo > hi`, `safe_div`: all agree with hand values and the reference. Draws: `u >> 1`, `chance` on the unshifted `u`, `p = 0` with `u = 0` is false, `p = 1` with `u = 2^64 - 1` is true, a one-ulp `p` hits at `u = 2^48 - 1` and misses at `2^48`, `k` as an entity, an alias pair (`x` and `holder` the same person) raises `E_RNG_REUSE`, a person-typed `k = 7` against `int k = 7` collides (GAP-C as ruled), a loop-keyed draw sums correctly, `evaluations = 3` is idempotent. Verb ledger: three distinct actors, the same set reordered, a repeat in the middle, a repeat reordered (same verdict), the same actor with another target (independent), and two sites in one program (`E_RNG_REUSE`).

Succession on my dynasty (17 persons, row ids differing from person ids, a dead elder son with a living son, a dead childless son, twins with equal keys, a 16-year boundary to the minute, a one-parent branch): the seven lines equal my oracle's direct semantics. Effects: typed encodings (`-0.1` as q8_8 is `-26`, enum ordinals, `event`/`none`, dropped actions between kept ones, empty program, `for` over empty), 14 rejections.

### 2.3 Truncation (no golden case pins it)
Every bounded relation at the bound (diagnostic 0) and one over (1), and which end is dropped: `children` 64, `kin` 256, `descendants` 256 (kept ids 2..257 exactly, 258 dropped), `titles`, `offices`, `claims_of` 16, `vassals`, `claims_on` 64, `court` 128, `vows_of` 8. All 26 agree between reference and Rust. This is what GAP-1 is about.

## 3. Determinism (AGT-003-DET)
| Property | Probe | Result |
|---|---|---|
| Debug = release | golden 558 and own 348 in debug, `--release`, release + `target-cpu=native`, release + `overflow-checks` + `debug-assertions` | identical pass in all four; own 354 in debug and release |
| Row order | a transformer permutes every row table of every golden and own world except `tenure`, and re-numbers the life-course row ids; seeds 1, 2, 3 | 558/558 and 348/348 unchanged each time |
| Tenure order is semantic (control) | reverse the tenure row ids | exactly the tenure-dependent cases change: 7 of 558 (`holders_of_*`, `deep_holders_of_*`, `rota_*`) and 6 of 348; reference and Rust agree on which |
| Threads | one `CompiledLaw`/`Compiled` is `Send + Sync` (asserted at compile time, with `Env`); the golden and own cases evaluated at 1, 4 and 16 threads, 20 repetitions each, case order rotated per thread | 0 failures in about 380,000 evaluations |
| Static | `grep` over `src/effect/`: no `static`, `thread_local`, `Cell`, `OnceCell`, `HashMap`, `HashSet`, `f32`, `f64`, `Instant`, `SystemTime`, `unsafe`, `std::fs`; `unwrap` only in tests | clean |
| Draw ledger | §2.2 | per evaluation for laws, per step for verbs, as ruled |

Not run yet (T-07): `SystemView` refusals with a planted `AccessSpec`, and row permutation through a host with unsorted storage. The runner builds worlds into `BTreeMap`s, so permuting rows at that level can only prove the loader, not a host.

## 4. Mutants, whole check, CI
**The 12 golden mutants** (`--features mutant-effect`), failing cases with the golden file: `LogByRowId` 91, `ShortCircuit` 2, `LineageHeightTwo` 5, `BoundBeforePrune` 2, `SilentCut` 1, `NeutralIsZero` 2, `MissingEnumOrdinalZero` 1, `MulHalfUp` 2, `MotherFirst` 4, `LedgerPerStep` 1, `TieBreakDescending` 11, `PrefixLast` 11; `Mutant::None` passes all 558. Each fails its sentinel with the wrong value it predicts (check_all's `mutants_are_killed`, and my own run). Without the feature the mutants change nothing.

**My 14 mutants** (patches to a copy; each run on golden and on my file):

| Mutant | Golden kills | Own kills |
|---|---|---|
| M01 `/` ties away from zero | 5 | 7 |
| M02 `mod` truncating | 2 | 5 |
| M03 `chance` on the shifted draw | 6 | 2 |
| M04 `draw` unshifted | 6 | 5 |
| M05 truncation counted per dropped element | **0** | 11 |
| M06 truncation never counted | **0** | 18 |
| M07 `holders_of` oldest first | 5 | 5 |
| M08 ledger never records | 2 | 5 |
| M09 `lineage` prunes every dead member | 3 | 2 |
| M10 non-ASCII whitespace accepted | **0** | 1 |
| M11 decimal ties round up | 2 | 4 |
| M12 depth limit 65 | **0** | 1 |
| M13 ledger key ignores `k` | 3 | 2 |
| M14 ledger key separates `k`'s type (GAP-C) | **0** | 1 |

All 14 die under golden + mine; four survive the golden file alone.

**`bin/check_all.sh`**: 542 of 542, both parts, clean clone, no uncommitted file. The Developer's count matches. It includes smoke 1,000 years at 1 and N threads (ladders equal) and solo 200 years at 1 and N threads, both baselines equal to the recomputation, the A-52 refusal control, V-PERF-04, and the Rust golden test in debug and release with its positive control. `git diff origin/main -- worlds` is empty; the golden file is not in the diff; `effect_ref.py --check` reproduces 558 and my 354.

**CI on `eab84d5`** (PR #90, run 39087048857): `core` 12 m 09 s in `check_all.sh`, `golden` 11 m 09 s. Both green.

## 5. Defects and gaps
| # | Kind | Item | Owner |
|---|---|---|---|
| DEF-1 | defect, medium | `div`/`mod` on literals adopting a fixed type: accepted, then internal error at run time (§2.1) | Developer (Rust) |
| DEF-2 | defect, medium | the same programs accepted by `effect_ref.py` with a wrong value | Developer (reference) |
| GAP-1 | test strength | no golden row pins `truncated`: a per-element or a never-counted diagnostic passes all 558 (M05, M06) | Architect: add rows at 0.5.1 |
| GAP-2 | test strength | no golden row pins ASCII-only whitespace (M10) | Architect |
| GAP-3 | test strength | no golden row pins the nesting boundary 63/64 (M12) or the person-`k` against int-`k` collision (M14) | Architect |
| GAP-4 | observation | the `core` CI part took 12 m 09 s against the plan's 12 minutes (15 hard); `golden` 11 m 09 s | Architect |

For GAP-1 to GAP-3 my case ids `own_trunc_*`, `own_rej_11`, `own_adv_paren_63`, `own_adv_paren_64` and `own_draw_13` are ready to become golden rows once the Architect approves them. DEF-1 and DEF-2 want golden rows too (`q16(1) + 7 mod 2` and its siblings, `E_TYPE`).

## 6. Next
The 0.5.1 erratum (a relation-typed expression program, `if` or key element is `E_TYPE`; `all` means every element satisfies `c`) will be checked against my oracle on its arrival; DEF-1 and DEF-2 fit the same commit. Pass 2 (T-06 to T-09) covers the real environment, the Registry refusals and `[commands]` lint with my planted fixtures, the host seam, the Developer's fuzz with my seeds and a planted-reference control, and the ladders again.

## 7. Pass 2 (handoff draft, head `34a7f56`; unfinished parts say "not run")

The director paused this lane and handed the project on; this section is the state at the pause. Probes and scripts are in `tests/tester_cases/spr017/probes/` on the handoff branch; my 389-case file is on `review/SPR-017-tester-cases`.

| Task | Verdict | Basis |
|---|---|---|
| 0.5.1 (DEF-1, DEF-2, erratum 2) on `5f0312c` | **PASS** | §7.1 |
| T-06 on `661a558` | **PASS** | `check_all` 550 of 550; §7.2 |
| T-07 host seam | **PARTIAL PASS, not complete** | §7.3 |
| T-08 fuzz | **PASS on what I ran; power partly run** | §7.4 |
| GAP-5 | **PASS** | §7.5 |
| T-09 extension, four gates | **not run** | |
| `check_all.sh` whole on `34a7f56` | **not run** (two attempts were cut by a worker restart; the 503rd check was passing when I stopped) | |
| Ladders, CI times on `34a7f56` | **not run** | |
| The developer's five deviations | **partly assessed** | §7.6 |

### 7.1 0.5.1
`q16(1) + 7 mod 2` and 21 siblings are `E_TYPE` in the reference and in Rust. 60,000 random arithmetic programs (int, q8_8, q16_16, `div`, `mod`, `/`, the builtins, every position; `gen_arith.py`) and 40,000 more mutated programs: 0 mismatches. The four golden-surviving mutants (M05, M06, M10, M12, M14) now die on the golden file alone. 35 new cases of mine for erratum 2 (relations are not values, `all`'s `where`) agree in the reference and Rust, debug and release (389 cases).

### 7.2 T-06
See `probes/t06/` and the notes in the Architect's message of that date: 13 `[commands]` equal the 13 `[[applies]]` by an independent tomllib script; 13 enums, 21 law texts, `refs` and `opaque` columns agree; M7 has 11 identity reads; `key_order()` agrees with tomllib on 62 nasty sections and on the real Registry plus 4 shuffles; about 30 planted refusals and 19 single-fault `lint_odd --commands` fixtures all fail for the right clause; verb rules (Q8, Q13, shared sites, payload plus draw) hold.

### 7.3 T-07 (probe copy of `effect_host.rs` with a shuffle parameter: `zz_host_probe.rs.txt`)
| Probe | Result |
|---|---|
| Row order through `SystemView`: the life-course log, titles, offices, claims, vows and artifacts permuted (12 seeds; the ledger and the tenure log are not, the engine's `ledger-sorted` invariant forbids the first and the second is semantic); 7 laws x 2 titles x 2 evaluations | 12 x 28 evaluations equal the unpermuted lines, sortition included |
| Each of M7's 18 declared reads dropped in turn from the `AccessSpec` | 7 refused as `SimError::UndeclaredRead` naming exactly the dropped column (`life_course.birth_tick`, `.father`, `.mother`, `.person_id`, `.sex`, `person.alive`, `person.id`); the 11 the laws never read leave every line identical; no silent zero |
| Absent owner | the stub engine still serves the columns from `WorldTables` when an owner is not registered, so the seam is not exercised on its neutral path: not testable before SPR-014 (the host never answers `Neutral`) |
| Reads only through `SystemView`, draws and submits only through `StepCtx` | by reading `seam.rs`: every column read goes through `SystemView::read`, draws through `StepCtx::rng`; no other access. `static` grep clean |
| Draws and queued commands, `to_command` with event arguments | not run beyond the Developer's own tests |

### 7.4 T-08
`bin/effect_fuzz.py --seeds 1-16 --assert-coverage`: PASS (2,608 cases, 0 mismatches, 0 crashes, 40 s), coverage counters all non-zero. My seeds 2001-2048: PASS (7,824 cases); the nightly range edge 1017-1024: PASS (1,304); seeds 0, 1, 2^32 and 2^64-1: PASS (652). Power through the `mutant-effect` build with my seeds 2001-2016: **all 12 T-05 mutants are caught**, the first mismatch at seeds 2001-2004 (LogByRowId, ShortCircuit, LineageHeightTwo, BoundBeforePrune, NeutralIsZero, MulHalfUp, LedgerPerStep, TieBreakDescending, PrefixLast at 2001; MissingEnumOrdinalZero 2002; MotherFirst 2003; SilentCut 2004). Not run: planted reference mutants (half-up products, unshifted draw, mother-first `common`), my own M01 to M14 as Rust builds against the fuzz, the check that each of the three new generators is exercised, the 60-minute nightly timeout.

### 7.5 GAP-5
`perzon` as a command parameter type: `E_UNKNOWN_NAME: 'perzon' is not a language type or a known entity kind`, exit 1. A duplicated parameter: `parameter 'about' is declared twice, at positions 1 and 2`, exit 3. Both through `rh-cli effect check`. Closed.

### 7.6 The developer's deviations
| Deviation | Does it hide a defect? |
|---|---|
| `StepDraws` keyed on persons and regions only | No for the seven laws (sortition keys a person). A law drawing on a title or office entity gets `DrawUnavailable`, an `EvalError`, not a silent value. SPR-021a widens it |
| `SystemViewHost` never answers `Neutral` | No defect, but `neutral_reads` through the seam is untested until SPR-014 (§7.3) |
| The sortition golden line checked against the step's draw order, not through the step | Not run; the reason is plausible (draw values come from the engine's stream, not the fixture), and my shuffle test covers the sortition line against itself |
| FLED's test-only office 299 | Not run |
| Nightly timeout of 60 minutes | Not run; 1,024 seeds at about 0.8 s each is under it |
| Found while reading `seam.rs` | M7 0.3.4's `[[reads]]` lacks `person.id`: through the seam the real M7 cannot find a person row, so every law would fail `UndeclaredRead(person.id)` until SPR-021 adds the identity read. The Developer reported it; I confirmed it with the dropped-read probe (`person.id` is refused) |

### 7.7 Left to do
`check_all.sh` whole on `34a7f56`; T-09 (the drift test, the control that fails by name, the data-only commit has no `.rs` or `Cargo.*` file, the 8 laws and 24 programs); T-08's reference and Rust mutant plants; the ladders at 1 and N threads and the CI times for this head; the three `not run` deviations.

