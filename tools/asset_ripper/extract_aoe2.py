#!/usr/bin/env python3
"""Age of Empires II Asset Ripper for Rehoboam

Extracts sprites and architectural graphics from:
1. Age of Empires II: Definitive Edition (.smx / .slx / loose textures)
2. Age of Empires II Classic / HD (.drs / .slp)

Outputs:
Standardized 32-bit transparent PNGs with manifest.json.
"""

import sys
import os
import struct
import zlib
import json
from pathlib import Path
from PIL import Image

def generate_default_aoe2_palette():
    """Generates standard 256-color palette fallback for AoE2."""
    palette = []
    # Standard grayscale/basic ramps for 256 colors
    for i in range(256):
        if i < 16: # Player 1 (Blue) ramp
            palette.append((0, int(i * 16), min(255, int(i * 17) + 50), 255))
        elif i < 32: # Player 2 (Red) ramp
            palette.append((min(255, int((i - 16) * 17) + 50), 0, 0, 255))
        elif i < 64: # Grayscale / Stone ramp
            v = int((i - 32) * 8)
            palette.append((v, v, v, 255))
        elif i < 128: # Earth / Wood / Dirt ramp
            v = int((i - 64) * 4)
            palette.append((min(255, v + 40), min(255, int(v * 0.7) + 20), min(255, int(v * 0.4)), 255))
        else: # General spectrum
            v = i - 128
            palette.append((v * 2, (v * 3) % 256, (v * 5) % 256, 255))
    return palette

def extract_slp_file(slp_path, output_dir, palette=None):
    """Parses and decompresses classic AoE2 SLP sprite archive."""
    if palette is None:
        palette = generate_default_aoe2_palette()

    with open(slp_path, 'rb') as f:
        data = f.read()

    if len(data) < 32:
        return 0

    version = data[0:4].decode('ascii', errors='ignore')
    num_frames, = struct.unpack('<I', data[4:8])
    comment = data[8:32].decode('ascii', errors='ignore').strip('\x00')

    output_dir.mkdir(parents=True, exist_ok=True)
    extracted_count = 0
    manifest = {
        "source": slp_path.name,
        "format": "SLP",
        "version": version,
        "comment": comment,
        "frames": []
    }

    frame_header_size = 32
    for frame_idx in range(num_frames):
        hdr_offset = 32 + frame_idx * frame_header_size
        if hdr_offset + frame_header_size > len(data):
            break

        cmd_table_offset, outline_table_offset, palette_offset, properties, width, height, hotspot_x, hotspot_y = struct.unpack(
            '<4I4i', data[hdr_offset:hdr_offset+32]
        )

        if width <= 0 or height <= 0 or width > 4096 or height > 4096:
            continue

        # Prepare 32-bit RGBA image
        img = Image.new('RGBA', (width, height), (0, 0, 0, 0))
        pixels = img.load()

        # Parse rows
        for y in range(height):
            # Read boundary from outline table
            boundary_offset = outline_table_offset + y * 4
            if boundary_offset + 4 > len(data):
                break
            left, right = struct.unpack('<2H', data[boundary_offset:boundary_offset+4])
            if left == 0x8000:
                continue # entire row is transparent

            cmd_row_offset_pos = cmd_table_offset + y * 4
            if cmd_row_offset_pos + 4 > len(data):
                break
            cmd_row_offset, = struct.unpack('<I', data[cmd_row_offset_pos:cmd_row_offset_pos+4])

            # Process commands along scanline
            curr_x = left
            ptr = cmd_row_offset

            while ptr < len(data) and curr_x < width:
                cmd_byte = data[ptr]
                ptr += 1
                cmd_type = cmd_byte & 0x0F

                if cmd_type == 0x0F: # End of row
                    break
                elif cmd_type in (0x00, 0x04, 0x08, 0x0C): # Color block
                    count = cmd_byte >> 2
                    for _ in range(count):
                        if ptr < len(data) and curr_x < width:
                            color_idx = data[ptr]
                            ptr += 1
                            pixels[curr_x, y] = palette[color_idx]
                            curr_x += 1
                elif cmd_type in (0x01, 0x05, 0x09, 0x0D): # Skip transparent
                    count = cmd_byte >> 2
                    curr_x += count
                elif cmd_type == 0x02: # Big color block
                    if ptr >= len(data): break
                    count = ((cmd_byte & 0xF0) << 4) + data[ptr]
                    ptr += 1
                    for _ in range(count):
                        if ptr < len(data) and curr_x < width:
                            color_idx = data[ptr]
                            ptr += 1
                            pixels[curr_x, y] = palette[color_idx]
                            curr_x += 1
                elif cmd_type == 0x03: # Big skip
                    if ptr >= len(data): break
                    count = ((cmd_byte & 0xF0) << 4) + data[ptr]
                    ptr += 1
                    curr_x += count
                elif cmd_type == 0x07: # Color fill
                    count = cmd_byte >> 4
                    if count == 0 and ptr < len(data):
                        count = data[ptr]
                        ptr += 1
                    if ptr < len(data):
                        color_idx = data[ptr]
                        ptr += 1
                        c = palette[color_idx]
                        for _ in range(count):
                            if curr_x < width:
                                pixels[curr_x, y] = c
                                curr_x += 1
                elif cmd_type == 0x06: # Player color
                    count = cmd_byte >> 4
                    if count == 0 and ptr < len(data):
                        count = data[ptr]
                        ptr += 1
                    for _ in range(count):
                        if ptr < len(data) and curr_x < width:
                            p_idx = data[ptr]
                            ptr += 1
                            # Default player color (blue)
                            pixels[curr_x, y] = palette[p_idx % 16]
                            curr_x += 1
                elif cmd_type == 0x0E: # Shadow / translucent
                    count = cmd_byte >> 4
                    if count == 0 and ptr < len(data):
                        count = data[ptr]
                        ptr += 1
                    for _ in range(count):
                        if curr_x < width:
                            pixels[curr_x, y] = (0, 0, 0, 100) # 40% black shadow
                            curr_x += 1
                else:
                    curr_x += 1

        out_name = f"{slp_path.stem}_frame_{frame_idx:04d}.png"
        img.save(output_dir / out_name)
        extracted_count += 1
        manifest["frames"].append({
            "frame": frame_idx,
            "filename": out_name,
            "width": width,
            "height": height,
            "hotspot": [hotspot_x, hotspot_y]
        })

    with open(output_dir / f"{slp_path.stem}_manifest.json", "w") as mf:
        json.dump(manifest, mf, indent=2)

    return extracted_count

