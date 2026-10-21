# SPEC-002: Mortality, Succession & Cohort Aging Invariants

- **Sprint / Wave:** `WAVE-002` (P1a Bloodlines)
- **Author:** Architect Coordinator (`claude-architect`)
- **Status:** `ACTIVE` *(Moves to `.spec/archive/` with `VER-SPEC-002_WaveAudit.md` upon wave completion)*
- **Created:** 2026-10-21

---

## 1. Observable Outcomes (Behavioral State Transitions)
- [ ] **Day/Vital Mortality Drain (`ScheduleDeath`):** When the `Day`/`Vital` pulse occurs, queued `ScheduleDeath` commands drain without event loss (`REV-P1a MAJ-4`). Deceased persons have `alive = false` updated in the person table, exit timers are cleared, and `DEATH` events are emitted with causal references.
- [ ] **Dynastic Succession (`SuccessionF1`):** When a titled ruler or lord dies, dynastic heir evaluation identifies the legal successor according to agnatic primogeniture / succession law. The title transitions to the new holder, or triggers an interregnum crisis if no valid heir exists.
- [ ] **Cohort Aging Invariants:** Invariant checks verify that cohort ages advance monotonically across day/year boundaries with zero drift, no negative intervals, and deterministic death probability progression.
- [ ] **Edge/Error Handling:** Re-entrant death calls on already deceased individuals return `DeathDrainOutcome::AlreadyDead` without emitting redundant events or panicking.

## 2. Explicit Non-Goals (Out of Scope — Enforced by Gemini Verifier)
- **DO NOT** implement P1b systems (`S6.artifacts`, `S7.vows`, or `S8.harvest`).
- **DO NOT** modify any files outside the decomposed task `Allowed Write Paths`.
- **DO NOT** add new third-party dependencies to `Cargo.toml` or Python environments.
- **DO NOT** use `HashMap`, `HashSet`, or `SystemTime::now()` in plane A simulation code (strict `BTreeMap` and deterministic integer/fixed-point arithmetic only).

## 3. Frozen Interface Contract (`CON-002` — Committed to `main` Before Worker Dispatch)
> *"The contract is truth. The code is an attempt to match it."* (GOV-007 §3.1)

- **Contract / Trait Files:**
  - `crates/rh-sys-demography/src/contracts.rs`
  - `crates/rh-sys-dynast/src/succession.rs`
- **Reference Spec Scripts:**
  - `specs/demography/selftest_m2_f1.py`
  - `specs/demography/selftest_m2_f1_mortality.py`
- **Input / Output & Error Schema:**
  | Field / Method | Input Type & Constraints | Output Type | Deterministic Error Behavior (GOV-004) |
  |:--|:--|:--|:--|
  | `drain_pending_deaths` | `victim: PersonId, cause: u32, cause_event: EventRef` | `Result<DeathDrainOutcome, SimError>` | Returns `DeathDrainOutcome::AlreadyDead` if dead; explicit `SimError` on state store corruption; zero `.unwrap()` |
  | `arbitrate_ruler_death` | `deceased_ruler: PersonId, title_id: u64` | `Result<SuccessionOutcome, SimError>` | Returns `SuccessionOutcome::Interregnum` if no candidate; zero panics |

- **Frozen Types & Signatures:**
  ```rust
  // crates/rh-sys-demography/src/contracts.rs
  pub enum DeathDrainOutcome {
      Processed { victim: PersonId, cause: u32 },
      AlreadyDead,
      Pending,
  }

  pub trait ScheduleDeathDrain {
      fn drain_pending_deaths(
          &mut self,
          victim: PersonId,
          cause: u32,
          cause_event: EventRef,
      ) -> Result<DeathDrainOutcome, SimError>;
  }

  // crates/rh-sys-dynast/src/succession.rs
  pub enum SuccessionOutcome {
      SuccessorFound { heir: PersonId },
      Interregnum,
      Extinct,
  }

  pub trait SuccessionF1 {
      fn arbitrate_ruler_death(
          &self,
          deceased_ruler: PersonId,
          title_id: u64,
      ) -> Result<SuccessionOutcome, SimError>;
  }
  ```

## 4. Mandatory Governance Compliance Table (`GOV-001..008`)
| Governance Standard | Mandatory Requirement for All Leaf Tasks in This Wave |
|:--|:--|
| **GOV-002 (Testing)** | 1:1 test coverage required: every modified `.rs`/`.py` source file must include unit/spec test assertions. No `#[ignore]` or `.skip`. |
| **GOV-003 (Determinism)** | `BTreeMap` / `BTreeSet` / sorted `Vec` only in `crates/rh-*`. Zero `HashMap`, `SystemTime::now()`, or `thread_rng()`. |
| **GOV-004 (Errors)** | Explicit typed `Result<T, E>` error handling. Zero runtime `.unwrap()` or `panic!()` on fallible inputs. |
| **GOV-005 (Commits)** | Forensic commit trailers (`Why:`, `What:`, `Files:`, `Agent:`, `Refs:`) enforced via `scion_api_submit.py`. |
| **GOV-007 (Discrepancy)** | If this contract is ambiguous, Worker MUST run `cerberus_orchestrator.py report-upstream --type EVO` rather than guessing. |
| **GOV-008 (Hygiene)** | Zero secrets, `.env` files, `*.pyc`, `*.log`, or `target/` artifacts permitted in diff. |

## 5. Deterministic Verification Criteria
Every worker must pass these exact commands with exit code `0` before opening a PR:
```bash
python3 -I specs/demography/selftest_m2_f1.py
cargo test -p rh-sys-demography
cargo test -p rh-sys-dynast
```

## 6. Decomposed Leaf Work Orders (Forgejo Issues)
- [ ] **Issue #4 (Work Order 1):** WAVE-002 Implement Day/Vital ScheduleDeath drain in rh-sys-demography | Allowed Write Paths: `crates/rh-sys-demography/*, specs/demography/*, tests/*` | Worker: `worker-or-01`
- [ ] **Issue #6 (Work Order 2):** WAVE-002 Implement SuccessionF1 dynastic arbitration in rh-sys-dynast | Allowed Write Paths: `crates/rh-sys-dynast/*, specs/dynasty/*` | Worker: `worker-or-02`
