#!/usr/bin/env python3
"""Unified Commercial Game Asset Ripper for Rehoboam

Automatically detects and extracts retail game files from:
1. Stronghold Crusader HD / Stronghold HD (.gm1 / .tgx)
2. Caesar III (.sg2 / .555)
3. SimCity 2000 (.DAT / .BMP)

Usage:
    python tools/asset_ripper/extract_all.py --input /path/to/game/folder --output assets/sprites
"""

import sys
import argparse
from pathlib import Path

from extract_stronghold import scan_and_extract_stronghold
from extract_caesar3 import scan_and_extract_c3
from extract_sc2k import extract_sc2k_archive, parse_bmp_palette
from extract_aoe2 import scan_and_extract_aoe2

def auto_rip_directory(input_dir, output_dir):
    input_path = Path(input_dir)
    output_path = Path(output_dir)

    if not input_path.exists():
        print(f"Error: Input path {input_path} does not exist.")
        return

    print("=" * 70)
    print("REHOBOAM // UNIFIED COMMERCIAL ASSET RIPPER")
    print(f"Scanning target directory: {input_path}")
    print(f"Output directory:         {output_path}")
    print("=" * 70)

    # 1. Check for Stronghold Crusader
    gm1_files = list(input_path.rglob("*.gm1")) + list(input_path.rglob("*.GM1"))
    if gm1_files:
        print(f"\n[+] DETECTED: Stronghold Crusader / Stronghold ({len(gm1_files)} GM1 archives)")
        scan_and_extract_stronghold(input_path, output_path)

    # 2. Check for Caesar III
    sg2_files = list(input_path.rglob("*.sg2")) + list(input_path.rglob("*.SG2"))
    if sg2_files:
        print(f"\n[+] DETECTED: Caesar III ({len(sg2_files)} SG2 archives)")
        scan_and_extract_c3(input_path, output_path)

    # 3. Check for SimCity 2000
    dat_files = [f for f in input_path.rglob("*.dat") if "sc2" in f.name.lower() or "sprite" in f.name.lower() or "city" in f.name.lower()]
    dat_files += [f for f in input_path.rglob("*.DAT") if "sc2" in f.name.lower() or "sprite" in f.name.lower() or "city" in f.name.lower()]
    if dat_files:
        print(f"\n[+] DETECTED: SimCity 2000 ({len(dat_files)} DAT archives)")
        # Look for palette BMP
        pal_files = list(input_path.rglob("*.bmp")) + list(input_path.rglob("*.BMP"))
        palette = None
        for p in pal_files:
            if "pal" in p.name.lower() or "terrain" in p.name.lower():
                try:
                    palette = parse_bmp_palette(p)
                    print(f"    * Using palette from {p.name}")
                    break
                except Exception:
                    pass
        for df in dat_files:
            extract_sc2k_archive(df, output_path / "sc2k" / df.stem, palette)

    # 4. Check for Age of Empires II (Classic or DE)
    aoe2_files = list(input_path.rglob("*.smx")) + list(input_path.rglob("*.slp")) + list(input_path.rglob("*.drs"))
    if aoe2_files or (input_path / "resources" / "_common").exists():
        print(f"\n[+] DETECTED: Age of Empires II ({len(aoe2_files)} sprite/archive files)")
        scan_and_extract_aoe2(input_path, output_path)

    if not gm1_files and not sg2_files and not dat_files and not aoe2_files:
        print("[!] No recognized archives found in target directory.")
        print("    Looking for: .gm1 (Stronghold), .sg2 (Caesar III), .dat (SimCity 2000), .smx/.slp/.drs (AoE2)")

    print("\n" + "=" * 70)
    print("Extraction scan complete.")
    print("=" * 70)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description="Rehoboam Commercial Asset Extraction Utility")
    parser.add_argument("--input", "-i", required=True, help="Path to commercial game installation or mounted disk")
    parser.add_argument("--output", "-o", default="assets/sprites", help="Destination folder for transparent PNGs")
    args = parser.parse_args()

    auto_rip_directory(args.input, args.output)
