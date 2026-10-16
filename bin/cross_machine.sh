#!/usr/bin/env bash
# The cross-machine golden check (SPR-008 T-02, ruling 8; GOV-008 §1 D5; RUN-001, RUN-002 §4):
# on this machine, run the smoke world for 1,000 sim-years with a monthly ladder at 1 and N threads,
# check the two agree, and compare them with a reference ladder written elsewhere (the nightly
# artifact `smoke_monthly_1000y.ladder` from a GitHub-hosted runner). Ladders hold no build id,
# wall time or host, so they compare byte for byte across machines and CPU vendors.
#
# Usage: bin/cross_machine.sh <reference.ladder>
# Writes artifacts/cross_machine/<vendor>-<date>.txt (CPU, toolchain, verdict) and the local
# ladders next to it. Exit 0 when everything agrees, 1 on any difference, 2 on a tool error.
set -uo pipefail
cd "$(dirname "$0")/.."
[ $# -eq 1 ] && [ -f "$1" ] || { echo "usage: bin/cross_machine.sh <reference.ladder>"; exit 2; }
reference="$1"
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
cargo build --release --locked -p rh-cli || exit 2
cli="$TARGET_DIR/release/rh-cli"
cpu=$( (grep -m1 'model name' /proc/cpuinfo 2>/dev/null | cut -d: -f2) || sysctl -n machdep.cpu.brand_string 2>/dev/null || echo unknown)
vendor=$( (grep -m1 'vendor_id' /proc/cpuinfo 2>/dev/null | cut -d: -f2 | tr -d ' ') || echo unknown)
dir=artifacts/cross_machine
mkdir -p "$dir"
stamp="${vendor:-unknown}-$(date -u +%Y%m%d)"
n=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 2)
status=0
"$cli" run smoke --years 1000 --threads 1 --ladder monthly --out "$dir/$stamp.t1.ladder" || status=2
"$cli" run smoke --years 1000 --threads "$n" --ladder monthly --out "$dir/$stamp.tN.ladder" || status=2
{
  echo "cross-machine golden check, smoke, 1,000 sim-years, monthly"
  echo "cpu:${cpu}"
  echo "vendor: ${vendor}"
  echo "threads: 1 and $n"
  echo "toolchain: $(rustc --version)"
  echo "reference: $reference"
  echo "1 vs N threads: $("$cli" ladder compare "$dir/$stamp.t1.ladder" "$dir/$stamp.tN.ladder")"
  echo "this machine vs reference: $("$cli" ladder compare "$reference" "$dir/$stamp.t1.ladder")"
} | tee "$dir/$stamp.txt"
"$cli" ladder compare "$dir/$stamp.t1.ladder" "$dir/$stamp.tN.ladder" >/dev/null || status=$(( status > 1 ? status : 1 ))
"$cli" ladder compare "$reference" "$dir/$stamp.t1.ladder" >/dev/null || status=$(( status > 1 ? status : 1 ))
[ "$status" -eq 0 ] && echo "VERDICT: equal" || echo "VERDICT: DIFFERENT or failed (exit $status); run rh-cli bisect with the two builds"
exit "$status"
