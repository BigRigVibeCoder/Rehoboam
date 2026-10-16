---
id: VER-SPR-003-TEST
title: "Determinism tester evidence for SPR-003"
type: reference
status: CLOSED
owner: tester
agents: [tester]
tags: [verification, testing, determinism]
related: [SPR-003, VER-SPR-003, VER-SPR-002-TEST, VER-SPR-010-TEST, AGT-003-DET, ADR-0006]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** All five samplers match the exact distributions I computed myself, and they are bit-reproducible. I found no correctness defect in the samplers and no breach of the float-zone ruling that the gates were meant to catch. Statistics: 4×10^8 draws per case for binomial and normal tails, 10^8 for the rest, with every case at p > 10^-3 (the lowest, p = 0.0078, passed three replicates on fresh keys). Determinism: an independent Python port of the five algorithms, on `bin/gen_rng_golden.py`'s reference stream, reproduces all 136 golden lines and the draw counts, and 2,650 further random parameter sets bit for bit. Ten builds (opt-level 0, 1, 2, 3, s, LTO, `+fma`, `target-cpu=native`, and more) give identical hashes in all 27 cases. `bin/check_all.sh` is green (105 checks); 36 of 50 gate mutations turn it red, each at the intended gate. **One low defect (DEF-1):** the documented draw counts for BTPE and PTRS are wrong near their regime thresholds. **The 14 gate mutations that stay green are the real findings (GAP-1 to GAP-5):** a public `f64` in `rh-sample` passes every gate, the clippy transcendental list is guarded for 2 of 44 names, the float exemption can be widened to another real crate unseen, and a third-party dependency passes. On the Developer's weakness: the PTRS 0.43 → 0.93 shift is caught (see section 4), and the answer on T-01 case count is in section 5.

