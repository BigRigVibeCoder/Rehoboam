# ADR-0029 — Lo-fi glyph client first; 3D becomes a later art phase

**Status:** Proposed · **Date:** 2026-10-15 · **Defers:** ADR-0018 (Godot 3D client) to art phase 2 · **Amends:** `05.7`, `11` (P5–P7) · **Amended by:** ADR-0030 (no graphical view before G2: the P1.0 glyph view becomes an ASCII map in `rh-tui`, and the P3 Diorama preview is dropped; P5–P7 are unchanged)

## Context

The director: *"For art think lo-fi for phase 1, something very minimal like Dwarf Fortress. Something that comes in a repository on GitHub, something we can bootstrap."*

The v1.1 plan made a Godot 3D "painted miniature" client the first inhabitable view (P5–P7, about 34–40 weeks with spikes SPK-1 to SPK-3). It carried the heaviest content cost in the plan: architecture kits, crowds, animation, and art per era (R-28). A glyph view (CP437 or a small tileset, people as `@`-style glyphs, in the manner of Dwarf Fortress or Caves of Qud) serves every modality:
- the Diorama, as an ambient view and desktop wallpaper;
- the Street-level Inhabitant, as an avatar plus a dialogue panel;
- the Oracle travel map;
- the Time-Machine map.

It costs a fraction as much, and a new era's art is a glyph pack, not a 3D kit.

A verified scout (R12 batch 2 companion; licences and activity checked against the LICENSE files and git history) found:
- **No roguelike framework fits as the renderer.** rot.js draws one call per cell, malwoden is dormant, and bracket-lib owns the game loop and has stale crates.
- **The best base is already in our stack.** deck.gl (MIT, Adopt) with instanced glyph quads draws ~10k cells at interactive rates, and its `OrthographicView` and `TileLayer` cover world-to-street zoom. wglt (MIT) provides the single-batch shader pattern to port.
- **Assets:**
  - **Bitlands** (CC-BY 4.0) is a full CP437 12×18 font sheet plus about 500 tintable creature sprites and map, building, item and plant sheets.
  - **Unscii-8/16** (public domain or CC0) is for text panels.
  - **Kenney 1-Bit** (CC0, licence to be archived on download) is for modern extras.
- **Wallpaper:** a web page runs as a **Lively web wallpaper**. Lively suspends it when covered and fires a playback event, so the power policy comes almost for free. macOS uses Plash.
- **Optional terminal client:** ratatui (MIT) shares the Rust frame decoder and serves headless or SSH use and contract tests.

## Decision

1. **Art phase 1 is a lo-fi glyph client inside the existing TypeScript console:**
   - **Renderer:** a deck.gl `GlyphGridLayer` (instanced quads, CP437 atlas, foreground and background palette indices, binary attributes straight from IF-4 columns), starting from the stock `IconLayer`.
   - **Art:** Bitlands as the default pack; Unscii for the DOM dialogue and log panels.
   - **Bootstrap:** the shader pattern ported from wglt with attribution. **Code only, never its bundled fonts.**
2. **Zoom levels map to the existing data:**
   - **Z0, world/travel:** server glyph tiles from regional aggregates and the chronicle, so scrubbing stays under 100 ms.
   - **Z1, settlement:** the server rasterises the shared `rh-worldgen` plan into glyph chunks.
   - **Z2, street:** a bubble subscription with `lod_profile: "glyph"`. Entities snap to cells. When several people share a cell, one glyph is shown (named first) with a count badge.
3. **Era packs for art are glyph packs.** Each is an atlas PNG, a palette and a JSON map from Registry IDs (terrain, building type, occupation, vehicle, creature, status) to a glyph, colours and variants. Variants are chosen by rendezvous hashing over `appearance`, never by client randomness. A CI lint checks that every Registry presentation key is mapped. A pack costs about 2–4 days per era.
4. **The client decides nothing.** It computes no field of view, pathing or visibility. Oracle-mode filtering stays on the server. Moving the `@` sends a journaled `PlayerAction`, and the new position arrives in the next frame.
5. **ICD additions** (`05.9`): `player.act(action)` (a journaled material input); `map.glyph_tiles(z, x, y, tick)`; `lod_profile: "glyph"` for `bubble.subscribe`.
6. **The 3D client (ADR-0018) becomes art phase 2.** It is optional and is decided after the lo-fi modalities have been used. Its designs (`05.7` Part B), research (R06) and spikes are kept. About 40% of the lo-fi work carries over to it: decoder logic, LoD policy, director scoring, era-pack keying and power policy.
7. **Optional `rh-tui`** (ratatui + crossterm, MIT): a second client of IF-3/IF-4 for contract tests and terminal use.

**Licence guardrails (ADR-0021):**
- **Allowed:** Bitlands (CC-BY 4.0, attribution tracked) and Unscii-8/16 (PD/CC0).
- **Excluded:**
  - `unscii-16-full` (GPL);
  - DawnLike (conflicting CC-BY-SA vs CC-BY statements; on hold);
  - int10h fonts (CC BY-SA);
  - unlicensed Dwarf Fortress community tilesets;
  - fonts bundled inside libraries (wglt, libtcod, bracket-lib);
  - the WTFPL `tcod` and `rexpaint` crates.
- No Dwarf Fortress name or look-alike branding.

## Alternatives considered

| Alternative | Why not |
|---|---|
| Fork a roguelike engine (bracket-lib + the Rust tutorial, rot.js) | They own the game loop or simulation, or draw too slowly; our server is the simulation. Their ideas are useful, but they are the wrong base |
| Terminal-only (ratatui) as the main view | No wallpaper, no mouse-rich inspection; it duplicates the console. Kept as an optional second client |
| Keep 3D first | Heaviest content cost and spike risk, and every era multiplies it. It delays inhabiting the world by months |

## Consequences

- **P5 becomes "Lo-fi Diorama" and P6 "Lo-fi Street-level"** (`11`). The glyph-client work is about **7–11 pw** against the Godot P5/P6 rendering work. The simulation and LLM WPs (M4 F2, hydration, schedules, dialogue, speech) are unchanged.
- **The program shortens by about 10–14 weeks**, unless art phase 2 is chosen later (about +18–24 weeks then).
- ~~**A first lo-fi view appears much earlier:** a minimal glyph view is part of the walking skeleton (P1.0). A watchable Diorama can ship during P3, about 6–8 weeks after the P2 server.~~ *Superseded by ADR-0030: phase 1 is text-only; the walking skeleton ends at an ASCII map in `rh-tui`, and the glyph client starts at P5.*
- **SPK-1 (crowds) and SPK-3 (wallpaper) move with art phase 2.** A small spike replaces them: Lively WebView2 can open a localhost WebSocket, and timers behave in a hidden page. SPK-2 (street from seed) stays, because settlement plans feed the glyph view too.
- **Per-era art cost drops from weeks of 3D kits to days of glyph packs,** which de-risks R-28.

## Revisit when

- The director wants a 3D or "painted miniature" view after using the lo-fi modalities: start art phase 2.
- A deck.gl glyph benchmark misses 10k cells at 60 FPS: fall back to beamterm (MIT, one instanced draw) or PixiJS.

## Evidence

The scout report (2026-10-15), with sources verified in shallow clones: deck.gl 9.4.0 (MIT); wglt 0.6.4 (MIT; bundled fonts excluded); rot.js 2.2.1 (BSD-3; per-cell draws); ratatui 0.30.2 (MIT); Bitlands (CC-BY 4.0; 12×18 CP437 sheet inspected); Unscii (PD/CC0 except `-full`); Lively (GPL-3.0, separate app; web wallpaper playback events).
