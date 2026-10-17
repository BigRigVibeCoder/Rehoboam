# ADR-0035 — Legends over ledgers: the saga engine

**Status:** Accepted (director's directive, 2026-10-16; the roadmap, the guardrails, the engine-purity principle and the later directives on audio delivery, the event envelope, stress and the red team's five gaps ratified the same day) · **Date:** 2026-10-16 · **Amends:** ADR-0028 (default world scale), ADR-0029 (art phase 2 cut; Kenney CC0 tiles join the glyph packs), ADR-0018 (cut), ADR-0019 (audio chronicle elevated and delivered locally; Godot audio cut), ADR-0001 and ADR-0004 (Godot consumers cut), ADR-0016 (the console is not served by the engine) · **Amended by:** ADR-0036 (D51, 2026-10-17: directive 5 and the "Cut" row of §5; a procedural voxel viewer is allowed as a plane-B observation harness) · **Extends:** ADR-0028 §3 (mythic catalysts), ADR-0017 (graphic chronicle, visual style card), ADR-0007 (the event envelope, typed motive edges), ADR-0012 (stress and mental breaks) · **Spec:** `05.11`, `05.2` §4, `specs/systems/S6.artifacts.odd.toml`, `specs/systems/S7.vows.odd.toml`, `11`

## Context

The director's directive of 2026-10-16, "Hero vs. wheat":

> Rehoboam must be a **saga engine** (Silmarillion, Dwarf Fortress legends), not an agrarian-economy simulator. Wheat, demography and taxes are the stage lighting and the pressure, not the story.

**The director is a reader and listener, not a gamer.** They want to:
- **listen** to an episodic audio chronicle of their world, multi-voice, as local MP3 chapters;
- **read** graphic-chronicle storyboards: the sifted key scenes as illustrated panels, with clickable causal links;
- **scrub** an interactive causal timeline;
- **glance** at a 2D ambient diorama built from CC0 tiles.

Five directives:
1. Keep P0 intact: SPR-001 to SPR-010, the three planes, determinism.
2. Put **artifacts** and **vows, grudges and quests** into plane A.
3. Support **mythic catalysts** as scenario seeds.
4. Put the chronicle pipeline (text, storyboards, images and audio, read in a local reader) before interactive GUI clients.
5. No custom 3D assets: Kenney CC0 2D tiles and glyph layers for every visual. *(Amended by ADR-0036, D51, 2026-10-17: still no hand-made 3D models, rigs or textures; the procedural voxel viewer `rh-view`, a plane-B observation harness, is allowed.)*

**Emphasis** (a guide for effort, not a quota): about 30% environmental pressure (famine, plague, war), 40% dynastic ambition, vows, betrayals and relics, and 30% story sifting, the illustrated chronicle and TTS.

**Rehoboam's edge over Dwarf Fortress-style legends.** Legends there are reconstructed from event logs, so a reader often has to guess why a tragedy happened. Here every event carries typed causal and motive edges into an explicit causal DAG (ADR-0007), so every tragedy points to its origin, and the downstream tools never have to guess a motive.

**The plan already held much of this.** Agentive catalysts (ADR-0028 §3), the dynasty system M7, minds (`05.3`), the narrative compiler (ADR-0017), the audio drama (ADR-0019) and the lo-fi view (ADR-0029) were all there. **The ordering was wrong for this director.** The economy (M4 F1 markets, wages, prices) came before minds. Minds came after the thesis gate (P3a). Narration and audio came last (P4, "optional"). A reader would have waited about 18 months for the first chapter.

## Decision

### 1. Rehoboam generates legends
**North star:** *Rehoboam generates legends: a deterministic world whose dynasties, vows, relics and betrayals become sagas you can listen to, read and scrub through.*

