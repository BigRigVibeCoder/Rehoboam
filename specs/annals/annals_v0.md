# Family annals v0: templates over the event envelope, with a provenance sidecar

*Status: v0, a C1 contract for WP P2.8a (SPR-029), drafted for the Architect on 2026-10-17. It is the template set and its contract that PRJ-002 §10 lists as missing, and NAR-1's first binding at P1a (`docs/03`), checked by V-NAR-05 (every sentence maps to event ids) and V-NAR-07 (annals need no model). Binding texts: `05.11` §0 (plane C, downstream, a template fallback at every stage), §3 stage 2 (templated episode text with provenance), §9 (`narrative_hash` covers texts from templates) and §10; `05.4` §5 (the Annals form) and §6.1 (canon tiers: an annal is CT0 and CT1 only); ADR-0017 (forms escalate from templated annals; a provenance sidecar), ADR-0024 (plane C: nothing here is read by plane A), ADR-0034 (describe, never decide: annals use no model), ADR-0035 §5a (the engine simulates truth; downstream tools generate perception). Rendered inputs: `specs/chronicle/envelope_v0.md` (roles, anchored snapshots, typed and motive edges, `cause_told`), `specs/stream/canonical_v0.md` (the chronicle's text form, parsed by `canonical_v0.py`), the Registry's `[event_roles]`, `[motive_edges]` and `[attributes]`, and the `[[emits]]` entries of `M2.demography` (0.2.2 and the 0.3.0 draft: `DEATH`, `EXTINCTION`, `BIRTH`, `MARRIAGE`), `M7.dynasty` 0.3.3 (successions, claims, regencies, seats, extinction) and `T1.minds` 0.3.3 (feuds). Data: `templates_v0.toml`. Reference: `python -I specs/annals/selftest_annals_v0.py --check` (stdlib only). Lint: `python -I specs/annals/lint_templates.py`. Fixtures and goldens: `fixtures/`, `golden/`. Two items are the director's, D49 (names) and D50 (V-NAR-05 and the prologue), in `docs/decisions_pending.md`; the rest fits either answer.*

## 0. What this contract fixes, in one paragraph

A family annal is plain text rendered from a sealed channel-A chronicle by a fixed set of templates, one sentence per admitted event of an in-scope kind, in `EventId` order under year headings, grouped by house, by family line or by year, and opened, for a house whose past is in the genesis keyframe only, by a short prologue rendered from the keyframe's life-course and tenure rows. A template is a sentence with slots, and a slot binds only to what the event carries: its roles, its anchored snapshots, its integer payload (through a layout that quotes the emitting system's `[[emits]]` text verbatim), its causal edges and its tick. A template never reads world state and never invents a fact (§3); the lint refuses a slot outside that vocabulary by name (§7). A person is designated by the manifest's name when there is one, else by kinship to a named person, else by id (§4; the names question is D49). Every sentence has one row in a sidecar: the event ids it shows, the persons it designates, the records it read, the template id and version (§8); the text and sidecar of one chronicle under one template version are byte-identical on every machine, and their digest is what `narrative_hash` folds for annals (§9). No model is called anywhere (V-NAR-07); the renderer is a pure function of its four inputs (§1).

## 1. Inputs, and what each may be used for

| Input | Form | What it gives the annal | Plane |
|---|---|---|---|
| The chronicle | A channel-A file in the canonical text form (`canonical_v0.md`): `EVENT`, `LIFE_ENTRY`, `LIFE_EXIT`, the rest ignored. The renderer parses it with `canonical_v0.py` and refuses what the parser refuses (one spelling, R4 order, a cause before its effect) | Every sentence of the body (§2) and the kinship designations (§4) | A, read only |
| The world's calendar | `days_per_year` (360 or 365) and `epoch_label` from the manifest (`world.calendar`; `05.1` §4.1) | Every date (§3, `date` and `year`) | the manifest |
| The names file | `[persons]`, `[houses]`, `[titles]`: id to name, from the manifest (a cast member `k` is founder `k` in `rh-worldgen`'s order; house `k` is the `k`-th dynasty in manifest order, SPR-019 B1; titles are the polity's labels). Written by a plane-C tool from the manifest and the run's genesis ids; the reference takes it as a TOML file | Names (§4) | C |
| The keyframe extract | The genesis keyframe's rows a house prologue needs: every `life_course.*` row (person, sex, mother, father, birth tick, exit tick when dead), M7's `house_member` and `tenure` rows (`specs/systems/M7.dynasty.odd.toml` `[[owns]]`), as a TOML file the reference reads; the tool reads them from the keyframe's Arrow tables (`rh-snapshot`) | The prologue (§6) and the house of a person with no name (§4) | A at genesis, read only |

