#!/usr/bin/env python3
"""Builds REHOBOAM // LIVE WORLD MATRIX STREAM with Animated Moving People

Embeds real 8-frame walk cycles from Stronghold Crusader HD and Caesar III.
Simulates a live Rehoboam agent telemetry stream with autonomous pathfinding,
patrols, courier supply runs, clickable agent inspector, and impulse events.
"""

import base64
import json
from pathlib import Path

def encode_img(path_str):
    p = Path(path_str)
    if not p.exists():
        return ""
    with open(p, 'rb') as f:
        return 'data:image/png;base64,' + base64.b64encode(f.read()).decode('ascii')

def main():
    print("[*] Encoding premier buildings and 40 walk-cycle animation frames...")
    
    sprites = {
        # Buildings - Stronghold
        "sh_keep": encode_img("assets/sprites/stronghold/tile_castle/tile_castle_0003.png"),
        "sh_tower": encode_img("assets/sprites/stronghold/tile_castle/tile_castle_0002.png"),
        "sh_wall_1": encode_img("assets/sprites/stronghold/tile_walls/tile_walls_0001.png"),
        "sh_wall_2": encode_img("assets/sprites/stronghold/tile_walls/tile_walls_0002.png"),
        "sh_granary": encode_img("assets/sprites/stronghold/tile_buildings1/tile_buildings1_0000.png"),
        
        # Buildings - Caesar III
        "c3_coliseum": encode_img("assets/sprites/caesar3/caesar3/c3_img_3074_type30.png"),
        "c3_temple": encode_img("assets/sprites/caesar3/caesar3/c3_img_3214_type30.png"),
        "c3_basilica": encode_img("assets/sprites/caesar3/caesar3/c3_img_2826_type30.png"),
        "c3_tower": encode_img("assets/sprites/caesar3/caesar3/c3_img_7780_type30.png"),
        "c3_gate": encode_img("assets/sprites/caesar3/caesar3/c3_img_7789_type30.png"),
        "c3_aqueduct": encode_img("assets/sprites/caesar3/caesar3/c3_img_7784_type30.png"),
        "c3_insula": encode_img("assets/sprites/caesar3/caesar3/c3_img_0821_type30.png"),
        
        # Buildings - SimCity 2000 SE
        "sc2k_bigben": encode_img("assets/sprites/sc2k/scurk/bigben.png"),
        "sc2k_cityhall": encode_img("assets/sprites/sc2k/scurk/ctyhall.png"),
        "sc2k_university": encode_img("assets/sprites/sc2k/scurk/unvir.png"),
        "sc2k_hydro": encode_img("assets/sprites/sc2k/scurk/hydro1.png"),
        "sc2k_house": encode_img("assets/sprites/sc2k/scurk/smhouse.png"),
        "sc2k_road": encode_img("assets/sprites/sc2k/large/sc2k_large_1001.png"),
        "sc2k_civic": encode_img("assets/sprites/sc2k/large/sc2k_large_1010.png"),
    }

    # Animated Walk Cycles (8 frames each)
    # 1. Stronghold Peasant
    sprites["sh_peasant_walk"] = [
        encode_img(f"assets/sprites/stronghold/body_peasant/body_peasant_{i:04d}.png") for i in range(8)
    ]
    # 2. Stronghold Archer
    sprites["sh_archer_walk"] = [
        encode_img(f"assets/sprites/stronghold/body_archer/body_archer_{i:04d}.png") for i in range(8)
    ]
    # 3. Stronghold Mounted Knight
    sprites["sh_knight_walk"] = [
        encode_img(f"assets/sprites/stronghold/body_knight/body_knight_{i:04d}.png") for i in range(8)
    ]
    # 4. Caesar III Roman Citizen
    sprites["c3_citizen_walk"] = [
        encode_img(f"assets/sprites/caesar3/caesar3/c3_img_{7500+i:04d}_type0.png") for i in range(8)
    ]
    # 5. Caesar III Roman Legionary
    sprites["c3_legionary_walk"] = [
        encode_img(f"assets/sprites/caesar3/caesar3/c3_img_{4840+i:04d}_type0.png") for i in range(8)
    ]

    sprites_json = json.dumps(sprites)

    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>REHOBOAM // Live World Matrix Stream (Autonomous Agents & Diorama)</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;600;700;800&family=Outfit:wght@300;400;600;700;800&display=swap" rel="stylesheet">
  <style>
    :root {{
      --bg: #070a12;
      --card-bg: rgba(14, 20, 34, 0.92);
      --border: #1e293b;
      --border-glow: #38bdf8;
      --accent-cyan: #00e5ff;
      --accent-emerald: #10b981;
      --accent-amber: #f59e0b;
      --accent-crimson: #ef4444;
      --accent-purple: #a855f7;
      --text: #f8fafc;
      --text-muted: #94a3b8;
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

    header {{
      padding: 14px 28px;
      background: rgba(11, 16, 27, 0.96);
      border-bottom: 1px solid var(--border);
      backdrop-filter: blur(12px);
      display: flex;
      justify-content: space-between;
      align-items: center;
      position: sticky;
      top: 0;
      z-index: 100;
    }}

    .brand {{
      display: flex;
      align-items: center;
      gap: 14px;
    }}

    .logo-badge {{
      width: 40px; height: 40px;
      border-radius: 8px;
      background: linear-gradient(135deg, #00e5ff, #3b82f6);
      display: flex; align-items: center; justify-content: center;
      font-weight: 800; font-size: 20px; color: #070a12;
      box-shadow: 0 0 18px rgba(0, 229, 255, 0.4);
    }}

    .title-group h1 {{
      font-size: 17px; font-weight: 700;
      display: flex; align-items: center; gap: 10px;
    }}

    .badge-live {{
      font-size: 11px;
      background: rgba(16, 185, 129, 0.2);
      color: var(--accent-emerald);
      border: 1px solid rgba(16, 185, 129, 0.4);
      padding: 2px 8px; border-radius: 999px;
      font-family: 'JetBrains Mono', monospace;
    }}

    .hud-counters {{
      display: flex; gap: 14px;
    }}

    .hud-box {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      padding: 6px 14px; border-radius: 6px;
      text-align: right;
    }}
    .hud-box .val {{
      font-family: 'JetBrains Mono', monospace;
      font-size: 15px; font-weight: 700; color: var(--accent-cyan);
    }}
    .hud-box .lbl {{
      font-size: 9px; color: var(--text-muted); text-transform: uppercase;
    }}

    main {{
      display: grid;
      grid-template-columns: 1fr 440px;
      gap: 18px;
      padding: 18px 28px;
      flex: 1;
    }}

    /* Viewport Panel */
    .viewport-panel {{
      display: flex;
      flex-direction: column;
      gap: 12px;
    }}

    .canvas-container {{
      position: relative;
      background: radial-gradient(circle at 50% 50%, #111a2e 0%, #060913 100%);
      border: 1px solid var(--border);
      border-radius: 12px;
      overflow: hidden;
      height: 600px;
      display: flex;
      align-items: center;
      justify-content: center;
      box-shadow: inset 0 0 50px rgba(0, 0, 0, 0.85);
    }}

    canvas {{
      display: block;
      cursor: crosshair;
      image-rendering: pixelated;
    }}

    /* Climax Modal / Banner */
    .climax-banner {{
      position: absolute;
      top: 16px; left: 50%;
      transform: translateX(-50%);
      background: linear-gradient(135deg, rgba(239, 68, 68, 0.95), rgba(185, 28, 28, 0.95));
      border: 1px solid #fca5a5;
      padding: 8px 20px;
      border-radius: 999px;
      font-weight: 700;
      font-size: 12px;
      display: none;
      align-items: center;
      gap: 12px;
      box-shadow: 0 0 25px rgba(239, 68, 68, 0.6);
      z-index: 50;
      animation: pulseAlert 1.5s infinite;
    }}
    @keyframes pulseAlert {{
      0%, 100% {{ transform: translateX(-50%) scale(1); }}
      50% {{ transform: translateX(-50%) scale(1.02); }}
    }}

    .btn-climax {{
      background: #000;
      color: #fff;
      border: 1px solid rgba(255,255,255,0.4);
      padding: 3px 10px;
      border-radius: 999px;
      font-size: 10px;
      font-weight: 800;
      cursor: pointer;
      font-family: 'JetBrains Mono', monospace;
    }}

    /* Controls Bar */
    .controls-bar {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      padding: 10px 16px;
      border-radius: 8px;
      display: flex;
      justify-content: space-between;
      align-items: center;
      flex-wrap: wrap;
      gap: 10px;
    }}

    .skin-tabs {{
      display: flex; gap: 6px;
    }}
    .skin-btn {{
      background: #0c1220;
      border: 1px solid var(--border);
      color: var(--text-muted);
      padding: 5px 12px;
      border-radius: 5px;
      font-size: 12px;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
    }}
    .skin-btn:hover {{ border-color: var(--accent-cyan); color: var(--text); }}
    .skin-btn.active {{
      background: rgba(0, 229, 255, 0.15);
      border-color: var(--accent-cyan);
      color: var(--accent-cyan);
    }}

    .playback-controls {{
      display: flex; align-items: center; gap: 6px;
    }}
    .btn-ctrl {{
      background: #0c1220;
      border: 1px solid var(--border);
      color: var(--text);
      padding: 5px 10px;
      border-radius: 5px;
      font-size: 11px;
      font-weight: 600;
      cursor: pointer;
      font-family: 'JetBrains Mono', monospace;
    }}
    .btn-ctrl:hover {{ border-color: var(--accent-cyan); }}
    .btn-ctrl.primary {{
      background: rgba(0, 229, 255, 0.2);
      border-color: var(--accent-cyan);
      color: var(--accent-cyan);
    }}

    /* Impulse Bar */
    .impulse-bar {{
      display: flex; gap: 8px; align-items: center;
      background: #090e1a;
      padding: 8px 12px;
      border-radius: 6px;
      border: 1px dashed var(--border);
    }}
    .impulse-btn {{
      background: #131c2e;
      border: 1px solid #2d3b55;
      color: var(--text);
      padding: 4px 10px;
      border-radius: 4px;
      font-size: 11px;
      cursor: pointer;
      font-family: 'JetBrains Mono', monospace;
      transition: all 0.2s;
    }}
    .impulse-btn:hover {{
      background: var(--accent-amber);
      color: #000;
      font-weight: 700;
    }}

    /* Sidebar / Chronicle */
    .sidebar {{
      display: flex;
      flex-direction: column;
      gap: 14px;
    }}

    .panel-card {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 14px;
      display: flex;
      flex-direction: column;
      gap: 10px;
    }}

    .panel-title {{
      font-size: 12px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      color: var(--accent-cyan);
      display: flex;
      justify-content: space-between;
      align-items: center;
    }}

    /* Agent Dossier */
    .agent-dossier {{
      background: #080d1a;
      border: 1px solid #1f2a40;
      border-radius: 6px;
      padding: 12px;
      display: flex;
      flex-direction: column;
      gap: 8px;
      font-family: 'JetBrains Mono', monospace;
      font-size: 11px;
    }}
    .agent-header {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      border-bottom: 1px solid #1f2a40;
      padding-bottom: 6px;
    }}
    .agent-name {{
      font-weight: 700;
      color: var(--accent-cyan);
      font-size: 12px;
    }}
    .agent-role {{
      font-size: 10px;
      background: rgba(0, 229, 255, 0.15);
      color: var(--accent-cyan);
      padding: 1px 6px;
      border-radius: 4px;
    }}
    .agent-grid {{
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 6px;
    }}
    .agent-prop {{
      display: flex;
      flex-direction: column;
      gap: 1px;
    }}
    .agent-prop .lbl {{ font-size: 9px; color: var(--text-muted); text-transform: uppercase; }}
    .agent-prop .val {{ font-weight: 600; color: #fff; }}
    .agent-thoughts {{
      background: #040710;
      padding: 8px;
      border-radius: 4px;
      border-left: 2px solid var(--accent-amber);
      font-size: 10px;
      line-height: 1.4;
      color: #fde68a;
    }}

    .chronicle-feed {{
      background: #080c16;
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 10px;
      height: 240px;
      overflow-y: auto;
      display: flex;
      flex-direction: column;
      gap: 8px;
      font-family: 'JetBrains Mono', monospace;
      font-size: 10px;
    }}

    .feed-item {{
      border-left: 2px solid var(--border);
      padding-left: 8px;
      line-height: 1.4;
    }}
    .feed-item.alert {{
      border-color: var(--accent-crimson);
      color: #fca5a5;
    }}
    .feed-item.warning {{
      border-color: var(--accent-amber);
      color: #fde68a;
    }}
    .feed-item.active {{
      background: rgba(0, 229, 255, 0.08);
      padding: 4px 8px;
      border-radius: 4px;
    }}
    .feed-tick {{
      font-weight: 800;
      color: var(--accent-cyan);
      margin-bottom: 2px;
    }}
  </style>
</head>
<body>

  <header>
    <div class="brand">
      <div class="logo-badge">R</div>
      <div class="title-group">
        <h1>REHOBOAM // LIVE WORLD MATRIX STREAM <span class="badge-live">Hardware Accelerated</span></h1>
      </div>
    </div>

    <div class="hud-counters">
      <div class="hud-box">
        <div class="val" id="hudTick">MONTH 01 / 06</div>
        <div class="lbl">Simulation Epoch</div>
      </div>
      <div class="hud-box">
        <div class="val" id="hudPopulation">648 Citizens</div>
        <div class="lbl">Living Souls</div>
      </div>
      <div class="hud-box">
        <div class="val" id="hudGrain">12,400</div>
        <div class="lbl">Granary Food</div>
      </div>
      <div class="hud-box">
        <div class="val" id="hudSurprise">12.4 bits</div>
        <div class="lbl">Surprise Index (U)</div>
      </div>
    </div>
  </header>

  <main>
    <!-- Viewport Panel -->
    <div class="viewport-panel">
      <div class="canvas-container" id="canvasBox">
        <div class="climax-banner" id="climaxBanner">
          <span>⚡ HIGH SURPRISE EVENT (U = 62.4 bits): LORD FALLEN IN BREACH</span>
          <button class="btn-climax" onclick="openClimaxVignette()">ENTER 3D CAMERA ➔</button>
        </div>
        <canvas id="worldCanvas" width="960" height="600"></canvas>
      </div>

      <!-- Controls & Impulses -->
      <div class="controls-bar">
        <div class="skin-tabs">
          <button class="skin-btn active" id="btnSkinSH" onclick="switchSkin('stronghold')">🏰 Stronghold Crusader HD</button>
          <button class="skin-btn" id="btnSkinC3" onclick="switchSkin('caesar3')">🏛️ Caesar III Roman</button>
          <button class="skin-btn" id="btnSkinSC2K" onclick="switchSkin('sc2k')">🏙️ SimCity 2000 Retro</button>
        </div>

        <div class="playback-controls">
          <button class="btn-ctrl primary" id="btnPlay" onclick="togglePlay()">▶ PLAY STREAM</button>
          <button class="btn-ctrl" onclick="stepTick()">⏭ STEP</button>
          <button class="btn-ctrl" onclick="resetSim()">↺ RESET</button>
        </div>
      </div>

      <!-- Interactive Rehoboam Impulse Bar -->
      <div class="impulse-bar">
        <span style="font-size: 11px; color: var(--accent-cyan); font-weight: 700;">INJECT REHOBOAM IMPULSE:</span>
        <button class="impulse-btn" onclick="impulseRally()">⚡ Rally Garrison</button>
        <button class="impulse-btn" onclick="impulseDistributeGrain()">🌾 Haul Grain Rations</button>
        <button class="impulse-btn" onclick="impulseCatapultStrike()">🔥 Trebuchet Strike</button>
      </div>
    </div>

    <!-- Sidebar Dossier & Chronicle -->
    <div class="sidebar">
      <!-- Live Agent Inspector Dossier -->
      <div class="panel-card">
        <div class="panel-title">
          <span>Agent Telemetry Dossier</span>
          <span style="font-size: 10px; color: var(--accent-emerald);">CLICK PERSON ON MAP</span>
        </div>
        <div class="agent-dossier" id="agentDossier">
          <div class="agent-header">
            <div class="agent-name" id="dosName">Sir Guy de Montfort</div>
            <div class="agent-role" id="dosRole">KNIGHT_COMMANDER</div>
          </div>
          <div class="agent-grid">
            <div class="agent-prop">
              <span class="lbl">World Position</span>
              <span class="val" id="dosPos">X: 6.50, Y: 7.20</span>
            </div>
            <div class="agent-prop">
              <span class="lbl">Assigned Duty</span>
              <span class="val" id="dosDuty">COURTYARD_PATROL</span>
            </div>
            <div class="agent-prop">
              <span class="lbl">Garrison Loyalty</span>
              <span class="val" id="dosLoyalty" style="color: var(--accent-emerald);">98%</span>
            </div>
            <div class="agent-prop">
              <span class="lbl">Caloric Hunger</span>
              <span class="val" id="dosHunger">6%</span>
            </div>
          </div>
          <div class="agent-thoughts" id="dosThoughts">
            "The walls hold firm. The granaries are stocked. Let the heathens taste our cold iron if they dare approach."
          </div>
        </div>
      </div>

      <!-- Live Chronicle -->
      <div class="panel-card" style="flex: 1;">
        <div class="panel-title">
          <span>Downstream Chronicle Stream</span>
          <span style="font-size: 10px; color: var(--accent-emerald);">● PARQUET SYNC</span>
        </div>
        <div class="chronicle-feed" id="chronicleFeed">
          <!-- Dynamically populated -->
        </div>
      </div>
    </div>
  </main>

  <script>
    const SPRITES = {sprites_json};
    const loadedImages = {{}};

    // Preload Base64 sprites and animation arrays
    for (let k in SPRITES) {{
      if (Array.isArray(SPRITES[k])) {{
        loadedImages[k] = SPRITES[k].map(b64 => {{
          const img = new Image();
          img.src = b64;
          return img;
        }});
      }} else if (SPRITES[k]) {{
        const img = new Image();
        img.src = SPRITES[k];
        loadedImages[k] = img;
      }}
    }}

    // World Grid Definition
    const GRID_W = 14;
    const GRID_H = 14;
    const TILE_W = 64;
    const TILE_H = 32;

    let currentSkin = 'stronghold';
    let currentTick = 1;
    let isPlaying = true;
    let playTimer = null;
    let zoom = 1.0;
    let panX = 0, panY = 0;
    let selectedAgentId = 'knight_1';

    let particles = [];
    let projectiles = [];

    // Static Settlement Buildings
    const WORLD_LAYOUT = [
      {{ x: 6, y: 6, type: 'keep', hp: 1.0 }},
      {{ x: 7, y: 6, type: 'temple', hp: 1.0 }},
      {{ x: 6, y: 7, type: 'granary', hp: 1.0 }},
      {{ x: 7, y: 7, type: 'coliseum', hp: 1.0 }},
      
      // Walls & Towers
      {{ x: 4, y: 4, type: 'tower', hp: 1.0 }},
      {{ x: 5, y: 4, type: 'wall_h', hp: 1.0 }},
      {{ x: 6, y: 4, type: 'wall_h', hp: 1.0 }},
      {{ x: 7, y: 4, type: 'wall_h', hp: 1.0 }},
      {{ x: 8, y: 4, type: 'wall_h', hp: 1.0 }},
      {{ x: 9, y: 4, type: 'tower', hp: 1.0 }},
      
      {{ x: 4, y: 9, type: 'tower', hp: 1.0 }},
      {{ x: 5, y: 9, type: 'wall_h', hp: 1.0 }},
      {{ x: 6, y: 9, type: 'gate', hp: 1.0 }},
      {{ x: 7, y: 9, type: 'wall_h', hp: 1.0 }},
      {{ x: 8, y: 9, type: 'wall_h', hp: 1.0 }},
      {{ x: 9, y: 9, type: 'tower', hp: 1.0 }},

      {{ x: 4, y: 5, type: 'wall_v', hp: 1.0 }},
      {{ x: 4, y: 6, type: 'wall_v', hp: 1.0 }},
      {{ x: 4, y: 7, type: 'wall_v', hp: 1.0 }},
      {{ x: 4, y: 8, type: 'wall_v', hp: 1.0 }},

      {{ x: 9, y: 5, type: 'wall_v', hp: 1.0 }},
      {{ x: 9, y: 6, type: 'wall_v', hp: 1.0 }},
      {{ x: 9, y: 7, type: 'wall_v', hp: 1.0 }},
      {{ x: 9, y: 8, type: 'wall_v', hp: 1.0 }},

      // Houses
      {{ x: 5, y: 5, type: 'house', hp: 1.0 }},
      {{ x: 8, y: 5, type: 'house', hp: 1.0 }},
      {{ x: 5, y: 8, type: 'house', hp: 1.0 }},
      {{ x: 8, y: 8, type: 'house', hp: 1.0 }},
    ];

    // AUTONOMOUS MOVING AGENTS (People moving in the simulation stream)
    let AGENTS = [
      {{
        id: 'knight_1',
        name: 'Sir Guy de Montfort',
        role: 'KNIGHT_COMMANDER',
        x: 6.5, y: 7.2,
        targetX: 6.5, targetY: 8.8,
        speed: 0.015,
        walkCycle: 0,
        morale: 98,
        hunger: 6,
        duty: 'COURTYARD_PATROL',
        thoughts: 'The walls hold firm. The granaries are stocked. Let the heathens taste our cold iron if they dare approach.',
        unitType: 'knight',
        waypoints: [{{ x: 6.5, y: 7.0 }}, {{ x: 6.5, y: 8.8 }}, {{ x: 7.5, y: 8.8 }}, {{ x: 7.5, y: 7.0 }}],
        wpIndex: 0
      }},
      {{
        id: 'sentry_1',
        name: 'Baelor the Sentry',
        role: 'NORTH_WALL_SENTRY',
        x: 5.0, y: 4.0,
        targetX: 8.5, targetY: 4.0,
        speed: 0.018,
        walkCycle: 0,
        morale: 89,
        hunger: 14,
        duty: 'NORTH_BATTLEMENT_PATROL',
        thoughts: 'North treeline is quiet, but smoke rising in the distance. Checking bowstrings.',
        unitType: 'archer',
        waypoints: [{{ x: 4.8, y: 4.0 }}, {{ x: 8.2, y: 4.0 }}],
        wpIndex: 1
      }},
      {{
        id: 'sentry_2',
        name: 'Merrick Longshot',
        role: 'WEST_WALL_SENTRY',
        x: 4.0, y: 5.2,
        targetX: 4.0, targetY: 8.0,
        speed: 0.016,
        walkCycle: 0,
        morale: 84,
        hunger: 19,
        duty: 'WEST_BATTLEMENT_PATROL',
        thoughts: 'Siege engines spotted maneuvering into firing position across the river.',
        unitType: 'archer',
        waypoints: [{{ x: 4.0, y: 5.0 }}, {{ x: 4.0, y: 8.2 }}],
        wpIndex: 1
      }},
      {{
        id: 'courier_1',
        name: 'Eldred Millerboy',
        role: 'GRANARY_COURIER',
        x: 6.0, y: 7.0,
        targetX: 5.2, targetY: 8.2,
        speed: 0.022,
        walkCycle: 0,
        morale: 76,
        hunger: 38,
        duty: 'HAULING_GRAIN_RATIONS',
        thoughts: 'Three sacks of flour for the South tenements. The queue grows longer every morning.',
        unitType: 'peasant',
        waypoints: [{{ x: 6.0, y: 7.0 }}, {{ x: 5.2, y: 8.2 }}, {{ x: 6.0, y: 7.0 }}, {{ x: 8.2, y: 5.2 }}],
        wpIndex: 1
      }},
      {{
        id: 'citizen_1',
        name: 'Widow Alys',
        role: 'SETTLEMENT_CITIZEN',
        x: 7.8, y: 6.2,
        targetX: 5.2, targetY: 5.2,
        speed: 0.012,
        walkCycle: 0,
        morale: 62,
        hunger: 44,
        duty: 'COLLECTING_WELL_WATER',
        thoughts: 'Lord Montfort says the relief army will arrive by the new moon. Pray heaven he is right.',
        unitType: 'peasant',
        waypoints: [{{ x: 7.8, y: 6.2 }}, {{ x: 5.2, y: 5.2 }}],
        wpIndex: 1
      }}
    ];

    // Simulation Epochs Data
    const TIMELINE_DATA = [
      {{
        tick: 1,
        title: "INVESTMENT & BLOCKADE",
        grain: 12400,
        surprise: 12.4,
        population: 648,
        desc: "Hostile vanguard establishes circumvallation. Trade conduits severed. Sentries on wall patrol.",
        hits: []
      }},
      {{
        tick: 2,
        title: "TREBUCHET BOMBARDMENT",
        grain: 9800,
        surprise: 24.1,
        population: 639,
        desc: "Siege engines commence barrage on West curtain wall. Outer hovels catch fire.",
        hits: [{{ x: 4, y: 6 }}, {{ x: 5, y: 5 }}]
      }},
      {{
        tick: 3,
        title: "FIRST GATE ASSAULT",
        grain: 6900,
        surprise: 38.2,
        population: 612,
        desc: "Battering ram engages the Southern Gatehouse. Defenders repel with boiling oil.",
        hits: [{{ x: 6, y: 9 }}]
      }},
      {{
        tick: 4,
        title: "GRAIN ROT & DESPAIR",
        grain: 3200,
        surprise: 44.7,
        population: 574,
        desc: "Granary moisture causes mold outbreak. Rations halved. Sentry mutiny suppressed.",
        hits: [{{ x: 6, y: 7 }}, {{ x: 9, y: 7 }}]
      }},
      {{
        tick: 5,
        title: "CLIMAX: BREACH & REGICIDE",
        grain: 1100,
        surprise: 62.4,
        population: 489,
        desc: "East wall collapses under sapping. Lord killed defending inner sanctuary. High unexpectedness event!",
        hits: [{{ x: 6, y: 6 }}, {{ x: 9, y: 5 }}, {{ x: 4, y: 7 }}]
      }},
      {{
        tick: 6,
        title: "FALLEN STRONGHOLD",
        grain: 200,
        surprise: 18.0,
        population: 410,
        desc: "Sanctuary pillaged. Garrison shattered. Remnants surrender under terms.",
        hits: [{{ x: 7, y: 6 }}, {{ x: 8, y: 8 }}]
      }}
    ];

    const canvas = document.getElementById('worldCanvas');
    const ctx = canvas.getContext('2d');

    function worldToScreen(gx, gy) {{
      const originX = canvas.width / 2 + panX;
      const originY = 120 + panY;
      const sx = originX + (gx - gy) * (TILE_W * zoom / 2);
      const sy = originY + (gx + gy) * (TILE_H * zoom / 2);
      return {{ x: sx, y: sy }};
    }}

    function screenToWorld(sx, sy) {{
      const originX = canvas.width / 2 + panX;
      const originY = 120 + panY;
      const dx = (sx - originX) / (TILE_W * zoom / 2);
      const dy = (sy - originY) / (TILE_H * zoom / 2);
      const gx = (dx + dy) / 2;
      const gy = (dy - dx) / 2;
      return {{ x: gx, y: gy }};
    }}

    function getSpriteForElement(elem) {{
      const s = currentSkin;
      if (s === 'stronghold') {{
        if (elem.type === 'keep') return loadedImages['sh_keep'];
        if (elem.type === 'temple') return loadedImages['sh_keep'];
        if (elem.type === 'granary') return loadedImages['sh_granary'];
        if (elem.type === 'coliseum') return loadedImages['sh_granary'];
        if (elem.type === 'tower') return loadedImages['sh_tower'];
        if (elem.type === 'wall_h') return loadedImages['sh_wall_1'];
        if (elem.type === 'wall_v') return loadedImages['sh_wall_2'];
        if (elem.type === 'gate') return loadedImages['sh_wall_1'];
        if (elem.type === 'house') return loadedImages['sh_granary'];
      }} else if (s === 'caesar3') {{
        if (elem.type === 'keep') return loadedImages['c3_basilica'];
        if (elem.type === 'temple') return loadedImages['c3_temple'];
        if (elem.type === 'granary') return loadedImages['c3_gate'];
        if (elem.type === 'coliseum') return loadedImages['c3_coliseum'];
        if (elem.type === 'tower') return loadedImages['c3_tower'];
        if (elem.type === 'wall_h' || elem.type === 'wall_v') return loadedImages['c3_aqueduct'];
        if (elem.type === 'gate') return loadedImages['c3_gate'];
        if (elem.type === 'house') return loadedImages['c3_insula'];
      }} else if (s === 'sc2k') {{
        if (elem.type === 'keep') return loadedImages['sc2k_cityhall'];
        if (elem.type === 'temple') return loadedImages['sc2k_bigben'];
        if (elem.type === 'granary') return loadedImages['sc2k_hydro'];
        if (elem.type === 'coliseum') return loadedImages['sc2k_university'];
        if (elem.type === 'tower') return loadedImages['sc2k_bigben'];
        if (elem.type === 'wall_h' || elem.type === 'wall_v' || elem.type === 'gate') return loadedImages['sc2k_road'];
        if (elem.type === 'house') return loadedImages['sc2k_house'];
      }}
      return null;
    }}

    function getAgentFrame(agent) {{
      const frameIdx = Math.floor(agent.walkCycle) % 8;
      const s = currentSkin;

      if (s === 'stronghold') {{
        if (agent.unitType === 'archer') return loadedImages['sh_archer_walk'][frameIdx];
        if (agent.unitType === 'knight') return loadedImages['sh_knight_walk'][frameIdx];
        return loadedImages['sh_peasant_walk'][frameIdx];
      }} else if (s === 'caesar3') {{
        if (agent.unitType === 'archer' || agent.unitType === 'knight') {{
          return loadedImages['c3_legionary_walk'][frameIdx];
        }}
        return loadedImages['c3_citizen_walk'][frameIdx];
      }} else if (s === 'sc2k') {{
        return loadedImages['sc2k_civic'];
      }}
      return null;
    }}

    function updateSimulation(dt) {{
      // Update autonomous moving agents
      AGENTS.forEach(ag => {{
        ag.walkCycle += ag.speed * 120;
        const target = ag.waypoints[ag.wpIndex];
        const dx = target.x - ag.x;
        const dy = target.y - ag.y;
        const dist = Math.sqrt(dx * dx + dy * dy);

        if (dist < 0.1) {{
          ag.wpIndex = (ag.wpIndex + 1) % ag.waypoints.length;
        }} else {{
          ag.x += (dx / dist) * ag.speed;
          ag.y += (dy / dist) * ag.speed;
        }}
      }});

      // Update selected agent dossier
      const sel = AGENTS.find(a => a.id === selectedAgentId);
      if (sel) {{
        document.getElementById('dosName').textContent = sel.name;
        document.getElementById('dosRole').textContent = sel.role;
        document.getElementById('dosPos').textContent = `X: ${{sel.x.toFixed(2)}}, Y: ${{sel.y.toFixed(2)}}`;
        document.getElementById('dosDuty').textContent = sel.duty;
        document.getElementById('dosLoyalty').textContent = `${{sel.morale}}%`;
        document.getElementById('dosHunger').textContent = `${{sel.hunger}}%`;
        document.getElementById('dosThoughts').textContent = `"${{sel.thoughts}}"`;
      }}
    }}

    function render() {{
      updateSimulation(1);
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      // 1. Draw Diamond Ground Grid
      for (let y = 0; y < GRID_H; y++) {{
        for (let x = 0; x < GRID_W; x++) {{
          const pt = worldToScreen(x, y);
          const tw = TILE_W * zoom;
          const th = TILE_H * zoom;

          ctx.beginPath();
          ctx.moveTo(pt.x, pt.y);
          ctx.lineTo(pt.x + tw / 2, pt.y + th / 2);
          ctx.lineTo(pt.x, pt.y + th);
          ctx.lineTo(pt.x - tw / 2, pt.y + th / 2);
          ctx.closePath();

          const isRoad = (x === 6 || y === 6);
          ctx.fillStyle = isRoad ? '#1b253b' : (((x + y) % 2 === 0) ? '#121b2d' : '#0d1524');
          ctx.fill();
          ctx.strokeStyle = '#1b2a45';
          ctx.lineWidth = 1;
          ctx.stroke();
        }}
      }}

      // 2. Combine Buildings + People into Single Depth-Sorted Draw List
      const drawList = [];

      WORLD_LAYOUT.forEach(bld => {{
        drawList.push({{
          depth: bld.x + bld.y,
          isBuilding: true,
          item: bld
        }});
      }});

      AGENTS.forEach(ag => {{
        drawList.push({{
          depth: ag.x + ag.y,
          isAgent: true,
          item: ag
        }});
      }});

      drawList.sort((a, b) => a.depth - b.depth);

      // 3. Render In Depth Order
      drawList.forEach(entry => {{
        if (entry.isBuilding) {{
          const bld = entry.item;
          const pt = worldToScreen(bld.x, bld.y);
          const img = getSpriteForElement(bld);

          if (img && img.complete && img.naturalWidth > 0) {{
            const scale = zoom * (currentSkin === 'caesar3' ? 0.8 : 1.2);
            const dw = img.naturalWidth * scale;
            const dh = img.naturalHeight * scale;
            const dx = pt.x - dw / 2;
            const dy = pt.y + (TILE_H * zoom) - dh;

            ctx.drawImage(img, dx, dy, dw, dh);

            // Smoke / Fire particles on damaged buildings
            if (bld.hp < 0.8 && Math.random() < 0.25) {{
              particles.push({{
                x: pt.x + (Math.random() - 0.5) * 20,
                y: dy + dh * 0.4,
                vx: (Math.random() - 0.5) * 0.5,
                vy: -Math.random() * 1.5 - 0.4,
                radius: Math.random() * 8 + 4,
                life: 1.0, decay: 0.02,
                color: bld.hp < 0.4 ? 'rgba(239, 68, 68, 0.7)' : 'rgba(148, 163, 184, 0.5)'
              }});
            }}
          }}
        }} else if (entry.isAgent) {{
          const ag = entry.item;
          const pt = worldToScreen(ag.x, ag.y);
          const img = getAgentFrame(ag);

          if (img && img.complete && img.naturalWidth > 0) {{
            const scale = zoom * (ag.unitType === 'knight' ? 0.7 : 0.9);
            const dw = img.naturalWidth * scale;
            const dh = img.naturalHeight * scale;
            const dx = pt.x - dw / 2;
            const dy = pt.y + (TILE_H * zoom) / 2 - dh;

            ctx.drawImage(img, dx, dy, dw, dh);

            // Selection Reticle & Nameplate
            if (ag.id === selectedAgentId) {{
              ctx.strokeStyle = '#00e5ff';
              ctx.lineWidth = 1.5;
              ctx.beginPath();
              ctx.ellipse(pt.x, pt.y + (TILE_H * zoom) / 2, 14 * zoom, 7 * zoom, 0, 0, Math.PI * 2);
              ctx.stroke();

              // Name label
              ctx.font = '9px "JetBrains Mono"';
              ctx.fillStyle = '#00e5ff';
              ctx.textAlign = 'center';
              ctx.fillText(ag.name.split(' ')[0], pt.x, dy - 4);
            }}
          }}
        }}
      }});

      // 4. Update and draw particles
      for (let i = particles.length - 1; i >= 0; i--) {{
        const p = particles[i];
        p.x += p.vx; p.y += p.vy; p.life -= p.decay;
        if (p.life <= 0) {{ particles.splice(i, 1); continue; }}
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.radius * (1.5 - p.life * 0.5), 0, Math.PI * 2);
        ctx.fillStyle = p.color;
        ctx.globalAlpha = p.life;
        ctx.fill();
        ctx.globalAlpha = 1.0;
      }}

      // 5. Update and draw catapult projectiles
      for (let i = projectiles.length - 1; i >= 0; i--) {{
        const pr = projectiles[i];
        pr.progress += 0.03;
        const curX = pr.startX + (pr.targetX - pr.startX) * pr.progress;
        const curY = pr.startY + (pr.targetY - pr.startY) * pr.progress - Math.sin(pr.progress * Math.PI) * 120;

        ctx.beginPath();
        ctx.arc(curX, curY, 4, 0, Math.PI * 2);
        ctx.fillStyle = '#f59e0b';
        ctx.fill();

        if (pr.progress >= 1.0) {{
          for (let k = 0; k < 12; k++) {{
            particles.push({{
              x: pr.targetX, y: pr.targetY,
              vx: (Math.random() - 0.5) * 4, vy: (Math.random() - 0.5) * 4,
              radius: Math.random() * 6 + 2, life: 1.0, decay: 0.04,
              color: 'rgba(239, 68, 68, 0.9)'
            }});
          }}
          projectiles.splice(i, 1);
        }}
      }}

      requestAnimationFrame(render);
    }}

    function applyTickData(tickNum) {{
      currentTick = tickNum;
      const data = TIMELINE_DATA[tickNum - 1];

      document.getElementById('hudTick').textContent = `MONTH ${{String(tickNum).padStart(2, '0')}} / 06`;
      document.getElementById('hudGrain').textContent = data.grain.toLocaleString();
      document.getElementById('hudSurprise').textContent = `${{data.surprise}} bits`;
      document.getElementById('hudPopulation').textContent = `${{data.population}} Citizens`;

      // Climax banner check
      const climaxBanner = document.getElementById('climaxBanner');
      if (data.surprise > 50) {{
        climaxBanner.style.display = 'flex';
      }} else {{
        climaxBanner.style.display = 'none';
      }}

      // Apply structural damage
      WORLD_LAYOUT.forEach(elem => elem.hp = 1.0);
      for (let t = 0; t < tickNum; t++) {{
        const epoch = TIMELINE_DATA[t];
        epoch.hits.forEach(hit => {{
          const b = WORLD_LAYOUT.find(e => e.x === hit.x && e.y === hit.y);
          if (b) b.hp = Math.max(0.1, b.hp - 0.4);
        }});
      }}

      // Catapult fireballs
      if (data.hits.length > 0) {{
        data.hits.forEach(hit => {{
          const targetPt = worldToScreen(hit.x, hit.y);
          projectiles.push({{
            startX: targetPt.x - 220, startY: targetPt.y + 160,
            targetX: targetPt.x, targetY: targetPt.y, progress: 0
          }});
        }});
      }}

      // Populate chronicle feed
      const feed = document.getElementById('chronicleFeed');
      feed.innerHTML = '';
      for (let i = 0; i < tickNum; i++) {{
        const ep = TIMELINE_DATA[i];
        const item = document.createElement('div');
        item.className = `feed-item ${{ep.surprise > 50 ? 'alert' : (ep.surprise > 30 ? 'warning' : '')}} ${{i === tickNum - 1 ? 'active' : ''}}`;
        item.innerHTML = `
          <div class="feed-tick">[TICK 0${{ep.tick}}] ${{ep.title}} &bull; U = ${{ep.surprise}} bits</div>
          <div>${{ep.desc}}</div>
        `;
        feed.appendChild(item);
      }}
      feed.scrollTop = feed.scrollHeight;
    }}

    // Click on canvas to select moving agent
    canvas.addEventListener('click', e => {{
      const rect = canvas.getBoundingClientRect();
      const clickX = e.clientX - rect.left;
      const clickY = e.clientY - rect.top;

      let closest = null;
      let minD = 30;

      AGENTS.forEach(ag => {{
        const pt = worldToScreen(ag.x, ag.y);
        const d = Math.hypot(clickX - pt.x, clickY - (pt.y + (TILE_H * zoom) / 2));
        if (d < minD) {{
          minD = d;
          closest = ag;
        }}
      }});

      if (closest) {{
        selectedAgentId = closest.id;
      }}
    }});

    // Impulses
    function impulseRally() {{
      AGENTS.forEach(ag => {{
        if (ag.unitType === 'archer' || ag.unitType === 'knight') {{
          ag.targetX = 6.0; ag.targetY = 9.0;
          ag.waypoints = [{{ x: 6.0, y: 9.0 }}];
          ag.wpIndex = 0;
          ag.duty = 'DEFENDING_SOUTHERN_GATE';
          ag.thoughts = 'All hands to the breach! Form shield wall at the gate portcullis!';
        }}
      }});
    }}

    function impulseDistributeGrain() {{
      const courier = AGENTS.find(a => a.id === 'courier_1');
      if (courier) {{
        courier.duty = 'EMERGENCY_GRAIN_RATIONING';
        courier.thoughts = 'Ration trucks deploying from royal granary. Moving double-time to the West quarter.';
        courier.speed = 0.035;
      }}
    }}

    function impulseCatapultStrike() {{
      const targetPt = worldToScreen(6, 6);
      projectiles.push({{
        startX: targetPt.x - 220, startY: targetPt.y + 160,
        targetX: targetPt.x, targetY: targetPt.y, progress: 0
      }});
      // Scatter nearby peasants
      AGENTS.forEach(ag => {{
        if (ag.unitType === 'peasant') {{
          ag.thoughts = 'INCOMING CATAPULT FIRE! Take cover in the stone undercroft!';
          ag.speed = 0.03;
        }}
      }});
    }}

    function switchSkin(skin) {{
      currentSkin = skin;
      document.getElementById('btnSkinSH').classList.toggle('active', skin === 'stronghold');
      document.getElementById('btnSkinC3').classList.toggle('active', skin === 'caesar3');
      document.getElementById('btnSkinSC2K').classList.toggle('active', skin === 'sc2k');
    }}

    function togglePlay() {{
      isPlaying = !isPlaying;
      const btn = document.getElementById('btnPlay');
      if (isPlaying) {{
        btn.textContent = '⏸ PAUSE';
        playTimer = setInterval(() => {{
          if (currentTick < 6) applyTickData(currentTick + 1);
          else togglePlay();
        }}, 2400);
      }} else {{
        btn.textContent = '▶ PLAY STREAM';
        clearInterval(playTimer);
      }}
    }}

    function stepTick() {{ if (currentTick < 6) applyTickData(currentTick + 1); }}
    function resetSim() {{ if (isPlaying) togglePlay(); applyTickData(1); }}
    function openClimaxVignette() {{ window.location.href = 'climax_voxel_vignette.html'; }}

    // Start
    applyTickData(1);
    togglePlay();
    requestAnimationFrame(render);
  </script>
</body>
</html>
"""
    out_file = Path("docs/red_team/prototypes/live_world_stream.html")
    with open(out_file, 'w', encoding='utf-8') as f:
        f.write(html)
    print(f"[★] Successfully compiled Live World Stream with animated people to {out_file} ({len(html)/1024:.1f} KB)")

if __name__ == '__main__':
    main()
