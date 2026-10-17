# The event envelope v0: roles, anchored snapshots, typed edges and provenance of change

*Status: v0, a C2 contract for WP P1.9 (SPR-020), drafted for the Architect on 2026-10-17; it reconciles the role codes of `specs/chronicle/schema.sql`, the Registry `[event_roles]` and channel A v0 (PRJ-002 §10 item 2; SPR-011 Blocker 6). Binding texts: `05.2` §4, §4.0 and §4.1a; `05.1` §2 rules 2 and 4 (the level merge, `DraftRef`, `EventRef`) and §6 item 6 (`state_hash`, the running event-log hash); `05.4` §2 (anchored reads `attr@eN`); `05.10` §4–5; `05.11` §0 and §3 (what plane C derives); ADR-0024, ADR-0025, ADR-0034, ADR-0035 §4a and §7a; `docs/03` DATA-2, DATA-3, DATA-5, SAGA-10, SAGA-12, SAGA-15; `docs/09` V-DATA-07, V-DATA-08, V-DATA-09, V-SAGA-15, V-SAGA-17, V-SAGA-21. Machine-readable twins: `schema.sql` (the chronicle), `specs/stream/channel_a_v0.sql` (the sealed segments), `specs/stream/canonical_v0.md` (the text form). Self-test: `python -I specs/chronicle/selftest_envelope.py`. The two sentences marked **D38** depend on the director's ruling on DATA-5 (`05.2` §4.0); the rest fits either answer.*

## 0. What this contract fixes, in one paragraph

An event is what a system emitted at the level merge: its kind, its **cast** (role rows: who played which Registry role), its **anchored snapshots** (the attribute values the emitter read at emission, keyed by role entity), its **typed causal edges** (each to an earlier event, with a kind from DATA-2, a motive from the Registry `[motive_edges]` when the kind is `motivated_by`, and a fixed-point weight and evidence), and its integer payload. Everything in it is plane A, enters the running event-log hash and therefore `state_hash`, and is written to channel A exactly as emitted. Nothing perceptual is in it (ADR-0035 §6, §7a): time of day, lighting, acoustics, inferred emotion, prop status and phrasing are derived in plane C from these fields (`05.11` §3 stage 3). Damage and transformation state carries its cause `EventId` inside the value (SAGA-15), and "what last happened here?" is a plane-B index over the chronicle (§7). One role vocabulary serves the chronicle, the stream, the Registry, the system specs and the sifting patterns (§1).

## 1. The role vocabulary (the reconciliation)

**Ruling asked of the Architect, recommended at 8/10.** The one vocabulary is the Registry `[event_roles]`, in list order: `actor` 0, `target` 1, `witness` 2, `instrument` 3. It is what channel A v0 already writes (`event_role.role`), what SPR-011 ruling 11 fixed for `DEATH` (the deceased is `target`), what the births spec fills (`BIRTH`: `actor` the mother, `target` the child; `MARRIAGE`: `actor` the husband, `target` the wife) and what `05.2` §4 names. `schema.sql`'s `concrete_role` (initiator, target, partner, bystander, instrument) and `abstract_role` go:

| Old | New | Why |
|---|---|---|
| `initiator` (0) | `actor` (0) | The same thing under the Registry's name. The pattern clause key `initiator` becomes `actor` (`specs/sifting/lint_patterns.py`, `patterns.example.toml`, `05.4` §2.2): clause keys are the `[event_roles]` ids, so a pattern binds a role variable to a cast member by the role's own name |
| `target` (1) | `target` (1) | Unchanged |
| `partner` (2) | dropped | A symmetric act names its two parties `actor` and `target` by a convention its verb states (`MARRIAGE`: husband, wife; `DESPAIR_PACT`: the breaking actor, the relation). No P1a verb needs a third party role; one that does (a mediated pact) appends a role to `[event_roles]` then |
| `bystander` (3) | `witness` (2) | The same thing: present, neither acting nor acted on. The knowledge rows of P1b (`knowledge.channel = witness`) are derived from these rows |
| `instrument` (4) | `instrument` (3) | Unchanged in meaning: the S6 artifact used. Its entity kind needs `rh_ids::EntityKind::Artifact` (tag 5), which S6's sprint adds (P1b); until then no event fills it, and channel A's `entity_kind` domain stays 0–4 |
| `abstract_role` | **dropped from plane A** | An abstract role (`authority`, `successor`, `kin`, `rival`, …; Registry `[roles]`) is the *domain* of a pattern's role variable (`roles = { A = "authority" }`, `05.4` §2.1), resolved by the world compiler from titles, kinship and faction, all of which are state with a time: titles are `state_interval`/tenure rows (M7), kinship is the life-course log, factions are intervals. The sifter (plane C) evaluates the domain at the clause event's tick from those rows, as the twin's `title_holder_at` does; storing a plane-C classification per role row would make plane A compute the pattern vocabulary at every emission and cost a byte per row for nothing the chronicle cannot derive. P1a's only pattern (`life_ends`) uses `any`. Alternative, rejected 3/10: keep `abstract_role` as a plane-C sidecar column written by the sifter, which is the same derivation stored twice |

