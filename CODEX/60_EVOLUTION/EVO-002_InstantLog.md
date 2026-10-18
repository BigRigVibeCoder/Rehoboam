---
id: EVO-002
title: "The instant log and the core genealogy index: this instant's inserted and exited persons and merged events, read-only to steps, filled only at the serial merges, outside every hash; a same-instant event cited in the merge-time form; children by parent kept at the life-course append (05.1 §2 rule 2)"
type: explanation
status: DRAFT
owner: architect
agents: [architect, coder, tester]
tags: [evolution, determinism, simulation]
related: [PRJ-002, BCK-002, EVO-001, SPR-014, SPR-015, SPR-017, SPR-018, SPR-020, SPR-021, SPR-023, SPR-024, SPR-025, SPR-038, REV-EVO-001, REV-SPR-038, ADR-0006, ADR-0007, ADR-0024, ADR-0025]
created: 2026-10-18
updated: 2026-10-18
version: 0.2.0
---

> **BLUF:** Three plans needed to know, inside one instant, what the instant's earlier merges did: which persons were inserted (S3's selector, M7's `on_birth`), which died (M7's `on_death`, T1's grief and feuds) and the `EventId` of another system's event (M7's `SUCCEED` triggered to M2's `DEATH`, BCK-002 A-59). Each had proposed its own mechanism. The P1a shared rulings X1 and X2 give each one builder, and this EVO specifies them as amendments of `05.1` §2 rule 2, **applied in this PR** as EVO-001 was (merging is acceptance; the version is set at merge). **The instant log** is one engine-owned record per instant in `rh-core`: (a) `inserted()`, (b) `exited()`, (c) the merged events of the kinds some system declares in `[[observes]]`, by (kind, target) and by kind (`events_of`, R10), each with its id and cast. It is read-only to steps, filled only at the serial merges (so thread-invariant), visible to a job exactly as the merges before its level left it, cleared at the instant's end and outside every hash, keyframe and journal. A step cites an event another system merged earlier in the instant through **the merge-time form** `EventRef::SameInstant { kind, target }`, which the merge resolves against (c) as the citing job's level started and refuses by name (`absent`, `same_level`, `own_draft`, `ambiguous`). M7 and T1 cite through that form; T1 lists marriages through `events_of`. **The core genealogy index** (children by parent, and a person's life-course row) is kept at the merge of each life-course append, O(births), outside every hash, and read by M7 through SPR-017's host `Indexes`; no system keeps a private copy. An executable model, `specs/engine/instant_log_model.py`, simulates phases, levels, bindings, jobs and merges on eleven scripted scenarios, checks eleven properties against oracles built from the merges' own buffers, fails eight planted controls, and writes vectors that SPR-021a T-02's engine test replays (V-DET-22). SPR-038 stage B builds (a), (b) and the no-re-entry refusal; SPR-021a T-02 builds (c), the merge-time form and the index. No hash moves. Revision 1 (0.2.0) applies the Architect's rulings on the round-1 review (§14). Recommendation 7/10; approver: the Architect (C2); the requirement-text items F-15 and F-16 go to the director under D58 item 6.

# EVO-002 — The instant log and the core genealogy index

| Field | Value |
|---|---|
| Proposed by | the Architect's spec drafter (AGT-001), T-00a, for SPR-021 (and SPR-018, SPR-023, SPR-024, SPR-025, SPR-038) |
| Trigger | new need (BCK-002 A-59: a system learns the id of another system's event emitted earlier in the same instant; A-60: no O(rows) work per instant) and a cross-plan conflict (REV-WAVE7 K1, K27; REV-MINDS M2: three plans, three per-instant mechanisms, no owner) |
| Affects | `05.1` §2 rule 2 (two new parts; the event-typed-slots part; the re-sample rule's exception), rule 4's **Threads** bullet, §4.2's visibility sentence (applied, this PR; version at merge); `specs/chronicle/envelope_v0.md` §2 (the fourth `EventRef` form); `docs/09` V-DET-22 (new) and DET-1's traceability row; `specs/systems/lint_odd.py` (`[[observes]]`; the frozen-default probe, R3); `specs/systems/next/M7.dynasty.odd.toml` 0.6.0 (`[[observes]] DEATH`, the reads of `inserted()` and `exited()`, the merge-time citations; the other shared amendments are listed in its header); `specs/engine/instant_log_model.py` with its transcript and vectors (new); the engine: `rh-core` (`InstantLog`, `StepCtx::inserted`, `exited`, `instant_event`, `events_of`, the genealogy index), `rh-events` (`EventRef::SameInstant`), registration (`AccessSpec::observes`); SPR-021a T-02, SPR-038b T-07 (to this text if it merges first), SPR-018 T-03, SPR-023, SPR-024, SPR-025 (consumers) |
| Change class if accepted | **C2** for this PR (contract text, a lint schema, a model, a draft spec; no hash moves). **C2** for the builders (SPR-038b, SPR-021a): no `state_hash`, hash layout, keyframe, journal, phase-order or RNG change; the four ladders byte-identical |
| Approver | the Architect (X1, X2; C2). The director: F-15 and F-16's requirement texts only (D58 item 6), listed in §13 |

## 1. Problem

**What the contract says.** An `EventId` exists only after a merge (`05.1` §2 rule 4): a step names an event it emitted by a `DraftRef` and an event it read from state by `EventRef::Known` (rule 2, "Event-typed slots"). Events are plane A and their ids are assigned in emission order (ADR-0024 rule 1; ADR-0025 item 2: "`EventId` order = emission order = `(tick, phase, seq)`"). Nothing gives a step the id of an event **another** system merged earlier in the same instant, and nothing names the persons who entered or left the ledger in this instant.

**What the code does** (SPR-014's branch, `origin/sprint/SPR-014-lifecycle-queues`; lines at `921d372`, which are also the branch head's for these four files):
- `Engine::merge` (`crates/rh-core/src/engine.rs:1369–1442`) assigns each draft its id as it appends it (`:1385–1400`), resolves a timer's `cause` (`:1403–1415`) and a command's event-typed slots (`:1417–1433`) against the ids of the same job only, then inserts the job's ledger rows (`:1436`, `insert_rows` `:1447–1513`, the duplicate-id refusal `:1490–1495`).
- `run_level` (`:1206–1312`) runs a level's jobs, puts their owned columns back (`:1286–1288`), checks the life-course guards (`:1301–1305`) and merges the jobs' buffers in job order (`:1308–1310`): no merge happens while a level's jobs run.
- `StepCtx` (`crates/rh-core/src/system.rs:517–531`) lends a job its inbox, the agenda as the level started and its buffers; `emit` (`:966–982`) admits draft-local causes only. No read of events exists.
- The person ledger and the life-course log are kept by the core (`crates/rh-core/src/ledger.rs:1–33`); an instant starts with no row at `person.alive = 0` (compaction at `Bookkeeping`, `engine.rs:876–882`; the invariant, `ledger.rs:24–28`).

**What P1a needs:**

| # | Need | Who |
|---|---|---|
| (a) | This instant's newborns, without a scan | S3's selector (SPR-018 T-03, replacing R4's per-day scan), M7's `on_birth` (SPR-021a T-03) |
| (b) | This instant's dead, without a scan of `person.alive` and `death_tick = now` | M7's `on_death` (SPR-021b T-04), T1's death path (SPR-023 row 6; SPR-025 row 2) |
| (c) | The id of M2's `DEATH` in the same instant: `SUCCEED`, `INTERREGNUM_CRISIS` and `HOUSE_EXTINCT` are `triggered` to it (`specs/chronicle/envelope_v0.md:83`; M7 0.6.0 INV-M7-02), and T1's grief break carries `GRIEF_FOR` to it (SPR-025 row 3); T1 lists the instant's `MARRIAGE`s (R10) | M7 (SPR-021b), T1 (SPR-024, SPR-025) |
| (d) | Children by parent over the life-course log, through dead kin, for `kin`, `children`, `lineage` (`specs/registry/effect_language_v0.md` §5.1, §5.3; SPR-017's host `Indexes::children`, `crates/rh-registry/src/effect/host.rs:41–46` on `origin/sprint/SPR-017-effect-language`) | M7 (heirs, regents, dormancy distance), T1 and M2 where they want it |

The three plans' mechanisms (REV-WAVE7 §4): SPR-038's `inserted()`; SPR-023's `exits()`/`entries()`; SPR-021's revision-0 EVO-002 (per-system hook state, and lagged foreign inputs to a derived index); SPR-025's M2 `event_ref` column; M7's private children index (`next/M7` 0.5.0, the `[[reads]]` comment). X1 and X2 replace them with one record and one index.

## 2. The instant log (`05.1` §2 rule 2, "The instant log"; applied)

### 2.1 Shape

`InstantLog` is a field of the engine, not of `EngineState`. It has three parts:

| Part | Holds | Order | Accessor (`StepCtx`, read-only) |
|---|---|---|---|
| (a) | the `PersonId` of each ledger row inserted at the merges so far this instant | ascending (the ledger's order), kept by a sorted insert at each row insert | `inserted() -> &[PersonId]` |
| (b) | each person whose `person.alive` went from 1 to 0 at a merge so far this instant | ascending by `PersonId` **across the instant** (SPR-038 revision 2, T5; R9), kept by a sorted insert | `exited() -> &[PersonId]` |
| (c) | each merged event of a kind that some registered system observes (`[[observes]]`, §5): its `EventId`, kind, emitter, phase and cast (the role rows as the merge appended them, `envelope_v0.md` §1's canonical order) | merge order, which is id order | `instant_event(kind, target: Option<EntityRef>) -> Result<&[EventId], SimError>`: the ids of the events of `kind` whose `target` role is `target`, ascending; `events_of(kind) -> Result<&[InstantEvent], SimError>`: every event of `kind`, in merge order, each with its id and cast (R10) |

The target is an `Option<EntityRef>`, in the read and in the merge-time form alike: a kind with no `target` role (`INTERREGNUM_CRISIS`: a title is not an entity, `envelope_v0.md` §1) is keyed by (kind, `None`). A step that names a kind its system does not observe is refused at the step, `SimError::UndeclaredObserve { system, kind }`, as `emit` refuses an undeclared kind.

### 2.2 Filling: only at the serial merges

At the merge of each level, for each job in canonical order (rule 4), in the merge's existing order:
1. **(c)** as each of the job's drafts gets its id (`engine.rs:1385–1400`), when the draft's kind is in the **observed set** (the union of the registered systems' `[[observes]]`, fixed at registration; an unobserved kind is never recorded, so a world with no observer pays nothing);
2. **(a)** as each of the job's rows is inserted (`insert_rows`), by a sorted insert;
3. **(b)** after the inserts, **only at a merge whose job took `person.alive` through `write()`** (a step's `write()` put-back sets the written flag, SPR-038 stage A; the model's check 4, SCAN, holds the scan to exactly those merges): the rows at `person.alive = 0` that (b) does not yet list, each by a sorted insert. Every such row went from 1 to 0 at this merge: an instant starts with none (compaction), an inserted row enters alive (M2's `apply_birth`), and every earlier one is listed.

Nothing else writes the log, and no step can. Because a level's jobs merge one after another in canonical order whatever the thread count (`engine.rs:1308–1310`), the log's content at every merge is thread-invariant.

### 2.3 A death and a re-entry in one instant

**Impossible, and refused by name.** Before `Bookkeeping`, the dead row is still in the ledger (it stays readable until compaction, SPR-011 ruling 5), so an insert of the same id is `insert_rows`' duplicate refusal (`engine.rs:1490–1495`). After compaction no P1a binding inserts (every birth is an M2 timer or drain in `Vital`), and the engine refuses an insert of a `PersonId` that (b) lists: `SimError::Ledger { system, "inserted person <id>, who exited this instant: no re-entry within an instant" }`. That refusal is new; it costs a binary search of (b) per insert, and it names the fault before the core's invariant at `Bookkeeping` would (a ledger row with no open life-course entry, `ledger.rs:24–28`). **SPR-038 stage B builds it** with (a) and (b): the Architect adds the deliverable to SPR-038 T-07, with a stub that inserts at `Bookkeeping` after compaction as its acceptance (the round-1 review's MIN-4 and Q2). The model shows both (`death_then_reentry`, `reentry_after_compaction`). **A newborn who dies in its birth instant** is in (a) and in (b); compaction removes its row, not its entries (SPR-038 T5; the model's `newborn_dies`).

### 2.4 Lifetime and planes

- **Cleared at the instant's end**, after `Bookkeeping`'s last level and the invariants (`engine.rs:930–932`), before the `InstantEnd` notice (`:945`); debug builds assert it is empty when an instant starts. A failed instant poisons the engine (SPR-006), and the log goes with it.
- **Outside every hash, keyframe, journal and crash artifact.** It is in no `EngineState` and no plane table. A keyframe is cut at an instant's end, after the clear, so no keyframe could hold it; a crash replay re-runs the instant from its start and refills it.
- **Planes (ADR-0024).** The log is derived from plane A's merges only and is read only by plane-A steps; nothing in plane B or C reaches it. Reading it is reading plane A as it stood at the level's start, so history stays a function of `(engine_build, registry_version, manifest_hash, seed, input_journal)` (`05.1` §6).
- **Cost.** O(1) at an instant where nothing is inserted, killed or observed; (a) O(log n) per insert; (c) O(1) amortised per observed event; (b) one scan of `person.alive` per merge that wrote it (SPR-038's design, at most a few merges per instant, all in `Vital`). It keeps its capacity and allocates nothing in steady state (SPR-038 T-07).

## 3. Visibility: exactly what each step sees

Number the merges of an instant in the order they run: phase by phase (`05.1` §4.2); in each phase the timer pass's levels, then the binding pass's levels; a level split because one system's second binding appears counts as two (rule 4). **A job of the level merged m-th reads exactly what merges 0 to m − 1 recorded.** So it sees:
- everything merged in earlier phases of the instant;
- in its own phase, everything merged at earlier levels: in the binding pass, the whole timer pass of its phase too;
- **never its own level**: not its peers' records (they merge after it ran, whatever the thread count), and not its own (it holds its own drafts, inserts and kills in its own buffers, and names its drafts by `DraftRef`).

The rule needs no watermark for reads: no merge runs while a level's jobs run, so the log a job borrows is the log as its level started. The merge-time form (§4.2) does need one, because it resolves during the merge, after earlier jobs of the same level have appended.

| Reader | Sees in the log | Does not see |
|---|---|---|
| M2's `Day`/`Vital` drain (binding level 0) | the births and deaths of `Vital`'s timer pass (M2's own timers, the senders' `MURDER_DUE`/`DEATH_DUE` submissions) | the deaths its own drain applies |
| M7's, T1's and S3's `Day`/`Vital` bindings (a later level: each reads M2 at `same_instant`, which gives the edge) | everything M2 merged in `Vital`, timer pass and drain | each other's records (one level) |
| M7's `Day`/`Polity` (`on_birth`, `on_death`) | the whole of `Vital`: every birth and death of the instant in P1a, and their `DEATH`s in (c) | — |
| M7's `Year`/`Polity` and `Day`/`Polity` at a year's first instant | `Year` runs first (a split level, the coarser pulse first); `Day` sees `Year`'s merge (K12: a holder dead this instant still holds for the `Year` pass; the model's `year_start` EXPECTs it) | `Year` does not see `Day`'s |
| T1's `Day`/`Agenda` | every merge of `Vital` and `Polity` (M7's successions, if it observed them) | — |
| a `Bookkeeping` binding, after compaction | every entry of the instant; the dead's rows are gone, their life-course rows remain | — |

**To read another system's record in the same phase, a reader runs at a later level**: a `same_instant` `[[reads]]` entry of a column the other system writes gives the edge (rule 4), and `[[observes]]` requires one of the emitter (§5). A reader in a later phase needs nothing. The model checks this rule as sets against oracles built from each job's own buffers (its check 2, VISIBILITY), and every job-order permutation of every multi-job level gives the same transcript (check 9, THREAD INVARIANCE).

## 4. Citing a same-instant event: the read and the merge-time form

### 4.1 The read

`instant_event(kind, target)` and `events_of(kind)` (§2.1) are the primitive: what a job reads is (c) as its level started. A step may use an id it read as `EventRef::Known(id)` in any slot.

### 4.2 The merge-time form (X1's sugar over (c))

`EventRef::SameInstant { kind, target }` (`target: Option<EntityRef>`, as in `instant_event`) is a fourth form of the opaque slot, beside `Draft(DraftRef)`, `Known(EventId)` and `None` (`crates/rh-events/src/event_ref.rs` on SPR-014's branch; `envelope_v0.md` §2 "`EventRef` is EVO-001's", which now names the fourth form). It is allowed in every slot the merge rewrites: a draft's causal edge, a timer's `cause`, a command's event-typed payload field, a `set_cause` slot.

- **Resolution.** At the citing job's merge, the slot becomes `Known(id)` for **the one entry of (c) with that kind and target among the entries recorded before the job's level**: the prefix of (c) whose length the engine notes when the level starts (entries are appended in merge order, so the prefix is exactly the earlier merges'). An event an earlier job of the same level appended in the same merge is outside the prefix; resolving against (c) as it stands at the job's own merge would make a citation depend on canonical order within a level (the model's `late-watermark` control).
- **Refusals,** at the merge, `SimError::InstantCause { system, kind, target, reason }`, fatal like every merge error:
  - `absent`: no entry, and no job of the level emitted one;
  - `same_level`: no entry before the level, but a job of the citing job's own level emitted one (the error names that job; the fix is a `same_instant` read that puts the emitter a level earlier);
  - `own_draft`: the step's own draft matches (a `DraftRef` names it); `own_draft` wins when its own draft and a peer's of the same level both match, so the error points at the step's own fault first;
  - `ambiguous`: two or more entries (the step reads (c) and names one by `Known`).
  At the step, a kind the system does not observe is `SimError::UndeclaredObserve`.
- **It never reaches state.** Every stored slot (the event log's edges, the agenda's `cause`, a queued command's `args`, a cause column) holds `Known` or `None`, exactly as today, so no row layout, hash or keyframe changes.
- **Determinism.** The result is a function of plane A as the level started, so it is the same at every thread count. The cited event merged at an earlier level, so its id is below the citing event's and `ForwardCause` cannot fire (ADR-0025 item 2). Both forms name the same id: the form is sugar.

### 4.3 Which consumer uses which form

| Consumer | What it needs | Form |
|---|---|---|
| M7 `on_death` (SPR-021b T-04): `SUCCEED`, `INTERREGNUM_CRISIS` (a sovereign's, or a vassal's joining its liege's, K2) and `HOUSE_EXTINCT`, each `triggered` to the vacating `DEATH` | a citation M7 never inspects | **the merge-time form**, `SameInstant { kind: DEATH, target: Some(the dead person) }`. M7's dead are exactly (b), and the single death path puts each one's `DEATH` in (c) at the same merge, so `absent` would mean a broken death path, which the merge then names |
| M7 at an interregnum's resolution: `SUCCEED` → the crisis | an event of an earlier instant | `Known`, from the resolution timer's `cause` slot (EVO-001) |
| M7 `CLAIM_CREATED` → the `SUCCEED` or `DEPOSED` of the same step, and its `claim.cause_event` | its own draft | `DraftRef` and `set_cause` (K7) |
| M7 `on_birth` (SPR-021a T-03) | the newborns | `inserted()` |
| T1's death path (SPR-023) | the dead | `exited()` |
| T1's grief break (SPR-025 row 3): the break timer's `cause`, later `MENTAL_BREAK`'s `GRIEF_FOR` source | M2's `DEATH`, at T1's `Day`/`Vital` | **the merge-time form** in the timer's `cause` slot |
| T1's `FEUD_ENDED` at a party's death (SPR-024 T-03) | the `DEATH` | **the merge-time form** |
| T1's marriages (`MARRIED_TO`, SPR-024) | each `MARRIAGE` of the instant, husband and wife | `events_of(MARRIAGE)` (R10) |
| S3's selector (SPR-018 T-03) | the newborns | `inserted()` |
| M2 | nothing: it emits `DEATH`, `BIRTH` and `MARRIAGE` and observes nothing | — |
| `instant_event(kind, target)` | the primitive both forms are defined by | tests; a step that needs an id as a value (none in P1a) |

T1's `[[observes]]` (`DEATH`, `MARRIAGE`) is declared by its own spec PR (X7: T1-I/S). The plan of SPR-021 assumed the read for M7 (its F-1); this EVO assigns the merge-time form, which SPR-021a T-02 builds with (c) (§12, Q1, ruled: +0.5 day for the form, the Rust analogues of the model's controls as mutants, and `UndeclaredObserve` and `InstantCause`; the Architect amends SPR-021, MIN-14).

## 5. Declaring part (c): `[[observes]]`

A system declares each kind it reads in (c), or cites in the merge-time form, in its ODD spec:

```toml
[[observes]]
kind = "DEATH"                 # a Registry verb
owner = "M2.demography"        # its emitter: the verb's emitted_by, another system, which declares it in [[emits]]
why = "..."                    # what the system reads it for
```

`lint_odd.py` checks, as an error on every status (the schema is new: no draft predates it): the kind is a Registry verb (**known kinds only**); `owner` is the verb's `emitted_by` and not the declaring system (a system names its own drafts by `DraftRef`); the owner's spec declares the kind in `[[emits]]`; **the owner is one of the declaring system's reads at `same_instant`**, so the scheduling edge orders the owner's bindings before the observer's in every phase both bind (rule 4) and the identity rule requires the owner in every world that registers the observer (EVO-001 R12); the observer binds a phase at or after one the owner binds (the log is cleared at the instant's end); a kind is observed once; no other field. Fixtures: `tests/lint_fixtures/odd_observes/` (`ok` passes; `unknown_kind`, `wrong_owner` and `no_read` each fail with one diagnostic), and six controls `bin/check_all.sh` renders from `ok` (`odd_observes_control`, as `odd_next_control` renders the `--next` ones), one per remaining rule, each failing with its own diagnostic: a missing `why`, an unknown field, a kind observed twice, the system's own kind, an observer bound only before its emitter's phase, an entry that is not a table (the round-1 review's MIN-8); all in part core. Under `--next`, a frozen `[[observes]]` entry is a keyed section (`observes.<kind>.<field>`): it is protected and cannot be withdrawn, like a frozen read.

**The skeleton's constant** (Q9). `lint_odd.py --rust` emits, for every spec, one `(kind, owner)` pair per `[[observes]]` entry, in the spec's order, the `why` left out as the skeleton leaves out every `why`:

```rust
pub const OBSERVES: &[(&str, &str)] = &[("DEATH", "M2.demography")];   // M7's; an empty slice for a spec with none
```

SPR-021a T-01 adds it with M7's promotion and regenerates the committed skeletons (`gen_skeletons.py --check`). **Registration** (SPR-021a T-02) reads it into `AccessSpec::observes: Vec<EventObserve>`, where `EventObserve { kind: &'static str, owner: &'static str }` sits beside `CommandSend { command, target }` (`crates/rh-core/src/access.rs:137–144` on `main`); it builds the observed set (the union of the registered systems' kinds, a set, so the entries' order changes nothing) and refuses by name an observer every one of whose bindings is in a phase before every binding of the kind's emitter.

## 6. The core genealogy index (`05.1` §2 rule 2, "The core genealogy index"; X2)

- **What.** `children(p)`: every person whose `life_course.mother` or `life_course.father` is `p`, alive or dead, ascending by `PersonId`, each once; and the life-course row of each person (`PersonId` → row), which SPR-017's host needs to read a life-course column "with the person, never a row id" (`host.rs:5–6`). Parents are already columns. **An owner's derived index** (M7's house → title-holders) is not here: other systems read it through the owner's contract crate (`rh-contracts::m7`, built by SPR-021a; R11), as an accessor over lagged columns. The core hosts only the genealogy index (F-27; the round-1 review's MIN-2).
- **Where and when.** In `rh-core`, beside the ledger, over `life_course.person_id`, `.mother` and `.father` (M2's columns, read by the core as compaction reads `person.alive`). It is maintained **at the merge of each level whose job appended life-course rows**, from the appended rows alone (SPR-038 stage B's `append()` and the guard's append base give them), in row order: the row index of the new person, and its id inserted in its mother's and its father's lists (0, unknown, skipped). That is O(births × log k) with k ≤ 64 (`children`'s bound, effect language §5.3). It is built in full when the engine is constructed or restored (back-filled ancestors appended closed at genesis included, `ledger.rs:15–22`) and never saved. Life-course rows are never removed (ADR-0007), so the index only grows.
- **Visibility.** As the log's: a step reads the index as its level started (the model's check 5, GENEALOGY, against a rebuild over the rows merged before the level; the `genealogy-late` control keeps it only at the instant's end and fails).
- **Outside every hash, keyframe and journal**: a cache of plane A. A keyframe round trip rebuilds an equal index (SPR-021a T-02). The model reads a founder parent's children on day 1 and, after the resume from the day-1 keyframe, on days 2 and 3, so its RESUME check compares the index kept through day 1's merges with one rebuilt at the restore (MIN-6).
- **Readers.** M7, through SPR-017's host (`SystemViewHost` over M7's view and the core index; `Indexes::children`); T1 and M2 where they want it, through the same read-only accessor. **No system keeps a private copy**: M7 0.6.0 drops the 0.5.0 comment's own index.
- **Builder:** SPR-021a T-02, its first consumer. SPR-024's row 12 becomes a consumer test.

## 7. What is dropped (X1)

- SPR-021's revision-0 EVO-002 part (ii), lagged foreign inputs to a derived index, until a consumer exists. M2's elite lookup in `house_member` is X3 instead: M7 0.6.0 keeps `house_member` rows in ascending person order (INV-M7-09), and M2's F4/F6 read is a binary search on the lagged column (SPR-015, X13g).
- SPR-023's `exits()` and `entries()`: `exited()` replaces `exits()`; `entries()` has no consumer.
- SPR-025's M2 `event_ref` column: the merge-time form gives T1 the `DEATH`'s id with no M2 change and no C3 on every rung.
- Per-system hook state (SPR-021 revision 0's mechanism, REV-WAVE7 K1 (a)), and M7's private children index (X2).

## 8. Related engine surfaces, by reference

- **`EntityKind::Title` and journal scopes** (REV-WAVE7 K11 (b): "EVO-002 says which"). **No P1a input scope names a title.** The journal's scope entities are a person, a household, a settlement and a region (`Scope::entity` and `Scope::of_entity`, `crates/rh-snapshot/src/journal.rs:242–263` on `main`); a journal row whose scope entity carries the title tag is refused at decode, by name (SPR-021a T-02 tests the refusal). M7's title-keyed timers (the interregnum resolution, about the title) are therefore never re-sampled by an input; M7 0.6.0 states each timer's rule anyway (F-9). **The re-sample exception** (the round-1 review's MIN-9, Q4; applied to `05.1` §2 rule 2 in this PR): a re-sample re-draws what was drawn, except a timer whose drawn time decided an outcome already emitted, which keeps its tick. The interregnum-resolution timer is that case: its delay was drawn at the `INTERREGNUM_CRISIS` it resolves, and the crisis is already in the event log. M7 0.6.0's `scales.restart` states it so, and states no rule for the regency-end timer, which has no `[[rng]]` purpose yet: T-00b declares it, a no-draw purpose like T1's `outcome_delay`, with its re-sample rule (MIN-10).
- **`DropReason`** (the re-check's R14). SPR-038 stage B owns the one change: `Ended`'s generic meaning ("the timer's or command's premise no longer holds"), `Deferred`, `StepCtx::cancel_because` and the `cancelled{system, timer kind, reason}` counter, with the `05.1` sentence. This EVO references it and adds no variant. SPR-021's line widening `Ended` (its K11 (d)) is superseded by R14.

## 9. The `05.1` text (applied: §2 rule 2, rule 4 and §4.2, this PR)

The header gains the version line (the next free number at merge). Rule 2's head names the parts without a count (it said "six" and listed seven). Rule 2, **"Event-typed slots"**, now reads (the added words in bold here only):

> An `EventId` exists only after a merge (rule 4), so a step names an event either by a `DraftRef` for one of its own drafts, by an `EventId` it read from state, **or, for an event another system merged earlier in the same instant, by the merge-time form (the instant log, below)**. Every event-typed slot … is an opaque `EventRef`: `Draft(DraftRef)`, `Known(EventId)`, **`SameInstant { kind, target }`** or `None`. The merge replaces each `Draft` by the id it assigns in the same merge, **and each `SameInstant` by the id the instant log holds**, so the owner only ever sees ids. …

Rule 2 gains two parts, after "Event-typed slots", given in full in `05.1` (not repeated here so the two cannot drift): **"The instant log (EVO-002; P1a shared ruling X1; BCK-002 A-59, A-60)"**, which states §2 to §5 of this EVO, and **"The core genealogy index (EVO-002; X2)"**, which states §6. Rule 4's **Threads** bullet gains one sentence: the merge is where `rh-core` fills the instant log, and nothing a merge records is visible to the level it merges. Rule 2's re-sample sentence gains the exception of §8 (a timer whose drawn time decided an outcome already emitted keeps its tick; no P1a input scope can name a title; MIN-9). §4.2's visibility sentence, which said events of phase *k* reach later phases "only through owned tables" and so contradicted part (c), now reads (the round-1 review's MAJ-2):

> Events emitted in phase *k* are visible to later phases in the same instant only through owned tables, the instant log and the merge-time citation form (§2 rule 2; EVO-002).

`specs/chronicle/envelope_v0.md` §2's `EventRef` sentence names the fourth form, `SameInstant { kind, target }`, resolved at the merge (NIT-3).

## 10. The executable model (`specs/engine/instant_log_model.py`)

A stdlib reference (GOV-003 §3) of the engine **around** the log, not of any system's rules: the phases in `rh_core::Phase` order; in each phase a timer pass and a binding pass, each a list of levels from the `same_instant` edges (rule 4), a level split where one system's second binding appears; a level's jobs run against one view and merge serially in canonical order; the `Bookkeeping` compaction; the state hash at the instant's end. The systems are scripted stand-ins named after the consumers (M2, M7, T1, S3, a `Bookkeeping` reader) and stubs (X1 to X3). The transcript (`specs/engine/golden/instant_log_v0.txt`, 237 lines) shows what each job sees in (a), (b), (c) and the index, every merge, every resolution and refusal, and each instant's state hash.

**The vectors** (`specs/engine/golden/instant_log_vectors_v0.json`, format `rehoboam/engine/instant-log-vectors/v0`; the round-1 review's MIN-5) hold the same runs as data. For each scenario they give:
- its systems (bindings, `same_instant` edges, observed kinds) and its world at creation (ledger, life-course rows, titles, houses, the agenda);
- every job, with the view it saw, the actions it buffered (its scripted drafts, timers, commands, kills and inserts, each cause in its `EventRef` form) and what its merge recorded (the ids assigned, the inserts, the exits);
- every resolution and refusal;
- each instant's log and state hash at its end.

Only `--write` writes either file; `--emit-vectors` rewrites the vectors alone. `--check` reports `STALE` when either file differs from the reference. `bin/check_all.sh` plants an altered state hash in a copy of the vectors, and that copy must FAIL as stale. **SPR-021a T-02 replays the vectors**: stub systems buffer each job's actions, and the engine's views, merges, resolutions, refusals and logs must equal the vectors' (V-DET-22).

| Scenario | What it shows |
|---|---|
| `succession_day` | deaths at two merges, the later one's below the earlier's: P5 and P8 die by M2's timers in `Vital`'s timer pass, and P3, whose `MURDER_DUE` M7's timer pass fires, dies at M2's drain, so (b) goes [5, 8], then [3, 5, 8] (R9; MAJ-1). It also shows:<ul><li>two births in reverse key order (the sorted insert);</li><li>a marriage;</li><li>M7's two-phase `on_death`, with a sovereign crisis, a major `SUCCEED` and a minor vassal joining its liege's interregnum (K2), `HOUSE_EXTINCT` after them;</li><li>T1's grief timer and `FEUD_ENDED` in the merge-time form, and `events_of(MARRIAGE)` with its cast;</li><li>the children of P1 (a founder parent) and of P3 read at `Polity` every day (MIN-6);</li><li>the `Bookkeeping` reader after compaction;</li><li>a keyframe after day 1, and the resume from it;</li><li>the crisis resolved at day 3, the joined vassal after its liege.</li></ul> |
| `same_level_peer` | a step and its emitter in one level: the read sees nothing, and the merge-time form is refused `same_level`, naming the emitter |
| `next_level` | the same pair with a `same_instant` edge: a level later, the form resolves |
| `absent_target` | a citation of a death that did not happen: refused `absent` |
| `ambiguous` | two events of one (kind, target): refused `ambiguous` |
| `undeclared_kind` | a kind outside the system's `[[observes]]`: refused at the step |
| `death_then_reentry` | a death, then an insert of the same id before compaction: the duplicate refusal |
| `reentry_after_compaction` | model only (no P1a binding inserts at `Bookkeeping`): the insert after compaction is refused, re-entry |
| `newborn_dies` | model only: a newborn (P50) who dies in its birth instant is in (a) and (b), and compaction removes the row, not the entries; P45, born at a later merge, enters (a) below P50 (the sorted insert across merges; MAJ-1) |
| `year_start` | M7's `Year`/`Polity` and `Day`/`Polity` split, `Year` first; the dead holder still holds for the `Year` pass (K12), a line the scenario EXPECTs (MIN-7) |
| `world_start_cast` | tick 0: M7's `Year`/`Polity` dormancy step is the world-start naming pass (X4); a member of the manifest's cast who meets no other criterion stays named (M7 0.6.0's criterion (e), membership in `cast_ids`, the cast compiled at load (MIN-11); the Architect's cast ruling of 2026-10-18), a holder genesis did not name is `Promote`d, a named adult meeting none is `Demote`d, and T1 applies both at `Day`/`Agenda` of the same instant (EVO-001's delivery) |

**Checks**, each against oracles built from the jobs' own buffers, never from the log:
1. CLEARED: the log is empty at every instant's start.
2. VISIBILITY: every view equals, as sets, what earlier merges recorded, and every resolution names an id that an earlier merge assigned.
3. ORDER: (a) and (b) are ascending across the instant; (c) is in id order.
4. SCAN: (b)'s scan of `person.alive` runs at exactly the merges whose job wrote it (§2.2). A scan at every merge gives the same (b), so only this check catches it (§2.4's cost).
5. GENEALOGY: every `children(p)` read equals a rebuild over the rows merged before the reader's level.
6. HASH: the state hash reads neither the log nor the index.
7. READERS: removing the pure readers changes no hash.
8. RESUME: a run resumed from a keyframe equals the continuous one, genealogy reads included.
9. THREAD INVARIANCE: every permutation of every multi-job level gives the same transcript (18 orders in `succession_day`).
10. EXPECT: each scenario's outcome, and the lines its script requires (`year_start`'s K12 line).
11. NAMED: the named set a scenario expects at its end.

**Controls**, each of which must FAIL with its own message (`bin/check_all.sh` pins the text):
- `peer-visible`: jobs merge as they finish (THREAD INVARIANCE).
- `unsorted-inserted`: an append in place of (a)'s sorted insert (ORDER).
- `unsorted-exited`: an append in place of (b)'s sorted insert, which `succession_day`'s drain death exposes (ORDER; R9; MAJ-1).
- `log-not-cleared` (CLEARED).
- `hash-includes-log` (HASH).
- `genealogy-late`: the index kept only at the instant's end (GENEALOGY).
- `late-watermark`: the merge-time form resolved against the log as it stands at the job's own merge (VISIBILITY).
- `cast-demoted`: M7's criteria without the cast (NAMED).

`check_all.sh` also runs the vectors' staleness control (above). The model runs in about 0.1 s.

Against revision 1, every mutant of the round-1 review is killed by the self-test:
- the appended `exited()`, by ORDER;
- the index not rebuilt from the rows at a restore, by RESUME's genealogy reads;
- the swapped `Year` and `Day` pulse order, by EXPECT;
- a scan at every merge, by SCAN.

The review's independent oracle finds no mismatch (300 seeds, 39,830 views).

## 11. Hash effect and class

This PR changes text, a lint, a model and a draft spec: no hash moves. The builders change no `state_hash`, hash layout, keyframe, journal, phase order or RNG: the log and the index are outside every hash; the merge-time form is resolved before anything is stored; the re-entry refusal and `UndeclaredObserve`/`InstantCause` are errors on paths no existing world takes. The four default ladders stay byte-identical (SPR-038 T-07's and SPR-021a T-02's acceptance). C2 throughout.

## 12. Options and open questions

| Topic | Option | Consequence | Confidence |
|---|---|---|---|
| (c)'s mechanism | **R (taken, X1):** one engine record, filled at the merges, with the read and the merge-time form | One builder; thread-invariant by construction; no stored state | 7/10 |
| | Per-system hook state (SPR-021 revision 0) | A second record per system; REV-WAVE7 K1 | 2/10 |
| | M2 `event_ref` column (SPR-025's fallback) | Plane-A state for a citation; a C3 on every rung with M2 | 2/10 |
| | A step reads the event log | Unbounded and unindexed; every event of the instant, observed or not | 3/10 |
| M7's citation (Q1) | **The merge-time form (taken)** | M7 never handles an id; a broken death path is refused by name at the merge; SPR-021a T-02 builds the form (+0.5 day, ruled; MIN-14) | 6.5/10 |
| | The read, citing `Known` (SPR-021 F-1's assumption) | No `EventRef` variant; M7 must invent its own error for an empty read | 3.5/10 |
| `events_of`'s entries (Q2) | **The cast (taken):** id, kind, emitter, phase and role rows | T1 reads husband and wife from one entry | 7.5/10 |
| | Ids only | T1 must read `person.spouse` back | 2.5/10 |
| The genealogy index | **Core-owned (taken, X2)** | One copy; restore rebuilds it | 7/10 |
| | M7-private | Three copies once T1 and M2 want one | 2/10 |

**Ruled.** The Architect's round-1 rulings adopt the review's answers to the drafter's nine questions. The numbers below are the review's, from the round-1 hand-off; where 0.1.0 of this EVO numbered a question differently, its number follows in brackets:
- **Q1:** M7 cites through the merge-time form. *Adopted*; SPR-021a T-02 is +0.5 day (MIN-14).
- **Q2** [0.1.0: Q3]: SPR-038 stage B builds the re-entry refusal after compaction (§2.3; MIN-4).
- **Q3** [0.1.0: Q4]: `05.1` is amended in this PR, with §4.2's visibility sentence fixed (MAJ-2).
- **Q4:** the interregnum timer keeps its tick at a re-sample, under an exception clause (MIN-9; §8).
- **Q5:** T-00b declares the regency-end purpose (MIN-10).
- **Q6:** T-00b also states that the persons in `exited()` are skipped by `Promote`, `Demote`, regrant and vocation. M7 0.6.0's header records this for T-00b.
- **Q7:** the requirement texts go to the director as narrowings, honestly put (§13).
- **Q8:** `[[observes]]` is an error on drafts (§5).
- **Q9:** the `OBSERVES` constant's shape is in §5.

## 13. Tests that gate it (`docs/09`), and the director's items

| ID | Status | What |
|---|---|---|
| **V-DET-22** | new (this PR); in DET-1's traceability row (NIT-4) | the instant log and the core genealogy index: on stub worlds at 1, 2, 4 and N threads, (a), (b) and (c) at every read equal oracles from the merges' buffers, with nothing of the reader's level; every resolution of the merge-time form names an earlier level's event and each refusal is named; the log is empty at each instant's start; the state hash, keyframes and journal are equal with and without readers; the index equals a rebuild at every keyframe and after a restore; on the model's scenarios, stub systems replay each job's actions from `specs/engine/golden/instant_log_vectors_v0.json`, and the engine's views, merges, resolutions, refusals and logs equal the vectors' |
| V-DET-01, 02, 05, 12 | existing | unchanged |
| V-SAGA-19 | existing | unchanged (K2 keeps it, M7 0.6.0) |

The spec side gates in this PR, all in `bin/check_all.sh` part core:
- the model, with its transcript and vectors current, its eight controls and the vectors' staleness control;
- `lint_odd.py`'s `[[observes]]` fixtures and its six rendered controls;
- the `--next` lint of M7 0.6.0, with its 34 records in the amendments lock.

**For the director (D58 item 6; not edited here).** Both texts below **narrow** the requirement they amend, and are put to the director as narrowings (Q7): `docs/03` SIM-9 to read "more than N degrees from any title, claim or office **held by a member of their house**" (F-16; M7 0.6.0 states the house-scoped reading in its dormancy criterion (c)); D35, `docs/11` P1.7 and `docs/09` V-INV-03's P1a variant to read "titles, **and the claims the death path creates or voids**" for "titles and claims inherited" (F-15; M7 0.6.0 states it in V-INV-03's pattern).

## 14. Revision 1 (0.2.0): the round-1 review, applied

The independent review of `a3ffe64` returned AFTER SMALL FIXES (0 BLK, 2 MAJ, 14 MIN, 10 NIT). The Architect accepted every finding. This revision applies the drafter's rows; MIN-4's SPR-038 line, MIN-12, MIN-13's A-69 and MIN-14 are the Architect's.

| Row | Where |
|---|---|
| MAJ-1: a later merge's death below an earlier one's; an `inserted()` case across merges; the `unsorted-exited` control | §10 (`succession_day`, `newborn_dies`, the controls); `check_all.sh`; `CLAUDE.md` |
| MAJ-2: §4.2's visibility sentence | `05.1` §4.2; §9 |
| MIN-1: `title.exit_timer` as a 0/1 flag, type u64 (F-3) | M7 0.6.0 (two new records; `on_death` and `arbitrate` re-stated) |
| MIN-2: F-27 | §6 |
| MIN-3: the sections leave the delta one at a time (F-28) | `registry_delta.toml`'s header |
| MIN-4: SPR-038b builds the no-re-entry refusal | §2.3, §12; `05.1` |
| MIN-5: the vectors and their staleness check | §10; `check_all.sh`; `docs/09` V-DET-22 |
| MIN-6, MIN-7: genealogy reads on day 1 and after the resume; K12 EXPECTed | §6, §10 |
| MIN-8: six rendered observes controls | §5; `check_all.sh`; `tests/lint_fixtures/README.md` |
| MIN-9: the re-sample exception | `05.1` §2 rule 2; §8, §9 |
| MIN-10: `scales.restart` states no rule for a purposeless timer | M7 0.6.0; §8 |
| MIN-11: `cast_ids` compiled from the manifest's cast by V-DATA-05's founder index, in `[input_data]` | M7 0.6.0 (`initialization.state`, dormancy (e)); §10 |
| MIN-13: the two `type = "count"` additions named | `next/README.md` |
| Q6, Q9 | M7 0.6.0's header; §5, §12 |
| NIT-1: six re-stated records | `next/README.md` |
| NIT-2: a separator before each 0.6.0 addition in the re-stated reasons | M7 0.6.0 |
| NIT-3: the fourth `EventRef` form | `envelope_v0.md` §2; §9 |
| NIT-4: V-DET-22 in DET-1's traceability row | `docs/09` |
| NIT-6: `own_draft` wins when both match | §4.2; `05.1` |
| NIT-7: the `Option<EntityRef>` target | §2.1, §4.2, §4.3; `05.1` |
| NIT-8: a step's `write()` put-back | §2.2; `05.1` |
| NIT-5, NIT-9, NIT-10 | no change (ruled) |

M7 0.6.0 now carries 34 `[[amends]]` records: the 10 of 0.5.0, six of them re-stated, and 24 new. The lock holds 79 amendments: M2's 45 and M7's 34. The tool regenerated the lock, and `--write` regenerated the goldens.

## Decision

<the Architect's; acceptance is the merge of this PR (EVO-001's precedent, REV-EVO-001 R8 (f)); the director rules only on §13's requirement texts>
