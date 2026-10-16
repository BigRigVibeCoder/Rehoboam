---
id: SPR-003
title: "Samplers with golden vectors"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001, SPR-002, SPR-010, ADR-0006]
created: 2026-10-16
updated: 2026-10-16
version: 1.1.0
---

> **BLUF:** Vendor the distribution samplers the systems need, so that every draw is bit-identical across releases and machines. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-003 — Samplers with golden vectors

- **Roadmap item:** `docs/11` P0.3 · **Phase:** P0 (gate G0)
- **Depends on:** SPR-002 · **Plan approved by the director 2026-10-16** (P0 batch)
- **Binding contracts:** ADR-0006 items 3–4; `05.1` §6 item 3; DET-3; `docs/14` (vendored rand_distr: BTPE, ZIGNOR; libm)

## Architect ruling, 2026-10-16: the sampler float zone
SPR-010's float gate allows a float only in a declared `f32_field` column. ADR-0006 and `05.1` §6 item 3 plan vendored floating-point samplers, using `libm` only and no FMA. The gate was narrower than the contract, and this ruling reconciles the two:
1. **The samplers live in a new crate, `rh-sample`** (plane A, kind machinery). It is pinned by name in `bin/lint_arch.py`. System crates may depend on it: it joins the allowed dependencies of `rh-sys-*` (`docs/13` §3, `05.0` §6).
2. **`rh-sample` is the only plane-A crate where float arithmetic is allowed.**
   - The `float` rule of `bin/lint_forbidden.py` exempts it by its pinned name; every other rule still applies.
   - A positive control shows the exemption reaches no other crate.
   - The clippy float lints are allowed in it by one crate-level `#![allow(..., reason)]`.
3. **Transcendentals come from `libm` only.**
   - `clippy.toml` disallows the `std` float transcendentals workspace-wide: `ln`, `log`, `log2`, `log10`, `ln_1p`, `exp`, `exp2`, `exp_m1`, `powf`, `powi`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `cbrt`, `hypot`, for `f32` and `f64`. A positive control is required.
   - `sqrt` is allowed, because IEEE 754 rounds it correctly.
   - `mul_add` stays forbidden.
4. **Floats never leave the crate.**
   - The public API returns integers (counts) or `rh_core::fixed` values, never `f64`.
   - An internal `f64` becomes fixed point from its exact value, rounded half to even (the `rh_core::fixed` rule), without going through a cast.
   - Draws come from `rh-ids`'s keyed streams. Each sampler documents how many draws it consumes; for rejection samplers the count varies, but it is deterministic given the stream.
5. **Port, don't depend.** Vendor the `rand_distr` algorithms (MIT OR Apache-2.0) with their licence header; `rand_distr` itself depends on `rand`, which plane A may not use. Its Poisson is Knuth plus rejection, not PTRS, so write PTRS (Hörmann 1993) from the paper.
6. **External dependencies:** `libm` only, plus workspace crates. Record it for the plane-A dependency allow-list (BCK-002 A-20).
- **Requirements covered:** DET-3

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C3 (determinism machinery) |
| Planes touched (ADR-0024) | A (machinery) |
| Owned columns written | none |
| RNG purposes | uses SPR-002 streams |
| Golden-hash effect | none yet |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-DET-09 (bit-equal golden vectors for every sampler) |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change (V-DET-19's float exemption, V-DET-09); `CLAUDE.md` checks; `05.0` §6 and `docs/13` §3 (the new crate) |

## Tasks

### T-00: The float zone
- **Deliverable:** the `rh-sample` crate and the ruling above in the gates: the `lint_arch` pin and the system allow-list; the `lint_forbidden` exemption by name; the clippy transcendental list; and the docs (`05.0` §6, `docs/13` §3, `05.1` §6 item 3, the `docs/09` V-DET-19 row)
- **Acceptance:** each of these turns `bin/check_all.sh` red:
  - a float in any other plane-A crate;
  - a `std` `ln` or `exp` in `rh-sample`;
  - `rh-sample` relabelled out of plane A.

  A float in `rh-sample` passes `lint_forbidden`.
- **Status:** [ ] not started

### T-01: Binomial
- **Deliverable:** BTPE plus inversion for small n·p
- **Acceptance:** Golden vectors; chi-square sanity against the exact distribution
- **Status:** [ ] not started

### T-02: Poisson
- **Deliverable:** PTRS (Hörmann 1993), written from the paper because `rand_distr`'s Poisson is not PTRS
- **Acceptance:** Golden vectors; moment tests
- **Status:** [ ] not started

### T-03: Normal
- **Deliverable:** Ziggurat
- **Acceptance:** Golden vectors; tail test
- **Status:** [ ] not started

### T-04: Beta and Gamma
- **Deliverable:** Marsaglia–Tsang Gamma; Beta from two Gammas
- **Acceptance:** Golden vectors; moment tests
- **Status:** [ ] not started

### T-05: No floating-point drift
- **Deliverable:** Fixed operation order; no fused multiply-add; `libm` for every transcendental; documented
- **Acceptance:** V-DET-19 passes. The golden vectors are identical on GitHub's runner and in the developer's container (a second Linux machine), and at opt-level 0 and 3
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-003` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-003` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
