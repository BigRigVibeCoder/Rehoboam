#!/usr/bin/env bash
# The PR tier of GOV-002, fail-fast, with a forensic report (GOV-002 §4, §6: no report means no pass).
# Every sprint adds its own checks here in the same sprint that creates them.
# Usage: bin/check_all.sh            (uses $PYTHON; else python/.venv/bin/python; else python3)
#        PYTHON=/path/to/python bin/check_all.sh
# Needs the Rust toolchain from rust-toolchain.toml, cargo-nextest, and the uv environment from
# `uv sync --frozen --project python` (RUN-001 §2-4). A missing tool fails the first check by name;
# nothing is ever skipped silently.
set -u
cd "$(dirname "$0")/.." || { echo "check_all.sh: cannot enter the repository root" >&2; exit 2; }
if [ -n "${PYTHON:-}" ]; then PY="$PYTHON"; elif [ -x python/.venv/bin/python ]; then PY=python/.venv/bin/python; else PY="$(command -v python3 || command -v python)"; fi
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
  # On GitHub Actions the checkout is detached; name the run so the report can be traced to it.
  if [ "${GITHUB_ACTIONS:-}" = true ]; then
    echo "- **CI run:** ${GITHUB_SERVER_URL:-}/${GITHUB_REPOSITORY:-}/actions/runs/${GITHUB_RUN_ID:-} (${GITHUB_EVENT_NAME:-} on ${GITHUB_REF:-}, head ${GITHUB_SHA:-})"
  fi
  echo
  echo "| # | Check | Result | Seconds |"
  echo "|---|---|---|---|"
} > "$REPORT"

n=0; failed=""
# run <name> <expect> <command...>
#   expect: pass | fail | fail:<text>. A positive control uses fail:<text>: the command must fail AND
#   print <text>, so a control that fails for the wrong reason (a crash, a missing lockfile) is caught.
run() {
  local name="$1" expect="$2"; shift 2
  n=$((n+1))
  local t0 out rc
  t0=$(date +%s)
  out="$("$@" 2>&1)"; rc=$?
  local dt=$(( $(date +%s) - t0 ))
  local ok=0
  case "$expect" in
    pass) if [ $rc -eq 0 ]; then ok=1; fi ;;
    fail) if [ $rc -ne 0 ]; then ok=1; fi ;;
    fail:*) if [ $rc -ne 0 ] && grep -qF -- "${expect#fail:}" <<<"$out"; then ok=1; fi ;;
  esac
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
  cargo fmt --version >/dev/null 2>&1 || missing="$missing rustfmt(RUN-001 §2)"
  cargo clippy --version >/dev/null 2>&1 || missing="$missing clippy(RUN-001 §2)"
  cargo nextest --version >/dev/null 2>&1 || missing="$missing cargo-nextest(RUN-001 §3)"
  cargo deny --version >/dev/null 2>&1 || missing="$missing cargo-deny(RUN-001 §3)"
  local mod
  for mod in ruff mypy pytest pydantic yaml duckdb; do
    "$PY" -I -c "import $mod" >/dev/null 2>&1 || missing="$missing python:$mod(RUN-001 §4)"
  done
  if [ -n "$missing" ]; then echo "Missing tools:$missing"; return 1; fi
  echo "All required tools present."
}

# Positive controls for the forbidden-API gate (V-DET-19): one planted file per API, each of which
# must be caught by its own rule (tests/lint_fixtures/README.md).
forbidden_api_controls() {
  local spec file rule
  for spec in hash_map:hash-collections hash_set:hash-collections thread_rng:thread-rng std_rng:std-rng \
      rand_random:rand-random mul_add:mul-add instant:wall-clock system_time:wall-clock unsafe:unsafe; do
    file="${spec%%:*}"; rule="${spec#*:}"
    run "Forbidden-API positive control: $file must FAIL" "fail:VIOLATION [V-DET-19/$rule]" \
      "$PY" -I bin/lint_forbidden.py --scan "tests/lint_fixtures/forbidden/planted/$file.rs" || return 1
  done
}

