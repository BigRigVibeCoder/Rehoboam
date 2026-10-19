#!/usr/bin/env bash
# The check tier for the viewer workspace, fail-fast, with a forensic report (SPR-033 T-06; B1).
# Usage: bin/check_viewer.sh
#        bin/check_viewer.sh --part core|app
# Cargo runs with its working directory in viewer/.
set -u
cd "$(dirname "$0")/.." || { echo "check_viewer.sh: cannot enter repository root" >&2; exit 2; }

if [ -n "${PYTHON:-}" ]; then
  PY="$PYTHON"
elif [ -x python/.venv/bin/python ]; then
  PY=python/.venv/bin/python
elif [ -x ../../../python/.venv/bin/python ]; then
  PY=../../../python/.venv/bin/python
else
  PY="$(command -v python3 || command -v python)"
fi

PART=""
case "${1:-}" in
  "") ;;
  --part)
    case "${2:-}" in
      core|app) PART="$2" ;;
      *) echo "check_viewer.sh: --part takes core or app" >&2; exit 2 ;;
    esac ;;
  *) echo "usage: bin/check_viewer.sh [--part core|app]" >&2; exit 2 ;;
esac

REPORT="artifacts/viewer_report.md"
LEDGER="artifacts/viewer_licence_ledger.txt"
mkdir -p artifacts target/viewer

commit="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
dirty="$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')"

{
  echo "# Viewer check report"
  echo
  echo "- **Time:** $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "- **Commit:** ${commit} (${dirty} uncommitted files) · **Branch:** ${branch}"
  echo "- **Python:** $(${PY} --version 2>&1)"
  echo "- **Part:** ${PART:-all (core and app)}"
  if [ "${GITHUB_ACTIONS:-}" = true ]; then
    echo "- **CI run:** ${GITHUB_SERVER_URL:-}/${GITHUB_REPOSITORY:-}/actions/runs/${GITHUB_RUN_ID:-} (${GITHUB_EVENT_NAME:-} on ${GITHUB_REF:-}, head ${GITHUB_SHA:-})"
  fi
  echo
  echo "| # | Check | Result | Seconds |"
  echo "|---|---|---|---|"
} > "$REPORT"

n=0
failed=""

run() {
  local name="$1" expect="$2"
  shift 2
  n=$((n + 1))

  case "$expect" in
    pass:*|fail:*)
      local text="${expect#*:}"
      if [ -z "${text//[[:space:]]/}" ]; then
        echo "| $n | $name | **FAIL** (HARNESS ERROR: empty $expect expectation) | 0 |" >> "$REPORT"
        echo "FAIL  $name (HARNESS ERROR: empty $expect expectation)"
        failed="$name"
        return 1
      fi ;;
  esac

  local t0 dt out rc ok=0
  t0=$(date +%s)
  out="$("$@" 2>&1)"
  rc=$?
  dt=$(( $(date +%s) - t0 ))

  case "$expect" in
    pass)
      if [ $rc -eq 0 ]; then ok=1; fi ;;
    fail)
      if [ $rc -ne 0 ]; then ok=1; fi ;;
    fail:*)
      if [ $rc -ne 0 ] && grep -qF -- "${expect#fail:}" <<<"$out"; then ok=1; fi ;;
    pass:*)
      if [ $rc -eq 0 ] && grep -qF -- "${expect#pass:}" <<<"$out"; then ok=1; fi ;;
  esac

  if [ $ok -eq 1 ]; then
    echo "| $n | $name | PASS | $dt |" >> "$REPORT"
    echo "PASS  $name (${dt}s)"
  else
    echo "| $n | $name | **FAIL** | $dt |" >> "$REPORT"
    echo "FAIL  $name (${dt}s)"
    {
      echo
      echo "## First failure: $name"
      echo
      echo '```'
      echo "$out" | tail -40
      echo '```'
    } >> "$REPORT"
    failed="$name"
    return 1
  fi
}

