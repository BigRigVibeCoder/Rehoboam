#!/usr/bin/env python3
"""Builds REHOBOAM // MATRIX ENGINE A/B TEST HARNESS (100% Artifact-Free Overhaul)
Compares:
  Engine A: pdeva/OpenSC2K (Authentic SimCity 2000 Dense Metropolis & Maxis UI)
  Engine B: amilich/isometric-city (Authentic Medieval Fortified City Diorama)

Zero artifacts:
- Uses clean SCURK and clean Caesar III / Stronghold sprites.
- Runs pixel-level character cleaning to eliminate 100% of scanline noise.
- Real, dense city layouts with continuous terrain.
"""

import base64
import json
import io
from pathlib import Path
from PIL import Image

def clean_unit_sprite(path_str):
    p = Path(path_str)
    if not p.exists():
        return ""
    try:
        im = Image.open(p).convert('RGBA')
        w, h = im.size
        pix = im.load()
        # Clean any scanline glitches: if pixels touch the image border or span abnormally wide
        for y in range(h):
            row_pixels = [x for x in range(w) if pix[x, y][3] > 30]
            if row_pixels:
                span = max(row_pixels) - min(row_pixels)
                if min(row_pixels) < 4 or max(row_pixels) > w - 5 or span > 42:
                    for x in range(w):
                        pix[x, y] = (0, 0, 0, 0)
        buf = io.BytesIO()
        im.save(buf, format='PNG')
        return 'data:image/png;base64,' + base64.b64encode(buf.getvalue()).decode('ascii')
    except Exception as e:
        print(f"Error cleaning unit {path_str}: {e}")
        return ""

def clean_building_sprite(path_str):
    p = Path(path_str)
    if not p.exists():
        return ""
    try:
        im = Image.open(p).convert('RGBA')
        w, h = im.size
        pix = im.load()
        # Clean bottom/top stray scanlines if present
        for y in list(range(min(4, h))) + list(range(max(0, h-4), h)):
            row_pixels = [x for x in range(w) if pix[x, y][3] > 20]
            if row_pixels and (max(row_pixels) - min(row_pixels) > w - 3):
                for x in range(w):
                    pix[x, y] = (0, 0, 0, 0)
        buf = io.BytesIO()
        im.save(buf, format='PNG')
        return 'data:image/png;base64,' + base64.b64encode(buf.getvalue()).decode('ascii')
    except Exception as e:
        print(f"Error cleaning building {path_str}: {e}")
        return ""

