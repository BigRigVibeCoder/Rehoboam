# ADR-0028 — One engine from one person to one million: scale profiles and micro worlds

**Status:** Accepted (director, 2026-10-16) · **Date:** 2026-10-15 · **Amends:** ADR-0003 (ledger), ADR-0010 (compiler validators), ADR-0012 (cognition tiers), ADR-0020 (scale-out); extends ADR-0027 \
**Amended by ADR-0034 (2026-10-16, D27):** deliberation consults and pull inputs retired.

**Amended by ADR-0035 (2026-10-16):** the default saga world is **meso** (a kingdom of about 10k–50k persons, 200–2,000 named actors); `val_khor` (100k) and 1M are later and optional. Mythic catalysts (awakening, inheritance, prophecy) extend §3 and land in P1b.

## Context

The director's goal: *"a simulation with one person to 1 million would be the real goal. This engine needs to be scalable and robust. What if we want one guy going through the trials of Job from the Bible?"*

The v1.1 plan was built for 100k–1M persons and quietly assumes large N in several places:
- **Schema minimums.** The manifest requires `population ≥ 100` and `region_count ≥ 20`.
- **Large-N statistics.** Certification is ensemble statistics over thousands of events. Pop-level beliefs and stylized facts are meaningless for ten people.
- **Cadence.** A monthly macro tick is far too coarse for one person's ordeal, which unfolds over days.
- **Patterns.** The liveness gate's "≥ 2 completed arcs per century" uses dynastic and polity patterns, which a household never produces.
- **Wealth.** Wealth is a per-person `wealth_band u8`. Job's 7,000 sheep, 3,000 camels, 500 yoke of oxen, 500 donkeys and servants need household holdings.
- **Catalysts are initial conditions only.** Job's trials, though, are *interventions* by an adversary acting under a wager.

## Decision

### 1. One engine, one binary, 1 → 1M persons per world
- The **target range is 1 to 1,000,000 persons** per world. Above 1M is a stretch goal through implicit individuals (ADR-0003, ADR-0020) and is not required.
- **No minimum-N assumptions anywhere.**
  - Schema minimums drop to `population ≥ 1` and `region_count ≥ 1`.
  - Every statistic declares its minimum sample and returns **INSUFFICIENT** instead of a false pass or fail.
  - No division by population without a guard.
  - Extinction of a world (the last person dies) is a valid, journaled end state.
- **The determinism contract, planes, IDs and journal are identical at every scale.** Nothing scale-specific exists in `rh-core`.

### 2. Scale profiles set configuration defaults; mechanics read only the configuration
The compiler derives a **scale profile** from the manifest: **micro** ≤ 1k persons, **meso** 1k–100k, **macro** 100k–1M. The profile **fills defaults** in the compiled manifest, and the manifest may override any of them. As with the era label (ADR-0027), **no mechanic reads the profile name**, only the explicit values. This keeps behaviour continuous: a world that grows from 800 to 5,000 people changes nothing until the manifest says so.

| Setting | Micro (≤ 1k) | Meso (1k–100k) | Macro (100k–1M) |
|---|---|---|---|
| Agenda and pulse cadence | Daily; hourly inside scenes | Weekly / monthly | Monthly; daily for active systems |
| Named minds (T1) | **Everyone**. Protagonists can be flagged `deep`: larger belief store, emotion appraisal, richer theory of mind | Notable persons, up to 5k | 1–5k |
| Economy | **Household holdings** (herds, land, servants, goods) | Holdings plus regional markets | Regional markets plus holdings for named households |
| The outside world | Off-map compartments and **off-map actors** that enter (raiders, caravans, envoys) | Off-map compartments | Off-map compartments |
| Validation | Invariants, determinism, life-course plausibility, **personal-arc liveness**, human rating | Stylized facts where N suffices; INSUFFICIENT otherwise | Full certification |
| Default pattern packs | Personal: `trial_of_faith`, `grief_and_lament`, `temptation`, `counsel_dispute`, `restoration`, `betrayal_by_friends` | Mixed | Dynastic, polity, war, plague |
| Default narrative forms | Biography, drama (dialogue), diary, novel | Chronicle, saga | Annals, chronicle, history |
| Default view | Lo-fi room, camp or house view with a dialogue panel | Settlement view | World map |

