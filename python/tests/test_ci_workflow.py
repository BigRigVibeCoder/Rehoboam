"""ci.yml runs every part of bin/check_all.sh (VER-SPR-009-TEST GAP-19; BCK-002 A-41).

The PR tier is two CI jobs, one per part of the chain. Nothing else ties the workflow to the
parts: with the matrix cut to `[core]`, or both jobs running `--part core`, the golden part (the
golden runs, the perf gates, the vitals, the allocation budget) would leave CI with every test
green. This test reads `.github/workflows/ci.yml` and the parts `bin/check_all.sh` declares, and
proves the `check-all` job's matrix is exactly those parts and that its step passes the matrix
value to `--part`. The controls plant both mistakes.

Refs: VER-SPR-009-TEST GAP-19; SPR-009 review 2b.
"""

import copy
import re
from pathlib import Path
from typing import Any

import yaml  # type: ignore[import-untyped]  # PyYAML ships no stubs; adding types-PyYAML would change the lock

REPO = Path(__file__).resolve().parents[2]
WORKFLOW = REPO / ".github" / "workflows" / "ci.yml"
SCRIPT = REPO / "bin" / "check_all.sh"
PART_MARKER = re.compile(r"^part (\w+) &&$")
# The step's command: the matrix value, directly or through one of the step's env variables.
RUN = re.compile(r'^bin/check_all\.sh --part (?:"?\$\{\{ matrix\.part \}\}"?|"\$(?P<var>\w+)")$')
MATRIX_PART = "${{ matrix.part }}"


def declared_parts() -> list[str]:
    """The parts bin/check_all.sh's chain declares, in order."""
    lines = SCRIPT.read_text(encoding="utf-8").splitlines()
    return [m.group(1) for line in lines if (m := PART_MARKER.match(line.strip()))]


def workflow() -> dict[str, Any]:
    """ci.yml, parsed."""
    loaded = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    assert isinstance(loaded, dict)
    return loaded


def problems(flow: dict[str, Any], parts: list[str]) -> list[str]:
    """Why the check-all job does not run every part exactly once; empty if it does."""
    job = flow.get("jobs", {}).get("check-all")
    if not isinstance(job, dict):
        return ["no check-all job"]
    found = []
    matrix = job.get("strategy", {}).get("matrix", {}).get("part")
    if (
        not isinstance(matrix, list)
        or sorted(matrix) != sorted(parts)
        or len(set(matrix)) != len(matrix)
    ):
        found.append(f"the matrix's parts {matrix!r} are not exactly check_all's {parts!r}")
    steps = [s for s in job.get("steps", []) if "bin/check_all.sh" in str(s.get("run", ""))]
    if len(steps) != 1:
        return [*found, f"{len(steps)} steps run bin/check_all.sh, not one"]
    command = str(steps[0]["run"]).strip()
    m = RUN.match(command)
    if m is None:
        found.append(f"the step runs {command!r}, not bin/check_all.sh --part {MATRIX_PART}")
    elif m.group("var") and steps[0].get("env", {}).get(m.group("var")) != MATRIX_PART:
        found.append(f"the step's ${m.group('var')} is not {MATRIX_PART}")
    return found


def test_check_all_declares_two_parts() -> None:
    """The chain's parts, as test_check_all_inventory.py pins them."""
    assert declared_parts() == ["core", "golden"]


def test_the_workflow_runs_every_part_once() -> None:
    """GAP-19: the matrix is exactly check_all's parts, and each job runs its own part."""
    assert problems(workflow(), declared_parts()) == []


def test_a_matrix_without_the_golden_part_fails() -> None:
    """Control: `part: [core]` would drop the golden part from CI."""
    flow = copy.deepcopy(workflow())
    flow["jobs"]["check-all"]["strategy"]["matrix"]["part"] = ["core"]
    assert problems(flow, declared_parts()) != []
    flow["jobs"]["check-all"]["strategy"]["matrix"]["part"] = ["core", "core"]
    assert problems(flow, declared_parts()) != []


def test_a_hard_coded_part_fails() -> None:
    """Control: both jobs running `--part core`, directly or through the step's env, fails."""
    flow = copy.deepcopy(workflow())
    step = next(
        s for s in flow["jobs"]["check-all"]["steps"] if "bin/check_all.sh" in str(s.get("run", ""))
    )
    step["run"] = "bin/check_all.sh --part core"
    assert problems(flow, declared_parts()) != []
    step["run"] = 'bin/check_all.sh --part "$CHECK_PART"'
    step["env"] = {**step.get("env", {}), "CHECK_PART": "core"}
    assert problems(flow, declared_parts()) == ["the step's $CHECK_PART is not ${{ matrix.part }}"]
    step["run"] = "bin/check_all.sh"
    assert problems(flow, declared_parts()) != []


def test_the_pr_tier_times_out_at_20_minutes() -> None:
    """BCK-002 A-38: each PR-tier job's timeout guards the 15-minute budget (V-PERF-04)."""
    assert workflow()["jobs"]["check-all"]["timeout-minutes"] == 20
