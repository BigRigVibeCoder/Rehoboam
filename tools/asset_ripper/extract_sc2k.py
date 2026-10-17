#!/usr/bin/env python3
"""SimCity 2000 Asset Extraction Tool for Rehoboam

Extracts proprietary Maxis SC2K sprite archives (.DAT files) and 256-color
palettes into standardized 32-bit RGBA PNG files with transparency.
"""

import sys
import os
import struct
import json
from pathlib import Path
from PIL import Image

def decode_sc2k_rows(raw_data, w, h, palette):
    """Decodes SimCity 2000 row-based RLE chunks into a transparent PNG image."""
    posn = 0
    rows = []
    while posn < len(raw_data) - 2:
        block_len = raw_data[posn]
        extra = raw_data[posn + 1]
        posn += 2
        if extra == 2 or block_len == 0:
            break
        rows.append(raw_data[posn:posn + block_len])
        posn += block_len

    img_h = max(h, len(rows))
    img_w = max(w, 1)
    img = Image.new('RGBA', (img_w, img_h), (0, 0, 0, 0))
    pixels = img.load()

    for row_idx, row_data in enumerate(rows):
        offset = 0
        pixel_idx = 0
        while offset < len(row_data) - 1:
            row_mode = row_data[offset + 1]
            count = row_data[offset]
            offset += 2
            if row_mode == 3: # Skip transparent
                pixel_idx += count
            elif row_mode == 4: # Pixel stream
                for _ in range(count):
                    if offset < len(row_data) and pixel_idx < img.width and row_idx < img.height:
                        color_idx = row_data[offset]
                        pixels[pixel_idx, row_idx] = palette[color_idx]
                        pixel_idx += 1
                        offset += 1
                if count % 2 != 0:
                    offset += 1 # 16-bit alignment padding

    return img

def extract_sc2000_dat(dat_path, output_dir):
    """Unpacks Maxis SC2000.DAT container and extracts all large and small sprites."""
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    with open(dat_path, 'rb') as f:
        data = f.read()

    pos = 0
    table = []
    while pos + 16 <= len(data):
        entry = data[pos:pos + 16]
        name = entry[:12].decode('ascii', errors='ignore').strip('\x00')
        if not name or name[0] == '\x00':
            break
        off, = struct.unpack('<I', entry[12:16])
        table.append((name, off))
        pos += 16

    files = {}
    for i in range(len(table)):
        name, off = table[i]
        next_off = table[i + 1][1] if i + 1 < len(table) else len(data)
        files[name] = data[off:next_off]

    print(f"[*] Unpacking SC2000.DAT: Found {len(files)} subfiles.")

    # 1. Parse Palette (MINE.PAL)
    palette = []
    if 'MINE.PAL' in files:
        pal_raw = files['MINE.PAL']
        for i in range(256):
            if (i + 1) * 3 <= len(pal_raw):
                r, g, b = pal_raw[i*3], pal_raw[i*3+1], pal_raw[i*3+2]
                palette.append((r, g, b, 255))
            else:
                palette.append((i, i, i, 255))
    else:
        # Grayscale fallback
        palette = [(i, i, i, 255) for i in range(256)]

    manifest = {
        "source": dat_path.name,
        "subfiles_count": len(files),
        "sprites": []
    }

    extracted_count = 0

    # 2. Extract LARGE sprites
    if 'LARGE.HED' in files and 'LARGE.DAT' in files:
        hed = files['LARGE.HED']
        dat = files['LARGE.DAT']
        large_dir = output_dir / "large"
        large_dir.mkdir(parents=True, exist_ok=True)

        for i in range(len(hed) // 8):
            chunk = hed[i * 8:(i + 1) * 8]
            off, = struct.unpack('<I', chunk[:4])
            if off == 0xFFFFFFFF or off >= len(dat):
                continue
            h = chunk[4]
            w = chunk[5] * 2

            img = decode_sc2k_rows(dat[off:], w, h, palette)
            out_name = f"sc2k_large_{i:04d}.png"
            img.save(large_dir / out_name)
            extracted_count += 1
            manifest["sprites"].append({
                "tier": "large",
                "id": i,
                "filename": f"large/{out_name}",
                "width": img.width,
                "height": img.height
            })

    # 3. Extract SMALL sprites
    if 'SMALL.HED' in files and 'SMALL.DAT' in files:
        hed = files['SMALL.HED']
        dat = files['SMALL.DAT']
        small_dir = output_dir / "small"
        small_dir.mkdir(parents=True, exist_ok=True)

        for i in range(len(hed) // 8):
            chunk = hed[i * 8:(i + 1) * 8]
            off, = struct.unpack('<I', chunk[:4])
            if off == 0xFFFFFFFF or off >= len(dat):
                continue
            h = chunk[4]
            w = chunk[5] * 2

            img = decode_sc2k_rows(dat[off:], w, h, palette)
            out_name = f"sc2k_small_{i:04d}.png"
            img.save(small_dir / out_name)
            extracted_count += 1
            manifest["sprites"].append({
                "tier": "small",
                "id": i,
                "filename": f"small/{out_name}",
                "width": img.width,
                "height": img.height
            })

    with open(output_dir / "manifest.json", "w") as mf:
        json.dump(manifest, mf, indent=2)

    print(f"[✓] Extracted {extracted_count} Maxis SimCity 2000 sprites to {output_dir}")
    return extracted_count

def scan_and_extract_sc2k(game_dir, output_dir):
    """Scans for SC2000.DAT and extracts sprites."""
    game_dir = Path(game_dir)
    dat_files = list(game_dir.rglob("SC2000.DAT")) + list(game_dir.rglob("sc2000.dat"))
    if not dat_files:
        dat_files = [f for f in game_dir.rglob("*.dat") if "sc2" in f.name.lower()]

    total = 0
    for dat in dat_files:
        total += extract_sc2000_dat(dat, Path(output_dir) / dat.stem.lower())
    return total

if __name__ == '__main__':
    if len(sys.argv) < 3:
        print("Usage: python extract_sc2k.py <path_to_SC2000.DAT_or_dir> <output_dir>")
        sys.exit(1)

    target = Path(sys.argv[1])
    out = Path(sys.argv[2])
    if target.is_dir():
        scan_and_extract_sc2k(target, out)
    else:
        extract_sc2000_dat(target, out)
