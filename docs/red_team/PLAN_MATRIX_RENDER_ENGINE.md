# REHOBOAM // MASTER PROJECT PLAN: THE MATRIX RENDER ENGINE & ASSET EXTRACTION PIPELINE

> **Document ID:** `PLAN-2026-10-16-MATRIX-ENGINE`  
> **Status:** Active Execution Plan (Pillars 1 & 2 Focus)  
> **Lead Role:** Red Team Leader & Quality Auditor  
> **Target Delivery:** Multi-Scale 2.5D/3D Viewport (`war_room.html`) & Commercial Asset Ripper (`tools/asset_ripper/`)  

---

## 1. STRATEGIC PURPOSE & DECISION RECORD

Per Director instruction (2026-10-16), Pillars 3 (Comic Strip Compiler) and 4 (Kokoro Audio Chapters) are tabled to preserve tokens. We focus 100% of our energy on **Pillars 1 and 2**:

1. **Pillar 1: The Multi-Scale Viewport ("The Powers of Ten" GUI)**  
   - Tier 1: 2D Strategic Realm Map (Macro).  
   - Tier 2: 2.5D Isometric Municipal Diorama (Meso, SimCity/Stronghold style).  
   - Tier 3: 3D Voxel / Low-Poly Climax Vignette (Micro, "The Mt. Doom Camera").
2. **Pillar 2: Universal Commercial Asset Extraction Pipeline**  
   - Decoupled from simulation logic.  
   - Rips retail assets from commercial simulation titles owned by the Director into standardized 32-bit transparent PNG sprite sheets with manifest metadata.

---

## 2. COMMERCIAL TITLES TO ACQUIRE (RANKED BY REHOBOAM FIDELITY)

To maximize visual richness, the Director should acquire the following titles (readily available on GOG / Steam / physical ISO):

| Rank | Title | Source Platform | Key Assets Extracted | Why It Fits Rehoboam |
|:---:|:---|:---:|---|---|
| **#1** | **Stronghold Crusader HD** (or Stronghold HD) | GOG / Steam | Stone keeps, crenellated curtain walls, round towers, moats, drawbridges, trebuchets, catapults, siege towers, armories, blacksmiths, granaries, animated knights, archers, peasants, fire & rubble frames. | **100% PERFECT FIT.** The gold standard of medieval fortress architecture and siege warfare. |
| **#2** | **Caesar III** (or Pharaoh / Zeus) | GOG / Steam | Aqueducts, Roman stone forums, granaries, bathhouses, temples, prefectures, senates, plebeian insulae, patrician villas, rioting citizen mobs, fire collapse sprites. | **95% EXCELLENT.** Flawless for classical, ancient, and Mediterranean stone settlements. |
| **#3** | **SimCity 2000 Special Edition** | GOG / CD ISO | 1,200 iconic Maxis 256-color sprites: civic halls, courthouses, suspension bridges, water networks, terrain slopes, disaster destruction frames. | **75% RETRO NOSTALGIA.** The definitive 1993 isometric benchmark that started it all. |
| **#4** | **Age of Empires II: Definitive Edition** (Optional) | Steam | High-resolution 4K medieval European, Middle Eastern, and Asian architectural sets, fortresses, and cavalry. | **90% HIGH-DPI.** Modern high-res sprites. |

---

## 3. FOUR PHASES OF EXECUTION

