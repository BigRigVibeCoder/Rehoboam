"""infra/adjutant/idle-shutdown.sh: adjutant powers off only after 30 idle minutes (RUN-002 §3).

The script runs with stub pgrep, who, systemctl and logger on PATH, so the test sees what it would
do without touching the machine. Refs: SPR-009 T-04; RUN-002 §3.
"""

import os
import subprocess
import time
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "infra" / "adjutant" / "idle-shutdown.sh"


def run(tmp_path: Path, *, job: bool, login: bool, marker_age_min: int | None) -> bool:
    """Runs the script once; returns whether it asked systemctl to power off."""
    stubs = tmp_path / "bin"
    stubs.mkdir(exist_ok=True)
    calls = tmp_path / "calls"
    for name, body in {
        "pgrep": "exit 0" if job else "exit 1",
        "who": "echo 'director pts/0'" if login else "true",
        "systemctl": f'echo "$@" >> {calls}',
        "logger": "true",
    }.items():
        path = stubs / name
        path.write_text(f"#!/bin/sh\n{body}\n")
        path.chmod(0o755)
    marker = tmp_path / "last-busy"
    if marker_age_min is not None:
        marker.touch()
        then = time.time() - marker_age_min * 60
        os.utime(marker, (then, then))
    env = {"PATH": f"{stubs}:/usr/bin:/bin", "REHOBOAM_IDLE_MARKER": str(marker)}
    subprocess.run(["sh", str(SCRIPT)], env=env, check=True)
    return calls.exists() and "poweroff" in calls.read_text()


def test_idle_for_more_than_30_minutes_powers_off(tmp_path: Path) -> None:
    """No job, no login, last busy 31 minutes ago: power off."""
    assert run(tmp_path, job=False, login=False, marker_age_min=31)


def test_idle_for_less_than_30_minutes_waits(tmp_path: Path) -> None:
    """No job, no login, last busy 20 minutes ago: keep running."""
    assert not run(tmp_path, job=False, login=False, marker_age_min=20)


def test_a_job_or_a_login_keeps_it_on(tmp_path: Path) -> None:
    """A runner job or a login counts as busy, whatever the marker says, and renews it."""
    assert not run(tmp_path, job=True, login=False, marker_age_min=120)
    assert not run(tmp_path, job=False, login=True, marker_age_min=120)
    assert time.time() - (tmp_path / "last-busy").stat().st_mtime < 60


def test_a_fresh_boot_starts_the_clock(tmp_path: Path) -> None:
    """No marker yet (the VM just booted): create it, do not power off."""
    assert not run(tmp_path, job=False, login=False, marker_age_min=None)
    assert (tmp_path / "last-busy").exists()
