"""viewer/tools/lock_age.py: lock-age table and two-week rule check for viewer/ workspace.

Checked offline by check_viewer.sh:
- The table lists exactly the packages in viewer/Cargo.lock.
- Every direct pin was at least 14 days old when it entered the lock.
- Transitive packages are recorded with their publication dates, never gated.

Usage:
  python3 -I viewer/tools/lock_age.py --check
  python3 -I viewer/tools/lock_age.py --write
"""

from __future__ import annotations

import argparse
import datetime
import json
import sys
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
VIEWER_DIR = HERE.parent
LOCK_FILE = VIEWER_DIR / "Cargo.lock"
TABLE_FILE = VIEWER_DIR / "lock_age.txt"
MIN_AGE_DAYS = 14
MIN_PARTS = 5


def get_direct_pins() -> set[str]:
    """Find all direct dependency names declared in viewer crates."""
    direct: set[str] = set()
    for manifest_path in VIEWER_DIR.rglob("Cargo.toml"):
        if "target" in manifest_path.parts:
            continue
        try:
            data = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        except Exception:
            continue
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            table = data.get(section, {})
            if isinstance(table, dict):
                for name, spec in table.items():
                    # Ignore workspace internal path dependencies
                    if isinstance(spec, dict) and "path" in spec:
                        continue
                    direct.add(name)
    return direct


def get_lock_packages() -> dict[str, str]:
    """Return {package_name: version} for all registry packages in viewer/Cargo.lock."""
    if not LOCK_FILE.exists():
        raise FileNotFoundError(f"Missing {LOCK_FILE}")
    data = tomllib.loads(LOCK_FILE.read_text(encoding="utf-8"))
    pkgs: dict[str, str] = {}
    for p in data.get("package", []):
        if "source" in p:
            pkgs[p["name"]] = p["version"]
    return pkgs


def parse_table() -> dict[str, dict[str, str]]:
    """Parse viewer/lock_age.txt into {name: {version, published, entered, direct}}."""
    if not TABLE_FILE.exists():
        return {}
    res: dict[str, dict[str, str]] = {}
    for raw_line in TABLE_FILE.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) >= MIN_PARTS:
            name, ver, pub, ent, direct = parts[0], parts[1], parts[2], parts[3], parts[4]
            res[name] = {
                "version": ver,
                "published": pub,
                "entered": ent,
                "direct": direct,
            }
    return res


def fetch_published_date(name: str, version: str) -> str:
    """Fetch published date YYYY-MM-DD from crates.io API."""
    url = f"https://crates.io/api/v1/crates/{name}/{version}"
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "Rehoboam-viewer-lock-age/1.0 (bdavidriggins@gmail.com)"},
    )
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read().decode("utf-8"))
        created_at = data["version"]["created_at"]
        return str(created_at[:10])


def _verify_package_age(name: str, ver: str, entry: dict[str, str], is_direct: bool) -> list[str]:
    """Verify lock version, direct classification, and the 14-day rule for one package."""
    errors: list[str] = []
    if entry["version"] != ver:
        errors.append(f"{name}: lock version {ver} != table version {entry['version']}")
        return errors

    expected_direct_str = "direct" if is_direct else "transitive"
    if entry["direct"] != expected_direct_str:
        errors.append(f"{name}: table direct={entry['direct']} but expected {expected_direct_str}")

    if is_direct:
        try:
            pub_date = datetime.date.fromisoformat(entry["published"])
            ent_date = datetime.date.fromisoformat(entry["entered"])
        except ValueError as e:
            errors.append(f"{name}: invalid date format: {e}")
            return errors

        age_days = (ent_date - pub_date).days
        if age_days < MIN_AGE_DAYS:
            errors.append(
                f"{name} {ver}: direct pin entered on {ent_date} but published on {pub_date} "
                f"({age_days} days old, must be >= {MIN_AGE_DAYS} days)"
            )
    return errors


def check() -> int:
    """Offline check of lock_age.txt against Cargo.lock and the 14-day rule."""
    if not TABLE_FILE.exists():
        print(f"FAIL: {TABLE_FILE} does not exist", file=sys.stderr)
        return 1

    lock_pkgs = get_lock_packages()
    direct_pins = get_direct_pins()
    table = parse_table()

    # 1. Exactly the lock's packages
    missing_in_table = set(lock_pkgs.keys()) - set(table.keys())
    if missing_in_table:
        print(
            f"FAIL: packages in Cargo.lock missing from {TABLE_FILE.name}: "
            f"{sorted(missing_in_table)}",
            file=sys.stderr,
        )
        return 1

    extra_in_table = set(table.keys()) - set(lock_pkgs.keys())
    if extra_in_table:
        print(
            f"FAIL: packages in {TABLE_FILE.name} not in Cargo.lock: {sorted(extra_in_table)}",
            file=sys.stderr,
        )
        return 1

    # Check versions and 14-day rule
    errors: list[str] = []
    for name, ver in sorted(lock_pkgs.items()):
        errors.extend(_verify_package_age(name, ver, table[name], name in direct_pins))

    if errors:
        for err in errors:
            print(f"FAIL: {err}", file=sys.stderr)
        return 1

    print(
        f"PASS: {len(lock_pkgs)} packages checked, "
        f"all direct pins >= {MIN_AGE_DAYS} days old at lock entry"
    )
    return 0


def write() -> int:
    """Generate or update viewer/lock_age.txt."""
    lock_pkgs = get_lock_packages()
    direct_pins = get_direct_pins()
    existing = parse_table()
    today = datetime.datetime.now(datetime.UTC).date().isoformat()

    rows: list[str] = []
    rows.append(
        "# viewer/lock_age.txt: publication and lock-entry dates for packages in viewer/Cargo.lock."
    )
    rows.append(
        "# Generated by viewer/tools/lock_age.py --write; checked offline by lock_age.py --check."
    )
    rows.append("# Columns: package version published entered direct")

    for name in sorted(lock_pkgs.keys()):
        ver = lock_pkgs[name]
        is_direct = name in direct_pins
        direct_str = "direct" if is_direct else "transitive"

        if name in existing and existing[name]["version"] == ver:
            pub = existing[name]["published"]
            ent = existing[name]["entered"]
        else:
            print(f"Querying crates.io for {name} {ver}...")
            try:
                pub = fetch_published_date(name, ver)
            except Exception as e:
                print(
                    f"Error fetching crates.io data for {name} {ver}: {e}",
                    file=sys.stderr,
                )
                return 1
            ent = today

        rows.append(f"{name:<25} {ver:<10} {pub:<12} {ent:<12} {direct_str}")

    content = "\n".join(rows) + "\n"
    TABLE_FILE.write_text(content, encoding="utf-8")
    print(f"Wrote {TABLE_FILE} with {len(lock_pkgs)} packages")
    return 0


def main(argv: list[str] | None = None) -> int:
    """Run lock_age tool with command line arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true", help="Run offline check")
    group.add_argument("--write", action="store_true", help="Update lock_age.txt")
    args = parser.parse_args(argv)

    if args.write:
        return write()
    elif args.check:
        return check()
    return 0


if __name__ == "__main__":
    sys.exit(main())
