# ADR-0027 — Era-generic mechanisms: eras are state, not templates

**Status:** Proposed · **Date:** 2026-10-15 · **Amends:** ADR-0010 (templates and horizons), ADR-0012 (cognition), ADR-0022 (work packaging); extends `05.1` §3

## Context

The director asked: *"What if we want to emulate into a modern era? We don't want to be stuck in time."* A world that starts in the 13th century should be able to reach early-modern, industrial and modern conditions in one continuous run, and fantasy or speculative worlds should be able to move along their own technology paths.

The v1.1 plan would get stuck in time, for four structural reasons:

1. **Era is baked into templates.** `feudal_realm` and `modern_metropolis` are separate scenario types. Nothing moves a world from one to the other.
2. **Era-sensitive quantities are constants per world.** Examples: the regicide hazard (`murder_hazard_base`), e₀ and TFR bounds, M6 thresholds, the M8 Helmbold exponent. A constant calibrated to 1000–1500 cannot also describe 1850 or 1950.
3. **Institutions are static.** Governments, succession laws and institutions are declared in the manifest and never change. Yet the primary sources show that institutions drive the era effects (R12):
   - Under primogeniture the deposition hazard is 0.25–0.37 of that under elective succession, and no calendar trend remains after controls (Kokkonen & Sundell 2014).
   - Reigns lengthened from about 10 to about 20 years after a structural break around 790 CE, as feudal institutions arrived. Over the same centuries deposition *rose* in the Islamic world (Blaydes & Chaney 2013).
   - Regicide fell from 25 to about 2 per 1,000 ruler-years between the 7th and 18th centuries (Eisner 2011).

   A date-driven "civilising trend" is therefore wrong. Trends must come from institutional state.
4. **Validation bands are pre-modern only.** The certification bands in `09` are labelled "pre-modern unless noted". A world that modernised correctly would *fail* certification.

There is also a mechanical trap. **Fidelity switches happen only through a migration keyframe** (`05.1` §3). If modern mechanisms were separate fidelity levels, a world could not pass into modernity during a run.

## Decision

### 1. No system hard-codes an era
Every era-sensitive parameter is a **Registry-declared function of state**, written in the effect language (`05.5` §2.1): fixed point, total, monotone where theory says so. Constants remain only for quantities shown to be era-invariant.

The inputs are a small **era-state vector** of plane-A variables, each owned by one system:

| Variable | Owner | Scope | Drives (examples) |
|---|---|---|---|
| `tech[k]` adoption share per technology | `S2.technology` | region | Yields, IFR, battle lethality, logistics range, channel latency |
| `energy_per_capita` (organic vs fossil) | `M4.economy` | region | Productivity ceiling; Malthusian escape |
| `urban_share` | `S3.migration` | region | Fertility, contagion contact, mobilisation |
| `literacy` / human capital | `S1.culture` | region × class | Fertility (quantity–quality), elite credentials, belief diffusion |
| `fiscal_capacity` | `M6.polity` | polity | War finance, state breakdown risk |
| `exec_constraint` (elite bargaining, parliaments, courts) | `M6.polity` | polity | Ruler hazards, deposition, coups |
| `succession_institutionalisation` | `M7.dynasty` | polity | Regicide, heir conflict, minority crises |
| `franchise` (share of adults with a political voice) | `M6.polity` | polity | Regime type, mass mobilisation channels |
| `medical_level`, `sanitation_level` | `S2.technology` | region | Age-specific mortality; e₀ ceiling |

An **era is a label computed from this vector** (agrarian, early-modern, industrialising, industrial, contemporary, or a world-specific label for fantasy and science-fiction paths). The label is used by presentation (era art, style cards), narrative and validation band selection. **Plane-A mechanics never read the label**, so no hidden switch can fire when a world crosses a threshold. Mechanics read only the continuous variables.

### 2. Institutions change endogenously
Governments, succession laws and institutions become **state with transition rules**, declared in the Registry as a graph:
- **Adoption and diffusion.** Bass-style emulation of neighbours that are doing well, like technologies.
- **Reform.** Elite bargaining under fiscal stress, e.g. a parliament granted in exchange for taxes, or a franchise extension under the threat of revolt.
- **Revolution and conquest.** A state breakdown in M6 selects the successor regime from the Registry's permitted transitions, weighted by the era state and the winning coalition.

Each transition is an event with causal edges, so the Chronicle can tell *how* a kingdom became a republic.

### 3. Modern-capable mechanisms are present from the start, dormant until state activates them
In-run era change is never a fidelity switch. Examples:
- The M4 goods set includes an industrial sector and fossil energy, at zero share until technology makes them productive.
- M2 fertility responds to child survival, literacy and urbanisation all along. In agrarian conditions this response is flat.
- M8 battle and logistics functions take technology inputs; rail removes the forage limit when it appears.
- Communication technologies change channel latency and reach in the epistemics layer.

