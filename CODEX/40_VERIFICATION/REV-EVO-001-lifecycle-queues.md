---
id: REV-EVO-001
title: "Independent review of EVO-001, lifecycle queues and absent owners (PR #49)"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [EVO-001, PRJ-002, SPR-014, BCK-002, REV-P1a, REV-SPR-017]
created: 2026-10-17
updated: 2026-10-17
version: 1.0.0
---

> **BLUF: CLOSED, approved.** PR #49 took four rounds:
> - **Round 1** (`6bc202e`): 1 blocker, 5 majors.
> - **Round 2** (`b1c6890`): 1 major. R6 was ruled for shape (b), strengthened: an outcome-carrying command is submitted only by its sender's timer at the outcome's instant. DET-13 and ADR-0025 stand as written, and D39 is withdrawn.
> - **Round 3** (`1d08b06`): 1 major (MAJ-7, wording). The reviewer traced both murders end to end and found no day lost.
> - **Round 4** (`9be14c4`): verified by the Architect. The smoke ladder is untouched: no Rust change beyond a test comment, and both SPR-014 changes are now hash-neutral.
>
> `05.1` is amended as v1.12. Every review ran in a session on a different model from the drafter's.

# REV-EVO-001: independent review of EVO-001

## 0. Architect rulings, round 1 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R1 | BLK-1 | Accepted. The dead-person drop applies only in a world with a ledger (`person.id` registered), and only to commands whose `[[applies]]` entry declares that its key person must be living (an explicit opt-in; the default is no drop). Smoke's death-`Settle` keeps flowing. SPR-014's acceptance gains "the smoke ladder is unchanged". Correct EVO §5 and the PRJ-002 line that says P1.0 sends no lifecycle command (amended 2026-10-17, SPR-014: the engine cursor moves on worlds whose instant set grows; world tables stay neutral) | 8/10 |
| R2 | MAJ-1 | Accepted. A command is taken only by the binding its owner's spec names: a machine-readable `drained_by = "<Pulse>/<Phase>"` on `[[applies]]` (one binding per command kind). Commands no step takes stay queued, in key order. The queue-age allowance is that binding's pulse. M7 and T1 already change version in this PR, so the frozen-spec cost is paid. The smoke specs carry `drained_by` matching what their code does today (no ladder change). Rewrite §3's "no mechanics change" accordingly. This supersedes the drafter's question (a): a timer pass is not a binding and drains nothing | 8/10 |
| R3 | MAJ-2 | Accepted. The drop text states: (1) which `person.alive` it reads, meaning the ledger as the draining step sees it, given that compaction runs at the start of `Bookkeeping`; (2) that worlds without a ledger drop nothing; (3) that commands keyed by the dead, such as estate settlement, simply do not opt in; (4) that a death timer whose person is already dead is dropped by M2, with an invariant; (5) the owner's own drops, as a named diagnostic counter per (system, command kind, reason) | 8/10 |
| R4 | MAJ-3 | Accepted, as the reviewer recommends. (1) `neutral` is required only on reads with `use = rate` or `mode`, checked against the coupling's prose. (2) An identity read declares its owner required, and registration refuses a world without that owner, by name. (3) A dropped read enumerates as an empty table, and keyed lookups return the neutral. (4) Enum columns carry machine-readable codes in the owner's `[[owns]]`. (5) Strike the V-SF-22 sentence, or say that a stub owner does that job. SPR-006 G5's refusal is withdrawn for sends and for rate and mode reads only (the drafter's question (d)) | 8/10 |
| R5 | MAJ-4 | Accepted. `[[applies]]` payloads become machine-typed in a closed vocabulary (`PersonId`, `EventRef`, `SimTime`, the id types, enum, `q8_8`, `q16_16`, `u64`, `i64`, `bool`), checked by `lint_odd.py`, in every spec with `[[applies]]` (M2, M7, T1, the smoke overlay). The rewrite applies to `EventRef` slots. M2 gets a no-mechanics version bump for its payload types, and the M2 F1 births draft (`specs/systems/next/`, PR #50) carries the same types when it merges `main` | 7/10 |
| R6 | MAJ-5 | Held for round 2. The drafter writes both options exactly. (a) A queued dated command is the scheduled outcome, with ADR-0025 item 3 extended to queued commands in scope; this amends DET-13 and ADR-0025, so it is the director's. (b) Outcomes dated later are timers owned by the system that sampled them; a command carries no future date, and the sender's timer submits it when it fires; DET-13 and ADR-0025 stand as written. For each option give: the instant at which a murder's death applies (V-SAGA-19), who re-draws on a material input, M2's frozen `ScheduleDeath.tick`, and the smoke effect. Recommend one. If (b) holds with no timing loss, the Architect rules it; otherwise it goes to the director as a D-item | — |
| R7 | MIN-1 to MIN-6, nits | Accepted, with the reviewer's fixes. MIN-2: a mistyped `neutral` is always an error, so the text changes, not the lint. MIN-6: whichever of PR #41 and PR #49 merges second takes M7 0.3.3 and merges `main` first | 8/10 |
| R8 | Drafter's (b), (c), (e), (f) | (b) A late command is counted in a named diagnostic, not refused (6/10; may be moot under R6 (b)). (c) As R2. (e) The `docs/09` rows are added in this PR, adjusted to these rulings. (f) The `05.1` §2 amendment (v1.10) is applied in this PR: merging is acceptance | 7/10 |

## 1. The review (round 1, independent reviewer)

Reviewer: independent, adversarial. Copy reviewed: `$R/src` (a `git archive` of `6bc202e`). Nothing in `/home/user/Rehoboam` was modified.

## BLUF

**Not approvable as written: 1 blocker, 5 majors, 6 minors, 4 nits.** The lint, fixtures and gates work: every planted control fails or passes for its own reason, and the inventory test passes. The rule-2 text has problems.

- **BLK-1.** The hash-effect section is wrong, and the dead-person drop as written moves the smoke golden baseline. The smoke world sends commands keyed by the person who just died.
- **MAJ-1.** The mandatory "drain at every step" rule changes when frozen M7 and T1 apply commands. The proposal says no mechanics change.
- **MAJ-2.** The dead-person drop cannot be built without guessing.
- **MAJ-3.** The absent-owner neutrals for identity reads produce impossible states (everyone dead with cause `none`; a `minor` sovereign title).
- **MAJ-4.** The payload rewrite (N3) needs payload types that frozen T1 does not declare.
- **MAJ-5.** Counting a queued command as the scheduled outcome amends requirement DET-13 and ADR-0025, which makes it the director's decision. The EVO does not present it as one.

The determinism core holds:
- the delivery order is total and does not depend on thread count;
- delivery is bounded within an instant;
- no plane-A code reads plane B or C.

Recommendation: return the EVO to the Architect for one revision.
- Drain only at the binding the spec names for it (Q2, carried in the same 0.3.x bump).
- Scope the dead-person drop: only worlds with a ledger, and only commands whose spec requires a living key.
- Require a `neutral` only on rate and mode reads. Mark identity reads of M2 as needing a registered owner.
- Type the payloads.
- Put the DET-13 question to the director explicitly.

Land PR #41 first and renumber M7 to 0.3.3 here. Confidence 8/10.

## Findings

### BLK-1. §5's hash claim is false: the smoke world sends lifecycle commands, and the dead-person drop as written changes its history

- **Where:** `CODEX/60_EVOLUTION/EVO-001_LifecycleQueuesP1a.md:113` says "`smoke` sends no command and its systems register every owner". Also the BLUF (`:15`, "moves no golden baseline") and §2.1 (`:51`, the dead-person drop).
- **Evidence:**
  - Smoke has four command flows, all owned by X4 or X7:
    - `X3.smoke_vital` sends `Settle` from `fire` (`crates/rh-sys-smoke/src/vital.rs:138` writes `alive[r] = 0`, then `:146-149` submits `Settle { person: person(r) }`, keyed by the person who just died);
    - `X5.smoke_polity` sends `Settle` (`polity.rs:57-59`);
    - `X7.smoke_market` sends `Settle` (`market.rs:97-99`);
    - `X4.smoke_estate` sends `Audit` to X7 (`estate.rs:59`).
  - X4 applies `Settle` with draws keyed by `(person, seq)` at the applying instant (`estate.rs:47`), so any change to which commands survive, or when they are applied, changes `estate.heirs`, `estate.order` and the event log.
  - EVO §2.1 drops "every command whose key person has no ledger row, or whose row has `person.alive = 0`". Under SPR-011 ruling 5 "the smoke world has no `person.id`", so no smoke person has a ledger row. Read literally, the rule drops every smoke command.
  - Read as "check `person.alive`", it still drops every death-`Settle`: X3 has written `alive = 0` before X4 drains.
  - Either way the 1,000-year smoke ladder (`worlds/smoke/baselines/smoke_yearly_1000y.ladder`) moves. That is a `[C3]` baseline regeneration the EVO says will not happen.
  - The plan has the same error: `PRJ-002:193` says "P1.0 sends no lifecycle command".
- **Fix:**
  - Correct §5.
  - State that the drop applies only when the world has a ledger (`person.id` registered), and only to commands whose `[[applies]]` entry requires a living key person (see MAJ-2).
  - Add "the smoke ladder is unchanged" to SPR-014's acceptance criteria.
- **Confidence:** 9/10.

### MAJ-1. "Drain at every step" changes when frozen M7 and T1 apply commands; §3's "No mechanics change" does not hold

- **Where:** EVO `:50` ("An owner takes its whole inbox at the start of every step and settles each command in that step: applied, turned into a timer, or dropped"), D1 (`:92`), Q1 (`:107`, "exact with D1") and §3's last bullet (`:83`).
- **Evidence:**
  - The frozen specs apply commands in one named binding:
    - M7 applies T1's `PressClaim`, `PetitionSeat`, `RenounceClaim` and `Resign` in `Day/Agenda` only (`specs/systems/M7.dynasty.odd.toml:510-512`);
    - T1 applies `Promote`, `Demote`, `OpinionModifier` and the others in `Day/Agenda` only (`T1.minds.odd.toml:700-702`). Its `Day/Vital` and `Month/Agenda` steps apply nothing (`:695-697`, `:705-707`).
  - Under D1 the first step after the merge must settle the command, and "settle" has no "defer" outcome:
    - **T1 → M7.** A `PressClaim` sent by T1 at `Agenda` d is applied in M7's `Day/Polity` step of d+1, or its `Year`/`Month` `Polity` step at a period start. That is before `on_death`'s arbitration, which counts "claimants with pressed active claims". Under the spec it is applied at `Agenda` d+1, after that arbitration.
    - **M7 → T1 at a month start.** `real_specs.rs` asserts the `Agenda` order `["T1.minds", "M7.dynasty", "T1.minds"]`: T1's Month binding comes first. So M7's `Promote` from `Polity` is applied in T1's `Month/Agenda` step, before `stress_month`, rather than in `Day/Agenda`.
    - **T1 self-sends.** `OpinionModifier` sent from `Agenda` (DESPAIR_PACT) would be applied in T1's `Day/Vital` grief step.
  - Each of these gives a different history.
  - Smoke also breaks D1:
    - X7's `Day/Environment` step never takes its `Audit`s (`market.rs:83` takes them only in `trade`, the Activity binding);
    - X4's `gossip` step at `Contagion` takes `&StepCtx`, so it cannot drain the `Settle`s X7 sends at the same instant's `Activity` (`estate.rs:70`).
  - Enforcing D1 would move smoke too: X4's draws are keyed by the instant.
- **Fix:** Drain at the binding the owner's spec names, and nowhere else. Use Q2's `drained_by` on `[[applies]]`, or equivalently a `[[schedule]] applies = true` flag, and derive the queue-age allowance from that binding's pulse.
  - The EVO rejected Q2 because it is "a frozen-spec change for M7 and T1", but this PR already bumps both specs, so the cost has already been paid.
  - Until then, state that a step that leaves commands untaken keeps them queued, in order, and that only the age invariant bounds the wait.
  - Rewrite §3's last bullet.
- **Confidence:** 8/10.

### MAJ-2. The dead-person drop cannot be built without guessing

- **Where:** EVO `:51` and §2.3 (`:62`).
- **Evidence:**
  1. **Which value of `person.alive`.** The core reads M2's column "at the start of the job", but the rule gives no lag and creates no scheduling edge. `run_level` moves every job's owned columns out of `tables` before dispatch (`crates/rh-core/src/engine.rs:1038-1050`). When an owner shares a level with M2, `person.alive` is absent from `tables` at that moment, so a naive lookup finds "no ledger row". Whether the drop sees M2's writes from this phase depends on level placement, which the EVO never states.
  2. **Worlds without a ledger.** Smoke has none (BLK-1), and neither do the rh-core stub worlds in `tests/agenda_events.rs`, whose commands use hashed `PersonId`s.
  3. **Commands about the dead.**
     - Smoke's death `Settle` is keyed by the person who died (BLK-1).
     - M7's `TransferHoldings` ("property inheritance through the death path", `M7:` `[[commands]]`, P1b) has no payload yet. Keying it by the deceased would silently drop every inheritance: M4 drains after `Bookkeeping` has compacted the deceased's row.
     - The EVO says only that "other persons a payload names are the owner's spec's business". It never says the key person of a command must be living.
  4. **Death timers that outlive their victim.** The drain turns a later-dated `ScheduleDeath` into an M2 timer. A sovereign can then hold M2's natural-death timer for day 6 (drawn at `Month/Vital`) and M7's murder timer for day 10 (drawn at `Month/Polity`, `k ∈ [2, last]`). After the day-6 death the row is compacted, and the day-10 timer fires for nobody. The drop covers only commands; neither the EVO nor M2's `apply_death` says what happens.
  5. **"Dropped by the owner … before anything else in the drain reads it".** How the owner counts it "in the diagnostics" is not stated: `StepCtx` has no diagnostics API.
- **Fix:** In the rule text:
  - The drop reads `person.alive` as it stands at the start of the level, before any columns move.
  - It applies only when `person.id` is registered.
  - A command's key person must be living; an estate command is keyed by the heir. Alternatively, `[[applies]]` declares `key = "living" | "any"`.
  - `apply_death` cancels the victim's other death timers, or a death timer firing for a dead victim is dropped and counted.
  - Name the `StepCtx` call by which an owner reports its own drops.
- **Confidence:** 8/10.

### MAJ-3. The `neutral` values for identity reads describe impossible states, and what the view serves for a dropped read is not specified

- **Where:**
  - `M7.dynasty.odd.toml:387` (`person.alive = false`), `:393` (`death_cause = "none"`), `:399` (`death_tick = 0`), `:405` (`birth_tick = 0`), `:411` and `:453` (`sex = "male"`);
  - `T1.minds.odd.toml:520-538` (the same M2 values), `:598` (`title.holder = 0`), `:604` (`title.liege = 0`), `:610` (`title.tier = "minor"`), `:622` (`office.holder = 0`);
  - EVO `:53` and `:134`.
- **Evidence:**
  - **Everyone looks dead, in a state M2 forbids.** M2 defines `death_tick` as "i64::MAX while alive" and `death_cause` as "none while alive". INV-M2-05 says "apply_death never writes death_cause = none" (`M2.demography.odd.toml:366`). The neutral triple `(alive=false, death_cause=none, death_tick=0)` is a death at the epoch with no cause: a state M2's own invariant rules out. With `birth_tick = 0`, every person is also born at the epoch.
  - **`sex = "male"` is not neutral.** It biases every male-preference law.
  - **An impossible title.** In T1's M7 reads, `title.liege = 0` means "sovereign" while `title.tier = "minor"`, which cannot happen for a sovereign. `title.holder = 0` and `office.holder = 0` mean every seat is vacant or in interregnum, which is exactly what T1's `seek_office` acts on.
  - **The view is not specified.**
    - `SystemView::read` returns the whole column as `&[i64]` (`crates/rh-core/src/world.rs:186`). "Answers every read of its column, for any key, with the neutral" does not say how long that slice is: are there no rows, or how many?
    - Enum neutrals need a name-to-code mapping. `vow.status` and `claim.status` declare names with no codes.
  - **V-SF-22 is not this rule.** The EVO says "V-SF-22 … the absent-owner rule is that mechanism" (`:134`), but the specs' own `[isolation]` sections script these inputs instead: T1 has "genealogy and seats scripted, M7 and S7 stubbed" and forces `person.alive` from `historical:court_deaths`; M7 has "M2 live".
  - Only the rate reads' neutrals carry meaning. M7's `exec_constraint = 0.0`, T1's `vow.conflict = 0.0` and `rumour_reach = 0` match the prose `neutral` of their `[[couplings]]`.
- **Fix:**
  - A dropped read enumerates as an empty table, and keyed lookups return the neutral.
  - Require `neutral` only for `use = rate | mode` reads, checked against the coupling's prose.
  - Identity reads of the ledger and of M7's tables declare the owner required, so registration refuses a world without it rather than serving invented people.
  - Give enum columns machine-readable codes.
  - Strike the V-SF-22 sentence, or say a stub owner does that job.
- **Confidence:** 8/10.

### MAJ-4. The N3 payload rewrite relies on payload types that frozen T1 does not declare

- **Where:** EVO `:54` (the slot types "come from the owner's spec through the spec table at registration"), C1 (`:101`).
- **Evidence:**
  - T1's `[[applies]]` payloads are untyped: `OpinionModifier` is `"{ holder, about, kind, magnitude, source_event }"`, and `ApplyThought` is likewise untyped.
  - M2's payloads are typed in prose (`"cause_event: EventId or 0"`, `"tick: SimTime (a day start)"`).
  - `lint_odd.py:116` (`payload_fields`) parses names only.
  - `rh_registry` `SpecTable` and `DeclaredCommand` carry no payload, and `Command::args` is `Vec<i64>` (`crates/rh-core/src/command.rs:38`).
  - So nothing machine-readable says `source_event` is event-typed. Building C1 needs a payload type grammar, a lint check for it, and a further frozen-T1 change (and M4's `BurnHoldings` when M4 gets a spec). None of these is in the EVO's Affects row or its cost column.
- **Fix:** Type the `[[applies]]` payloads in the same 0.3.x bump, with a closed type vocabulary (`PersonId`, `EventId`, `SimTime`, enum, `q8_8`, …). Make `lint_odd.py` check it, and list the T1 change.
- **Confidence:** 8/10.

### MAJ-5 (the director's). Treating a queued command as the scheduled outcome amends DET-13 and ADR-0025; the EVO does not present it as a requirement change

- **Where:** EVO `:50` ("while the command waits for its date it is the scheduled outcome DET-13 requires, so V-DET-12's 'sits in the agenda as a timer' reads 'in the agenda or in a lifecycle queue'"), and §6 Q5.
- **Evidence:**
  - `docs/03_requirements.md:75`, DET-13: "An outcome sampled for a later instant (death, …, marriage) shall be a plane-A **timer**."
  - ADR-0025 item 1 says the same ("Each one is inserted as a timer"). Item 3 re-samples "pending timers inside the scope" on a material input.
  - T1's successful `MURDER_ATTEMPT` at `Agenda` d is a death sampled for d+1. Under the EVO it is never a timer: it waits in M2's queue and is applied in M2's drain at d+1.
  - `Engine::apply_inputs` re-samples only `self.agenda.pending_for(entity)` (`engine.rs:936`). A scope-only re-sample input on the victim at `Inputs` d+1 therefore cannot touch the queued death, although it would re-sample the same death if it were a timer. P1a's golden worlds do run such inputs (SPR-011 ruling 15).
  - Once M2 converts a murder into its own timer, M2's `resample_timer` would re-draw it, but M2 did not draw it.
- **Fix:**
  - List DET-13 and ADR-0025 under Affects and ask the director a separate question with a recommendation. Either accept queued outcomes and extend ADR-0025 item 3 to queued commands in scope, or have senders schedule a timer that submits at fire time.
  - State who re-draws a command-born timer.
- **Confidence:** 8/10.

### MIN-1. "Every P1a death is an M2 timer fired in the Vital timer pass" contradicts the drain path

- **Where:** EVO `:37` (§1 (a)) and BCK-002 A-30 (`BCK-002:50`, "every death … takes effect as an M2 death timer in `Vital`"). Also T1's `Day/Vital` `when` ("where M2's death timers fired in this instant's Vital timer pass").
- **Evidence:** Under `:50`, every T1 murder (`tick` is the next day start, which is the drain instant) and every M7 exit dated day 2 is applied in M2's `Day/Vital` binding step, after the timer pass. M2's own text already says so (`M2.demography.odd.toml:191-193`). The `M2 → T1` edge in `Vital` (asserted in `real_specs.rs`, `order(Vital) == [M2, T1]`) makes these deaths visible to grief. The wording still points a Developer at the timer pass.
- **Fix:** Restate as "every death takes effect through M2's `apply_death` in `Vital`, at a day start: a timer fire or the drain", in §1, in A-30, and in T1's next version.
- **Confidence:** 8/10.

### MIN-2. The lint and the proposed text disagree on a mistyped `neutral` in a draft spec

- **Where:** EVO §2.6 (`:74`) says "A missing or mistyped `neutral` is an error on a frozen spec and a warning on a draft". `lint_odd.py` (the `errs.append` for `neutral_problem`) makes a mistyped neutral an error always. Its self-check asserts this: the probe `("draft", … "must be a TOML integer …", True)`.
- **Fix:** Align the text with the lint, and record the stricter rule as a deliberate departure from ADR-0033's warning-on-draft. Or align the lint with the text.
- **Confidence:** 9/10.

### MIN-3. The lint's type check is shallow

- **Where:** `specs/systems/lint_odd.py`, `neutral_problem`.
- **Evidence:** I copied the specs and set M7's `person.sex` neutral to `"not_a_sex"`, `person.mother` (a `u64` column) to `-5`, and `polity.exec_constraint` (a `q8_8` column) to `1e9`. `python3 -I specs/systems/lint_odd.py --specs <copy>` gave `PASS: 6 system specs linted`, exit 0.
- **Fix:**
  - Check that an enum value is a member of its enum, which needs machine-readable enum values (see MAJ-3).
  - Range-check unsigned and fixed-point values.
  - Parse fixed-point literals as decimals (`tomllib.load(parse_float=Decimal)`), so that "the exact decimal of the literal" is checked rather than assumed.
- **Confidence:** 9/10.

### MIN-4. The stale-skeleton positive control now fails for three reasons, not the one it plants

- **Where:** `tests/lint_fixtures/skeletons/stale/M7_dynasty.rs` and `T1_minds.rs`, still `0.3.1`.
- **Evidence:**
  - `python3 -I bin/gen_skeletons.py --check --out tests/lint_fixtures/skeletons/stale` reports STALE for `M2_demography.rs` (planted) and also for `M7_dynasty.rs` and `T1_minds.rs`.
  - On `main` both files are byte-identical to the generated ones (same sha1).
  - `tests/lint_fixtures/README.md:106` says: "After a spec change, regenerate it … and re-apply the edit".
  - The check_all matcher sees only the M2 line, so the gate still passes, but the fixture no longer isolates its rule.
- **Fix:** Regenerate the stale directory and re-plant the M2 edit.
- **Confidence:** 10/10.

### MIN-5. Several refusals and counters have no names

- **Where:** EVO `:51-54`.
- **Evidence:** These are unnamed:
  - the queue-age invariant's id (compare the core's `agenda-no-stale-timer`);
  - the `SimError` raised when `submit` gets an integer in an event-typed slot;
  - the diagnostic counters for `late`, dropped sends and dead-person drops;
  - the owner-side drop report (MAJ-2 point 5).

  V-INV-15's positive controls need to match on these names.
- **Fix:** Name each one in the rule text, as `05.1` already does for the core invariants.
- **Confidence:** 8/10.

### MIN-6. M7 0.3.2 collides with open PR #41

- **Where:** `specs/systems/M7.dynasty.odd.toml:27,35`; `origin/arch/P1a-effect-language` `19eda28`.
- **Evidence:**
  - PR #41 bumps M7 from 0.3.1 to 0.3.2 for `holders_of(title, 8 → 16)`, which REV-SPR-017 r4-4 (a closed record) already calls "0.3.2". This commit bumps M7 to 0.3.2 for `neutral`.
  - A trial merge (scratch clone, `6bc202e` + `19eda28`) conflicts in the M7 header comments.
  - The `spec_version` line and the generated `M7_dynasty.rs` change identically, so they merge cleanly and `gen_skeletons --check` stays green. A resolver who keeps both changelog lines would ship one "0.3.2" with two different contents.
  - `check_all.sh` and the inventory test also conflict, mostly because #41's base (`462bc03`) predates SPR-009.
- **Recommended order:** PR #41 first: it is older, its version is fixed in a closed review record, and it waits on D37. Then rebase EVO-001 with M7 at 0.3.3 (T1 stays at 0.3.2), regenerating `M7_dynasty.rs` and the stale fixture. The EVO's own §6 Q6 allows this.
- **Confidence:** 9/10 on the collision, 7/10 on the order.

### NIT-1. The smoke read count is wrong

- **Where:** EVO `:124` says "seven foreign reads". The diff adds `neutral` to nine reads across six specs (X2 1, X3 1, X4 1, X5 2, X6 2, X7 2).
- **Confidence:** 10/10.

### NIT-2. Some statuses are pre-claimed or stale

- **Where and evidence:**
  - The frozen M7 and T1 changelog lines say "approved with its PR" before the director has ruled.
  - `PRJ-002:153` and `:157` still say "Missing", and `:40`, `:161` and `:162` still say "frozen 0.3.1".
  - `docs/decisions_pending.md` has no D-number for EVO-001, although the EVO names the director as approver.
- **Fix:** Write "frozen; amended by EVO-001 (D3x)" once approved, and update the plan rows and the pending decision list.

### NIT-3. The termination bound is true but not written down

- **Where:** EVO `:49`.
- **Fix:** Add one sentence. Each binding and each timer pass runs at most once per instant, and a command reaches only jobs after its merge level. So a chain of sends within an instant has at most as many hops as the instant has jobs, and a ping-pong between A and B crosses an instant boundary each round.

### NIT-4. The Affects row leaves out M2

- **Where:** EVO `:23`.
- **Evidence:** The text depends on M2's new `Day/Vital` binding. That is a change to frozen M2 0.2.0, done in SPR-014: `real_specs.rs` currently refuses `binds Day/Vital but its spec's [[schedule]] declares Month/Vital`.
- **Fix:** List it under Affects.

## Sound

**Determinism**
- The queue key `(tick, phase, PersonId, seq)` is a total order. It is held in a `BTreeMap`, with `Phase`'s derived `Ord` in the `05.1` §4.2 order, and `seq` assigned by the merge in canonical order.
- Inboxes are taken before dispatch and merged after the level (`engine.rs:1038-1050`, `:1194-1205`), so delivery does not depend on thread count or on HashMap order.
- Delivery within an instant: the code does what §2.1 says. The `M7 Polity → T1 Agenda` path holds, and an Agenda send to M7 or M2 waits for the next day start.
- There is no cycle or fixed-point loop (NIT-3).
- EventIds are still assigned only by the merge, so EventId order is emission order. Commands are plane-A state (`rh-core/engine.commands` is hashed and stored in keyframes).

**Planes and registration**
- Planes: nothing in the proposal lets plane A read plane B or C. The dead-person drop, the neutral values and the queues are all plane A, and diagnostics and reports are host-side.
- The rate-read neutrals agree with their `[[couplings]]` prose: M7 `exec_constraint = 0`; T1 `vow.conflict = 0` and `rumour_reach = 0`.
- Typing via the owner's `[[owns]]`, else `[era_state]` by bare name and owner, gives the right type for every value in both specs. `exec_constraint` is `q8_8` from M6's `[era_state]` entry.
- The withdrawal of G5 and the "drop and report" rule match NEW-4, D36 and REV-SPR-017 M5.
- The queue-age formula holds for P1a and for smoke: every owner's finest pulse is `Day`, every wait is shorter than 1,440 minutes, and the period "at t" is the period containing `tick`.

**Gates and generated files**
- `python3 -I specs/systems/lint_odd.py` gives `PASS: 6 system specs linted (27 warnings)`, exit 0. The warnings are on the draft S5, S6 and S7 only.
- The fixtures `typed` and `draft_missing` exit 0; `missing`, `mistyped` and `untyped` exit 1. Each fixture differs from `typed` only in its planted line and header comment, and the check_all matchers match the actual output text.
- The five gates sit inside `part core` (`check_all.sh:767-771`, between `part core` at `:656` and `part golden` at `:855`). The inventory test passes: `pytest python/tests/test_check_all_inventory.py`, 12 passed.
- `gen_skeletons --check` is up to date for rh-core (6 files) and smoke (7). `ruff check .` and `mypy` pass.
- Smoke: adding `neutral` changes no generated file. The Rust spec loader (`rh_registry::ownership::DeclaredRead`, no `deny_unknown_fields`) ignores the field, and `SPEC_VERSION` is not hashed. No smoke hash can move from this commit itself; BLK-1 concerns the proposed rule, not this diff.

**What is the director's**
- Changing the frozen M7 and T1 specs is correctly routed to the director (D36).
- The G5 withdrawal and the V-test ids are correctly the Architect's.
- Nothing in the diff changes the roadmap. The only requirement-level change is the one in MAJ-5.

## Not run

- The full `bin/check_all.sh`, either part. Of the documented commands, I ran `lint_odd.py`, the smoke `lint_odd`, the five fixture commands, both `gen_skeletons --check` runs, the stale control, `ruff`, `mypy` and the inventory pytest.
- The smoke golden run and `check_baseline.py`. BLK-1 is shown from the code, not from a run: no implementation of the proposed rule exists to run.
- I did run the Rust tests `cargo nextest run --locked -p rh-sys-smoke -p rh-registry -p rh-core --test smoke --test real_specs --test registration`, in the copy with a scratch target directory: 31 passed. One smoke test is named `a_run_exercises_timers_commands_events_and_invariants`, which is further evidence for BLK-1.
- The rest of the Rust suite (`cargo nextest run --workspace`), clippy and cargo-deny.
- SPR-014's ladder comparison for `solo`, `household` and `hamlet` under M2's new Day binding, which §5 itself defers.
- The effect-language reference (`specs/registry/effect_ref.py`) against the new `neutral` fields: PR #41 is not merged here.


## 2. Round 2 (2026-10-17, `b1c6890`)

### 2.0 Architect rulings, round 2

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R9 | R6 (held from round 1) and MAJ-6 | **Shape (b), strengthened.** A command that carries an outcome sampled for a later instant (`ScheduleDeath`, `Marry`; any command whose effect is a drawn, dated outcome) is never submitted ahead of that instant. The sender holds the outcome as its own plane-A timer under its own purpose, from the moment it draws it, and the timer submits the command when it fires: in that instant's timer pass, at or before the owner's `drained_by` binding, so the owner applies it in the same instant. This applies to T1 as well as M7 (T1 fires from its existing `Day/Vital` binding; M7 from `murder_due`). Re-sampling is the sender's `resample_timer`, under the sender's purpose. An undated request command (`PressClaim`, `Promote` and the like) is not a sampled outcome and may wait for the owner's binding, bounded by the queue-age invariant. DET-13, V-DET-12 and ADR-0025 stand as written, so **D39 is withdrawn**. A lint check: a `[[commands]]` entry for an outcome-carrying command names the sender's timer kind and purpose. The births draft (PR #50) switches to (b) | 7.5/10 |
| R10 | MIN-7 | Accepted. M7's murder timer states its `resample_timer`, a re-draw under `exit_murder` that never errors, and carries the culprit and the cause, in the payload or through an M7-owned plane-A pending table. Do the same for T1's new timer | 8/10 |
| R11 | MIN-8 | Accepted. One counter per (system, command kind, reason), and the timer case reaches it | 8/10 |
| R12 | MIN-9 | Every identity read always requires its owner; there is no flag. A system with an identity read of a system absent from P1a does not register in P1a, by name. `neutral` is explicit on every `rate` and `mode` read; the type's zero is never a fallback (enum zeros mean something). This corrects round 1's R4 as built | 8/10 |
| R13 | MIN-10, MIN-11, MIN-12, the nits | Accepted. The key person names a payload field. The lint refuses two `[[applies]]` entries for one command. The `sprint` escape carries a named sprint and is an error on a frozen spec once that sprint is on `main`. Fix the nits | 8/10 |
| R14 | `05.1` version | Set the version line when merging. SPR-011's audit is likely first (v1.10); PR #51 is next; this PR takes the next free number after merging `main` | 9/10 |

### 2.1 The review (round 2, independent reviewer)

Reviewer: independent, adversarial, on a different model from the drafter's. Copy reviewed: `$R/src`, a `git archive` of `b1c6890`. Round 2 adds three commits on top of `6bc202e`: `f44db0c`, `b604098` and `b1c6890`. Nothing in `/home/user/Rehoboam` was modified. Line numbers refer to the copy.

### BLUF

**Approvable after one small revision: 0 blockers, 1 major, 6 minors, 7 nits.**

- **R1, R2, R3, R5 and R8 are applied.**
  - The smoke ladder is byte-identical to its baseline.
  - M7's and T1's command timing now matches their frozen texts.
  - The lint works: every fixture, and every planted probe listed in §4 except the gaps listed there, behaves as expected.
- **R4, R6 and R7 are partly applied.**
- **The major (MAJ-6).** Shape (b) as written does not keep DET-13 "as written". T1's `ScheduleDeath` and `Marry` still travel as undated commands from `Agenda` d to M2's drain at d+1, across the `Inputs` phase of d+1, without being timers. That is round 1's MAJ-5 evidence, unchanged for T1. The births drafter's own shape-(b) paragraph (L5) gives T1 a timer; the EVO says T1 needs none.
- **M7's path is exact.** Its murder is an M7 timer from the moment it is drawn, and it is applied in the same `Vital` slot as under the frozen text.
- **Recommendation: shape (b), confidence 7/10.** Add to D39 a one-line reading of DET-13 that covers commands drained at their owner's next draining run, and have M7 state how it re-samples its murder timer and that the timer carries the culprit (MIN-7).
- **What goes to the director.** Shape (a) amends DET-13 and, in effect, ADR-0025 item 3. Shape (b) needs only the DET-13 reading, and only because of T1's next-day commands. Either way, the point rides on D39, which is already the director's.

### 1. R1–R8 status

| # | Ruling | Status | Evidence, and what is missing |
|---|---|---|---|
| R1 | BLK-1: drop only in ledger worlds, only for `key = "living"`; smoke unchanged | **APPLIED** | <ul><li>Every smoke `[[applies]]` entry is `key = "any"`, `drained_by = "Day/Activity"`. That matches the code: `estate.rs:33` and `market.rs:83` are the only `take_commands` calls, and both sit in the Activity steps.</li><li>Built and ran `rh-cli run smoke --years 1000 --ladder yearly` (release, `CARGO_TARGET_DIR=$R/target`). `bin/check_baseline.py` reports `BASELINE OK` and `cmp` reports the ladder byte-identical to `worlds/smoke/baselines/smoke_yearly_1000y.ladder`; the last rung's state hash is `bef936ec026341a0`.</li><li>EVO §5, PRJ-002 §8, V-INV-15 and SPR-014's acceptance carry "smoke ladder unchanged".</li><li>(No code changed in this PR, so the run proves the specs and the code agree today. The rule itself is SPR-014's.)</li></ul> |
| R2 | MAJ-1: `drained_by`, one binding per kind, untaken commands kept queued in key order; M7 and T1 timing | **APPLIED** (lint gaps MIN-11, MIN-12) | Timing traced against `05.1` §4.2 and the derived order (`real_specs.rs:779-792`):<ul><li>**T1 → M7.** On ordinary days M7's and T1's `Day`/`Agenda` bindings share a level. A `PressClaim` T1 sends at `Agenda` d merges after M7 has run and is drained at `Agenda` d+1, after that day's `on_death` arbitration at `Polity`. That is the frozen "next tick boundary".</li><li>**M7 → T1.** `Promote` and `OpinionModifier`, sent at `Polity`, are drained by T1's `Day`/`Agenda` binding in the same instant. At a month start, T1's `Month`/`Agenda` binding comes first and no longer drains (D1 is gone).</li><li>**T1 self-sends.** They wait for the next `Day`/`Agenda`, not the next `Vital`.</li><li>**Text.** §3 no longer claims "no mechanics change" for delivery.</li></ul> |
| R3 | MAJ-2: the five points of the drop | **APPLIED** (MIN-8) | <ul><li>(1) The value of `person.alive` is pinned to the level start (`05.1:67`).</li><li>(2) Ledger-less worlds drop nothing.</li><li>(3) Commands about the dead opt out (`key = "any"`).</li><li>(4) `apply_death` is idempotent.</li><li>(5) The owner's drops are counted under `commands.dropped_by_owner{owner, command, reason}` via `StepCtx::drop_command`.</li><li>The counters for the dead-victim cases contradict each other (MIN-8).</li></ul> |
| R4 | MAJ-3: `neutral` only on rate and mode reads; identity reads require their owner; dropped reads served as an empty table; enum codes; V-SF-22 sentence; G5 | **PARTLY APPLIED** | Done:<ul><li>(1) `neutral` on rate and mode reads, checked against the coupling's prose (the check is weak: NIT-8).</li><li>(3) Empty tables.</li><li>(4) `values` or `registry` codes on every enum `[[owns]]` column.</li><li>(5) The V-SF-22 sentence is struck.</li></ul>Not as ruled:<ul><li>(2) became a per-read flag. 18 identity reads of M7, S6, S7 and T1 are `required = false` (for example `T1.minds.odd.toml:609-700`, `M7.dynasty.odd.toml:496`).</li><li>The "type's zero means none" fallback is false for three enum columns (MIN-9).</li><li>G5 is withdrawn for every non-required read, not just rate and mode reads.</li></ul>The Architect should accept or reject the generalization explicitly. |
| R5 | MAJ-4: typed payloads in a closed vocabulary, lint-checked, in M2, M7, T1 and smoke | **APPLIED** (NIT-6) | <ul><li>15 typed `[[applies]]` entries: M2 2, M7 4, T1 7, smoke 2.</li><li>`EventRef` on `cause_event` and `source_event`.</li><li>The lint refuses untyped and out-of-vocabulary fields and `enum(...)` of a non-enum column (probes A1-A3).</li><li>M2 0.2.1's changelog says "no mechanics changed", yet it removes 0.2.0's later-tick hold (NIT-6).</li><li>The births draft (`35226c0`) still has untyped, dated payloads; that is PR #50's work.</li></ul> |
| R6 | MAJ-5: write both shapes exactly; recommend one | **PARTLY APPLIED** (MAJ-6, MIN-7) | <ul><li>Both shapes are written, with the four facts.</li><li>M7's path under (b) is exact (§3 (i)).</li><li>The claim that DET-13 and ADR-0025 "stand as written" is false for T1's next-day `ScheduleDeath` and `Marry`.</li><li>M7's re-sample and the culprit's carrier are unstated.</li></ul> |
| R7 | MIN-1 to MIN-6, nits | **MOSTLY APPLIED** | <ul><li>MIN-2, MIN-4 (only `M2_demography.rs` is STALE now), MIN-5, MIN-6, NIT-1 (nine), NIT-3 and NIT-4: done.</li><li>MIN-1: done in EVO §1, BCK-002 A-30, T1 `Day`/`Vital` and `05.1`, but M7 0.3.2 keeps the old wording at `:537` and `:713` (NIT-5).</li><li>MIN-3: membership, integer ranges and decimal parsing are done; fixed-point representability and range after ruling-9 rounding are not checked (NIT-8).</li><li>NIT-2: done, but D39 pre-claims round 2 (NIT-9).</li></ul> |
| R8 | (b) late commands; (e) `docs/09` rows; (f) `05.1` applied as v1.10 | **APPLIED** (NIT-7) | <ul><li>(b) is moot under undated commands.</li><li>V-DET-21 and V-INV-15 are added and V-INV-04 amended.</li><li>The `05.1` text is applied, but the file header has no v1.10 line.</li></ul> |

### 2. New findings

#### MAJ-6. Shape (b) as written still lets an undated command stand in for a later-instant outcome: T1's `ScheduleDeath` and `Marry`

- **Where:**
  - `05.1:65` ("T1's `Marry` and `ScheduleDeath`, sent at `Agenda`, are applied by M2 at the next day start's `Vital`");
  - `05.1:66` ("so DET-13 and ADR-0025 stand as written: the agenda holds every dated outcome");
  - EVO `:83` ("a sender whose outcome is due at the drainer's next run just submits"; "T1 needs no timer at all"; "DET-13 and ADR-0025 stand as written");
  - EVO `:85` ("The two agree on the binding rule and on the facts");
  - `docs/09:117` (V-DET-21: "a dated outcome never travels as a command");
  - `T1.minds.odd.toml` `[[commands]] ScheduleDeath` ("so the victim dies at the next day start").
- **Evidence:**
  - DET-13 (`docs/03:75`): "An outcome sampled for a later instant (death, …, marriage) shall be a plane-A **timer**." V-DET-12: "sampled outcomes sit in the agenda as timers until they fire."
  - T1 draws a successful `MURDER_ATTEMPT` at `Agenda` d. The victim dies at `Vital` d+1, a later instant. Between the two, the death is a queued command, not a timer.
  - `Engine::apply_inputs` re-samples only `self.agenda.pending_for(entity)` (`engine.rs:936`). A scoped material input on the victim at `Inputs` d+1 therefore cannot reach the death.
  - This is round 1's MAJ-5 evidence word for word. R6 (b), as the Architect worded it, says "Outcomes dated later are timers owned by the system that sampled them". The drafter's carve-out ("due at the drainer's next run") is not in the ruling.
  - The carve-out also generalizes: under the queue-age invariant, a `Month`-drained owner's "next run" is up to a month away.
  - The births drafter's shape-(b) paragraph (`35226c0`, `specs/systems/next/M2.demography.odd.toml`, `[algorithm.lifecycle_f1]` L5) has "the senders' specs schedule their own timers (M7's murders, T1's one-day delay)". So the two drafts do not agree on the facts.
  - The same one-day crossing applies to T1 → M7 commands (`PressClaim` and the others) and to T1's self-sends. DET-13's list does not name those.
- **Why the cheap fix is not free.** T1 could hold a timer at (d+1, `Vital`); its `Vital` timer pass precedes M2's drain, so the timing would be identical. But:
  - `Timer::payload` is one `i64` (`agenda.rs:91`);
  - the merge rewrites `DraftRef`s only in events and commands, and it merges timers before events (`05.1` rule 4);
  - so the timer could not carry `cause_event` (the `MURDER_ATTEMPT`). The DEATH ← MURDER_ATTEMPT edge would be lost, against (iii). Keeping it needs a core change (rewriting a `DraftRef` in a timer payload, plus a second payload slot) or a T1 pending table.
- **Fix:** choose one.
  - **(1) Recommended.** Put a one-line reading of DET-13 to the director inside D39: *a lifecycle command submitted at t and drained at its owner's next draining run, which P1a bounds to one day by `drained_by` at `Day`, delivers an act decided and emitted at t; it is not an outcome sampled for a later instant, and ADR-0025 item 3 (timers in scope) does not reach it.* The argument for it: re-drawing a death after its `MURDER_ATTEMPT(success)` event was emitted would contradict an event already in the log. Bound it in the rule: only for `Day` drains, and never for an outcome that a hazard draw dates.
  - **(2)** Give T1 timers too, and pay the core change above.
  - Either way: correct `05.1:66`, EVO `:83` and `:85`, V-DET-21's last clause, and D39's text.
- **Confidence:** 8/10 that the defect is real; 7/10 for (1) over (2).

#### MIN-7. M7 0.3.2 does not say how its murder timer is re-sampled, or that it carries the culprit

- **Where:**
  - `M7.dynasty.odd.toml:572-575` (`[[rng]] exit_murder`: no `resample_on_fork`, no re-sample rule);
  - `:705` (`exit_hazards`: "whose fire (murder_due) submits ScheduleDeath {… culprit = murder_culprit …}");
  - `:525` (`restart` lists deposition, interregnum and regency timers, not murder timers).
- **Evidence:**
  - Under (b), every material input whose scope holds a ruler with a pending murder timer calls `M7::resample_timer` (`engine.rs:936-1001`).
  - The trait's default returns an error (`system.rs:759-771`), so the run fails. P1a golden worlds run scope-only re-sample inputs (SPR-011 ruling 15).
  - EVO §4 (b) fact (2) ("the sender re-draws its own timer through the existing `resample_timer` … no new path") depends on a rule M7 never states.
  - The culprit: `murder_culprit` is keyed at the month's first instant over "the title's living claimants". If the developer draws it at fire time on day k, the claimant set differs and so may the culprit. The timer's `i64` payload can carry the day-1 draw; the engine keeps the payload through a re-sample.
  - The deposition, interregnum and regency timers have the same gap (pre-existing, in M7 0.3.1).
- **Fix:**
  - In M7 0.3.2, state that the murder timer's payload is the culprit `PersonId` drawn at `exit_hazards` (0 = none).
  - State `exit_murder`'s re-sample. Either keep the day (`resample_timer` returns the timer's own tick), or re-draw the day among the month's remaining day starts at the new epoch, like M2's S11. Set `resample_on_fork` to match.
  - Add the murder timer to `restart`.
  - Note the deposition timer's identical gap for SPR-021.
- **Confidence:** 8/10.

#### MIN-8. The counters for a dead victim contradict each other, and one cannot be reached

- **Where:** `05.1:67`.
- **Evidence:**
  - One sentence counts "a key person it killed earlier in the same drain" under `commands.dropped_by_owner`. The next counts "two senders' deaths for one victim" under `commands.dropped_dead`, and in one drain these are the same case (both victims are alive at take, so `rh-core`'s drop sees neither).
  - The same sentence counts "a death timer … for a person already dead" under `commands.dropped_dead{owner, command, sender}`. A timer has neither a command nor a sender, and the only owner-side API named, `StepCtx::drop_command(command, reason)`, takes a command.
  - V-INV-15's controls will have to match one of these names.
- **Fix:**
  - `commands.dropped_dead` is `rh-core`'s drop at take, and only that.
  - Everything the owner finds in its drain is `drop_command`, with reason `dead`.
  - A timer that fires for a dead person gets its own named counter and call (for example `timers.dropped_dead{owner, kind}` via a `StepCtx` method), or `apply_death` cancels the victim's other death timers.
- **Confidence:** 8/10.

#### MIN-9. Identity reads of absent owners: R4(2) applied as a per-read flag, and the "type's zero means none" fallback is false

- **Where:** `05.1:69` ("or, for an identity read that declares none, with the type's zero (id 0, `false`, 0, code 0), which the owner's spec makes mean 'none'"); T1's and M7's `required = false` identity reads.
- **Evidence:**
  - R4(2): "An identity read declares its owner required." Round 2 makes `required` a per-read choice and marks 18 identity reads of M7, S6, S7 and T1 `required = false`.
  - Enumeration as an empty table is sound: no seats or claims, not "every seat vacant".
  - A keyed lookup by an id the reader obtained elsewhere returns the type's zero. For three enum columns, code 0 is not "none":
    - `claim.status` → `active`;
    - `vow.status` → `active`;
    - `title.tier` → `sovereign` (Registry `title_tiers.ids`).
  - Together with `title.holder = 0` and `title.liege = 0`, a T1 lookup of a `CLAIM_TITLE` goal's target TitleId in a world without M7 returns a vacant sovereign title: MAJ-3's class of state.
  - Unreachable in P1a (S7 is P1b), but the contract sentence is false as written.
- **Fix:**
  - The Architect explicitly accepts or rejects the per-read flag.
  - A keyed lookup of a dropped identity column returns "no row" (the accessor's `None`), never a value; `neutral` is for rate and mode reads only.
  - Or the lint requires a `neutral` on every non-required enum identity read.
- **Confidence:** 8/10.

#### MIN-10. A command's key person is not tied to a payload field

- **Where:** `05.1:67` ("its key person (the `PersonId` of its key)"); the `[[applies]]` schema.
- **Evidence:**
  - The key's `PersonId` is whatever the sender passes as `Command::person`.
  - It decides both the dead-person drop and the delivery order.
  - `OpinionModifier` has three senders (M7, S7, T1) and two `PersonId` fields (`holder`, `about`). Only a comment in T1 says "keyed by the holder".
  - Keying by `about` would silently drop a grief or feud opinion about the dead (`key = "living"`).
- **Fix:** Add `key_field = "<a PersonId field>"` to `[[applies]]`, checked by the lint (the field exists and is `PersonId`), and require senders to key by it.
- **Confidence:** 7/10.

#### MIN-11. The lint does not enforce one draining binding per command kind

- **Where:** `lint_odd.py:216`, `applies = {a.get("command"): a for a in ts.get("applies", [])}`.
- **Evidence:** Probe A10 adds a second `[[applies]] PressClaim` to M7 with `drained_by = "Day/Polity"`. Result: `PASS`. The dict keeps the last entry, so the first entry's binding is silently ignored.
- **Fix:** Make a repeated command name an error.
- **Confidence:** 9/10.

#### MIN-12. The lint's `sprint` escape is unbounded on frozen specs

- **Where:** `lint_odd.py:230`: any non-empty `sprint` turns an undeclared `drained_by` into a warning.
- **Evidence:** Probe A9 sets frozen M2's `drained_by = "Hour/Inputs"`. Result: `PASS`, one WARN.
  - The escape exists for one real case: M2's `Day`/`Vital`, which SPR-014 adds.
  - As written it would let any frozen spec name any binding indefinitely.
- **Fix:** Restrict the escape to an allow-list. The simplest is M2's `Day`/`Vital`, keyed by the spec id and SPR-014, printed like a counted `LINT-ALLOW` and deleted by SPR-014. Or let a `[[schedule]]` entry carry `status = "planned"`.
- **Confidence:** 8/10.

#### Nits

- **NIT-5.** M7 0.3.2 keeps the old wording:
  - `:537` ("M2's death timers fire in the timer pass");
  - `:713` ("every death is an M2 timer in the timer pass of this day start's `Vital` slot"), which is MIN-1's residue; `:525` is covered by MIN-7.
  - `:554`: the `murder_due` step reads as work of the `Day`/`Vital` binding. The work is M7's `fire` in the `Vital` timer pass; the binding's step is empty. Built inside `step`, which runs at level 1 of the binding pass after M2's drain at level 0, the death would slip a day. Say so.
- **NIT-6.** `M2.demography.odd.toml:8` says "no mechanics changed", but 0.2.1 deletes 0.2.0's "a later tick becomes a timer at that tick" in the (unbuilt) drain. Say "no built mechanics or hashes changed".
- **NIT-7.** Ordering and headers:
  - The `05.1` header has no v1.10 line (R8 (f)).
  - In `docs/09`, V-DET-21 (`:117`) precedes V-DET-20, and V-INV-15 (`:137`) precedes V-INV-14.
- **NIT-8.** Lint value checks are shallow:
  - The prose match (`mentions`, `lint_odd.py:199`) accepts any whole-token number. M7's `exec_constraint` neutral `1.0` passes because the prose contains "m_ec = 1" (probe R6).
  - Fixed-point literals are range-checked before ruling-9 rounding and never checked for representability. `0.001` (which rounds to 0 in `q8_8`) and `127.999` (which rounds to 128, so it does not fit) both pass.
  - Duplicate payload field names pass.
  - `cause_event: u64` passes, so the merge would not rewrite it.
  - A missing `key` on a frozen spec silently means `any`.
  - The `mistyped` fixture trips two errors (type and prose).
- **NIT-9.** Claims that do not match the files:
  - EVO §4 says "13 typed entries"; there are 15.
  - D39 says "Reviewed … rounds 1 and 2" before round 2 has reported.
  - EVO `:85` says the two drafts agree on the facts (see MAJ-6).
- **NIT-10.** `drained_by` has no check against the code. Setting smoke X4's `Settle` to `drained_by = "Day/Contagion"`, a declared binding the code never drains, passes the lint (probe S1). SPR-014 should emit the drain bindings into the generated skeleton and compare them at registration, as `SCHEDULE` is compared.
- **NIT-11.** Merge conflicts:
  - With `origin/main` in `PRJ-002` (main's `132e17d` status rows; a trivial conflict).
  - With the births draft in `lint_odd.py`, `check_all.sh`, the inventory test, the fixtures README and `PRJ-002`.
  - Merge `main` before landing.

### 3. R6: scheduled outcomes

#### (i) The instant at which a murder's death applies under (b)

**M7, background murder.** Drawn at `Month`/`Polity` on day 1 (`exit_hazards`). The step schedules an M7 timer at (day k, `Vital`), k ∈ [2, last]; it is strictly after (day 1, `Polity`), so the agenda accepts it, and it enters the agenda at that level's merge. On day k:

1. `Inputs`. A scoped input on the ruler re-samples the timer through M7 (MIN-7).
2. `Vital`. `run_instant` (`engine.rs:842-879`) runs the whole timer pass before the binding pass:
   - Timer-pass level 0: M2's own death timers (natural deaths).
   - Timer-pass level 1: M7's fire. M7 is at the level of its first `Vital` binding, after M2 because it reads M2 at `same_instant`, which `real_specs.rs:779` asserts. The fire submits `ScheduleDeath`, which merges into M2's inbox.
   - Binding-pass level 0: M2's `Day`/`Vital` drain applies it.
   - Binding-pass level 1: M7 (empty step) and T1's grief.
3. `Polity`. M7's `on_death` sees the death.

The result is the same instant as the frozen text, where M2 converted the dated command into an M2 timer at (day k, `Vital`). **No day is lost**, and this holds whatever M7's level, because the timer pass always precedes the binding pass. What M7 does need is some binding in `Vital`: without one, the engine raises `SimError::Timer` ("due in phase Vital, where the system is not bound"). That is what the new binding is for.

**T1, foreground murder.** Sent at `Agenda` d; M2's `Day`/`Vital` binding drains it at d+1. That is the frozen timing (`tick` = the next day start, applied in the drain). It is exact, but it is not a timer (MAJ-6).

**Two within-instant differences from the frozen text** (rare; no golden world holds them):

- **A natural death and a murder on the same day.** Under (b), the natural death in the timer pass always comes first. Under the frozen text, M2 held both as timers in one slot and fired them in agenda-key order, so a natural-death timer re-sampled after the murder's conversion would have come second.
- **An M7 murder and a T1 murder of the same victim on the same day.** Under (b), T1's command is applied first: it has the earlier key tick, d. Under the frozen text, M7's converted timer fired in the timer pass ahead of the drain. The DEATH's culprit and `cause_event` differ.

#### (ii) DET-13 and ADR-0025

- **Holds:** for M7's murder and deposition and for every M2 outcome. Each is a plane-A timer from the merge of the step that drew it, with no `Inputs` phase in between. An input reaches it through its owner's `resample_timer`, once M7 states that rule (MIN-7).
- **Does not hold as written:** for T1's next-day `ScheduleDeath` and `Marry` (MAJ-6), and, if DET-13 is read broadly, T1's other next-day commands.

#### (iii) Nothing is lost

- **M7:** cause = murder (constant); `cause_event = None` (a hazard draw has no inciting event); the culprit fits the timer's `i64` payload and survives a re-sample, since the engine re-inserts `entry.timer` unchanged. The spec has to say that the payload carries it (MIN-7).
- **T1:** `cause_event` is the `MURDER_ATTEMPT` `DraftRef`, rewritten at the merge (R5's `EventRef`).

#### (iv) M7's new binding is a frozen-spec mechanics change

It is acceptable: M7 is not built, no hash moves, R2 already accepted the frozen-spec cost in this PR, and D39 carries it to the director. It is **not the only one**:

- M7: the murder becomes an M7 timer, with a re-sample obligation and a payload (MIN-7).
- M7: the two within-instant order changes in (i).
- M2 0.2.1: the drain loses its later-tick hold (NIT-6).
- T1 0.3.2: none in effect, since the timing is unchanged.

#### Shape (a), as the births draft writes it (`35226c0`, `[algorithm.lifecycle_f1]` L1–L4, `pending.*`, purpose `commanded`)

- **Timing.** Exact, and it keeps the frozen within-slot order: commanded deaths fire in M2's timer pass.
- **DET-13.** Amended, as the EVO and L-header say themselves:
  - M7's dated command waits as a queued command from (day 1, `Polity`) to the drain at day 2, crossing `Inputs` of day 2;
  - T1's waits from d to d+1.
- **ADR-0025 item 3.** Hollowed out for commanded outcomes. L4's `resample_timer` returns the timer's own tick, so a scoped input cancels and re-inserts the timer unchanged: a re-sample in name only.
- **ADR-0025's intent.** It also turns M7's **hazard-drawn** murder into an outcome no input can ever re-draw, only because it crossed a system boundary, whereas M2's own hazard deaths are re-drawn (S11). That cuts against what ADR-0025 is for.
- **Cost:**
  - an M2-owned plane-A table (`pending.*`, 8 columns, keyframed);
  - INV-M2-13;
  - two new timer kinds and a purpose;
  - a `restart` obligation.

#### Recommendation

**Shape (b)**, confidence **7/10**, with:

1. **MAJ-6's option (1):** one line in D39 reading DET-13 for commands drained at their owner's next `Day` run. T1 then keeps its plain send, and the `MURDER_ATTEMPT` → DEATH edge is kept.
2. **MIN-7:** M7 states its murder timer's payload and re-sample.

Under (b), ADR-0025 is not amended, and DET-13 needs only that bounded reading. Under (a), DET-13 is amended (the drafts' own words) and ADR-0025 item 3 is amended in effect. **Neither is the Architect's alone; both ride on D39, which already goes to the director.** If the director declines the DET-13 reading, (b) still wins over (a), but T1 then needs timers, which costs a core change (a `DraftRef` in a timer payload, plus a second payload slot).

The 3/10 against: (a) keeps the frozen within-slot order and the "dated command" vocabulary that the frozen specs used, and the births draft is already written in it.

### 4. Lint probes (new rules; planted copies of `specs/systems` or `worlds/smoke/systems`, run with `--specs`)

Planted violations that must fail, and do:

| Rule | Probes |
|---|---|
| `required` | R1 (missing on frozen M7); R8 (`"yes"`, not a bool); R2 (a required read with a `neutral`) |
| `neutral` | R3 (rate read with no `neutral`); R4 (128.0, outside `q8_8`); R5 (−128.0, prose mismatch); R11 (an enum code that is not a member) |
| Enum codes | E1 (missing on frozen); E2 (both `values` and `registry`); E3 (`registry` names no table); E4 (duplicate values) |
| `[[applies]]` | A1 (`PersonID` typo); A2 (`enum(person.mother)`); A3 (untyped); A4 (undeclared `Hour/Agenda`); A5 (malformed `drained_by`); A7 (`key = "Living"`); A8 (missing `drained_by`); A14 and A15 (the sender's field names or order, including a stale `tick`) |
| Smoke | S3 (`row: int`) |

Legitimate specs that must pass, and do: R9, R10 (a non-required mode read with neutral `"none"`), A6 (`drained_by` naming another declared binding, T1 `Month`/`Agenda`).

**Gaps** (a planted violation passes): A10 (MIN-11), A9 (MIN-12), R6, the representability and rounding probes, duplicate field names, A12 and A13 (NIT-8), S1 and S2 (NIT-10).

### 5. Checks run

All in `$R/src`, the copy.

**Python**
- `python3 -I specs/systems/lint_odd.py`: `PASS: 6 system specs linted (33 warnings)`, exit 0. The warnings are on the drafts S5, S6 and S7, plus M2's two sprint-escaped `drained_by`.
- The smoke overlay lint: `PASS: 7`, 0 warnings.
- `bin/gen_skeletons.py --check`: rh-core 6 up to date; smoke 7 up to date.
- The stale control fails on `M2_demography.rs` only.
- All 11 `odd_applies/` and `odd_neutral/` fixtures exit as their `check_all` lines expect. Each differs from its control only in its planted lines and header.
- `pytest python/tests/test_check_all_inventory.py` with `python/.venv`: 12 passed.
- `ruff check .`: passes.
- `mypy`: no issues in 38 files.

**Rust**
- `cargo nextest run -p rh-core -p rh-registry -p rh-sys-smoke --locked`: 144 passed.
- `cargo build --release -p rh-cli`, then the smoke golden run (1 thread), then `check_baseline.py`: `BASELINE OK`, byte-identical.

**Trial merges** (`git merge-tree`) with `origin/main` and with `origin/arch/M2-F1-births`: conflicts as in NIT-11.

### 6. Not run

- `bin/check_all.sh` (either part), clippy, cargo-deny, the full workspace `nextest`, doctests, nightly tiers, the perf and allocation gates.
- The smoke golden run with `--threads 4` / `--require-parallel 5`, and `rh-golden`'s tests.
- V-DET-21, V-INV-15 and the dead-person drop: SPR-014 has no code yet, so the rule was checked by tracing the engine and specs, not by running it.
- The `solo`, `household` and `hamlet` ladders under M2's future `Day`/`Vital` binding (§5 defers this to SPR-014).
- The births draft's own gates (`lint_odd.py --next`, `selftest_m2_f1.py`).


## 3. Round 3 (2026-10-17, `1d08b06`; main merged in `757141b`)

### 3.0 Architect rulings, round 3
**Approve after one text revision:** 0 blockers, 1 major, 3 minors, 4 nits. R9 is accepted at 8/10. The reviewer traced both murders end to end: no day is lost, no death is applied early, each outcome is a timer from the step that drew it, and an input on the victim reaches it. T1's "own tick" re-sample is a re-draw over a single possible value and does not defeat ADR-0025.

| # | Finding | Ruling |
|---|---|---|
| R15 | MAJ-7 | Both changes the text gives SPR-014 must leave smoke's ladder unchanged, as the PR requires. (1) For smoke's commands, `key_field` names the `Command`'s existing `person` field; no new `args` entry. (2) The timer's `EventRef` cause slot is hash-neutral when empty: an absent cause adds no bytes to the timer's hashed encoding. If either cannot be made neutral, the text must say that SPR-014 regenerates smoke in a `[C3]` commit, with a bisect whose first divergence is the first command or timer it touches, announced to the director |
| R16 | MIN-13 | The lint's substrate rule is advisory. Registration enforces it: a world lacking a required owner is refused by name (R12). Say so in the rule's text; the lint keeps printing each spec's required owners |
| R17 | MIN-14 | M7's murder re-sample draws among the day starts at or after the input's instant within the month (the input's own day included when its day start is after the input), never cancelling when one remains; a murder whose month has no remaining day start moves to the next month's first day start |
| R18 | MIN-15 | The `sprint` escape requires that the named sprint's document exists under `CODEX/05_PROJECT/`; `SPR-999` is refused |
| R19 | Nits | As listed. The conflicts with `main` were resolved in `757141b` |

### 3.1 The review (round 3, independent reviewer)

Reviewer: independent, on a different model from the drafter's. Copy reviewed: `$R/src`, a `git archive` of `1d08b06`. Round 3 adds `b71f85e` (the round-3 text), `85c7e51` (a merge of `main` at `513e281`: PR #51), `afa6886` and `1d08b06`. Nothing in `/home/user/Rehoboam` was modified. Line numbers refer to the copy. `$R` = `/tmp/claude-0/-home-user-Rehoboam/78f410bc-aa41-55ce-883d-f1b2ebd2cbcd/scratchpad/evo001_review_r3`.

### BLUF

**Approvable after one more text revision: 0 blockers, 1 major, 3 minors, 4 nits.**

- **R9 works as built.** I traced a T1 murder and an M7 murder against `05.1` §4.2 and the derived order (`real_specs.rs:779-792`, `Vital` = M2, M7, T1):
  - No day is lost.
  - No death is applied before its sampled instant.
  - Each outcome is a plane-A timer from the merge of the step that drew it, with no `Inputs` phase in between.
  - An input that scopes the victim reaches the timer through the sender's `resample_timer`.
  - T1's "own tick" re-sample re-draws an outcome that has only one possible value (a fixed delay). It does not defeat ADR-0025.
- **R10, R11, R12 and R14 are applied. R13 is partly applied** (MIN-15).
- **The major (MAJ-7).** Round 3 tells SPR-014 to do two things that move the smoke ladder, while the same PR makes "the smoke ladder is unchanged; a difference is a defect" an acceptance criterion (R1). I measured both:
  - adding the key person to the smoke commands' `args`: `BASELINE MISMATCH`, `engine.commands` at 307 rungs;
  - encoding a timer's `cause` slot as an empty option: `BASELINE MISMATCH`, `agenda.timers` at 849 rungs, and the final state hash changes.
  - The fix is text only.
- **Checks.**
  - The smoke ladder built from the copy is byte-identical to the baseline.
  - The lint, the smoke lint, both skeleton checks and all 25 fixture gates pass.
  - The inventory pytest: 12 passed. ruff and mypy: clean. `nextest` on rh-core, rh-registry and rh-sys-smoke: 144 passed.
- **Main has moved** (`e9e0fe3`, PR #52). A trial merge has 5 conflicts, and 12 of this PR's lint fixtures use an `M1.regions` id, which now collides with the real `M1.regions` spec (NIT-13).

### 1. R9–R14 status

| # | Ruling | Status | Evidence, and what is missing |
|---|---|---|---|
| R9 | Shape (b), strengthened: an outcome command is submitted only by its sender's timer, in the outcome instant's timer pass | **APPLIED** (MAJ-7) | <ul><li>`05.1:68` states it, with the timer `cause` slot.</li><li>M2 0.2.1 marks `ScheduleDeath` and `Marry` `outcome = true`.</li><li>T1 0.3.2 `[[commands]]` (`:438-450`): `DEATH_DUE` and `MARRY_DUE` under `outcome_delay` (`[[rng]] :756-760`); `outcome_due` in `Day/Vital` (`:706`); `restart`.</li><li>M7 0.3.2 (`:366-368`, `:542`, `:563`): `MURDER_DUE` under `exit_murder`.</li><li>The lint requires `timer` and `purpose` (fixture `outcome_untimed`; probes P4, P11, P12, P13).</li><li>D39 is withdrawn from `decisions_pending.md`.</li><li>The trace is in §3.</li></ul> |
| R10 | M7's murder re-sample, never errors, carries the culprit; the same for T1 | **APPLIED** (MIN-14) | <ul><li>M7 `:563`: the day is re-drawn among the month's remaining day starts; the payload is the culprit; `cause = None`; `resample_on_fork = true`; `restart` `:512`; the deposition gap is passed to SPR-021.</li><li>T1 `:759`: own tick, keeps payload and cause, `resample_on_fork = false`.</li><li>The engine's API allows `None` (`system.rs:751-763`) and re-inserts `entry.timer` unchanged (`engine.rs:969-985`), so payload and cause survive.</li><li>M7's rule mishandles the input's own day (MIN-14).</li></ul> |
| R11 | One diagnostic per (system, kind, reason), timers included | **APPLIED** (NIT-12) | <ul><li>`05.1:69`: `dropped{system, kind, reason}`, `StepCtx::drop_timer`, reasons `dead_key`, `dead`, `ended`, `absent_owner`.</li><li>V-INV-15 (`docs/09:138`) uses it.</li><li>Residue: M7 adds a fifth reason, `resampled_out` (`:563`), that `05.1` does not list and that the engine's `None` path does not count (MIN-14).</li><li>EVO §6's MIN-5 row (`:105`) still names `commands.dropped_dead`, `dropped_by_owner` and `dropped_send`.</li><li>"`system` the dropper" vs `dropped{owner, command, dead_key}` for `rh-core`'s drop.</li></ul> |
| R12 | Identity reads always require their owner; no flag; explicit `neutral` on rate and mode reads | **APPLIED** (MIN-13, NIT-12) | <ul><li>No `required` field remains in any spec; the lint refuses one (`lint_odd.py:482`).</li><li>NOTE lines print each spec's required owners: M7 requires M2; T1 requires M2 and M7; M2 prints none.</li><li>**M2 0.2.1 has 0 foreign reads.**</li><li>The S6/S7 identity reads leave M7 and T1 (`vow.vower`, `vow.status`, `artifact.bearer`, `artifact.location`).</li><li>M7's `person.named` becomes a mode read, `neutral = false`, with an `[isolation]` forcing (`:825`). `false` is a real state ("nobody holds a mind"), not a MAJ-3 impossible one. Accept it explicitly.</li><li>`real_specs.rs` registers M2, M7 and T1 with the S6, S7 and M6 reads reported.</li><li>**So M7 and T1 are registrable in P1a without S6 or S7.**</li><li>PRJ-002 makes SPR-023 follow SPR-021.</li><li>The substrate rule is keyed on the wrong property (MIN-13).</li></ul> |
| R13 | `key_field`; one `[[applies]]` per command; the `sprint` escape bounded; the nits | **PARTLY APPLIED** (MIN-15, NIT-12) | <ul><li>`key_field` is on all 15 `[[applies]]` entries, checked as a `PersonId` payload field (P5, P6).</li><li>A duplicate entry is an error (fixture `dup_command`).</li><li>NIT-5 to NIT-8 and NIT-10 are done: representability, `*_event` as `EventRef`, duplicate fields and a stated `key` all fail as they should (N1-N4).</li><li>The `sprint` escape ends only when the sprint's document is CLOSED. A sprint with no document never ends: `SPR-999` passes, and SPR-014 itself has no document (MIN-15).</li><li>NIT-9's "13 typed entries" is still at EVO `:75` (there are 15).</li></ul> |
| R14 | `05.1` version at merge | **APPLIED** | <ul><li>`05.1:13`: v1.11, "PR #51 took v1.10", applied with PR #49.</li><li>On current `main` (`e9e0fe3`) the header still ends at v1.10, so v1.11 is still free.</li></ul> |

### 2. New findings

#### MAJ-7. Round 3 tells SPR-014 to make two encoding changes that move the smoke ladder, against R1

- **Where:**
  - `worlds/smoke/systems/X4.smoke_estate.odd.toml:54-55` ("SPR-014 … adds the person to args (no draw reads args: the ladder is unchanged)"). With it: X3, X5 and X7's payloads become `{ row, person }`, and `05.1:69` says "`submit` refuses a command whose key differs from that field".
  - `05.1:68` and EVO `:83` and `:23` (the engine work list): "a timer carries, beside its integer payload, one `EventRef` (`cause`)". EVO `:83` says "hash-neutral", but only about the merge order.
  - Against: EVO §5 `:89` ("no existing baseline moves … a difference is a defect, not a `[C3]` regeneration"), V-INV-15 ("the smoke ladder is unchanged") and PRJ-002 `:153`.
- **Evidence:** A queue and the agenda are plane-A state and appear in the ladder.
  - `rh-snapshot/src/state.rs:150-161` encodes each timer row as fixed fields.
  - `:165-182` encodes each command row, `list_i64(args)` included.
  - The smoke ladder has an `rh-core/agenda.timers` digest at all 1,000 rungs (850 distinct). It has a non-empty `rh-core/engine.commands` digest at 307 rungs: X5's `Polity` sends wait across the year end.
  - I ran two experiments in a second copy (`$R/exp`), each with a release build and the 1,000-year yearly ladder:
    1. The four smoke `submit`s with the person appended to `args`: `BASELINE MISMATCH` at rung 1. 307 `engine.commands` lines and 992 rung lines differ.
    2. `timer_rows` with one `u8(0)` after the payload (what `opt_u64(None)` writes for an absent `cause`): `BASELINE MISMATCH`. 849 `agenda.timers` lines differ, and the final state hash goes from `bef936ec026341a0` to `10512eebeaa73e01`.
  - The SPR-007 hash golden (`crates/rh-snapshot/tests/golden/rh_snapshot_hash_v1.txt`, `bin/gen_hash_golden.py:534`) pins the same timer layout.
  - An empty side table would not avoid this either: `plane_hash_of` (`hash.rs:365`) hashes every table name present.
- **Why it matters:** SPR-014 would get two requirements that contradict each other. Whichever it obeys, it fails R1's acceptance criterion or this text.
- **Fix (text, in this PR):**
  - The key field is carried by `Command::person` and is not duplicated in `args`. The smoke payloads stay `{ row: i64 }` plus the key, or `key_field` may name `Command::person` directly. Strike the X4 comment's "adds the person to args".
  - A timer's `cause` is encoded only when present, so a timer without a cause is byte-identical to v1: for example a trailing field that is written only when `Some`, decoded by row length, and the hash golden stays v1.
  - Or the Architect accepts a `[C3]` regeneration in SPR-014 for these two format changes, and R1's "a difference is a defect" is amended to exclude them.
  - Name the timer `cause` slot in SPR-014's row in PRJ-002 (`:35`), which lists only the command payload rewrite.
- **Confidence:** 9/10 for the `args` comment (measured); 8/10 for the timer slot (a hash-neutral encoding exists, but the text does not ask for one).

#### MIN-13. The substrate rule depends on the role label, not on which systems a world registers

- **Where:**
  - `lint_odd.py:29-30`, `:477-480`: "a substrate (`role = "substrate"`, a system that must run alone: M2 on `solo`) may read as identity only another substrate's column".
  - EVO `:53`, `:117`.
- **Evidence:**
  - `role` is the ODD "higher-level purpose" ("substrate for other systems | drama coupling | stylized-fact producer", `docs/templates/system_spec_odd_r.md:14`). It does not mean "runs alone".
  - The S5 draft is a substrate. The rule tells it to read `person.residence` (S3) and `person.occupation` (M4) "as rate or mode with a neutral". That is MAJ-3's impossible state (one neutral residence for everyone). It becomes an error when S5 is frozen.
  - It also fails its stated purpose. On a trial merge with `main` (`e9e0fe3`), `M1.regions` and `S3.migration` are both `role = "substrate"`. S3 requires M1 and M2, so a substrate already cannot run alone. Planting an identity read of `region.land_cells` (M1) in frozen M2 gives `PASS` (NOTE: "M2.demography requires M1.regions"). Yet `solo` and `hamlet` register M2 without M1.
  - The rule happens to hold today only because M2 reads nothing foreign.
- **Fix:** Check what the rule is for. For each `specs/manifest/examples/*.yaml`, every identity-read owner of every registered system must be registered. Drop the role condition, or keep it as a warning.
- **Confidence:** 8/10.

#### MIN-14. M7's murder re-sample leaves out the input's own day

- **Where:** M7 `:563`: "the day is re-drawn among the month's remaining day starts after the input's instant … (no remaining day start: the timer is dropped and counted, `dropped{M7, MURDER_DUE, resampled_out}`)".
- **Evidence:**
  - A timer due at `(k, Vital)` is still pending at `(k, Inputs)`, and the engine accepts a re-draw at `(k, Vital)`: it refuses only `<= (t, Inputs)` (`engine.rs:970`).
  - "After the input's instant" leaves day k out. An input on the ruler on the murder day therefore always postpones the murder. On the month's last day it always cancels it.
  - The hit itself is kept (unlike M2's S11, which re-draws the hit with q′). Given a hit placed uniformly over days 2 to the last, the exact conditional re-draw is uniform over the day starts at or after `t_in`, which is never empty while the timer is pending.
  - `resampled_out` is outside `05.1:69`'s reasons, and `Engine::resample` counts nothing on `None`.
  - "Keyed as exit_day" contradicts "under `exit_murder`" (`ResampleCtx.purpose` is the timer's purpose).
- **Fix:** Re-draw among the month's day starts **at or after** `t_in`, under `exit_murder`, with k as `exit_day` uses it. Then the drop and `resampled_out` disappear. Say that keeping the hit (unlike M2's S11) is deliberate.
- **Confidence:** 8/10.

#### MIN-15. The `sprint` escape never ends for a sprint without a document

- **Where:** `lint_odd.py:222-229` (`closed_sprints` reads `CODEX/05_PROJECT/SPR-*.md`) and `:262-270`.
- **Evidence:**
  - Probe P1: frozen M7 `PressClaim` with `drained_by = "Hour/Inputs"` and `sprint = "SPR-999"`: `PASS`, one ESCAPE warning.
  - P2 (`SPR-010`, which is CLOSED) fails correctly.
  - The real escape names SPR-014, which has no document in `CODEX/05_PROJECT/` yet.
- **Fix:** A `sprint` must name an existing sprint document, or a PRJ-002 row until the document exists. Otherwise it is an error on a frozen spec.
- **Confidence:** 8/10.

#### Nits

- **NIT-12. Stale text** (it contradicts R9, R11 and R12):
  - EVO-001:
    - `:23` "`required` and `neutral` on `[[reads]]`", "non-required reads";
    - `:73` B1 "`required` per read … `required` on 43 reads";
    - `:75` "13 typed entries" (there are 15);
    - `:83` fact (1) "T1 sends at `Agenda` d", and "D39 records the choice for the director", although D39 is withdrawn (`:25`, `:139`);
    - `:100` MAJ-5 row "**Open:** the Architect's ruling on (b)";
    - `:105` the old counter names;
    - `:126` V-INV-04 "a required read … a non-required read";
    - `:131` "a foreign read that lacks `required`".
  - PRJ-002 `:161-162` ("`required`"; T1 "undated `Marry` and `ScheduleDeath`").
  - T1 `:984` "M7 reads person.named … only as identity".
  - M7 `:803` (S7 "in": "drops the read and the neutral applies"; the read is gone, and as an identity read it would be refused).
  - `real_specs.rs:778` "R6 (b)".
  - **Forward note:** under R12, M7's P1b dormancy reads of `vow.*` as identity would make every M7 world require S7, and S7 requires S6 and T1. M7's own `[isolation]` run (T1 stubbed) would then be impossible. Decide in P1b whether to read them as mode reads.
- **NIT-13. Main has moved** (`e9e0fe3`, PR #52, SPR-018).
  - `git merge-tree` reports conflicts in `PRJ-002`, `check_all.sh`, `tests/lint_fixtures/README.md` and the two `odd_applies` M3 fixtures.
  - Main moved the fixtures off `M1.regions`, but 12 of this PR's fixtures (`odd_applies/{bad_type,dup_command,outcome_untimed,unbound_drain}`, all 8 `odd_neutral/*`) are `M1.regions.odd.toml`. On the merged tree each is refused with "system M1.regions already has a spec", so their gates fail for the wrong reason.
  - After the merge, `lint_odd.py` passes on the real specs (8 specs, M1 and S3 are drafts).
- **NIT-14. Lint gaps outside R9's wording:**
  - A sender of an outcome command with no binding in the drain's phase passes (P3: M7 without `Day/Vital`; P8: T1 without it). The engine would raise `SimError::Timer` at the first fire. SPR-014 could check at registration.
  - `05.1:71` says a mode read's neutral is "named by the read's in-coupling prose", but the lint checks this only for rate reads, and M7's `person.named` has no in-coupling with that column.
  - `outcome = false` with senders still naming `timer` passes silently (P15).
- **NIT-15. M7 does not keep its `MURDER_DUE` id.**
  - `exit_timer` stores only the deposition timer (`:694`), so `on_death` cannot cancel a pending murder when the ruler dies or loses the title earlier in the month.
  - The fire then submits for a dead ruler, which `rh-core` drops (`dead_key`, counted under M2), or it kills an ex-ruler, which V-SF-06 would count as a ruler murder.
  - `05.1:69` says a timer firing for someone dead is dropped with `drop_timer`, but neither M7 nor T1 says it checks before submitting.
  - The frozen 0.3.1 text could not cancel either, so nothing is lost. Under (b), though, M7 owns the timer and can.

### 3. The R9 trace

**Order.**
- `05.1` §4.2: Inputs, …, Vital, Activity, Contagion, Polity, Military, Agenda, Culture, Bookkeeping.
- `Vital` = [M2, M7, T1] (`real_specs.rs:780`). M7 and T1 are at level 1 after M2 (`same_instant` reads), with M7 before T1 by id.
- The timer pass runs before the binding pass: each owner once, at the level of its first binding in the phase (`engine.rs:844-860`).
- Commands submitted in the timer pass merge at the end of their level, so the binding pass's level 0 (M2's `Day/Vital` drain) sees them.

**T1, foreground murder (`DEATH_DUE`).**

| When | What |
|---|---|
| (d, `Agenda`), T1 `Day/Agenda` | `attack_outcome` or `break_outcome` hits; T1 emits `MURDER_ATTEMPT` (a `DraftRef`) and schedules `DEATH_DUE` at (d+1 start, `Vital`): entity = victim, payload = actor, cause = the draft. It is strictly after (d, `Agenda`), so it is accepted. The level merge assigns the event's id, then the timer's `seq`, and rewrites the cause (events before timers, `05.1:87`) |
| (d+1, `Inputs`) | An input scoping the victim: `pending_for(victim)` finds the timer; `T1::resample_timer` returns its own tick; the engine checks `(d+1, Vital) > (d+1, Inputs)` and re-inserts it, payload and cause kept |
| (d+1, `Vital`) timer pass | Level 0: M2's own timers (a natural death of the victim would land here). Level 1: M7's fires, then T1's: submit `ScheduleDeath {victim, murder, actor, cause_event}`, keyed `(d+1, Vital, victim, seq)` |
| (d+1, `Vital`) binding pass | Level 0: M2 `Day/Vital` takes the inbox. `rh-core` drops the command if the victim is already dead (`dead_key`); otherwise `apply_death`, and `DEATH` at (d+1, `Vital`). Level 1: M7 (empty), T1 grief |
| (d+1, `Polity`) | M7 `on_death` arbitrates |

**M7, background murder (`MURDER_DUE`).**

| When | What |
|---|---|
| (day 1, `Polity`), M7 `Month/Polity` | `exit_murder` hits; `exit_day` gives k ∈ [2, last]; `murder_culprit` draws the culprit. M7 schedules `MURDER_DUE` at (k, `Vital`): entity = ruler, payload = culprit. It merges at the level's end |
| (k, `Inputs`) | An input scoping the ruler: `M7::resample_timer` re-draws the day (MIN-14) |
| (k, `Vital`) timer pass | Level 1: M7's fire submits `ScheduleDeath {ruler, murder, culprit, None}` |
| (k, `Vital`) binding pass | Level 0: M2 drains and applies; `DEATH` at (k, `Vital`). This is the drawn day, as in the frozen text |
| (k, `Polity`) | `on_death` arbitrates, the same instant (V-SAGA-19) |

**Answers.**
- **Day lost?** No. Both deaths apply in the same `Vital` slot as in the frozen 0.3.1 timing. M7's `Day/Vital` binding exists only so that its timer pass has a level. Its step is empty, and the spec now says the fire must not be done in it (`:542`), which settles NIT-5.
- **Applied before the sampled instant?** Never. T1 applies at d+1 > (d, `Agenda`); M7 at k ≥ 2 > (1, `Polity`). A re-draw is refused if it falls at or before (t, `Inputs`).
- **A timer from the instant it was sampled (DET-13)?** Yes for both. The timer is inserted by the step that drew the outcome and enters the agenda at that level's merge. No `Inputs` phase falls between the draw and the agenda. V-DET-21's "never queued across an `Inputs` phase" holds, because the command exists only between a `Vital` timer pass and the same `Vital`'s drain.
- **Does a victim-scoped input reach it (ADR-0025 item 3)?** Yes. Both timers' entity is the victim (T1) or the ruler (M7). `MARRY_DUE`'s entity is `a`, so an input that scopes only `b` does not reach it. That is harmless: the re-sample is the timer's own tick, and M2's drain drops a `Marry` with a dead `b`.
- **T1's own-tick re-sample.**
  - It is a re-draw from a distribution with a single value, not a way round ADR-0025. The date is a fixed one-day delay, so nothing about it was drawn.
  - The draw (success) was made at d and is already in the log as `MURDER_ATTEMPT`. Under ADR-0025 item 4 (the earlier tick wins), the d decision takes precedence over a d+1 input.
  - This is not the case round 2 objected to. Shape (a) froze a *hazard-drawn* date (M7's murder) so that no input could re-draw it. Under (b), M7's drawn date is re-drawn by M7 (R10).
  - The rule should be written down, so that "own tick" cannot be used later for a drawn date. Add one sentence to `05.1:68`: *a timer whose time is not drawn (a fixed delay after a logged decision) re-samples to its own tick; a drawn time is re-drawn.*
- **Within-instant order (EVO `:85`).** It is correct as written:
  - A natural death in M2's timer pass comes before any commanded death.
  - For an M7 and a T1 murder of one victim on one day, the keys are equal up to `seq`, so M7's (merged first) is applied and T1's is dropped by M2 as "killed earlier in the same drain".

**Recommendation: accept R9 as built, confidence 8/10**, with:
- MAJ-7's encoding sentences, so that SPR-014 can keep the smoke ladder unchanged;
- MIN-14's "at or after `t_in`";
- the one-sentence own-tick rule above.

The 2/10 against is cost: a binding M7 needs only so that its timer pass has a level, two T1 timer kinds, and a timer `cause` slot in plane-A state, where shape (a) put the same information in an M2 pending table. None of these changes timing or a requirement.

### 4. Lint probes

Probes were planted in copies of `specs/systems` and run with `--specs` (scripts `$R/probes/run_probes*.py`). The M2 → M1 probe ran on the trial-merged tree.

| Probe | Planted | Result | Expected |
|---|---|---|---|
| P1 | `sprint = "SPR-999"`, undeclared `drained_by` (frozen M7) | PASS, ESCAPE warning | FAIL (MIN-15) |
| P2 | `sprint = "SPR-010"` (CLOSED) | FAIL | FAIL |
| P3, P8 | M7 or T1 without a `Day/Vital` binding | PASS | gap (NIT-14) |
| P4 | T1 `DEATH_DUE` under an undeclared purpose | FAIL | FAIL |
| P5 | `key_field = "cause"` (not a `PersonId`) | FAIL | FAIL |
| P6 | `key_field` missing (frozen) | FAIL | FAIL |
| P7b | `outcome = "yes"` | FAIL | FAIL |
| P9 | identity read with a `neutral` | FAIL | FAIL |
| P10 | frozen M2 identity-reads T1 | FAIL (substrate rule) | FAIL |
| M2 → M1 (merged tree) | frozen M2 identity-reads M1 (a "substrate") | PASS | FAIL (MIN-13) |
| P11, P13 | lower-case or missing `timer` | FAIL | FAIL |
| P12 | `outcome = true` on `PressClaim` | FAIL (T1 lacks timer and purpose) | FAIL |
| P14 | enum neutral outside the codes | FAIL | FAIL |
| P15 | `outcome = false` on `ScheduleDeath` | PASS | not checkable (NIT-14) |
| N1 | `neutral = 0.00001` in q8_8 | FAIL (not representable) | FAIL |
| N2 | `cause_event: u64` | FAIL | FAIL |
| N3 | repeated payload field | FAIL | FAIL |
| N4 | `key` missing (frozen) | FAIL | FAIL |

### 5. Checks run (in `$R/src`)

**Smoke ladder**
- `cargo build --release --locked -p rh-cli` with `CARGO_TARGET_DIR=$R/target`, toolchain 1.97.0: built in 1 m 40 s.
- `rh-cli run smoke --years 1000 --ladder yearly`: last rung state hash `bef936ec026341a0`.
- `bin/check_baseline.py`: `BASELINE OK`. `cmp`: byte-identical to `worlds/smoke/baselines/smoke_yearly_1000y.ladder`.

**Lints and skeletons**
- `python3 -I specs/systems/lint_odd.py`: `PASS: 6 system specs linted (16 warnings)`, 5 NOTE lines; M2 has none.
- The smoke overlay lint: `PASS: 7 (0 warnings)`.
- `bin/gen_skeletons.py --check`: rh-core 6 up to date; smoke 7 up to date.
- The stale control fails on `M2_demography.rs` only.
- The 25 `check_all.sh` gates from "System specs (lint_odd)" to the smoke stale control (`check_all.sh:761-785`), run with check_all's own `run`: all PASS.

**Python** (with `python/.venv`)
- `pytest python/tests/test_check_all_inventory.py`: 12 passed.
- `ruff`: all checks passed. `mypy`: no issues in 40 files.

**Rust**
- `cargo nextest run -p rh-core -p rh-registry -p rh-sys-smoke --locked`: 144 passed.

**Experiments** (MAJ-7; `$R/exp`, `CARGO_TARGET_DIR=$R/target_exp`): both `BASELINE MISMATCH`, as described in MAJ-7.

**Trial merge** with `origin/main` `e9e0fe3` (`git merge-tree`, objects only; tree `926c4eb`, extracted to `$R/merged`): 5 conflicts. The lint passes on the real specs; the M1-based fixtures collide (NIT-13).

### 6. Not run

- `bin/check_all.sh` (either part), clippy, cargo-deny, the full workspace `nextest`, doctests, the nightly tiers, the perf and allocation gates.
- The smoke run at `--threads 4` / `--require-parallel 5`, and `rh-golden`.
- SPR-014's behaviour (drain, drop, re-sample counters). Nothing is built yet, so R9 was checked by tracing the engine and the specs.
- PR #50 (births) against R9.


## 4. Round 4 (2026-10-17, `9be14c4`), verified by the Architect
R15 to R19 are applied:
- **R15:** `key_field = "person"` names `Command::person`, and smoke's payloads are back to `{ row: i64 }`; an empty timer `cause` adds no bytes to the hashed encoding.
- **R16:** the substrate rule is advisory, and registration enforces it.
- **R17:** M7's murder re-sample is uniform over the remaining day starts, and rolls to the next month when none remain.
- **R18:** `SPR-999` is refused. The Architect accepts the drafter's reading (7/10): "exists" means a document under `CODEX/05_PROJECT/` or a PRJ plan row until the document lands.
- **R19:** the nits.

The Architect re-ran on `9be14c4`: `lint_odd` (8 specs), the smoke overlay lint (7 specs), both `gen_skeletons --check` runs, `gen_manifest --check`, and pytest (619 in a git worktree). `05.1` reads v1.10, v1.11, v1.12. The diff to `crates/` since `757141b` is one test comment. The drafter's full tier at `757141b` passed 337 checks; `--part core` at `9be14c4` passed 284.
