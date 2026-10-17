# RED TEAM TECHNICAL DOSSIER: GUI RESEARCH & RAPID VISUAL BOOTSTRAPPING

> **Document ID:** `RT-2026-10-16-GUI-ASSESSMENT`  
> **Target Subsystems:** Plane C Downstream Presentation, Matrix Render Engine, Municipal Dioramas  
> **Authority:** Red Team Leader & Chief Antagonist  
> **Status:** Strategic Assessment & Architecture Blueprint  

---

## 1. EXECUTIVE SUMMARY & THE ASSET PIVOT

The Director has confirmed access to legitimate **SimCity 2000 (SC2K)** installation assets (GOG installer / CD-ROM ISO). 

**Does this make a difference in our assessment?**  
**Yes, profoundly.** It eliminates the single biggest bottleneck of retro isometric simulation—**art asset creation**. 

SimCity 2000 contains over **1,200 hand-crafted, 256-color isometric sprites** representing decades of architectural evolution (tenements, civic halls, water towers, stone aqueducts, bridges, and disaster ruins).

However, having the assets does **not** automatically mean we should run OpenSC2K’s native Godot engine. We must separate the **Art Assets** from the **Rendering Runtime**.

This dossier evaluates the four viable GUI architectural tiers for Rehoboam, assesses how to achieve **maximum visual fidelity with minimum engineering friction**, and blueprints the **Matrix Render Engine** connecting Rehoboam’s DuckDB data lake to an interactive 2.5D diorama.

---

## 2. COMPARATIVE EVALUATION MATRIX

| Evaluation Criteria | **Tier 1: IsoCity**<br>*(Next.js / Canvas)* | **Tier 2: OpenSC2K**<br>*(Godot 4.7 / Rust)* | **Tier 3: Cytopia**<br>*(C++ / SDL2)* | **Tier 4: Custom Canvas**<br>*(Vanilla HTML5 / WebGL)* |
|:---|:---:|:---:|:---:|:---:|
| **Visual "WOW" Factor** | ⭐⭐⭐⭐⭐<br>Smooth high-DPI, infinite zoom, dynamic lighting, particles | ⭐⭐⭐⭐<br>Exact authentic 1993 SC2K retro pixel art | ⭐⭐⭐⭐⭐<br>Pinnacle modern pixel art, day/night cycles | ⭐⭐⭐<br>Clean retro 16-bit, functional |
| **Rapid Dev & Iteration Speed** | 🚀 **Instant**<br>Hot-reloading, TypeScript, zero native compile | ⚠️ **Slow**<br>Godot 4.7 build step + Rust native GDExtension | 🛑 **Very Slow**<br>CMake, C++ headers, SDL2 linking | 🚀 **Fast**<br>Single-file browser scripting |
| **License / Freedom** | **MIT** (100% clean) | **MIT** (Code only, requires user assets) | **GPL-3.0** (Copyleft) | **Public Domain / MIT** |
| **Data Lake Integration**<br>*(DuckDB / Parquet)* | 🟢 **Native Web/WASM**<br>Reads DuckDB-WASM or local REST/IPC stream | 🟡 **Requires Rust Crate**<br>Binds DuckDB Rust into Godot GDExtension | 🔴 **Heavy C++ Binding**<br>Requires manual C++ DuckDB bindings | 🟢 **Native Web/WASM**<br>Direct JSON/Arrow WebSocket |
| **Browser Dashboard Embedding** | 🟢 **100% Native**<br>Embeds directly into our web dashboard | 🔴 **Impossible / Heavy**<br>WASM Godot export is >80MB & slow | 🔴 **Complex**<br>Requires Emscripten port | 🟢 **100% Native**<br>Embeds as an `<iframe>` or component |
| **Medieval / Feudal Theming** | 🟢 **Built-in!**<br>Already includes `medeival.webp` sprite pack | 🟡 **Asset Hack**<br>Requires SCURK custom tile overriding | 🔴 **Fixed Tileset**<br>Modern city tiles hardcoded | 🟡 **Manual**<br>Must supply sprites |
| **Rehoboam Law 1 Adherence**<br>*(Engine Only Engines)* | 🟢 **Strictly Downstream**<br>Pure Plane C consumer | 🟢 **Downstream**<br>Independent binary | 🟢 **Downstream**<br>Independent binary | 🟢 **Strictly Downstream**<br>Pure Plane C consumer |

---

## 3. DEEP DIVE ON OPENSC2K (GODOT & RUST)

### 3.1 The Tech Architecture
Our inspection of `github.com/nicholas-ochoa/OpenSC2K` confirms:
- **Engine Core:** Godot 4.7 acting as the presentation viewport.
- **Native Extension:** Rust crates in `native/` (`simulation`, `formats`, `rendering`, `audio`).
- **Asset Importer:** GDScript decoder in `game/src/assets/original_game_installer.gd` and `sc2_sprite_archive.gd`.

### 3.2 The Asset Pipeline: Extracting SC2K Art
OpenSC2K’s greatest architectural value to Rehoboam is its **format decoder**:
1. It parses SimCity 2000 Windows `DAT` files, DOS resource forks, and Windows 95 PE executables.
2. It extracts raw 8-bit indexed bitmaps and maps them to the classic 256-color palette (`sc2_palette.gd`).
3. It exports them into clean, transparent PNG files categorized by tile ID (houses, police stations, churches, bridges, terrain slopes).

### 3.3 The OpenSC2K Assessment for Rehoboam
* **The Good:** Running OpenSC2K gives you instant, 100% authentic SimCity 2000 nostalgia with custom 2048×2048 map extensions.
* **The Catch:** As a Godot desktop application, it cannot easily sit inside your local command dashboard or web browser. Driving Godot from our Parquet event stream requires compiling a custom GDExtension or socket bridge.
* **The Strategic Play:** **We can use OpenSC2K’s asset extraction logic** to dump the pristine SC2K tileset into transparent PNGs, giving us the ultimate retro asset library!

