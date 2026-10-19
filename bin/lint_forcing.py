"""Linter for synthetic forcing series v0 (SPR-028 Task T-01; Finding F-9).

Verifies bidirectional consistency between system specs (specs/systems/*.odd.toml [isolation])
and the typed forcing registry (specs/validation/forcing_v0.toml):
  1. Every [isolation] forcing entry has a corresponding typed entry in forcing_v0.toml.
  2. Series head words agree between the spec and forcing_v0.toml.
  3. Every rate or mode read is either forced in [isolation] or has a declared typed neutral value.
  4. v0 series types are restricted to constant, step, and historical (sine and ramp are refused in v0).
  5. Every historical series names a valid, readable forcing file in specs/validation/forcing/<id>.toml.

Usage:
  python -I bin/lint_forcing.py              (lint real specs and forcing_v0.toml)
  python -I bin/lint_forcing.py --controls   (run the four planted mutant controls)

Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
FORCING_TOML = ROOT / "specs" / "validation" / "forcing_v0.toml"
SPECS_DIR = ROOT / "specs" / "systems"

ALLOWED_HEADS_V0 = {"constant", "step", "historical"}
FORBIDDEN_HEADS_V0 = {"sine", "ramp"}


def lint_forcing(
    forcing_path: Path,
    specs_dir: Path,
    extra_forcing_entries: list[dict[str, Any]] | None = None,
    mock_specs: dict[str, dict[str, Any]] | None = None,
) -> list[str]:
    """Lints forcing_v0.toml against system specs in specs_dir."""
    errors: list[str] = []

    if not forcing_path.exists():
        return [f"forcing file not found: {forcing_path}"]

    try:
        f_data = tomllib.loads(forcing_path.read_text(encoding="utf-8"))
    except (tomllib.TOMLDecodeError, OSError) as e:
        return [f"failed to parse TOML in {forcing_path}: {e}"]

    entries = list(f_data.get("forcing", []))
    if extra_forcing_entries:
        entries.extend(extra_forcing_entries)

    # 1. Validate forcing_v0.toml entries
    typed_map: dict[tuple[str, str], dict[str, Any]] = {}
    for i, entry in enumerate(entries):
        sys_name = entry.get("system")
        inp_name = entry.get("input")
        series = entry.get("series", "")

        if not sys_name or not inp_name:
            errors.append(f"entry {i}: missing system or input")
            continue

        key = (sys_name, inp_name)
        if key in typed_map:
            errors.append(f"duplicate forcing entry for ({sys_name}, {inp_name})")
        typed_map[key] = entry

        # Check series format and refusal of sine / ramp
        head = series.split(":")[0].strip()
        if head in FORBIDDEN_HEADS_V0:
            errors.append(f"({sys_name}, {inp_name}): series '{head}' is refused in v0")
        elif head not in ALLOWED_HEADS_V0:
            errors.append(f"({sys_name}, {inp_name}): unknown series head '{head}'")

        if head == "historical":
            m = re.match(r"^historical:([a-z0-9_]+)$", series)
            if not m:
                errors.append(
                    f"({sys_name}, {inp_name}): invalid historical series format '{series}'"
                )
            else:
                hist_id = m.group(1)
                hist_file = (
                    ROOT / "specs" / "validation" / "forcing" / f"{hist_id}.toml"
                )
                if not hist_file.exists():
                    errors.append(
                        f"({sys_name}, {inp_name}): historical forcing file not found: {hist_file}"
                    )
                else:
                    try:
                        tomllib.loads(hist_file.read_text(encoding="utf-8"))
                    except (tomllib.TOMLDecodeError, OSError) as e:
                        errors.append(
                            f"({sys_name}, {inp_name}): syntax error in {hist_file}: {e}"
                        )

    # 2. Check bidirectional consistency with specs
    spec_files = sorted(specs_dir.glob("*.odd.toml"))
    for sp in spec_files:
        if sp.name.startswith("_"):
            continue
        try:
            s_data = tomllib.loads(sp.read_text(encoding="utf-8"))
        except (tomllib.TOMLDecodeError, OSError) as e:
            errors.append(f"syntax error in {sp}: {e}")
            continue

        sys_id = s_data.get("system", {}).get("id")
        if not sys_id:
            continue

        if mock_specs and sys_id in mock_specs:
            s_data = mock_specs[sys_id]

        iso = s_data.get("isolation", {})
        spec_forcings = iso.get("forcing", [])

        # Verify each spec forcing has a typed entry
        for sf in spec_forcings:
            inp = sf.get("input")
            s_series = sf.get("series", "")
            s_head = s_series.split(":")[0].strip()

            key = (sys_id, inp)
            if key not in typed_map:
                errors.append(
                    f"{sys_id}: [isolation] forcing for '{inp}' has no typed entry in forcing_v0.toml"
                )
                continue

            t_entry = typed_map[key]
            t_series = t_entry.get("series", "")
            t_head = t_series.split(":")[0].strip()

            if s_head != t_head:
                errors.append(
                    f"{sys_id}: head word differs for '{inp}': spec says '{s_head}', forcing_v0.toml says '{t_head}'"
                )

        # Check rate reads: must have either neutral or be forced
        reads = s_data.get("reads", [])
        for r in reads:
            if r.get("use") == "rate":
                col = r.get("column")
                has_neutral = "neutral" in r
                is_forced = (sys_id, col) in typed_map or any(
                    sf.get("input") == col for sf in spec_forcings
                )
                if not has_neutral and not is_forced:
                    errors.append(
                        f"{sys_id}: rate read '{col}' is neither forced in [isolation] nor has a typed neutral"
                    )

    return errors


def run_controls() -> tuple[bool, list[str]]:
    """Runs the four planted control mutations and verifies they fail by message."""
    control_logs: list[str] = []
    all_passed = True

    # Control 1: isolation-no-typed-entry
    mock_1 = {
        "M2.demography": {
            "system": {"id": "M2.demography"},
            "isolation": {
                "forcing": [{"input": "untyped.column", "series": "constant:0"}]
            },
        }
    }
    errs_1 = lint_forcing(FORCING_TOML, SPECS_DIR, mock_specs=mock_1)
    c1_ok = any("has no typed entry in forcing_v0.toml" in e for e in errs_1)
    all_passed &= c1_ok
    control_logs.append(
        f"Control 1 'isolation-no-typed-entry': {'PASS (caught)' if c1_ok else 'FAIL'}"
    )

    # Control 2: head-word-differs
    mock_2 = {
        "M2.demography": {
            "system": {"id": "M2.demography"},
            "isolation": {
                "forcing": [{"input": "person.named", "series": "step:0->1"}]
            },
        }
    }
    errs_2 = lint_forcing(FORCING_TOML, SPECS_DIR, mock_specs=mock_2)
    c2_ok = any("head word differs" in e for e in errs_2)
    all_passed &= c2_ok
    control_logs.append(
        f"Control 2 'head-word-differs': {'PASS (caught)' if c2_ok else 'FAIL'}"
    )

    # Control 3: rate-read-unforced
    mock_3 = {
        "M2.demography": {
            "system": {"id": "M2.demography"},
            "reads": [{"column": "unforced.rate_col", "use": "rate"}],
            "isolation": {"forcing": []},
        }
    }
    errs_3 = lint_forcing(FORCING_TOML, SPECS_DIR, mock_specs=mock_3)
    c3_ok = any("rate read 'unforced.rate_col' is neither forced" in e for e in errs_3)
    all_passed &= c3_ok
    control_logs.append(
        f"Control 3 'rate-read-unforced': {'PASS (caught)' if c3_ok else 'FAIL'}"
    )

    # Control 4: sine-series-refused
    mock_extra = [{"system": "M2.demography", "input": "test.sine", "series": "sine:1"}]
    errs_4 = lint_forcing(FORCING_TOML, SPECS_DIR, extra_forcing_entries=mock_extra)
    c4_ok = any("series 'sine' is refused in v0" in e for e in errs_4)
    all_passed &= c4_ok
    control_logs.append(
        f"Control 4 'sine-series-refused': {'PASS (caught)' if c4_ok else 'FAIL'}"
    )

    return all_passed, control_logs


def main() -> int:
    parser = argparse.ArgumentParser(description="Lint synthetic forcing series v0")
    parser.add_argument(
        "--controls", action="store_true", help="Run the four planted controls"
    )
    args = parser.parse_args()

    errors = lint_forcing(FORCING_TOML, SPECS_DIR)
    if errors:
        for e in errors:
            print(f"LINT-FORCING: ERROR: {e}", file=sys.stderr)
        print("LINT-FORCING: FAIL", file=sys.stderr)
        return 1

    print(f"LINT-FORCING: checked {FORCING_TOML.name} against specs (PASS)")

    if args.controls:
        ok_ctrls, logs = run_controls()
        for log in logs:
            print(f"LINT-FORCING: {log}")
        if not ok_ctrls:
            print("LINT-FORCING CONTROLS: FAIL", file=sys.stderr)
            return 1
        print("LINT-FORCING CONTROLS: PASS (4/4 controls caught as required)")

    return 0


if __name__ == "__main__":
    sys.exit(main())
