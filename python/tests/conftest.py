"""Shared pytest fixtures for python/tests.

CI sets gate variables (RH_EVENT, RH_PR_TITLE, RH_PUSH_BEFORE, GITHUB_EVENT_NAME, ...) for its whole
job, so a test that reads one without setting it passes locally and fails in CI (SPR-008's
test_check_baseline, SPR-009's test_perf_gate). Every test therefore starts with no RH_* and no
GITHUB_* variable; a test that needs one sets it itself. bin/check_all.sh runs the suite once more
with hostile CI values set, which proves this fixture works.

Refs: SPR-009 (Architect review of c29787c, blocker 1).
"""

import os

import pytest


@pytest.fixture(autouse=True)
def _isolated_environment(monkeypatch: pytest.MonkeyPatch) -> None:
    """Clear every RH_* and GITHUB_* variable before each test."""
    for name in [n for n in os.environ if n.startswith(("RH_", "GITHUB_"))]:
        monkeypatch.delenv(name, raising=False)
