# ADR-0031 — Fidelity by zoom: the World Language, virtual telemetry everywhere, authoritative daily life where you declare it

**Status:** Proposed · **Date:** 2026-10-16 · **Extends:** ADR-0030 (World Stream), ADR-0028 (scale profiles), ADR-0006 (keyed RNG) · **Amends:** `05.1` (new system S5), `05.2` §3.5 (schedules), `05.10`

## Context

The director wants "as much fidelity as I can get, so I can attach my viewer and see exactly how things are going". He is unsure whether that means SimCity or The Sims, and asks four things:
- whether there is a compressed "world language" for the stream;
- whether the stream is compressed;
- whether every person's movement can be stored as telemetry;
- whether we can build a super-compressed, high-fidelity, scalable world language of our own.

**The two reference points differ in kind:**
- **SimCity** simulates aggregates. Its citizens are not persistent individuals.
- **The Sims** simulates a handful of people minute by minute: needs, movement and every interaction.
- **Dwarf Fortress** is the closest precedent for what we want. Its world generation records centuries of history for many historical figures at event level, and its fortress mode simulates about 200 individuals tick by tick.

**The plan as it stands:**
- **Everyone is a ledger row** with a complete life course (ADR-0003, ADR-0026).
- **1–5k named minds** decide and believe (`05.3`).
- **Street-level movement is plane-B hydration from schedule templates.** It is consistent with history but has no consequences (`05.2` §3).

**Storing everyone's movement does not scale.** Daily activity at about 15 blocks a day is about 11 GB per simulated year at 1M persons (about 11 TB per millennium); minute-level movement is far more. Simulating it authoritatively for everyone costs about 15 hours per millennium at 1M (blocks) and about 1.5 hours per simulated *year* (minutes), against a 15-minute budget (PERF-1). All figures are estimates at about 10⁸ simple updates per second on the reference desktop.

**But storage is not the real constraint, because the world is deterministic.**
- **The whole history is already compressed into kilobytes:** History = f(build, Registry, manifest, seed, journal) (ADR-0006).
- **The keyed counter-based RNG is random-access:** `H(seed, system, purpose, entity, t, k)`. Any person's draw for any day can be computed without computing anything else.

So movement does not have to be stored to be exact. It has to be **recomputable on demand**.

## Decision

### 1. The World Language: bespoke grammar, standard container, generative core
1. **Vocabulary (ours).** The Registry: closed sets of act classes, roles, verbs, attributes, goods and places, each a small integer code. No free strings (ADR-0010).
2. **Grammar (ours).** Every stream record is a typed sentence:
   - *when, where, who, verb, whom, with which anchored values, because of which events*;
   - plus a `DECISION_TRACE` record for named minds: the top options with their utility terms, so "why did she do it" is answerable from the stream.
3. **Canonical text form.** Each record has a lossless one-line rendering, which is the "Matrix" view (`05.10` §6). It is greppable, diffable and readable by people and LLMs.
4. **Container (standard).**
   - **Live edge:** Arrow IPC with LZ4.
   - **At rest:** Parquet with dictionary and delta encoding and zstd.
   - **Measured:** a probe of 1M synthetic core event records gave **10.5 bytes per event** with zstd (snappy 19.2; 37 raw fixed-width). Full rows with roles, anchored snapshots and edges are 20–60 bytes per event (`08` §7).
   - **No bespoke binary codec.** Hand-packing might gain a small constant factor (a judgement, not measured), and it would cost all off-the-shelf tooling (DuckDB, Arrow, Python).
5. **Generator (the real compression).**
   - Seed + manifest + journal + build reproduce everything exactly. The engine is the decompressor.
   - The stored tiers are caches for speed, like a video codec: keyframes are the I-frames, events and step digests the P-frames.
   - The tiers are: the generator (KB) → keyframes → events and digests → the life-course log → **virtual telemetry** (computed, not stored).

### 2. Virtual telemetry: anyone's day, anywhere in history, computed on demand
A new system, **`S5.daily_life`**, defines one pure function:

> `activity(person, day) → [(block, place cell, activity)]`

The function depends only on:
- the person's **life-course state at the start of the day**: residence, occupation, household, age, sex, health, from the life-course log (DATA-8);
- the **regional conditions** from the step digests (season, curfew, siege, famine, plague);
- the **keyed RNG**.

It must **not** depend on what happens to anyone else that day. Encounters are resolved in a second phase, after positions are fixed.