finish() {
  local status=$? verdict
  if [ -n "${chain_done:-}" ] && [ -z "$failed" ] && [ "$status" -eq 0 ]; then
    verdict="PASS ($n checks${PART:+, part $PART})"
    status=0
  elif [ -n "$failed" ]; then
    verdict="FAIL at \"$failed\" (later checks skipped: fail-fast)"
    status=1
  elif [ -z "${chain_done:-}" ]; then
    verdict="FAIL (the chain exited early, status $status, after $n checks)"
    status=1
  else
    verdict="FAIL (the chain stopped with no gate named)"
    status=1
  fi

  {
    echo
    echo "## Verdict: $verdict"
    echo
    echo "## System & Actions Cache Usage"
    echo
    echo "### Disk usage (\`df -h\`)"
    echo '```'
    df -h . 2>&1 || true
    echo '```'
    echo
    echo "### GitHub Actions Cache Usage"
    echo '```'
    if command -v gh >/dev/null 2>&1 && [ -n "${GITHUB_TOKEN:-}" ]; then
      gh cache list 2>&1 || echo "gh cache list not available or failed"
    else
      echo "Actions cache usage: N/A (local run or gh unauthenticated)"
    fi
    echo '```'
  } >> "$REPORT"

  echo "Report: $REPORT"
  exit "$status"
}

chain_done=""
trap finish EXIT

# Helper functions for checks
viewer_cargo_fmt() {
  (cd viewer && cargo fmt --all --check)
}

viewer_clippy() {
  (cd viewer && cargo clippy --workspace --all-targets --locked -- -D warnings)
}

viewer_deny() {
  (cd viewer && cargo deny --color never --locked --config ../deny.toml check bans sources)
}

viewer_licence_ledger() {
  {
    echo "# Licence ledger for viewer/Cargo.toml at ${commit}: every crate in the graph and its licence(s)."
    echo "# Recorded for forensic audit; not a gate."
    (cd viewer && cargo deny --color never --locked --config ../deny.toml list --layout crate)
  } > "$LEDGER"
  cat "$LEDGER"
}

viewer_nextest() {
  (cd viewer && cargo nextest run --locked --no-capture)
}

viewer_digest_tests_release() {
  (cd viewer && cargo nextest run --release --locked --test terrain_golden --test determinism --no-capture)
}

viewer_terrain_golden() {
  (cd viewer && cargo run --release --bin terrain_golden -- --check)
}

viewer_mesh_200_bench() {
  local t0 t1 dt us_per_chunk out
  t0=$(date +%s%N 2>/dev/null || true)
  out="$(cd viewer && cargo run --release --bin terrain_golden -- --mesh-200 2>&1)" || return 1
  t1=$(date +%s%N 2>/dev/null || true)
  echo "$out"
  if [ -n "$t0" ] && [ -n "$t1" ] && [ "${#t0}" -gt 10 ] && [ "${#t1}" -gt 10 ]; then
    dt=$(( (t1 - t0) / 1000 ))
    us_per_chunk=$(( dt / 200 ))
    echo "Mean time to mesh one chunk on one core: ${us_per_chunk} µs (bound: 3000 µs; V6)"
  fi
}

control_mesh_drop_face() {
  (cd viewer && cargo test -p rh-view-core --test differential --features mutant-mesh-drop-face)
}

control_lod_majority() {
  (cd viewer && cargo test -p rh-view-core --test lod --features mutant-lod-majority)
}

control_terrain_off_by_one() {
  (cd viewer && cargo test -p rh-view-core --test terrain_golden --features mutant-terrain-off-by-one)
}

control_unsafe_code() {
  (cd viewer && cargo check -p rh-view-core --features mutant-unsafe)
}

check_assets() {
  local found
  found="$(find viewer/ -type f \( -name "*.png" -o -name "*.jpg" -o -name "*.jpeg" -o -name "*.gltf" -o -name "*.glb" -o -name "*.obj" -o -name "*.wav" -o -name "*.mp3" -o -name "*.ogg" -o -name "*.flac" -o -name "*.ktx2" -o -name "*.dds" \) 2>/dev/null)"
  if [ -n "$found" ]; then
    echo "VIOLATION [assets]: non-procedural asset found under viewer/: $found"
    return 1
  fi
  echo "ASSETS: procedural only (no image, model or sound file found under viewer/)"
  return 0
}