The renderer reads nothing else: no engine, no model, no network, no clock (`05.11` §0 rules 1, 2 and 5; V-NAR-07). It writes two files per group, the text and the sidecar (§8).

## 2. Templates and variants

`templates_v0.toml` is the whole template set: a header (`format`, `version`, `scope`), enums, payload layouts, chronicle templates and prologue templates.

- **Scope** lists the kinds an annal tells, grouped as the WP names them: births (`BIRTH`), marriages (`MARRIAGE`), deaths (`DEATH`, `EXTINCTION`), successions (`SUCCEED`, `INTERREGNUM_CRISIS`, `DEPOSED`, `REGENCY_BEGUN`, `REGENCY_ENDED`, `SEAT_GRANTED`, `HOUSE_EXTINCT`), claims (`CLAIM_CREATED`, `CLAIM_PRESSED`, `CLAIM_RENOUNCED`) and feuds (`FEUD_DECLARED`, `FEUD_ENDED`). Sixteen kinds, every one declared by a system's `[[emits]]` (the lint checks). An admitted event of another kind (`MURDER_ATTEMPT`, `MENTAL_BREAK`, the smoke verbs) is not rendered; the sidecar's trailer counts it by kind (§5). The kinds T1 emits are told through their consequences in P1a (a `DEATH` with an actor is "slain by"); P2.8 widens the scope.
- **A template** is `{ id, version, kind, when?, text, label }`. `text` is the sentence; `label` is the noun phrase another sentence uses when it names this event as a cause ("after the death of X"). For one kind the variants are tried **in file order** and the first whose `when` holds renders; the last variant of a kind has no `when` (the lint checks), so every admitted event of an in-scope kind renders exactly one sentence, and the choice is a function of the event alone.
- **`when`** has four keys, each a predicate on the event: `has = [roles]` (each role filled), `lacks = [roles]` (each role unfilled), `payload = { slot = enum id }` (the payload slot's code is that id) and `edge = kind` (a told causal edge of that kind is present). Nothing else is a condition: not a date, not another event, not state.
- **Versions.** `templates.version` is the set's semver; a template's `version` is an integer. Any change to a text, a slot, a layout, an enum word or the scope bumps `templates.version` and the changed template's `version`, and the goldens are regenerated by the reference (`--write`, never by hand). The sidecar names both (§8).

## 3. The slot grammar (what a template may bind to)

A slot is `{name}` in a template text, parsed by the standard library's `string.Formatter` (the one parser, shared by the lint and the renderer). The names, exactly:

| Slot | Binds to | Renders as | Lint: resolves iff |
|---|---|---|---|
| `date` | the event's `tick` | `day 14 of month 3` (the year is the heading, §5) | always |
| `year` | the event's `tick` | `the twelfth year (Year of Ash)`; before the epoch, `the third year before the epoch (…)` | always |
| `actor`, `target`, `instrument` | the cast row of that role (`event_role`) | the person's designation (§4); `no one` when the role is unfilled on this event | the kind's `[[emits]] roles` lists the role |
| `witnesses` | every `witness` row | the designations, comma-joined; `no one` when there is none | `roles` lists `witness` |
| `payload.<slot>` | one integer of the payload, by the kind's layout | an enum's words (`by murder`), an ordinal (`second`), or the integer | `[payloads.<KIND>]` declares the slot **and** its `cited` text occurs verbatim in the emitting `[[emits]]` `when`, and `system` is the emitter |
| `snapshot.<attr>.<role>` | the `event_snapshot` row of attribute `attr` whose entity is the role's person, no partner | by the attribute's Registry type: a `q8_8` as an exact decimal (`63.0`), an enum by its value name, a bool or int as the number; `not recorded` when the row is absent (an absent owner, §4 rule 4 of the envelope) | the kind's `snapshots` declares `{ attr, of = role }` |
| `motive` | the `motive` of the earliest told `motivated_by` edge | `[enums.motive]` words by `[motive_edges]` index (`out of jealousy of`); `for a motive the chronicle does not name` when the edge carries none | the kind's `motives` is non-empty |
| `cause.triggered`, `cause.contingent`, `cause.motivated_by` | the earliest told edge of that kind (edges are in `cause_id` order), its cause event | the cause's `label`, rendered by the cause's own variant with the cause's own fields, and the cause id joins the sentence's events; `an event the chronicle does not tell` when the cause is untold (`cause_told = 0`) or absent from the file | the slot is in a `text`, never in a `label` (no recursion) |

Three rules the grammar enforces by construction:

1. **Only the event.** Every family above reads the event's own record (the tick, `event_role`, `event_snapshot`, `causal_edge`, `payload`) or, for a cause slot, another event's record. No slot reads a world column, a keyframe row, a sift match, a significance value or the manifest. A name that is not in the table, such as `state.title.holder`, `weather` or `house`, is refused by the lint as "outside the envelope" (§7). `05.11` §3's envelope derivations (time of day, lighting, inferred emotion) are not slots in v0: a future derivation is a declared function of envelope fields, added to the table with its own lint rule, never a free read.
2. **Nothing invented.** A template may add fixed words (the historian's register: D16) but no fact: no cause of death the payload does not carry, no title the event does not name, no pronoun (the chronicle carries sex only in life-course records, so a person is always designated, never "he" or "she"), no emotion (none is simulated: envelope §4 rule 4). When an event does not carry a fact, the variant for that case says less (`death.other`: "died by accident"; `claim.created`: "gained a claim to a vacant title").
3. **Payloads are cited, not assumed.** The envelope fixes a payload's meaning per verb in the emitting spec's `when` text, not in a typed field (M2 0.3.0: "Payload: [sex code, birth order (parity after the birth)]"; `DEATH`: "Payload: [death_cause code]"). A layout therefore quotes that text and the lint checks the quotation against `specs/systems` (the `next/` version superseding the frozen one, as `lint_odd.py --next` does), so a layout cannot outlive the sentence it reads. A kind whose `when` states no layout has no payload slot: M7 0.3.3's verbs name the title in the payload ("a title is not an entity: the payload names it") without a layout, so v0's succession sentences do not name the title (§10 item 1).

## 4. Designating a person (what the chronicle carries, and the fallback)

The chronicle carries **no names**: an `EVENT` names persons by `PersonId`; a `LIFE_ENTRY` carries sex, mother and father ids and the birth tick; the manifest names its cast (`cast[].name`), its dynasties (`polities[].dynasties[].name`) and its titles (`polities[].titles`), and nothing names the synthesised population, the back-filled dynasts or an in-run birth (D49). The designation rule, fixed so that two renderers agree, in order:

1. **A name**, from the names file. Source token `name/<pid>`.
2. **The Nth child of a named mother**: the person's `BIRTH` event is in the chronicle and its `actor` (the mother) is named: `the second child of Maud` (the parity is `payload.parity`, the mother's live births after this one). Sources `birth/<event id>`, `name/<mother>`; the mother joins the sentence's persons.
3. **A son or daughter of a named parent**: the person's life-course row (a `LIFE_ENTRY`, else the keyframe extract) names a parent who is named, the father first: `a son of Edric Ashcombe`. Sources `life/<pid>` or `kf:life/<pid>`, and `name/<parent>`.
4. **The id**, with the house when known: `person #1007 of House Ashcombe`, where the house is the keyframe's `house_member` row, else the father's house, else the mother's (M7's `house_member.house` rule, followed through life-course rows). Sources `kf:member/<pid>` or `life/<pid>`, and `house/<house id>`. Without a house, `person #1007`.

