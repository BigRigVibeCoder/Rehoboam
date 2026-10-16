#!/usr/bin/env bash
# The PR tier of GOV-002, fail-fast, with a forensic report (GOV-002 §4, §6: no report means no pass).
# Every sprint adds its own checks here in the same sprint that creates them.
# Usage: bin/check_all.sh            (uses $PYTHON; else .venv/bin/python; else python3)
#        PYTHON=/path/to/python bin/check_all.sh
# Needs the Rust toolchain from rust-toolchain.toml and cargo-nextest (RUN-001 §2-3). A missing tool
# fails the first check by name; nothing is ever skipped silently.
set -u
cd "$(dirname "$0")/.."
if [ -n "${PYTHON:-}" ]; then PY="$PYTHON"; elif [ -x .venv/bin/python ]; then PY=.venv/bin/python; else PY="$(command -v python3 || command -v python)"; fi
REPORT="artifacts/check_report.md"
mkdir -p artifacts
commit="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
dirty="$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
{
  echo "# Check report"
  echo
  echo "- **Time:** $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "- **Commit:** ${commit} (${dirty} uncommitted files) · **Branch:** ${branch}"
  echo "- **Python:** $(${PY} --version 2>&1)"
  echo
  echo "| # | Check | Result | Seconds |"
  echo "|---|---|---|---|"
} > "$REPORT"

n=0; failed=""
run() {   # run <name> <expect: pass|fail> <command...>
  local name="$1" expect="$2"; shift 2
  n=$((n+1))
  local t0=$(date +%s) out rc
  out="$("$@" 2>&1)"; rc=$?
  local dt=$(( $(date +%s) - t0 ))
  local ok=0
  if [ "$expect" = pass ] && [ $rc -eq 0 ]; then ok=1; fi
  if [ "$expect" = fail ] && [ $rc -ne 0 ]; then ok=1; fi
  if [ $ok -eq 1 ]; then
    echo "| $n | $name | PASS | $dt |" >> "$REPORT"; echo "PASS  $name"
  else
    echo "| $n | $name | **FAIL** | $dt |" >> "$REPORT"; echo "FAIL  $name"
    { echo; echo "## First failure: $name"; echo; echo '```'; echo "$out" | tail -40; echo '```'; } >> "$REPORT"
    failed="$name"
    return 1
  fi
}

# Every tool the tier needs, checked up front so a missing one is named rather than mistaken for a
# failing check (RUN-001 says how to install each).
require_tools() {
  local missing=""
  command -v cargo >/dev/null 2>&1 || missing="$missing cargo(RUN-001 §2)"
  cargo nextest --version >/dev/null 2>&1 || missing="$missing cargo-nextest(RUN-001 §3)"
  if [ -n "$missing" ]; then echo "Missing tools:$missing"; return 1; fi
  echo "All required tools present."
}

run "Required tools present" pass require_tools &&
# Tier 1 — static and contract lints (fastest first)
run "Document index is current (gen_manifest --check)" pass "$PY" -I bin/gen_manifest.py --check &&
run "System specs (lint_odd)" pass "$PY" -I specs/systems/lint_odd.py &&
run "Sifting patterns (lint_patterns)" pass "$PY" -I specs/sifting/lint_patterns.py specs/sifting/patterns.example.toml specs/manifest/registry_v0.toml &&
run "Manifest: solo" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/solo.yaml &&
run "Manifest: val_khor" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/val_khor.yaml &&
run "Manifest: aethelgard" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/aethelgard.yaml &&
run "Manifest: uz_job" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/uz_job.yaml &&
run "Manifest: invalid_draft_style must FAIL" fail "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/invalid_draft_style.yaml &&
# Tier 2 — unit tests
run "Rust: build the workspace (cargo build)" pass cargo build --workspace --locked --quiet &&
run "Rust: unit tests (cargo nextest)" pass cargo nextest run --workspace --locked --no-fail-fast --status-level fail &&
# Tier 5 — golden vectors
run "Golden vectors: Brass (M2)" pass "$PY" -I specs/demography/selftest_brass.py &&
run "Golden vectors: M4 basket and price rule" pass "$PY" -I specs/economy/selftest_economy.py &&
# Tier 6/8 — determinism-style and contract prototypes
run "Virtual telemetry (S5 prototype)" pass "$PY" -I specs/stream/selftest_virtual_telemetry.py &&
run "Chronicle SQL-twin equivalence" pass "$PY" -I specs/chronicle/selftest.py
status=$?

{
  echo
  if [ $status -eq 0 ]; then echo "## Verdict: PASS ($n checks)"; else echo "## Verdict: FAIL at \"$failed\" (later checks skipped: fail-fast)"; fi
} >> "$REPORT"
echo "Report: $REPORT"
exit $status
