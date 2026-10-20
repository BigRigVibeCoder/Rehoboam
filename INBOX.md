# Exuberant (AGT-002-CORE): Work Order — Stage B (Partnering & Households)

**Role:** Core Developer (Rust Core Engine)  
**Assigned VM:** `exuberant`  
**Branch:** `sprint/SPR-015-births-f1` (PR #107)  
**Date:** 2026-10-18  

---

## 1. Status Update & Verification Report
- ✅ **Task T-03 Approved**: Your `[algorithm.fertility_f1]` implementation passed all 15 crate tests, the 9-step Python selftest, and all 6 Red Team adversarial failure-mode traps.
- ✅ **Downstream Fixtures Approved**: Commit `3ead3ab` properly updated downstream test fixtures.
- ℹ️ **Genesis Invariant Fix**: The Architect on Adjutant is handling the `rh-worldgen` sentinel column plumbing for `person.conceived_tick` to turn PR #107 CI green. Before starting, run `git pull origin sprint/SPR-015-births-f1` to pick up any changes.

---

## 2. Stage B Implementation: Task T-04 (Partnering & Marriages)

Implement `[algorithm.partnering_f1]` (N1–N7, H3, H4; INV-M2-08, 11, 13) in `crates/rh-sys-demography`:

### 1. Constants & Hazard Lookup (N1–N2)
- Monthly nuptiality hazard $\mu_i$ from `constants.nuptiality_thousandths`.
- Marriage age bounds: `MIN_AGE_F = 15`, `MIN_AGE_M = 18`.

### 2. Month/Vital Partnering Step (N3–N5)
- Scan eligible women (unmarried, female, alive, age in marriage range).
- Draw $k = 0$ against $\mu_i$.
- On a hit: scan candidate men (alive, male, unmarried, age $\ge 18$, `named = 0`, `holy_orders = 0`, not matched this step, not in `kin(w, 2)` over the index).
- **CRN Husband Pick**: Select candidate with minimum keyed hash `H(husband_pick, branch_salt, candidate_id)`.
- Schedule `MARRIAGE_TIMER` on day $d$ ($k = 1$).

### 3. `apply_marry` (N7)
- Re-check ages for M2 timers.
- Set reciprocal `person.spouse` on both parties (`a.spouse = b`, `b.spouse = a`).
- Append `marriage` row (`source = partnering` or `source = command`).
- Emit `MARRIAGE` event (`actor` = husband, `target` = wife).
- Update household membership per H3.

### 4. H4 Household Dissolution & Widowhood
- On spouse death in S12: clear `person.spouse`, close marriage row (`end_tick = now`), transfer headship to surviving spouse or oldest living member, dissolve household if empty.

---

## 3. Verification Commands
- `cargo check --tests --workspace`
- `cargo test -p rh-sys-demography`
- `cargo test -p rh-worldgen`

When ready, commit as:
`feat(SPR-015): T-04 implement partnering, apply_marry, and households F1`
and push to `origin/sprint/SPR-015-births-f1`.