The rule goes one generation deep by name (rule 2 and 3 name a parent only when the parent is named, not described), so a designation is at most "the Nth child of X" or "a son of X", never a chain. Every person a sentence designates is listed in its sidecar row with the records the designation read, so a reader can audit it. A `LIFE_ENTRY` is a channel-A record, not world state: reading it is reading the chronicle (`05.10` §4's fidelity promise), and the sidecar says so.

**Recommendation on names (D49, the director's):** a deterministic plane-C name generator at P2.8, `H(world_seed, person_id, "name")` over the culture's name base (the manifest's `cultures[].naming_seed` exists; Azgaar's name bases are PORT in `docs/14` §C), computed like visual DNA (`05.11` §5) and never stored in plane A; G1a reads with the fallback above. Confidence 70%.

## 5. Grouping, ordering, headings and what is left out

- **Groups.** `house:<HouseId>`: the house's members are the keyframe's `house_member` rows of that house plus every in-run birth whose father is a member, else (no father) whose mother is (M7's rule); an event belongs to the group when a cast member is a member. `line:<PersonId>`: the person and every descendant through life-course rows (channel A, then the keyframe). `year`: every event. One event may appear in two houses' annals (a marriage) and does, with the same sentence and sidecar row in each.
- **Order.** Events in `EventId` order (emission order, ADR-0025), which is tick order and, within an instant, phase order: a death at `Vital` precedes the succession it triggers at `Polity`. A year heading opens each calendar year that holds an event: `## The twelfth year (Year of Ash)`; before the epoch, `## The third year before the epoch (…)`. The prologue (§6), when there is one, precedes under `## Before the chronicle began`.
- **The calendar.** A tick is minutes from the epoch (`05.1` §4.1). The day is `tick div 1440` (floor division, so a tick before the epoch falls in the day that contains it), the year `day div days_per_year`, the month from the month lengths (twelve 30-day months, or 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31), no leap year. Year 0 is the first year (`the first year`); year −1 is `the first year before the epoch`. Ordinals are words to twelve, then `13th`, `21st`, `22nd`, `23rd`, `111th`. Months have no names in the Registry, so dates are numeric: `day 14 of month 3`.
- **Left out.** (i) An event of a kind outside the scope: counted in the sidecar trailer (`# omitted: 1 event(s) of kinds outside the template set: MURDER_ATTEMPT x1`). (ii) An event the chronicle does not hold: nothing in the annal depends on a refused draft except a cause slot, which then says "an event the chronicle does not tell" and records `untold/<id>`. **D38 either way:** under the admission rule every P1a world is under the ceiling, so every birth, marriage, death and succession is a row and v0 tells all of them; above the ceiling (the kingdom worlds, P1.11) the chronicle itself omits commoner acts, the annal tells what the chronicle tells, and a named actor's acts stay (criterion 2). v0 adds no tellability or significance filter of its own: an annal is the whole told record of its group, which is the Annals form (`05.4` §5); selection by significance is P2.8's grouped arcs. (iii) `STEP_DIGEST`, `KEYFRAME`, `INPUT` records: not events; ignored.

## 6. Opening a house whose past is only in the keyframe (the prologue)

PR #62 (SPR-019) back-fills a dynasty's pre-epoch genealogy and each title's past holders into the genesis keyframe: closed life-course rows with `exit_tick < 0` for the dead, `house_member` rows for every agnatic descendant of the root, tenure rows by the title's law. **No channel-A record exists for the back-filled dead** (REV-SPR-019 R1): the living founders have a `LIFE_ENTRY` at tick 0 whose `mother_id` and `father_id` point at persons the stream never mentions. So a house's past is readable only from the keyframe, and a house annal opens with a prologue rendered from the keyframe extract (§1):

| Prologue template | Sentence | Sources (sidecar) |
|---|---|---|
| `prologue.house` | `House Ashcombe entered the chronicle with 2 living members.` (members of the house whose keyframe row has no exit) | `house/<id>`, `kf:life/<pid>` of each living member |
| `prologue.ancestor` | `Person #1001 of House Ashcombe, born on day 1 of month 1 of the 60th year before the epoch (Year of Ash), died on day 1 of month 1 of the twelfth year before the epoch (Year of Ash), before the chronicle began.` for each member with an exit tick, in `PersonId` order | `kf:life/<pid>` (and the designation's sources) |
| `prologue.tenure`, `prologue.tenure.open` | `Person #1001 of House Ashcombe held the lordship of the Manor of Ash from … until ….` / `Edric Ashcombe has held the lordship of the Manor of Ash since ….` for each tenure row whose holder is a member, by title then start | `kf:tenure/<title>/<start>`, `title/<id>` |

Prologue slots are their own vocabulary (`house`, `living`, `person`, `birth_date`, `death_date`, `holder`, `title`, `start_date`, `end_date`: keyframe rows and names only); the lint keeps the two vocabularies apart. A prologue sentence cites keyframe rows, not event ids: that is D50 (§11). A `line` or `year` annal has no prologue; a house without a keyframe extract (a world with no dynasties) opens at its first event.

## 7. The lint (`lint_templates.py`)

Run on the real set in `check_all --part core` and on nine rendered controls (`tests/lint_fixtures/README.md`). It refuses, each by name:

1. a slot outside the grammar of §3 (`weather`, `state.title.holder`, `witness` in the singular): "is outside the envelope: a template binds only to roles, snapshots, payload, edges and the tick";
2. a role slot the kind's `[[emits]] roles` does not list; `witnesses` on a kind without `witness`;
3. a snapshot slot the kind's `snapshots` does not declare with that attribute and role;
4. a payload slot without a layout, a layout whose `system` is not the emitter, or whose `cited` text is not verbatim in the emitter's `when`;
5. `motive` on a kind that declares no `motives`; a cause slot in a `label`;
6. a kind in scope with no template, a template whose kind is out of scope, a kind whose last variant has a `when`, a duplicate template id, a non-integer version;
7. a `when` with an unknown key, a role the kind does not declare, a payload id not in the slot's enum, an edge kind outside the three;
8. `[enums.motive].ids` not equal to the Registry `[motive_edges]` in list order; an enum whose `ids` and `words` differ in length; a missing prologue template or a prologue slot outside its vocabulary.

It reads `specs/systems/*.odd.toml` and `specs/systems/next/*.odd.toml` (a next version supersedes its frozen one by system id, the `_template` file skipped), so the templates follow the spec text the sprints build against; when the 0.3.0 M2 freezes, nothing here changes.

## 8. The sidecar and the output files

A group renders to two files, `<world>_<group>.txt` and `<world>_<group>.sidecar.txt` (`group` with `:` as `-`), UTF-8, LF.

**The text:** a line is a heading (`#` or `##` first), blank, or one sentence. Every sentence line has exactly one sidecar row, keyed by its 1-based line number, so a reader audits a sentence by its line.

**The sidecar:**

```text
#rehoboam/annals/v0 world=ashcombe group=house:9001 templates=0.1.0 templates_digest=<xxh3-64 hex> calendar=360
P <line> <template id> <version> [] [person ids] [sources]         # a prologue sentence
S <line> <template id> <version> [event ids] [person ids] [sources] # a chronicle sentence
# omitted: <n> event(s) of kinds outside the template set: <KIND> x<n>, ...   # when any
digest <xxh3-64 hex>
```

`event ids` are every event the sentence shows: its own, plus a cause slot's cause (V-NAR-05's "the event IDs it narrates"). `person ids` are every person designated, cast members and the named parents a designation reaches. `sources` are the records read beyond the event: `name/<pid>`, `birth/<event>`, `life/<pid>`, `kf:life/<pid>`, `kf:member/<pid>`, `kf:tenure/<title>/<start>`, `house/<id>`, `title/<id>`, `untold/<event>`. Lists are sorted and comma-joined, so a row is canonical.

## 9. Determinism, the digest and `narrative_hash`

- **The render is a pure function** of the chronicle file, the calendar, the names file, the keyframe extract and the template file: no clock, no random draw, no environment, no iteration over a hash map that reaches the output (events are rendered in id order, members and sources are sorted). The reference renders every fixture group twice and requires byte equality (check 6).
- **`annals_digest = XXH3-64("rehoboam/annals/v0" ‖ templates_digest ‖ text ‖ LF ‖ sidecar-before-the-digest-line)`**, with `templates_digest = XXH3-64(the bytes of templates_v0.toml)`, both little-endian 8 bytes, the hash `rh_snapshot::hash::xxh3_64` and `canonical_v0.py` already share. It is the last line of the sidecar. The same chronicle under the same template file gives the same digest on every machine; a changed template word changes every digest, which is the point.
- **`narrative_hash`** (`05.11` §9, ADR-0024: "a pure function of A's event stream plus pattern/compiler versions") covers the deterministic artefacts; annals are one. When the pipeline compiles annals (P2.8a's tool over a sealed timeline), it folds each group's `annals_digest` into the timeline's `narrative_hash` in group-id order; this contract fixes the digest, and the folding order, so that the tool and the reference agree. The engine's `STEP_DIGEST.narrative_hash` is unaffected until then (the sifter writes it today).

## 10. For the Architect (specs and contract text; decided here with a confidence)

| # | Decision | Why | Confidence |
|---|---|---|---|
| 1 | **M7's payloads carry no layout, so v0 succession sentences name no title.** Recommended: M7's next version (the 0.4.0 draft in PR #62) states `Payload: [title id]` on `SUCCEED`, `DEPOSED`, `INTERREGNUM_CRISIS`, `REGENCY_*`, `SEAT_GRANTED`, `CLAIM_*` and `[house id]` on `HOUSE_EXTINCT` in each `when`, as M2 0.3.0 does for `BIRTH`; then a `[payloads.SUCCEED]` layout with `{ name = "title", names = "titles" }` renders "succeeded Edric Ashcombe in the lordship of the Manor of Ash" through the names file (§1). A one-line amendment per kind, no column change | A template may not assume a meaning the spec does not state (§3 rule 3) | 80% |
| 2 | **No `parent` role is appended to `[event_roles]`** (envelope §1, R12 (b) left it to this sprint). The father is in the child's `LIFE_ENTRY` and the designation rule reads it (§4 rule 3); a role row would duplicate a life-course fact in every `BIRTH` | 35 bytes per event is the envelope's budget; the life-course record already carries it | 75% |
| 3 | **The keyframe extract is an input file, not a read of the keyframe by the reference.** The reference stays stdlib (`05.11` §0; GOV-003 §3) and Arrow is not; the P2.8a tool reads the Arrow tables and writes the extract's rows, and the reference fixes what those rows are (§1) | Keeps the exact reference runnable anywhere | 80% |
| 4 | **Payload layouts cite the `when` text** rather than a typed `payload = [...]` field on `[[emits]]`. A typed field is better and is `lint_odd.py`'s to add (an Architect-level spec change across M2, M7 and T1); when it lands, `cited` becomes that list and the lint compares lists | The citation makes v0 exact today without touching frozen specs | 70% |
| 5 | **No pronouns, no months' names, numeric dates.** The chronicle carries sex only in life-course records and the Registry names no month; the annal says what the record says | §3 rule 2 | 85% |
| 6 | **Reuse.** The template language is slot substitution over the stdlib formatter, with improv's pattern (snippets chosen by facts, here variants chosen by envelope predicates; `docs/14` §N, PORT) and without Tracery's grammar expansion, which v0 does not need (no recursion, no random choice: a random choice would break §9). `canonical_v0.py` parses the chronicle; `gen_hash_golden.py`'s XXH3 hashes the digest | `docs/14` before writing anything new | 80% |

## 11. For the director (`docs/decisions_pending.md`; the Architect files them at the merge)

| # | Item | Recommendation | Confidence |
|---|---|---|---|
| D49 | Names for persons (§4) | A plane-C name generator at P2.8 (`docs/11`); G1a reads with the kinship fallback | 70% |
| D50 | V-NAR-05 and the prologue (§6): a sentence of state before the chronicle began maps to keyframe rows, not event ids | Amend V-NAR-05's gate text (`docs/09`) to "every chronicle sentence maps to the event IDs it narrates; a sentence of state before the chronicle began maps to the keyframe rows it was rendered from, and only the opening section may hold one" | 70% |

## 12. Acceptance tests (SPR-029), as concrete probes

| Test | Probe, in `selftest_annals_v0.py --check` | Control that must FAIL |
|---|---|---|
| **V-NAR-05** sentence provenance | Check 1: every non-heading, non-blank line of every fixture group has exactly one sidecar row; an `S` row cites at least one event id, every cited id is an `EVENT` of the chronicle and of an in-scope kind; a `P` row cites a `kf:` row; no row is without a sentence. Check 8: the goldens equal the render byte for byte, so the sidecar a reviewer reads is the one the probe checked | `--control no-event-id` plants a sentence with no row: "1. provenance (V-NAR-05): FAIL" |
| **V-NAR-07** annals need no LLM | Check 3: the renderer's imports are the standard library and the two sibling references only (parsed from the file; `sys.modules` holds the siblings), so nothing it calls can reach a model; and the render is a pure function (check 6). The forms-and-gate half of V-NAR-07 (digests routed to Saga or Chronicle) is P3's | a planted import is not rendered (a one-line edit would show in review); the determinism control is the no-event-id plant, which also fails check 6 |
| **§3, a template never reads state** | Check 2 runs the lint (§7) on the loaded set | `--control undeclared-slot` (`{instrument}` on `DEATH`); `--control state-read` (`{state.title.holder}`); nine rendered lint controls in `check_all` |
| **§5 the calendar** | Check 4: known answers for 360 and 365 days, before and after the epoch; ordinals; `q8_8` rendering | (a wrong answer is a failing known answer) |
| **The fixtures are real events** | Check 5: every fixture `EVENT`'s `verb_id`, `act_classes` and `tellability` equal the Registry's (plus the next M2 delta's `BIRTH` and `MARRIAGE`), and every snapshot attribute is a Registry attribute | (a stale fixture fails here) |
| **Coverage** | Check 7: every in-scope kind is rendered by some fixture (every variant is, today; an unexercised variant is printed) | a kind without a fixture event fails |

**The readability bar for the director's G1a reading (a report, not a gate).** The sample is `golden/ashcombe_house-9001.txt` (House Ashcombe over 39 years: a birth, a marriage, a death and a succession in one instant, a feud with a motive and a stress snapshot, a murder told through the death it caused, a regency, claims, a deposition, a seat, an interregnum) and `golden/solitary_year.txt` (Ada's death and the extinction, on a 365-day calendar). Read each aloud once. The bar: (i) every sentence says who did what, when, to whom, in one reading; (ii) a cause reads as a cause ("after the death of…", "out of jealousy of the succession of…"); (iii) a person without a name is still recognisable across sentences ("a son of Edric Ashcombe" is the same person each time); (iv) no sentence makes the reader ask "how does the annal know that?" (the sidecar answers it, but the sentence should not raise it). Report what fails the bar as `EVO-` items for P2.8; the known weak points are the id fallback (D49) and the unnamed title (§10 item 1).

## 13. Alternatives considered

- *Tracery or improv as a dependency.* Rejected for v0: no grammar expansion is needed, both choose randomly among alternatives (which §9 forbids), and the reference must stay stdlib. improv's design (facts filter snippets) is kept as variants with `when`. 7/10.
- *Reading sex from `LIFE_ENTRY` for pronouns.* Rejected: a pronoun from the ledger reads as a fact the sentence does not cite, and the Historian register does not need it. 6/10.
- *Rendering the prologue from `LIFE_ENTRY` records only (no keyframe extract).* Rejected: the back-filled dead and every tenure are absent from channel A (R1), so a house would open with its living founders and no past, which fails the G1a reading ("a dynasty lives, dies and inherits"). 7/10.
- *A tellability or significance filter in v0.* Rejected: the Annals form is the whole told record; selection is P2.8's. 8/10.
- *Naming the title on succession sentences from `tenure` rows at the event's tick.* Rejected: that is a state read anchored by time, exactly what the envelope's snapshot rule exists to avoid (envelope §1, MAJ-1); the title belongs in the payload (§10 item 1). 8/10.

## 14. Gates this PR adds (`bin/check_all.sh --part core`)

| Gate | What it checks |
|---|---|
| `Family annals v0: … (selftest_annals_v0 --check; SPR-029)` | §12's eight checks on the two fixture chronicles and five groups |
| its three positive controls | `--control no-event-id`, `undeclared-slot`, `state-read`, each failing its check by name |
| `selftest_annals positive control: a stale golden annal must FAIL` | a copy of the goldens with one digest zeroed is `STALE:` |
| `Annals template lint: … (lint_templates; SPR-029)` | §7 on the real set |
| nine `lint_templates positive control: …` gates | the rendered controls of `tests/lint_fixtures/README.md` |

The inventory test (`python/tests/test_check_all_inventory.py`) lists every gate; `CLAUDE.md` carries the two commands.