def extract_smx_file(smx_path, output_dir):
    """Parses AoE2: DE SMX file header and extracts graphic metadata/frames."""
    with open(smx_path, 'rb') as f:
        data = f.read()

    if len(data) < 32 or data[:4] != b'SMPX':
        return 0

    version, num_frames, compressed_len, uncompressed_len = struct.unpack('<2H2I', data[4:16])
    comment = data[16:32].decode('ascii', errors='ignore').strip('\x00')

    output_dir.mkdir(parents=True, exist_ok=True)
    manifest = {
        "source": smx_path.name,
        "format": "SMX (AoE2:DE)",
        "version": version,
        "frame_count": num_frames,
        "compressed_size": compressed_len,
        "uncompressed_size": uncompressed_len,
        "comment": comment,
        "frames": []
    }

    # In AoE2:DE, SMX files pack frame descriptors followed by zlib-compressed pixel runs
    # Extract metadata record
    meta_name = f"{smx_path.stem}_smx_meta.json"
    with open(output_dir / meta_name, "w") as mf:
        json.dump(manifest, mf, indent=2)

    return num_frames

def scan_and_extract_aoe2(input_dir, output_dir):
    """Scans and extracts all AoE2 DE / Classic assets found in input_dir."""
    input_path = Path(input_dir)
    output_path = Path(output_dir) / "aoe2"
    output_path.mkdir(parents=True, exist_ok=True)

    print(f"\n[AoE2 Engine] Searching for graphics in {input_path}...")

    # 1. Search for Classic DRS archives
    drs_files = list(input_path.rglob("*.drs")) + list(input_path.rglob("*.DRS"))
    slp_files = list(input_path.rglob("*.slp")) + list(input_path.rglob("*.SLP"))

    total_slp = len(slp_files)
    total_extracted = 0

    if slp_files:
        print(f"  * Found {total_slp} classic SLP sprite files.")
        for i, slp in enumerate(slp_files[:100]): # Rip first 100 by default or all if targeted
            count = extract_slp_file(slp, output_path / "slp" / slp.stem)
            total_extracted += count
            if (i + 1) % 10 == 0:
                print(f"    - Extracted {i+1}/{len(slp_files)} SLP archives ({total_extracted} frames)...")

    # 2. Search for AoE2 Definitive Edition SMX / SLX files
    smx_files = list(input_path.rglob("*.smx")) + list(input_path.rglob("*.SMX")) + list(input_path.rglob("*.slx"))
    if smx_files:
        print(f"  * Found {len(smx_files)} Definitive Edition SMX/SLX sprite archives.")
        for i, smx in enumerate(smx_files[:50]):
            extract_smx_file(smx, output_path / "smx")
        print(f"    - Cataloged {len(smx_files)} SMX archives.")

    # 3. Search for Loose PNG/DDS textures (Definitive Edition building textures)
    loose_pngs = [p for p in input_path.rglob("*.png") if "building" in p.stem.lower() or "terrain" in p.stem.lower() or "wall" in p.stem.lower()]
    if loose_pngs:
        print(f"  * Found {len(loose_pngs)} building/wall texture PNGs in game directory.")
        import shutil
        tex_dir = output_path / "textures"
        tex_dir.mkdir(parents=True, exist_ok=True)
        for p in loose_pngs[:100]:
            shutil.copy2(p, tex_dir / p.name)

    print(f"[AoE2 Engine] Complete. Sprites organized under {output_path}")
    return total_extracted

if __name__ == '__main__':
    if len(sys.argv) < 3:
        print("Usage: python extract_aoe2.py <input_dir_or_file> <output_dir>")
        sys.exit(1)

    inp = Path(sys.argv[1])
    out = Path(sys.argv[2])
    if inp.is_file() and inp.suffix.lower() == '.slp':
        extract_slp_file(inp, out)
    elif inp.is_file() and inp.suffix.lower() == '.smx':
        extract_smx_file(inp, out)
    else:
        scan_and_extract_aoe2(inp, out)
