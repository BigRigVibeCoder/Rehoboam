#!/usr/bin/env python3
"""Stronghold & Stronghold Crusader GM1 / TGX Asset Ripper for Rehoboam

Reference-aligned parser for Firefly Studios .gm1 container and .tgx formats.
Extracts 32-bit transparent PNGs with full anchor coordinates into manifest.json.
"""

import sys
import os
import struct
import json
from pathlib import Path
from PIL import Image

def argb1555_to_rgba(val, force_alpha=False):
    """Converts a 16-bit ARGB 1555 integer into (R, G, B, A)."""
    a_bit = (val >> 15) & 0x01
    r = ((val >> 10) & 0x1F) * 255 // 31
    g = ((val >> 5) & 0x1F) * 255 // 31
    b = (val & 0x1F) * 255 // 31
    
    if force_alpha:
        alpha = 255
    else:
        alpha = 255 if a_bit else 0
    return (r, g, b, alpha)

def decode_tgx_stream(data, width, height, palette=None):
    """Decodes Stronghold TGX RLE byte stream into Pillow RGBA image."""
    img = Image.new('RGBA', (width, height), (0, 0, 0, 0))
    pixels = img.load()

    pos = 0
    col = 0
    row = 0

    while pos < len(data) and row < height:
        token = data[pos]
        pos += 1
        token_type = token >> 5
        length = (token & 0x1F) + 1

        if token_type == 4:  # Newline
            row += 1
            col = 0
        elif token_type == 1:  # Transparent run
            col += length
        elif token_type == 0:  # Stream of pixels
            for _ in range(length):
                if pos + 2 <= len(data) and col < width and row < height:
                    val = struct.unpack_from('<H', data, pos)[0]
                    pixels[col, row] = argb1555_to_rgba(val, force_alpha=True)
                    pos += 2
                    col += 1
                elif pos + 2 <= len(data):
                    pos += 2
        elif token_type == 2:  # Repeating pixels
            if pos + 2 <= len(data):
                val = struct.unpack_from('<H', data, pos)[0]
                pos += 2
                color = argb1555_to_rgba(val, force_alpha=True)
                for _ in range(length):
                    if col < width and row < height:
                        pixels[col, row] = color
                        col += 1
        else:
            # Unknown / single pixel fallback
            if pos + 2 <= len(data) and col < width and row < height:
                val = struct.unpack_from('<H', data, pos)[0]
                pixels[col, row] = argb1555_to_rgba(val, force_alpha=True)
                pos += 2
                col += 1
            elif pos + 2 <= len(data):
                pos += 2

    return img

