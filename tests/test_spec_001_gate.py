"""
tests/test_spec_001_gate.py — Verification gate test for SPEC-001 demography self-test.
Governed by: GOV-002, GOV-003, SPEC-001
"""

import subprocess
import sys
from pathlib import Path


def test_spec_001_demography_selftest() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    script = repo_root / "specs" / "demography" / "selftest_m2_f1.py"
    assert script.exists(), f"Missing spec script: {script}"
    res = subprocess.run([sys.executable, "-I", str(script)], capture_output=True, text=True)
    assert res.returncode == 0, f"Demography selftest failed: {res.stderr}\n{res.stdout}"


if __name__ == "__main__":
    test_spec_001_demography_selftest()
    print("test_spec_001_demography_selftest: PASS")