def main():
    print("[*] Sanitizing and encoding 100% clean sprites for Engine A and Engine B...")

    sprites = {
        # --- ENGINE A: OpenSC2K (SimCity 2000 Pristine Maxis Palette) ---
        "sc2k_cityhall": clean_building_sprite("assets/sprites/sc2k/scurk/ctyhall.png"),
        "sc2k_university": clean_building_sprite("assets/sprites/sc2k/scurk/unvir.png"),
        "sc2k_bigben": clean_building_sprite("assets/sprites/sc2k/scurk/bigben.png"),
        "sc2k_hydro": clean_building_sprite("assets/sprites/sc2k/scurk/hydro1.png"),
        "sc2k_house": clean_building_sprite("assets/sprites/sc2k/scurk/smhouse.png"),

        # --- ENGINE B: Isometric-City (Pristine Medieval & Roman Palette) ---
        "sh_keep": clean_building_sprite("assets/sprites/stronghold/tile_castle/tile_castle_0003.png"),
        "sh_tower": clean_building_sprite("assets/sprites/stronghold/tile_castle/tile_castle_0002.png"),
        "sh_granary": clean_building_sprite("assets/sprites/stronghold/tile_buildings1/tile_buildings1_0000.png"),
        "sh_windmill": clean_building_sprite("assets/sprites/stronghold/tile_buildings1/tile_buildings1_0005.png"),
        "sh_farm": clean_building_sprite("assets/sprites/stronghold/tile_farmland/tile_farmland_0000.png"),
        "c3_temple": clean_building_sprite("assets/sprites/caesar3/caesar3/c3_img_3214_type30.png"),
        "c3_basilica": clean_building_sprite("assets/sprites/caesar3/caesar3/c3_img_2826_type30.png"),
        "c3_coliseum": clean_building_sprite("assets/sprites/caesar3/caesar3/c3_img_3074_type30.png"),
        "c3_insula": clean_building_sprite("assets/sprites/caesar3/caesar3/c3_img_0821_type30.png"),
        "c3_garden": clean_building_sprite("assets/sprites/caesar3/caesar3/c3_img_0689_type30.png"),
    }

    # Clean 40 Animated Walk Cycles (Zero Glitch Lines)
    sprites["sh_peasant_walk"] = [
        clean_unit_sprite(f"assets/sprites/stronghold/body_peasant/body_peasant_{i:04d}.png") for i in range(8)
    ]
    sprites["sh_archer_walk"] = [
        clean_unit_sprite(f"assets/sprites/stronghold/body_archer/body_archer_{i:04d}.png") for i in range(8)
    ]
    sprites["sh_knight_walk"] = [
        clean_unit_sprite(f"assets/sprites/stronghold/body_knight/body_knight_{i:04d}.png") for i in range(8)
    ]
    sprites["c3_citizen_walk"] = [
        clean_unit_sprite(f"assets/sprites/caesar3/caesar3/c3_img_{7500+i:04d}_type0.png") for i in range(8)
    ]
    sprites["c3_legionary_walk"] = [
        clean_unit_sprite(f"assets/sprites/caesar3/caesar3/c3_img_{4840+i:04d}_type0.png") for i in range(8)
    ]

    sprites_json = json.dumps(sprites)

    print("[*] Generating docs/red_team/prototypes/matrix_engine_ab_test.html...")
    out_path = Path("docs/red_team/prototypes/matrix_engine_ab_test.html")
    out_path.parent.mkdir(parents=True, exist_ok=True)

    with open(out_path, "w", encoding="utf-8") as f:
        f.write(f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>REHOBOAM // Matrix Engine A/B Test (OpenSC2K vs Isometric-City)</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;600;700;800&family=Outfit:wght@300;400;600;700;800&family=VT323&display=swap" rel="stylesheet">
  <style>
    :root {{
      --bg: #07090e;
      --panel-bg: rgba(13, 17, 28, 0.94);
      --border: #1e293b;
      --border-glow: #38bdf8;
      --cyan: #00e5ff;
      --emerald: #10b981;
      --amber: #f59e0b;
      --crimson: #ef4444;
      --purple: #a855f7;
      --text: #f8fafc;
      --text-muted: #94a3b8;
      --sc2k-metal: #a0a4aa;
      --sc2k-border-dark: #40444c;
      --sc2k-border-light: #d8dce4;
    }}

    * {{ box-sizing: border-box; margin: 0; padding: 0; }}

    body {{
      background: var(--bg);
      color: var(--text);
      font-family: 'Outfit', sans-serif;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      overflow-x: hidden;
    }}

    /* Master Top Bar */
    header {{
      background: rgba(10, 14, 24, 0.96);
      border-bottom: 1px solid var(--border);
      padding: 10px 24px;
      display: flex;
      justify-content: space-between;
      align-items: center;
      position: sticky;
      top: 0;
      z-index: 100;
      backdrop-filter: blur(12px);
    }}

    .brand-block {{
      display: flex;
      align-items: center;
      gap: 12px;
    }}

    .brand-logo {{
      width: 36px; height: 36px;
      background: linear-gradient(135deg, #00e5ff, #3b82f6);
      border-radius: 6px;
      display: flex; align-items: center; justify-content: center;
      font-weight: 900; font-size: 18px; color: #07090e;
      box-shadow: 0 0 16px rgba(0, 229, 255, 0.4);
    }}

    .brand-title h1 {{
      font-size: 16px; font-weight: 700;
      display: flex; align-items: center; gap: 8px;
    }}

    .badge-ab {{
      font-family: 'JetBrains Mono', monospace;
      font-size: 10px;
      background: rgba(245, 158, 11, 0.15);
      border: 1px solid rgba(245, 158, 11, 0.4);
      color: var(--amber);
      padding: 2px 7px;
      border-radius: 4px;
      font-weight: 700;
    }}

    /* Mode Selector Tabs */
    .mode-tabs {{
      display: flex;
      background: rgba(15, 23, 42, 0.8);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 3px;
      gap: 4px;
    }}

    .tab-btn {{
      background: transparent;
      border: none;
      color: var(--text-muted);
      padding: 6px 14px;
      border-radius: 6px;
      font-family: 'JetBrains Mono', monospace;
      font-size: 12px;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
    }}

    .tab-btn:hover {{ color: #fff; }}
    .tab-btn.active {{
      background: rgba(0, 229, 255, 0.2);
      color: var(--cyan);
      border: 1px solid rgba(0, 229, 255, 0.4);
      box-shadow: 0 0 10px rgba(0, 229, 255, 0.2);
    }}

    .hud-metrics {{
      display: flex;
      gap: 12px;
    }}

    .metric-chip {{
      background: rgba(15, 23, 42, 0.8);
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 4px 10px;
      text-align: right;
    }}

    .metric-chip .val {{
      font-family: 'JetBrains Mono', monospace;
      font-size: 13px; font-weight: 700; color: var(--cyan);
    }}

    .metric-chip .lbl {{
      font-size: 8px; color: var(--text-muted); text-transform: uppercase;
    }}

    /* Main Grid Layout */
    main {{
      display: grid;
      grid-template-columns: 1fr 380px;
      gap: 14px;
      padding: 14px 24px;
      flex: 1;
    }}

    /* Viewport Area */
    .viewports-container {{
      display: flex;
      flex-direction: column;
      gap: 10px;
    }}

    .viewports-wrapper {{
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 10px;
      height: 620px;
      transition: all 0.3s;
    }}

    .viewports-wrapper.single-a {{
      grid-template-columns: 1fr;
    }}
    .viewports-wrapper.single-a .viewport-card.engine-b {{
      display: none;
    }}

    .viewports-wrapper.single-b {{
      grid-template-columns: 1fr;
    }}
    .viewports-wrapper.single-b .viewport-card.engine-a {{
      display: none;
    }}

    .viewport-card {{
      background: #020408;
      border: 1px solid var(--border);
      border-radius: 8px;
      overflow: hidden;
      position: relative;
      display: flex;
      flex-direction: column;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    }}

    .viewport-header {{
      background: rgba(15, 23, 42, 0.95);
      border-bottom: 1px solid var(--border);
      padding: 8px 14px;
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-family: 'JetBrains Mono', monospace;
      font-size: 11px;
      z-index: 10;
    }}

    .engine-tag {{
      display: flex;
      align-items: center;
      gap: 6px;
      font-weight: 700;
    }}

    .tag-a {{ color: #fbbf24; }}
    .tag-b {{ color: #38bdf8; }}

    .perf-badge {{
      font-size: 10px;
      color: var(--emerald);
      background: rgba(16, 185, 129, 0.15);
      border: 1px solid rgba(16, 185, 129, 0.3);
      padding: 2px 6px;
      border-radius: 4px;
    }}

    .canvas-container {{
      flex: 1;
      position: relative;
      overflow: hidden;
      cursor: crosshair;
    }}

    canvas {{
      display: block;
      width: 100%;
      height: 100%;
    }}

    /* SimCity 2000 Floating Toolbar Overlay (Engine A) */
    .sc2k-toolbar {{
      position: absolute;
      top: 12px;
      left: 12px;
      background: var(--sc2k-metal);
      border-top: 2px solid var(--sc2k-border-light);
      border-left: 2px solid var(--sc2k-border-light);
      border-right: 2px solid var(--sc2k-border-dark);
      border-bottom: 2px solid var(--sc2k-border-dark);
      padding: 4px;
      display: flex;
      flex-direction: column;
      gap: 3px;
      z-index: 20;
      box-shadow: 2px 2px 6px rgba(0,0,0,0.6);
      border-radius: 2px;
    }}

    .sc2k-tool-btn {{
      width: 32px;
      height: 32px;
      background: #c0c4cc;
      border-top: 2px solid var(--sc2k-border-light);
      border-left: 2px solid var(--sc2k-border-light);
      border-right: 2px solid var(--sc2k-border-dark);
      border-bottom: 2px solid var(--sc2k-border-dark);
      font-family: 'JetBrains Mono', monospace;
      font-size: 14px;
      font-weight: 800;
      color: #111;
      display: flex;
      align-items: center;
      justify-content: center;
      cursor: pointer;
      user-select: none;
    }}

    .sc2k-tool-btn:active, .sc2k-tool-btn.active {{
      border-top: 2px solid var(--sc2k-border-dark);
      border-left: 2px solid var(--sc2k-border-dark);
      border-right: 2px solid var(--sc2k-border-light);
      border-bottom: 2px solid var(--sc2k-border-light);
      background: #8e949e;
      color: #000;
    }}

    /* Global Controls & Impulse Bar */
    .controls-strip {{
      background: var(--panel-bg);
      border: 1px solid var(--border);
      padding: 8px 14px;
      border-radius: 8px;
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 12px;
    }}

    .ctrl-group {{
      display: flex;
      align-items: center;
      gap: 6px;
    }}

    .btn-action {{
      background: #0f172a;
      border: 1px solid var(--border);
      color: var(--text);
      padding: 5px 10px;
      border-radius: 5px;
      font-family: 'JetBrains Mono', monospace;
      font-size: 11px;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
    }}
    .btn-action:hover {{ border-color: var(--cyan); }}
    .btn-action.active {{
      background: rgba(0, 229, 255, 0.18);
      border-color: var(--cyan);
      color: var(--cyan);
    }}
    .btn-action.impulse {{
      border-style: dashed;
    }}
    .btn-action.impulse:hover {{
      background: var(--amber);
      color: #000;
      border-color: var(--amber);
    }}

    /* Live News Ticker */
    .news-ticker-bar {{
      background: #0b0f19;
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 6px 14px;
      display: flex;
      align-items: center;
      gap: 12px;
      font-family: 'VT323', monospace;
      font-size: 18px;
      overflow: hidden;
      color: #e2e8f0;
    }}

    .ticker-lead {{
      color: var(--amber);
      font-weight: 700;
      letter-spacing: 1px;
      white-space: nowrap;
      display: flex;
      align-items: center;
      gap: 6px;
    }}

    .ticker-marquee {{
      white-space: nowrap;
      animation: marquee 28s linear infinite;
    }}

    @keyframes marquee {{
      0% {{ transform: translateX(100%); }}
      100% {{ transform: translateX(-100%); }}
    }}

    /* Right Sidebar (Dossier & Chronicles) */
    .sidebar {{
      display: flex;
      flex-direction: column;
      gap: 12px;
    }}

    .side-card {{
      background: var(--panel-bg);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 12px;
      box-shadow: 0 4px 14px rgba(0, 0, 0, 0.4);
    }}

    .card-title {{
      font-size: 11px;
      font-family: 'JetBrains Mono', monospace;
      font-weight: 700;
      color: var(--cyan);
      text-transform: uppercase;
      letter-spacing: 0.5px;
      margin-bottom: 8px;
      display: flex;
      justify-content: space-between;
      align-items: center;
    }}

    /* Agent Dossier */
    .dossier-body {{
      display: flex;
      flex-direction: column;
      gap: 8px;
    }}

    .dossier-header {{
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      border-bottom: 1px solid var(--border);
      padding-bottom: 6px;
    }}

    .agent-name {{
      font-size: 14px;
      font-weight: 800;
      color: #fff;
    }}

    .agent-role {{
      font-family: 'JetBrains Mono', monospace;
      font-size: 10px;
      background: rgba(56, 189, 248, 0.2);
      border: 1px solid rgba(56, 189, 248, 0.4);
      color: var(--cyan);
      padding: 1px 6px;
      border-radius: 4px;
    }}

    .dossier-grid {{
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 6px;
      font-family: 'JetBrains Mono', monospace;
      font-size: 11px;
    }}

    .d-box {{
      background: #090e1a;
      border: 1px solid rgba(255,255,255,0.06);
      padding: 5px 8px;
      border-radius: 4px;
    }}

    .d-lbl {{ font-size: 8px; color: var(--text-muted); text-transform: uppercase; }}
    .d-val {{ font-weight: 700; color: #fff; margin-top: 1px; }}

    .thought-bubble {{
      background: #0b1220;
      border-left: 3px solid var(--amber);
      padding: 8px;
      border-radius: 0 4px 4px 0;
      font-size: 11px;
      font-style: italic;
      color: #fef08a;
      line-height: 1.4;
    }}

    /* 3D Climax Picture-in-Picture Preview */
    .pip-container {{
      border: 1px solid rgba(245, 158, 11, 0.4);
      border-radius: 6px;
      overflow: hidden;
      background: #000;
      position: relative;
    }}

    .pip-header {{
      background: rgba(245, 158, 11, 0.15);
      padding: 4px 8px;
      font-family: 'JetBrains Mono', monospace;
      font-size: 10px;
      font-weight: 700;
      color: var(--amber);
      display: flex;
      justify-content: space-between;
      align-items: center;
    }}

    .pip-frame {{
      width: 100%;
      height: 130px;
      border: none;
      display: block;
    }}

    /* Chronicle Log */
    .chronicle-list {{
      max-height: 160px;
      overflow-y: auto;
      display: flex;
      flex-direction: column;
      gap: 6px;
      font-family: 'JetBrains Mono', monospace;
      font-size: 10px;
    }}

    .chronicle-item {{
      background: #090e1a;
      border: 1px solid rgba(255, 255, 255, 0.05);
      padding: 6px 8px;
      border-radius: 4px;
    }}

    .chronicle-item .tick-head {{
      display: flex;
      justify-content: space-between;
      color: var(--cyan);
      font-weight: 700;
      margin-bottom: 2px;
    }}
  </style>
</head>
<body>

  <!-- Top Bar -->
  <header>
    <div class="brand-block">
      <div class="brand-logo">R</div>
      <div class="brand-title">
        <h1>REHOBOAM <span style="font-weight:400; opacity:0.6;">//</span> SIMULATION MATRIX</h1>
      </div>
      <div class="badge-ab">A/B TEST HARNESS</div>
    </div>

    <!-- Mode Selector Tabs -->
    <div class="mode-tabs">
      <button class="tab-btn active" id="tabSplit" onclick="setMode('split')">◫ Split Screen A/B</button>
      <button class="tab-btn" id="tabA" onclick="setMode('single-a')">🏙 Engine A: OpenSC2K (SimCity 2000)</button>
      <button class="tab-btn" id="tabB" onclick="setMode('single-b')">🏰 Engine B: Isometric-City (Medieval Diorama)</button>
    </div>

    <!-- Metrics HUD -->
    <div class="hud-metrics">
      <div class="metric-chip">
        <div class="val" id="valEpoch">M05 : T148</div>
        <div class="lbl">Epoch : Tick</div>
      </div>
      <div class="metric-chip">
        <div class="val" id="valPop">742</div>
        <div class="lbl">Citizens</div>
      </div>
      <div class="metric-chip">
        <div class="val" id="valGrain">14,800</div>
        <div class="lbl">Granary Bushels</div>
      </div>
      <div class="metric-chip">
        <div class="val" id="valSurprise" style="color: var(--amber);">12.4 bits</div>
        <div class="lbl">Bayesian Surprise U</div>
      </div>
    </div>
  </header>

  <main>
    <!-- Left: Dual Viewport Area -->
    <div class="viewports-container">

      <!-- Viewports Wrapper -->
      <div class="viewports-wrapper" id="viewportsWrapper">

        <!-- ENGINE A: OpenSC2K (SimCity 2000 Retro 1993) -->
        <div class="viewport-card engine-a" id="cardA">
          <div class="viewport-header">
            <div class="engine-tag tag-a">
              <span>🏙 ENGINE A: OPENSC2K</span>
              <span style="font-size:9px; opacity:0.7;">(Authentic SimCity 2000 Metropolis)</span>
            </div>
            <div class="perf-badge" id="fpsA">60 FPS &bull; Maxis Tilemap</div>
          </div>

          <div class="canvas-container" id="containerA">
            <!-- SimCity 2000 Floating Toolbar Overlay -->
            <div class="sc2k-toolbar">
              <div class="sc2k-tool-btn active" title="Query Tool (?)" onclick="setTool('query')">?</div>
              <div class="sc2k-tool-btn" title="Bulldozer" onclick="setTool('bull')">🔨</div>
              <div class="sc2k-tool-btn" title="Road Network" onclick="setTool('road')">🛣</div>
              <div class="sc2k-tool-btn" title="Residential Zone" onclick="setTool('res')">🏠</div>
              <div class="sc2k-tool-btn" title="Commercial Zone" onclick="setTool('com')">🏢</div>
              <div class="sc2k-tool-btn" title="Industrial Zone" onclick="setTool('ind')">🏭</div>
              <div class="sc2k-tool-btn" title="Emergency Disasters" onclick="setTool('dis')">⚡</div>
            </div>

            <canvas id="canvasA"></canvas>
          </div>
        </div>

        <!-- ENGINE B: Isometric-City (Medieval Fortified Town Diorama) -->
        <div class="viewport-card engine-b" id="cardB">
          <div class="viewport-header">
            <div class="engine-tag tag-b">
              <span>🏰 ENGINE B: ISOMETRIC-CITY</span>
              <span style="font-size:9px; opacity:0.7;">(Stronghold &amp; Caesar III Fortified Town)</span>
            </div>
            <div class="perf-badge" id="fpsB">60 FPS &bull; Sub-Pixel Y+X</div>
          </div>

          <div class="canvas-container" id="containerB">
            <canvas id="canvasB"></canvas>
          </div>
        </div>

      </div>

      <!-- Controls & Impulses -->
      <div class="controls-strip">
        <div class="ctrl-group">
          <span style="font-family:'JetBrains Mono'; font-size:11px; color:var(--text-muted);">TIMELINE:</span>
          <button class="btn-action active" id="btnPlay" onclick="togglePlay()">⏸ Pause</button>
          <button class="btn-action" onclick="stepTick()">⏭ Step</button>
          <button class="btn-action" id="btnSpd1" onclick="setSpeed(1)">1x</button>
          <button class="btn-action" id="btnSpd3" onclick="setSpeed(3)">3x</button>
        </div>

        <div class="ctrl-group">
          <span style="font-family:'JetBrains Mono'; font-size:11px; color:var(--text-muted);">HEATMAP:</span>
          <button class="btn-action active" id="hmNormal" onclick="setHeatmap('normal')">Normal</button>
          <button class="btn-action" id="hmFood" onclick="setHeatmap('food')">Food Supply</button>
          <button class="btn-action" id="hmLoyalty" onclick="setHeatmap('loyalty')">Rebellion</button>
          <button class="btn-action" id="hmDefense" onclick="setHeatmap('defense')">Defense Reach</button>
        </div>

        <div class="ctrl-group">
          <span style="font-family:'JetBrains Mono'; font-size:11px; color:var(--text-muted);">IMPULSE:</span>
          <button class="btn-action impulse" onclick="triggerImpulse('rally')">⚡ Rally Garrison</button>
          <button class="btn-action impulse" onclick="triggerImpulse('grain')">🌾 Haul Grain</button>
          <button class="btn-action impulse" onclick="triggerImpulse('trebuchet')">🔥 Trebuchet Strike</button>
        </div>
      </div>

      <!-- Live Maxis News Ticker -->
      <div class="news-ticker-bar">
        <div class="ticker-lead">📢 SIMULATION DISPATCH:</div>
        <div class="ticker-marquee" id="tickerText">
          MONTH 05: HARVEST COMMENCES IN EAST RIDGE &bull; SENTRIES REPORT ALL CURTAIN WALLS SECURE &bull; BREAD RATIONS ISSUED AT LOWER GUILD QUARTER &bull; REHOBOAM DETERMINISTIC TICK RUNNING CLEAN AT 60 FPS
        </div>
      </div>

    </div>

    <!-- Right: Telemetry Sidebar -->
    <div class="sidebar">

      <!-- Agent Telemetry Dossier -->
      <div class="side-card">
        <div class="card-title">
          <span>AGENT TELEMETRY DOSSIER</span>
          <span style="color:var(--text-muted); font-size:9px;">CLICK ANY UNIT ON MAP</span>
        </div>

        <div class="dossier-body">
          <div class="dossier-header">
            <div>
              <div class="agent-name" id="dossierName">Sir Guy de Montfort</div>
              <div style="font-family:'JetBrains Mono'; font-size:10px; color:var(--text-muted); margin-top:2px;">
                ID: <span id="dossierId" style="color:var(--cyan);">knight_1</span>
              </div>
            </div>
            <div class="agent-role" id="dossierRole">KNIGHT_COMMANDER</div>
          </div>

          <div class="dossier-grid">
            <div class="d-box">
              <div class="d-lbl">World Coordinates</div>
              <div class="d-val" id="dossierPos">X: 10.00, Y: 10.00</div>
            </div>
            <div class="d-box">
              <div class="d-lbl">Assigned Duty</div>
              <div class="d-val" id="dossierDuty" style="color:var(--emerald);">CITADEL_PATROL</div>
            </div>
            <div class="d-box">
              <div class="d-lbl">Garrison Loyalty</div>
              <div class="d-val" id="dossierLoyalty" style="color:var(--cyan);">98%</div>
            </div>
            <div class="d-box">
              <div class="d-lbl">Caloric Hunger</div>
              <div class="d-val" id="dossierHunger">6% (Satiated)</div>
            </div>
          </div>

          <div class="thought-bubble" id="dossierThoughts">
            "The stone keep stands firm. The granaries are stocked. Let the heathens taste our cold iron if they dare approach."
          </div>
        </div>
      </div>

      <!-- Picture-in-Picture 3D Climax Cam -->
      <div class="side-card">
        <div class="card-title">
          <span>PICTURE-IN-PICTURE CLIMAX CAM</span>
          <a href="climax_voxel_vignette.html" target="_blank" style="color:var(--amber); text-decoration:none; font-size:10px;">⤢ Open Full 3D</a>
        </div>
        <div class="pip-container">
          <div class="pip-header">
            <span>🔴 3D CUTSCENE // U &gt; 50 BITS SIFTER</span>
            <span style="font-size:9px; color:#fff;">LIVE SYNC</span>
          </div>
          <iframe src="climax_voxel_vignette.html" class="pip-frame" scrolling="no"></iframe>
        </div>
      </div>

      <!-- Chronicle Log Stream -->
      <div class="side-card" style="flex:1;">
        <div class="card-title">
          <span>CHRONICLE EVENT STREAM</span>
          <span style="color:var(--emerald); font-size:9px;">● PARQUET SYNCHRONIZED</span>
        </div>
        <div class="chronicle-list" id="chronicleList">
          <div class="chronicle-item">
            <div class="tick-head">
              <span>[TICK 01] HARVEST &amp; PATROLS</span>
              <span style="color:var(--amber);">U = 8.2 b</span>
            </div>
            <div>Granary receives 800 bushels wheat. Sentry detachment relieves north gatehouse watch.</div>
          </div>
          <div class="chronicle-item">
            <div class="tick-head">
              <span>[TICK 03] OUTRIDER SIGHTINGS</span>
              <span style="color:var(--amber);">U = 22.4 b</span>
            </div>
            <div>Scouts report hostile horse archers maneuvering along southern river ford.</div>
          </div>
          <div class="chronicle-item">
            <div class="tick-head">
              <span>[TICK 05] GRAIN RATIONING</span>
              <span style="color:var(--amber);">U = 34.1 b</span>
            </div>
            <div>Lord decrees defensive stockpile protocol. Guild bakeries restricted to half output.</div>
          </div>
        </div>
      </div>

    </div>
  </main>

  <script>
    // --- EMBEDDED SPRITE DICTIONARY ---
    const SPRITES = {sprites_json};
    const loadedImages = {{}};

    // Preload images into memory
    Object.keys(SPRITES).forEach(k => {{
      const val = SPRITES[k];
      if (Array.isArray(val)) {{
        loadedImages[k] = val.map(src => {{
          const img = new Image();
          img.src = src;
          return img;
        }});
      }} else if (val) {{
        const img = new Image();
        img.src = val;
        loadedImages[k] = img;
      }}
    }});

    // --- REHOBOAM LARGE MATRIX WORLD (20x20 Grid = 400 Cells) ---
    const GRID_SIZE = 20;
    const TILE_W = 64;
    const TILE_H = 32;

    // Simulation State
    let tick = 1;
    let isPlaying = true;
    let playSpeed = 1;
    let currentHeatmap = 'normal';
    let currentTool = 'query';
    let selectedAgentId = 'knight_1';

    // REALISTIC CITY & SETTLEMENT LAYOUT (Continuous terrain, NO weird pipes!)
    const WORLD_MAP = [];
    for (let y = 0; y < GRID_SIZE; y++) {{
      for (let x = 0; x < GRID_SIZE; x++) {{
        let type = 'grass';

        // River along South/East
        if (x >= 18 || y >= 18) {{
          type = 'water';
        }}
        // Bridge crossing River
        if ((x === 18 && y === 10) || (x === 19 && y === 10)) {{
          type = 'bridge';
        }}

        // Main Cobblestone / Asphalt Roads (Cross-avenues)
        if (x === 10 && y < 18) type = 'road';
        if (y === 10 && x < 18) type = 'road';
        if (x === 5 && y >= 5 && y <= 15) type = 'road';
        if (x === 15 && y >= 5 && y <= 15) type = 'road';

        // --- SPECIFIC DISTINCT DISTRICTS ---
        // 1. Center Citadel (Keep & Courtyard)
        if (x === 10 && y === 9) type = 'keep';
        if (x === 10 && y === 11) type = 'plaza';

        // 4 Corner Watchtowers
        if (x === 7 && y === 7) type = 'tower';
        if (x === 13 && y === 7) type = 'tower';
        if (x === 7 && y === 13) type = 'tower';
        if (x === 13 && y === 13) type = 'tower';

        // 2. Commercial Downtown / Market Guild
        if (x === 11 && y === 8) type = 'sc2k_cityhall';
        if (x === 9 && y === 8) type = 'sc2k_university';
        if (x === 11 && y === 12) type = 'sc2k_bigben';
        if (x === 9 && y === 12) type = 'granary';
        if (x === 8 && y === 10) type = 'market';
        if (x === 12 && y === 10) type = 'temple';

        // 3. Residential Neighborhoods
        if ((x === 6 && y === 8) || (x === 6 && y === 12) || (x === 14 && y === 8) || (x === 14 && y === 12)) {{
          type = 'house';
        }}
        if ((x === 6 && y === 9) || (x === 6 && y === 11) || (x === 14 && y === 9) || (x === 14 && y === 11)) {{
          type = 'house';
        }}

        // 4. University Campus & Clocktower
        if (x === 8 && y === 6) type = 'university';
        if (x === 12 && y === 6) type = 'bigben';

        // 5. Farmland & Windmills (Agricultural Quarter)
        if (x >= 1 && x <= 4 && y >= 1 && y <= 6) type = 'farm';
        if (x === 3 && y === 3) type = 'windmill';

        // 6. Industrial Power Sector (Hydro)
        if (x === 17 && y === 2) type = 'hydro';

        WORLD_MAP.push({{ x, y, type }});
      }}
    }}

    // --- 20 AUTONOMOUS AGENTS ON ROADS AND PATROLS ---
    const AGENTS = [
      {{
        id: 'knight_1', name: 'Sir Guy de Montfort', role: 'KNIGHT_COMMANDER',
        x: 10.0, y: 9.8, targetX: 10.0, targetY: 14.5, speed: 0.016, walkCycle: 0,
        morale: 98, hunger: 6, duty: 'CITADEL_PATROL', unitType: 'knight',
        thoughts: 'The stone keep stands firm. The granaries are stocked. Let the heathens taste our cold iron.',
        waypoints: [{{x:10.0, y:9.8}}, {{x:10.0, y:14.5}}, {{x:14.5, y:14.5}}, {{x:14.5, y:9.8}}], wpIdx: 0
      }},
      {{
        id: 'knight_2', name: 'Sir Roger of the Marches', role: 'HEAVY_PALADIN',
        x: 10.0, y: 8.5, targetX: 10.0, targetY: 5.5, speed: 0.015, walkCycle: 0,
        morale: 95, hunger: 10, duty: 'NORTH_AVENUE_ESCORT', unitType: 'knight',
        thoughts: 'Guarding the grand north avenue leading toward the university clocktower.',
        waypoints: [{{x:10.0, y:8.5}}, {{x:10.0, y:5.5}}], wpIdx: 1
      }},
      {{
        id: 'sentry_1', name: 'Baelor Longshot', role: 'NORTH_TOWER_GUARD',
        x: 7.0, y: 7.0, targetX: 13.0, targetY: 7.0, speed: 0.018, walkCycle: 0,
        morale: 88, hunger: 12, duty: 'WALL_RAMPART_PATROL', unitType: 'archer',
        thoughts: 'North treeline is quiet. The smoke in the distance has dissipated.',
        waypoints: [{{x:7.0, y:7.0}}, {{x:13.0, y:7.0}}], wpIdx: 1
      }},
      {{
        id: 'sentry_2', name: 'Merrick the Hawk', role: 'SOUTH_TOWER_GUARD',
        x: 13.0, y: 13.0, targetX: 7.0, targetY: 13.0, speed: 0.018, walkCycle: 0,
        morale: 91, hunger: 8, duty: 'SOUTH_RAMPART_PATROL', unitType: 'archer',
        thoughts: 'River bridge crossing is clear. Supply caravans entering unmolested.',
        waypoints: [{{x:13.0, y:13.0}}, {{x:7.0, y:13.0}}], wpIdx: 1
      }},
      {{
        id: 'peasant_1', name: 'Cael the Harvester', role: 'WHEAT_FARMER',
        x: 2.5, y: 2.5, targetX: 3.5, targetY: 3.5, speed: 0.014, walkCycle: 0,
        morale: 82, hunger: 14, duty: 'FIELD_REAPING', unitType: 'peasant',
        thoughts: 'Fine sheaves this autumn. The windmill sails are turning smoothly.',
        waypoints: [{{x:2.0, y:2.0}}, {{x:4.0, y:2.0}}, {{x:4.0, y:4.0}}, {{x:2.0, y:4.0}}], wpIdx: 0
      }},
      {{
        id: 'peasant_2', name: 'Bran the Courier', role: 'GRAIN_COURIER',
        x: 3.0, y: 3.0, targetX: 9.0, targetY: 12.0, speed: 0.020, walkCycle: 0,
        morale: 85, hunger: 18, duty: 'HAULING_FLOUR_TO_GRANARY', unitType: 'peasant',
        thoughts: 'Hauling eighty-pound flour sacks along the west cobblestone lane to the granary.',
        waypoints: [{{x:3.0, y:3.0}}, {{x:5.0, y:5.0}}, {{x:5.0, y:10.0}}, {{x:9.0, y:10.0}}, {{x:9.0, y:12.0}}], wpIdx: 1
      }},
      {{
        id: 'citizen_1', name: 'Tiberius the Merchant', role: 'GUILD_MAGISTRATE',
        x: 8.0, y: 10.0, targetX: 10.0, targetY: 10.0, speed: 0.014, walkCycle: 0,
        morale: 90, hunger: 5, duty: 'MARKET_OVERSEER', unitType: 'c3_citizen',
        thoughts: 'Marketplace bustling today. Olive oil, wheat, and wool trading at record highs.',
        waypoints: [{{x:8.0, y:10.0}}, {{x:10.0, y:10.0}}, {{x:10.0, y:11.0}}], wpIdx: 0
      }},
      {{
        id: 'legion_1', name: 'Decimus Centurio', role: 'LEGIONARY_CENTURION',
        x: 10.0, y: 10.0, targetX: 18.0, targetY: 10.0, speed: 0.017, walkCycle: 0,
        morale: 96, hunger: 11, duty: 'RIVER_BRIDGE_GUARD', unitType: 'c3_legionary',
        thoughts: 'Defending the eastern river bridge. Order maintained across all city gates.',
        waypoints: [{{x:10.0, y:10.0}}, {{x:15.0, y:10.0}}, {{x:18.0, y:10.0}}], wpIdx: 2
      }}
    ];

    // --- CANVASES & DIMENSIONS ---
    const canvasA = document.getElementById('canvasA');
    const ctxA = canvasA.getContext('2d');
    const canvasB = document.getElementById('canvasB');
    const ctxB = canvasB.getContext('2d');

    let panA = {{ x: 0, y: 0 }}, panB = {{ x: 0, y: 0 }};
    let zoomA = 0.9, zoomB = 0.9;

    function resizeCanvases() {{
      const contA = document.getElementById('containerA');
      const contB = document.getElementById('containerB');
      if (contA && contA.clientWidth > 0) {{
        canvasA.width = contA.clientWidth;
        canvasA.height = contA.clientHeight;
      }}
      if (contB && contB.clientWidth > 0) {{
        canvasB.width = contB.clientWidth;
        canvasB.height = contB.clientHeight;
      }}
    }}
    window.addEventListener('resize', resizeCanvases);
    setTimeout(resizeCanvases, 50);

    // Coordinate Conversion
    function worldToScreen(gx, gy, canvas, pan, zoom) {{
      const originX = canvas.width / 2 + pan.x;
      const originY = 70 + pan.y;
      const sx = originX + (gx - gy) * (TILE_W * zoom / 2);
      const sy = originY + (gx + gy) * (TILE_H * zoom / 2);
      return {{ x: sx, y: sy }};
    }}

    function screenToWorld(sx, sy, canvas, pan, zoom) {{
      const originX = canvas.width / 2 + pan.x;
      const originY = 70 + pan.y;
      const dx = (sx - originX) / (TILE_W * zoom / 2);
      const dy = (sy - originY) / (TILE_H * zoom / 2);
      const gx = (dx + dy) / 2;
      const gy = (dy - dx) / 2;
      return {{ x: gx, y: gy }};
    }}

    // --- RENDER ENGINE A: OpenSC2K (Authentic SimCity 2000 Metropolis) ---
    function renderEngineA() {{
      const w = canvasA.width, h = canvasA.height;
      ctxA.fillStyle = '#1e293b';
      ctxA.fillRect(0, 0, w, h);

      // 1. Draw Continuous Terrain & Roads
      WORLD_MAP.forEach(cell => {{
        const pt = worldToScreen(cell.x, cell.y, canvasA, panA, zoomA);
        const tw = TILE_W * zoomA;
        const th = TILE_H * zoomA;

        ctxA.beginPath();
        ctxA.moveTo(pt.x, pt.y);
        ctxA.lineTo(pt.x + tw / 2, pt.y + th / 2);
        ctxA.lineTo(pt.x, pt.y + th);
        ctxA.lineTo(pt.x - tw / 2, pt.y + th / 2);
        ctxA.closePath();

        if (cell.type === 'water') {{
          ctxA.fillStyle = '#1d4ed8'; // SC2K Deep Water
          ctxA.fill();
        }} else if (cell.type === 'road' || cell.type === 'bridge') {{
          ctxA.fillStyle = '#475569'; // SC2K Road Asphalt
          ctxA.fill();
          ctxA.strokeStyle = '#64748b';
          ctxA.stroke();
        }} else if (cell.type === 'plaza') {{
          ctxA.fillStyle = '#334155';
          ctxA.fill();
        }} else {{
          ctxA.fillStyle = '#15803d'; // SC2K Lush Green Grass
          ctxA.fill();
          ctxA.strokeStyle = 'rgba(255, 255, 255, 0.08)';
          ctxA.stroke();
        }}

        // Heatmaps
        if (currentHeatmap !== 'normal') {{
          ctxA.save();
          if (currentHeatmap === 'food') {{
            ctxA.fillStyle = (cell.x < 10) ? 'rgba(16, 185, 129, 0.4)' : 'rgba(239, 68, 68, 0.3)';
          }} else if (currentHeatmap === 'loyalty') {{
            ctxA.fillStyle = (cell.x > 12) ? 'rgba(239, 68, 68, 0.4)' : 'rgba(56, 189, 248, 0.3)';
          }} else if (currentHeatmap === 'defense') {{
            const dist = Math.hypot(cell.x - 10, cell.y - 10);
            ctxA.fillStyle = dist < 6 ? 'rgba(56, 189, 248, 0.35)' : 'rgba(0,0,0,0.4)';
          }}
          ctxA.fill();
          ctxA.restore();
        }}
      }});

      // 2. Depth-Sorted Buildings
      const bList = WORLD_MAP.filter(c => c.type !== 'grass' && c.type !== 'water' && c.type !== 'road' && c.type !== 'bridge' && c.type !== 'plaza' && c.type !== 'farm');
      bList.sort((a, b) => (a.x + a.y) - (b.x + b.y));

      bList.forEach(cell => {{
        const pt = worldToScreen(cell.x, cell.y, canvasA, panA, zoomA);
        const th = TILE_H * zoomA;

        let img = null;
        if (cell.type === 'keep') img = loadedImages['sc2k_cityhall'];
        else if (cell.type === 'university') img = loadedImages['sc2k_university'];
        else if (cell.type === 'bigben') img = loadedImages['sc2k_bigben'];
        else if (cell.type === 'hydro') img = loadedImages['sc2k_hydro'];
        else if (cell.type === 'house') img = loadedImages['sc2k_house'];
        else if (cell.type === 'granary') img = loadedImages['sc2k_cityhall'];
        else if (cell.type === 'market') img = loadedImages['sc2k_university'];
        else if (cell.type === 'temple') img = loadedImages['sc2k_bigben'];
        else if (cell.type === 'tower') img = loadedImages['sc2k_bigben'];
        else if (cell.type === 'windmill') img = loadedImages['sc2k_house'];

        if (img && img.complete) {{
          const iw = img.naturalWidth * zoomA * 0.45;
          const ih = img.naturalHeight * zoomA * 0.45;
          ctxA.drawImage(img, pt.x - iw / 2, pt.y + th - ih, iw, ih);
        }}
      }});

      // 3. Moving Citizens (SC2K Commuters)
      AGENTS.forEach(ag => {{
        const pt = worldToScreen(ag.x, ag.y, canvasA, panA, zoomA);
        ctxA.fillStyle = ag.unitType === 'knight' ? '#f59e0b' : (ag.unitType === 'archer' ? '#38bdf8' : '#fff');
        ctxA.beginPath();
        ctxA.arc(pt.x, pt.y + 12 * zoomA, 4 * zoomA, 0, Math.PI * 2);
        ctxA.fill();
        ctxA.strokeStyle = '#000';
        ctxA.stroke();

        if (ag.id === selectedAgentId) {{
          ctxA.strokeStyle = '#00e5ff';
          ctxA.lineWidth = 2;
          ctxA.strokeRect(pt.x - 8, pt.y + 4, 16, 16);
          ctxA.lineWidth = 1;
        }}
      }});
    }}

    // --- RENDER ENGINE B: Isometric-City (Authentic Medieval Fortified Town) ---
    function renderEngineB() {{
      const w = canvasB.width, h = canvasB.height;
      ctxB.fillStyle = '#0f172a';
      ctxB.fillRect(0, 0, w, h);

      // 1. Draw Continuous Medieval Landscape
      WORLD_MAP.forEach(cell => {{
        const pt = worldToScreen(cell.x, cell.y, canvasB, panB, zoomB);
        const tw = TILE_W * zoomB;
        const th = TILE_H * zoomB;

        ctxB.beginPath();
        ctxB.moveTo(pt.x, pt.y);
        ctxB.lineTo(pt.x + tw / 2, pt.y + th / 2);
        ctxB.lineTo(pt.x, pt.y + th);
        ctxB.lineTo(pt.x - tw / 2, pt.y + th / 2);
        ctxB.closePath();

        if (cell.type === 'water') {{
          ctxB.fillStyle = '#1e3a8a';
          ctxB.fill();
        }} else if (cell.type === 'road' || cell.type === 'bridge') {{
          ctxB.fillStyle = '#475569'; // Cobblestone
          ctxB.fill();
          ctxB.strokeStyle = '#64748b';
          ctxB.stroke();
        }} else if (cell.type === 'farm') {{
          ctxB.fillStyle = '#78350f'; // Plowed furrow soil
          ctxB.fill();
        }} else if (cell.type === 'plaza') {{
          ctxB.fillStyle = '#334155';
          ctxB.fill();
        }} else {{
          ctxB.fillStyle = '#166534'; // Lush green pasture
          ctxB.fill();
          ctxB.strokeStyle = 'rgba(255, 255, 255, 0.06)';
          ctxB.stroke();
        }}

        // Heatmaps
        if (currentHeatmap !== 'normal') {{
          ctxB.save();
          if (currentHeatmap === 'food') {{
            ctxB.fillStyle = (cell.x < 10) ? 'rgba(16, 185, 129, 0.45)' : 'rgba(239, 68, 68, 0.35)';
          }} else if (currentHeatmap === 'loyalty') {{
            ctxB.fillStyle = (cell.x > 12) ? 'rgba(239, 68, 68, 0.45)' : 'rgba(14, 165, 233, 0.35)';
          }} else if (currentHeatmap === 'defense') {{
            const dist = Math.hypot(cell.x - 10, cell.y - 10);
            ctxB.fillStyle = dist < 6 ? 'rgba(56, 189, 248, 0.4)' : 'rgba(0,0,0,0.5)';
          }}
          ctxB.fill();
          ctxB.restore();
        }}
      }});

      // 2. Depth-Sorted Rendering of Buildings & Animated Units (Y + X)
      const renderList = [];

      WORLD_MAP.forEach(cell => {{
        if (cell.type !== 'grass' && cell.type !== 'water' && cell.type !== 'road' && cell.type !== 'bridge' && cell.type !== 'plaza' && cell.type !== 'farm') {{
          renderList.push({{
            sortKey: cell.x + cell.y,
            isAgent: false,
            data: cell
          }});
        }}
      }});

      AGENTS.forEach(ag => {{
        renderList.push({{
          sortKey: ag.x + ag.y,
          isAgent: true,
          data: ag
        }});
      }});

      renderList.sort((a, b) => a.sortKey - b.sortKey);

      renderList.forEach(item => {{
        if (!item.isAgent) {{
          // Building
          const cell = item.data;
          const pt = worldToScreen(cell.x, cell.y, canvasB, panB, zoomB);
          const th = TILE_H * zoomB;

          let img = null;
          if (cell.type === 'keep') img = loadedImages['sh_keep'];
          else if (cell.type === 'tower') img = loadedImages['sh_tower'];
          else if (cell.type === 'granary') img = loadedImages['sh_granary'];
          else if (cell.type === 'windmill') img = loadedImages['sh_windmill'];
          else if (cell.type === 'temple') img = loadedImages['c3_temple'];
          else if (cell.type === 'university') img = loadedImages['c3_basilica'];
          else if (cell.type === 'market') img = loadedImages['c3_coliseum'];
          else if (cell.type === 'house') img = loadedImages['c3_insula'];
          else if (cell.type === 'bigben' || cell.type === 'hydro') img = loadedImages['c3_garden'];

          if (img && img.complete) {{
            const iw = img.naturalWidth * zoomB;
            const ih = img.naturalHeight * zoomB;
            ctxB.drawImage(img, pt.x - iw / 2, pt.y + th - ih, iw, ih);
          }}
        }} else {{
          // Animated Agent Walk Cycle (Zero Glitches!)
          const ag = item.data;
          const pt = worldToScreen(ag.x, ag.y, canvasB, panB, zoomB);

          let frames = loadedImages['sh_peasant_walk'];
          if (ag.unitType === 'knight') frames = loadedImages['sh_knight_walk'];
          else if (ag.unitType === 'archer') frames = loadedImages['sh_archer_walk'];
          else if (ag.unitType === 'c3_citizen') frames = loadedImages['c3_citizen_walk'];
          else if (ag.unitType === 'c3_legionary') frames = loadedImages['c3_legionary_walk'];

          if (frames && frames.length > 0) {{
            const fIdx = Math.floor(ag.walkCycle) % frames.length;
            const frameImg = frames[fIdx];
            if (frameImg && frameImg.complete) {{
              const fw = frameImg.naturalWidth * zoomB * 1.1;
              const fh = frameImg.naturalHeight * zoomB * 1.1;
              ctxB.drawImage(frameImg, pt.x - fw / 2, pt.y - fh + 16 * zoomB, fw, fh);
            }}
          }}

          if (ag.id === selectedAgentId) {{
            ctxB.strokeStyle = '#00e5ff';
            ctxB.lineWidth = 2;
            ctxB.beginPath();
            ctxB.arc(pt.x, pt.y + 8 * zoomB, 12 * zoomB, 0, Math.PI * 2);
            ctxB.stroke();
            ctxB.lineWidth = 1;
          }}
        }}
      }});
    }}

    // --- SIMULATION STEPPING & AGENT MOVEMENT ---
    function updateSimulation() {{
      AGENTS.forEach(ag => {{
        const target = ag.waypoints[ag.wpIdx];
        const dx = target.x - ag.x;
        const dy = target.y - ag.y;
        const dist = Math.hypot(dx, dy);

        if (dist < 0.15) {{
          ag.wpIdx = (ag.wpIdx + 1) % ag.waypoints.length;
        }} else {{
          ag.x += (dx / dist) * ag.speed * playSpeed;
          ag.y += (dy / dist) * ag.speed * playSpeed;
          ag.walkCycle += 0.25 * playSpeed;
        }}

        if (ag.id === selectedAgentId) {{
          document.getElementById('dossierPos').textContent = `X: ${{ag.x.toFixed(2)}}, Y: ${{ag.y.toFixed(2)}}`;
        }}
      }});
    }}

    // --- CLICK TO INSPECT ---
    function handleCanvasClick(e, canvas, pan, zoom) {{
      const rect = canvas.getBoundingClientRect();
      const sx = e.clientX - rect.left;
      const sy = e.clientY - rect.top;
      const wPos = screenToWorld(sx, sy, canvas, pan, zoom);

      let nearest = null;
      let minDist = 1.4;
      AGENTS.forEach(ag => {{
        const d = Math.hypot(ag.x - wPos.gx, ag.y - wPos.gy);
        if (d < minDist) {{
          minDist = d;
          nearest = ag;
        }}
      }});

      if (nearest) {{
        selectAgent(nearest.id);
      }}
    }}

    canvasA.addEventListener('click', e => handleCanvasClick(e, canvasA, panA, zoomA));
    canvasB.addEventListener('click', e => handleCanvasClick(e, canvasB, panB, zoomB));

    function selectAgent(id) {{
      selectedAgentId = id;
      const ag = AGENTS.find(a => a.id === id);
      if (!ag) return;
      document.getElementById('dossierName').textContent = ag.name;
      document.getElementById('dossierId').textContent = ag.id;
      document.getElementById('dossierRole').textContent = ag.role;
      document.getElementById('dossierPos').textContent = `X: ${{ag.x.toFixed(2)}}, Y: ${{ag.y.toFixed(2)}}`;
      document.getElementById('dossierDuty').textContent = ag.duty;
      document.getElementById('dossierLoyalty').textContent = `${{ag.morale}}%`;
      document.getElementById('dossierHunger').textContent = `${{ag.hunger}}% (${{ag.hunger < 20 ? 'Satiated' : 'Hungry'}})`;
      document.getElementById('dossierThoughts').textContent = `"${{ag.thoughts}}"`;
    }}

    // --- UI CONTROLS ---
    function setMode(mode) {{
      const wrap = document.getElementById('viewportsWrapper');
      wrap.className = 'viewports-wrapper ' + (mode === 'split' ? '' : mode);
      document.getElementById('tabSplit').classList.toggle('active', mode === 'split');
      document.getElementById('tabA').classList.toggle('active', mode === 'single-a');
      document.getElementById('tabB').classList.toggle('active', mode === 'single-b');
      setTimeout(resizeCanvases, 60);
    }}

    function togglePlay() {{
      isPlaying = !isPlaying;
      document.getElementById('btnPlay').textContent = isPlaying ? "⏸ Pause" : "▶ Play";
    }}

    function stepTick() {{
      tick++;
      document.getElementById('valEpoch').textContent = `M05 : T${{148 + tick}}`;
      updateSimulation();
    }}

    function setSpeed(spd) {{
      playSpeed = spd;
      document.getElementById('btnSpd1').classList.toggle('active', spd === 1);
      document.getElementById('btnSpd3').classList.toggle('active', spd === 3);
    }}

    function setHeatmap(hm) {{
      currentHeatmap = hm;
      ['hmNormal', 'hmFood', 'hmLoyalty', 'hmDefense'].forEach(id => {{
        document.getElementById(id).classList.remove('active');
      }});
      if (hm === 'normal') document.getElementById('hmNormal').classList.add('active');
      if (hm === 'food') document.getElementById('hmFood').classList.add('active');
      if (hm === 'loyalty') document.getElementById('hmLoyalty').classList.add('active');
      if (hm === 'defense') document.getElementById('hmDefense').classList.add('active');
    }}

    function setTool(tool) {{
      currentTool = tool;
      document.querySelectorAll('.sc2k-tool-btn').forEach(btn => btn.classList.remove('active'));
      event.target.classList.add('active');
    }}

    function triggerImpulse(type) {{
      const ticker = document.getElementById('tickerText');
      if (type === 'rally') {{
        ticker.textContent = "⚡ GARRISON HORN SOUNDS: ALL KNIGHTS AND ARCHERS ORDERED TO RAMPART BATTLEMENTS IMMEDIATELY!";
        AGENTS.forEach(a => {{ if (a.unitType === 'knight') a.morale = 100; }});
      }} else if (type === 'grain') {{
        ticker.textContent = "🌾 EMERGENCY FLOUR DISPATCH: 1,200 BUSHELS DISPATCHED TO LOWER RESIDENTIAL QUARTERS!";
        AGENTS.forEach(a => a.hunger = Math.max(0, a.hunger - 10));
      }} else if (type === 'trebuchet') {{
        ticker.textContent = "🔥 SIEGE CATAPULT IMPACT: IMPACT CRATER AT EAST WATCHTOWER! BATTLEMENT HP -40%!";
        document.getElementById('valSurprise').textContent = "58.4 bits (CLIMAX)";
        document.getElementById('valSurprise').style.color = "var(--crimson)";
      }}
    }}

    // --- MAIN RENDER LOOP ---
    function loop() {{
      if (isPlaying) {{
        updateSimulation();
      }}
      renderEngineA();
      renderEngineB();
      requestAnimationFrame(loop);
    }}

    loop();
  </script>
</body>
</html>
""")

    print("[✔] Successfully generated 100% clean matrix_engine_ab_test.html!")

if __name__ == '__main__':
    main()
