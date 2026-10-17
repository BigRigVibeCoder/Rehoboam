#!/usr/bin/env python3
"""Caesar III Asset Extraction Tool for Rehoboam

Extracts Impressions Games SG2 image archives (c3.sg2 / c3.555) into standardized
32-bit RGBA transparent PNG files for classical Roman architecture.
Reverse-engineered using the Julius reference engine specifications.
"""

import sys
import os
import struct
import json
from pathlib import Path
from PIL import Image

def rgb555_to_rgba(val):
    """Converts 16-bit 555 RGB to (R, G, B, A). 0x0000, 0x7C1F, or 0xF81F is transparent mask."""
    if val == 0 or val == 0x7C1F or val == 0xF81F:
        return (0, 0, 0, 0)
    r = ((val >> 10) & 0x1F) * 255 // 31
    g = ((val >> 5) & 0x1F) * 255 // 31
    b = (val & 0x1F) * 255 // 31
    return (r, g, b, 255)

FOOTPRINT_X_START = [
    28, 26, 24, 22, 20, 18, 16, 14, 12, 10, 8, 6, 4, 2, 0,
    0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28
]

def draw_single_footprint(tile_bytes, pixels, x_offset, y_offset, max_w, max_h):
    """Draws a single 58x30 diamond isometric tile (900 pixels / 1800 bytes)."""
    if len(tile_bytes) < 1800:
        return
    tile_p = 0
    for row in range(30):
        x_start = FOOTPRINT_X_START[row]
        row_len = 58 - x_start * 2
        for x in range(row_len):
            val = struct.unpack_from('<H', tile_bytes, tile_p * 2)[0]
            tile_p += 1
            c = rgb555_to_rgba(val)
            px = x_offset + x_start + x
            py = y_offset + row
            if c[3] > 0 and 0 <= px < max_w and 0 <= py < max_h:
                pixels[px, py] = c

