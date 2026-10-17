---
id: EVO-001
title: "Lifecycle queues for P1a: same-instant delivery, absent owners, event-typed payloads, the dead-person drop and the queue-age invariant (05.1 §2 rule 2)"
type: explanation
status: DRAFT
owner: architect
agents: [architect, coder, tester]
tags: [evolution, determinism, simulation]
related: [PRJ-002, BCK-002, SPR-006, SPR-014, SPR-017, REV-P1a, REV-SPR-017, ADR-0025, ADR-0033]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF:** `05.1` §2 rule 2 says a command's owner "receives its queue when it next runs" and stops there. SPR-014 cannot be planned on that sentence: it does not say whether a command sent at `Polity` reaches an owner stepping at `Agenda` of the same instant (it does, in the code and in M7's and T1's specs), what an owner does with a command dated for a later day, what happens to a command for someone who died in the same slot (BCK-002 A-30), how old a queued command may get (A-31), what registration does with a read or a send whose owner no P1a world registers (REV-P1a NEW-4), or how the merge treats an `EventId` carried in a payload field (REV-SPR-017 N3). This EVO proposes the exact text that closes the five gaps, as one amendment of rule 2 (with one sentence each in rule 4, §4.3 and §9). Recommendation: adopt it as written, **confidence 8/10**; the implementation is SPR-014 (C3, scheduler semantics) and the text itself moves no golden baseline. Approver: the director (C2 contract change; D36 already put NEW-4 in SPR-014's scope), on the Architect's recommendation. Six points want a ruling (§6).

# EVO-001 — Lifecycle queues for P1a

