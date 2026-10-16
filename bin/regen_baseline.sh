#!/usr/bin/env bash
# Regenerates the smoke world's golden baseline with the golden-run tool (SPR-008 T-04, ruling 6).
#
# A baseline is regenerated, never typed: bin/check_baseline.py (in bin/check_all.sh) requires the
# committed file to equal CI's recomputation byte for byte, and a changed baseline to come in a PR
# whose title carries [C3]. There is no merge queue (GitHub Free), so the C3 PR's own final commit
# writes the baseline with this script (the fallback of docs/13 §4.4).
#
# The commit that carries the regenerated baseline must end its subject with the tag [C3]: on a
# push, bin/check_baseline.py requires a [C3]-tagged commit in the pushed range whenever the
# baseline changed (VER-SPR-008 MAJ-1), as it requires [C3] at the end of the PR title.
#
# Usage: bin/regen_baseline.sh            then commit worlds/smoke/baselines/ in the C3 PR
set -euo pipefail
cd "$(dirname "$0")/.."
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
cargo build --release --locked -p rh-cli
work="$TARGET_DIR/golden/regen"
mkdir -p worlds/smoke/baselines "$work"
"$TARGET_DIR/release/rh-cli" run smoke --years 1000 --threads 1 --ladder yearly --require-parallel 5 \
  --out worlds/smoke/baselines/smoke_yearly_1000y.ladder --journal "$work/journal.sqlite"
echo "Regenerated worlds/smoke/baselines/smoke_yearly_1000y.ladder."
echo "Commit it in the [C3] PR, with [C3] at the end of the commit subject, e.g.:"
echo "  git commit -m 'feat(SPR-NNN): regenerate the smoke golden baseline [C3]' worlds/smoke/baselines/"