def decode_caesar3_image(e, data_555):
    """Decodes a single Caesar III image entry into a PIL Image."""
    w, h = e['width'], e['height']
    dlen = e['dlen']
    offset = e['offset']

    if w == 0 or h == 0 or dlen == 0 or offset + dlen > len(data_555):
        return None

    raw = data_555[offset:offset + dlen]
    img = Image.new('RGBA', (w, h), (0, 0, 0, 0))
    pixels = img.load()

    if e['type'] == 30:  # IMAGE_TYPE_ISOMETRIC (Buildings & Terrain)
        uncomp = e['uncomp_len']
        # 1. Base footprint tiles (each 1800 bytes / 900 pixels)
        if uncomp >= 1800:
            if w == 58:
                draw_single_footprint(raw[:1800], pixels, 0, h - 30, w, h)
            elif w == 118:
                # 2x2 tile footprint
                draw_single_footprint(raw[0:1800], pixels, 30, h - 60, w, h)
                draw_single_footprint(raw[1800:3600], pixels, 0, h - 45, w, h)
                draw_single_footprint(raw[3600:5400], pixels, 60, h - 45, w, h)
                draw_single_footprint(raw[5400:7200], pixels, 30, h - 30, w, h)
            elif w == 178:
                # 3x3 tile footprint
                draw_single_footprint(raw[0:1800], pixels, 60, h - 90, w, h)
                draw_single_footprint(raw[1800:3600], pixels, 30, h - 75, w, h)
                draw_single_footprint(raw[3600:5400], pixels, 90, h - 75, w, h)
                draw_single_footprint(raw[5400:7200], pixels, 0, h - 60, w, h)
                draw_single_footprint(raw[7200:9000], pixels, 60, h - 60, w, h)
                draw_single_footprint(raw[9000:10800], pixels, 120, h - 60, w, h)
            else:
                # Fallback: draw single tile at bottom center
                draw_single_footprint(raw[:1800], pixels, (w - 58) // 2, h - 30, w, h)

        # 2. Building Top (Compressed RLE)
        if e['has_comp'] and uncomp < len(raw):
            comp_data = raw[uncomp:]
            cp = 0
            for y in range(h):
                x = 0
                while x < w and cp < len(comp_data):
                    control = comp_data[cp]
                    cp += 1
                    if control == 255:
                        if cp >= len(comp_data):
                            break
                        skip = comp_data[cp]
                        cp += 1
                        x += skip
                    else:
                        for _ in range(control):
                            if cp + 1 < len(comp_data):
                                val = struct.unpack_from('<H', comp_data, cp)[0]
                                cp += 2
                                c = rgb555_to_rgba(val)
                                if c[3] > 0 and 0 <= x < w and 0 <= y < h:
                                    pixels[x, y] = c
                            x += 1
        return img

    elif e['comp']:  # Fully compressed RLE (characters, animations, monuments)
        cp = 0
        for y in range(h):
            x = 0
            while x < w and cp < len(raw):
                control = raw[cp]
                cp += 1
                if control == 255:
                    if cp >= len(raw):
                        break
                    skip = raw[cp]
                    cp += 1
                    x += skip
                else:
                    for _ in range(control):
                        if cp + 1 < len(raw):
                            val = struct.unpack_from('<H', raw, cp)[0]
                            cp += 2
                            c = rgb555_to_rgba(val)
                            if c[3] > 0 and 0 <= x < w and 0 <= y < h:
                                pixels[x, y] = c
                        x += 1
        return img

    else:  # Uncompressed 16-bit 555 bitmap
        p = 0
        for y in range(h):
            for x in range(w):
                if p * 2 + 1 < len(raw):
                    val = struct.unpack_from('<H', raw, p * 2)[0]
                    p += 1
                    c = rgb555_to_rgba(val)
                    if c[3] > 0:
                        pixels[x, y] = c
        return img

def extract_caesar3(sg2_path, bitmap_555_path, output_dir):
    """Parses c3.sg2 metadata and c3.555 pixel data to export PNGs."""
    output_dir = Path(output_dir) / "caesar3"
    output_dir.mkdir(parents=True, exist_ok=True)

    with open(sg2_path, 'rb') as f_sg2:
        sg2_data = f_sg2.read()
    with open(bitmap_555_path, 'rb') as f_555:
        data_555 = f_555.read()

    HEADER_SIZE = 20680
    ENTRY_SIZE = 64
    if len(sg2_data) < HEADER_SIZE + ENTRY_SIZE:
        print(f"Error: {sg2_path} too small.")
        return 0

    num_records = min(10000, (len(sg2_data) - HEADER_SIZE) // ENTRY_SIZE)
    print(f"[*] Parsing Caesar III SG2 ({sg2_path.name}): {num_records} index entries...")

    # Read and parse index entries
    entries = []
    curr_offset = 4
    for i in range(num_records):
        rec_off = HEADER_SIZE + i * ENTRY_SIZE
        offset, data_length, uncomp_len = struct.unpack_from('<iii', sg2_data, rec_off)
        width, height = struct.unpack_from('<HH', sg2_data, rec_off + 20)
        draw_type = sg2_data[rec_off + 50]
        is_fully_compressed = struct.unpack_from('<b', sg2_data, rec_off + 51)[0]
        is_external = struct.unpack_from('<b', sg2_data, rec_off + 52)[0]
        has_compressed_part = struct.unpack_from('<b', sg2_data, rec_off + 53)[0]

        if is_external:
            calc_off = offset
        else:
            calc_off = curr_offset
            curr_offset += data_length

        entries.append({
            'id': i,
            'width': width,
            'height': height,
            'dlen': data_length,
            'uncomp_len': uncomp_len,
            'type': draw_type,
            'comp': is_fully_compressed,
            'has_comp': has_compressed_part,
            'offset': calc_off
        })

    manifest = {
        "source_sg2": str(sg2_path),
        "source_555": str(bitmap_555_path),
        "total_records": num_records,
        "images": []
    }

    extracted = 0
    for e in entries:
        w, h = e['width'], e['height']
        if w < 16 or h < 16 or e['dlen'] == 0:
            continue

        img = decode_caesar3_image(e, data_555)
        if not img:
            continue

        # Check that image has non-transparent pixels
        non_trans = sum(1 for p in img.getdata() if p[3] > 0)
        if non_trans < 20:
            continue

        out_name = f"c3_img_{e['id']:04d}_type{e['type']}.png"
        img.save(output_dir / out_name)

        manifest["images"].append({
            "index": e['id'],
            "filename": out_name,
            "width": w,
            "height": h,
            "type": e['type'],
            "non_transparent_pixels": non_trans
        })
        extracted += 1

    with open(output_dir / "manifest.json", 'w') as mf:
        json.dump(manifest, mf, indent=2)

    print(f"[✓] Extracted {extracted} Roman classical sprites to {output_dir}")
    return extracted

def scan_and_extract_c3(game_dir, output_dir):
    """Scans and extracts all Caesar III SG2/555 pairs in game directory."""
    game_dir = Path(game_dir)
    sg2_files = sorted(list(game_dir.rglob("*.sg2")) + list(game_dir.rglob("*.SG2")))
    five_files = {f.name.lower(): f for f in (list(game_dir.rglob("*.555")) + list(game_dir.rglob("*.555")))}

    total = 0
    for sg2 in sg2_files:
        stem = sg2.stem.lower()
        matching_555 = five_files.get(f"{stem}.555")
        if not matching_555:
            # Try fuzzy match
            for k, v in five_files.items():
                if k.replace("_", "") == f"{stem.replace('_', '')}.555":
                    matching_555 = v
                    break
        if matching_555:
            sub_out = Path(output_dir) / sg2.stem
            count = extract_caesar3(sg2, matching_555, sub_out)
            total += count

    print(f"\n[Caesar III Ripper] Total {total} sprites extracted across {len(sg2_files)} archives.")
    return total

if __name__ == '__main__':
    if len(sys.argv) < 3:
        print("Usage: extract_caesar3.py <game_folder_or_sg2_file> <output_dir> [path_to_555_file]")
        sys.exit(1)

    target = Path(sys.argv[1])
    out = Path(sys.argv[2])
    if target.is_dir():
        scan_and_extract_c3(target, out)
    elif len(sys.argv) > 3:
        extract_caesar3(target, sys.argv[3], out)
    else:
        print("Provide path to .555 bitmap container.")