| Field | Value |
|---|---|
| Proposed by | the Architect's spec drafter (AGT-001), for SPR-014 and SPR-017 |
| Trigger | contract ambiguity (rule 2 is silent on delivery timing) and new need (NEW-4, N3, A-30, A-31) |
| Affects | `05.1` §2 rule 2 and rule 4, §4.3, §9; `docs/09` V-DET-12, V-INV-04 (amended), V-DET-21 and V-INV-15 (new); `specs/systems/lint_odd.py` (`neutral` on `[[reads]]`, this PR); `M7.dynasty` 0.3.2 and `T1.minds` 0.3.2 (this PR); SPR-006 ruling 5 (G5's refusal of a send to an unregistered owner is withdrawn); SPR-014, SPR-017, SPR-021, SPR-023 |
| Change class if accepted | C2 for the contract text and the spec field (this PR; no hash moves). C3 for SPR-014's implementation, per PRJ-002 §2: scheduler semantics, although no existing golden baseline is expected to move (§5) |
| Approver | the director (C2: a contract and two frozen specs), with the Architect's recommendation; the Architect for the V-test ids |

## 1. Problem

**What the contract says.** Rule 2 (`05.1` v1.6): commands are typed, queued by `rh-core`, keyed `(tick, phase, PersonId, seq)`, applied by their one owner, "the owner receives its queue when it next runs"; registration refuses a non-owner and a sender whose owner is not registered (SPR-006 G5). Rule 4 says the merge at the end of each level assigns `seq` and `EventId`s, and that a command's `cause` may be a `DraftRef` the merge replaces.

**What the code does** (`crates/rh-core/src/command.rs`, `engine.rs`). Every job (a timer pass or a binding) is handed the owner's whole queue when it starts (`run_level` takes it; `StepCtx::take_commands`); what the job does not take goes back (`restore`). A step's submissions reach the queues at the merge of the step's level, each with the submitting `(tick, phase)` and the next `seq`; so a command submitted at level L is in its owner's inbox from the owner's next job after L, at any later level, phase or instant. `Command::cause` is the one event-typed slot the merge rewrites; the payload is `Vec<i64>`. Registration reports a read of an unregistered owner and **refuses** a send to one (`register.rs`, rule `command`). Nothing drops a command for a dead person, and nothing bounds a queue's age (VER-SPR-006-TEST GAP-7: 101 commands in 100 days, never applied).

**What P1a needs** (PRJ-002 §2, SPR-014's row; §6):

| # | Need | Who needs it |
|---|---|---|
| (a) | A stated delivery rule. Within one instant, M7's `on_birth` and `on_death` send `Promote` and `OpinionModifier` at `Day`/`Polity`, and T1 applies them at `Day`/`Agenda` of the same instant (`M7.dynasty` schedule; `T1.minds` `apply_commands`). T1's `MURDER_ATTEMPT` sends `ScheduleDeath {tick = the next day start}` at `Agenda`, and M2's new `Day`/`Vital` drain (REV-P1a MAJ-4) applies it the next day. M7's `exit_hazards` sends `ScheduleDeath` at `Month`/`Polity` dated a day in the same month, which M2 must turn into a death timer. Immediate succession (V-SAGA-19, SAGA-13) itself needs no command delivery since REV-P1a BLK-1: every P1a death is an M2 timer at a day-start instant, fired in the `Vital` timer pass, and M7 reads the dead at `Day`/`Polity` of that instant through `same_instant` column reads | SPR-014, SPR-021, SPR-023, SPR-024 |
| (b) | A registration rule for absent owners. M7 reads `polity.exec_constraint` (M6) and `vow.*` (S7); T1 reads `vow.*`, `artifact.*` (S6, S7) and sends `BurnHoldings` to M4 (P1b). No P1a world registers M4, M6, S6 or S7. Under today's rule T1 cannot register at all (its send to M4 is refused), and a read of M6 has no column at run time (REV-P1a MAJ-2). NEW-4 and D36: one rule in `rh-core`, specs declare everything unconditionally | SPR-014, SPR-017 (the effect language's loader needs the same neutral value: REV-SPR-017 M5, round 3 m-list), SPR-021, SPR-023 |
| (c) | A rewrite rule for event-typed payload fields. M2's `ScheduleDeath` carries `cause_event: EventId or 0`; `OpinionModifier`, `ApplyThought` and `BurnHoldings` carry `source_event` or `cause_event`. A step cannot know an `EventId` before the merge, so a sender can only name its own `MURDER_ATTEMPT` by `DraftRef`; today only `Command::cause` is rewritten (REV-SPR-017 N3) | SPR-014, SPR-017, SPR-023 |
| (d) | The dead-person drop (A-30) and the queue-age invariant (A-31), both "due with the first real lifecycle command (M2, P1a)" | SPR-014 |

Both are in rule 2's subject (what a queue is and how its owner applies it), so they belong in the same text.

## 2. Proposed text

### 2.1 `05.1` §2 rule 2, replaced in full

> 2. **Lifecycle command queues.** Births, deaths, marriages, migrations, transfers of office or property, and infection-state changes are submitted as **typed commands** to queues owned by `rh-core`. Each command has one owner, declared in its `AccessSpec` (`applies`) and named by every sender's spec (`[[commands]] target`); the owner's spec declares the command's payload (`[[applies]] payload`), which every sender uses (`lint_odd.py` checks it). Registration refuses a non-owner (SPR-006 ruling 5). Every consequence of a death goes through one path: ledger removal, life-course entry, succession trigger, inheritance and the death event. This holds whichever system caused the death (M2, M5, M8 or an agenda action). The rule has six parts (SPR-014; EVO-001):
>    - **Key and delivery.** A step submits a command with `StepCtx::submit` into its own buffer; the command reaches a queue only at the merge of the step's level (rule 4: the level's systems in canonical order, each system's commands after its events), which assigns its `seq` and its key `(tick, phase, PersonId, seq)`, where `tick` and `phase` are the instant and phase of the submitting step. From the next job its owner runs after that merge, whether at a later level of the same phase, a later phase of the same instant, or a later instant, the command is in the owner's inbox, which `StepCtx::take_commands` hands over in key order: the earliest submission instant first, then phase, then `PersonId`, then merge order. So a command sent in phase *p* of instant *t* is delivered within *t* to an owner that steps in a later phase of *t* (M7's `Promote` and `OpinionModifier`, sent at `Polity`, are applied by `T1.minds` at `Agenda` of the same instant), while a command sent at or after the owner's last binding of *t*, or at an instant where no owner binding fires (§4.3: bindings run only at their pulse instants; BCK-002 A-35), waits in its queue until the owner's next step. Nothing merged at a level is visible to any job of that level, so delivery never depends on the thread count, and a queue is plane-A state (§6: `state_hash`, keyframes), so a wait is part of history.
>    - **Draining.** An owner takes its whole inbox at the start of every step and settles each command in that step: it is **applied**, or **turned into a timer** when its payload dates it later than the instant (M2's `ScheduleDeath` for a later day start becomes a death timer at that `(tick, Vital)`), or **dropped** with a diagnostic (the next part). A timer pass is not a step for this rule: `fire` may leave the inbox untouched, and an untaken command stays queued, in order. A command dated at the drain instant is applied in the drain, in that phase (M2's `Day`/`Vital` drain applies today's `ScheduleDeath` before the bindings that read M2 at `same_instant`, so the death is seen in the instant it is dated for); while the command waits for its date it is the scheduled outcome DET-13 requires, so V-DET-12's "sits in the agenda as a timer" reads "in the agenda or in a lifecycle queue". A command dated earlier than its drain instant is applied at the drain instant with its events dated there (ADR-0025: no outcome takes effect earlier than the instant it is applied in), and counted as `late` in the engine's diagnostics; P1a senders date every command at a day start at or after the owner's next pulse instant, so none is late.
>    - **The dead-person drop (BCK-002 A-30).** The key person of a command is the `PersonId` its key orders by. When an owner takes its inbox, `rh-core` drops every command whose key person has no ledger row, or whose row has `person.alive = 0`, at the start of the job, and counts it per (owner, command, sender) in the diagnostics; it is never an error. A command whose key person dies earlier in the same drain (an earlier command of the inbox killed them) is dropped by the owner under the same rule before anything else in the drain reads it. Other persons a payload names are the owner's spec's business (`Marry {a, b}` with `b` dead: M2 drops it, P1.3). Because every death takes effect through M2's one path in `Vital` (A-30's constraint; REV-P1a BLK-1: at a day-start instant), no `Polity` or `Agenda` step applies a command for someone who died in the same instant without seeing the death.
>    - **Queue age (BCK-002 A-31; §9).** At `Bookkeeping` of every instant *t*, no queued command for an owner O is older than O's allowance: `t − tick < period(p_O)`, where `tick` is the command's submission instant, `p_O` is the finest pulse among O's bindings and `period` is that pulse's length in minutes at *t* (`Day`: 1,440). An owner that drains at every step keeps it by construction; a violation means a skipped drain, or a command that reached an owner at instants where it never steps (A-35). It is a core invariant, checked with the others at `Bookkeeping` (rule 4) and fatal like every §9 invariant.
>    - **Absent owners (REV-P1a NEW-4; D36).** A spec declares every read and every send unconditionally, and `access()` never depends on which systems a world registers. At registration `rh-core` **drops** each declared read whose owner is not registered and each declared send whose owner is not registered, and **reports** each one (a registration report, not a violation; the refusal of such a send, SPR-006 G5, is withdrawn). A dropped read creates no scheduling edge and no lag snapshot, and the step view answers every read of its column, for any key, with the coupling's **neutral value**: the `neutral` of the reader's `[[reads]]` entry, typed by the column's type (the Registry effect language's loader reads the same field, SPR-017). A dropped send is accepted by `StepCtx::submit` and discarded, never queued, counted per (sender, command) in the diagnostics; the sender's code never asks whether the owner exists. A send to a registered owner that does not declare the command, and a read of a column its registered owner does not write, stay refused.
>    - **Event-typed payload fields (REV-SPR-017 N3).** An `EventId` exists only after a merge (rule 4), so a step names an event either by a `DraftRef` for one of its own drafts or by an `EventId` it read from state. Every event-typed slot of a command, `Command::cause` and every payload field the owner's `[[applies]] payload` types `EventId` (`cause_event`, `source_event`), is an opaque `EventRef`: `Draft(DraftRef)`, `Known(EventId)` or `None`. The merge replaces each `Draft` by the id it assigns in the same merge, so the owner only ever sees ids. A `DraftRef` cannot be read as an integer, computed with or used as a draw key, and `submit` refuses an integer in an event-typed slot (the slot types come from the owner's spec through the spec table at registration).

