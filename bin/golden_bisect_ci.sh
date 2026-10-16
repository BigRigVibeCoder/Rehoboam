#!/usr/bin/env bash
# On a golden-run mismatch in CI, bisect the base revision against this head and write the report
# (SPR-008 ruling 7; docs/13 §4.5: the report is attached to the PR as a job artifact).
#
# Runs only when bin/check_all.sh's report shows a failed golden-run gate: a "Golden run:",
# "Golden ladders:" or "Golden baseline:" row marked FAIL (not the "Golden vectors" rows, and not
# the planted controls); otherwise it says so and exits 0. The baseline build is the base revision,
# built in a worktree under target/: on a pull request $RH_BASELINE_BASE; on a push
# $RH_PUSH_BEFORE (ci.yml fetches the full history for a push), skipped with a message for a push
# with no previous commit; locally $RH_BASELINE_BASE, else HEAD^ (VER-SPR-008 MIN-4). The candidate
# is this checkout's release rh-cli. Both read their own revision's worlds/smoke.
#
# Usage: bin/golden_bisect_ci.sh            writes artifacts/bisect_report.txt
set -uo pipefail
cd "$(dirname "$0")/.."
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
REPORT=artifacts/check_report.md
OUT=artifacts/bisect_report.txt
mkdir -p artifacts
if ! grep -qE '^\| [0-9]+ \| Golden (run|ladders|baseline): [^|]* \| \*\*FAIL\*\* \|' "$REPORT" 2>/dev/null; then
  echo "golden_bisect_ci: no golden-run gate failed; nothing to bisect"
  exit 0
fi
case "${RH_EVENT:-local}" in
  push)
    base="${RH_PUSH_BEFORE:-}"
    if [[ -z "$base" || "$base" =~ ^0+$ ]]; then
      echo "golden_bisect_ci: the push has no previous commit, so there is no base to bisect against" | tee "$OUT"
      exit 0
    fi ;;
  *) base="${RH_BASELINE_BASE:-HEAD^}" ;;
esac
src="$TARGET_DIR/bisect-base-src"
rm -rf "$src" && git worktree prune
if ! git worktree add --detach "$src" "$base" >/dev/null 2>&1; then
  echo "golden_bisect_ci: cannot check out the base revision $base" | tee "$OUT"
  exit 1
fi
echo "golden_bisect_ci: building the base revision $(git -C "$src" rev-parse --short HEAD)"
cargo build --release --locked -p rh-cli --manifest-path "$src/Cargo.toml" --target-dir "$TARGET_DIR/bisect-base" \
  || { echo "golden_bisect_ci: the base revision $base does not build" | tee "$OUT"; exit 1; }
cargo build --release --locked -p rh-cli || exit 1
"$TARGET_DIR/release/rh-cli" bisect --baseline "$TARGET_DIR/bisect-base/release/rh-cli" \
  --candidate "$TARGET_DIR/release/rh-cli" --years 1000 --work "$TARGET_DIR/bisect-work" --out "$OUT"
rc=$?
# 1 means a divergence was located: the expected outcome after a golden failure.
[ "$rc" -le 1 ] || { echo "golden_bisect_ci: the bisect itself failed (exit $rc)" | tee -a "$OUT"; exit 1; }
echo "golden_bisect_ci: report written to $OUT"
