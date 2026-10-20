#!/usr/bin/env bash
# G1a calibration ensembles runner (SPR-028 Task T-03 / SPR-030b2).
#
# Governed by:
#   CODEX/05_PROJECT/SPR-028_ValidationHarness.md §T-03, §T-04;
#   docs/09_verification_and_validation.md §3.1, §4.
#
# Usage:
#   bin/g1a_ensembles.sh [--mode session|weekly] [--jobs J] [--out DIR] [--dry-run]
#
# Modes:
#   session (default):
#     - fixture-demography_1000: 32 seeds (1-32) x 1,000 years (gates V-SF-01.r, V-SF-02, V-SF-02b)
#     - kingdom_10k: 32 seeds (1-32) x 1,000 years (reports V-SF-01.r)
#   weekly:
#     - fixture-demography_1000: 32 seeds (1-32) x 300 years (gates V-SF-02, V-SF-02b)

set -euo pipefail
cd "$(dirname "$0")/.."

MODE="session"
JOBS=4
OUT_DIR="target/calibration/g1a"
DRY_RUN=0
RUST_BIN=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      MODE="$2"
      shift 2
      ;;
    --jobs)
      JOBS="$2"
      shift 2
      ;;
    --out)
      OUT_DIR="$2"
      shift 2
      ;;
    --rust-bin)
      RUST_BIN="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      echo "Usage: $0 [--mode session|weekly] [--jobs J] [--out DIR] [--rust-bin BIN] [--dry-run]"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 2
      ;;
  esac
done

if [ -n "${PYTHON:-}" ]; then
  PY="$PYTHON"
elif [ -x python/.venv/bin/python ]; then
  PY=python/.venv/bin/python
else
  PY=python3
fi

RUN_CMD=("$PY" -m rehoboam_calib ensemble)
if [ -n "$RUST_BIN" ]; then
  RUN_CMD+=(--rust-bin "$RUST_BIN")
fi

run_or_dry() {
  if [ "$DRY_RUN" -eq 1 ]; then
    echo "$*"
  else
    echo ">>> Running: $*"
    "$@"
  fi
}

case "$MODE" in
  session)
    echo "=== Running G1a Demography Session Ensembles (32 seeds x 1,000 years) ==="
    run_or_dry "${RUN_CMD[@]}" \
      --world fixture-demography_1000 \
      --seeds 1-32 \
      --years 1000 \
      --jobs "$JOBS" \
      --out "$OUT_DIR/demography_1000" \
      --bands V-SF-01.r,V-SF-01.pop,V-SF-02,V-SF-02b

    echo "=== Running G1a kingdom_10k Growth Report (32 seeds x 1,000 years) ==="
    run_or_dry "${RUN_CMD[@]}" \
      --world kingdom_10k \
      --seeds 1-32 \
      --years 1000 \
      --jobs "$JOBS" \
      --out "$OUT_DIR/kingdom_10k" \
      --bands V-SF-01.r
    ;;

  weekly)
    echo "=== Running Weekly Demography Ensembles (32 seeds x 300 years) ==="
    run_or_dry "${RUN_CMD[@]}" \
      --world fixture-demography_1000 \
      --seeds 1-32 \
      --years 300 \
      --jobs "$JOBS" \
      --out "$OUT_DIR/demography_300" \
      --bands V-SF-02,V-SF-02b
    ;;

  *)
    echo "Invalid mode '$MODE'. Expected 'session' or 'weekly'." >&2
    exit 2
    ;;
esac

echo "G1a ensembles completed successfully."
