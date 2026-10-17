#!/usr/bin/env bash
# The nightly golden tier (SPR-008 ruling 5; docs/09 V-DET-01, V-DET-02, V-DET-11; the G0 criterion):
# the smoke world for 1,000 sim-years with a monthly ladder at 1, 2, 4 and N threads, three repeats
# each, every ladder byte-identical to the first, each run with at least 5% of its scheduler
# levels holding two or more jobs (VER-SPR-008 DEF-2), and the first ladder's yearly cut equal to
# the committed baseline (VER-SPR-008-TEST GAP-6); then V-DET-11 on 64 injected perturbations. Since
# SPR-012 T-08 the same for solo (no parallel floor: one system, one job per level).
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
# SPR-012 T-08 (V-DET-01/02 on solo): 1,000 sim-years, monthly, at 1, 2, 4 and N threads x 3 repeats,
# every ladder byte-identical; and the yearly cut of a 200-year monthly run = the PR tier's baseline.
solo_reference=""
for threads in 1 2 4 "$n"; do
  for repeat in 1 2 3; do
    ladder="$work/solo_monthly_t${threads}_r${repeat}.ladder"
    "$cli" run solo --years 1000 --threads "$threads" --ladder monthly --out "$ladder"
    if [ -z "$solo_reference" ]; then
      solo_reference="$ladder"
      cp "$ladder" "$out/solo_monthly_1000y.ladder"
    else
      "$cli" ladder compare "$solo_reference" "$ladder"
    fi
  done
done
echo "V-DET-01/02: 12 monthly ladders of solo, 1,000 sim-years, equal at 1, 2, 4 and $n threads x 3 repeats"
"$cli" run solo --years 200 --threads 1 --ladder monthly --out "$work/solo_monthly_200y.ladder"
"$cli" ladder cut-yearly "$work/solo_monthly_200y.ladder" "$work/solo_yearly_cut.ladder"
"$cli" ladder compare "$work/solo_yearly_cut.ladder" worlds/solo/baselines/solo_yearly_200y.ladder
cmp "$work/solo_yearly_cut.ladder" worlds/solo/baselines/solo_yearly_200y.ladder
echo "GAP-6: the yearly cut of solo's 200-year monthly run = worlds/solo/baselines/solo_yearly_200y.ladder"
RH_GOLDEN_V_DET_11_CASES=64 cargo nextest run -p rh-golden --release --locked --test golden --no-capture \
  v_det_11_the_bisect_names_an_injected_month_table_and_row
# SPR-016 (05.1 §6 item 4 v1.10; REV-SPR-016 R5): the exhaustive sweep of the fixed-point exp/ln reference
# against its 60-digit oracle: every Q16_16-granular input of exp and ln, plus 2^16 pseudo-random Q32_32
# inputs of each (about 5 minutes of Python). The reference is standard-library only, so the system
# python3 serves: the nightly job has no python/.venv (REV-SPR-016 round 2).
"${PYTHON:-python3}" -I specs/numerics/exp_ln_v0.py --check --exhaustive
echo "exp/ln v0: the exhaustive sweep is faithful and the golden file is current"
# SPR-030 (REV-SPR-030 R10): the fixture worlds' every case (the PR tier renders kingdom_10k at seed 1 only) and
# the CRN pairing protocol at both seeds with its controls (the PR tier runs seed 1). Standard-library references.
"${PYTHON:-python3}" -I specs/worldgen/selftest_fixture_worlds.py --tier nightly
"${PYTHON:-python3}" -I specs/determinism/selftest_crn_pairing.py --seeds 1,2 --controls
echo "SPR-030 fixture worlds and CRN pairing: every case equals the golden outputs; every control fails as required"
