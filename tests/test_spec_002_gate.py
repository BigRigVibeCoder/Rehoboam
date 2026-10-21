"""
tests/test_spec_002_gate.py — Verification gate test for SPEC-002 demography mortality self-test.
Governed by: GOV-002, GOV-003, SPEC-002
"""

import subprocess
import sys
from pathlib import Path


def test_spec_002_mortality_selftest() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    script = repo_root / "specs" / "demography" / "selftest_m2_f1_mortality.py"
    assert script.exists(), f"Missing spec script: {script}"
    res = subprocess.run([sys.executable, "-I", str(script), "--check"], capture_output=True, text=True)
    assert res.returncode == 0, f"Mortality selftest failed: {res.stderr}\n{res.stdout}"


if __name__ == "__main__":
    test_spec_002_mortality_selftest()
    print("test_spec_002_mortality_selftest: PASS")
