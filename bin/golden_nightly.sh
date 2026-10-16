#!/usr/bin/env bash
# The nightly golden tier (SPR-008 ruling 5; docs/09 V-DET-01, V-DET-02, V-DET-11; the G0 criterion):
# the smoke world for 1,000 sim-years with a monthly ladder at 1, 2, 4 and N threads, three repeats
# each, every ladder byte-identical to the first, each run with at least 5% of its scheduler
# levels holding two or more jobs (VER-SPR-008 DEF-2), and the first ladder's yearly cut equal to
# the committed baseline (VER-SPR-008-TEST GAP-6); then V-DET-11 on 64 injected perturbations.
# The first ladder is kept as artifacts/nightly/smoke_monthly_1000y.ladder: the reference for the
# cross-machine check (bin/cross_machine.sh; worlds/smoke/README.md).
#
# Usage: bin/golden_nightly.sh        exit 0 when every ladder agrees and every case is named
set -euo pipefail
cd "$(dirname "$0")/.."
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
cargo build --release --locked -p rh-cli
cli="$TARGET_DIR/release/rh-cli"
out=artifacts/nightly
work="$TARGET_DIR/golden/nightly"
mkdir -p "$out" "$work"
n=$(nproc 2>/dev/null || echo 2)
[ "$n" -ge 2 ] || n=2
reference=""
for threads in 1 2 4 "$n"; do
  for repeat in 1 2 3; do
    ladder="$work/smoke_monthly_t${threads}_r${repeat}.ladder"
    "$cli" run smoke --years 1000 --threads "$threads" --ladder monthly --require-parallel 5 --out "$ladder"
    if [ -z "$reference" ]; then
      reference="$ladder"
      cp "$ladder" "$out/smoke_monthly_1000y.ladder"
    else
      "$cli" ladder compare "$reference" "$ladder"
    fi
  done
done
echo "V-DET-01/02: 12 monthly ladders of 1,000 sim-years equal at 1, 2, 4 and $n threads x 3 repeats"
# GAP-6: the yearly cut of the monthly run equals the committed yearly baseline, so the nightly
# history is the PR tier's (and a change the PR tier missed is seen here).
"$cli" ladder cut-yearly "$reference" "$work/smoke_yearly_cut.ladder"
"$cli" ladder compare "$work/smoke_yearly_cut.ladder" worlds/smoke/baselines/smoke_yearly_1000y.ladder
cmp "$work/smoke_yearly_cut.ladder" worlds/smoke/baselines/smoke_yearly_1000y.ladder
echo "GAP-6: the yearly cut of the monthly run = worlds/smoke/baselines/smoke_yearly_1000y.ladder"
RH_GOLDEN_V_DET_11_CASES=64 cargo nextest run -p rh-golden --release --locked --test golden --no-capture \
  v_det_11_the_bisect_names_an_injected_month_table_and_row