What this changes on disk and in code: `schema.sql`'s `event_role` becomes channel A's child table column for column (`ordinal, entity_kind, entity_id, role`); the chronicle self-test's `ttb_*` views read `role = 0` and `role = 1`; `specs/stream/selftest_channel_a.py` checks that the three child tables of channel A equal `schema.sql`'s column for column (as it already does for `life_course`); `lint_patterns.py` reads the clause keys from `[event_roles]`. Channel A, the canonical form and the `life_ends` twin do not change for roles.

**Multiplicity.** A verb's `[[emits]]` entry lists the roles it may fill (§3). Each listed role appears at most once on an event, except `witness`, which may repeat. An event may fill none (`EXTINCTION`, `INTERREGNUM_CRISIS`: a title is not an entity; the payload names it). A role not listed for the kind is `SimError::UndeclaredRole` at `StepCtx::emit`.

**Canonical row order (the `ordinal`).** Role rows are ordered by role code, then by entity (`entity_kind`, then `entity_id` ascending); `StepCtx::emit` sorts the draft's rows into that order and refuses a duplicate `(role, entity)` and a second row of a non-`witness` role. The ordinal is therefore a function of the set, so two implementations of one spec write the same rows and the same canonical line.

## 2. The `EventDraft` contract

`rh-events` (plane A, machinery) replaces SPR-006's `subjects` and untyped `causes` with the envelope. The shape, as the sprint builds it:

```rust
pub struct EventDraft {
    pub kind: &'static str,                 // the [[emits]] kind, a Registry verb
    pub roles: Vec<RoleRow>,                // RoleRow { role: RoleId, entity: EntityRef }
    pub snapshots: Vec<Snapshot>,           // Snapshot { attr: AttrId, entity: EntityRef, partner: Option<EntityRef>, value: i32 }
    pub causes: Vec<CausalEdge>,            // CausalEdge { cause: EventRef, kind: EdgeKind, motive: Option<MotiveId>, weight: Q8_8, evidence: Q8_8 }
    pub payload: Vec<i64>,                  // as today: meaning per verb
}
// Builders: .role(RoleId, EntityRef) .witness(EntityRef) .snapshot(AttrId, EntityRef, i32)
//           .relational_snapshot(AttrId, EntityRef, EntityRef, i32) .event_snapshot(AttrId, i32)
//           .caused_by(EventRef, EdgeKind) .motivated_by(EventRef, MotiveId, Q8_8 weight, Q8_8 evidence)
//           .after(DraftRef) == .caused_by(EventRef::Draft(r), EdgeKind::Triggered) .with_payload(Vec<i64>)
```

- **Codes come from the Registry, through the skeleton.** `RoleId(u8)` is the index in `[event_roles].ids`, `MotiveId(u16)` the index in `[motive_edges].ids`, `AttrId(u16)` the index in `[attributes]` **sorted by name** (TOML tables carry no order; the same rule as `verb_id`, SPR-011 ruling 11). `bin/gen_skeletons.py` emits them as named constants in the system's generated skeleton (`pub const ROLE_TARGET: RoleId = RoleId(1)`, `ATTR_STRESS`, `MOTIVE_GRIEF_FOR`), from the Registry (or the smoke overlay) at generation time, so no system crate types a code; registration compares the skeleton's constants with the Registry it runs under, as it compares `SCHEDULE`. The segment dictionaries (§5) carry the same indexes with their names.
- **`EventRef`** is EVO-001's: `Draft(DraftRef)` for one of the step's own earlier drafts, `Known(EventId)` for an id read from state (a timer's or command's `cause`, `feud.cause_event`, `artifact.chain_head`). `None` is not an edge. The merge replaces every `Draft` by the id it assigns in the same merge (`05.1` §2 rule 4), then sorts the edges by cause id; `EventLog::append` refuses a draft that still holds a `Draft`, a cause at or above the new id (`ForwardCause`, unchanged), a `motive` on an edge whose kind is not `motivated_by`, and two edges to one cause.
- **`StepCtx::emit` checks the declaration** (the spec table at registration, as `emits_kind` does today): the kind is declared; every role is in the kind's `roles`; every snapshot's `attr` is one of the kind's `snapshots`, its entity is a cast member of the draft (or the event itself, §4), its value is in the attribute's domain; every motive is one of the kind's `motives`. Each refusal is a `SimError` naming the system, the kind and the offending code.
- **What the merge assigns**, besides the id, tick and phase: the admission flag (§6) and nothing else. The merge never adds a role, a snapshot or an edge: an event carries exactly what its emitter put on the draft. The timers and commands the step submitted carry their `EventRef`s as EVO-001 says; the merge rewrites those too, events before timers before commands.