# The checks: ONE fail-fast chain. Define every function above this line; a definition placed between
# two `&&` lines splits the chain, and later checks would run after a failure (DEF-2).
run "Required tools present" pass require_tools &&
# Tier 1 — static and contract lints (fastest first)
run "Document index is current (gen_manifest --check)" pass "$PY" -I bin/gen_manifest.py --check &&
run "CI workflows parse (YAML)" pass "$PY" -I -c 'import sys, yaml; [yaml.safe_load(open(f, encoding="utf-8")) for f in sys.argv[1:]]' .github/workflows/ci.yml .github/workflows/advisories.yml &&
run "Python: lint (ruff check)" pass "$PY" -I -m ruff check --config python/pyproject.toml --no-cache . &&
run "Python: format (ruff format --check)" pass "$PY" -I -m ruff format --config python/pyproject.toml --no-cache --check . &&
run "Python: types (mypy --strict)" pass "$PY" -I -m mypy --config-file python/pyproject.toml &&
run "Rust: format (cargo fmt --check)" pass cargo fmt --all --check &&
run "Rust: licences, bans, sources (cargo deny; ADR-0021)" pass cargo deny --color never --locked check bans licenses sources &&
run "cargo deny positive control: GPL dependency must FAIL" "fail:error[rejected]" cargo deny --color never --offline --locked --manifest-path tests/lint_fixtures/deny/gpl_dependency/Cargo.toml check licenses &&
run "cargo deny positive control: xxhash-rust must FAIL" "fail:error[banned]" cargo deny --color never --offline --locked --manifest-path tests/lint_fixtures/deny/banned_crate/Cargo.toml check bans &&
run "Architecture: crate boundaries and planes (lint_arch; V-INSP-03, V-PLN-01)" pass "$PY" -I bin/lint_arch.py &&
run "lint_arch positive control: system -> system must FAIL" "fail:VIOLATION [V-INSP-03]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/system_to_system/Cargo.toml &&
run "lint_arch positive control: system -> rh-snapshot must FAIL" "fail:VIOLATION [V-INSP-03]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/system_to_disallowed/Cargo.toml &&
run "lint_arch positive control: plane A -> plane B must FAIL" "fail:VIOLATION [V-PLN-01]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_to_b/Cargo.toml &&
run "lint_arch positive control: plane A -> plane C must FAIL" "fail:VIOLATION [V-PLN-01]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_to_c/Cargo.toml &&
run "lint_arch positive control: missing plane/kind must FAIL" "fail:VIOLATION [metadata]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/missing_metadata/Cargo.toml &&
run "V-PLN-01 compile-fail fixture has a clean dependency graph" pass "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/compile_fail/plane_a_names_plane_b/Cargo.toml &&
run "V-PLN-01 compile-fail control: plane A naming a plane-B type must FAIL" 'fail:unlinked crate `fixture_observation`' cargo check --offline --locked --quiet --manifest-path tests/lint_fixtures/compile_fail/plane_a_names_plane_b/Cargo.toml --target-dir target/lint-fixtures &&
run "Forbidden APIs in plane A (lint_forbidden; V-DET-19)" pass "$PY" -I bin/lint_forbidden.py &&
forbidden_api_controls &&
run "lint_forbidden positive control: workspace plane-A crate must FAIL" "fail:VIOLATION [V-DET-19/hash-collections] tests/lint_fixtures/forbidden/workspace/fixture-core/" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/workspace/Cargo.toml &&
run "System specs (lint_odd)" pass "$PY" -I specs/systems/lint_odd.py &&
run "Sifting patterns (lint_patterns)" pass "$PY" -I specs/sifting/lint_patterns.py specs/sifting/patterns.example.toml specs/manifest/registry_v0.toml &&
run "Manifest: solo" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/solo.yaml &&
run "Manifest: val_khor" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/val_khor.yaml &&
run "Manifest: aethelgard" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/aethelgard.yaml &&
run "Manifest: uz_job" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/uz_job.yaml &&
run "Manifest: invalid_draft_style must FAIL" fail "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/invalid_draft_style.yaml &&
run "Rust: lint (cargo clippy -D warnings)" pass cargo clippy --workspace --all-targets --locked --quiet -- -D warnings &&
# Tier 2 — unit tests
run "Rust: build the workspace (cargo build)" pass cargo build --workspace --locked --quiet &&
run "Rust: unit tests (cargo nextest)" pass cargo nextest run --workspace --locked --no-fail-fast --status-level fail &&
run "Python: unit tests (pytest)" pass "$PY" -I -m pytest -c python/pyproject.toml --rootdir=. -q &&
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
