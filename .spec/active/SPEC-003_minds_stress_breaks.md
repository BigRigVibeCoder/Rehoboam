# SPEC-003: Minds v0 & Mental Break Invariants

- **Sprint / Wave:** `WAVE-003` (P1a Bloodlines)
- **Author:** Architect Coordinator (`gemini-architect`)
- **Status:** `ACTIVE 🚀`
- **Created:** 2026-10-21

---

## 1. Observable Outcomes (Behavioral State Transitions)

- [ ] **I-Step: Grief Observation & Per-Instant Saturation (`MindsIStep`):**
  When deceased persons are reported at an instant (via `StepCtx::exited()` and the engine instant log's `DEATH` events), each living named actor with a recognized kin or relational bond (`parent: 1.0`, `child: 1.0`, `spouse: 1.0`, `sibling: 0.6`, `grandparent: 0.4`, `kin_other: 0.2` per `[relation_kinds]`) accrues grief stress. The total grief an actor receives in a single instant saturates at `grief_instant_cap` (bounded in `[5.0, 45.0]` points, strictly below the lower bound of `stress_threshold` at `50.0`, per `REV-P1a MIN-5`). Each grief accrual records a `GRIEF_FOR` motive reference pointing to the inciting `DEATH` event.
- [ ] **S-Step: Deterministic Stress Accounting & Invariant Identity (`V-SAGA-16`):**
  Stress is tracked as a fixed-point `q8_8` integer on a `0..100` point scale (`0..=25600` raw units). The exact accounting invariant holds bit-for-bit with zero drift:
  $$\text{stress}_{\text{current}} = \text{initial} + \text{grief}_{\text{accrued}} + \text{dissonance}_{\text{accrued}} - \text{stress}_{\text{relieved}} - \text{stress}_{\text{decayed}}$$
  Zero floating-point operations (`f32`/`f64`) are permitted in plane A.
- [ ] **S-Step: Threshold Crossing & Keyed Delay (`BreakState::Pending`):**
  When `stress >= stress_threshold` (evaluated during state transition) and the actor's break state is `Normal`, the actor transitions to `BreakState::Pending`. A break timer is scheduled with a deterministic delay in days drawn uniformly via `Prob32` over `[1, break_delay_max_days]`, seeded by the actor's epoch-keyed RNG stream (`rh-ids`). The break timer carries the crossing's inciting cause event (`EventId`).
- [ ] **Break Execution, Relief & Refractory Cooldown (`V-SAGA-16b`):**
  When a break timer fires:
  1. An act is selected from the typed `[mental_breaks]` enum based on availability of requirements (`stressor`, `seat`, `kin`, `holdings`) and weighted by HEXACO trait affinities:
     $$\text{weight}_k = \text{clamp}\left(1 + \sum_{t} w_{k,t} \cdot \frac{\text{trait}_t}{100}, 0.1, 2.0\right)$$
  2. The simulation emits `MENTAL_BREAK` (with a causal motive edge to the crossing cause) and the specific verb program (`FLED`, `DESPAIR_PACT`, `MURDER_ATTEMPT`, `HOLDINGS_BURNED`, `PARANOIA`, or `CATATONIA`).
  3. Stress is relieved: $\text{stress} \leftarrow \max(0, \text{stress} - \text{relief})$, and $\text{stress}_{\text{relieved}} += \text{applied\_relief}$.
  4. The actor transitions immediately to `BreakState::Refractory` until $\text{now} + \text{break\_refractory\_days} \times \text{TICKS\_PER\_DAY}$.
  5. During the refractory window, **zero new breaks may be scheduled under any condition**. Back-to-back breaks within the cooldown window are strictly forbidden.
- [ ] **Stress Cascade & Massacre Gate Invariants (`V-SAGA-16b`):**
  Under the stress court fixture (`specs/validation/forcing/court_deaths.toml`) and the massacre variant (4 kin deaths at closeness 1.0 in a single instant):
  - Every variant produces $\ge 30$ breaks across the ensemble without state divergence ("NOT EXERCISED" failure if count $< 30$).
  - Branching ratio upper 95% bound remains $< 1.0$.
  - The planted control mutant (`mutant-no-refractory`) fails the refractory assertion by name.
- [ ] **Edge / Error Handling:**
  - If a `Demote` command is received while `break_state` is `Pending` or `Refractory`, the demotion is dropped as deferred (`DropReason::Deferred`), preventing actors from going dormant with an active break cycle.
  - If an actor dies while a break is pending, the break timer is cancelled and recorded in the cancelled timer tally without orphan event emission.

---

## 2. Explicit Non-Goals (Out of Scope — Enforced by Gemini Verifier)

- **DO NOT** implement P1b systems (`S6.artifacts`, `S7.vows`, or `S8.harvest`).
- **DO NOT** use `f32` or `f64` anywhere in plane A simulation code. All stress, grief, weights, and relief math MUST use `q8_8` fixed-point integer arithmetic.
- **DO NOT** use `HashMap`, `HashSet`, or `SystemTime::now()` in `crates/rh-sys-minds/*`. Strict `BTreeMap`, `BTreeSet`, and deterministic epoch-keyed RNG streams (`rh-ids`) only.
- **DO NOT** implement any fiat outcome capping or tone filters (ADR-0009: damping is strictly mechanic-based via per-instant saturation and refractory cooldowns).
- **DO NOT** modify any files outside the decomposed task `Allowed Write Paths`.
- **DO NOT** add new third-party dependencies to `Cargo.toml` or Python environments.

---

## 3. Frozen Interface Contract (`CON-003` — Committed to `main` Before Worker Dispatch)
> *"The contract is truth. The code is an attempt to match it."* (GOV-007 §3.1)

- **Contract / Trait Files:**
  - `crates/rh-sys-minds/src/contracts.rs`
  - `crates/rh-sys-minds/src/stress.rs`
- **Reference Spec Scripts:**
  - `specs/minds/selftest_minds_v0.py`
  - `specs/minds/selftest_minds_stress_breaks.py`
  - `specs/validation/forcing/court_deaths.toml`
- **Input / Output & Error Schema:**
  | Field / Method | Input Type & Constraints | Output Type | Deterministic Error Behavior (GOV-004) |
  |:--|:--|:--|:--|
  | `observe_instant_deaths` | `deaths: &[DeathObservation], person_index: &PersonRelIndex` | `Result<GriefAccrualOutcome, MindsError>` | Returns `MindsError::InvalidRelation` on index mismatch; zero panics |
  | `evaluate_stress_and_breaks` | `actor: PersonId, current_tick: u64, rng: &mut EpochRngStream` | `Result<BreakEvaluationOutcome, MindsError>` | Returns `MindsError::ActorNotFound` if actor unpromoted; zero `.unwrap()` |
  | `apply_break_relief` | `actor: PersonId, break_kind: MentalBreakKind, current_tick: u64` | `Result<ReliefOutcome, MindsError>` | Returns `MindsError::RefractoryViolation` if fired while refractory |

- **Frozen Types & Signatures:**
  ```rust
  // crates/rh-sys-minds/src/contracts.rs
  use rh_ids::{EventId, PersonId};

  pub const Q8_8_ONE: u16 = 256;
  pub const MAX_STRESS_Q8_8: u16 = 100 * Q8_8_ONE; // 25600

  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
  pub enum MentalBreakKind {
      AssassinationAttempt,
      Flight,
      BurnHoldings,
      DespairPact,
      Paranoia,
      Catatonia,
  }

  impl MentalBreakKind {
      pub const fn relief_q8_8(&self) -> u16 {
          match self {
              Self::AssassinationAttempt => 60 * Q8_8_ONE,
              Self::Flight => 50 * Q8_8_ONE,
              Self::BurnHoldings => 50 * Q8_8_ONE,
              Self::DespairPact => 40 * Q8_8_ONE,
              Self::Paranoia => 35 * Q8_8_ONE,
              Self::Catatonia => 45 * Q8_8_ONE,
          }
      }

      pub const fn verb(&self) -> &'static str {
          match self {
              Self::AssassinationAttempt => "MURDER_ATTEMPT",
              Self::Flight => "FLED",
              Self::BurnHoldings => "HOLDINGS_BURNED",
              Self::DespairPact => "DESPAIR_PACT",
              Self::Paranoia => "PARANOIA",
              Self::Catatonia => "CATATONIA",
          }
      }
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum BreakState {
      Normal,
      Pending {
          delay_days: u32,
          scheduled_tick: u64,
          kind: MentalBreakKind,
          cause_event: EventId,
      },
      Refractory {
          refractory_until_tick: u64,
      },
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct StressState {
      pub current_stress_q8_8: u16,
      pub grief_accrued_q8_8: u32,
      pub dissonance_accrued_q8_8: u32,
      pub stress_relieved_q8_8: u32,
      pub stress_decayed_q8_8: u32,
      pub break_state: BreakState,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct DeathObservation {
      pub deceased: PersonId,
      pub death_event: EventId,
      pub instant_tick: u64,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum GriefAccrualOutcome {
      Accrued { total_grief_q8_8: u16, saturated: bool },
      NoRelationsAffected,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum BreakEvaluationOutcome {
      NoChange,
      BreakScheduled {
          kind: MentalBreakKind,
          delay_days: u32,
          scheduled_tick: u64,
          cause_event: EventId,
      },
      BreakFired {
          kind: MentalBreakKind,
          verb: &'static str,
          relief_applied_q8_8: u16,
          refractory_until_tick: u64,
      },
      RefractoryActive {
          remaining_ticks: u64,
      },
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum MindsError {
      ActorNotFound(PersonId),
      InvalidRelation,
      RefractoryViolation { actor: PersonId, active_until: u64 },
      CorruptedAccounting { expected: u32, actual: u32 },
  }

  pub trait MindsIStep {
      fn observe_instant_deaths(
          &mut self,
          deaths: &[DeathObservation],
      ) -> Result<GriefAccrualOutcome, MindsError>;
  }

  pub trait MindsSStep {
      fn evaluate_stress_and_breaks(
          &mut self,
          actor: PersonId,
          current_tick: u64,
      ) -> Result<BreakEvaluationOutcome, MindsError>;

      fn verify_accounting_invariants(
          &self,
          actor: PersonId,
      ) -> Result<(), MindsError>;
  }
  ```

---

## 4. Mandatory Governance Compliance Table (`GOV-001..008`)

| Governance Standard | Mandatory Requirement for All Leaf Tasks in This Wave |
|:--|:--|
| **GOV-001 (Docs)** | YAML frontmatter, BLUF-first architecture, clean separation of observable outcomes and contracts. |
| **GOV-002 (Testing)** | 1:1 test coverage required: every modified `.rs`/`.py` source file must include unit/spec test assertions. Zero `#[ignore]` or `.skip`. |
| **GOV-003 (Determinism)** | `BTreeMap` / `BTreeSet` / sorted `Vec` only in `crates/rh-*`. Zero `HashMap`, `SystemTime::now()`, `thread_rng()`, or `f32`/`f64` in plane A. |
| **GOV-004 (Errors)** | Explicit typed `Result<T, MindsError>` error handling. Zero runtime `.unwrap()` or `panic!()` on fallible inputs. |
| **GOV-005 (Commits)** | Forensic commit trailers (`Why:`, `What:`, `Files:`, `Agent:`, `Refs:`) enforced via `scion_api_submit.py`. |
| **GOV-006 (Logging)** | Structured logging with `actor`, `event_id`, and `stress_q8_8` correlation fields. |
| **GOV-007 (Discrepancy)** | If this contract is ambiguous, Worker MUST run `cerberus_orchestrator.py report-upstream --type EVO` rather than guessing. |
| **GOV-008 (Hygiene)** | Zero secrets, `.env` files, `*.pyc`, `*.log`, or `target/` artifacts permitted in diff. |

---

## 5. Deterministic Verification Criteria

Every worker must pass these exact commands with exit code `0` before opening a PR:
```bash
python3 -I specs/minds/selftest_minds_v0.py
cargo test -p rh-sys-minds
cargo clippy -p rh-sys-minds --all-targets -- -D warnings
```

---

## 6. Decomposed Leaf Work Orders (Forgejo Issues)

- [ ] **Issue #14 (Work Order 1):** WAVE-003 Implement `rh-sys-minds` state model, `q8_8` fixed-point stress accounting, I-step grief saturation, and kin relationship walks.
  - *Allowed Write Paths:* `crates/rh-sys-minds/src/contracts.rs`, `crates/rh-sys-minds/src/stress.rs`, `crates/rh-sys-minds/Cargo.toml`
  - *Worker:* `worker-or-01`
- [ ] **Issue #15 (Work Order 2):** WAVE-003 Implement S-step break scheduler, typed `[mental_breaks]` enum, refractory cooldown timers, and verb programs (`FLED`, `DESPAIR_PACT`, `MURDER_ATTEMPT`).
  - *Allowed Write Paths:* `crates/rh-sys-minds/src/breaks.rs`, `crates/rh-sys-minds/src/verbs.rs`, `crates/rh-sys-minds/src/lib.rs`
  - *Worker:* `worker-or-02`
- [ ] **Issue #16 (Work Order 3):** WAVE-003 Implement `specs/minds/selftest_minds_stress_breaks.py` & `V-SAGA-16b` test fixture (`tests/stress_breaks_golden.rs`).
  - *Allowed Write Paths:* `specs/minds/*`, `tests/stress_breaks_golden.rs`, `specs/validation/forcing/court_deaths.toml`
  - *Worker:* `worker-or-01`
