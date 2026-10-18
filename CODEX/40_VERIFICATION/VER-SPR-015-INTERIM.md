---
id: VER-SPR-015-INTERIM
title: "Interim Audit of SPR-015 (Stage A/B Checkpoint)"
type: record
status: DRAFT
owner: architect
agents: [architect, tester, red-team]
tags: [verification]
related: [SPR-015, REV-SPR-015-births, SPR-016, PRJ-002, RT-2026-10-18-batch-09-dossier, probe_fertility_adversarial_traps]
created: 2026-10-18
updated: 2026-10-18
version: 0.2.0
---

> **BLUF: INTERIM PROGRESS APPROVED WITH REMEDIATION REQUIRED.**  
> Developer Exuberant (`AGT-002-CORE`) has completed Stage A (T-01, T-02, T-03) and pushed Genesis alignment on `sprint/SPR-015-births-f1` (head `5ee789ff`, PR #107).  
> The Red Team adversarial probe `B09-01` (`probe_fertility_adversarial_traps.py`) has run against Exuberant's code and passed 6/6 adversarial traps.  
> CI run 39176612229 on PR #107 failed across 3 isolated, diagnosable gates (pytest fixture sync in core, keyframe state snapshot in stream, and instruction baseline in golden). Exuberant must apply the triage fixes below before Stage B closeout and final `[C3]` audit.

# VER-SPR-015-INTERIM: Interim Audit of SPR-015 (Stage A/B Checkpoint)

## 1. Scope & Task Progress

| Task | Description | Status | Evidence / Notes |
|:---|:---|:---:|:---|
| **T-01** | Promotion of M2 0.3.0 & Registry Delta Merge | **DONE** | Commit `a61e9195`. `specs/systems/M2.demography.odd.toml` promoted to frozen 0.3.0. Registry delta merged into `specs/manifest/registry_v0.toml`. |
| **T-02** | Derived Per-System Index in `rh-core` (C2) | **DONE** | Commit `34b5e4f9` & `a61e9195`. `SystemModel::Index` trait bound introduced with `build()` and `on_merge()`. Verified 0 allocations in quiet steps. |
| **T-03** | Fertility F1 in `rh-sys-demography` & `HouseholdId` in `rh-ids` | **DONE** | Commit `2519e912`. Coale–Trussell fertility schedule implemented, pregnancy timer (9 months), 105:100 sex ratio, and couple-level elite fertility read. |
| **T-04** | Partnering N1–N7, `apply_marry`, Households H3/H4 | **IN PROGRESS** | Active work on Exuberant node (`100.115.214.32`). Sole-household dissolution & re-opening logic in progress. |
| **T-05** | Genesis Sentinel Column Alignment | **DONE** | Commit `5ee789ff`. Added 5 owned columns (`conceived_tick`, `household`, `last_birth_tick`, `parity`, `unborn_father`) to `rh-worldgen` genesis. |

---

## 2. Red Team Adversarial Verification (Probe B09-01)

The Red Team (`arch/red-team-batch-04` commit `63046c9`) subjected the fertility implementation to the Batch 09 Dossier adversarial battery ([`probe_fertility_adversarial_traps.py`](file:///home/bdavidriggins/Documents/Rehoboam/tests/probes/probe_fertility_adversarial_traps.py)):

| Trap # | Invariant / Property | Red Team Scenario | Result |
|:---:|:---|:---|:---:|
| 1 | **Childbirth Death Race (`INV-M2-08`)** | Mother dies at `(t, Vital)` while scheduled to deliver at `(t, Vital)`. | **PASS** (Child born before maternal mortality executes; father linked). |
| 2 | **Compacted Mother Drop (`INV-M2-04`)** | Positional compaction drops mother during gestation across keyframe. | **PASS** (Childbirth resolves safely without panics or dangling references). |
| 3 | **Sole-Household Dissolution (`INV-M2-09`)** | Sole occupant leaves household; new occupant re-opens ID in same tick. | **PASS** (Lifecycle state correctly transitions; zero ID aliasing). |
| 4 | **Posthumous Father Linkage** | Father dies mid-gestation before child delivery. | **PASS** (`unborn_father` preserves genealogical lineage faithfully). |
| 5 | **100k Rehash Collision Boundedness** | Rehash of `HouseholdId` across 100,000 synthetic collision draws. | **PASS** (Collision bound strictly $\le 4$; zero infinite retry cycles). |
| 6 | **S12 Newborn Extinction Shield** | Last adult dies on the exact tick of a viable birth. | **PASS** (`EXTINCTION` event correctly suppressed while newborn survives). |

---

## 3. CI Triage & Remediation Plan for PR #107 (Run 39176612229)

Full triage of the 3 failures on commit `5ee789ff`:

### 1. Part Core Failure: Pytest Unit Tests (`test_registry_effect_extension.py`)
- **Diagnosis:** `python/tests/test_registry_effect_extension.py` line 37 asserts byte-for-byte identity between `specs/manifest/registry_v0.toml` (minus the `absolute_primogeniture` law) and `tests/registry_effect/registry_absolute_primogeniture.toml`.
- **Cause:** Commit `a61e9195` added `BIRTH`, `MARRIAGE`, and F1 parameters (`tfr`, `elite_fertility_mult`, `density_capacity`, `density_mortality_slope`) to `registry_v0.toml`, but did not update the fixture copy `registry_absolute_primogeniture.toml`.
- **Remediation for Exuberant:** Synchronize `tests/registry_effect/registry_absolute_primogeniture.toml` with the new M2 parameters and verbs from `registry_v0.toml`.

### 2. Part Stream Failure: Tail Snapshot Mismatch (`solo_story.txt`)
- **Diagnosis:** Gate 8 (`rh-golden::tail the_display_form_matches_its_snapshots`) panicked at line 46:
  ```diff
  -    121 0009-12-30 23:59 KEYFRAME keyframes/year-0010.rhkf state dc4436a63873b9b9
  +    121 0009-12-30 23:59 KEYFRAME keyframes/year-0010.rhkf state 14a9bacfe1c3bbc0
  ```
- **Cause:** Commit `5ee789ff` populated 5 new person sentinel columns in `rh-worldgen` genesis, altering the Plane A keyframe state hash. This is expected behavior for a C3 change.
- **Remediation for Exuberant:** Run with `RH_UPDATE_SNAPSHOTS=1 cargo nextest run -p rh-golden --release --test tail` and commit the updated snapshot `crates/rh-golden/tests/snapshots/solo_story.txt`.

### 3. Part Golden Failure: `V-PERF-04` Instruction Counts
- **Diagnosis:** Gate 9 reported regression in `instant/quiet` (+62.26%) and `instant/year` (+14.44%) against `crates/rh-bench/baselines/instructions_v1.txt`.
- **Cause:** Additional M2 demography passes and sentinel column maintenance increase baseline instruction counts.
- **Remediation for Exuberant:** As SPR-015 is a C3 `[perf-baseline]` sprint, re-record instruction baselines in the final commit using:
  ```bash
  python -I bin/perf_gate.py record
  ```
  Provide the required `Reason:` line in the commit body.

---

## 4. Architect Rulings for Stage B Closeout

1. **R-015-01 (Fixture Synchronization):** All fixture registries (`tests/registry_effect/registry_absolute_primogeniture.toml`) MUST strictly track `registry_v0.toml`.
2. **R-015-02 (Snapshot Authorization):** The solo keyframe state hash change to `14a9bacfe1c3bbc0` is AUTHORIZED as a valid consequence of M2 0.3.0 entity columns.
3. **R-015-03 (Perf Baseline Re-record):** Baseline updates for instruction counts are restricted to the final `[C3] [perf-baseline]` commit on PR #107.
4. **R-015-04 (Handoff to SPR-016):** Developer on `SPR-016` (PR #88) must hold Stage 2 until PR #107 lands with the new M2 column layout and keyframe hashes.
