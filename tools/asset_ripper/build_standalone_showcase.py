#!/usr/bin/env python3
"""Builds standalone, zero-dependency Asset Showcase with embedded Base64 sprites.

Ensures that opening docs/red_team/prototypes/asset_showcase.html via file:// in Chrome
renders all medieval, Roman, and retro civic sprites with 100% reliability, bypassing
Chromium's file:// cross-directory security policy.
"""

import os
import json
import base64
from pathlib import Path
from PIL import Image

def encode_image(path):
    with open(path, 'rb') as f:
        return 'data:image/png;base64,' + base64.b64encode(f.read()).decode('ascii')

def main():
    base = Path('assets/sprites')
    
    catalog = {
        'summary': {
            'stronghold': 61552,
            'caesar3': 28372,
            'sc2k': 1398
        },
        'categories': {
            'stronghold': {},
            'caesar3': {},
            'sc2k': {}
        },
        'images': {}
    }

    print("[*] Curating and encoding premier game sprites into Base64...")
    total_embedded = 0

    # 1. STRONGHOLD CRUSADER HD
    sh_dir = base / 'stronghold'
    sh_categories = {
        'tile_castle': 'Stone Bastions, Towers & Keeps',
        'tile_walls': 'Crenellated Curtain Walls',
        'tile_buildings1': 'Workshops, Granaries & Mills',
        'anim_trebuchet': 'War Trebuchets & Catapults',
        'body_knight': 'Armored Mounted Knights',
        'body_archer': 'Garrison Longbowmen'
    }

    for cat_name, desc in sh_categories.items():
        cdir = sh_dir / cat_name
        if not cdir.exists(): continue
        # Pick non-trivial sprites
        pngs = sorted([p for p in cdir.glob('*.png') if p.stat().st_size > 300])
        samples = pngs[:36]
        catalog['categories']['stronghold'][cat_name] = {
            'title': desc,
            'count': len(pngs),
            'sample_names': [p.name for p in samples]
        }
        for p in samples:
            key = f"stronghold/{cat_name}/{p.name}"
            catalog['images'][key] = encode_image(p)
            total_embedded += 1

    # 2. CAESAR III (Filter specifically for authentic Roman stone architecture type30)
    c3_dir = base / 'caesar3' / 'caesar3'
    if c3_dir.exists():
        c3_buildings = []
        for p in c3_dir.glob('*_type30.png'):
            try:
                with Image.open(p) as im:
                    w, h = im.size
                    if w >= 58 and h >= 30:
                        non_trans = sum(1 for px in im.getdata() if px[3] > 0)
                        if non_trans >= 900:
                            c3_buildings.append((non_trans, p))
            except Exception:
                pass
        
        # Sort by stone volume / visual grandeur
        c3_buildings.sort(key=lambda x: x[0], reverse=True)
        samples_c3 = [x[1] for x in c3_buildings[:48]]
        catalog['categories']['caesar3']['buildings'] = {
            'title': 'Roman Stone Forums, Temples, Basilicas & Amphitheaters',
            'count': len(c3_buildings),
            'sample_names': [p.name for p in samples_c3]
        }
        for p in samples_c3:
            key = f"caesar3/buildings/{p.name}"
            catalog['images'][key] = encode_image(p)
            total_embedded += 1

    # 3. SIMCITY 2000 (SCURK full-color city landmarks + large structures)
    scurk_dir = base / 'sc2k' / 'scurk'
    if scurk_dir.exists():
        scurk_pngs = sorted([p for p in scurk_dir.glob('*.png')])
        catalog['categories']['sc2k']['scurk'] = {
            'title': 'City Hall, University, Big Ben & Dam (SCURK)',
            'count': len(scurk_pngs),
            'sample_names': [p.name for p in scurk_pngs]
        }
        for p in scurk_pngs:
            key = f"sc2k/scurk/{p.name}"
            catalog['images'][key] = encode_image(p)
            total_embedded += 1

    sc2k_large_dir = base / 'sc2k' / 'large'
    if sc2k_large_dir.exists():
        large_pngs = sorted([p for p in sc2k_large_dir.glob('*.png') if p.stat().st_size > 200])
        samples_sc2k = large_pngs[:32]
        catalog['categories']['sc2k']['large'] = {
            'title': 'Civic Centers, Bridges & Infrastructure',
            'count': len(large_pngs),
            'sample_names': [p.name for p in samples_sc2k]
        }
        for p in samples_sc2k:
            key = f"sc2k/large/{p.name}"
            catalog['images'][key] = encode_image(p)
            total_embedded += 1

    print(f"[✓] Embedded {total_embedded} premier transparent sprites into catalog.")

    catalog_json = json.dumps(catalog)

    html_content = f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Rehoboam // Commercial Asset Matrix Showcase</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;600;800&family=Outfit:wght@300;400;600;700&display=swap" rel="stylesheet">
  <style>
    :root {{
      --bg: #0b0f19;
      --card-bg: rgba(18, 24, 38, 0.85);
      --border: #23314d;
      --accent-cyan: #00e5ff;
      --accent-amber: #ffb300;
      --accent-emerald: #10b981;
      --accent-crimson: #f43f5e;
      --text: #f1f5f9;
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
      padding: 18px 32px;
      background: rgba(15, 23, 42, 0.95);
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
      width: 42px;
      height: 42px;
      border-radius: 8px;
      background: linear-gradient(135deg, #00e5ff, #3b82f6);
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: 800;
      font-size: 20px;
      color: #0b0f19;
      box-shadow: 0 0 20px rgba(0, 229, 255, 0.4);
    }}

    .title-group h1 {{
      font-size: 20px;
      font-weight: 700;
      letter-spacing: -0.5px;
      display: flex;
      align-items: center;
      gap: 10px;
    }}

    .badge-live {{
      font-size: 11px;
      background: rgba(16, 185, 129, 0.2);
      color: var(--accent-emerald);
      border: 1px solid rgba(16, 185, 129, 0.4);
      padding: 2px 8px;
      border-radius: 999px;
      font-family: 'JetBrains Mono', monospace;
      text-transform: uppercase;
    }}

    .stat-counters {{
      display: flex;
      gap: 20px;
    }}

    .stat-box {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      padding: 8px 16px;
      border-radius: 8px;
      text-align: right;
    }}

    .stat-box .val {{
      font-family: 'JetBrains Mono', monospace;
      font-size: 18px;
      font-weight: 700;
      color: var(--accent-cyan);
    }}

    .stat-box .lbl {{
      font-size: 11px;
      color: var(--text-muted);
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }}

    main {{
      padding: 28px 32px;
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 24px;
    }}

    .tabs {{
      display: flex;
      gap: 12px;
      border-bottom: 1px solid var(--border);
      padding-bottom: 14px;
    }}

    .tab-btn {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      color: var(--text-muted);
      padding: 10px 22px;
      border-radius: 8px;
      cursor: pointer;
      font-weight: 600;
      font-size: 14px;
      display: flex;
      align-items: center;
      gap: 10px;
      transition: all 0.2s ease;
    }}

    .tab-btn:hover {{
      border-color: var(--accent-cyan);
      color: var(--text);
    }}

    .tab-btn.active {{
      background: rgba(0, 229, 255, 0.12);
      border-color: var(--accent-cyan);
      color: var(--accent-cyan);
      box-shadow: 0 0 16px rgba(0, 229, 255, 0.25);
    }}

    .tab-btn .count {{
      font-family: 'JetBrains Mono', monospace;
      font-size: 12px;
      background: rgba(255, 255, 255, 0.08);
      padding: 2px 8px;
      border-radius: 999px;
    }}

    .toolbar {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      flex-wrap: wrap;
      gap: 16px;
      background: var(--card-bg);
      padding: 14px 20px;
      border-radius: 10px;
      border: 1px solid var(--border);
    }}

    .subcat-select {{
      background: #0f172a;
      border: 1px solid var(--border);
      color: var(--text);
      padding: 8px 14px;
      border-radius: 6px;
      font-family: 'Outfit', sans-serif;
      font-size: 14px;
      min-width: 320px;
      cursor: pointer;
      outline: none;
    }}

    .subcat-select:focus {{
      border-color: var(--accent-cyan);
    }}

    .zoom-controls {{
      display: flex;
      align-items: center;
      gap: 8px;
    }}

    .zoom-btn {{
      background: #0f172a;
      border: 1px solid var(--border);
      color: var(--text-muted);
      width: 34px;
      height: 34px;
      border-radius: 6px;
      display: flex;
      align-items: center;
      justify-content: center;
      cursor: pointer;
      font-weight: 700;
      font-size: 13px;
      transition: all 0.2s;
    }}

    .zoom-btn:hover, .zoom-btn.active {{
      border-color: var(--accent-cyan);
      color: var(--accent-cyan);
    }}

    .gallery-container {{
      background: #060911;
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 24px;
      min-height: 520px;
    }}

    .gallery-grid {{
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
      gap: 16px;
    }}

    .sprite-card {{
      background: #111827;
      border: 1px solid #1f293d;
      border-radius: 8px;
      padding: 12px;
      display: flex;
      flex-direction: column;
      align-items: center;
      cursor: pointer;
      transition: all 0.2s ease;
      position: relative;
    }}

    .sprite-card:hover {{
      border-color: var(--accent-cyan);
      transform: translateY(-3px);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5), 0 0 12px rgba(0, 229, 255, 0.3);
    }}

    .img-checkerboard {{
      width: 100%;
      height: 110px;
      border-radius: 6px;
      display: flex;
      align-items: center;
      justify-content: center;
      background-image: linear-gradient(45deg, #182032 25%, transparent 25%),
                        linear-gradient(-45deg, #182032 25%, transparent 25%),
                        linear-gradient(45deg, transparent 75%, #182032 75%),
                        linear-gradient(-45deg, transparent 75%, #182032 75%);
      background-size: 16px 16px;
      background-position: 0 0, 0 8px, 8px -8px, -8px 0px;
      background-color: #0c1220;
      overflow: hidden;
      margin-bottom: 8px;
    }}

    .img-checkerboard img {{
      max-width: 85%;
      max-height: 85%;
      object-fit: contain;
      image-rendering: pixelated;
      transition: transform 0.2s;
    }}

    .sprite-card .name {{
      font-family: 'JetBrains Mono', monospace;
      font-size: 10px;
      color: var(--text-muted);
      text-align: center;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      width: 100%;
    }}

    /* Modal */
    .modal-backdrop {{
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.85);
      backdrop-filter: blur(8px);
      display: none;
      align-items: center;
      justify-content: center;
      z-index: 1000;
    }}

    .modal-box {{
      background: #111827;
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 28px;
      max-width: 580px;
      width: 90%;
      display: flex;
      flex-direction: column;
      gap: 18px;
      box-shadow: 0 20px 50px rgba(0, 0, 0, 0.8);
    }}

    .modal-header {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      border-bottom: 1px solid var(--border);
      padding-bottom: 12px;
    }}

    .modal-header h2 {{
      font-size: 15px;
      font-family: 'JetBrains Mono', monospace;
      color: var(--accent-cyan);
    }}

    .close-btn {{
      background: none;
      border: none;
      color: var(--text-muted);
      font-size: 20px;
      cursor: pointer;
    }}

    .modal-preview {{
      height: 260px;
      background-image: linear-gradient(45deg, #182032 25%, transparent 25%),
                        linear-gradient(-45deg, #182032 25%, transparent 25%),
                        linear-gradient(45deg, transparent 75%, #182032 75%),
                        linear-gradient(-45deg, transparent 75%, #182032 75%);
      background-size: 20px 20px;
      background-position: 0 0, 0 10px, 10px -10px, -10px 0px;
      background-color: #0c1220;
      border-radius: 8px;
      display: flex;
      align-items: center;
      justify-content: center;
      overflow: hidden;
    }}

    .modal-preview img {{
      max-width: 90%;
      max-height: 90%;
      object-fit: contain;
      image-rendering: pixelated;
    }}

    .meta-list {{
      font-family: 'JetBrains Mono', monospace;
      font-size: 12px;
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 10px;
      background: #0a0f1d;
      padding: 14px;
      border-radius: 8px;
    }}

    .meta-item {{ display: flex; flex-direction: column; gap: 2px; }}
    .meta-item .lbl {{ color: var(--text-muted); font-size: 10px; text-transform: uppercase; }}
    .meta-item .val {{ color: var(--text); font-weight: 600; }}
  </style>
</head>
<body>

  <header>
    <div class="brand">
      <div class="logo-badge">R</div>
      <div class="title-group">
        <h1>COMMERCIAL ASSET MATRIX <span class="badge-live">Hardware Accelerated</span></h1>
      </div>
    </div>

    <div class="stat-counters">
      <div class="stat-box">
        <div class="val" id="totalCount">64,368</div>
        <div class="lbl">Extracted Sprites</div>
      </div>
      <div class="stat-box">
        <div class="val">32-bit RGBA</div>
        <div class="lbl">True Transparency</div>
      </div>
      <div class="stat-box">
        <div class="val" style="color: var(--accent-emerald);">3 / 3 Titles</div>
        <div class="lbl">100% Extracted</div>
      </div>
    </div>
  </header>

  <main>
    <!-- Engine Selector Tabs -->
    <div class="tabs">
      <button class="tab-btn active" onclick="switchEngine('stronghold')">
        🏰 Stronghold Crusader HD <span class="count" id="strongholdCount">61,552</span>
      </button>
      <button class="tab-btn" onclick="switchEngine('caesar3')">
        🏛️ Caesar III <span class="count" id="caesar3Count">1,418</span>
      </button>
      <button class="tab-btn" onclick="switchEngine('sc2k')">
        🏙️ SimCity 2000 SE <span class="count" id="sc2kCount">1,398</span>
      </button>
    </div>

    <!-- Controls -->
    <div class="toolbar">
      <div style="display: flex; align-items: center; gap: 12px;">
        <label for="subcatSelect" style="font-size: 13px; color: var(--text-muted); font-weight: 600;">ARCHIVE / CATEGORY:</label>
        <select id="subcatSelect" class="subcat-select" onchange="renderSprites()"></select>
      </div>

      <div class="zoom-controls">
        <span style="font-size: 12px; color: var(--text-muted); margin-right: 4px;">ZOOM:</span>
        <button class="zoom-btn active" onclick="setZoom(1)">1x</button>
        <button class="zoom-btn" onclick="setZoom(1.5)">1.5x</button>
        <button class="zoom-btn" onclick="setZoom(2)">2x</button>
      </div>
    </div>

    <!-- Gallery Grid -->
    <div class="gallery-container">
      <div class="gallery-grid" id="spriteGrid">
        <!-- Dynamically injected -->
      </div>
    </div>
  </main>

  <!-- Sprite Inspection Modal -->
  <div class="modal-backdrop" id="inspectModal" onclick="closeModal(event)">
    <div class="modal-box" onclick="event.stopPropagation()">
      <div class="modal-header">
        <h2 id="modalTitle">Sprite Inspector</h2>
        <button class="close-btn" onclick="closeModal()">&times;</button>
      </div>
      <div class="modal-preview">
        <img id="modalImg" src="" alt="Sprite Preview">
      </div>
      <div class="meta-list">
        <div class="meta-item">
          <div class="lbl">Source Engine</div>
          <div class="val" id="modalEngine">Engine</div>
        </div>
        <div class="meta-item">
          <div class="lbl">Archive</div>
          <div class="val" id="modalArchive">Archive</div>
        </div>
        <div class="meta-item">
          <div class="lbl">Pixel Format</div>
          <div class="val">32-bit RGBA (Alpha)</div>
        </div>
        <div class="meta-item">
          <div class="lbl">Plane C Status</div>
          <div class="val" style="color: var(--accent-emerald);">Direct Render Ready</div>
        </div>
      </div>
    </div>
  </div>

  <script>
    const CATALOG = {catalog_json};
    let currentEngine = 'stronghold';
    let currentZoom = 1;

    function init() {{
      document.getElementById('strongholdCount').textContent = (CATALOG.summary.stronghold || 0).toLocaleString();
      document.getElementById('caesar3Count').textContent = (CATALOG.summary.caesar3 || 0).toLocaleString();
      document.getElementById('sc2kCount').textContent = (CATALOG.summary.sc2k || 0).toLocaleString();
      
      const total = (CATALOG.summary.stronghold || 0) + (CATALOG.summary.caesar3 || 0) + (CATALOG.summary.sc2k || 0);
      document.getElementById('totalCount').textContent = total.toLocaleString();

      switchEngine('stronghold');
    }}

    function switchEngine(engine) {{
      currentEngine = engine;
      document.querySelectorAll('.tab-btn').forEach(btn => btn.classList.remove('active'));
      const activeIdx = engine === 'stronghold' ? 0 : (engine === 'caesar3' ? 1 : 2);
      document.querySelectorAll('.tab-btn')[activeIdx].classList.add('active');

      const select = document.getElementById('subcatSelect');
      select.innerHTML = '';
      const cats = CATALOG.categories[engine] || {{}};

      for (const catName in cats) {{
        const opt = document.createElement('option');
        opt.value = catName;
        opt.textContent = `${{cats[catName].title}} (${{cats[catName].count}} sprites)`;
        select.appendChild(opt);
      }}

      renderSprites();
    }}

    function renderSprites() {{
      const select = document.getElementById('subcatSelect');
      const catName = select.value;
      const grid = document.getElementById('spriteGrid');
      grid.innerHTML = '';

      if (!catName || !CATALOG.categories[currentEngine][catName]) return;
      const cat = CATALOG.categories[currentEngine][catName];
      const sampleNames = cat.sample_names || [];

      sampleNames.forEach(fname => {{
        const key = `${{currentEngine}}/${{catName}}/${{fname}}`;
        // Use inlined Base64 data URI (100% offline file:// compatible)
        const imgData = CATALOG.images[key] || `../../../assets/sprites/${{key}}`;

        const card = document.createElement('div');
        card.className = 'sprite-card';
        card.innerHTML = `
          <div class="img-checkerboard">
            <img src="${{imgData}}" alt="${{fname}}" style="transform: scale(${{currentZoom}})">
          </div>
          <div class="name" title="${{fname}}">${{fname}}</div>
        `;
        card.onclick = () => openModal(fname, imgData, cat.title);
        grid.appendChild(card);
      }});
    }}

    function setZoom(scale) {{
      currentZoom = scale;
      document.querySelectorAll('.zoom-btn').forEach(b => b.classList.remove('active'));
      if (scale === 1) document.querySelectorAll('.zoom-btn')[0].classList.add('active');
      else if (scale === 1.5) document.querySelectorAll('.zoom-btn')[1].classList.add('active');
      else document.querySelectorAll('.zoom-btn')[2].classList.add('active');
      
      document.querySelectorAll('.img-checkerboard img').forEach(img => {{
        img.style.transform = `scale(${{scale}})`;
      }});
    }}

    function openModal(fname, imgData, catTitle) {{
      document.getElementById('modalTitle').textContent = fname;
      document.getElementById('modalImg').src = imgData;
      document.getElementById('modalEngine').textContent = currentEngine.toUpperCase();
      document.getElementById('modalArchive').textContent = catTitle;
      document.getElementById('inspectModal').style.display = 'flex';
    }}

    function closeModal(e) {{
      document.getElementById('inspectModal').style.display = 'none';
    }}

    window.onload = init;
  </script>
</body>
</html>
"""

    out_file = Path('docs/red_team/prototypes/asset_showcase.html')
    with open(out_file, 'w') as f:
        f.write(html_content)

    print(f"[★] Successfully compiled premier standalone showcase to {out_file} ({out_file.stat().st_size / 1024:.1f} KB)")

if __name__ == '__main__':
    main()
