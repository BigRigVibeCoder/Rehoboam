#!/usr/bin/env bash
# Regenerates the golden baselines with the golden-run tool (SPR-008 T-04, ruling 6): smoke's, and
# since SPR-012 T-08 solo's (its 200-year yearly ladder, the PR tier of SPR-011 ruling 16), and since
# SPR-013 T-13 household's and hamlet's (rungs 1 and 2: 200-year yearly ladders at 1 thread, serial,
# with their committed input scripts).
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
# Usage: bin/regen_baseline.sh [smoke|solo|household|hamlet]   (no argument: all four) then commit
#        worlds/*/baselines/
#        in the C3 PR
set -euo pipefail
cd "$(dirname "$0")/.."
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
cargo build --release --locked -p rh-cli
# BCK-002 A-52: never a baseline from a build with a mutant-* feature compiled in (rh-cli run refuses
# it too; this says so before any run).
if ! info="$("$TARGET_DIR/release/rh-cli" build-info)" || [ "$info" != "mutants: none" ]; then
  echo "regen_baseline.sh: refused: the rh-cli build reports '$info' (BCK-002 A-52)" >&2
  exit 3
fi
work="$TARGET_DIR/golden/regen"
mkdir -p "$work"
worlds="${1:-smoke solo household hamlet}"
for world in $worlds; do
  case "$world" in
    smoke)
      mkdir -p worlds/smoke/baselines
      "$TARGET_DIR/release/rh-cli" run smoke --years 1000 --threads 1 --ladder yearly --require-parallel 5 \
        --out worlds/smoke/baselines/smoke_yearly_1000y.ladder --journal "$work/journal.sqlite"
      echo "Regenerated worlds/smoke/baselines/smoke_yearly_1000y.ladder." ;;
    solo)
      mkdir -p worlds/solo/baselines
      "$TARGET_DIR/release/rh-cli" run solo --years 200 --threads 1 --ladder yearly \
        --out worlds/solo/baselines/solo_yearly_200y.ladder --journal "$work/solo_journal.sqlite"
      echo "Regenerated worlds/solo/baselines/solo_yearly_200y.ladder." ;;
    household|hamlet)
      mkdir -p "worlds/$world/baselines"
      "$TARGET_DIR/release/rh-cli" run "$world" --years 200 --threads 1 --ladder yearly --expect-serial \
        --out "worlds/$world/baselines/${world}_yearly_200y.ladder" --journal "$work/${world}_journal.sqlite"
      echo "Regenerated worlds/$world/baselines/${world}_yearly_200y.ladder." ;;
    *) echo "regen_baseline.sh: unknown world $world (smoke, solo, household or hamlet)" >&2; exit 2 ;;
  esac
done
echo "Commit it in the [C3] PR, with [C3] at the end of the commit subject, e.g.:"
echo "  git commit -m 'feat(SPR-NNN): regenerate the golden baselines [C3]' worlds/*/baselines/"