**How a system declares what it fills** (`specs/systems/*.odd.toml`, `[[emits]]`; `lint_odd.py` checks it against the Registry and the spec's own `[[owns]]` and `[[reads]]`):

```toml
[[emits]]
kind = "MENTAL_BREAK"
when = "..."
roles = ["actor"]                                            # [event_roles] ids this kind may fill; witness may repeat
motives = ["GRIEF_FOR", "RETALIATION_FOR", "AMBITION_FOR", "JEALOUSY_OF", "VOW_OF"]   # [motive_edges] ids its motivated_by edges may carry
snapshots = [{ attr = "stress", of = "actor", from = "mind.stress" }]   # [attributes] id, the role (or "event") it is read of, the column it is read from
```

`roles` and `motives` default to none; `snapshots` to none. A missing `roles` is a warning on every spec in this version of the lint and becomes an error on a frozen spec when SPR-020 closes (the frozen specs on `main` declare theirs in this PR: M2 0.2.2, M7 0.3.3, T1 0.3.3, the smoke world's X-specs; the births draft of M2 declares `BIRTH`'s and `MARRIAGE`'s). A wrong value (a role, motive or attribute the Registry lacks; a duplicate; a snapshot of a role the kind does not fill; a `from` the spec neither owns nor reads; a type that does not match) is an error on every spec.

## 3. Roles and edges by system, in P1a

The declarations this PR adds, so that a reader sees the contract in use. A row in italics is a note for the system's next version, not a change to its frozen text.

| System, event | Roles | Edges it carries | Snapshots |
|---|---|---|---|
| M2 `DEATH` | `target` the deceased; `actor` the culprit of a caused death (`ScheduleDeath.culprit`, P1.13), none for a natural one | `triggered` to `ScheduleDeath.cause_event` when the command carried one (the `MURDER_ATTEMPT`); none for a natural death | none |
| M2 `EXTINCTION` | none | `triggered` to the `DEATH` of the same fire (a draft-local cause, as SPR-011 ruling 17) | none |
| M2 `BIRTH`, `MARRIAGE` (births draft) | `actor` the mother / husband, `target` the child / wife | none (the marriage is state, not a cause) | none |
| M7 `SUCCEED` | `actor` the successor, `target` the predecessor | *`triggered` to the death or deposition.* The frozen text says "motive edge"; a motive needs a mind, and M7 has none. Recommend the amendment at M7's next version (SPR-021), 8/10 | none |
| M7 `DEPOSED`, `CLAIM_*`, `SEAT_GRANTED`, `REGENCY_*`, `HOUSE_EXTINCT`, `INTERREGNUM_CRISIS` | as M7 0.3.3 lists them (claimant, ruler, grantee, regent, holder, the last member; none for a title) | `triggered` to the pressed claim, the deposition timer's cause, the death | none |
| T1 `MENTAL_BREAK` | `actor` | `motivated_by` with the motive to the event that pushed stress over the threshold (`GRIEF_FOR` a death, `RETALIATION_FOR` a harm, …) | `stress(actor)` from `mind.stress` |
| T1 `MURDER_ATTEMPT` | `actor`, `target` | `motivated_by` (`RETALIATION_FOR`, `AMBITION_FOR`) to the inciting event; `triggered` to the `MENTAL_BREAK` of the same step when the act is a break act | `stress(actor)` |
| T1 `FEUD_DECLARED` | `actor`, `target` | `motivated_by` `RETALIATION_FOR` to `feud.cause_event` | `stress(actor)` |
| T1 `FLED`, `HOLDINGS_BURNED`, `DESPAIR_PACT`, `FEUD_ENDED` | `actor` (+ `target` for the pact and the feud) | `triggered` to the break; `motivated_by` for an IAUS choice (`05.2` §4: the top contributors, one edge each, `weight` the contribution and `evidence` the margin) | `stress(actor)` |
| S6 `ARTIFACT_*` (P1b) | `actor`, `target` (the receiver), `instrument` the artifact | `contingent` to `chain_head` (SAGA-12: the chain); the act's motive edges on `ARTIFACT_PASSED` | `contaminants(event)`, `bears_artifact(actor)` where S6 reads them |
| S7 `VOW_*` (P1b) | `actor` the vower, `target` the beneficiary or enemy | `VOW_OF` to the oath on every act in pursuit and on `VOW_BROKEN`; `triggered` to the death on `VOW_VOIDED` | `vow_public(actor)` |

The smoke world's dummies fill `target` for their one subject (X3's `X_DEATH`, `CURED`, `BIRTH` the person; X1's `FROST` and X7's `TRADE` the region; X4's `INHERITANCE` the heir) and nothing for `DECREE`, `CENSUS` and `CENSUS_NOTE`; `CENSUS_NOTE` keeps its `triggered` edge to the `CENSUS`.

## 4. The anchored snapshot rule

DATA-3: an event captures the predicate values that rules and patterns read **at emission**; `05.4` §2.1: a pattern reads them as `attr(args)@e`, the value captured in `event_snapshot` for the event bound to `e`; V-DATA-08 compares the stored value with the value at emission. The rule, exactly:

1. **Which attributes.** Exactly the `snapshots` the emitter's `[[emits]]` entry declares for the kind, no more and no fewer (a declared snapshot whose source is absent is absent, rule 4). An attribute not declared for a kind is never on its events, so a pattern read `attr(X)@eN` is satisfiable only when every verb the clause `eN` can match declares `attr` with a role that binds `X`; `lint_patterns.py` gains that check when the first multi-clause pattern lands (P2.1), with the declarations in hand.
2. **Read when, by whom.** The emitter fills each snapshot from its own `SystemView` in the same step, at or before `ctx.emit`: for an own column, the value as the emitter has written it by then; for a foreign column, the value the emitter's declared read returns under its declared lag (`05.1` §2 rule 4). So the snapshot is what the rule read, which is DATA-3's sentence, and V-DATA-08's oracle is the emitter's view, not the end-of-instant column. The engine never fills a snapshot.
3. **Of whom.** `of` names one of the kind's roles (the attribute's first argument is that role's entity) or `event` (the attribute's argument is the event itself: `weather`, `contaminants`, `cell_x`, `cell_y`); a two-argument attribute (`trust(actor, partner)`) names `partner` as a second role. An `event`-argument row stores `entity_kind 0, entity_id 0` (the world; "this event"). An attribute whose argument is *another* event (`knows_event(actor, event)`, `in_realm(event, actor)`) is declared in the Registry but not capturable until the knowledge tables exist (P1b), which fix its encoding then.
4. **Only what is simulated** (`05.2` §4.1a). A snapshot is written only when its source column's owner is registered: registration marks each declared snapshot whose `from` is a dropped read of an absent owner (`05.1` §2 rule 2, absent owners), and `StepCtx::emit` discards such a row silently, counted under `dropped{system, snapshot, absent_owner}`; the neutral value of a read is never a snapshot. No weather before M3, no emotion before P4 (T1 0.3.x: "no emotion snapshot is stored until then"), no guess in plane A.
5. **Encodings** (`event_snapshot.value INTEGER`, i32; the attribute's Registry `type` decides): `q8_8` the raw bits (value / 256, signed, in [−32768, 32767.996]); `bool` 0 or 1; `enum` the index in the attribute's `values`; `int` the integer itself, refused at `emit` outside i32 (`contaminants` is the `[contaminants]` bitset in list order, bit *i* = `ids[i]`; `cell_x`, `cell_y` are cell coordinates). The source column's type must match: `q8_8` ↔ `q8_8`; `bool` ↔ `bool`; `enum` ↔ an `enum` column with the same `values` in the same order; `int` ↔ `u8`, `u16`, `u32`, `i32` or `i64` (the lint checks the pairing; the i32 range at emit).
6. **Canonical order.** Snapshot rows are ordered by `(attr_id, entity_kind, entity_id, partner_kind, partner_id)`, a row without a partner before any with one for the same attribute and entity; `emit` sorts and refuses a duplicate key. The canonical line's `snapshots` list follows the same order.
7. **In P1a** the only declared snapshot is T1's `stress(actor)` on its six acts (table of §3); M2 and M7 declare none. `solo`, `household` and `hamlet` therefore write no `event_snapshot` row, and their channel-A content changes only by the three typed-edge fields of §5.

## 5. Causal edges: kinds, motives, weight, evidence

**Kinds.** `causal_edge.kind` keeps `schema.sql`'s codes, which channel A v0 and the canonical form fixed: 0 `contingent`, 1 `triggered`, 2 `relayed_knowledge`, 3 `motivated_by`, 4 `enabled`, 5 `probabilistic`, 6 `player`, 7 `oracle`, 8 `director`. Their use:

| Kind | Who writes it | When |
|---|---|---|
| `contingent` | a system whose verb declares `references` (Registry `[verbs]`) | only from an explicit unlocking reference (R02 §R2; V-DATA-09): the event names a past event it was enabled by (a `REVENGE` naming the harm; an `ARTIFACT_*` naming `chain_head`) |
| `triggered` | any system | the mechanical consequence of an earlier event in the same causal chain: a timer's or command's `cause` (EVO-001), a draft-local `after`, the death behind a succession |
| `relayed_knowledge` | the knowledge systems (P1b) | the event was possible because of a testimony hop; `evidence` is the fidelity of the hop |
| `motivated_by` | a system with a mind (`T1.minds`) | a utility decision's contributors (`05.2` §4: the top contributors and the margin), one edge per contributor; with a `motive` when the Registry names the kind of motive (§3) |
| `enabled`, `probabilistic` | any system | an enabling condition that is itself an event (a famine behind a riot), or a hazard whose draw this event raised (`evidence` the share of the hazard the cause accounts for) |
| `player`, `oracle`, `director` | `rh-core` at the merge | the event is the direct effect of a journaled input of that origin, when the input itself has an event row (`origin` 1, 3, 4; ADR-0025); never written by a system |

**DATA-2 lists eight kinds and the schema nine** (`oracle`, added with the `oracle` origin on 2026-10-15). V-DATA-09 says "only the eight kinds". This spec keeps the nine codes, because channel A v0 and the canonical form already refuse a kind outside 0–8 and an oracle input is a distinct origin everywhere else; the requirement text is the director's: **D-item 1** (§10). Nothing in P1a writes 6, 7 or 8.

**Motives.** A motive edge is a `motivated_by` edge whose `motive` names a Registry `[motive_edges]` id (index in list order: `RETALIATION_FOR` 0 … `CORRUPTION_BY` 8). `motive` is NULL on every other kind and on a `motivated_by` edge whose contributor is not one of the named motives (an opinion term, a trait term). A kind code is never allocated per motive: the list grows by appending to the Registry, and segments carry its dictionary (§5 of `channel_a_v0.sql`). V-SAGA-15's "every motive edge points to an earlier event" holds by construction (`ForwardCause`).

**Weight and evidence, fixed point.** Both are `Q8_8` raw bits in an i16 (`SMALLINT`): `weight` is signed, the contributor's contribution to the chosen option's score on T1's utility scale (an inhibiting contributor is negative), 0 on every kind but `motivated_by`; `evidence` is in [0, 1] (0 to 256): 1 for a deterministic link (`contingent`, `triggered`, `player`, `oracle`, `director`), the hop's fidelity for `relayed_knowledge`, the attributed share for `enabled` and `probabilistic`, and the decision **margin** for `motivated_by` (how far the chosen option led the runner-up, saturated at 1; SPR-024 states the formula with the IAUS). `append` refuses `evidence` outside [0, 256]. Neither is ever a float (R2 of channel A).

**One edge per cause.** The key is `(effect_id, cause_id)`; a system that has both a mechanical and a motivated link to one event writes the motive edge (the richer one). Edges are ordered by `cause_id` ascending (canonical form).

**`cause_told`** (`05.2` §4.0; **D38**). Every draft gets an id; under the admission rule only admitted events get a row. An edge whose cause was refused keeps the refused id, and the row says so: `cause_told BOOLEAN NOT NULL`. It is **not** part of the event (not hashed: §6); the Chronicle Writer computes it when it writes the `EVENT` record, from an admission bitset it keeps per timeline (host memory, one bit per `EventId`, rebuilt from the sealed `event` tables at resume), so it needs no plane-A memory and no read-back. In every P1a world every draft is admitted (`05.2` §4.0: every P1a world is under the ceiling; P1.0 admits everything by rule), so `cause_told` is `true` on every P1a row whichever way D38 goes; only the kingdom worlds (P1.11) can write `false`.

## 6. What enters `state_hash`

`rh-core/events.meta.running` folds `Event::digest()` of every event in id order (`05.1` §6 item 6; SPR-006 G4). SPR-020 changes the digest to version 2, tag `rehoboam/rh-events/log-hash/v2`, over these words (little-endian `u64`s through `h64_words`, as v1):

```text
id, tick, phase, h(emitter), h(kind),
n_roles,     then per role row:  role, entity_kind_tag, entity_raw
n_snapshots, then per snapshot:  attr, entity_kind_tag, entity_raw, has_partner (0|1), partner_kind_tag, partner_raw (0, 0 when absent), value (i32 sign-extended)
n_causes,    then per edge:      cause_id, kind, has_motive (0|1), motive (0 when absent), weight (i16 sign-extended), evidence
n_payload,   then the payload words
```

So **every stored envelope field enters `state_hash`**: roles with their codes, every snapshot, every edge's kind, motive, weight and evidence. **Two things do not:** the admission flag and `cause_told`, because `05.2` §4.0 fixes that the counter and the running hash do not depend on admission (plane A never branches on the flag; the chronicle is a filter on the stream of ids), and because `cause_told` is a function of admission alone.

**Effect on the baselines (the C3 part of SPR-020).** The digest of an event with no snapshot, no motive and the P1.0 edge shape is still not the v1 digest: a role row now carries its code, and an edge its kind and two zeros. The alternative, a v1-compatible encoding that adds no bytes for `actor` rows and `triggered` edges, was rejected (3/10): it would make the hash blind to exactly the mislabelling the envelope is meant to make visible (a deceased hashed as `actor` would hash as before), and the precedent for a hash-neutral slot (EVO-001 R15) was an *empty* slot, not a default value. Therefore:

- **`smoke` changes**, from its first event (`CENSUS` at the first Month pulse): the bisect (V-DET-11) reports `rh-core/events.meta` as the first divergent table at the first instant with an event, and nothing else diverges (no column, timer, queue or draw moves: the smoke systems emit the same events with the same subjects, now as `target` rows). SPR-020 regenerates `worlds/smoke/baselines/smoke_yearly_1000y.ladder` in its final `[C3]` commit with `bin/regen_baseline.sh`, and its acceptance criterion is that the bisect names only `rh-core/events.meta`.
- **`solo`, `household` and `hamlet`** (SPR-013's baselines, recorded before SPR-020 lands) change the same way: `DEATH` rows gain the `target` code, `EXTINCTION`'s edge gains `kind = triggered`. Their channel-A canonical digests change too (§5 of `canonical_v0.md`: the `EVENT` line gains fields), so SPR-020 re-records them with the same evidence. No draw, timer or column moves on any of the four worlds: a difference outside `events.meta` and the segment digests is a defect.
- The SPR-007 hash golden (`bin/gen_hash_golden.py`) changes for the carried `rh-core/events.held` rows, whose keyframe encoding becomes `u64 id, i64 tick, u8 phase, str emitter, str kind, list(u8 role ‖ entity) roles, list(u16 attr ‖ entity ‖ opt(entity) partner ‖ i32 value) snapshots, list(u64 cause ‖ u8 kind ‖ opt(u16) motive ‖ i16 weight ‖ i16 evidence) causes, list(i64) payload`; the plane-hash layout (`rehoboam/rh-snapshot/hash/v1`) does not change, since only a table's row bytes change and `events.held` is outside `state_hash`. A v1 keyframe cannot be decoded by a v2 reader, so the keyframe format becomes `rehoboam/rh-snapshot/keyframe/v2` (envelope, guard and layers unchanged; recommended 7/10, the alternative being to keep `v1` and let the row-length check refuse old files, which is the same refusal with a less honest name).
- `[event_rows]` and the admission flag (**D38**) change no hash under either ruling, by §4.0's own rule.

## 7. Provenance of change (SAGA-15) and the plane-B index

**The typed value rule.** A column that records damage or transformation (`Ruin { cause }`, `Scar { cause, site }`, a captured or cursed artifact, a burned field) stores its cause inside the value: in the column model of `[[owns]]`, as a sibling column `<column>_cause` of type `u64` (an `EventId`, 0 = none) owned by the same system, declared on the state column as `cause_column = "<entity>.<column>_cause"`. The rule the lint and V-SAGA-21 check: whenever the state column holds a non-default value, `<column>_cause` holds the id of an existing, earlier event (an event the same system emitted in the step that wrote the state, or a `Known` id it read); `rh-core` refuses a write of a non-default value with a zero cause (`SimError::ChangeWithoutCause`, the engine side of SAGA-15). A system with such a column names the event kind that causes each change in the column's `meaning`. `S6.artifacts` already does this (`artifact.condition_cause` for `condition`, `contaminants`, `charge` and a curse); `lint_odd.py` accepts `cause_column` on `[[owns]]` from this PR and checks it names an owned `u64` column of the same entity. **There is no `last_mutating_event_id` column on tiles, settlements or persons**: a place's history is read from the chronicle (next).

**The "what last happened here?" index (plane B).** DuckDB macros in `schema.sql` over the Parquet chronicle, never a plane-A column:

| Macro | Answers |
|---|---|
| `last_event_at_place(tl, place_id, t)` | the latest admitted event at `place_id` with `tick <= t` (`event.place_id`) |
| `last_event_in_cell(tl, attr_x, attr_y, x, y, t)` | the same for a cell, through the `cell_x` and `cell_y` snapshots (their `attr_id`s are the Registry indexes, passed in) |
| `last_event_for(tl, entity_kind, entity_id, t)` | the latest event in whose cast the entity appears (`event_role`), with its role |
| `events_at_place(tl, place_id, t0, t1)` | the window form of the first, for the inspector |

`selftest_envelope.py` exercises them on a synthetic chronicle (§9). A viewer, the inspector and the saga pipeline read them; plane A never does (V-PLN-01).

## 8. The Registry loader (BCK-002 A-24, the envelope part)

`rh-manifest`'s `Registry` (the typed loader; `rh-registry` stays plane A and reads the compiled table) types two more sections, unknown fields refused as for every typed section, and `Registry::problems` gains these refusals, in its stable order:

| Section | Type | `problems` refuses |
|---|---|---|
| `[event_roles]` | `IdList` (`ids: Vec<String>`) | an empty list; a duplicate (the generic rule); an id not `[a-z][a-z0-9_]*`; a list whose first four ids are not `actor, target, witness, instrument` in that order (the codes `schema.sql`, channel A and the `life_ends` twin read as literals 0 and 1; a role is only ever appended, and reordering is a C3 change) |
| `[motive_edges]` | `IdList` | an empty list; a duplicate; an id not `[A-Z][A-Z0-9_]*`; an id that is also a `[verbs]` key (a `tail` line and a pattern name a motive and a verb in the same style, and must never confuse them) |
| `[attributes]` | already typed | nothing changes in the table. The attribute index is its position in the table sorted by name, computed by the loader (`Registry::attr_id(name)`) and written to the segment dictionary |

`lint_odd.py` and `lint_patterns.py` read the same sections from the TOML. The smoke overlay may add none of these sections (an overlay is `[systems]` and `[verbs]` only; the smoke world uses the Registry's roles).

## 9. The byte budget, and the test

`05.2` §4.1a budgets about 35 bytes per event beyond what the event already holds (about 43 with cells and contaminants); V-SAGA-15 gates a **mean stored** overhead of at most 40 (48) bytes per event, measured on the 10k dynastic world over 100 years (D34). Two measures, both stated here so that the gate is unambiguous:

- **Engine bytes** (the `events.held` encoding of §6, which is also what the running hash covers): a role row beyond `actor` and `target` costs 10 (`u8 role`, `u8 kind`, `u64 id`); a snapshot 16 (25 with a partner); a typed edge 6 more than P1.0's bare `u64` (14 in all; 16 with a motive). P1a's fullest event (a T1 act: one `stress` snapshot, one motive edge, no witness) adds 32 bytes over P1.0, inside the budget. The §4.1a figure for emotion (16 bytes for two actors) assumed a packed PAD‖OCC‖intensity value; as five Registry attributes they would cost 80 raw bytes per actor at P4, which is why the stored measure is the gate: if it breaks the budget then, a packed `emotion` attribute type (one row, 8 bytes) is the fix, a C2 change at P4, not now.
- **Stored bytes** (what V-SAGA-15 measures): the Parquet bytes of `event_role` rows beyond the first two per event, `event_snapshot`, and the `motive`, `weight`, `evidence` and `cause_told` columns of `causal_edge`, zstd at the writer's level, divided by the number of `EVENT` records. The reference measure is check 6 of `selftest_envelope.py`, which writes a synthetic 100-year chronicle at the P1a design densities (a quarter of the events are T1 acts with a `stress` snapshot and a motive edge, half of the deaths and successions carry a `triggered` edge, a tenth of the events have a witness, a fifth carry cell coordinates; the densities are constants of the file) through DuckDB's Parquet writer and asserts the mean at or under 40: it measures **3.6 bytes per event**, and **10.8** when every event carries the fullest P1a envelope (a witness, a `stress` snapshot and a motive edge), against the 48-byte ceiling. Dictionary and delta encoding make the stored cost about a quarter of the engine bytes. It is the measure SPR-030's run reports against the same budget.

## 10. For the director (drafted D-items; the Architect numbers them)

| # | Item | Recommendation | Confidence |
|---|---|---|---|
| 1 | **DATA-2's edge kinds.** DATA-2 lists eight kinds; `schema.sql`, channel A v0 and the canonical form carry nine (`oracle` = 7, with the `oracle` origin), and V-DATA-09 says "only the eight" | Amend DATA-2 to nine kinds, adding `oracle` between `player` and `director`, and V-DATA-09 to "only the nine kinds". The alternative retires code 7 as reserved, which leaves an oracle's direct effect typed `player` | 80% |
| 2 | **Abstract roles leave plane A** (§1). `05.2` §4's table named `abstract_role` a key column of `event_role`; this spec derives the domain in plane C. A spec matter, not a requirement: the Architect rules it; listed here because `05.2` §4 changes | Approve the Architect's ruling | 80% |

Nothing else here touches a requirement, an ADR or the roadmap. **D38** is not decided here: §5's `cause_told` and §6's admission flag are the plumbing under either answer, and every P1a row is told either way.

## 11. Alternatives considered

- *Channel A v1 instead of amending v0.* No build writes channel A yet (SPR-013 has not started its code), no segment exists, and the golden canonical file is tool-generated; a v1 would make `rh-stream` carry two formats from its first day. Amend v0 in place, with the file headers saying so. **8/10.** If the Architect prefers the letter of "a change is a new version", the same diff under `rehoboam/stream/channel-a/v1` and `canonical/v1` costs two string constants and nothing else.
- *Engine-filled snapshots* (the merge reads the Registry's attribute sources after the level). Rejected: the value at the merge is not what the rule read (the emitter's lagged view is), which breaks DATA-3's letter and V-DATA-08's oracle; and it puts a Registry-driven column read into `rh-core`. 2/10.
- *Edge kind codes per motive* (9..17). Rejected: a Registry list would move codes, and the canonical form fixes 0–8. 1/10.
- *`cause_told` as plane-A state.* Rejected: a per-`EventId` admission map in the engine is unbounded memory in plane A for a fact the writer already knows. 2/10.
- *A v1-compatible event digest* (§6). Rejected, 3/10, for blindness to mislabelled roles.

## 12. Gates this PR adds (`bin/check_all.sh --part core`)

| Gate | What it checks |
|---|---|
| `Chronicle envelope v0: anchored snapshots, typed edges, cause_told, provenance index, byte budget (selftest_envelope; SPR-020)` | §4 (every snapshot equals the emitter's value at emission and not the column's final value: V-DATA-08 in miniature), §5 (only the nine kinds, `contingent` only on a verb with `references`, one edge per cause, a motive only on `motivated_by`, `evidence` in range, no forward cause: V-DATA-09), §5 (`cause_told` is exactly whether the cause has a row), §7 (every ruin state's cause is an existing earlier event: V-SAGA-21; the four macros agree with brute force on random queries), §9 (the stored mean at or under 40 bytes) |
| its three positive controls | `--control snapshot-drift` (the end-of-run value in place of the emitter's), `--control motive-on-triggered`, `--control cause-later` (a state whose cause is the run's last event): each must FAIL its check by name |
| `lint_odd envelope` controls (12) | `tests/lint_fixtures/odd_envelope/*`: the passing case and the warning case; a role, motive or attribute the Registry lacks; a duplicate non-witness role; a snapshot of an unfilled role; a `from` the spec neither owns nor reads; a mismatched type; an enum column with other codes; a snapshot without a source; a `cause_column` that is not an owned `u64` |
| `lint_patterns` (existing gates) | the clause keys are the `[event_roles]` ids (`actor`, not `initiator`), each bound to a role variable |
| channel A and C self-tests, `canonical_v0 --check` (existing gates) | the amended tables, dictionaries, lines and golden file; channel A's child tables equal `schema.sql`'s column for column |