**Consequence: random access without replay.**
- Where was Aldric at the 10th hour of 1342-11-19? Who was in the Ashford market at noon? Each answer is computed in microseconds from the life-course log and seed.
- It is exact, and identical in every viewer.
- It costs **zero storage** at any scale. The life-course log is about 0.2 GB per millennium at 100k and 1.5–2.5 GB at 1M (`08` §7).

Calibration of the schedule templates follows ActivitySim's day-pattern → tour → trip hierarchy (BSD-3, reference only; `14`) and time-use data. MATSim is GPL and is used for concepts only.

### 3. Authoritative daily life where you declare it (the fidelity dial)
The same function can run at three **activity resolutions**. A resolution is a world setting, never a side effect of looking (observer invariance, ADR-0024/0025):

| Resolution | What counts as history | Cost (estimates, ≈ 10⁸ updates/s) | Default |
|---|---|---|---|
| `life_course` | Life events, decisions, markets. Daily activity is virtual telemetry only (shown, not consequential) | Baseline | Meso and macro worlds |
| `blocks` (≈ 15 per day) | Where each person is, and with whom, per block. **Encounters become events**: meetings form ties, contacts carry infection, crimes happen | ≈ 0.06 s per sim-year per 1k persons. A 5k-person focus town adds about 5% to a 100k world; a whole 100k world ≈ 1.5 h per millennium | Micro worlds; declared **focus regions** |
| `minutes` (tile grid) | Sims-level movement and interaction | ≈ 5 s per sim-year per 1k persons: a 100-year micro story ≈ 9 min | Opt-in for micro worlds |

- **Focus regions** are declared in the manifest (`focus: [{settlement: ashford, activity_resolution: blocks}]`). Changing them mid-run is a journaled migration keyframe (`05.1` §3).
- **Inside a focus region, what you see is the history,** because viewer and engine call the same function. Outside one, what you see is exactly what that history would have looked like, but it cannot change anything.
- **Raising a region's resolution never changes how it looks.** It changes whether what happens there counts.

### 4. What you see at each zoom
| Zoom | Comparable to | Source |
|---|---|---|
| World, centuries | Grand strategy and chronicle | Step digests and significant events |
| Region, years | SimCity with persistent individuals | Events, markets, migrations, life courses |
| Settlement, days | Dwarf Fortress fortress mode | Virtual telemetry; authoritative in focus regions |
| A person, hours | The Sims | Virtual telemetry plus named-mind decision traces; minute-level only where declared |

## Alternatives considered

| Alternative | Why not |
|---|---|
| Store full movement telemetry for everyone | ≈ 11 GB per sim-year at 1M even at block resolution. Regeneration gives the same answers exactly, at zero storage |
| Simulate everyone at Sims level | ≈ 1.5 h per simulated *year* at 1M (minutes). Unaffordable, and nobody can watch more than a few hundred people at once |
| Raise fidelity where the viewer looks | History would depend on where you looked, breaking observer invariance and replay |
| A bespoke binary stream codec | A small constant-factor gain at best, at the cost of all standard tooling. The large gains come from generation |

## Consequences

- **New system `S5.daily_life`** (P1b: the activity function and virtual telemetry; P3a: `blocks` resolution with encounter events and M5 contact networks; P5–P6: `minutes` for micro worlds).
- **The World Stream gains two record kinds:** `DECISION_TRACE`, and `ENCOUNTER` (in focus regions).
- **The text viewer gains `rh-cli where <person> <date>` and `rh-cli who <place> <date>`,** answered from virtual telemetry.
- **New tests:**
  - V-STR-06: virtual telemetry equals what the engine used in focus regions;
  - V-STR-07: random access without replay, < 1 ms per person-day;
  - V-STR-08: changing a region's resolution leaves its displayed activity unchanged.
- **Cost stays under control:** high fidelity costs compute only where it is declared, and storage never grows with fidelity.

## Revisit when

- Encounter events in focus regions turn out to matter for stories more broadly. In that case, raise the meso default to `blocks` if PERF-1 allows.
- An activity model needs same-day dependence on others (e.g. following someone). Model that as a scheduled outcome or an encounter, not inside `activity()`.

## Evidence

- ADR-0006 (keyed counter-based RNG); ADR-0026 and DATA-8 (life-course log); `05.2` §3.5 (schedule templates).
- `08` §1 and §7 (event volumes, storage).
- Compression probe: `specs/stream/compress_probe.py` (synthetic, 1M events; DuckDB Parquet writer).
- Virtual telemetry prototype: `specs/stream/selftest_virtual_telemetry.py` (a stateful engine loop and on-demand queries agree exactly; about 35 µs per person-day in pure Python).
- `14`: ActivitySim (BSD-3), Random123 (BSD-3), MATSim (GPL, concepts only).