def decode_uncompressed(data, width, height):
    """Decodes raw 16-bit uncompressed pixels (DataTypes 5 and 7)."""
    img = Image.new('RGBA', (width, height), (0, 0, 0, 0))
    pixels = img.load()
    num_pixels = min(width * height, len(data) // 2)

    for i in range(num_pixels):
        x = i % width
        y = i // width
        val = struct.unpack_from('<H', data, i * 2)[0]
        # In Stronghold, 0 is often transparent for raw tiles
        if val == 0:
            pixels[x, y] = (0, 0, 0, 0)
        else:
            pixels[x, y] = argb1555_to_rgba(val, force_alpha=True)
    return img

def decode_tile_object(data, width, height):
    """Decodes building tile object (DataType 3): 512-byte diamond base + optional TGX."""
    # Base tile is 30x16 diamond (512 bytes)
    img = Image.new('RGBA', (max(width, 30), max(height, 16)), (0, 0, 0, 0))
    pixels = img.load()

    if len(data) >= 512:
        # Decode diamond base
        diamond_widths = [2, 6, 10, 14, 18, 22, 26, 30, 30, 26, 22, 18, 14, 10, 6, 2]
        base_ptr = 0
        for r, rw in enumerate(diamond_widths):
            start_x = (30 - rw) // 2
            for cx in range(rw):
                if base_ptr + 2 <= len(data):
                    val = struct.unpack_from('<H', data, base_ptr)[0]
                    base_ptr += 2
                    if val != 0 and start_x + cx < img.width and r < img.height:
                        pixels[start_x + cx, r] = argb1555_to_rgba(val, force_alpha=True)

    if len(data) > 512 and width > 0 and height > 0:
        # Overlay TGX top
        tgx_top = decode_tgx_stream(data[512:], width, height)
        img.paste(tgx_top, (0, 0), tgx_top)

    return img

def extract_gm1_archive(gm1_path, output_dir):
    """Extracts all images from a Stronghold .gm1 container."""
    with open(gm1_path, 'rb') as f:
        data = f.read()

    if len(data) < 88 + 5120:
        return 0

    # Header: 88 bytes (22 uint32s)
    fields = struct.unpack_from('<22I', data, 0)
    image_count = fields[3]
    data_type = fields[5]
    data_size = fields[20]

    if image_count == 0 or image_count > 10000:
        return 0

    tables_start = 88 + 5120
    bytes_per_image = 4 + 4 + 16  # offset (4), size (4), tgx_header (16)
    data_start = tables_start + image_count * bytes_per_image

    if data_start > len(data):
        return 0

    offsets_start = tables_start
    sizes_start = offsets_start + image_count * 4
    headers_start = sizes_start + image_count * 4

    offsets = [struct.unpack_from('<I', data, offsets_start + i * 4)[0] for i in range(image_count)]
    sizes = [struct.unpack_from('<I', data, sizes_start + i * 4)[0] for i in range(image_count)]

    target_dir = Path(output_dir) / gm1_path.stem
    target_dir.mkdir(parents=True, exist_ok=True)

    manifest = {
        "archive": gm1_path.name,
        "data_type": data_type,
        "image_count": image_count,
        "images": []
    }

    extracted_count = 0
    for i in range(image_count):
        h_pos = headers_start + i * 16
        w, h, off_x, off_y = struct.unpack_from('<HHHH', data, h_pos)
        part, subparts = data[h_pos + 8], data[h_pos + 9]
        tile_offset = struct.unpack_from('<H', data, h_pos + 10)[0]
        direction = data[h_pos + 12]

        start = data_start + offsets[i]
        length = sizes[i]

        if start + length > len(data) or length == 0:
            continue

        raw_bytes = data[start:start + length]

        # Decode based on DataType
        if data_type in (5, 7): # Uncompressed
            img = decode_uncompressed(raw_bytes, w if w > 0 else 30, h if h > 0 else 16)
        elif data_type == 3: # Tile Object (Buildings)
            img = decode_tile_object(raw_bytes, w, h)
        else: # Standard TGX (Types 1, 2, 4, 6)
            img = decode_tgx_stream(raw_bytes, w if w > 0 else 32, h if h > 0 else 32)

        out_name = f"{gm1_path.stem}_{i:04d}.png"
        img.save(target_dir / out_name)
        extracted_count += 1

        manifest["images"].append({
            "index": i,
            "filename": out_name,
            "width": img.width,
            "height": img.height,
            "offset_x": off_x,
            "offset_y": off_y,
            "part": part,
            "subparts": subparts,
            "tile_offset": tile_offset,
            "direction": direction
        })

    with open(target_dir / "manifest.json", "w") as mf:
        json.dump(manifest, mf, indent=2)

    return extracted_count

def scan_and_extract_stronghold(input_dir, output_dir):
    """Scans and extracts all Stronghold .gm1 and .tgx files."""
    input_path = Path(input_dir)
    gm1_files = sorted(list(input_path.rglob("*.gm1")) + list(input_path.rglob("*.GM1")))

    print(f"\n[Stronghold Ripper] Discovered {len(gm1_files)} GM1 archives.")
    total = 0
    for i, gm1 in enumerate(gm1_files):
        count = extract_gm1_archive(gm1, output_dir)
        total += count
        if (i + 1) % 10 == 0 or i == len(gm1_files) - 1:
            print(f"  * Processed {i+1}/{len(gm1_files)} GM1 archives (Extracted {total} sprites)...")

    print(f"[Stronghold Ripper] Done! Total {total} medieval sprites extracted to {output_dir}")
    return total

if __name__ == '__main__':
    if len(sys.argv) < 3:
        print("Usage: python extract_stronghold.py <input_dir_or_file> <output_dir>")
        sys.exit(1)

    inp = Path(sys.argv[1])
    out = Path(sys.argv[2])
    if inp.is_file() and inp.suffix.lower() == '.gm1':
        count = extract_gm1_archive(inp, out)
        print(f"Extracted {count} images.")
    else:
        scan_and_extract_stronghold(inp, out)
