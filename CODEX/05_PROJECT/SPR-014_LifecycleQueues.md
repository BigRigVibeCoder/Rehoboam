---
id: SPR-014
title: "Lifecycle queues for P1a: delivery, one draining binding, event-typed slots, the dead-person drop, absent owners (P1.13)"
type: how-to
status: ARCHIVED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-002, PRJ-002, EVO-001, REV-EVO-001, REV-P1a, REV-SPR-017, REV-SPR-015, SPR-006, SPR-013, SPR-015, SPR-020, SPR-026, ADR-0025, ADR-0033]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---
> [!NOTE]
> **ARCHIVED BASELINE:** Traditional sprint-based tracking has been superseded by Specification-Driven Development (SDD v2.0). All future deliverables are tracked as Spec Waves in `.spec/ARCHITECT_BACKLOG.md`.

> **BLUF:** Builds EVO-001 (`05.1` v1.12 §2 rule 2) in the engine. A command reaches only the one binding its owner's spec names, within the instant when that binding runs later. Event-typed slots in commands and timers are rewritten at the merge. The sprint also delivers the dead-person drop (BCK-002 A-30), the queue-age invariant (A-31), the absent-owner registration rule (REV-P1a NEW-4), M2's `Day`/`Vital` drain, the ledger row insert that births need, `rh-contracts` v0 for M2's columns, and A-50. `smoke` stays byte-identical; `solo`, `household` and `hamlet` move only through `rh-core/engine.meta`'s cursor, which M2's daily drain instants advance, and are re-recorded in a `[C3]` commit (REV-EVO-001 R1 and EVO-001 §5 as amended; T-08; the Architect's option A, 2026-10-17). Developer: AGT-002-CORE. Testers: AGT-003-DET and AGT-003-IF (one Tester session may hold both roles, on a different model from the Developer's). About 3.5 agent-days. The class is **C3** because the sprint changes scheduler semantics (PRJ-002); the only hash that moves is the engine cursor's. It contains C2 work: the contract crate and M2 0.2.3. **Plan approved by the director on 2026-10-17 (D48; GOV-005 §2).**

# SPR-014 — Lifecycle queues for P1a (WP P1.13)

- **Roadmap item:** `docs/11` P1.13 · **Phase / rung:** P1a, rungs 0–2 and `smoke`
- **Depends on:**
  - SPR-013 merged. Its `household` and `hamlet` baselines and its channel-A segments are what T-08 holds fixed.
  - The spec-table loader that BCK-002 A-51 moves into a tool crate, which T-01 extends. If A-51 slipped out of SPR-013, it falls to this sprint ("SPR-013 or SPR-014, whichever is planned first") and adds about 0.5 agent-day.
  - The merge order with SPR-020 is under "Merge route".
- **Binding contracts:**
  - `05.1` §2 rule 2, all six parts (v1.12); rule 4's **Threads** bullet (the merge order: cancellations, events, timers, commands); §4.3; §9.
  - `EVO-001` §2, §3, §5, §7.
  - `specs/chronicle/envelope_v0.md` §2: the exact `rh-core/agenda.timers` row with EVO-001's `[, u8 1 ‖ u64 cause]` tail, which SPR-020's origin tail follows.
  - `specs/systems/next/M2.demography.odd.toml` lines 84–87, with REV-SPR-015 R11 (the row insert).
  - ADR-0025, ADR-0033; `docs/09` V-DET-21, V-INV-15, V-INV-04 (as amended by EVO-001).
- **System spec:**
  - `specs/systems/M2.demography.odd.toml` goes from frozen 0.2.2 to **0.2.3** in this PR. It gains the `Day`/`Vital` `[[schedule]]` entry its `drained_by` names, and its two `sprint = "SPR-014"` escapes are removed. This is an Architect-approved C2 change.
  - `M7.dynasty` 0.3.3 and `T1.minds` 0.3.3 are read by registration tests only; neither is built here.
  - The smoke specs keep their declarations (`Settle` and `Audit`, `Day`/`Activity`, `key = "any"`, `key_field = "person"`).
- **Rulings (binding):**
  - REV-EVO-001 R1–R19, above all:
    - R1: the smoke ladder unchanged;
    - R2: `drained_by`, and a timer pass drains nothing;
    - R3: the drop's five points;
    - R4 and R12: absent owners;
    - R5: typed payloads;
    - R9: an outcome command is submitted only by its sender's timer;
    - R11: one `dropped{…}` diagnostic;
    - R15: both encodings hash-neutral;
    - NIT-10: drain bindings in the skeletons;
    - NIT-15: `drop_timer`.
  - REV-P1a MAJ-4, NEW-4, NEW-6, NEW-7.
  - REV-SPR-017 N3.
- **Requirements covered:**
  - DET-13 (no back-dating, kept as written: R9);
  - SIM-4 (registration and `AccessSpec`);
  - SIM-5 (commanded deaths at day starts);
  - EVO-1 (contract crates; the single death path, subset);
  - DATA-8 (the row-insert path, subset);
  - DET-1 and DET-2 (A-50).

## Specs this sprint needs

| Spec | Exists? | Gap | Writer | Needed by |
|---|---|---|---|---|
| `05.1` §2 rule 2 v1.12, rule 4, §4.3, §9 | Exists (PR #49) | — | — | T-01–T-04 |
| `lint_odd.py`: typed `[[applies]]`, `drained_by`, `key`, `key_field`, `outcome`; `neutral` on rate and mode reads; `timer` and `purpose` on outcome `[[commands]]` | Exists (PR #49), fixtures `odd_applies/`, `odd_neutral/` | — | — | T-01, T-04 |
| M2 0.2.3 with `Day`/`Vital` in `[[schedule]]` | Missing. 0.2.2 names `drained_by = "Day/Vital"` under a `sprint = "SPR-014"` escape that becomes an error once this document is CLOSED (R13, R18) | The binding, the step text, the version line | The Developer drafts; the Architect approves in this PR (C2) | T-05 |
| The ledger row-insert contract | **Exists:** `05.1` v1.14 §2 rule 2, "Ledger rows enter by insert" (Architect ruling at this plan, 2026-10-17, confirming REV-SPR-015 R11). Another owner's column is inserted at its declared `default` when it has one, else the zero of its type, so SPR-020's cause check (R26) accepts it | — | — | T-05 |
| How an `EventRef` payload field is encoded in `rh-core/engine.commands` | **Exists:** `05.1` v1.14 §2 rule 2, "Event-typed slots" (Architect ruling at this plan, 7/10): the field keeps its position in `list(i64 args)`, the resolved id or −1 for none. The layout does not change, smoke's rows do not change, and SPR-020's origin tail stays decodable by length | — | — | T-02 |
| `rh-contracts` v0 shape | Only "column types and read accessors, no implementation" (`docs/13` §4.1) | T-06 proposes hand-written typed accessors with a spec-drift test | — | T-06 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C3** by class: scheduler semantics, which is determinism machinery (PRJ-002 §2), with no world-table change on any golden world (R1 as amended, R15): the `state_hash` of `solo`, `household` and `hamlet` moves only through `rh-core/engine.meta`'s cursor, which M2's daily `Day`/`Vital` instants advance, re-recorded in the final `[C3]` commit (Architect ruling, 2026-10-17, option A). It contains C2: `rh-contracts` v0, the typed `[[applies]]` payloads in `AccessSpec`, and M2 0.2.3. CI will detect C2 (no hash moves); the declared class is C3 and is the higher of the two |
| Planes touched (ADR-0024) | A: `rh-core`, `rh-events` (`EventRef`), `rh-snapshot` (row encodings), `rh-contracts`, `rh-sys-demography`, and `rh-registry` if the loader is still there. Tool: the spec-table loader, `bin/gen_skeletons.py`, `bin/lint_forbidden.py`, `specs/`. B and C: none |
| Owned columns written | `M2.demography`, through commands: `person.alive`, `person.death_tick`, `person.death_cause`, and the life-course exit fields (a drained `ScheduleDeath`); `person.spouse` (a drained `Marry`). `rh-core`'s row insert writes no column as a system: it is a core act like compaction (SPR-011 ruling 5) |
| Reads (via `rh-contracts`) | No production reader. Test-only stub readers of M2's columns go through `rh-contracts` v0 (T-06). M7's and T1's foreign reads are checked at registration only (T-04) |
| RNG purposes added | None in production code. Test-only stub senders declare their own purposes in fixture specs |
| Golden-hash effect | **Cursor only** (amended 2026-10-17, the Architect's option A): `smoke` (1,000 years) stays byte-identical; on `solo`, `household` and `hamlet` every world table and every engine table but `rh-core/engine.meta` stays byte-identical at every rung, and `engine.meta`'s cursor moves because M2's daily instants advance it; the three are re-recorded with `bin/regen_baseline.sh` in the final `[C3]` commit, with a divergence record. In channel A only `STEP_DIGEST.state_hash` and `KEYFRAME.state_hash` differ (and the segments' `canonical_digest` as their consequence). Any other difference is a defect to fix (EVO-001 §5 as amended). If R15's neutrality cannot be met, R15's fallback needs a text change and an announcement to the director first, outside this plan |
| Fixed-point quantities | None new. `q8_8` and `q16_16` payload fields of the closed vocabulary are carried as raw integers and type-checked at `submit` |
| Determinism lints (GOV-003 §2, §6) | Must pass. New: A-50's `env!`/`option_env!` rule in `bin/lint_forbidden.py`, with planted controls (T-07). `rh-contracts` stays (A, contract) |
| Tests that gate this sprint | V-DET-01, V-DET-02, V-DET-05, V-DET-12 (unchanged), **V-DET-21**, V-INV-01, V-INV-03 (subset, stub senders), **V-INV-04** (absent owners), **V-INV-15** (stub systems; the court soak is SPR-021's and SPR-023's), V-DET-19, V-INSP-03, V-GOLD-01 on all four worlds, V-PERF-04, V-PERF-11, plus `bin/check_all.sh` |
| Docs updated in the same sprint | `bin/check_all.sh` and `python/tests/test_check_all_inventory.py` (same commit); `CLAUDE.md` checks; `docs/09` V-DET-21, V-INV-15 and V-INV-04 method lines given the test names; `05.0` §6 (`rh-contracts` v0); BCK-002 A-30, A-31 and A-50 closed; M2 0.2.3 and its changelog line |

**Rules carried from P0–SPR-013 (every task):**
- **Property tests vary world size and seed.** This sprint's floor is three sizes and eight seeds unless a task says otherwise (SPR-007's 48-row lesson).
- **A positive control fails for the right reason.** Its `check_all` line matches the expected message (`fail:<message>`), never a bare failure (REV-EVO-001 MIN-4).
- **A test input exercises the path it claims.** A probe asserts it: a command really waited across an instant; a timer was really pending when the input landed (`worlds/solo/inputs.toml`'s rule).
- **Python tests read no CI environment variable.**
- **No statistical test here.** Any test added later states its power (A-53 GAP-9).
- **CI time.** Each CI part stays at or under 12 minutes (15 hard), reported in the PR.
- **Merge `origin/main` when main moves, and re-run both parts.**
- **Every new gate is in exactly one `check_all.sh` part,** with the inventory test updated in the same commit.
- **No new third-party crate.**
- **One commit per task** (GOV-005 §4).

## Tasks

#### T-01: Delivery to the one draining binding (rule 2 parts 1–2; R2; NIT-10)
- **Deliverable:**
  - **The spec table carries the new declarations.** The table the host builds from the specs gains:
    - for each `[[applies]]` entry: its typed payload, `drained_by`, `key`, `key_field` and `outcome`;
    - for each outcome `[[commands]]` entry: its `timer` and `purpose`;
    - for each `[[reads]]` entry: its `use` and `neutral`.
  - **`AccessSpec::applies` names the draining binding** for each command kind.
  - **The engine hands an owner's inbox for a kind only to the binding the spec names:**
    - `StepCtx::take_commands` in that binding returns its inbox, whole, in key order `(tick, phase, PersonId, seq)`;
    - every other job of the owner, its timer pass included, sees none of it, and the queue is left as it was.
  - **The merge within one system** takes cancellations, then events, then timers, then commands (rule 4).
  - **The skeletons carry the drains.** `lint_odd.py --rust` and `bin/gen_skeletons.py` emit a `DRAINS` constant per spec, one `(command, Pulse, Phase)` per `[[applies]]`. Registration compares it with the code's `AccessSpec` under rule `spec`, as it compares `SCHEDULE`, and refuses a mismatch by name. The committed skeletons are regenerated.
- **Acceptance:**
  - **V-DET-21, the delivery part:** a property test in `crates/rh-core/tests/agenda_events.rs` over random small worlds of stub systems, with sizes and seeds varied. A command submitted at (t, phase, level L):
    - is in the inbox of its owner's `drained_by` binding at that binding's first run after L, and of no other job;
    - is delivered within the instant to a later phase (a `Polity` send drained at `Agenda` of the same instant);
    - waits for the next day start when sent at or after the binding's run;
    - arrives in key order;
    - gives inboxes equal at 1, 2, 4 and N threads.
  - **A probe** counts the cross-instant waits and the same-instant deliveries, and both are non-zero.
  - **Planted controls:**
    - an engine that hands the inbox to every job (EVO-001 option D1) fails with "delivered to a non-draining job";
    - a skeleton whose `DRAINS` differs from its spec is refused at registration under rule `spec`, naming the command and both bindings.
  - **Skeleton checks:** `gen_skeletons.py --check` is green on the real and the smoke specs, and the stale-skeleton control still fails for its one reason.
- **Status:** [x] done (0ea28df)

#### T-02: Event-typed slots in commands and timers (rule 2 part 6; rule 4; R5, R15; REV-SPR-017 N3)
- **Deliverable:**
  - **`rh_events::EventRef`:** `Draft(DraftRef)`, `Known(EventId)` or `None`. It is opaque: it has no integer conversion and cannot be used as a draw key.
  - **Payloads are typed by the owner's spec through the spec table.** `StepCtx::submit` refuses:
    - an integer in an `EventRef` slot (`SimError::EventSlot`, naming the system, the command and the field);
    - a field outside its type;
    - a key person that differs from the field `key_field` names (or from `Command::person` when `key_field = "person"`).
  - **The merge rewrites every `Draft` to the id it assigned in the same merge:**
    - `Command::cause`;
    - every `EventRef` payload field (`cause_event`, `source_event`);
    - a timer's new `cause` slot.
  - **`Timer` gains `cause: EventRef`.** `StepCtx::schedule` takes it, and `fire` and `resample_timer` see it. A re-sample keeps the timer's payload and cause.
  - **Encodings:**
    - **The timer row** is `… i64 payload, u8 tombstoned [, u8 1 ‖ u64 cause]`. The tail is written only when a cause is present (envelope §2; R15; `05.1` rule 2).
    - **The command row** keeps envelope §2's layout. An `EventRef` payload field is stored in `list(i64 args)` at its position: the id, or −1 for none (`05.1` v1.14).
    - **The keyframe** stores the same row bytes.
    - **`bin/gen_hash_golden.py`** gains a timer-with-cause case. The existing vectors stay unchanged.
- **Acceptance:**
  - **V-DET-21, the slot part:**
    - every `EventRef` the owner sees is the id the sender's draft got at the merge, for commands and timers, at 1, 2, 4 and N threads;
    - an integer in an `EventRef` slot is refused at `submit` and at `schedule` by name;
    - a queued command with an `EventRef` field and a pending timer with a cause survive a keyframe round trip (the V-DET-05 harness, at three world sizes).
  - **Hash neutrality (R15):**
    - `bin/gen_hash_golden.py --check` and `rh-snapshot --test golden` pass, with `rh_snapshot_hash_v1.txt` unchanged: a timer without a cause hashes as before;
    - a planted encoder that writes `u8 0` for an absent cause fails the golden test at its first differing vector. The round-3 reviewer measured that this encoding moves `agenda.timers` at 849 smoke rungs.
  - **Smoke's command rows** in `rh-core/engine.commands` are byte-identical. They are non-empty at 307 rungs of the smoke ladder.
- **Status:** [x] done (0ea28df, 20c9ad4, 3329fc2, 87809eb, 8de3783)

#### T-03: The dead-person drop, the owner's drops and the queue age (rule 2 parts 3–4; A-30, A-31; R1, R3, R11; NIT-15)
- **Deliverable:**
  - **The core drop.** In a world with a ledger (`person.id` registered), when the draining binding takes its inbox, `rh-core` drops each `key = "living"` command whose key person has no ledger row or has `person.alive = 0`.
    - It reads the ledger as it stands when the level starts (the compaction pass's own core-only read).
    - It counts each drop under `dropped{owner, kind, dead_key}`. A drop is never an error.
    - A world without a ledger drops nothing, and `rh-core` never drops a `key = "any"` command.
  - **The owner's drops:** `StepCtx::drop_command(command, reason)` and `StepCtx::drop_timer(timer, reason)`, with the reasons `dead` and `ended`. `absent_owner` comes from T-04.
  - **One diagnostic counter, `dropped{system, kind, reason}`,** printed with the run's vitals. It stays outside every plane hash and every keyframe (GOV-006 §3: observability never changes history).
  - **The core invariant `queue-no-stale-command`,** checked at every `Bookkeeping`: `t − tick < period(pulse(drained_by))`. It is fatal like every §9 invariant, with its crash artifact.
- **Acceptance:**
  - **V-INV-15 on the stub systems of `agenda_events.rs`:**
    - a `living` command for a person dead at the take is counted and never applied;
    - one for a person the owner killed earlier in the same drain is dropped by the owner (`dead`);
    - a death timer that fires for a dead person is dropped with `drop_timer` (`dead`);
    - a `key = "any"` command for a dead person is applied;
    - the ledger-less smoke world drops nothing over 1,000 years (the counter reads 0).
  - **Positive controls, each failing by name:**
    - an owner that skips one drain trips `queue-no-stale-command`, naming the owner, the kind and the age;
    - a planted `living` command for a dead person is counted, not applied;
    - a stub `apply_death` that is not idempotent trips V-INV-01's "exit fields written once".
  - **The counter changes no hash.** A run with a planted extra drop gives the same ladder (an observer-invariance check in the style of `rh-snapshot --test observer`).
- **Status:** [x] done (0ea28df, b52ebde, 3329fc2)

#### T-04: Absent owners (rule 2 part 5; REV-P1a NEW-4; R4, R12, R16; V-INV-04)
- **Deliverable:**
  - **Rule `absent-owner`.** A world that lacks the owner of a registered system's **identity** read is refused, naming the system, the column and the owner.
  - **Rate and mode reads.** A rate or mode read whose owner is absent is dropped and reported:
    - it creates no scheduling edge and no lag snapshot;
    - its column enumerates as an empty table;
    - a keyed lookup returns the spec's typed `neutral`, never the type's zero.

    The smallest API: a `SystemView` keyed lookup that `rh-contracts` accessors call.
  - **Sends.** A send to an absent owner is reported at registration. `StepCtx::submit` accepts it and discards it, counted under `dropped{sender, command, absent_owner}`.
  - **SPR-006 G5's refusal is withdrawn for sends and for rate and mode reads.** Two refusals stay:
    - a send to a registered owner that does not declare the command;
    - a read of a column that its registered owner does not write.
- **Acceptance:** V-INV-04's absent-owner rows, run on stub systems and on the real specs (`real_specs.rs`, or its successor after A-51).
  - **Real-spec worlds:**

    | World | Outcome |
    |---|---|
    | {M2} | Registers |
    | {M2, M7} | Registers. M7's rate and mode reads of M6 are dropped and reported |
    | {M2, T1} | Refused under `absent-owner`, naming T1, the M7 column and M7 |
    | {M2, M7, T1} | Registers. Its derived order puts M2 before M7 and T1 in `Vital`. Every `life_course.*` read is `same_instant`, and the instant's lag-snapshot set holds no `life_course.*` column (NEW-6). T1's `BurnHoldings` send to the absent M4 is reported, and a `submit` of it is discarded and counted |

  - **Neutral lookups.** A keyed lookup of a dropped read returns its declared neutral, for a `q16_16` value and for an enum whose neutral is not code 0.
  - **Controls, each refused by name:**
    - two `[[applies]]` owners for one command;
    - a non-owner applying a command;
    - a send to a registered owner that does not declare the command.
- **Status:** [x] done (0ea28df, 4a96bb1)

#### T-05: M2's `Day`/`Vital` drain and the ledger row insert (REV-P1a MAJ-4, NEW-6, NEW-7; `next/M2` lines 84–87; REV-SPR-015 R11)
- **Deliverable:**
  - **M2 0.2.3.** Frozen, approved by the Architect in this PR:
    - the `Day`/`Vital` `[[schedule]]` entry and its step text: take the inbox, apply each command or drop it;
    - the two `sprint` escapes removed.

    `next/M2.demography.odd.toml` follows: `lint_odd.py --next` passes. The Architect reads the `AMEND` lines before `--write-amendments-lock` is run (CLAUDE.md).
  - **`rh-sys-demography`** gains the `Day`/`Vital` binding and two drain paths:
    - **A drained `ScheduleDeath`** goes through `apply_death(victim, cause, culprit, cause_event)`. `apply_death` is idempotent: a dead or absent victim is dropped and counted. It writes `death_cause` from the command, the life-course exit, and `DEATH` with the deceased as its subject and a `caused_by` edge from `cause_event` when that is present. The culprit's `actor` role is SPR-020's.
    - **A drained `Marry`** is applied as frozen 0.2.x states it: `person.spouse` written both ways, and a dead party dropped. `apply_marry`'s full N7 (households, marriage rows) is SPR-015's.
  - **The core row insert.** A step hands `rh-core` one complete row of its own `person.*` columns. The merge of its level inserts the row at its sorted position in every `person.*` column: the inserter's columns with the given values, every other owner's with its declared `default` when it has one, else the zero of its type (`05.1` v1.14). A duplicate id is `SimError::Ledger`. The life-course append stays the step's, under `LifeCourseGuard`. The existing refusal of a step that changes a `person.*` row count stays.
- **Acceptance:**
  - **V-INV-03, subset (single death path).** Two test-only senders, each a timer under its own purpose fired in `Vital`'s timer pass:
    - one stands in for M7 (`MURDER_DUE`);
    - one stands in for T1 (`DEATH_DUE` under `outcome_delay`, with its draft as the cause).

    Each gives the same consequence set as an M2 hazard death: alive 0 and then compaction, one life-course exit, and `DEATH`. Each is applied in the same `Vital`, so it is never queued across an `Inputs` phase (V-DET-21).
  - **Same-slot deaths.** A natural death and a commanded death of one person in one slot apply once, the natural one first (EVO-001 §4).
  - **Re-sampling.** An input that scopes the victim re-samples the sender's timer (R9). A probe asserts that the timer was pending at the input.
  - **Row insert,** on random small worlds with two or three owners of `person.*` columns and varied sizes and seeds:
    - the ledger stays sorted and aligned, and passes the core invariants at `Bookkeeping`;
    - a later phase of the same instant reads the new row;
    - a duplicate id is refused;
    - a keyframe round trip holds.
  - **Lint and registration.** M2 registers against 0.2.3 under rule `spec`. `lint_odd.py` passes with no `SPR-014` escape left in a frozen spec.
- **Status:** [x] done (0ea28df, 3329fc2, 1753bed, 3a1943f (M2 0.2.3))

#### T-06: `rh-contracts` v0 for M2's columns (C2; `docs/13` §4.1; PRJ-002)
- **Deliverable:** `rh_contracts::m2`, one typed read accessor per M2 `[[owns]]` column.
  - It covers `person.*` and `life_course.*`, `life_course.birth_tick` and `life_course.sex` included (NEW-7).
  - Each accessor returns the spec's type: `PersonId`, `SimTime`, enums carrying the spec's `values` codes, or `bool`.
  - It reads through the caller's `SystemView`, so the `AccessSpec` check applies.
  - It holds no M2 logic.
  - A test compares every accessor's column, type and enum codes with M2's spec.
- **Acceptance:**
  - The drift test passes. Its planted control, a fixture spec with an enum's values reordered, fails naming the column.
  - `lint_arch` passes: `rh-contracts` is (A, contract) and depends only on plane-A crates.
  - The stub readers of T-03 and T-04 read M2's columns only through the crate: the consumer contract test that `docs/13` §4.3 requires.
- **Status:** [x] done (b38bec1)

#### T-07: BCK-002 A-50: the build environment and the entity kind of a draw (VER-SPR-011-TEST GAP-5, GAP-7)
- **Deliverable:**
  - **A `bin/lint_forbidden.py` rule** that refuses `env!` and `option_env!` in plane-A `src/`, except for `CARGO_PKG_*` (A-50's recommended option). Planted controls both ways.
  - **A check at the draw** (`StepCtx::rng`, `stream`, `chance`, and `ResampleCtx`) that the entity's kind is the kind the purpose's `[[rng]] key` declares: `person` → `Person`, `region` → `Region`, `world` → `World`, and so on. A mismatch is a `SimError` naming the system, the purpose and both kinds.
  - **Key names with no `EntityKind` yet** (`title`, `house`) are reported at registration, not refused, until their kinds exist (M7, SPR-021). This is the sprint's choice; A-50 is silent on it.
- **Acceptance:**
  - The rule passes on the workspace. Any plane-A `CARGO_MANIFEST_DIR` left after A-51 is removed or carries a counted `LINT-ALLOW`.
  - The planted `env!("HOME")` fails with the rule's name, and a `CARGO_PKG_VERSION` use passes.
  - A stub drawing a `person`-keyed purpose with a `RegionId` is refused by name.
  - All four golden ladders are unchanged: every existing draw passes the kind check.
- **Status:** [x] done (de169c6)

#### T-08: Every ladder unchanged but for the cursor; the gates and the docs (R1 and EVO-001 §5 as amended)
- **Deliverable:**
  - **`check_all.sh` gates in part `core`:**
    - the new `nextest` targets: `agenda_events` for V-DET-21 and V-INV-15, `registration` for V-INV-04, the ledger insert, and the `rh-contracts` drift test;
    - the A-50 lint and its controls.

    Part `golden` gets no new run: the existing ladders are the evidence. The inventory test is updated in the same commit.
  - **The docs of the compliance table.**
- **Acceptance:**
  - **Ladders.** The `smoke` ladder equals its baseline at 1 and N threads (PR tier), and its file under `worlds/smoke/baselines/` is unchanged. `solo`, `household` and `hamlet` are re-recorded in the final `[C3]` commit; the divergence record shows, for each, every table digest at every rung from `8f7edb4` and from the head, with `rh-core/engine.meta` the only differing table (amended 2026-10-17). The nightly monthly ladders at 1, 2, 4 and N threads × 3 equal the new baselines: `bin/golden_nightly.sh` runs on the PR head before the audit.
  - **Channel A.** On `solo`, `household` and `hamlet` only `STEP_DIGEST.state_hash` and `KEYFRAME.state_hash` differ from SPR-013's segments (the state hash holds the cursor); every other record and field is byte-identical, and each segment's `canonical_digest` differs only as their consequence (amended 2026-10-17).
  - **Parallelism floors.** SPR-013 states `--require-parallel` floors for `household` and `hamlet`. M2's daily single-job `Day`/`Vital` levels may dilute that fraction. If a floor no longer holds, the Architect restates it with the measured figure; it is never lowered silently.
  - **Performance.** V-PERF-04 and V-PERF-11 are green. The empty daily drain should allocate nothing in steady state. If a recorded count moves, the tool regenerates the baseline in a `[perf-baseline]` commit with a reason line (SPR-009 ruling 3), and the audit explains it.
  - **CI time.** Both CI parts finish in 12 minutes or less; the times are in the PR.
- **Status:** [~] in progress (a37c1d5, 96f768b, 72791f9, 9bd277e; the `[C3]` regeneration is the final commit)

## Out of scope (explicit)
- **M7 and T1 code** (SPR-021, SPR-023) and V-INV-15's court soak; V-SAGA-19 (SPR-021).
- **Births themselves,** `apply_marry`'s N7 and households (SPR-015). This sprint builds only the insert path and the minimal `Marry` drain.
- **Things SPR-020 adds:** the envelope roles (DEATH's `actor` for a culprit) and the origin on timers, commands and inputs.
- **A registration check** that an outcome command's sender binds the drain's pulse and phase (REV-EVO-001 NIT-14). The lint has it, and the engine raises `SimError::Timer` at the first fire.
- **BCK-002 items placed elsewhere:**
  - A-35 (same-instant reactions at non-pulse instants: Later);
  - A-28 (exact lags: P1b);
  - A-33 (non-material inputs counted);
  - A-36 (P2);
  - A-51 to A-53 (SPR-013, unless A-51 slipped: see "Depends on").
- **The CRN-paired ensemble** (V-DET-17, SPR-030). A C3 PR carries it once it exists (GOV-005 §6).

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | T-01, T-02, T-03, T-05, T-08, written from `05.1` §2 rules 2 and 4, ADR-0025 and EVO-001 §7 before reading the implementation:<ul><li>V-DET-21 delivery and slots, thread invariance, keyframe round trips;</li><li>V-INV-15 and its controls;</li><li>the R15 hash neutrality, with a planted `u8 0` cause tail;</li><li>all four ladders and the channel-A segments unchanged;</li><li>the row insert under random worlds</li></ul> | `VER-SPR-014-TEST` |
| AGT-003-IF | T-04, T-06, T-07, written from `05.1` rule 2 part 5 and the specs:<ul><li>V-INV-04 absent owners on stub and real specs;</li><li>the `DRAINS` comparison;</li><li>`EventSlot` refusals;</li><li>the `rh-contracts` drift test and a consumer test of their own;</li><li>the A-50 rule with their own planted variants (the rule's text can be dodged: try a `concat!` inside `env!`)</li></ul> | As above |

## Merge route

- **Class and conditions.** C3, merged by the Architect after the Tester pass and the audit (GOV-005 §6). The divergence report is "no divergence: every ladder byte-identical". No golden-world history changes, so there is nothing to announce beyond the merge summary.
- **One C3 open for merge at a time** (GOV-005 §4.1). SPR-014 and SPR-020 are both C3 and both touch `rh-core`'s timers and commands: SPR-014 adds the `cause` slot, the one-binding inbox and the event-before-timer merge; SPR-020 adds the origin tails and `ctx.under`.
- **Recommendation: SPR-014 merges first, SPR-020 second (confidence 8/10).**
  - **SPR-014 is on the critical path:** SPR-015 waits for it.
  - **SPR-014 is hash-neutral,** so the wave's only baseline move stays SPR-020's single `[C3]` rebaseline.
  - **SPR-020 is written against SPR-014's surfaces:**
    - envelope §2's timer row puts the origin tail after EVO-001's `cause` tail;
    - `EventRef` is EVO-001's;
    - `DEATH`'s `actor` comes from a drained `ScheduleDeath`;
    - `ctx.under` scopes a drain that SPR-014 makes exist.
- **What SPR-020 re-verifies after this merge.** SPR-020 merges `origin/main`, then:
  - re-runs T-02's encoding cases with both tails (remaining lengths 0, 2, 9 and 11);
  - re-runs `origin_through_hops` through the real one-binding drain;
  - re-measures its divergence against these unchanged baselines.
- **If SPR-020 is ready first and merges first,** this sprint merges `origin/main` and then:
  - re-proves R1 and R15 against SPR-020's new baselines and its regenerated hash golden (`events.held` v2);
  - keeps the `cause` tail before SPR-020's origin tail, as envelope §2 orders them;
  - leaves `rh_snapshot_hash_v1.txt` as SPR-020 regenerated it.
- **SPR-026** (C2, no hash) may merge between them or after. It conflicts only textually, in `check_all.sh` and the inventory test.

## Blockers

| # | Blocker | Status |
|---|---|---|
| 1 | SPR-013 not merged | Open |
| 2 | The row-insert sentence is not yet in `05.1` (REV-SPR-015 R11 left it "to be confirmed") | **Closed:** `05.1` v1.14 |
| 3 | The `EventRef` payload encoding in `engine.commands` (T-02's proposal) | **Closed:** `05.1` v1.14 (7/10) |
| 4 | The director's plan approval (GOV-005 §2) | **Closed:** approved 2026-10-17 (D48) |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-014` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the declared class (C3); under `worlds/*/baselines/` only `solo`, `household` and `hamlet` change, in the final `[C3]` commit, by `bin/regen_baseline.sh` (amended 2026-10-17, the Architect's option A).
- [ ] BCK-002 A-30, A-31 and A-50 are closed; M2 0.2.3 is on `main` with no `SPR-014` escape left.
