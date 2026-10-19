"""Tests for .github/workflows/viewer.yml (SPR-033 T-06; B1).

Verifies:
1. Workflow parses as valid YAML.
2. Triggers on pull_request, schedule, and workflow_dispatch.
3. No push trigger on main (B1; V4).
4. Pull request paths filter covers viewer/**, specs/viewer/**, bin/check_viewer.sh,
   .github/workflows/viewer.yml, and python/tests/test_viewer_workflow.py.
5. Schedule runs twice a week at 04:43 UTC ("43 4 * * 1,4").
6. The job executes bin/check_viewer.sh.
"""

from pathlib import Path
from typing import Any

import yaml  # type: ignore[import-untyped]

REPO = Path(__file__).resolve().parents[2]
WORKFLOW_FILE = REPO / ".github" / "workflows" / "viewer.yml"

REQUIRED_PATHS = {
    "viewer/**",
    "specs/viewer/**",
    "bin/check_viewer.sh",
    ".github/workflows/viewer.yml",
    "python/tests/test_viewer_workflow.py",
}


MIN_CRON_FIELDS = 2
EXPECTED_TIMEOUT_MINUTES = 30


def load_workflow() -> dict[str, Any]:
    """Loads and returns the parsed viewer.yml workflow document."""
    assert WORKFLOW_FILE.is_file(), f"workflow not found at {WORKFLOW_FILE}"
    loaded = yaml.safe_load(WORKFLOW_FILE.read_text(encoding="utf-8"))
    assert isinstance(loaded, dict)
    if True in loaded and "on" not in loaded:
        loaded["on"] = loaded.pop(True)
    return loaded


def test_viewer_workflow_triggers() -> None:
    """Verifies pull_request, schedule, and workflow_dispatch without push trigger."""
    flow = load_workflow()
    triggers = flow.get("on", {})
    assert "push" not in triggers, "viewer workflow must have NO push trigger (B1; V4)"
    assert "pull_request" in triggers, "viewer workflow must trigger on pull_request"
    assert "schedule" in triggers, "viewer workflow must have schedule trigger"
    assert "workflow_dispatch" in triggers, "viewer workflow must have workflow_dispatch trigger"


def test_pull_request_paths() -> None:
    """Verifies that pull_request paths filter includes all required viewer paths."""
    flow = load_workflow()
    pr = flow["on"]["pull_request"]
    paths = set(pr.get("paths", []))
    missing = REQUIRED_PATHS - paths
    assert not missing, f"pull_request trigger missing required paths: {missing}"


def test_schedule_cron() -> None:
    """Verifies schedule runs twice a week at 04:43 UTC ('43 4 * * 1,4') and is unique."""
    flow = load_workflow()
    schedules = flow["on"]["schedule"]
    assert isinstance(schedules, list)
    crons = [s.get("cron") for s in schedules if isinstance(s, dict)]
    assert "43 4 * * 1,4" in crons, f"schedule must run '43 4 * * 1,4', found: {crons}"

    # Verify no other workflow shares this minute and hour (04:43)
    for wf_path in (REPO / ".github" / "workflows").glob("*.yml"):
        if wf_path == WORKFLOW_FILE:
            continue
        try:
            doc = yaml.safe_load(wf_path.read_text(encoding="utf-8"))
        except Exception:
            continue
        if not isinstance(doc, dict):
            continue
        on_field = doc.get("on") or doc.get(True)
        if isinstance(on_field, dict) and "schedule" in on_field:
            for other_sched in on_field["schedule"]:
                if isinstance(other_sched, dict) and "cron" in other_sched:
                    c = other_sched["cron"].split()
                    assert not (len(c) >= MIN_CRON_FIELDS and c[0] == "43" and c[1] == "4"), (
                        f"{wf_path.name} shares minute 43 and hour 4 with viewer.yml"
                    )


def test_job_timeout_and_runs_check_viewer() -> None:
    """Verifies that the check-viewer job has 30m timeout and runs bin/check_viewer.sh."""
    flow = load_workflow()
    jobs = flow.get("jobs", {})
    assert "check-viewer" in jobs, "missing check-viewer job"
    job = jobs["check-viewer"]
    assert job.get("timeout-minutes") == EXPECTED_TIMEOUT_MINUTES, (
        f"viewer workflow job timeout must be {EXPECTED_TIMEOUT_MINUTES} minutes"
    )
    steps = job.get("steps", [])
    run_commands = [s.get("run") for s in steps if isinstance(s, dict) and "run" in s]
    assert any("bin/check_viewer.sh" in cmd for cmd in run_commands if cmd), (
        "job must run bin/check_viewer.sh"
    )


def test_rust_cache_configuration() -> None:
    """Verifies rust-cache configuration: workspaces, save-if on main, no key, no hashFiles."""
    flow = load_workflow()
    job = flow.get("jobs", {}).get("check-viewer", {})
    steps = job.get("steps", [])
    cache_steps = [
        s for s in steps if isinstance(s, dict) and "Swatinem/rust-cache" in str(s.get("uses", ""))
    ]
    assert len(cache_steps) == 1, "must have exactly one Swatinem/rust-cache step"
    c_step = cache_steps[0]
    with_block = c_step.get("with", {})
    assert with_block.get("workspaces") == "viewer -> ../target/viewer"
    assert "key" not in with_block, "cache step must have no key of its own (B1, V4, V17)"
    save_if = with_block.get("save-if", "")
    assert "${{ github.ref == 'refs/heads/main' }}" in str(save_if), (
        f"save-if must be on main branch, got: {save_if}"
    )
    assert "hashFiles" not in str(c_step), (
        "cache step must not use hashFiles (defeats prefix fallback)"
    )
