# ADR-0018 — Godot 4.7+ separate process, Rust gdext, painted miniature, VAT crowds

**Status:** Proposed · **Date:** 2026-10-15

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
7. **Spikes first**: crowd, street-from-seed, and wallpaper power.

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

- Spike 1 (crowds) fails its targets, or Godot ships GPU-driven skinning or culling.

## Evidence

R06 (all); R01 §R1.
