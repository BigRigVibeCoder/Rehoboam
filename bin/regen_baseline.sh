#!/usr/bin/env bash
# Regenerates the smoke world's golden baseline with the golden-run tool (SPR-008 T-04, ruling 6).
#
# A baseline is regenerated, never typed: bin/check_baseline.py (in bin/check_all.sh) requires the
# committed file to equal CI's recomputation byte for byte, and a changed baseline to come in a PR
# whose title carries [C3]. There is no merge queue (GitHub Free), so the C3 PR's own final commit
# writes the baseline with this script (the fallback of docs/13 §4.4).
#
# Usage: bin/regen_baseline.sh            then commit worlds/smoke/baselines/ in the C3 PR
set -euo pipefail
cd "$(dirname "$0")/.."
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
cargo build --release --locked -p rh-cli
work="$TARGET_DIR/golden/regen"
mkdir -p worlds/smoke/baselines "$work"
"$TARGET_DIR/release/rh-cli" run smoke --years 1000 --threads 1 --ladder yearly \
  --out worlds/smoke/baselines/smoke_yearly_1000y.ladder --journal "$work/journal.sqlite"
echo "Regenerated worlds/smoke/baselines/smoke_yearly_1000y.ladder; commit it in the [C3] PR."