### 2.2 `05.1` §2 rule 4, the **Threads** bullet, last sentence amended

> … A step cannot know an `EventId` before the merge, so it names an event it emitted earlier in the same step by a `DraftRef` (an engine API: `rh_events::DraftRef`, returned by `StepCtx::emit`); as a causal edge of a later draft, as a command's cause, or in any event-typed payload field of a command (rule 2), the merge replaces it with that event's id (Architect ruling on SPR-006 D2, D3; EVO-001).

### 2.3 `05.1` §4.3, the **one slot fires together** bullet, last sentence amended

> … Conflicts such as "dead and acted" are settled when the lifecycle queue is applied: a command for a person who is dead when its owner takes its inbox, or who died earlier in the same drain, is dropped with a diagnostic (§2 rule 2, the dead-person drop; BCK-002 A-30).

### 2.4 `05.1` §9, one bullet added after **Life-course consistency**

> - **Queue age.** No lifecycle command waits in its queue longer than its owner's finest pulse period (§2 rule 2).

### 2.5 `05.1` header, one version line

> *v1.10 (2026-10-17): EVO-001 (SPR-014): §2 rule 2 states delivery within an instant, draining, the dead-person drop (A-30), the queue-age invariant (A-31), the absent-owner registration rule (REV-P1a NEW-4) and the rewrite of event-typed payload fields (REV-SPR-017 N3); rule 4, §4.3 and §9 follow.*