Deliberation consults, formerly on by default for `deep` minds, are retired by ADR-0034: every decision, a `deep` mind's included, comes from the simulation.

### 3. Agentive catalysts: adversaries with permission scopes
Some stories are driven by an agent who *intervenes*, such as the Adversary in Job, a curse, or a trickster god. This is modelled the way the Oracle is (ADR-0024, `05.3` §6):
- **The agent is in-world.** It is a plane-A entity with its own beliefs and a **Registry verb set**, e.g. `INCITE_RAID`, `CALL_FIRE`, `RAISE_WIND`, `AFFLICT`.
- **A permission scope bounds it,** declared by a catalyst. For Job: `wager {protagonist, scope: {spare_life: true}, stages: [...]}`.
- **Its verbs are material inputs** (journaled, scoped, bounded). The simulation decides their consequences: how many servants survive a raid, whether the house collapses on the children, how the illness runs.
- **Who drives it:**
  - by default, a deterministic policy, e.g. maximise the protagonist's probability of renouncing their faith, within scope;
  - or **you can play it**, or play its opposite, a witness or advocate in the Oracle role.
- **Outcomes are never forced.** Job may curse God or hold firm. The friends may persuade him or fail. Restoration is not scripted: whatever follows emerges from the state. That is consistent with "the simulation decides".

### 4. The micro reference world
`specs/manifest/examples/uz_job.yaml`, "The Land of Uz", is a pastoral household:
- about 60 persons: Job, his wife, 7 sons, 3 daughters, servants and herders;
- 3 friends and Elihu in neighbouring settlements;
- Sabean and Chaldean raiders as off-map actors;
- the Adversary under a wager catalyst.

It validates against the same schema as `val_khor` (100k) and `aethelgard` (1M). It is the **N ≈ 60** golden world.

### 5. Robustness across the range is tested, not assumed
A new **V-SCALE** suite (`09`):
- invariants, determinism and journal replay at N ∈ {1, 10, 100, 1k, 10k, 100k, 1M};
- no statistic reports pass or fail below its declared minimum sample;
- per-tick cost scales with active work, not with fixed overheads, so a 1-person world runs a millennium in seconds.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| A separate "character engine" for small worlds | Two engines diverge. The point is that Job's household and a 1M realm are the same world model at different scales. |
| Scale as a mode that mechanics branch on | Hidden thresholds, as with era labels (ADR-0027). |
| Scripted trials (fixed events at fixed dates) | It contradicts "the simulation decides"; the adversary-with-scope pattern gives the same drama without forcing outcomes. |

## Consequences

- **Schema:** the population and region minimums drop to 1. The validator is scale-aware (carrying capacity and energetics still apply). There is a new catalyst kind `wager` and holdings on households.
- **Registry:** a template `pastoral_household`, adversary verbs, attributes `faith(actor, deity)` and `integrity(actor)`, and the personal pattern packs.
- **M4 gains household holdings at F1** (also needed for wealth inequality at 1M).
- **rh-minds gains the `deep` flag** (P3a).
- **Roadmap.** The walking skeleton (WP P1.0) runs at N = 1 and N = 100k from its first week. The Job world becomes watchable at the end of P3b, once minds and persuasion exist. It becomes inhabitable in the lo-fi Street view.
- **Cheap micro worlds make expensive features affordable,** such as `deep` minds, rich narration (dialogue, per-scene prose) and many Foresight rollouts. That gives a good test bed for cognition quality before 1M scale.

## Revisit when

- P1.0 shows a fixed per-tick overhead that dominates at N < 100.
- A micro world needs a mechanic that cannot be expressed through the effect language or the adversary pattern.