---

## 4. DEEP DIVE ON ISOCITY (`amilich/isometric-city`)

### 4.1 The Source Inspection
We cloned and audited `amilich/isometric-city` in our scratch workspace. Key architectural discoveries:
1. **License:** 100% MIT (Copyright 2025 amilich).
2. **Stack:** Modern Next.js, React, TypeScript, and HTML5 Canvas.
3. **The Rendering Core (`CanvasIsometricGrid.tsx`):**
   - Implements hardware-accelerated 2.5D isometric projection:
     $$\text{Screen}_X = (X - Y) \cdot \frac{W_{\text{tile}}}{2}, \quad \text{Screen}_Y = (X + Y) \cdot \frac{H_{\text{tile}}}{2}$$
   - Depth-sorting handles elevation, stacked structures, moving vehicles, and pedestrians with zero z-fighting.
   - Built-in camera pan, infinite zoom, and mouse-coordinate picking (`screenToGrid`).
4. **The "Secret Weapon" in `public/assets/ages/`:**
   IsoCity **already includes a complete, high-resolution medieval sprite sheet** (`medeival.webp`, 605 KB) and an Enlightenment sprite sheet (`enlightenment.webp`, 702 KB), featuring:
   - Stone fortress bastions and curtain walls
   - Thatched timber tenements and water mills
   - Medieval market squares, guild halls, and cathedrals

---

## 5. THE MATRIX RENDER ENGINE: THE RECOMMENDED ARCHITECTURE

To achieve **maximum visual fidelity, zero compilation headaches, and instantaneous rapid prototyping**, we recommend a **Two-Tiered Downstream Architecture**:

```
+-----------------------------------------------------------------------------------------+
|                              PLANE A: REHOBOAM RUST CORE                                |
|  - Headless deterministic world engine (10,000 ticks/sec). Zero UI.                     |
|  - Emits immutable event rows: [Tick 120] SETTLEMENT="Alden Keep" EVENT="SIEGE_ASSAULT" |
+-----------------------------------------------------------------------------------------+
                                             │
                                             ▼ Parquet Columnar Storage
+-----------------------------------------------------------------------------------------+
|                            THE DATA LAKE (DuckDB / Parquet)                             |
|  `chronicle.parquet`  &  `settlement_state.parquet`                                     |
|  - Sub-millisecond SQL queries: filter events, trace casualties, load tile deltas       |
+-----------------------------------------------------------------------------------------+
                                             │
                                             ▼ Local WebSocket / JSON IPC Bridge
+-----------------------------------------------------------------------------------------+
|                     PLANE C: THE ISOMETRIC MATRIX RENDER ENGINE                         |
|                                                                                         |
|   +---------------------------------------------------------------------------------+   |
|   |                       VIEWPORT 1: ISOCITY WEB ENGINE (Default)                  |   |
|   |   - Embedded directly in the Command Dashboard (`dashboard.html`)               |   |
|   |   - Hardware-accelerated Next.js/Canvas 2.5D rendering                          |   |
|   |   - Dual-Skin Support:                                                          |   |
|   |       • Skin A: Built-in Medieval Era Theme (`medeival.webp`)                   |   |
|   |       • Skin B: Extracted SC2K Authentic Retro Theme (from SimCity 2000 GOG)    |   |
|   +---------------------------------------------------------------------------------+   |
|                                                                                         |
|   +---------------------------------------------------------------------------------+   |
|   |                   VIEWPORT 2: OPENSC2K GODOT RUNNER (Optional Standalone)       |   |
|   |   - Standalone desktop client for full-screen retro immersion                   |   |
|   |   - Reads `.sc2x` city snapshots exported by Rehoboam                           |   |
|   +---------------------------------------------------------------------------------+   |
+-----------------------------------------------------------------------------------------+
```

---

## 6. RAPID BOOTSTRAPPING ROADMAP (RED TEAM BACKLOG)

To bring this to life immediately without distracting the Lead Architect from Phase 0 completion:

### Milestone 1: The SC2K Asset Extractor (`RT-ACT-021`)
* Build a lightweight Python extraction script using OpenSC2K’s extraction schemas.
* Read a mounted SimCity 2000 GOG directory / ISO.
* Dump all 256-color buildings, roads, water tiles, and disaster sprites into a clean `/assets/sc2k_retro/` folder.

### Milestone 2: IsoCity Medieval Matrix Viewer (`RT-ACT-022`)
* Vendor `amilich/isometric-city` into `clients/matrix_viewer/`.
* Strip out modern 20th-century municipal taxes and car traffic.
* Activate the `medeival.webp` sprite sheet.
* Wire the viewport to a timeline scrubber that loads settlement states from Parquet events.

### Milestone 3: Live Siege & Famine Playback (`RT-ACT-023`)
* Take the event logs from `prototype_4_micropolis_siege.py` (famine depletion $\to$ riots $\to$ fire spread $\to$ West Gate breach).
* Play the disaster in **full isometric 2.5D** with particle smoke and flame shaders!

---

## 7. CONCLUSION & VERDICT

1. **Having the SimCity 2000 game files is a huge win:** It gives us the authentic 1993 sprites without any legal ambiguity on your private machine.
2. **IsoCity is our best rendering horse:** It runs directly in the browser, uses clean MIT code, already has medieval sprites, and can ingest SC2K tiles seamlessly.
3. **The Lead Architect stays happy:** All of this lives in **Plane C (downstream tools)**. The Rust core simulation remains 100% headless, pure, and fast.
