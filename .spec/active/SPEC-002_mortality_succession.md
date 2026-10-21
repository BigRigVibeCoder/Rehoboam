# SPEC-002: Mortality, Succession & Cohort Aging Invariants

- **Sprint / Wave:** `WAVE-002`
- **Author:** Architect Coordinator (`claude-architect`)
- **Status:** `ACTIVE` *(Moves to `.spec/archive/` with `VER-002_WaveAudit.md` upon wave completion)*
- **Created:** 2026-10-21

---

## 1. Observable Outcomes (Behavioral State Transitions)
- [ ] When `ScheduleDeath { victim, cause, culprit, cause_event }` is processed in `Day`/`Vital` phase, `apply_death` executes idempotently and emits a `DEATH` event without event loss (`REV-P1a MAJ-4`).
- [ ] When an already dead or non-existent victim is targeted by `ScheduleDeath`, the command is dropped with `DropReason::Dead` and reported without panicking.
- [ ] Brass general mortality and infant sub-schedule evaluations in `specs/demography/selftest_m2_f1_mortality.py` reproduce bit-for-bit invariants across all 8 checks.
- [ ] Dynastic succession (`SuccessionF1`) evaluates heir priority deterministically across titled lines upon ruler death.

## 2. Explicit Non-Goals (Out of Scope — Enforced by Gemini Verifier)
- **DO NOT** implement off-schedule instantaneous death execution outside `Day/Vital` drain.
- **DO NOT** modify any files outside the decomposed task `Allowed Write Paths`.
- **DO NOT** add new third-party dependencies to `Cargo.toml` or Python environments.
- **DO NOT** use `HashMap` or `SystemTime` in simulation logic (strictly `BTreeMap` and deterministic tick counters).

## 3. Frozen Interface Contract (`CON-002` — Committed to `main` Before Worker Dispatch)
> *"The contract is truth. The code is an attempt to match it."* (GOV-007 §3.1)

- **Contract / Trait File:** `crates/rh-sys-demography/src/demography.rs`
- **Reference Spec Script:** `specs/demography/selftest_m2_f1_mortality.py`
- **Input / Output & Error Schema:**
  | Field / Method | Input Type & Constraints | Output Type | Deterministic Error Behavior (GOV-004) |
  |:--|:--|:--|:--|
  | `schedule_death` | `(view: &mut SystemView, ctx: &mut StepCtx, q: &QueuedCommand)` | `Result<(), SimError>` | Returns typed `SimError` on missing victim/cause; zero `.unwrap()` |
  | `drain` | `(view: &mut SystemView, ctx: &mut StepCtx)` | `Result<(), SimError>` | Dispatches `SCHEDULE_DEATH` and `MARRY` in queue order |

- **Frozen Types / Signatures:**
  ```rust
  pub const SCHEDULE_DEATH: &str = "ScheduleDeath";

  fn schedule_death(
      view: &mut SystemView<'_>,
      ctx: &mut StepCtx<'_>,
      q: &QueuedCommand,
  ) -> Result<(), SimError>;

  fn drain(view: &mut SystemView<'_>, ctx: &mut StepCtx<'_>) -> Result<(), SimError>;
  ```

## 4. Mandatory Governance Compliance Table (`GOV-001..008`)
| Governance Standard | Mandatory Requirement for All Leaf Tasks in This Wave |
|:--|:--|
| **GOV-002 (Testing)** | 1:1 test coverage required: every modified source file must include unit/spec test assertions. |
| **GOV-003 (Determinism)** | `BTreeMap` / `BTreeSet` / sorted `Vec` only in `crates/rh-*`. Zero `HashMap`, `SystemTime::now()`, or `thread_rng()`. |
| **GOV-004 (Errors)** | Explicit typed `Result<T, E>` error handling. Zero runtime `.unwrap()` or `panic!()` on fallible inputs. |
| **GOV-005 (Commits)** | Forensic commit trailers (`Why:`, `What:`, `Files:`, `Agent:`, `Refs:`) enforced via `scion_api_submit.py`. |
| **GOV-007 (Discrepancy)** | If this contract is ambiguous, Worker MUST run `cerberus_orchestrator.py report-upstream --type EVO` rather than guessing. |
| **GOV-008 (Hygiene)** | Zero secrets, `.env` files, `*.pyc`, `*.log`, or `target/` artifacts permitted in diff. |

## 5. Deterministic Verification Criteria
Every worker must pass these exact commands with exit code `0` before opening a PR:
```bash
cargo test -p rh-sys-demography
```

## 6. Decomposed Leaf Work Orders (Forgejo Issues)
- [ ] **Issue #4:** Implement Day/Vital ScheduleDeath drain in rh-sys-demography | Allowed Write Paths: `crates/rh-sys-demography/*, specs/demography/*, tests/*` | Worker: `worker-or-01`
