---
id: SPR-020
title: "The event envelope in the engine: roles, anchored snapshots, typed edges, origin and provenance of change (P1.9)"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-002, PRJ-002, REV-SPR-020, EVO-001, REV-EVO-001, SPR-013, SPR-014, SPR-026, SPR-029, SPR-030, ADR-0024, ADR-0025, ADR-0035]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF:** Builds the event envelope v0 (`specs/chronicle/envelope_v0.md`, merged in PR #61) in the engine:
> - the Registry's role vocabulary, typed;
> - `EventDraft`'s level fields, roles, anchored snapshots and typed, coalesced edges, with the event-log digest v2;
> - `emit` checked against each spec's `[[emits]]`;
> - the input's **origin** carried through every timer, command and drain;
> - the `set_cause` slot and its `ChangeWithoutCause` guard;
> - keyframe v2;
> - channel A's envelope tables filled from the event.
>
> **All four golden baselines move** (`smoke`, `solo`, `household`, `hamlet`). They are re-recorded in the final `[C3]` commit, with `registry_version` bumped there (REV-SPR-020 R20). The audit records the measured first divergence and the differing tables at every rung (R23).
>
> - **Developer:** AGT-002-CORE, with AGT-002-TOOLS for `gen_skeletons.py` and `gen_hash_golden.py` (one Developer session may hold both roles).
> - **Testers:** AGT-003-IF and AGT-003-DET (one Tester session may hold both, on a different model from the Developer's).
> - **Size and class:** about 3.5 agent-days. **C3**: the event-log hash is in `state_hash` (R13: "C3 either way"). It contains C2 work: the event schema, three Registry sections and the keyframe format.
> - **Status: awaiting the director's plan approval (GOV-005 §2).** D44 is pending: the sprint builds nine edge kinds, as the spec says, and §"Director items" states what changes under eight. The four baseline moves are announced to the director before the merge (GOV-005 §6; PRJ-002 §9).

# SPR-020 — The event envelope in the engine (WP P1.9)

- **Roadmap item:** `docs/11` P1.9 · **Phase / rung:** P1a, rungs 0–2 and `smoke`
- **Depends on:**
  - SPR-013 merged: channel A written in the amended v0 shape (REV-SPR-020 R6), the `household` and `hamlet` baselines, and `bin/regen_baseline.sh` covering all four worlds.
  - **SPR-014 merged before T-04 and T-05 land.** Those tasks need SPR-014's pieces: `EventRef`, the timer `cause` tail that the origin tail follows, typed payloads (`ScheduleDeath.cause_event`, `.culprit`) and the one-binding drain that `ctx.under` scopes. See "Merge route". PRJ-002's SPR-020 row lists SPR-013 only; this edge is new.
- **Binding contracts:**
  - `envelope_v0.md` §1–§9 and §12;
  - `05.1` §2 rule 2 and rule 4 (v1.13: `set_cause`, the guard, origin);
  - `05.2` §4, §4.0, §4.1a;
  - `specs/stream/channel_a_v0.sql`, `specs/stream/canonical_v0.md` and `specs/chronicle/schema.sql`, as amended in place by PR #61;
  - `specs/manifest/registry_v0.toml`: `[event_roles]`, `[motive_edges]`, `[role_domains]`, `[attributes]`;
  - ADR-0024, ADR-0025, ADR-0035 §7a;
  - `docs/09` V-DATA-08, V-DATA-09, V-SAGA-21 (the engine side), V-DET-11, V-DET-12, V-STR-01, V-GOLD-01.
- **System spec:**
  - M2 0.2.2 (0.2.3 after SPR-014): `[[emits]]` roles (`DEATH`: `target`, plus `actor` for a culprit; `EXTINCTION`: none, with a `triggered` edge);
  - M7 0.3.3 and T1 0.3.3: their `[[emits]]` roles, motives and snapshots are read by registration and `emit` tests only;
  - the S6 draft (`cause_column`, `default`, `event_ref` columns) is used by stub fixtures;
  - the smoke specs' roles (envelope §3, last paragraph).
- **Rulings (binding):** REV-SPR-020 R1–R27; REV-EVO-001 R15 (the `cause` tail SPR-020's origin tail follows).
- **Director items, pending:** D44 (eight or nine edge kinds) and D38 (DATA-5). D38 changes nothing in P1a: every draft is admitted, so `cause_told` is true whichever way it goes (envelope §5).
- **Requirements covered:**
  - DATA-2 (typed edges);
  - DATA-3 (anchored snapshots);
  - DATA-5 (the `cause_told` and admission plumbing);
  - SAGA-10 (envelope fields);
  - SAGA-15 (the engine side: the slot and the guard);
  - DET-1 (the `registry_version` bump);
  - DET-4 (digest v2, thread invariance);
  - DET-7 (input kind → origin).

## Specs this sprint needs

| Spec | Exists? | Gap | Writer | Needed by |
|---|---|---|---|---|
| `envelope_v0.md` | Exists (PR #61, `4e941ef`; `REV-SPR-020` closed). §6's channel-A sentence corrected by the Architect at this plan (2026-10-17): the canonical digests move in exactly the `state_hash` of `STEP_DIGEST` and `KEYFRAME`, `KEYFRAME`'s format and an input-caused `EVENT`'s `origin` | — | — | T-06, T-07 |
| The `event_ref` world-column row encoding | Left to the sprint (§7: "a presence bit beside the id, or `opt(u64)` per row") | `opt(u64)` per row, the encoding `rh-snapshot` already uses for engine tables: accepted by the Architect at this plan (8/10) | — | T-05 |
| `05.1` §2 rule 2: "a keyframe carries it as a nullable column" | Fixed in `05.1` v1.14 (rows are stored as the same bytes, tails included; REV-SPR-020 MIN-1) | — | — | T-04 |
| `docs/09` V-DET-22 (`origin_through_hops`) | Missing: the sprint files it (envelope §12) | The row | The Developer drafts; the Architect accepts | T-04 |
| The `registry_version` number | The sprint bumps it (R20). SPR-015 and SPR-018 bump it too | The next free number at merge (the REV-EVO-001 R14 precedent for `05.1`) | Architect | T-07 |
| D44 (DATA-2: eight or nine kinds) | Pending | Built as nine; nothing built changes under eight (see "Director items") | Director | T-02 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C3**: the event-log digest v2 and the origin tails move `state_hash` on every golden world (envelope §6). It contains C2: the event schema (`EventDraft`, `events.held` v2), three typed Registry sections and the keyframe format v2 |
| Planes touched (ADR-0024) | **A:** `rh-events`, `rh-core`, `rh-snapshot`, `rh-registry` (the compiled Registry table), `rh-stream`, `rh-sys-demography`, `rh-sys-smoke`. **Tool:** `rh-manifest`, `rh-cli`, `bin/gen_skeletons.py`, `bin/gen_hash_golden.py`. **B:** none (the `schema.sql` macros of §7 exist since PR #61 and are exercised by `selftest_envelope.py`). **C:** none |
| Owned columns written | None new by a built system. `event_ref` columns are written through `set_cause` by stub fixtures only: S6 and M7 are not built |
| Reads (via `rh-contracts`) | None |
| RNG purposes added | None |
| Golden-hash effect | **All four baselines move**, in `rh-core/events.meta` (digest v2) and `rh-core/engine.inputs` (the origin tail on queued `OracleMove` rows). `rh-core/agenda.timers` and `rh-core/engine.commands` move only at sub-yearly granularity inside input years (§6, R23). They are re-recorded with `bin/regen_baseline.sh` in the final commit, whose subject ends `[C3]`, and announced to the director before the merge |
| Fixed-point quantities | Edge `weight` (signed) and `evidence` (0–256) as `Q8_8` raw bits in `i16`; `q8_8` snapshot values as raw bits in `i32`. No float anywhere (channel A R2) |
| Determinism lints (GOV-003 §2, §6) | Must pass. No new rule, no new plane-A dependency |
| Tests that gate this sprint | V-DET-01, V-DET-02, V-DET-05, V-DET-11 (the divergence record), V-DET-12, **V-DET-22** (new), V-DATA-08 (in miniature), V-DATA-09, V-DATA-10 (subset), V-SAGA-21 (engine side), V-STR-01, V-WC-04 (the re-pinned manifests), V-GOLD-01 (the new baselines), V-PERF-04, V-PERF-11, and the existing envelope gates of §12, plus `bin/check_all.sh` |
| Docs updated in the same sprint | `bin/check_all.sh` and `python/tests/test_check_all_inventory.py` (same commit); the `CLAUDE.md` checks; `docs/09` V-DET-22 (new), V-DATA-09 and V-SAGA-21 method lines with the test names; `05.0` §6 (keyframe v2); BCK-002 A-24 (envelope part) closed |

**Rules carried from P0–SPR-013 (every task):**
- **Property tests vary world size and seed:** at least three sizes and eight seeds.
- **A positive control fails for the right reason:** its `check_all` line matches the message.
- **A test input exercises the path it claims,** and a probe asserts it: T-04's input lands while its timer is pending.
- **Python tests read no CI environment variable.**
- **No statistical test here.**
- **CI time:** each CI part runs in 12 minutes or less (15 hard), reported.
- **Merge `origin/main` when main moves,** and re-run both parts.
- **Every new gate is in exactly one `check_all.sh` part,** with the inventory test updated in the same commit.
- **Baselines are regenerated only by `bin/regen_baseline.sh`,** in the final commit, with `[C3]` at the end of its subject. Test goldens such as the stub-world hash vectors are regenerated by their own tools, never typed.
- **No new third-party crate.**
- **One commit per task.**

## Tasks

T-01 to T-03 and T-06 depend only on SPR-013 and may start at once. T-04 and T-05 build on SPR-014's surfaces. T-07 comes last.

#### T-01: The typed Registry sections and the skeleton constants (§8, §2; A-24, envelope part; R4, R12 NIT-4, R16, R22, R26)
- **Deliverable:**
  - **`rh-manifest` types three sections,** refusing unknown fields:
    - `[event_roles]` and `[motive_edges]` as `IdList`;
    - `[role_domains]` as `BTreeMap<String, RoleDomain { attribute, relational, owner, column }>`.
  - **`Registry::attr_id(name)`:** the attribute's index in the name-sorted `[attributes]`.
  - **`Registry::problems`** gains §8's refusals, in its stable order.
  - **`rh-registry`'s compiled table** gains the role ids, the motive ids and the attribute names.
  - **The skeleton constants.** `lint_odd.py --rust` and `bin/gen_skeletons.py` emit, per spec, the role, attribute and motive constants its `[[emits]]` uses (`ROLE_ACTOR: RoleId`, `ATTR_STRESS`, `MOTIVE_GRIEF_FOR`), from the Registry or the smoke overlay. Registration compares them with the Registry it runs under, as it compares `SCHEDULE`.
  - The committed skeletons are regenerated.
- **Acceptance:**
  - **One planted Registry per §8 refusal,** each refused by `Registry::problems` with its named message:
    - an empty list;
    - the first four roles reordered;
    - an id of the wrong form;
    - a motive equal to a verb key;
    - a domain key that is not a `[roles]` id;
    - a missing domain entry;
    - a non-`bool` attribute;
    - an arity mismatch;
    - an unknown `owner`;
    - a `column` of the wrong shape;
    - a `none` mismatch;
    - an unknown field.

    Where `lint_patterns.py` reads the same file, it refuses it too (R22 NIT-3).
  - **The real Registry passes.** A test pins `attr_id("stress") = 32` on today's Registry (§6) and shows the index is a function of the sorted names.
  - **Skeletons.** `gen_skeletons.py --check` is green on the real and the smoke specs, and a skeleton whose `ROLE_*` constant differs from the Registry is refused at registration by name.
- **Status:** [ ] not started

#### T-02: `rh-events`: the envelope, total coalescing, the append refusals and digest v2 (§2, §5, §6; R3, R4, R8, R9, R12 NIT-5, R21)
- **Deliverable:**
  - **`EventDraft` as in §2:** `place`, `magnitude`, `secrecy`, `roles`, `snapshots`, `causes: Vec<CausalEdge { cause: EventRef, kind, motive, weight, evidence }>` and `payload`, with §2's builders. It replaces SPR-006's `subjects` and untyped causes. Every call site moves to it:
    - M2: `DEATH` gets `target`; `EXTINCTION` gets a `triggered` edge;
    - the smoke dummies, as envelope §3's last paragraph says (`CENSUS_NOTE` keeps its `triggered` edge).
  - **`EdgeKind`** with codes 0–8.
  - **`Event`** carries `origin` and the admission flag. The flag is not hashed.
  - **The merge:**
    - resolves every `Draft`;
    - coalesces edges to one per cause by §5's total rule:
      - the kept kind is the first in the order `motivated_by`, `contingent`, `triggered`, `enabled`, `probabilistic`, `relayed_knowledge`;
      - the weight is the saturating `Q8_8` sum of the contributors' weights;
      - the motive is the one with the largest |weight|, a tie going to the lower index and an unnamed contributor counting last;
      - the evidence is the largest among the contributors of the kept kind;
    - sorts the edges by cause id.
  - **`EventLog::append` refuses:**
    - a remaining `Draft`;
    - a cause at or after the new id (`ForwardCause`);
    - a motive off `motivated_by`;
    - a non-zero weight off `motivated_by`;
    - an evidence other than 256 on a deterministic kind;
    - two edges to one cause;
    - kinds 6–8, which have no writer in P1a.
  - **Digest v2,** tag `rehoboam/rh-events/log-hash/v2`, over §6's words exactly.
- **Acceptance:**
  - **Golden vectors.** `bin/gen_hash_golden.py`, the independent stdlib reference, gives v2 digest vectors that `rh-events` reproduces bit for bit. They cover:
    - a place present and absent;
    - a negative magnitude;
    - each secrecy;
    - each origin;
    - all four role codes;
    - snapshots with and without a partner;
    - edge kinds 0–5, with and without a motive;
    - a negative weight.

    A one-field mutation of each field kind changes the digest.
  - **Coalescing.** A property test over random contributor sets equals a reference written in the test from §5's text, not from the code. The R21 case (a `contingent` link and a `RETALIATION_FOR` motive to one harm) keeps `motivated_by` with its motive, weight and margin.
  - **Refusals.** Each `append` refusal, planted, is refused by its named error.
- **Status:** [ ] not started

#### T-03: `rh-core`: `emit` checks the declaration (§1 multiplicity and order, §2, §4; R7, R10, R12 NIT-5)
- **Deliverable:**
  - **The spec table carries each `[[emits]]` kind's** roles, motives and snapshots (`attr`, `of`, `from`, `type`).
  - **`StepCtx::emit` checks:**
    - the kind is declared;
    - every role is among the kind's roles, once (witness excepted), with at most 256 rows; the cast is sorted to `(role, entity_kind, entity_id)`, and a duplicate `(role, entity)` is refused;
    - every snapshot's attribute is declared, its entity is a cast member (or the event), and its value is in the attribute's domain;
    - snapshot rows are sorted, and a duplicate is refused;
    - every declared snapshot whose role is filled and whose source is present is on the draft (`MissingSnapshot`);
    - every motive is declared.
  - **Absent sources.** A snapshot whose `from` is a dropped read of an absent owner (SPR-014's rule) is discarded and counted under `dropped{system, snapshot, absent_owner}` (§4 rule 4).
  - **Errors.** `UndeclaredRole`, `DuplicateRole`, `CastTooLarge`, `MissingSnapshot` and the rest; each names the system, the kind and the offending name or code.
  - **M2's `DEATH`** fills `target` (the deceased), `actor` (a drained `ScheduleDeath`'s non-zero culprit) and a `triggered` edge to `cause_event` when there is one (§3).
- **Acceptance:**
  - **Refusals.** One planted draft per refusal, each refused with its named `SimError`.
  - **V-DATA-08 in miniature.** A stub emitter in the shape of T1, with a `stress(actor)` snapshot from an owned column, stores the value it read before its own write for the act, not the column's value at the end of the step. A control emitter that snapshots after its write is caught.
  - **Deaths.** A commanded `DEATH`, from SPR-014's stub sender, carries `target`, `actor` and its `triggered` edge; a natural one carries `target` only.
  - **Absent sources.** A dropped-read snapshot is absent from the event and counted.
- **Status:** [ ] not started

#### T-04: Origin through every hop (§2, §6; R13, R14, R23, R24, R26; `05.1` rule 4 v1.13)
- **Deliverable:**
  - **The kind → origin map:**

    | Input kinds | Origin |
    |---|---|
    | `OracleMove`, `OracleWhisper`, `OraclePierce`, `OracleOmen`, `OracleWard` | `oracle` (3) |
    | `PlayerAction`, `SpeechAct` | `player` (1) |
    | `ForkOverride` | `director` (4) |

  - **The journal's `append` refuses a scope** on `ManifestSet`, `SettingChange` and `ConsoleMode`, by name (R26).
  - **Origin propagation:**
    - `MaterialInput.origin` exists, and the `Inputs` job runs at the input's origin;
    - `Timer.origin` and `QueuedCommand.origin` are inherited from the job that creates them, so a timer re-sampled by an input takes the input's origin;
    - a timer's fire is a job of the timer's origin;
    - `ctx.under(&command, |ctx| …)` scopes `emit`, `submit`, `schedule` and `set_cause` to the command's origin, and `ctx.emit_from(&command, draft)` is its one-draft form;
    - an effect of several commands at once is the job's.
  - **Row tails, exactly as §2 gives them,** each written only when non-zero and decoded by the remaining length:
    - timers: `[, u8 1 ‖ u64 cause] [, u8 1 ‖ u8 origin]` (a remaining length of 0, 2, 9 or 11);
    - commands: `[, u8 1 ‖ u8 origin]`;
    - inputs: `[, u8 1 ‖ u8 origin]`.

    The keyframe stores the same bytes.
  - **`bin/gen_hash_golden.py`** gains the tail cases.
  - **The engine test `origin_through_hops`** in `crates/rh-core/tests/agenda_events.rs`, filed as `docs/09` V-DET-22.
- **Acceptance:**
  - **V-DET-22.** An `OracleMove` re-samples a stub timer; a probe asserts the timer was pending and was re-sampled. Then:
    - the fire's event, the timer the fire inserts, the command that timer submits, the event the drain emits and the command the drain submits under `ctx.under` all carry `oracle`;
    - a drain effect over two commands of different origins is `sim`;
    - all of this holds at 1, 2, 4 and N threads.
  - **Instant-granularity comparison.** The same run with every origin forced to `sim` differs only in `engine.inputs`, `agenda.timers`, `engine.commands`, and `events.meta` (through the events' `origin` field).
  - **Planted control.** An engine that stamps the later hops `sim` (the engine twin of `selftest_envelope --control origin-lost`) fails the test by name.
  - **Neutrality at `sim`.** A `sim` row hashes exactly as before: the existing vectors are unchanged, and a planted `u8 0` origin tail fails the golden test.
  - **Journal.** A scoped `ManifestSet` append is refused by name. Smoke's 1,136 `OracleMove` rows and solo's one row load unchanged.
  - **Keyframes.** Tailed rows of all three tables round-trip through a keyframe (the V-DET-05 harness, three sizes).
- **Status:** [ ] not started

#### T-05: Provenance of change: the cause slot, the guard, `event_ref` columns and keyframe v2 (§6, §7; R2, R5, R12 (d), R17, R22 NIT-4, R25, R26)
- **Deliverable:**
  - **`StepCtx::set_cause(column, row, EventRef)`.** It is buffered; the merge writes it after the level's events, turning a `Draft` into the id. It writes any `event_ref` column:
    - a cause sibling;
    - a column whose value is an event the step emits (S6's `chain_head` and `origin_event`, M7's `claim.cause_event`).
  - **`SystemView::write`** hands no `event_ref` column to a step.
  - **The post-merge guard.** Before a level runs, the engine keeps the rows of every column with a `cause_column` that the level's jobs write. After the merge, two kinds of row must each have a present cause, else `SimError::ChangeWithoutCause`, naming the system, the column and the row:
    - every kept row whose value changed to something other than the declared `default`;
    - every inserted row whose value is not the default.
  - **The spec table** carries `[[owns]] default` and `cause_column`.
  - **Storage.** An `event_ref` column holds an `Option<EventId>` per row.
  - **`rh-snapshot` encodings:**
    - **The `event_ref` world-column row encoding: `opt(u64)` per row** (`u8 0` for none; `u8 1 ‖ u64 id`). This is the sprint's choice under §7, and it gets hash golden vectors.
    - **`events.held` v2:** exactly §6's row encoding, with `admitted` carried but not hashed (R5).
    - **The keyframe format becomes `rehoboam/rh-snapshot/keyframe/v2`** (R12 (d)). A v1 keyframe is refused by its format name.
- **Acceptance:**
  - **V-SAGA-21's engine side.** On a stub system in the shape of S6 (`condition`, `contaminants` and `charge` sharing `condition_cause`, with defaults declared), over random small worlds, every non-default value carries a cause that resolves to an earlier event.
  - **Planted, each refused by name:**
    - a change with no `set_cause` (`ChangeWithoutCause`, naming the column and the row);
    - an inserted non-default row with no cause (R26);
    - a step writing an `event_ref` column directly.
  - **The first event can be a cause.** Event id 0 works as one (R17).
  - **Keyframes.** A keyframe v2 with held events and an `event_ref` column round-trips at three sizes, and the guard's size bounds are pinned. A v1 keyframe file is refused by name. The nightly single-bit-flip sweep (DEF-3) covers a v2 keyframe.
  - **Test goldens.** The stub-world hash golden and the keyframe goldens are regenerated by their tools.
- **Status:** [ ] not started

#### T-06: `rh-stream`: the envelope tables from the event (§1, §4, §5, §8; R6)
- **Deliverable:**
  - **The dictionaries come from the typed Registry:**
    - `event_role_kind` from `[event_roles]`;
    - `motive_edge_kind` from `[motive_edges]`;
    - `attribute` by `attr_id`.
  - **The child tables come from the event,** replacing SPR-013's P1.0 constants:
    - `event_role` rows from the event's cast, in canonical order;
    - `event_snapshot` rows;
    - `causal_edge` rows with kind, motive index, weight and evidence;
    - `EVENT.origin` from the event.
  - **`cause_told`** is computed by the Chronicle Writer from a per-timeline admission bitset kept in host memory, one bit per `EventId`, and rebuilt from the sealed `event` tables at resume (§5).
- **Acceptance:**
  - **The three rungs.** On `solo`, `household` and `hamlet`, every table and line is byte-identical to SPR-013's writer, except the lines T-07 names.
  - **A stub world with the full envelope** (snapshots, motive edges, a witness and a place):
    - writes rows that DuckDB reads with `schema.sql`'s types (V-DATA-10 subset);
    - writes canonical lines that round-trip through `rh-stream`'s parser.
  - **`cause_told`:**
    - with a test-only admission predicate that refuses some drafts, an edge to a refused cause has `cause_told` false;
    - after a `kill -9` and a resume (SPR-013 T-09's path), the rebuilt bitset gives byte-identical segments;
    - with every draft admitted, every row is true.
  - **V-STR-01** holds on `solo` and the stub world: two runs, and 1 against N threads.
- **Status:** [ ] not started

#### T-07: The four baselines, `registry_version` and the divergence record (§6; R20, R23)
- **Deliverable:**
  - **The final commit,** subject ending `[C3]`:
    - `registry_version` bumped to the next free number in `registry_v0.toml`;
    - the example manifests' pins follow, and so do the worlds' pinned SHA-256s;
    - the four baselines are regenerated with `bin/regen_baseline.sh`.
  - **The divergence record,** in the PR body and in `VER-SPR-020`, for each world:
    - the bisect's (V-DET-11, `rh-golden`) first divergence, measured on a monthly ladder;
    - the set of differing tables at every yearly rung.
  - **The docs** of the compliance table.
- **Acceptance:**
  - **Robust form (R23).** At every rung of every world, the differing tables are a subset of `{rh-core/events.meta, rh-core/engine.inputs, rh-core/agenda.timers, rh-core/engine.commands}`. A world column or any other engine table in the set is a defect.
  - **Against §6's prediction,** recorded line by line:

    | World | Predicted |
    |---|---|
    | `smoke` (yearly) | `engine.inputs` at rungs 1–994; `events.meta` from rung 1 |
    | `solo` | `engine.inputs` at rungs 1–53; `events.meta` from the first rung after Ada's `DEATH` (year 57) |
    | `smoke`, monthly over year 7, months 4–6 | `agenda.timers` and `engine.commands` move here, and never at a yearly rung |
    | every world (first divergence) | The bisect names `rh-core/engine.inputs` at month 1 |

    A difference from the prediction inside the four tables is recorded, not a defect.
  - **The version bump alone is measured as a separate step:** the envelope code at the old version first, then the bump. The bump changes only the ladder header's `tuple` line, plus `journal` if the journal anchors the tuple. It changes no rung line and no root line.
  - **Channel A.** On `solo`, `household` and `hamlet`, only these lines differ from SPR-013's:
    - `STEP_DIGEST`'s `state_hash`;
    - `KEYFRAME`'s `keyframe_format` (now v2) and `state_hash`;
    - `EVENT`'s `origin`, for an event whose causal chain starts at a journaled input.

    Every other line is byte-identical, as envelope §6 states it (corrected 2026-10-17).
  - **Ladders.** The four PR-tier ladders at 1 and N threads equal the new baselines. The nightly monthly ladders at 1, 2, 4 and N threads × 3 are equal on the PR head (`bin/golden_nightly.sh`).
  - **Manifests.** V-WC-04 passes on the re-pinned manifests (`manifest_parity.py`).
  - **Performance.** V-PERF-04 and V-PERF-11 are within budget, or re-baselined by the tool in a `[perf-baseline]` commit with a reason line.
  - **CI time.** Both parts run in 12 minutes or less.
- **Status:** [ ] not started

## Director items, and what changes under the alternative

| Item | Built | Under the alternative |
|---|---|---|
| D44 (DATA-2: nine kinds, recommended at 80%) | Nine codes, 0–8. Kinds 6–8 have no writer in P1a: `append` and the canonical parser refuse them (§5; R22 NIT-5) | **Eight:** code 7 becomes a reserved code with no writer, refused exactly as 6 and 8 are today. No code path, hash, golden or segment changes in P1a. Text follows: DATA-2 and V-DATA-09 in `docs/03` and `docs/09`, and `selftest_envelope.py`'s "only the nine kinds" message. The sprint does not wait for D44 |
| D38 (DATA-5 admission) | The bitset plumbing; every P1a draft is admitted | Either answer gives the same P1a rows (§5, §6). The rule itself is P1.11's |

## Out of scope (explicit)
- **The V-SAGA-15 stored-byte gate** on the 10k dynastic world (SPR-030; the measure is fixed in §9, R11).
- **Code and data owned by later sprints:**
  - M7's and T1's code, and their declared snapshots and edges in use (SPR-021 to SPR-025);
  - S6 and S7 (P1b);
  - a `parent` role (SPR-029, R12 (b)).
- **The input-event format** that edge kinds 6–8 need (P2.9).
- **The DATA-5 admission rule** beyond its plumbing (D38; P1.11).
- **Pattern-side work:**
  - the satisfiability check of multi-clause anchored reads (P2.1);
  - the pattern DSL and `lint_patterns.py` changes (merged with PR #61).
- **BCK-002 A-40** (a per-month event digest list in the bisect dump). It would let T-07's record name the first differing event, but it is not assigned here.
- **The live socket** (SPR-026).

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-IF | T-01, T-02, T-03, T-06, written from `envelope_v0.md` §1–§5 and §8, `canonical_v0.md` and `channel_a_v0.sql` before reading the code:<ul><li>the Registry refusals;</li><li>an independent v2 digest and an independent coalescing reference;</li><li>`emit`'s refusals;</li><li>the pre-act snapshot;</li><li>channel-A tables read with DuckDB;</li><li>`cause_told` across a resume;</li><li>a fuzz of the canonical parser over lines with the full envelope</li></ul> | `VER-SPR-020-TEST` |
| AGT-003-DET | T-04, T-05, T-07, written from `05.1` rule 4 v1.13, ADR-0025 and envelope §6:<ul><li>`origin_through_hops` with their own chain;</li><li>tail neutrality at `sim`;</li><li>keyframe v2, with v1 refused;</li><li>the `ChangeWithoutCause` guard on inserted rows;</li><li>an independent divergence measurement of all four worlds, checked against the four-table rule;</li><li>the nightly ladders on the head</li></ul> | As above |

## Merge route

- **Class and conditions.** C3, merged by the Architect after the Tester pass, the audit and the announcement of the four baseline changes to the director, who may veto (GOV-005 §6). The PR title ends `[C3]`, and so does the subject of the final commit. The CRN-paired ensemble is not built yet (SPR-030).
- **One C3 open for merge at a time (GOV-005 §4.1).** This sprint and SPR-014 both touch `rh-core`'s timers and commands: SPR-014 adds the `cause` slot, the one-binding inbox and the merge's event-before-timer order; SPR-020 adds the origin tails and `ctx.under`.
- **Recommendation: SPR-014 first, then SPR-020 (8/10).** The reasons are in SPR-014's "Merge route". SPR-020 starts T-01–T-03 and T-06 at once, and lands T-04 and T-05 after merging `origin/main` with SPR-014 in it.
- **After SPR-014 merges, SPR-020 re-verifies:**
  - the timer tails at all four lengths (0, 2, 9, 11);
  - `origin_through_hops` through the real drain;
  - `DEATH`'s `actor` from a drained `ScheduleDeath`;
  - the divergence measured from SPR-014's baselines, which are unchanged.
- **If SPR-020 has to merge first** (only if SPR-014 slips past it):
  - it introduces `EventRef` and the timer `cause` tail itself, exactly as envelope §2 and `05.1` rule 2 state them;
  - SPR-014 then re-proves R1 and R15 against the new baselines and the regenerated hash golden.
- **SPR-026** (C2, no hash) is independent. The wire reads only the frozen prefix (`canonical_v0.md` §1), and its golden lines come from `golden/canonical_v0.txt`, already amended by PR #61. Whichever of the two merges second resolves the textual conflicts in `check_all.sh` and the inventory test, and if this sprint lands after SPR-026, SPR-026's V-STR-01 probe runs against the new baselines.

## Blockers

| # | Blocker | Status |
|---|---|---|
| 1 | SPR-013 not merged | Open |
| 2 | SPR-014 not merged (blocks T-04 and T-05 only) | Open |
| 3 | D44 | Open, director; non-blocking (built as nine; eight changes nothing built) |
| 4 | The director's plan approval (GOV-005 §2) | Open: D48 |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-020` is PASS, with the divergence record of T-07.
- [ ] No open DEF- against this sprint.
- [ ] The merge route matches the CI-detected change class (C3). Each golden-corpus change was announced to the director first.
- [ ] BCK-002 A-24 (envelope part) is closed, and `docs/09` V-DET-22 is filed.