### 2.6 The spec side (this PR)

`[[reads]]` gains a required `neutral` field for every read whose owner is another system (the value the step view returns for a dropped read, and the value the effect language's loader uses: REV-SPR-017 round 3). It is typed by the column's type as the owner's spec declares it (`[[owns]] type`), or, for an owner with no spec, as the Registry's `[era_state]` entry declares it: a TOML integer for an integer type, a boolean for `bool`, a value name for an `enum`, a TOML number for `q8_8`, `q16_16` and `f32_field` (the exact decimal of the literal, as `[[params]] default` and SPR-011 ruling 9). A missing or mistyped `neutral` is an error on a frozen spec and a warning on a draft, as the other coupling details (ADR-0033 as amended). `M7.dynasty` and `T1.minds` carry their neutral values at 0.3.2.

## 3. How the text answers the brief

- **Ordering within the owner:** key order, led by the submission instant. Two commands sent in the same instant arrive in phase order, then by `PersonId`, then by merge order (the canonical level order), whatever the thread count.
- **An owner whose phase has passed:** the command waits in its queue (plane-A state) until the owner's next step; in P1a every sender submits at a day-start instant (every P1a binding is `Year`, `Month` or `Day`, all of which fire at day starts, and every timer fires at a day start since BLK-1) and every owner binds `Day`, so a command waits at most one day. Minute-level senders are A-35.
- **The timer pass:** a command merged in the timer pass of phase *p* is in the inbox for the binding pass of *p*; the timer pass itself does not drain. The outcomes a drain creates for a later day are timers, so they fire in that day's timer pass like every other death (`05.1` §4.3; BLK-1).
- **The level merge:** unchanged. The merge is the only writer of the queues and assigns `seq`; a command sees its cause's `EventId` because a system's events merge before its commands; an event-typed payload field is rewritten at the same point.
- **V-SAGA-19:** satisfied without delivery, by the `Vital` timer pass and M7's `same_instant` reads at `Polity` (BLK-1). What delivery adds in the death instant is the `PASSED_OVER` opinion and a newborn's promotion reaching T1 at `Agenda` of the same day.
- **The frozen specs' wording:** M7's and T1's `[[couplings]]` say a command is "applied at the next tick boundary". Under this rule that reads "at the owner's next step", which for a `Polity` send to T1 is `Agenda` of the same instant and for an `Agenda` send to M7 or M2 is the next day start. No mechanics change; the phrase is corrected in the specs' next version, not here.

## 4. Options

| Topic | Option | Consequence | Cost | Confidence |
|---|---|---|---|---|
| (a) delivery | **A1 (recommended):** in the inbox from the owner's next job after the merge level; the owner drains at every step | What the code does; M7 → T1 within the instant; stated, testable, thread-invariant | A rule to write, no engine change beyond the drain contract | 8/10 |
| | A2: delivered only at the owner's next *instant* (SPR-006's "when it next runs", read strictly) | A newborn is promoted and a passed-over heir's opinion lands a day late; adds a hold on every queue for no gain | A hold the engine does not have today | 3/10 |
| | A3: delivery runs the owner's handler at the delivering instant even off-pulse (A-35) | Solves minute-level senders too | A new scheduling mechanism, C3, not needed before S5 F2 (Later) | 2/10 for P1a |
| draining | **D1 (recommended):** every step drains; `fire` need not | Simple allowance; a timer pass stays an outcome, not a drain | None | 7/10 |
| | D2: the timer pass drains too | A command can be applied at a non-pulse instant where the owner never steps | Couples timers to queues; breaks "a timer is one scheduled outcome" | 3/10 |
| late commands | **L1 (recommended):** applied at the drain instant, counted `late` | Total; no back-dating | A diagnostic the owner must emit | 6/10 |
| | L2: a `SimError` | Loud, but a later sender's clock error kills a 1,000-year run | | 4/10 |
| (b) absent owners | **B1 (recommended):** drop reads and sends, report, neutral from `[[reads]]` | T1 registers in P1a with its M4 send; one rule, in `rh-core`; specs stay unconditional (NEW-4) | A `neutral` field on 43 reads (this PR); `StepCtx::submit` learns to discard | 9/10 |
| | B2: drop reads, keep refusing sends (today) | T1 cannot register before M4 (P1b) | | 1/10 |
| | B3: `access()` declares only what the world registers (REV-P1a MAJ-2's first fix) | Every system needs the registered set at construction; rejected by NEW-4 | | 2/10 |
| neutral source | **N1 (recommended):** `[[reads]] neutral`, typed by the owner's `[[owns]]` (else `[era_state]`) | Machine-readable, one value per read, checked by the lint, read by `rh-core` and the effect language | Two frozen specs bump to 0.3.2 | 8/10 |
| | N2: the free-text `neutral` of `[[couplings]]` | Already there for rate reads | Prose; only rate reads have couplings; not machine-readable | 2/10 |
| (c) payload rewrite | **C1 (recommended):** opaque `EventRef` in every event-typed slot, typed from the owner's spec | N3 closed; one rule for `cause` and payloads | A typed payload slot in `Command` (SPR-014), slot types through the spec table | 8/10 |
| | C2: `Command::cause` is the only event slot; payloads carry no `EventId` | Simpler engine | M2's and T1's frozen payloads carry `cause_event` and `source_event`; a command may need two refs | 3/10 |
| | C3: `i64` with the sender responsible | Nothing to build | N3's bug by design | 0/10 |
| (d) dead-person drop | **P1 (recommended):** `rh-core` at inbox take; the owner within the drain | Core sees the ledger it already compacts; the owner sees its own kills | Two places | 7/10 |
| | P2: the owner only | One place | Every owner repeats the ledger check | 4/10 |
| | P3: `rh-core` only | One place | Core cannot see a death applied inside the drain | 3/10 |
| queue age | **Q1 (recommended):** allowance = the owner's finest bound pulse, from `[[schedule]]` | No new spec field; exact with D1 | A command for an owner that drains only at a coarser binding violates it (none in P1a) | 7/10 |
| | Q2: a declared `drained_by` on `[[applies]]` | Exact in every case | A frozen-spec change for M7 and T1 (they both apply commands) | 5/10 |
| | Q3: a length bound | Catches GAP-7's symptom | Not the cause; a busy court trips it | 2/10 |

## 5. Hash effect and class

This PR changes text, a lint and two specs: no golden hash moves. SPR-014's implementation is C3 by PRJ-002 (scheduler semantics), but no **existing** baseline is expected to move: `smoke` sends no command and its systems register every owner; `solo`, `household` and `hamlet` register M2 alone, send nothing and read nothing foreign, and M2's new `Day`/`Vital` drain binding makes the engine run an instant at every day start that changes no column, draws nothing and emits nothing, so month-end hashes are equal (to be shown by SPR-014's ladder comparison, not assumed; a difference makes the sprint's `[C3]` baseline regeneration the record). The first world whose history depends on delivery order is the dynastic fixture (SPR-021, SPR-030), whose baseline is recorded after this text, so any later change to delivery order is a C3 change to that world. The withdrawn G5 refusal changes registration reports only.

## 6. For the director and the Architect

| # | Question | Recommendation | Confidence |
|---|---|---|---|
| 1 | Does the timer pass drain the inbox? | No (D1): steps only; `fire` may ignore it | 7/10 |
| 2 | A command dated earlier than its drain instant | Applied at the drain instant, counted `late` (L1); never an error | 6/10 |
| 3 | The queue-age allowance | The owner's finest bound pulse, derived from `[[schedule]]` (Q1); no new spec field | 7/10 |
| 4 | SPR-006 G5 ("a sender whose owner is unregistered is refused") | Withdraw it: dropped and reported instead (B1). Record the amendment against SPR-006's rulings in `docs/decisions-log.md` | 9/10 |
| 5 | `docs/09`: amend V-DET-12's gate ("in the agenda or a lifecycle queue") and V-INV-04's method (a dropped read served the neutral, a dropped send discarded and reported, a send to a registered non-owner refused); add V-DET-21 (delivery order and the payload rewrite, thread-invariant) and V-INV-15 (queue age and the dead-person drop) | Add both ids with SPR-014's plan; this EVO proposes the rows (§7) | 7/10 |
| 6 | Spec versions: M7 and T1 to 0.3.2 for the `neutral` field; REV-SPR-017 r4-4 also planned "M7 amended to 16 (0.3.2)" for `holders_of(title, 16)` | If r4-4 lands first, this becomes 0.3.3; the field is the same either way. The smoke world's seven foreign reads carry `neutral` without a version bump (dummy specs outside the Registry, SPR-008 ruling 2; no code reads the field) | 7/10 |

## 7. Tests that gate it (`docs/09`)

| ID | Status | What |
|---|---|---|
| V-DET-12 | existing, gate wording amended | no back-dating: a drained command dated today is applied today; a later one becomes a timer; nothing is applied earlier than its date |
| V-INV-03 | existing (P1a variant, D34) | the single death path: a `ScheduleDeath` from M7 and from T1 produces the same consequence set as an M2 hazard death |
| V-INV-04 | existing, method amended | registration: a read of an absent owner is dropped and reported and the view serves its `neutral`; a send to an absent owner is dropped, reported and discarded at `submit`; a send to a registered owner that does not apply it, and a non-owner applying, are refused |
| V-SAGA-19 | existing | immediate succession at the death instant (BLK-1), with the passed-over opinion reaching T1 at `Agenda` of the same instant |
| V-SF-22 | existing | the isolation harness "holds other inputs at their neutral values": the absent-owner rule is that mechanism |
| **V-DET-21** | proposed: "Lifecycle delivery order (DET-13, SIM-4)" | property test over random small worlds: a command submitted at (t, p, level L) is in its owner's inbox at the owner's first step after L and not before, in key order; equal at 1, 2, 4 and N threads; every event-typed slot the owner sees is the id the sender's draft got, and an integer in such a slot is refused; a command that waits across instants survives a keyframe round-trip (with V-DET-05) |
| **V-INV-15** | proposed: "Queue age and the dead-person drop (A-30, A-31)" | a soak on a fixture court with M2, M7 and T1: at every `Bookkeeping`, no command older than its owner's allowance; a command for a person dead at the start of the drain is dropped and counted; one for a person killed earlier in the same drain is dropped; positive controls: an owner that skips a drain trips the invariant, and a planted command for a dead person is counted, not applied |

The `neutral` field has its own gate in this PR: `lint_odd.py` refuses a frozen spec with a foreign read and no `neutral`, or with one of the wrong type, and accepts one with the right type (`tests/lint_fixtures/odd_neutral/`, in `bin/check_all.sh` part `core`).

## 8. Recommendation (confidence 8/10)

Adopt §2 as written. The delivery rule is what the engine already does, made exact and testable; the drain, drop and age rules are the three P0 carry-forwards the plan put in SPR-014; the absent-owner rule is D36's; the payload rewrite closes N3 with one type. The alternatives that avoid a frozen-spec change (N2, Q2 avoided) cost precision the effect language needs, and the ones that avoid engine work (C3, B2) are the bugs the reviews found.

## Decision

<recorded by the approver; the director's rulings also go in `docs/decisions-log.md`; the Architect records the V-test ids and amends `05.1` to v1.10 on acceptance>