**Coverage grows by "era packs".** An era pack is a Registry extension plus system extensions, era-conditioned bands and hindcast tests (§5). Until a pack exists, its technologies are marked **beyond the engine frontier**. A world that reaches the frontier keeps running at the last supported era. It files a gap report ("this world is ready to industrialise"), which feeds the evolution loop (S4). When the pack ships, the world continues under the new build through a migration keyframe ("frozen past, migrated future", DET-10). **Your world is never stuck; at worst it waits for the engine.**

### 4. Calibrate the mapping across eras, not a constant per era
Each era-sensitive function is fitted to data that spans eras, with its source recorded in the Registry:
- the pre-modern end comes from the sources in `research/reading_list.md` Tier A;
- the modern end comes from modern datasets (leader exits from 1875, war and coup datasets, mortality databases, historical GDP and energy series; reading-list Tier A-M).

A function is labelled with the **range it has been validated over**. Outside that range the run carries a "beyond validated range" flag in its certification report.

### 5. Validation becomes era-conditioned and adds transition tests
- **Era-conditioned bands.** Each stylized-fact band is keyed to the era label of the window being tested (e.g. regicide per ruler-year in agrarian windows; irregular leader exits in modern windows). Bands that the sources show to be **era-invariant** stay global.
- **Transition tests (hindcasts).** A reference manifest that starts in agrarian conditions runs under a **forced technology schedule** (an exogenous inflow of inventions). We then check the shape of the transition, for example:
  - mortality decline precedes fertility decline, with a lag in the documented range;
  - growth in output per head becomes sustained;
  - the share of irregular ruler exits falls as `exec_constraint` rises.

  This tests the mechanisms, not a timetable.
- **No forced modernity.** Without technological change, worlds stay Malthusian. The vital-rate guard and the M6 cycle band still apply.

### 6. Horizons and templates are re-scoped
- **A template sets the starting state, not an era lock.** `feudal_realm` means "start agrarian". Whether the world modernises is decided by the simulation.
- **The "speculative" label now means beyond the technological frontier.** It applies to running a world past the last era pack, or forecasting a modern-start world beyond its plausibility horizon (currently 50 years). It no longer applies to passing *through* historical eras the engine supports.
- Fantasy and science-fiction technologies are ordinary Registry entries with their own effects and their own frontier.

### 7. Scale through the transition
A demographic transition multiplies population roughly 5–30 times.
- A 100k agrarian world can grow into the millions. The per-person ledger supports up to ~10M (ADR-0020), with implicit individuals beyond that (ADR-0003).
- Worlds that are part of a larger system keep the rest of the world as off-map T−1 compartments (`05.1` §3).
- Performance above 2M is batch-speed (`08`); manifests warn when a trajectory is likely to cross it.
- Named-actor cognition stays at 1–5k actors in every era.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Separate templates per era, with an "era jump" migration | A discontinuity in history: populations, institutions and grudges would be re-generated. It breaks continuity (MOE-4) and the Chronicle. |
| Era as a mode switch read by mechanics | Hidden thresholds produce artefacts at the switch and hide the causes of change from the causal record. |
| Model every era fully before P1 | It multiplies the P1 scope. Era packs let the agrarian thesis be tested first (G2) while keeping later eras reachable. |
| Treat modernity as speculative fiction | It wastes the best-documented centuries of data. The modern end is the *most* calibratable, not the least. |

## Consequences

- **More work up front in P1:** systems must take era-state inputs (hooks) even though only the agrarian range is calibrated. This is about 1–2 weeks across P1a/P1b, mostly interface work in the contract crates.
- **New work packages:** endogenous institutions (M6/M7, P3), and the era packs themselves (after G2; see `11`).
- **More data and reading:** reading-list Tier A-M (modern anchors).
- **Certification gains band selection by era label and the hindcast suite** (`09` V-ERA).
- **Two new Registry sections:** `era_state` and `eras` (sketched in `specs/manifest/registry_v0.toml`).
- **The narrative improves:** "how the kingdom became a republic" and "how the plague stopped mattering" become causal stories the Chronicle can tell.

## Revisit when

- The first hindcast run (end of P3) shows whether mechanisms calibrated at both ends interpolate plausibly through the transition.
- An era pack needs a mechanic that cannot be expressed as a function of the era state. That would mean a new system, so record it here.

## Evidence

- `research/R12_literature_verification.md`:
  - Eisner 2011, Blaydes & Chaney 2013, Kokkonen & Sundell 2014: institution-driven era effects.
  - Clauset 2018: the *shape* of the war-severity tail (α̂ = 1.53 ± 0.07) and memoryless onsets are stable over 1823–2003; scale and onset rate are not.
  - Cederman 2003: the tail needs technological change and contagion.
  - Turchin 2013: the structural-demographic mechanism applied to the antebellum and post-1960 United States, with dimensionless components.
  - Goldstone 1991: why state breakdowns faded in 19th-century Western Europe (fiscal reform, elite birth limitation, industrial labour absorption, trade and fertiliser, power sharing, policing). This is the mechanism list for era packs E2 and E3; states that lacked these broke down in the early 1900s.
- R09 §S2 (technology diffusion, loss below a population threshold); R09 TL;DR 11 (horizon rules, now re-scoped).
