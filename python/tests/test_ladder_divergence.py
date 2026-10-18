"""Unit tests for bin/ladder_divergence.py: a C3 divergence moves only the parts it names.

Refs: SPR-014 T-08; REV-EVO-001 R1 and EVO-001 §5 as amended.
"""

from pathlib import Path

import pytest
from ladder_divergence import main

HEAD = "rehoboam-ladder v1\nworld solo\nseed 1\n"
RUNG = "rung {i} {t} 000{i}-12 {h:016x} 00000000000000bb\n"


def ladder(tmp: Path, name: str, meta: str, alive: str = "01") -> Path:
    """A two-rung ladder whose `rh-core/engine.meta` root is ``meta``."""
    text = HEAD
    for i in (1, 2):
        text += RUNG.format(i=i, t=i * 100, h=int(meta, 16) + i)
        text += f"root person.alive 00000000000000{alive}\nroot rh-core/engine.meta {meta}\n"
    path = tmp / name
    path.write_text(text, encoding="ascii")
    return path


def test_an_allowed_table_alone_passes(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """Only the cursor moved: PASS, with the rung hashes reported as folds."""
    old = ladder(tmp_path, "old", "00000000000000a0")
    new = ladder(tmp_path, "new", "00000000000000c0")
    assert main(["ladder", str(old), str(new), "--allow", "rh-core/engine.meta", "--record"]) == 0
    out = capsys.readouterr().out
    assert "table person.alive: equal at all 2 rungs" in out
    assert "table rh-core/engine.meta: differs at 2 of 2 rungs (allowed)" in out
    assert "rung 1 0001-12 rh-core/engine.meta 00000000000000a0 00000000000000c0 DIFF" in out


def test_a_world_table_that_moves_fails(tmp_path: Path) -> None:
    """A world table differs: FAIL, even with the cursor allowed."""
    old = ladder(tmp_path, "old", "00000000000000a0")
    new = ladder(tmp_path, "new", "00000000000000c0", alive="02")
    assert main(["ladder", str(old), str(new), "--allow", "rh-core/engine.meta"]) == 1


def test_a_header_change_fails(tmp_path: Path) -> None:
    """The history tuple and every header line must be equal."""
    old = ladder(tmp_path, "old", "00000000000000a0")
    new = tmp_path / "new"
    new.write_text(old.read_text().replace("seed 1", "seed 2"), encoding="ascii")
    assert main(["ladder", str(old), str(new)]) == 1


CANON = "#rehoboam/stream/canonical/v0 timeline=0 channel=A registry_version=0.2.0 format=x\n"


def canonical(tmp: Path, name: str, state: str, living: str = "1") -> Path:
    """A step digest and a keyframe carrying ``state`` as their state hash."""
    path = tmp / name
    path.write_text(
        CANON
        + f"STEP_DIGEST 0 0 216000 {living} 0 0 {state} 0000000000000002 0000000000000003\n"
        + f"KEYFRAME 0 1 216000 v1 keyframes/k {state} 0\n",
        encoding="ascii",
    )
    return path


ALLOW = ["--allow", "STEP_DIGEST.state_hash", "--allow", "KEYFRAME.state_hash"]


def test_state_hashes_alone_pass(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """Only the two state_hash fields differ: PASS."""
    old = canonical(tmp_path, "old", "000000000000000a")
    new = canonical(tmp_path, "new", "000000000000000c")
    assert main(["canonical", str(old), str(new), *ALLOW]) == 0
    assert "field KEYFRAME.state_hash: differs in 1 records (allowed)" in capsys.readouterr().out


def test_another_field_fails(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """A step digest's living count differs: FAIL, naming the field."""
    old = canonical(tmp_path, "old", "000000000000000a")
    new = canonical(tmp_path, "new", "000000000000000c", living="2")
    assert main(["canonical", str(old), str(new), *ALLOW]) == 1
    assert "DIVERGENCE: STEP_DIGEST.living differs" in capsys.readouterr().out


def test_a_missing_file_is_exit_2(tmp_path: Path) -> None:
    """An unreadable file has no verdict."""
    assert main(["canonical", str(tmp_path / "a"), str(tmp_path / "b")]) == 2