```
+-----------------------------------------------------------------------------------------+
|                  PHASE 1: THE UNIVERSAL ASSET RIPPER (`tools/asset_ripper/`)            |
|  - Standalone Python CLI tooling parsing retail archives into standardized PNG sheets.  |
|  - Modules: SC2K parser, Stronghold `.gm1`/`.tgx` parser, Caesar III `.sg2` parser.     |
+-----------------------------------------------------------------------------------------+
                                             │
                                             ▼ Standardized Sprite Sheets (`assets/sprites/`)
+-----------------------------------------------------------------------------------------+
|                  PHASE 2: THE 2.5D ISOMETRIC ENGINE (MUNICIPAL DIORAMA)                 |
|  - Built on modern hardware-accelerated Canvas (adapted from `amilich/isometric-city`). |
|  - Depth-sorting, elevation, smooth camera pan/zoom, dynamic particle fire/smoke.      |
|  - Switchable skin packs: Medieval Stone, Classical Roman, or 1993 SC2K Retro.          |
+-----------------------------------------------------------------------------------------+
                                             │
                                             ▼ On-Demand Climax Trigger (U > 50 bits)
+-----------------------------------------------------------------------------------------+
|                  PHASE 3: THE 3D VOXEL CLIMAX VIGNETTE ("THE MT. DOOM ZOOM")            |
|  - Browser-native Three.js WebGL/WebGPU camera (`climax_voxel_vignette.html`).          |
|  - PBR soft shadows, torchlight flicker, volumetric fog, rotating cinematic orbit.     |
+-----------------------------------------------------------------------------------------+
                                             │
                                             ▼
+-----------------------------------------------------------------------------------------+
|                  PHASE 4: THE DIRECTOR'S WAR ROOM (`war_room.html`)                     |
|  - Unifies Tier 1 Strategic Map, Tier 2 2.5D Isometric City, and Tier 3 3D Climax      |
|    into ONE seamless, jaw-dropping browser command center.                              |
+-----------------------------------------------------------------------------------------+
```

---

## 4. DETAILED SPRINT SCHEDULE & BACKLOG TRACKING

### Milestone 1: Asset Ripper Tooling (`RT-ACT-021`)
- [x] Audit archive formats (`sc2_sprite_archive`, `sc2_palette`, `.tgx`, `.sg2`).
- [ ] Create `tools/asset_ripper/` package in Rehoboam repository.
- [ ] Implement `extract_sc2k.py`: Maxis RLE decoder + BMP palette parser.
- [ ] Implement `extract_stronghold.py`: Firefly `.tgx` / `.gm1` decompressor.
- [ ] Implement `extract_caesar3.py`: Impressions SG2 image reader.
- [ ] Implement `extract_all.py` unified CLI with directory scanning and auto-detection.

### Milestone 2: The Multi-Scale Web Viewport (`RT-ACT-022`)
- [x] Prototype 2D Canvas siege diorama (`micropolis_diorama.html`).
- [x] Prototype 3D Three.js Voxel Mount Doom vignette (`climax_voxel_vignette.html`).
- [ ] Create `docs/red_team/war_room.html` combining the 2.5D Isometric Diorama and 3D Voxel Climax into a unified dashboard.
- [ ] Add dual-skin switcher (Medieval Stronghold vs. SC2K Retro).

### Milestone 3: Parquet / DuckDB Stream Binding (`RT-ACT-023`)
- [ ] Create mock Parquet chronicle generator emitting multi-year siege timelines.
- [ ] Wire the timeline scrubber in `war_room.html` to update settlement damage state and trigger the 3D climax camera when unexpectedness $\mathcal{U} > 50\text{ bits}$.

---

## 5. TECHNICAL SPECIFICATIONS: ASSET MANIFEST SCHEMA

Every extracted asset pack will output a clean directory:
```
assets/sprites/<pack_id>/
├── manifest.json
├── terrain/
│   ├── grass.png
│   ├── water.png
│   └── elevation_slopes.png
├── structures/
│   ├── keep_stone.png
│   ├── curtain_wall.png
│   ├── gate_west.png
│   ├── granary.png
│   └── tenement_thatched.png
├── animated/
│   ├── fire_frames.png
│   └── siege_catapult.png
└── units/
    ├── knight_facing.png
    └── peasant_fleeing.png
```

`manifest.json` schema:
```json
{
  "pack_id": "stronghold_crusader_hd",
  "name": "Stronghold Crusader HD Medieval Fortress Set",
  "tile_width": 64,
  "tile_height": 32,
  "palette_type": "rgba32",
  "tiles": {
    "keep_stone": { "file": "structures/keep_stone.png", "width": 128, "height": 192, "anchor_x": 64, "anchor_y": 176 },
    "curtain_wall": { "file": "structures/curtain_wall.png", "width": 64, "height": 96, "anchor_x": 32, "anchor_y": 80 }
  }
}
```

---
*Authored by the Red Team Leader*  
*"The engine calculates history. The tools illuminate it."*
