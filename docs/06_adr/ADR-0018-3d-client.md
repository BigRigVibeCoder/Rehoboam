# ADR-0018 — Godot 4.7+ separate process, Rust gdext, painted miniature, VAT crowds

> **Deferred by ADR-0029 (2026-10-15):** art phase 1 is a lo-fi glyph client; this 3D client becomes the optional art phase 2.

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

## Context

The 3D modalities need a small runtime that is wallpaper-friendly, plus crowds, procedural towns and a stylised look a small team can produce. Options assessed:

- **Godot.** Skeletal animation lags at around 100 rigged characters, so vertex animation textures (VAT) are mandatory.
- **Unreal.** Has Mass and PCG, but it is heavy, C++-centric and charges a royalty.
- **Bevy.** Has no editor.
- **Unity.** Weak first-party DOTS animation.

## Decision

1. **Engine**: Godot 4.7.x, moving to 4.8 when stable. No double-precision build.
2. **Process**: a separate process from the simulation.
3. **Hot paths in Rust gdext**: frame decode, crowd LoD, and `rh-worldgen` plans.
4. **Crowd LoD tiers**: rig, then VAT MultiMesh, then impostors, then aggregates.
5. **Art direction**: a "painted miniature" bible.
6. **Wallpaper**: a self-throttling host.
7. **Spikes first**: SPK-1 crowds, SPK-2 street-from-seed, SPK-3 wallpaper power.

## Alternatives considered

| Option | Why not |
|---|---|
| Unreal | Strongest crowds, but a poor wallpaper citizen and heavy to own. |
| Bevy, or a custom wgpu renderer | No editor; years of renderer work. |
| Unity | Licence trust and the animation gap. |

## Consequences

- Godot's crowd limits shape the LoD budgets.
- Content volume is bounded by kits, procedural generation and offline AI props, all re-shaded to the art bible.

## Revisit when

- SPK-1 (crowds) fails its targets, or Godot ships GPU-driven skinning or culling.

## Evidence

R06 (all); R01 §R1.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-C1, RT-C2). Sources of truth: ADR-0024, ADR-0025, 05.0 §2, 05.2 §3, 05.7 Part B.*

**Corrected inline:**
- Spike names now use the roadmap IDs **SPK-1** (crowds), **SPK-2** (street-from-seed) and **SPK-3** (wallpaper power).

**Changes:**
1. **The client is plane B.** It renders views. Authority never moves to the bubble. Deaths, births and other outcomes are plane-A timers that the client only displays. Your material actions in the street are journaled inputs.
2. **T3 only at Live speed** (street clock 20–60:1). Faster bands collapse the bubble to T2c crowd proxies and Spectacle. Diorama replay of computed history costs no simulation work.
3. **Hysteresis.** Hydrate at 80 m. Stay T3 until 130 m **and** ≥ 10 s out of view. Never dehydrate an entity in view. Hard cap of 200 embodied persons; budget of 5–10 hydrations per second.
4. **Spectacle** means objects with a world-scale footprint: T1 actors (dragons) and SoA objects (armies, processions, fires). They are visible to draw distance at any speed band and are interpolated on their own clocks.
5. **T2c crowd proxies are real people:** `PersonId`s sampled without replacement from living persons whose schedules place them outdoors there at that hour.
6. **`rh-worldgen`** is the same crate the server links (ADR-0004 v1.1).