### 2. Two new plane-A systems
- **`S6.artifacts`** owns unique, singular objects. Each one has an identity, an **origin event**, a kind, exactly one bearer or one location, an **unbroken bearer chain**, **physical state** (fixed-point charge, wear or condition, contaminant bitflags such as blood or ash, each change storing its cause), a fixed-point influence profile and a rumour reach. Its influence acts on the bearer's mind only through lifecycle commands to the minds system, which owns traits. It emits `ARTIFACT_FORGED`, `ARTIFACT_PASSED` (each with its cause: inheritance, theft, gift, trade, loss), `ARTIFACT_USED`, `ARTIFACT_FOUND`, `ARTIFACT_DESTROYED` and `BEARER_CORRUPTED`.
- **`S7.vows`** owns vows, grudges and quests: persistent directional goals, such as `AVENGE_KIN(father, murderer)`, that last until fulfilled, broken or void (the vower died). It feeds the minds agenda through a bounded, fixed-point priority weight that never enters a survival decision. Vows can fail and can break under pressure, with consequences (opinion, honour, curse hooks). It emits `VOW_SWORN`, `VOW_FULFILLED`, `VOW_BROKEN` and `VOW_VOIDED`.
- **The minds system gets a Registry id, `T1.minds`** (the `rh-minds` crate of `05.3`). It has no spec yet; its spec is written before the P1a minds sprint.
- **Stress and mental breaks join minds v0** (P1a; inspired by Crusader Kings 3's design, with no code taken). A named actor's **stress** is a `q8_8` column on a 0–100 scale, owned by minds. It accumulates when a vow, kin loyalty or a value conflicts with the situation, such as being made to serve a father's murderer; `S7.vows` publishes a vow-conflict signal (`vow.conflict`) that minds reads. Crossing the Registry threshold is a **deterministic state transition** that schedules the **mental break** as an outcome (a timer, ADR-0025, at a keyed delay): when it fires, the actor drops rational self-preservation for a volatile act from a Registry break table (an assassination attempt, flight, burning their holdings, a pact of despair), and stress drops. The consequences are deterministic and journaled; the simulation decides (ADR-0034).

Both drafts are in `specs/systems/` and pass `lint_odd.py`. Both follow the coupling rules of RT-2026-10-16 and have an `[isolation]` forcing.

### 3. Mythic catalysts (extends ADR-0028 §3)
Manifest scenario seeds such as *"in year 100 an ancient entity awakens"*, *"a cursed relic passes to a farmer's nephew"*, or a prophecy. Each is either an initial condition or a **scheduled, journaled material input** with a scope. Every consequence is simulated, never forced (WC-5, SIM-16).

### 4. Secrets v0 and the information horizon
Betrayal needs someone not to know. P1b adds minimal epistemics (SAGA-4). Every hidden or witnessed act creates a knowledge record held at first by `{actor, accomplices, witnesses}`: witnesses come from S5 co-presence where the world runs at `blocks` or `minutes`, and from a keyed witness hazard elsewhere. The record spreads **only through scheduled outcomes** (ADR-0025): a discovery timer whose hazard rises with evidence (contaminant marks, a missing item, a body) and with patrol presence; an alarm that broadcasts within the settlement; eyewitness flight; news travelling along the route graph at travel speed; testimony and confession. **No actor reacts to an event before holding that knowledge:** every reaction references its knowledge-acquisition event, so a riot cannot precede discovery. Acoustic physics (decibels, damping) is deferred, because it would be precision with no data behind it. Full epistemics (`05.3` §4) stays in P4.

### 4a. The red team's five gaps (ratified 2026-10-16)
A prototype run by the red team found five ways the plan could still produce events that nothing in the world caused. Each is now a requirement with a test:

| Gap | Resolution | Where |
|---|---|---|
| **Spontaneous props** | Every artifact or prop in a chronicle-worthy event already exists, with an origin event (placed at world creation and journaled, introduced by a catalyst, or crafted). Unique artifacts keep an unbroken bearer chain (forged, passed with its cause, found, used) and physical state. Mundane props (a phial of poison, a knife) are drawn from an owner's stock through a simulated acquisition event. No event uses an item before its origin; every bearer change is an event | SAGA-12, V-SAGA-17; `S6.artifacts`; M4 F0 stocks |
| **Knowledge without a path** | The information horizon (§4) | SAGA-4, V-SAGA-11, V-SAGA-18 |
| **An empty throne** | When a titleholder dies, the death lifecycle command schedules M7's succession arbitration in the same instant, at a later phase (legal heir, claims, military proximity, treasury control); it emits a succession event or an interregnum-crisis event. No realm waits for the next yearly pulse | SAGA-13, V-SAGA-19; `M7.dynasty` (P1a) |
| **Teleporting fugitives** | After a crime or a mental break an actor can become a fugitive. Flight is a sequence of scheduled moves along the M1 route graph toward a border, a sanctuary or an ally, with an interception hazard per segment from garrisons and patrols. Tile-level paths only in focus regions and micro worlds | SAGA-14, V-SAGA-20; `S3.migration` (P1b) |
| **Change without a cause** | The "Legends reverse key": damage or transformation state stores its cause EventId in the value itself (`Ruin { cause: EventId }`, `Scar { cause: EventId, site }`, a captured or cursed artifact). "What last happened here?" for arbitrary places is a plane-B index over the Parquet chronicle by place, cell or entity, not a last-mutating-event column on every tile | SAGA-15, V-SAGA-21 |

### 5. The roadmap is rebalanced (`11`)

| Phase | Name | Gate |
|---|---|---|
| P0 | Foundations: unchanged, SPR-001 to SPR-010 | G0 |
| P1a | **Bloodlines:** worldgen-lite, M2 demography, M7 succession and dynasts with immediate succession (moved up), minds v0 with stress and mental breaks (moved up), the Chronicle Writer with the event envelope (moved up), the World Stream v1; rungs `solo` → `household` → `hamlet` | **G1a:** a dynasty lives, dies and inherits deterministically, and the templated chronicle reads as a family history |
| P1b | **Legends:** artifacts with provenance, vows, mythic catalysts, secrets v0 and the information horizon, fugitives; pressure at low fidelity (M3 harvest F0/F1, M4 F0 stores and wealth without markets, M6 polity-lite); rungs `village` → kingdom (≈ 10k–50k persons, 200–2,000 named actors) | **G1b:** sagas form; plausibility bands hold; V-SF-22 and V-SF-23 reported |
| P2 | **Sagas & audio v0:** the sifter (SQL twins, significance, saga pattern packs), the keyboard Time-Machine, the templated chronicle, the **offline audio exporter** and reader v0 | **G2 (thesis):** method unchanged; the rated items are sagas; a report-only saga-mechanics ablation beside the coupling ablation |
| P3 | **The illustrated saga:** narrative compiler, multi-voice audio drama, **graphic chronicle**, the local Web Graphic Novel / Storyboard Reader with the causal timeline | **G3:** you listen to and read a season of your world's saga, rated |
| P4 | **Deeper worlds:** full epistemics, contagion, war, Oracle, Foresight, whispers, the translator side of the LLM gateway, the world compiler, persuasion, the other should-systems | G4 |
| P5 | **2D diorama:** the glyph client, Kenney CC0 tile packs, a SimCity-style settlement growth view | G5 |
| Later | 1M scale-up (the default saga world is meso), M4 F2, era packs, the lo-fi street-level inhabitant | — |
| Viewer lane (ADR-0036, D51) | `rh-view`, the procedural voxel viewer: a read-only plane-B observation harness after SPR-027 (PRJ-003), off every gate's critical path; Gandalf's inputs with P2.7 | — |
| Cut | Every custom-3D (hand-made models, rigs, textures), Godot and VR work package (the procedural voxel viewer is ADR-0036's, not cut) | — |

### 5a. The engine simulates truth; downstream tools generate perception (ratified 2026-10-16)
**The engine simulates ontological truth:** matter, space, time, state and the causal graph. **Downstream tools generate perception:** pixels, prompts, dialogue, sound and prose. This is the rule behind §6, §7 and §7a: if a fact changes what can happen next, it is simulated and stored in plane A; if it only changes how something looks, sounds or reads, it is derived downstream in plane C.

### 6. The engine only engines (ratified 2026-10-16)
- **No presentation code in the engine.** The Rust engine crates (`rh-core`, `rh-ids`, `rh-events`, `rh-registry`, `rh-contracts`, `rh-snapshot`, `rh-sys-*`, `rh-sifter`, `rh-server`, `rh-cli`) contain no presentation, web UI, audio, image, TTS or LLM code. The engine never waits for media and never knows what an image or an audio file is.

  *Note (2026-10-16, consistency pass, PRJ-002 §10 item 17): this list omits two built engine crates, `rh-sample` (plane A, SPR-003) and `rh-manifest` (tool plane, SPR-004), and the planned `rh-minds` and `rh-worldgen`. The decision is unchanged; the full list is in `05.0` §6.*
- **Outputs:** the World Stream only: channel A (events) and channel C (sift matches and significance) as Arrow/Parquet segments. The sealed segments are the chronicle (`05.10`).
- **Inputs:** journaled inputs through the one local control interface (IF-0, ADR-0030). It carries tip actions and forks. It is a control and stream endpoint, not a presentation server, and it stays.
- **Presentation is downstream.** Every presentation tool is a separate, offline consumer that reads the sealed chronicle or tails the stream, and never calls back into the engine: the storyboard and comic generator, the map export, the offline audio exporter, and the local Web Graphic Novel / Storyboard Reader live in `python/`; only the P5 web diorama lives in `console/`. The LLM gateway is outside the engine too (ADR-0034).
- **Enforcement:** a planned lint rule (V-INSP-05; `13` §3): engine crates may not depend on media, HTTP-UI or LLM-client crates. One deny-list, kept in one place, e.g. `image`, `rodio`, `cpal`, `symphonia`, `reqwest`, `axum`, `tower-http`, `async-openai`, `tokenizers`. It is Architect backlog item A-14 (P1a).

### 7. The chronicle pipeline is plane C, downstream (`05.11`)
Chronicle text → episode scripts → storyboard scripts → images and voices → local MP3 chapters and pages → **the local Web Graphic Novel / Storyboard Reader**, the only presentation surface for sagas. Nothing is served or published: an **offline audio exporter** in `python/` reads the chronicle, synthesises the voice tracks, mixes ambience and writes local MP3 chapters. Every stage is a narrator-out role (ADR-0034), writes plane C, carries provenance and has a template fallback.

### 7a. The cinematic event envelope (`05.2` §4.1a)
Downstream tools need more than an actor, a target and a verb. Every chronicle-worthy event carries five groups of facts. **The engine only engines applies to the envelope itself:** only *simulated* facts are stored in channel A; anything purely presentational is derived downstream, in plane C, from those facts.

| Group | Stored in channel A (plane A) | Derived downstream (plane C) |
|---|---|---|
| 1. Cast | Actor, target and witnesses as event **roles** | Visual DNA (the P3 media table under Consequences) |
| 2. Staging | Place id, tick and **cell coordinates** where the world tracks cells; **weather as an anchored snapshot** once M3 simulates it | Time of day from the tick and calendar; lighting from time of day, place type and weather; the place's acoustic from its type (a Registry table) |
| 3. Emotional vector | The actor's and target's emotional state at that moment, as **anchored snapshot attributes** (`emotion@e`: PAD mood plus the strongest OCC emotion, fixed point), where minds simulate emotion | Where it is not simulated (pops, minor actors), a template inference from the act class, labelled as inferred |
| 4. Action and props | The verb; the **instrument role**: the S6 artifact id when an artifact was used; the instrument's or cell's **contaminant bitflags** at the event, as an anchored snapshot | Prop status ("blood dripping", "glowing") from the verb and outcome |
| 5. Causal "why" | **Typed motive edges** (Registry edge types such as `RETALIATION_FOR`, `JEALOUSY_OF`, `VOW_OF`) to the inciting event, extending the causal and contingency edges | Narrative phrasing |

The stored part costs about 35 bytes per event beyond what the event already holds, about 43 where cells and contaminants are tracked. Its attributes are declared in the Registry `[attributes]`, so patterns read them anchored (`attr@eN`). It is built by WP P1.9 (P1a). P0's event schema (SPR-006) must leave room for roles, anchored snapshots and typed edges; SPR-006 itself is not changed (BCK-001 §2).

### 8. The three guardrails (ratified by the director, 2026-10-16)
1. **Lean demography stays as the pressure cooker.** Births, deaths, kinship and succession remain simulated, because dynasties need them and a famine must actually starve people before anyone steals a relic over it. The economic-history depth is what drops.
2. **Vows must be able to fail.** A vow can be broken under pressure, its vower can die first, and a relic can be lost. Outcomes are simulated, never scripted (ADR-0034); this is the main defence against sagas that feel scripted (R-31).
3. **The G2 thesis gate stays pre-registered and honest.** It is rated on template-only text against the coupling-ablated control, with a report-only ablation of the saga mechanics beside it.

These hold alongside the rules already stated above: P0 and the non-negotiables do not move; the chronicle pipeline is plane C and downstream of the engine (§6, §7); no custom 3D (§5; ADR-0036 allows the procedural voxel viewer as an observation harness).

## What stays

- **Demography stays as the dynasty substrate,** but lean: births, deaths, kinship, households and succession. Brass stays (`specs/demography/`).
- **Pressure systems drop to low fidelity** until P4: M3 harvest F0/F1, M4 F0 (household and lord stores and wealth, no markets), M6 polity-lite (legitimacy and unrest). M4 F1 markets and the full M6 structural-demographic core move to P4.
- **Every band is kept.** The economic-history bands that the new gates do not need (welfare ratios, price dispersion, income inequality, Malthusian wage tests) become **soft / report**. Demography, dynasty duration, famine mortality and succession bands stay gates (`09` §3.1).
- **The G2 method is unchanged:** pre-registered, rated on template-only text, against the coupling-ablated control.
- **The complexity ladder** keeps rungs 0–3. Rung 4 becomes a kingdom of ≈ 10k–50k. `val_khor` (100k) and 1M become later and optional.

## Alternatives considered

| Alternative | Why not |
|---|---|
| Keep the economy-first order and add narration earlier | A reader still waits a year for the first saga, and the economy WPs absorb the review hours that minds, vows and relics need |
| Scripted saga beats (authored quest lines with fixed outcomes) | It breaks "the simulation decides" and makes sagas feel scripted, which is the main risk this ADR names (R-31). Vows and relics are state; their outcomes are simulated |
| Vows and artifacts as plane-C narrative readings of ordinary events | They would not change behaviour: a vow that does not move its vower's agenda is a caption, not a cause. They must be plane A, with lifecycle commands into minds |
| Keep the optional 3D art phase | The director is not a gamer and does not want custom 3D. It was the largest content cost in the plan (R-28). *ADR-0036 (D51) later allowed a procedural voxel viewer as an observation harness: voxels remove the hand-made content cost, and the red team needs to see the world* |
| Local image generation on the laptop | Its GPU is 4 GB, too small for SDXL or Flux. `adjutant` has no GPU |
| A GPU VM on demand for images | Viable, but it adds operations work for a batch job a serverless API does per panel. Kept as the fallback if API terms, quality or consistency fail (`12` Q24) |

## Consequences

- **Roadmap (`11`).** P0 to G2 is about 37–51 weeks (it was 33–47), because minds and M7 move before the thesis gate. The first listened-to season (G3) arrives at about 11–16 months instead of after P4 (about 19–25 months). The program total, without later and optional items, is about 76–105 weeks.
- **Requirements (`03`):** new SAGA-1 to SAGA-15, SN-12 and EVO-4. R3D-1, R3D-3, R3D-5, R3D-6 and ORA-7 are withdrawn. Phases are re-mapped.
- **Tests (`09`):** new V-SAGA suite (V-SAGA-01 to V-SAGA-21) and V-INSP-05; V-HUM-13 (the G3 season); V-R3D-02, 03, 05 and 09 withdrawn; band gate status re-tagged (§3.1).
- **Registry:** systems `S6.artifacts`, `S7.vows` and `T1.minds`; artifact and vow kinds; six act classes (`forge`, `pass`, `destroy`, `corrupt`, `fulfil`, `lapse`); the `ARTIFACT_*` and `VOW_*` verbs; the envelope's event roles, motive edges and attributes; a mental-break table; place acoustics; a visual vocabulary sketch; the first visual style card.
- **Crates (`05.0` §6):** `rh-sys-artifacts` and `rh-sys-vows` are added. `rh-godot` and `client_godot/` are removed.
- **Risks (`10`):** new R-31 (sagas feel scripted), R-32 (graphic-chronicle character consistency) and R-33 (image and TTS cost and access). R-09 (Godot crowds) is retired.

### P3 media choices: specified, to be evaluated
These are specified now and evaluated in P3. The items marked **decision** are settled.

| Topic | Specified | Status |
|---|---|---|
| Visual DNA | Every named actor's look is `visual = H(world_seed, person_id, "visual")`, unpacked into locked prompt tokens (physiognomy, colouring, build, signature items) drawn from a **Registry visual vocabulary** per culture and era. No LLM invents a visual description | **Decision** |
| Visual DNA is plane C | It is **not plane-A state.** It is a pure plane-C function of plane-A facts, like virtual telemetry: person id, genealogy and visible state (age, scars and injuries, artifacts borne, regalia, health). Its hash purpose is a plane-C purpose and never consumes a plane-A RNG draw | **Decision** |
| Heritable looks | A child's base tokens derive from the parents' tokens with a keyed variation, so dynasties resemble one another | **Decision** |
| Panel consistency | Panel 0 is a canonical character sheet, generated once, when an actor first becomes a sifted saga's protagonist. Later panels are conditioned on it (reference embedding or image-to-image). Their prompts describe only the setting and the action | **Decision** (method); InstantID, PuLID and IP-Adapter are candidates to evaluate in P3 |
| Visual style card | `dark_fantasy_ink`: no photorealism; a dark-fantasy graphic novel with heavy ink, cross-hatching, high-contrast chiaroscuro, muted watercolour wash, bold angular silhouettes and textured paper. **No living artist's name** in any prompt or document. It sits beside the prose style cards (WP P4.3, now in P3) | **Decision** |
| Rendering | Asynchronous batch, after a saga is compiled, through a serverless image API (candidates fal.ai, Replicate, Together AI; models FLUX.1 [dev] or SDXL). About $0.005–0.015 per panel, so $0.10–0.15 per 10-panel chapter (**estimate, verify at P3**). Images are plane C, with provenance (provider, model, seed, prompt hash, reference id) and a cache; they are not bit-reproducible across providers, which is acceptable for narration | Specified; provider and monthly budget open (`12` Q24, D29) |
| Narration voice | Kokoro-82M, faster than real time on a CPU, so it runs on the laptop | Specified, to be evaluated in P2 |
| Character dialogue | Chatterbox or Qwen3-TTS | Specified, to be evaluated in P3 |
| Soundscapes | Ambience beds and one-shots from CC0 or CC-BY libraries, in the provenance ledger | Specified |
| Where media are written | MP3 chapters and transcripts are written by the Python narrative pipeline in plane C (`python/`), never by the engine | **Decision** |
| Delivery | Local only. The offline audio exporter writes local MP3 chapters; they play in the local Web Graphic Novel / Storyboard Reader, the only presentation surface for sagas | **Decision** |
| Storyboard framing | Four panels per scene (inspired by Wildermyth): 1 staging (wide shot, atmosphere), 2 confrontation (medium shot, tension), 3 climax (close-up, the act), 4 consequence (aftermath), each filled from the event envelope; panel 0 is the character sheet | **Decision** |
| Maps | An export of the region graph and province state to an Azgaar Fantasy Map Generator map file (P2 candidate WP) | Specified, to be evaluated |
| Settlement layouts | Watabou's Medieval Fantasy City Generator (GPL-3.0; allowed in personal-use mode) as a presentation-only layout source seeded per settlement (P5 candidate) | Specified, to be evaluated |

### Risks this ADR takes on
- **Sagas feel scripted (R-31).** Vows and relics could become predictable plot devices. Mitigations: vows fail and break under pressure; the simulation decides every outcome; per-pattern firing budgets; the G2 diversity guard; V-SAGA-10 requires every vow kind to end in more than one way.
- **Character consistency across panels (R-32).** Mitigations: deterministic visual DNA, a canonical sheet per protagonist, reference conditioning, and a consistency report (V-SAGA-08) before it becomes a gate.
- **GPU access and cost (R-33).** Mitigations: serverless batch rendering after compilation, a cache, a monthly budget you set, and storyboards that are useful without images.

### What this amends
Each ADR below carries an "Amended by ADR-0035" line under its status.

| ADR | Change |
|---|---|
| ADR-0001 | GDScript and Rust gdext for a 3D client: cut |
| ADR-0004 | `rh-worldgen` is shared by the server only; `rh-godot` is cut |
| ADR-0016 | The web console is not served by `rh-server`; the saga reader (P3, `python/`) and the diorama (P5, `console/`) are separate consumers of the stream |
| ADR-0018 | The Godot 3D client is cut (it was deferred by ADR-0029) |
| ADR-0019 | Godot-native ambience is cut; the audio chronicle becomes core (P2 narration, P3 drama), exported as local MP3 chapters played in the reader |
| ADR-0028 | The default world scale is **meso** (a kingdom of ≈ 10k–50k); 100k and 1M are later and optional. Agentive catalysts are extended by mythic catalysts |
| ADR-0029 | Art phase 2 (3D) is cut; Kenney CC0 tile packs join the glyph packs in P5 |

## Revisit when

- **G2 fails** with the saga-mechanics ablation showing no gain: the vow and relic mechanics are not earning their place, so revisit their design before P3.
- **V-SAGA-08** shows that no evaluated reference method keeps characters recognisable: ship storyboards with tile and text panels, and keep images optional.
- **Image or TTS costs** exceed the monthly budget you set for two consecutive seasons.
- **You ask for an inhabitable modality** (walking the streets, talking to people): the later lo-fi street-level WPs come back into a phase.

## Evidence

- The director's directive (2026-10-16), the Architect's design decisions, two addenda, and the director's ratification (`decisions-log.md`).
- ADR-0028 §3, `05.3` §1–3, `05.4` §5–6, `05.8` §3, ADR-0029, ADR-0034.
- Licences checked for `docs/14` §S on 2026-10-16 against LICENSE files in the vendors' GitHub repositories; huggingface.co, kenney.nl and the image providers' sites were unreachable, so facts from them are marked "not verified".
