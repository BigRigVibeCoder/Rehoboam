"""The Registry's keyed tables in the Rust loader's order equal tomllib's (SPR-017 T-06).

An enum's ordinal is its declaration index (effect_language_v0.md §3.1), and `toml::Table` sorts its
keys, so the Rust loader takes a keyed table's ids from the source text
(`rh_manifest::effect_registry::key_order`). This compares `rh-cli effect keys` with the order
Python's tomllib keeps, for every keyed top-level table of the Registry (the Architect's keyed-table
ruling). Stdlib only.

Usage: python -I bin/check_key_order.py --rust-bin RH_CLI [--registry R] [--rust-output FILE]
  --rust-output reads the Rust lines from FILE instead of running RH_CLI (the planted control).
Exit 0 when every table agrees, 1 otherwise, 2 on bad arguments.
"""

import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "specs" / "manifest" / "registry_v0.toml"


def python_order(text: str) -> dict[str, list[str]]:
    """Every top-level table's keys in tomllib's (source) order."""
    data = tomllib.loads(text)
    return {name: list(value) for name, value in data.items() if isinstance(value, dict)}


def rust_order(lines: str) -> dict[str, list[str]]:
    """``section: id id ...`` lines as a mapping."""
    out = {}
    for line in lines.splitlines():
        section, _, ids = line.partition(":")
        out[section.strip()] = ids.split()
    return out


def main(argv: list[str]) -> int:
    """Compare the Rust order with tomllib's; the exit code of the module docstring."""
    args = dict(zip(argv[::2], argv[1::2], strict=False))
    if len(argv) % 2 or set(args) - {"--rust-bin", "--registry", "--rust-output"}:
        print(__doc__)
        return 2
    registry = Path(args.get("--registry", REGISTRY))
    if "--rust-output" in args:
        rust = Path(args["--rust-output"]).read_text(encoding="utf-8")
    elif "--rust-bin" in args:
        done = subprocess.run([args["--rust-bin"], "effect", "keys", "--registry", str(registry)],
                              capture_output=True, text=True, check=False)
        if done.returncode != 0:
            print(f"FAIL: rh-cli effect keys exited {done.returncode}: {done.stderr.strip()}")
            return 1
        rust = done.stdout
    else:
        print(__doc__)
        return 2
    want = python_order(registry.read_text(encoding="utf-8"))
    have = rust_order(rust)
    bad = [s for s in sorted(set(want) | set(have)) if want.get(s) != have.get(s)]
    for s in bad:
        print(f"MISMATCH [{s}]: tomllib {want.get(s)}, rust {have.get(s)}")
    if bad:
        print(f"FAIL: {len(bad)} of {len(want)} keyed tables differ")
        return 1
    keyed = sum(1 for ids in want.values() if ids != sorted(ids))
    print(f"KEY ORDER: PASS ({len(want)} tables, {keyed} not in sorted order)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
