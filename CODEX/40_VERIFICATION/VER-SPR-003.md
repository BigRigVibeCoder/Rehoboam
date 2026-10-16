---
id: VER-SPR-003
title: "Audit of SPR-003"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-003, VER-SPR-003-TEST]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** PASS. SPR-003 vendors the five samplers: binomial (BTPE and BINV), Poisson (PTRS and inversion), normal (ziggurat), Gamma (Marsaglia–Tsang) and Beta. They live in `rh-sample`, the one plane-A crate where float arithmetic is allowed, with `libm` for transcendentals and integer or fixed-point results only. The independent Tester found no correctness defect in the samplers: an independent Python port reproduces every golden line bit for bit, and outputs are identical across ten build configurations. Its main work was attacking the float zone's gates. Across four rounds it found 13 ways around them, and each was closed with a planted control. Change class C3. The Architect merges (GOV-005 §6).

# VER-SPR-003 — Audit of SPR-003

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh` | PASS (137 checks) | Developer and Tester at `911ef66`; Architect at the audit commit |
| CI on GitHub-hosted runners | PASS at `911ef66` (run 38879908795) | PR #16 |
| Nightly tier (64× statistics) | PASS at `911ef66`: 4 of 4 ignored tests in the Architect's container (`rh-sample` χ², moments and tails; `rh-ids` V-DET-20) and on GitHub's runner (run 38880438894). Required before a C3 sampler merge from now on (§5) | `.github/workflows/nightly.yml` |
| Change class | C3: determinism machinery (draw consumption and outputs that later systems' hashes cover); no golden world exists yet | — |

## 2. Tester verdicts
| Round | Head | Result |
|---|---|---|
| 1 | `743ce83` | T-01 to T-04 PASS. T-00 and T-05 PASS with GAP-1 to GAP-7. DEF-1 (documented draw counts). Evidence: 136 golden lines plus 2,650 random parameter sets reproduced bit for bit by an independent Python port; 27 cases identical across 10 builds (opt-level 0 to 3, s, LTO, `+fma`, native); 98,675 f64→`Q16_16` conversions equal to an exact rational reference; ziggurat tables byte-identical to `rand_distr` 0.6.0; up to 4×10⁸ draws per distribution case. 50 gate mutations: 36 red, 14 green (the gaps). 43 sampler mutants: 24 caught, 8 equivalent in law, 10 below any feasible detectability, and 1 caught only by the nightly tier |
| §8 | `fbe24b0` | The 13 green mutations are now red. New: GAP-9 (a crate-level `[lints]` override disables the transcendental ban), GAP-10, GAP-11 |
| §9 | `91f1ea6` | GAP-9, GAP-10 and the alias rule closed. New: GAP-12 (a crate-local `clippy.toml` replaces the root list), GAP-11b |
| §10 | `e6e03d2` | GAP-12 and GAP-11b closed. New: GAP-13 (contrived: a symlinked member under a pruned directory) |
| §11 | `911ef66` (rebased; report `c5d89e0`) | **PASS.** Rebase diff clean (23 commits paired; differences are merged lists and SPR-004 context). Python port 136/136 golden lines and 1,200 random sets; identical at opt-level 0, 1, 3 and LTO. 66 gate mutations: 61 red at the intended gate, 4 green as documented (B05 by ruling, the trait-method residual, two equivalent `[lints]` spellings), 1 invalid trial. GAP-13 closed, with variants: a symlinked `crates/`, a parent-level or root `.clippy.toml`, a hard link |

## 3. Defects and gaps: disposition
| Item | Disposition |
|---|---|
| DEF-1 documented draw counts wrong outside large parameters | Fixed (re-measured per regime) |
| GAP-1 the public API could expose `f64` | Fixed: `lint_forbidden` rule `float-api` over `rh-sample`'s pub surface and impl headers |
| GAP-2 42 of 44 `clippy.toml` names unguarded | Fixed: a pytest pins the exact 22 × 2 names plus `mul_add` |
| GAP-3 float exemption could widen unseen | Fixed: `FLOAT_EXEMPT_CRATES == {"rh-sample"}`, plus a float-in-`rh-core` control |
| GAP-4 ruling item 6 not enforced | Fixed: `plane-A-external` requires exact `=` pins, and `libm` with default features off |
| GAP-5(a) `libm::fma` | **Ruling:** allowed. It is a correctly rounded, deterministic software FMA; the ban covers hardware contraction and `mul_add` |
| GAP-5(b) an item-level `allow(disallowed_methods)` | Fixed: rule `lint-allow` (plane A; counted escapes only) |
| GAP-6 FMA contraction prevented only by rustc's default | Fixed: an `fp-contract` guard on `RUSTFLAGS` and cargo configs |
| GAP-7 draw positions depend on parameters | **Ruling:** the call-site rule, one keyed stream per sampler call (`05.1` §6 item 3) |
| GAP-8 PR-tier statistics are weak | Four binomial reference cases added. **Ruling:** a C3 sampler change needs the nightly tier green before merge |
| GAP-9 crate-level `[lints]` override | Fixed: `lint_arch` rule `lints` (plane-A crates have exactly `[lints] workspace = true`) |
| GAP-10 duplicate or target-specific `libm` edges, `features` | Fixed: every dependency entry is judged; `libm` features must be empty |
| GAP-11 `float-api` is a text rule | Partly fixed: every type alias in the zone is scanned. **Residual, accepted:** trait method signatures, projections, macro type tokens and `impl Trait` are covered by review (SPR-003 completion line). Moving to rustdoc JSON is BCK-002 A-26 |
| GAP-11b a `use … as` rename of `f64` | Fixed |
| GAP-12 crate-local `clippy.toml` | Fixed: `lint_arch` rule `clippy-config`, plus a pytest for `CLIPPY_CONF_DIR` |
| GAP-13 a symlinked member under a pruned directory | Fixed: `clippy-config` follows clippy's own ancestor lookup from each member's real path and reports a symlinked member (control `clippy_config/symlinked_member`) |
| OBS: `[env] CLIPPY_CONF_DIR` aimed at a pruned directory | Aimed at `node_modules/`, it is now red at `lint_arch`. Aimed at `target/`, it is stopped only by the std-ln clippy control running under the same cargo config. Recorded as a dependency between controls |

**Architect's review of `rh-sample`'s public items** (the GAP-11 residual, at the audit commit): five types (`Binomial`, `Poisson`, `Normal`, `Gamma`, `Beta`), each with `new` taking `u32` or `Q16_16` and returning `Result<Self, SampleError>`, and `sample(&self, &mut RngStream)` returning `u32` or `Q16_16`. `SampleError` exposes only `&'static str`. There are no public fields, traits, macros or `impl Trait`. No float crosses the API.

