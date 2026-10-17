---
id: REV-EVO-001
title: "Independent review of EVO-001, lifecycle queues and absent owners (PR #49)"
type: record
status: OPEN
owner: architect
agents: [architect]
tags: [verification]
related: [EVO-001, PRJ-002, SPR-014, BCK-002, REV-P1a, REV-SPR-017]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF:** Round 1 on PR #49 at `6bc202e`: **not approvable as written. 1 blocker, 5 majors, 6 minors, 4 nits.** The lint, the fixtures and the gates work. The proposed `05.1` §2 rule-2 text has the problems. The blocker: the smoke world does send lifecycle commands, including one keyed by the person who has just died, so the dead-person drop as written moves its golden baseline. The Architect's rulings are in §0. The drafter revises, and a round 2 re-checks. The review ran in a session on a different model from the drafter's.

# REV-EVO-001: independent review of EVO-001

## 0. Architect rulings, round 1 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R1 | BLK-1 | Accepted. The dead-person drop applies only in a world with a ledger (`person.id` registered), and only to commands whose `[[applies]]` entry declares that its key person must be living (an explicit opt-in; the default is no drop). Smoke's death-`Settle` keeps flowing. SPR-014's acceptance gains "the smoke ladder is unchanged". Correct EVO §5 and the PRJ-002 line that says P1.0 sends no lifecycle command | 8/10 |
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
