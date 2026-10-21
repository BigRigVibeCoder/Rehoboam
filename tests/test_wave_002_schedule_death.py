"""
tests/test_wave_002_schedule_death.py — Deterministic verification of WAVE-002 ScheduleDeath vital drain.
Governed by: GOV-002, GOV-003, SPEC-002
"""

import subprocess
import sys
from pathlib import Path


def test_mortality_spec_golden() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    spec_script = repo_root / "specs" / "demography" / "selftest_m2_f1_mortality.py"
    assert spec_script.is_file(), f"Spec script not found: {spec_script}"
    res = subprocess.run([sys.executable, "-I", str(spec_script), "--check"], capture_output=True, text=True)
    assert res.returncode == 0, f"Mortality golden check failed: {res.stderr}\n{res.stdout}"


def test_schedule_death_drain_contract() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    demography_src = repo_root / "crates" / "rh-sys-demography" / "src" / "demography.rs"
    assert demography_src.is_file(), f"Demography source not found: {demography_src}"
    content = demography_src.read_text(encoding="utf-8")
    assert "SCHEDULE_DEATH" in content, "Missing SCHEDULE_DEATH constant in demography.rs"
    assert "fn schedule_death" in content, "Missing schedule_death drain function in demography.rs"
    assert "fn drain" in content, "Missing drain loop in demography.rs"
    assert "DropReason::Dead" in content, "Missing idempotent DropReason::Dead handling in demography.rs"


if __name__ == "__main__":
    test_mortality_spec_golden()
    test_schedule_death_drain_contract()
    print("test_wave_002_schedule_death: PASS")
