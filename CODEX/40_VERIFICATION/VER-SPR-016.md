---
id: VER-SPR-016
title: "Audit of SPR-016 (M2 F1 mortality: Brass schedule, monthly trials, and density modifier)"
type: record
status: DRAFT
owner: architect
agents: [architect, coder]
tags: [verification, determinism, simulation]
related: [SPR-016, SPR-015, PRJ-002, GOV-002, GOV-003, ADR-0006, ADR-0025, ADR-0033]
created: 2026-10-20
updated: 2026-10-20
version: 0.9.0
---

> **BLUF: PENDING CI GREEN.** SPR-016 delivers the M2 F1 mortality schedule and monthly mortality trials (`PRJ-002`, priority 1, D62; `CODEX/05_PROJECT/SPR-016_MortalityF1.md`) into Rehoboam:
> - **T-01:** `Q32_32` fixed-point numeric primitives (`exp`, `ln`, `prob_from_hazard`) in `rh_core::fixed`, verified bit-identical against `specs/numerics/exp_ln_v0.py` and `specs/numerics/golden/exp_ln_v0.txt`.
> - **T-02:** F1 Brass mortality schedule compilation across all 438 knots, infant sub-schedule below 1 year, logistic old-age tail up to age 120, accepting arbitrary $\beta \in [0.6, 1.65]$ and compiling density parameters (`density_capacity`, `density_mortality_slope`).
> - **T-03 / T-04:** Monthly Bernoulli mortality trials (MR10–MR12) driven by keyed SplitMix64 RNG, density-dependent hazard scaling ($m_d$), newborn first-month draws, timer resampling on density updates, and obituary records extended with age in the DEATH event payload.
> - **H4 Integration:** Proper household dissolution and marriage closure upon spouse death.
> - **Critical Defect Remediation:** Corrected `household_on_marry` from unordered `Vec::push()` to binary search sorted insertion, restoring strict compliance with `INV-M2-09`.
> - **T-05:** Golden baselines regenerated via canonical `bin/regen_baseline.sh` for `solo`, `household`, and `hamlet` (`smoke` unchanged); committed under `[C3] [perf-baseline]`.
> - **Change Class:** C3 (demographic model upgrade moving baseline ladders). Merges to `main` via merge commit pinned to passing head SHA.

---

# VER-SPR-016: Audit of SPR-016

## 1. Automated Checks & Gates
| Gate / Check | Expected Target | Status |
|:---|:---|:---|
| `cargo test -p rh-core --lib` | Fixed-point `Q32_32` transcendental tests | **PASS** |
| `cargo test -p rh-sys-demography --lib` | Mortality schedule, knots, density multiplier, and partnering unit tests | **PASS** |
| `cargo test -p rh-sys-demography --test partnering_tests` | 7 partnering & marriage lifecycle tests | **PASS** |
| `cargo test -p rh-sys-demography --test draws_golden` | Rung-0 parity & differential draws validation | **PASS** |
| `specs/demography/selftest_m2_f1_mortality.py` | 9-step Python reference selftest | **PASS** |
| `bin/check_all.sh --part core` | All workspace crates, lints, manifests, and controls (560 checks) | **PASS** |
| `bin/check_all.sh --part golden` | Golden ladders, perf gate, and baseline comparisons | **PENDING CI** |
| `bin/check_all.sh --part stream` | Parquet outputs, sifter, and tail snapshots | **PENDING CI** |

---

## 2. Invariant & Architecture Audit

### 2.1 Fixed-Point & Determinism Primitives
- Transcendentals `exp` and `ln` implemented on `Q32_32` without floating-point instructions or standard library dependencies.
- Error bound across domain matches specification $\epsilon \le 2^{-30}$.
- Keyed `SplitMix64` RNG ensures completely deterministic, platform-independent mortality rolls across all seeds.

### 2.2 Schedule Compilation & Density Multiplier
- Mortality schedule pre-computes 438 knots from infant sub-schedule up to 120 years.
- When $N \le \text{cap}$, density modifier $m_d = 1.0$ (65,536 in Q16.16), remaining completely inert in standard sparse configurations.
- Over-capacity scaling $m_d = \min(4 \times 65536, 65536 + \text{rhe}(S_d \times (N - \text{cap}) / \text{cap}))$ smoothly increases mortality pressure under overcrowding.

### 2.3 Critical Invariant Fix: `INV-M2-09` Sorted Households
- **Defect Discovered:** During Stage 3 testing, new households created upon marriage via `household_on_marry` were appended with `Vec::push()`. This violated the invariant that households must be stored strictly sorted by `household_id` to enable `binary_search_by_key()`.
- **Consequence:** Lookups for existing households failed silently or returned unexpected indices, breaking `INV-M2-09`.
- **Resolution:** Replaced `push()` with `binary_search_by_key()` lookup followed by sorted `insert()`. Verified with 7 comprehensive partnering tests.

---

## 3. Golden Baseline Modifications (`[C3]`)
Baselines regenerated via `bin/regen_baseline.sh`:
- `smoke`: Completely unchanged (`fbcc6f644525b2d2`) — smoke world contains no M2 demography.
- `solo`: Regenerated (`35989528b9279793`).
- `household`: Regenerated (`f0a4e82fa07e5269`).
- `hamlet`: Regenerated (`fa891ff380741638`).
Baselines committed with required `[perf-baseline]` tag and `Reason:` explanatory body per `bin/perf_gate.py`.

---

## 4. Final Verdict & Merge Route
*(Awaiting final green CI run from PR #88 push)*
