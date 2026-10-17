---
id: VER-SPR-010-TEST
title: "Determinism tester evidence for SPR-010"
type: reference
status: CLOSED
owner: tester
agents: [tester]
tags: [verification, testing, determinism]
related: [SPR-010, VER-SPR-010, VER-SPR-001-TEST, AGT-003-DET]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** `rh-core::fixed` matches an exact rational reference that I computed myself (906,078 vectors, half to even, plus the 14,329 committed golden rows), saturates at both bounds, rounds ties correctly in both signs, and maps a `u64` draw to a `Q16_16` probability with an exact frequency (`clamp(p, 0, 1)`, checked over every probability and every 2^48-draw block). No float conversion is reachable from a plane-A crate. The float lint fails every planted float I tried and passes every near miss named in the contract; `bin/check_all.sh` is green on clean code and red for 25 of 26 gate mutations. **No contract defect found (0 DEFs).** Two observations and five contract gaps for the Architect; the three that matter: `checked_div` saturates on overflow instead of returning `None` (documented, needs a ruling), a float held in a variable that never names a float type or literal passes both lint and clippy, and one `check_all.sh` line (the golden-vector `--check`) can be deleted without turning anything red.

Tested commit: `a9962fd` on `sprint/SPR-010-fixed-point` (probes ran in a `git archive` copy; the original tree was only read, and `bin/check_all.sh` on it left it clean). Probes were written from the contracts first (`probe_plan.md`, before `crates/rh-core/src/fixed*`, `bin/lint_forbidden.py` or the sprint's commits were opened), then adapted to the layout. The reference is `ver/ref_vectors.py` (Python `fractions.Fraction`, `round()` = half to even), independent of `bin/gen_fixed_vectors.py`. Harness and raw results are in the session scratchpad `ver/` directory (`probe_vectors.txt`, `lint_results.txt`, `mut_results.txt`, `mut2_results.txt`). Tester model: set by the Architect in GOV-005 §4.2; not recorded in this file by instruction.

## 1. Verdicts

| Gating test | Verdict | Basis |
|---|---|---|
| T-01 types: `Q8_8` (i16), `Q16_16` (i32), `Money` (i64); no mixing; no float conversion | **PASS** | 32 compile probes (`ver/cf`): every mix of the three types, `Money` against `i64`, `+ 1`, `* 2.0`, `from(f64)`, `into::<f64>`, `f64::from`, `as f64`, `TryFrom`, `to_f64`/`from_f64`/`from_f32`, `FromStr`, `Display`, `/` on any type fail with the stated error; the corrected controls compile. `rh-core` has no `[features]`; `--all-features` builds the same API. |
| T-02 arithmetic and rounding | **PASS** | 906,078 vectors (add, sub, mul, checked div, neg, `from_ratio`, `from_int`, `narrow`, `widen`, money add/sub/neg/`mul_int`/`div_int`/`mul_rate`) all equal the Fraction reference, in release and with overflow checks on. 14,329 committed golden rows re-derived by my own code: 0 mismatches. 7 mutations of rounding, saturation, zero guard and draw edge each fail the probe, so it is live. |
| T-03 thresholds and draws | **PASS** | Draw hit iff `draw/2^64 < p`, checked against an exact `Fraction` comparison on 3,000+ boundary draws, and exhaustively: for every raw `p` in `[-3, 65540]` plus `i32` extremes, the number of 2^48-draw blocks that hit is exactly `clamp(p, 0, 65536)`, the hits form a prefix, and the first and last draw of every block agree. |
| T-04 float gate (V-DET-19, float part) | **PASS with GAP-2, GAP-3, OBS-1** | 99 lint cases (35 must-fail, 38 must-pass, 26 escape and scope); 3 not as predicted, none a contract breach (OBS-1, OBS-2 and my own probe). Escape honoured only for a column a spec declares `f32_field` in `[[owns]]`. Tests, benches, examples exempt; `src/`, `src/bin`, `build.rs` and unit-test modules scanned. |
| `bin/check_all.sh` | **PASS with GAP-4** | Green on clean code in 32 s, tree clean afterwards. Red for 25 of 26 mutations, each at the gate meant to catch it (section 2.4). One survivor: M11. |

## 2. Results

### 2.1 Arithmetic, rounding, money (T-02; `05.5` §2.1; ADR-0006 item 4)

| Probe | Clause | Expected | Actual | Verdict |
|---|---|---|---|---|
| ADD/SUB sat, q8, q16, money: `MAX+1`, `MIN+MIN`, `MIN-1`, `0-MIN`, `MIN-MAX`, `MAX-(-1)` | T-02 "saturation at both bounds" | clamp of exact sum | equal on all (pairs of 30+ edge and random values per type) | PASS |
| MUL sat: `MAX*MAX`, `MIN*MIN`, `MIN*MAX`, `MIN*-1.0` | T-02 | clamp of rounded exact product | equal | PASS |
| MUL ties, positive and negative (`k*0.5`, `±1.5`, `±2.5` steps, odd and even neighbours), 1 ulp off tie | T-02 one rounding rule; "round half to even" | `round(Fraction(a*b, 2^F))` | equal on 100% (q8 and q16) | PASS |
| NEG `-MIN` | T-02 saturating | `MAX` | `MAX` (q8, q16, money) | PASS |
| DIV zero divisor (all dividends incl. `MIN`, `MAX`) | `05.5` §2.1 guard | `None` | `None` | PASS |
| DIV `MIN / -1.0`, other overflowing quotients | T-02 "checked function that returns `Option`" | see GAP-1 | `Some(MAX)` (saturated), per the module DECISION | PASS vs. reference; **GAP-1** |
| DIV ties both signs, all four sign combinations of dividend and divisor | T-02 | Fraction, half to even | equal | PASS |
| `from_ratio` incl. zero and negative denominators, `i64::MIN`, out of range | T-01 "constructors from exact ratios" | `None` for d=0 or out of range, else rounded | equal (387 rows per type) | PASS |
| `from_int` at both ends of the integer type, `narrow_saturating` ties and saturation, `widen` | T-01 | exact | equal | PASS |
| Money add/sub/neg at `i64::MIN`/`MAX` (60+ values squared) | `05.1` §6.4 money i64 | clamp | equal | PASS |
| Money `mul_int`, `div_int` (incl. `MIN / -1`, `/ 0`), `mul_rate` at `i64` bounds with rates `i32::MIN`..`i32::MAX` | `05.1` §6.4; `05.5` §2.1 | Fraction, half to even, clamp; `None` only for zero | equal; no panic (overflow checks on) | PASS |
| Identities and laws at the edges: `x+0`, `x-0`, `x*1`, `x*0`, `x/1`, commutativity of add and mul, ordering equals raw ordering | T-02 properties | hold | hold on 121 pairs | PASS |
| Repo golden file re-derived with my own code (`check_golden.py`) | T-02 "golden vectors committed" | equal | 14,329 checked, 0 mismatches | PASS |
| Mutations of the code (probe still live): half-up shift, truncation, half-up division, wrapping add, `<=` in the draw, `>` at p=1, no zero guard | probe sensitivity | each fails | each fails, with the first failing vector named | PASS |

### 2.2 Draws (T-03)

| Probe | Clause | Expected | Actual | Verdict |
|---|---|---|---|---|
| p = 0 and every negative p down to `i32::MIN` | T-03 edge cases | never hits (draw 0 and `u64::MAX` included) | never | PASS |
| p = EPSILON (raw 1) | T-03 smallest step | hits iff `draw < 2^48` | draw `2^48-1` hits, `2^48` misses, `0` hits | PASS |
| p = 1 - EPSILON (raw 65535) | T-03 | hits iff `draw < 65535*2^48` | `u64::MAX` misses; boundary exact | PASS |
| p = 1 (raw 65536) and p > 1 up to `i32::MAX` | T-03 | always hits, no shift or cast overflow | always hits | PASS |
| Exhaustive frequency over every block, every p in `[-3, 65540]` and the extremes | T-03 "frequency test against the exact probability" | `P(hit) = clamp(p,0,1)` exactly | exact; prefix property holds; both ends of each block agree | PASS |
| Monotone in p for a fixed draw (8 draws, p from -2 to 65540) | T-03 | non-decreasing | yes | PASS |
| `chance` takes one draw per call whatever p is | T-03 / keyed-RNG counter discipline | one | one (repo test; my mutation probes do not touch it) | PASS |

### 2.3 Float lint and compile-fail (T-01, T-04; V-DET-19)

| Probe family | Clause | Expected | Actual | Verdict |
|---|---|---|---|---|
| Planted floats in plane-A `src/`: types in fields, signatures, aliases, generics and turbofish; literals `1.5`, `1e3`, `1E5`, `1e+3`, `1.0e-3`, `5e-1`, `1_000.0`, `1__0.0`, `(1.)`, `2.0f32`, `1f64`, `0.1_f64`, `1_f32`, `1e3_f32`, `0_f64`; `as f64`, `f64::from`, `f32::MAX`, `f64::EPSILON`, `std::f64::consts::PI`, `to_f64`, `.into()` to `f32`, raw identifier `r#f32`, float in a macro body, in a const, in a `#[cfg(test)] mod` inside `src/`, in `src/bin/`, in `build.rs`, `0.0..1.0`, `1.0.max(2)`, after a tuple index | V-DET-19: every `f32`/`f64` type, conversion, literal | fail (exit 1) | 35 of 35 fail | PASS |
| Near misses: `x.0`, `t.1.0`, `t.1 .0`, `0..1`, `0..=1`, `1..2`, `0x1f32`, `0x1E5`, `0xf64`, `0x1e3`, `0xDEAD_BEEF`, `0b…`, `0o…`, `1.max(2)`, `1_i32.min(2)`, `2.pow(3)`, `x.f32_value()`, `af64`, `vec![1; 3]`, `a[1..2]`, `1u8`, floats in `"…"`, `r"…"`, `r#"…"#`, `b"…"`, `'1'`, `"1.2.3"`, line, block and doc comments | contract: near misses must not fail | pass (exit 0) | 36 of 38 pass; the 2 others are OBS-1 and `stringify!(f32)`, which names the type | PASS |
| Escape: same line, line above | T-04 same syntax | accepted, counted | accepted | PASS |
| Escape naming an undeclared column; a column declared with another type; a column only in a TOML comment; a column only inside a TOML string; no column; another reason; wrong case; two lines above; unused; `region.soil_extra`; `f32_fieldregion.soil` | T-04 "honoured only for a column declared in a spec" | violation | violation in all 11 | PASS |
| Malformed spec TOML, missing specs directory | fail closed | exit 2 (unreadable), `check_all` red | exit 2 | PASS |
| Scope: `tests/`, `benches/`, `examples/` (and nested) | docs/09 row; T-04 | exempt from the float rule | exempt | PASS |
| Scope: `tests/` still gets the other rules | T-04 "this rule only" | `HashMap` in `tests/` still fails | fails | PASS |
| Scope: every plane-A crate, not only `rh-core` | V-DET-19 | float in `rh-ids` fails | fails (workspace run) | PASS |
| One escape for `f32` and `f64` on one line | T-04 | both excused | both excused | PASS |

### 2.4 `bin/check_all.sh` mutations (throwaway copy; each run is the whole tier)

| ID | Mutation | Expected red at | Actual | Verdict |
|---|---|---|---|---|
| M0 | none | green | green, 32 s, tree clean | PASS |
| M1 | `pub const BAD: f32` in `rh-core/src/lib.rs` | lint_forbidden | lint_forbidden | PASS |
| M2, M2b | `let _x = 0.5;` and `f64::from(a)` in `fixed/round.rs` | lint_forbidden | lint_forbidden | PASS |
| M2c | float inside `fixed/round.rs`'s unit-test module | lint_forbidden | lint_forbidden | PASS |
| M3 | one golden row changed | nextest (replay) or `--check` | nextest | PASS |
| M4 | half up instead of half to even | nextest | nextest | PASS |
| M5 | `wrapping_add` | nextest | nextest | PASS |
| M6 | zero guard removed | nextest | nextest | PASS |
| M7 | draw `<=` | nextest | nextest | PASS |
| M8a-c | float regex: type alternative, conversion alternative, decimal-literal alternative removed | the matching positive control | `float_type`, `float_conversion`, `float_literal` control | PASS |
| M8d | float rule never applied | control | `float_type` control | PASS |
| M8e | escape accepts any column | control | `float_undeclared_field` control | PASS |
| M8f | escape reason never checked | control | `float_escape_reason` control | PASS |
| M8g | `src` added to the exempt directories | workspace control | float workspace control | PASS |
| M8h | no exempt directories | real run | lint on the real workspace (real `tests/` hold floats and rely on the exemption) | PASS |
| M9, M9b | compile-fail fixtures `mix-q8-q16`, `float-into` made to compile | compile-fail control | the matching control | PASS |
| M10 | float fixture workspace cleaned of floats | workspace control | workspace control | PASS |
| M12, M12b | `Q16_16::to_f64` added; the same with an escape for an undeclared column | lint_forbidden | lint_forbidden | PASS |
| M13 | declared-field spec emptied | declared-escape control | declared-escape control | PASS |
| M17 | `f64` const in `rh-ids/src/lib.rs` | lint_forbidden | lint_forbidden | PASS |
| **M11** | **delete the `gen_fixed_vectors --check` line from `check_all.sh`** | red | **green** | **survivor (GAP-4)** |

Six first-round mutations (M2, M2b, M4, M12, M12b, M15) went red at `cargo fmt`, and M8h at `mypy`, because my edit was badly formatted or mistyped. M2, M2b, M4, M12, M12b and M8h were re-run in valid form and are reported above; M15 (regenerate the golden file after a code change) was not re-run and is not claimed, M4 covers the same code change. M12c (float in `src/bin`) and M16 (float in `tests/`) were run as direct lint runs instead: `src/bin` fails, `tests/` passes.

## 3. Defects

None filed. No probe showed behaviour that breaks a clause of the sprint, `05.1` §6.4, `05.5` §2.1, ADR-0006, GOV-003 §2 or the V-DET-19 row. The items below are not DEFs; they are for the Architect.

### Observations

- **OBS-1 (low).** The float rule flags any identifier ending `_f32` or `_f64`, including a plain local such as `let my_f32 = 1;` (reproduction: `python -I bin/lint_forbidden.py --scan a.rs` with that line). The module says it targets conversions named after the types (`to_f64`, `as_secs_f64`), so this is by design and fails closed. A rename is the workaround. Same for `stringify!(f32)`, which names the type.
- **OBS-2 (trivial).** `crates/rh-core/tests/golden/fixed_vectors.txt` header says "none is None"; the rows use lowercase `none`. The generator writes the header, so the fix is in `bin/gen_fixed_vectors.py`.

## 4. Contract gaps (for the Architect; recommendation and confidence given)

1. **GAP-1: `checked_div` saturates on overflow.** SPR-010 T-02 says "division only through a checked function that returns `Option`"; the implementation returns `None` only for a zero divisor and `Some(MAX)` for `MIN / -1.0` and any quotient out of range (`arith.rs` DECISION, mirrored by `Money::checked_div_int`). `05.5` §2.1 asks for saturating arithmetic and a guard against a non-constant divisor, which this satisfies. If "checked" was meant to include overflow, the code is wrong; if not, T-02's wording is. Recommendation: keep the code, amend T-02 to say "`None` only for a zero divisor; overflow saturates" and add one golden row for `MIN / -1.0`; confidence 80%.
2. **GAP-2: an escape excuses a line, not an item.** `soil > 0.5 // LINT-ALLOW(forbidden-api): f32_field region.soil` passes with two escapes counted, so a float threshold comparison on the same line as a declared field is excused. The contract says floats are allowed "in items marked as declared float fields". Recommendation: accept for now (the escape count is printed), and add a rule in the first system sprint that owns an `f32_field` that threshold comparisons read a fixed-point column; confidence 65%. No real spec declares `f32_field` yet (0 of the checked-in specs), so the escape path has only fixture coverage.
3. **GAP-3: a float that is never named passes.** `let u = ext::draw(); let p = ext::draw(); u < p`, where `ext::draw() -> f64` lives in another crate, passes `lint_forbidden` and `cargo clippy -D warnings` (reproduced: lint exit 0, `cargo check` exit 0). V-DET-19 as written (types, conversions, literals) is met; `float_arithmetic`/`float_cmp` are not enabled. Reachable once a plane-A crate depends on a crate that returns floats (`libm`, `rand`). Recommendation: add `clippy::float_arithmetic` and `clippy::float_cmp` at `deny` in `[workspace.lints.clippy]` with an `allow` per declared float field; confidence 75%.
4. **GAP-4: the golden `--check` line is unguarded.** Deleting `gen_fixed_vectors --check` from `check_all.sh` leaves the tier green (M11); nothing asserts the line exists. The committed vectors are still replayed by nextest, so a wrong rule is still caught, but a hand-edited golden file consistent with a wrong rule would no longer be caught by the reference. Same class as the SPR-001 survivors in `VER-SPR-001-TEST` §4 item 2. Recommendation: accept; confidence 70%.
5. **GAP-5: no mixed-precision conversion beyond `widen` and `narrow_saturating`, and no `Q8_8` draw.** `draw_hits` takes `Q16_16` only; money-by-money and `Q16_16`-by-`Money` products have no API. Not a requirement of SPR-010; recorded so SPR-011+ do not assume them.

## 5. Not run, or weaker than stated

1. Time-boxed to about 45 minutes of probe runs; the full probe set was run once on each of release and debug builds, with the lint probes run once.
2. Hash ladders, replay, thread-count and cross-platform bit-identity are not in this sprint's scope (no world, no RNG yet); `fixed` is `const fn`/pure integer, and I did not run a second architecture. Results were identical in release and in debug (overflow-checked) builds.
3. `cargo nextest` was run by `check_all.sh`; I did not re-read the developer's own property tests beyond mutation-testing the code they cover.
4. `my_f32`-style and `stringify!` outcomes are recorded as observations from the lint's behaviour, not from a stated contract rule.
5. The Architect audit `VER-SPR-010` is not part of this file.