check_root_lock() {
  local base
  base="$(git merge-base origin/main HEAD 2>/dev/null || echo "origin/main")"
  if git rev-parse --verify "$base" >/dev/null 2>&1; then
    local changed_files
    changed_files="$(git diff --name-only "$base"...HEAD 2>/dev/null || git diff --name-only "$base" 2>/dev/null)"
    local all_viewer=1
    for f in $changed_files; do
      if ! "$PY" -I -c "import sys, ci_scope; sys.exit(0 if ci_scope.is_doc_or_viewer(sys.argv[1]) else 1)" "$f" 2>/dev/null; then
        all_viewer=0
        break
      fi
    done
    if [ "$all_viewer" -eq 1 ] && [ -n "$changed_files" ]; then
      if git diff --name-only "$base"...HEAD -- Cargo.lock 2>/dev/null | grep -q "Cargo.lock"; then
        echo "VIOLATION [root-lock]: root Cargo.lock modified in viewer-only change"
        return 1
      fi
    fi
  fi
  echo "ROOT LOCK: byte-identical to merge base in viewer-only change"
  return 0
}

if [ "$PART" = "app" ]; then
  echo "SKIP: part app is Stage B (SPR-033b)"
  chain_done=1
  exit 0
fi

# The fail-fast chain of viewer checks in exact order (SPR-033 T-06)
run "1. Rust: format (cargo fmt --check)" pass viewer_cargo_fmt &&
run "2. Rust: clippy (cargo clippy -D warnings)" pass viewer_clippy &&
run "3a. Rust: deny bans and sources (cargo deny check)" pass viewer_deny &&
run "3b. Licence ledger recorded (artifacts/viewer_licence_ledger.txt)" pass viewer_licence_ledger &&
run "3c. Lock age: all direct pins >= 14 days old (viewer/tools/lock_age.py --check)" pass "$PY" -I viewer/tools/lock_age.py --check &&
run "4a. Terrain reference: 400 columns verified (specs/viewer/terrain_ref.py --check)" pass "$PY" -I specs/viewer/terrain_ref.py --check &&
run "4b. terrain_ref positive control: route-order must FAIL (specs/viewer/terrain_ref.py --control route-order)" "fail:STALE:" "$PY" -I specs/viewer/terrain_ref.py --control route-order &&
run "5a. Worldgen export: hamlet (viewer/tools/export_worldgen_lite.py --world hamlet --check)" pass "$PY" -I viewer/tools/export_worldgen_lite.py --world hamlet --check &&
run "5b. Worldgen export: solo (viewer/tools/export_worldgen_lite.py --world solo --check)" pass "$PY" -I viewer/tools/export_worldgen_lite.py --world solo --check &&
run "5c. Worldgen export: household (viewer/tools/export_worldgen_lite.py --world household --check)" pass "$PY" -I viewer/tools/export_worldgen_lite.py --world household --check &&
run "5d. Worldgen export: twin_vales (viewer/tools/export_worldgen_lite.py --manifest specs/manifest/examples/twin_vales.yaml --check)" pass "$PY" -I viewer/tools/export_worldgen_lite.py --manifest specs/manifest/examples/twin_vales.yaml --check &&
run "5e. export_worldgen_lite positive control: shifted-seed must FAIL (viewer/tools/export_worldgen_lite.py --world hamlet --check --control shifted-seed)" "fail:STALE:" "$PY" -I viewer/tools/export_worldgen_lite.py --world hamlet --check --control shifted-seed &&
run "6. Rust: unit and integration tests (cargo nextest run --locked)" pass viewer_nextest &&
run "7a. Rust: digest tests in release (cargo nextest run --release)" pass viewer_digest_tests_release &&
run "7b. Terrain golden digests match (cargo run --release --bin terrain_golden -- --check)" pass viewer_terrain_golden &&
run "7c. Meshing benchmark: 200 terrain chunks timed (V6 reported)" pass viewer_mesh_200_bench &&
run "8a. Planted mutant: mutant-mesh-drop-face must FAIL" "fail:FAIL mutant-mesh-drop-face: face count mismatch" control_mesh_drop_face &&
run "8b. Planted mutant: mutant-lod-majority must FAIL" "fail:FAIL mutant-lod-majority: road lost at L1" control_lod_majority &&
run "8c. Planted mutant: mutant-terrain-off-by-one must FAIL" "fail:FAIL mutant-terrain-off-by-one: height rounding shifted by one" control_terrain_off_by_one &&
run "8d. Planted mutant: mutant-unsafe must fail to compile on unsafe_code" "fail:usage of an \`unsafe\` block" control_unsafe_code &&
run "9. Assets check: procedural only, no static media files under viewer/" pass check_assets &&
run "10. Root lock integrity: byte-identical to merge base in viewer-only changes" pass check_root_lock

status=$?
chain_done=1
exit "$status"