Tested commit: `743ce830e01fc41a1aa014961ea53daccde038bb` on `sprint/SPR-003-samplers` (PR #16). Probes ran on `git archive` copies; the repository's working tree was only read. Probe plan written from the contracts first (SPR-003 v1.1.0 with the float-zone ruling, ADR-0006 items 3–4, `05.1` §6 items 2–4, DET-3, V-DET-09, V-DET-19, GOV-003 §2) before the branch was opened; the plan named a likely gap in the public-API float check and in the per-name coverage of the clippy list. Reference code: `refdist.py` (exact `Fraction` pmfs, ratio-recurrence pmfs for huge n and λ, incomplete gamma/beta, χ², Kolmogorov–Smirnov; stdlib only, self-checked against textbook values) and `pyref.py` (the independent port). Harness and raw results are in the session scratchpad `ver/` directory. Tester model: set by the Architect in GOV-005 §4.2; not recorded here by instruction.

## 1. Verdicts

| Task | Verdict | Basis |
|---|---|---|
| T-00 float zone | **PASS with GAP-1 to GAP-5** | 50 gate mutations of the whole `check_all.sh` tier: 36 red, each at the intended gate (section 3.1). The 14 green ones are listed in section 3.2; none is a regression of what the ruling promised in its acceptance line (float elsewhere, `std` `ln`/`exp` in the zone, zone relabelled), all three of which turn it red. |
| T-01 binomial | **PASS** | Exact-pmf χ² over a 60-cell grid (2×10^6 each, min p = 0.127) and 18 large-N cases (4×10^8 each) around the BINV/BTPE switch, mirrored p > 1/2, n up to 2^32 − 1 with p = 2^-16 and p = 1/2; every p > 0.02, and the three below 0.1 replicated at 0.11 / 0.42 / 0.26 / 0.64. DEF-1 (documentation only). |
| T-02 Poisson | **PASS** | 19 λ from 2^-16 to 32768 at 2×10^6; five λ at 10^8; 7 λ at 3×10^8 for the mutation work. λ = 10 − 2^-16, 10 and 10 + 2^-16 are continuous. PTRS is not Knuth: 2.3–2.7 draws per sample at λ = 1000 down to 10. DEF-1. |
| T-03 normal | **PASS** | 6 parameter sets at 10^8 (p 0.24–0.73) in the quantised law; 4×10^8 draws: both tails at 1 to 6 sd, inside and beyond the base box R = 3.6541…, all |z| < 1.3; sign balance z = −2.0; saturation at sd = MAX exact to z = 0.5. Tables byte-identical to `rand_distr` 0.6.0 (OBS-1). |
| T-04 gamma, beta | **PASS** | 14 gamma cases (shape 2^-16 to 32768) and 14 beta cases at 10^8: every p > 0.03 except Beta(½, ½) at 0.0078, which passed 3 replicates at 10^8 on fresh keys (p = 0.38, 0.85, 0.93). Deviation 2 (gamma at k = 1) and deviation 3 (beta in logs) judged below. |
| T-05 no drift | **PASS with GAP-6, GAP-7** | 27 cases × 10 builds identical; 98,675 float→`Q16_16` conversions equal an exact rational reference; golden vectors reproduced independently; CI run 38864821574 at this SHA is green on GitHub's runner. |
| V-DET-09 (golden vectors) | **PASS** | 136 lines reproduced by an implementation that shares no code with the Developer's. Mutation catch rates in section 4. |
| `bin/check_all.sh` | **PASS** | 105 PASS, 0 FAIL on a clean archive; tree clean afterwards. |

## 2. Results

### 2.1 Distributions (T-01 to T-04)

Method: a probe binary (example in the throwaway copy) draws from one keyed stream per entity and prints the histogram, the draws consumed and a hash; Python scores it against exact laws. Fixed-point outputs are compared in the quantised law, P(raw = j) from the continuous cdf at j ± ½ (ties have measure zero), never against the ideal continuous law. χ² cells are pooled to expected ≥ 5. All keys are fixed, so runs repeat exactly.

| Sampler | Cases | N per case | Result |
|---|---|---|---|
| Binomial, grid | n ∈ {1, 2, 10, 30, 100, 1000} × p ∈ {2^-16, 10^-3, 0.01, 0.1, 0.3, 0.5, 0.7, 0.9, 0.99, 1 − 2^-16} | 2×10^6 | 60 cells, min p = 0.127 |
| Binomial, regime and size | np = 9.995 / 10.01 and mirrors; n = 19, 20, 21 at ½; n = 655,359 and 655,360 at 2^-16 (np = 9.99998 / 10); n = 10^4, 10^6, 2^32 − 1 at ½, 0.3, 2^-16, 1 − 2^-16 | 4×10^8 | 18 cases, min p = 0.023, replicated; max count never exceeds n; mean z within ±2.5 (same keys across cases, so the signs correlate) |
| Poisson | λ = 2^-16, 0.01, ½, 1, 5, 9, 10 − 2^-16, 10, 10 + 2^-16, 12, 20, 29, 30, 31, 100, 10^3, 10^4, 32767, MAX | 2×10^6 (λ ≥ 10^4 also 10^8) | min p = 0.0156 at λ = 10^4, 0.195 at 10^8 |
| Normal | (0,1), (3.5, 2.25), (−1, 1.5×10^-2), sd = 2^-16, sd = 0.1, sd = 100, sd = MAX | 10^8; tails 4×10^8 | all p > 0.24; tails as above |
| Gamma | shape 2^-16, 0.01, 0.1, 0.5, 1 − 2^-16, 1, 1 + 2^-16, 2, 5, 50, 1000, 32767.99998; several scales | 10^8 | min p = 0.030 (shape 2^-16), the rest > 0.47 |
| Beta | (1,1), (½,½), (2,5), (0.1,10), (10,0.1), (100,100), (2^-16, 2^-16), (2^-16, 1000), (0.01, 0.01), (1000,1), (3,3) and others | 10^8 | min p = 0.0078 (Beta(½,½)), replicated three times at 0.38 / 0.85 / 0.93 |

Edge parameters: `n = 0`, `p = 0`, `p = 1`, `λ = 0`, `sd = 0` return the constant; `p` outside [0, 1], negative λ or sd, and non-positive shape or scale return `SampleError` with the sampler named. No panic or hang at any argument tried (n = 2^32 − 1, λ = MAX, shape and scale at both bounds). The longest rejection run seen in 10^9 draws was 30 attempts (60 draws).

### 2.2 Draws consumed (ruling item 4)

| Sampler | Documented | Observed | Verdict |
|---|---|---|---|
| Binomial, `p = 0`, `p = 1`, `n = 0` | none | 0 | as documented |
| Binomial, BINV | one per attempt, in practice one | 1.000 per sample, max 1 | as documented |
| Binomial, BTPE | two per attempt, "about 1.1 attempts" | 1.9–2.0 attempts at np = 10 to 21 (3.80–4.00 draws), 1.5 at np = 50, 1.14 at np ≥ 10^3 | **DEF-1** |
| Poisson, `λ = 0`; inversion | none; one | 0; 1.000 | as documented |
| Poisson, PTRS | two per attempt, "about 1.15 attempts at μ = 10 falling toward 1" | 1.33 at μ = 10, 1.30 at 12, 1.22 at 30, 1.17 at 100, 1.14 at 1000 | **DEF-1** (the trend is right, the 1.15 is not) |
| Normal | about 1.02 | 1.022 | as documented |
| Gamma, k ≥ 1 | about 2.1 | 2.02–2.12 | as documented |
| Gamma, k < 1 | one more than k + 1 | 3.06–3.12 | as documented |
| Beta | about 4.2 | 4.04–4.23 for shapes ≥ 1; 5.1–6.2 when a shape is < 1 | the "4.2" holds only for shapes ≥ 1 (part of DEF-1) |

The count is a pure function of the stream and the parameters in every case (2,650 random sets reproduced by the Python port, draw counts included).

### 2.3 Determinism (T-05, V-DET-09, DET-3)

| Probe | Expected | Actual | Verdict |
|---|---|---|---|
| Independent Python port of binomial (BINV, BTPE), Poisson (inversion, PTRS), normal (ziggurat), gamma (Marsaglia–Tsang), beta; streams from `bin/gen_rng_golden.py` | the 136 golden lines, 12 samples and `pos=` each | 136 of 136 equal | PASS |
| Same port, random parameters over all five samplers (150 + 2,500 sets, 6 keys × 120 samples each, FNV hash of values and positions) | equal | 0 mismatches | PASS |
| 10 builds: dev (opt-level 0, overflow checks), release opt-level 0/1/2/3/s, LTO fat + 1 codegen unit, `+fma`, `target-cpu=native`, `-C llvm-args=-fp-contract=fast` | identical hashes | 27 cases × 10 builds, 0 differences | PASS |
| FMA instructions in the binary | none unless the contraction flag is given | 0 in default, `+fma` and `native` builds; 135 with `-fp-contract=fast` (30 in `rh-sample` functions, 105 in `libm`, 67 of them in `lgamma_r`) | see GAP-6 |
| `f64` → `Q16_16`, 98,675 inputs: exact ties ± 1 ulp, subnormals, ±0, ±∞, 2^31 bounds, random bit patterns; `floor_to_i64` alongside | `round(Fraction(x)·65536)` half to even, saturating | 0 mismatches | PASS |
| Ziggurat tables against `rand_distr` 0.6.0 (the published crate) | verbatim | byte-identical; 257 X, 257 F, R | PASS (OBS-1) |
| Golden file regenerated; one value altered | red | both turn the golden test red (the repo's control) | PASS |
| CI at this SHA (run 38864821574, GitHub runner) | green | green | PASS |

## 3. Gates

### 3.1 Mutations that turn `check_all.sh` red (36 of 50), each at the intended gate

| IDs | Mutation | Red at |
|---|---|---|
| A01–A06 | `pub const … : f32` in `rh-core`, `rh-ids`, `rh-registry`, `rh-contracts`, `rh-events`, `rh-snapshot` | `lint_forbidden` |
| A07 | clone of the zone named `rh-sampler` (plane A, machinery) with floats | `lint_forbidden` |
| A09 | plane-A system crate depending on a float-bearing `tool`-plane clone | `lint_arch` (plane-A closure) |
| B01, B02, B04, B08 | `HashMap`, `Instant::now`, `mul_add`, `unsafe` in `rh-sample` | `lint_forbidden` |
| B03, B07 | `x.ln()`; an `as i32` cast of a float, in `rh-sample` | clippy (`disallowed_methods`; pedantic cast lints) |
| D01 | `x.ln()` in the tool crate `rh-cli` | clippy (the ban is workspace-wide, as ruled) |
| C01–C03 | `rh-sample` relabelled `tool`, plane B, kind `system` | `lint_arch` |
| C04 | `rh-sample` → `rand_chacha` | `lint_arch` (`rng-crate`) |
| D04 | `f64::ln` dropped from `clippy.toml` | its positive control |
| E01 | exemption by name prefix (`rh-sampler` would escape) | the "float outside `rh-sample`" control |
| E03 | exemption for every plane-A crate | the SPR-010 float-threshold control |
| E04, E05 | pin of `rh-sample` deleted; `rh-sample` dropped from the system allow-list | their controls |
| F01–F12 | each of the 12 `run` lines this sprint added to `check_all.sh`, deleted one at a time | pytest (`test_check_all_inventory.py`) |

The F-series is the strongest result: SPR-010 GAP-4 (a deletable `check_all` line) is closed for this sprint's lines.

I also called every one of the 44 disallowed names, plus the UFCS form, the path-as-value form (`let f = f64::exp`), a macro body, a type alias, a reference receiver, a generic receiver and a `#[cfg(test)]` module, in `rh-sample`: **56 of 56 are flagged by clippy, and the controls (`sqrt`, `libm::log/fma/sqrt`, `floor`, `round`, `abs`, `trunc`, `fract`, `ceil`, `rem_euclid`, `round_ties_even`, `signum`, `copysign`, `recip`, `to_degrees`) are not.** The list itself is correct and complete; the problem is that nothing guards its entries (GAP-2).

### 3.2 Mutations that stay green (14 of 50)

| ID | Mutation | Verdict |
|---|---|---|
| B09, B10, B11 | `pub fn …() -> f64`; `pub fn …(f32)` and a `pub` struct with a `pub f64` field; `pub use libm::log` | **GAP-1**: ruling item 4 (the public API never exposes `f64`) has no gate |
| D02, D03, D05 | drop `f64::cbrt`; `f32::ln`; `f64::powf` and `f64::powi` from `clippy.toml` | **GAP-2** |
| E02 | the float exemption also covers `rh-core` | **GAP-3** |
| C07, C08, C09 | add third-party `zerocopy` to `rh-sample`; loosen `libm = "=0.2.16"` to `"0.2"`; `libm` default features on | **GAP-4** |
| B05 | `libm::fma` in `rh-sample` | GAP-5 (a wording question) |
| B06 | `#[allow(clippy::disallowed_methods, reason = …)]` on an item, then `x.ln()` | GAP-5 |
| C05, C06 | `rh-sample` → `rh-snapshot`, → `rh-registry` | by design: "workspace crates" are allowed by ruling item 6 |

Not claimed: B12 (float in `rh-sample/tests`) and A08 (tool-plane clone) went red for reasons of my own edit (a constant-assertion lint; the clone's tests importing `rh_sample`). `tests/stats.rs` already holds floats and passes, which is the control for B12. First-round Rust mutations B03, B05, B07, B09 and D01 also went red at `rustc`/`cargo fmt` for my own formatting or a missing `reason`; they were re-run in valid form and only the second results are reported.

## 4. Sampler mutations (43 mutants of the Developer's code)

Each mutant ran the repo's own tests (`cargo nextest run -p rh-sample --release`, which includes golden vectors and the PR-tier statistics); survivors then ran my probes at 3×10^8 samples per parameter set.

| Outcome | Count | IDs |
|---|---|---|
| Caught by the repo's tests | 24 | b01 b03 b04 b06 b09 b10 b14, p01 p02 p10 p12, n02 n05 n06, g02–g07, e01 e02, c01 c02 |
| Survived, equivalent in law (a measure-zero comparison, an exact shortcut, a symmetric sign) | 8 | b02 b07 b08 b11, p07, n01 n03, g01 |
| Survived, effect below 10^-4 of a probability (constants that only shape the hat or the squeeze) | 10 | b05 b13, p03 p04 p05 p06 p08 p09 p11, n04; **my probes at 3×10^8 also see nothing** |
| Survived and real | 1 | **b12** (BTPE step 5.3 `0.5 → 0.0`): the repo's PR tier misses it; the repo's nightly tier catches it (`chi2 nightly binomial 1000 49152: 288.4 vs 168.8`); my probe gives p = 0 at n = 1000, p = 0.2 and n = 10^4, p = 0.45 |

The 24 caught are mostly caught by the golden vectors alone (the PR-tier statistics also catch b09, b10, g02, g03 and e01). Mutants that only reorder draws (g07 boost order, e02 beta gamma order, b06) are caught by the golden vectors and by nothing else, as expected; the golden file is what pins the draw order.

**The Developer's reported weakness (PTRS offset 0.43 → 0.93).** It is detected, three ways: (a) the golden vectors fail at the PR tier; (b) the repo's nightly tier fails (`chi2 nightly poisson 655360: 743.3 vs 54.1`, and the moments test); (c) my probe sees a mean shift of +78σ at λ = 10 with 10^8 draws (+17σ at λ = 100), and it would already see +25σ at 10^7. Only the repo's PR-tier statistics (2^16 samples) miss it, because the shift is 0.02 against a standard deviation of 3.2 (z ≈ 1.3 at that sample size). The sensitivity is a matter of sample count, not of the test design.

## 5. Defects

**DEF-1 (low, documentation): the documented draw counts for BTPE, PTRS and Beta are wrong outside their large-parameter regime.** `binomial.rs` says BTPE takes "about 1.1" attempts "on average"; `poisson.rs` says PTRS takes "about 1.15 expected attempts at μ = 10"; `beta.rs` says "about 4.2 draws per sample in all". Reproduction: `probe bin 100 32768 …` (np = 50): 2.998 draws per sample, i.e. 1.5 attempts; `probe bin 20 32768 …`: 3.996 draws, 2.0 attempts; `probe pois 655360 …`: 2.66 draws, 1.33 attempts; `probe beta 32768 32768 …`: 6.15 draws. The ruling asks each sampler to document its draw count; the contract is deterministic-given-the-stream, which holds, so no behaviour is wrong. Fix: state the count as a function of the regime (BTPE from about 2 attempts at np = 10 to about 1.1 at np ≥ 10^3; PTRS 1.33 at 10 to 1.13 at 10^3; Beta 4.2 or more).

No defect against a contract clause was found in the samplers.

## 6. Contract gaps and observations (for the Architect; recommendation and confidence given)

1. **GAP-1: the public API can expose `f64` and nothing notices.** Ruling item 4 says floats never leave the crate. `lint_forbidden` exempts the crate wholesale; clippy has no lint for a `pub` float signature; mutations B09–B11 pass the whole tier. The API as shipped is clean (every `pub` item is `u32`, `Q16_16`, `Result` or `SampleError`; I read all of it). Recommendation: a small gate that scans `rh-sample`'s `pub` surface (rustdoc JSON, or a regex over `pub` signatures and fields for `f32`/`f64`/`libm`), with a planted control; 80%.
2. **GAP-2: 42 of the 44 names in `clippy.toml` are unguarded.** The controls call `f64::ln` and `f64::exp` only; dropping any other entry passes (D02, D03, D05). Recommendation: a pytest that parses `clippy.toml` and asserts the exact 44 `std` names from ruling item 3 (as the clippy list does today), or a control that calls all 44 (my 56-call file is a ready fixture); 85%.
3. **GAP-3: the float exemption can be widened to another real crate unseen.** Making `lint_forbidden` exempt `rh-core` as well (E02) is invisible, because no real crate holds a float and the controls use fixture crates named `rh-sample` and `rh-sampler`. SPR-003 T-00 asks for "a positive control that shows the exemption reaches no other crate"; the control shows it for one name that is not a prefix, which E01 confirms, not for the set. Recommendation: a pytest asserting the exempt set equals exactly `{"rh-sample"}`, and a control per real plane-A crate name; 80%.
4. **GAP-4: ruling item 6 ("libm only, plus workspace crates") is not enforced.** A third-party crate (C07), a loosened `libm` requirement (C08) and `libm` default features (C09) all pass. The `Cargo.lock` pin and the golden vectors would catch a changed `libm` result, not a changed requirement or an added crate. The plane-A dependency allow-list (BCK-002 A-20) is the planned home. Recommendation: in `lint_arch`, `rh-sample`'s external dependencies must be exactly `{libm}`, pinned `=`, `default-features = false`; 80%.
5. **GAP-5: two wording questions in ruling item 3.** (a) `libm::fma` is a software, correctly rounded and deterministic fused multiply-add; T-05 says "no fused multiply-add". It passes (B05). If the intent is "no operation that fuses", forbid `libm::fma` in `lint_forbidden`; if the intent is "no cross-target difference", it is fine. (b) An item-level `#[allow(clippy::disallowed_methods, reason = …)]` silences the transcendental ban in the zone (B06) and the escape is not counted, unlike a `LINT-ALLOW(forbidden-api)`. Recommendation: forbid `allow(…disallowed_methods)` in `rh-sample` by a grep rule; keep `libm::fma` allowed and say so in the ruling; 60%.
6. **GAP-6: FMA contraction is prevented by rustc's default only.** A build with `RUSTFLAGS="-C llvm-args=-fp-contract=fast"` puts 135 FMA instructions into the probe, 30 in `rh-sample` and 105 in `libm`. Outputs stayed identical in all 27 cases (a 1-ulp change flips a decision with probability near 10^-16 per draw, so no feasible test would see it), but the golden vectors cannot defend against it either. The ADR says "no FMA contraction"; no gate checks the binary. Recommendation: accept, and record that CI builds with default flags; or add a disassembly count (`vfmadd` = 0) to the nightly tier; 55%.
7. **GAP-7: draw positions depend on parameters at the degenerate points, but not for the normal.** `p = 0`, `p = 1`, `n = 0` and `λ = 0` draw nothing; `sd = 0` still draws (documented as deliberate, "so draw positions never depend on the parameters"). A system that takes several samples from one stream will shift every later sample when a parameter crosses 0, which breaks common random numbers across counterfactual branches (ADR-0006 item 3). Recommendation: rule that call sites key each sampler call by its own `k` base, and say so in `05.1` §6 item 3; or make all five consume at least one draw; 60%.
8. **GAP-8: the PR-tier statistics are weak; the nightly tier is not.** At 2^16 samples the PR tier misses b12 and the PTRS shift; the golden vectors carry the PR tier, and the nightly tier (64×, 14 s here) catches both. A C3 change to a sampler should therefore require the nightly tier before merge. Answering the Architect's question on T-01: the seven binomial cases are enough to show the algorithm is right (three BINV, four BTPE, one flipped), but they omit what is most likely to break at scale: n above 2^20, p = 2^-16 with n = 2^32 − 1 (np = 65,535, the largest BTPE input), and np just below and above 10. Recommendation: add `(2^32 − 1, 1)`, `(2^32 − 1, 32768)`, `(1000, 655)` and `(1000, 656)` to `reference.txt`; I ran all four at 4×10^8 and they pass; 75%.
9. **OBS-1: ziggurat box areas differ by up to 10^-9 (relative).** The tables are byte-identical to `rand_distr` 0.6.0, whose generator rounds V to 0.00492867323399; box areas run from 0.004928673233990 to 0.004928673229093 (spread 4.9×10^-12). The law of the normal is therefore off by about 10^-9 in total variation. The `Q16_16` step (1.5×10^-5) is 10^4 times larger, and no feasible test could see it. No action unless the sampler is ever used at full `f64` precision.
10. **OBS-2: `count_in` uses an exact floor; `rand_distr` truncates with `as`.** In BTPE's left tail, upstream maps `y_tmp ∈ (−1, 0)` to 0; here it is rejected as −1, which is the correct treatment. The Developer's deviation 4 (restarts past `min(110, n)`, truncated draws redrawn, no Poisson branch) changes the law by under 10^-30 (BINV) and by exactly nothing (BTPE); "no Poisson branch" is sound because `p ≥ 2^-16`, which I tested at n = 2^32 − 1.
11. **OBS-3: the Developer's other declared deviations hold.** (1) Vectors and statistics landing in the T-05 commit is a planning matter, not a defect. (2) Gamma at k = 1 by Marsaglia–Tsang: χ² p = 0.73 at k = 1, 0.74 at 1 − 2^-16, 0.67 at 1 + 2^-16. (3) Beta in logs: Beta(2^-16, 2^-16) p = 0.67 and Beta(2^-16, 1000) p = 0.71; `X/(X+Y)` would give 0/0 there. (5) The `rh-ids` `stream_independence.rs` rewrite replaces `powi`/`powf` by multiplication and repeated squaring: algebraically the same statistic; I did not re-run the 10^6 sweep.

## 7. Not run, or weaker than stated

1. Cross-platform (Windows, macOS ARM, a second architecture, 32-bit) is not run; the only second machine is GitHub's runner, via the green CI at this SHA. Level 2 (ADR-0006 item 9) stays a goal.
2. The statistical power is set by the sample counts above. Effects below roughly 3×10^-4 of a probability (ten mutants, OBS-1) are invisible to every test I ran, and to any test that can be run in CI.
3. The Python port is mine; it shares the *specification* with the Developer's code (both follow `rand_distr` and the papers), so it proves the code does what the algorithms say and is machine-independent, not that the papers are right. The exact-law tests carry that.
4. Python uses glibc's `exp`/`log`/`lgamma`/`pow`, the Rust code uses `libm`'s; they agreed on every one of the 2.0×10^6 samples I compared, but 1-ulp differences would show only at a decision boundary.
5. I did not re-run the `rh-ids` V-DET-20 10^6 sweep, the `deny` licence ledger, or the Windows/macOS nightlies.
6. The Architect audit `VER-SPR-003` is not part of this file.

## 8. Re-verification at `fbe24b0d474d2a53929911f4d9f05ca52211401d` (119 checks, CI run 38872985485)

**BLUF:** The fixes work for what I tried before: all 13 mutations that stayed green in section 3.2 turn red at the intended gate, DEF-1 is fixed, and GAP-6, GAP-8 and the exact-pin rule hold against the evasions I tried first. The sampler code is unchanged. **But I found four ways around the new rules, and one of them silences the transcendental ban completely:** a `[lints.clippy] disallowed_methods = "allow"` table in `rh-sample/Cargo.toml` passes all 119 checks with `x.ln()` in the crate (GAP-9). The exact-pin rule does not see a second `libm` edge (a `[target.…]` table) or `libm`'s `features` (GAP-10). `float-api` is textual and misses a private alias, a trait method and an associated type (GAP-11). Details and reproductions below.

Method: `git archive` of `fbe24b0` into a throwaway copy; `bin/check_all.sh` on the clean copy is 119 PASS, 0 FAIL (`rc=0`). Each mutation below ran the whole tier on its own copy; trials of the new rules ran the gates directly.

### 8.1 The sampler code is unchanged

| Check | Result |
|---|---|
| Non-comment lines of every file in `crates/rh-sample/src` (`beta`, `binomial`, `convert`, `error`, `gamma`, `lib`, `normal`, `poisson`, `uniform`, `ziggurat_tables`), `743ce83` vs `fbe24b0` | identical; only `//!` documentation lines differ |
| `tests/golden/samplers_v1.txt` | byte-identical to `743ce83` |
| Independent Python port against the new head's golden file | 136 of 136 lines reproduced, draw counts included |
| Section 2 statistics, determinism builds, conversion check | not re-run: the code they ran on is the same code. Their evidence stands. |

### 8.2 The 14 mutations that stayed green in section 3.2

| ID | Mutation | Result at `fbe24b0` |
|---|---|---|
| B09 | `pub fn leak() -> f64` | **red**, `lint_forbidden` (`float-api`) |
| B10 | `pub fn(f32)`, `pub struct` with a `pub f64` field | **red**, `lint_forbidden` |
| B11 | `pub use libm::log` | **red**, `lint_forbidden` |
| B06 | `#[allow(clippy::disallowed_methods, reason = …)]` on an item, then `x.ln()` | **red**, `lint_forbidden` (`lint-allow`) |
| D02, D03, D05 | drop `f64::cbrt`; `f32::ln`; `f64::powf` and `f64::powi` from `clippy.toml` | **red**, pytest |
| E02 | the float exemption also covers `rh-core` | **red**, the new control "a float in a crate named rh-core must FAIL" |
| C07 | `zerocopy` in `rh-sample` | **red**, `lint_arch` (`plane-A-external`) |
| C08 | `libm` as `0.2`; also `^0.2.16`, `~0.2.16`, `>=0.2.16`, `=0.2`, bare `0.2.16`, `=0.2.16, <0.3` | **red**, `lint_arch`, all seven spellings |
| C09 | `libm` with default features | **red**, `lint_arch` |
| B05 | `libm::fma` | green, by the Architect's ruling (GAP-5(a)) |
| C05, C06 | `rh-sample` → `rh-snapshot`, → `rh-registry` | green by design; not re-run |

`fp-contract` guard (GAP-6): `.cargo/config.toml` with `-C llvm-args=-fp-contract=fast`, `--fp-contract=fast`, or the same under `[target.…]` all turn the tier red at the new guard. Other spellings I tried (`-enable-unsafe-fp-math`, `-fp_contract`) are rejected by rustc itself, and I could not build a binary with FMA from any flag that does not contain `fp-contract` (135 FMA instructions with the flag, none without), so a string match is enough for the flags that exist. Not covered: a `~/.cargo/config.toml` outside the repository (the guard looks under the root and in the environment), and `RUSTC_WRAPPER`; both affect only a developer's machine, not CI.

### 8.3 GAP-8 and DEF-1

| Item | Result |
|---|---|
| The four new binomial cases `(2^32−1, 1)`, `(2^32−1, 32768)`, `(1000, 655)`, `(1000, 656)` | in `reference.txt` for both tiers (`df` 342 / 400 / 18 / 18 at the PR tier); both tiers pass on the clean tree; I had already run all four at 4×10^8 draws against my own pmf |
| Nightly tier on the clean tree | 3 of 3 pass (12.6 s) |
| b12 (BTPE step 5.3, `0.5 → 0.0`) with the capped bins | PR tier still misses it (4 of 4 pass); **nightly tier fails it** on `binomial 1000 49152` (261.1 vs 109.8) and `binomial 5000 19661` (194.7 vs 192.7). The Developer's claim holds. |
| DEF-1 | the new text in `binomial.rs`, `poisson.rs`, `gamma.rs`, `beta.rs` and `normal.rs` matches what I measured (BTPE ≈ 2 attempts at np 10 to 20, 1.5 at 50, 1.15 from 10^3; PTRS 1.33 at 10 to 1.14 at 10^3; Beta 4.0 to 4.2, and 5.1 to 6.2 with a shape below 1). **Closed.** |
| GAP-7 | the call-site rule is in the crate docs of all five samplers. **`05.1` §6 item 3 does not contain it** at this head (it still reads "read sequentially; each sampler documents how many draws it consumes"). The Architect's ruling put it there; it is not in the Developer's diff. Open for the Architect. |

A caution for anyone repeating this: a shared `CARGO_TARGET_DIR` between a mutated copy and a clean copy made `cargo nextest` run the stale mutated build on the clean tree (it reported the mutant's failures on clean code) until I used a separate target directory. My first "clean" nightly result was wrong for that reason and is not reported; the clean result above comes from a fresh directory.

### 8.4 Evasions of the new rules

Each trial was compiled (`cargo check --all-targets`) and run through `lint_forbidden`, `lint_arch` and clippy; an evasion needs all three green and the code to compile.

**`lint-allow`: held against 15 of 16 attempts that really silence clippy.** Caught: plain `allow`, `expect`, `cfg_attr(all(), allow(…))`, a multi-line attribute, `allow` with a leading `unknown_lints`, `clippy::style` and `clippy::all` (group lints), spaces around `::`, an inner attribute in a function body, an `allow` on a statement, and `allow` inside a `macro_rules!` body. The cases that left `lint_forbidden` green do not silence clippy either: `allow(warnings)` (clippy still reports the disallowed method), `allow(clippy::pedantic, clippy::nursery)`, and a misspelled lint name (clippy rejects it). **The one real evasion is in `Cargo.toml`, not in source (GAP-9).**

**`float-api`: textual, and 5 of 16 shapes get past it.** Caught: `pub type X = f64`, a generic `Into<f64>`, a `where` clause, `macro_rules!` generating `pub fn … -> f64`, a glob re-export from a private module, `core::primitive::f64`, a signature split over lines, `libm` in a `pub use`. **Not caught, and they compile and pass clippy (GAP-11):**
1. a private alias: `type F = f64; pub fn leak() -> F`, and `type G = f64; pub struct S(pub G);`;
2. a trait method, required or default: `pub trait T { fn get(&self) -> f64; }`;
3. an associated type fixed in an impl body: `impl Tr for W { type Out = f64; } pub fn leak() -> <W as Tr>::Out`;
4. a macro that takes the type as a token: `mk!(leak, f64)` expanding to `pub fn $n() -> $t`;
5. an opaque return, `-> impl Debug` of an `f64` (leaks the value only through `Debug`; mild).

**Exact pin (`plane-A-external`): held for seven spellings and the build-dependency table; two holes (GAP-10).**
1. A second `libm` edge is invisible. `[target.'cfg(unix)'.dependencies] libm = { version = "0.2", default-features = true }` appears in `cargo metadata` with `req = "^0.2"` and default features on, but `Edge` compares only `(target, kind)` (`req` and `default_features` are `compare=False`), so the set keeps one of the two and the rule judges that one. Full tier: green (H03).
2. `features` is never read. `libm = { version = "=0.2.16", default-features = false, features = ["arch"] }` enables libm's architecture-specific routines, which is what `default-features = false` was meant to exclude. Full tier: green (H04).
3. A renamed second edge (`libm2 = { package = "libm", version = "0.2" }`) was stopped by `cargo deny`, not by `lint_arch` (H05); a `path` dependency named `libm` was stopped by `lint_arch` (`req = *`). A `dev-dependency` is exempt by design.

**The configuration layer (GAP-9, the serious one).** In `crates/rh-sample/Cargo.toml`, replace `[lints] workspace = true` with `[lints.clippy] disallowed_methods = "allow"` (and `[lints.rust] unsafe_code = "forbid"`), then put `x.ln()` in the crate: **119 of 119 checks pass** (H01). Nothing reads a plane-A crate's `[lints]` table. The same override removes every workspace deny for that crate (`float_arithmetic`, `unwrap_used`, `missing_docs`, `allow_attributes_without_reason` and the rest) while `lint_forbidden`, whose float rule is already off in the zone, sees nothing. (A table that drops the lints entirely failed clippy for an unrelated reason in my trial H02, so I do not claim it.)

### 8.5 New gaps, with recommendations

1. **GAP-9: a crate can opt out of the workspace lints, and no gate looks.** Recommendation: in `lint_arch`, every plane-A crate's manifest must declare `[lints] workspace = true` and no `[lints.*]` table of its own (read from cargo metadata's raw manifest or the TOML), with a planted control; 85%. This is the cheapest gap to close and the only one that defeats the transcendental ban outright.
2. **GAP-10: the exact-pin rule judges one edge per `(crate, kind)` and ignores `features`.** Recommendation: make `req` and `default_features` part of `Edge` equality (or keep a list per crate) so every edge is checked, add `features` to the reviewed entry (`libm`: none), and add a control for a `[target.…]` table; 80%.
3. **GAP-11: `float-api` is a text rule.** It stops the accidental leak and the plain `pub fn … -> f64`, not a deliberate alias. Recommendation: accept for now and say so in the rule's docstring, or add two textual rules for the zone (no `type … = f32|f64` alias anywhere in `rh-sample/src`, and scan trait bodies and impl bodies for `f32`/`f64`), or use rustdoc JSON; 65% for the textual additions, 75% for rustdoc JSON.
4. **OBS-4:** the call-site rule of GAP-7 is not yet in `05.1` §6 item 3 (section 8.3).

### 8.6 Not run in this pass

The determinism builds, the distribution statistics and the conversion check of section 2 (code unchanged); the Windows and macOS nightlies; `~/.cargo/config.toml` outside the repository; `float-api` against a rustdoc-JSON ground truth (I judged each shape by whether it compiles and what it exposes, not by an independent tool).

## 9. Re-verification at `91f1ea6e283f975a0c62e79fcbd514a14a738de9` (124 checks, CI run 38875277300)

**BLUF:** GAP-9, GAP-10 and the alias half of GAP-11 are fixed against every reproduction and evasion I tried from section 8, and the regression of the 13 mutations is clean. The fixture changes weakened no older control. **Two new holes remain, found by the same method:** a `clippy.toml` placed inside `crates/rh-sample/` replaces the workspace list for that crate, so removing `f64::ln` there and calling `x.ln()` passes all 124 checks (GAP-12); and a `use core::primitive::f64 as Fl;` rename carries a float through a `pub` signature past the new alias rule (GAP-11b). Neither needs more than one file; the first undoes the same ban that GAP-9 was about.

Method as in section 8: a `git archive` of `91f1ea6`, a fresh copy and a separate target directory for every mutation, the whole `bin/check_all.sh` tier each time. The clean copy is 124 PASS, 0 FAIL.

### 9.1 Regression of the 13 mutations

| IDs | Result at `91f1ea6` |
|---|---|
| B06, B09, B10, B11 | red, `lint_forbidden` (`lint-allow`, `float-api`) |
| C07 | red, `lint_arch` |
| C08, C08b to C08f (`0.2`, `^0.2.16`, `~0.2.16`, `>=0.2.16`, `=0.2`, `0.2.16`) | red, `lint_arch` |
| C09 | red, `lint_arch` |
| D02, D03, D05 | red, pytest |
| E02 | red, the "float in a crate named `rh-core`" control |
| B05 (`libm::fma`) | green, by ruling |

### 9.2 GAP-9: the `lints` rule

| Mutation | Result |
|---|---|
| H01: `[lints.clippy] disallowed_methods = "allow"` in place of `[lints] workspace = true`, plus `x.ln()` | red, `lint_arch` (`lints`). Was green in section 8. |
| `[lints]` table deleted, plus `x.ln()` | red, `lint_arch` |
| misspelt `[lint]` table | red, `lint_arch` |
| `lints.workspace = true` written inside `[package]` (no `[lints]` table) | red, `lint_arch` |
| the same override in a second plane-A crate (`rh-core`: `[lints.clippy] float_arithmetic = "allow"` plus float arithmetic) | red, `lint_arch` |
| `[lints] workspace = false`; `[lints] workspace = true` plus a `[lints.*]` table; an empty `[lints.rust]` | cargo itself refuses the manifest (`workspace cannot be false`), so every cargo command fails; not evasions, and the red I saw (at `cargo fmt`) is cargo's, not the rule's |
| dotted root key `lints.workspace = true`; inline root key `lints = { workspace = true }` | green, as they should be: equivalent spellings of the required declaration, and cargo accepts them |
| `[workspace.lints.clippy] disallowed_methods = "allow"` in the root `Cargo.toml`, plus `x.ln()` | red, at the existing "std `ln` in `rh-sample` must FAIL" clippy control (it renders the root lint tables) |
| `.cargo/config.toml` with `rustflags = ["-A", "clippy::disallowed_methods"]`, plus `x.ln()` | red, at the same control |

### 9.3 GAP-10: the exact-pin rule

| Mutation | Result |
|---|---|
| H03: `libm` again under `[target.'cfg(unix)'.dependencies]`, `0.2`, default features on | red, `lint_arch` (was green) |
| H04: `features = ["arch"]` (exact pin, defaults off) | red, `lint_arch` (was green) |
| H06: the same under `[target.'cfg(unix)'.dependencies]`; H07: under `[build-dependencies]`; H08: `features = ["unstable"]`; H09: table form with `features = ["force-soft-floats"]` | red, `lint_arch`, all four |
| H05: renamed `libm2 = { package = "libm", version = "0.2" }` | red, at `cargo deny` (not `lint_arch`; before and after) |

### 9.4 GAP-11: the alias rule

| Mutation | Result |
|---|---|
| A01: private `type F = f64;` returned from a `pub fn` | red, `lint_forbidden` (was green) |
| A02: associated type `type Out = f64;` in an impl body, returned through a projection | red (was green) |
| A06: generic alias `type V<T> = (T, f64);` | red |
| A07: alias declared inside a `macro_rules!` body | red |
| A05: `pub trait Tr { fn get(&self) -> f64; }` | green, a documented residual |
| **A03: `use core::primitive::f64 as Fl;` then `pub fn leak() -> Fl`** | **green: compiles, passes every gate (GAP-11b)** |
| **A04: the same rename, then `type G = Fl;` and `pub fn leak() -> G`** | **green** |

The rule scans `type … = …` definitions and `pub` signatures; a `use … as` rename is neither, and the signature then names `Fl`, not `f64`. The Architect's ruling documents the macro-token, projection and opaque-return residuals; the `use` rename is not among them.

### 9.5 GAP-12 (new): a crate-local `clippy.toml` replaces the workspace list

Clippy reads the configuration file nearest the crate being checked, not the root file. With `crates/rh-sample/clippy.toml` holding a copy of the root `clippy.toml` minus the `f64::ln` and `f64::exp` entries, and `x.ln()` in `rh-sample`, **all 124 checks pass** (L09b). The same with the hidden name `crates/rh-sample/.clippy.toml` passes too (L09c). The root-file pytest (GAP-2's fix) cannot see either; the "std `ln` in `rh-sample` must FAIL" control renders the root file into a fixture, so it tests the root list and not the file clippy actually reads for `rh-sample`. (An empty crate-local file is red, only because it also drops `allow-unwrap-in-tests` and friends and clippy then fails on the tests; the evasion has to copy the rest.) Recommendation: in `lint_arch` (or a pytest), fail if any `clippy.toml` or `.clippy.toml` exists in a workspace member's directory or below, other than at the workspace root. 90%.

### 9.6 The fixtures

77 files changed in the sprint's last three commits; 50-odd are the `[lints] workspace = true` additions to the lint fixtures. Every positive control in `check_all.sh` that uses one still turns red for its own reason: all 124 checks pass on the clean tree (the controls run there), and my 40 mutations show the controls fire (E02, L08, L10 above went red at their controls). The new `lints` rule applies to plane-A crates only, so the `tool`-plane fixture crates are unaffected. I did not diff each fixture's expected message against the earlier text; the controls match on the diagnostic text (`fail:<text>`), and they pass.

### 9.7 New gaps

1. **GAP-12: a crate-local `clippy.toml` or `.clippy.toml` replaces the workspace transcendental ban.** Section 9.5. Recommendation above; 90%.
2. **GAP-11b: a `use … as` rename of a float type is not seen.** Recommendation: extend the alias rule to `use … as <name>` where the path ends in or names `f32`/`f64` (one more regex in the zone), accept it as residual and say so, or use rustdoc JSON; 70% for the regex.
3. Everything else in sections 8.5 and 9: closed.

### 9.8 Not run in this pass

The sampler statistics, determinism builds and conversion checks (no code change since `743ce83`; section 8.1 holds); a `~/.config/clippy` or `CLIPPY_CONF_DIR` environment override (local to a machine, like `~/.cargo/config.toml`); rustdoc-JSON ground truth for `float-api`.
