"""
tests/test_wave_002_dynastic_succession.py - Verification of SuccessionF1 dynastic arbitration.
Governed by: GOV-002, GOV-003, SPEC-002
"""

from pathlib import Path


def test_dynastic_succession_contract() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    dynast_src = repo_root / "crates" / "rh-sys-dynast" / "src" / "succession.rs"
    assert dynast_src.is_file(), f"Dynast succession source not found: {dynast_src}"
    content = dynast_src.read_text(encoding="utf-8")
    assert "pub trait SuccessionF1" in content, "Missing SuccessionF1 trait in succession.rs"
    assert "arbitrate_ruler_death" in content, "Missing arbitrate_ruler_death method in succession.rs"
    assert "SuccessorFound" in content, "Missing SuccessorFound variant in succession.rs"
    assert "Interregnum" in content, "Missing Interregnum variant in succession.rs"


if __name__ == "__main__":
    test_dynastic_succession_contract()
    print("test_wave_002_dynastic_succession: PASS")