**Escapes found later:** none so far.

## 4. Acceptance criteria
| Task | Criterion | Met |
|---|---|---|
| T-00 | A float elsewhere in plane A, a `std` `ln`/`exp` in the zone, or the zone relabelled each fail `check_all` | Yes, plus the hardening above |
| T-01 | Golden vectors; chi-square against the exact binomial | Yes (11 reference cases, including n = 2³²−1) |
| T-02 | Golden vectors; moment tests (PTRS) | Yes |
| T-03 | Golden vectors; tail test (ziggurat) | Yes (tails to 6 sd at 4×10⁸ draws in the Tester's runs) |
| T-04 | Golden vectors; moment tests (Gamma, Beta) | Yes |
| T-05 | No floating-point drift; identical on a second machine and at opt-level 0 and 3 | Yes (10 builds identical) |

## 5. Contract edits and rulings in this audit
- **`05.1` §6 item 3:** the call-site rule (GAP-7) and the `libm::fma` ruling (GAP-5a).
- **GOV-005 §6 and the `docs/09` V-DET-09 row:** a C3 change to a sampler or its reference generator needs the nightly tier green on its head before merge (GAP-8).
- **BCK-002 A-26:** move `float-api` to rustdoc JSON when its format is stable.
- **Decisions log:** the float-zone rulings above.

## 6. Verdict
| Field | Value |
|---|---|
| Verdict | **PASS** |
| Merge route | The Architect merges (GOV-005 §6), after the nightly tier is green on the head |
| Carried forward | BCK-002 A-21 (`UniformDraw` for `RngStream`), A-26 (rustdoc JSON) |
| Escapes found later | — |
