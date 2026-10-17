#!/usr/bin/env bash
# One wall-clock baseline session on a director's machine (SPR-009 T-04; RUN-001 §6; RUN-002 §4-5).
#
# Usage: bin/bench_session.sh <adjutant|laptop> [runs]       (runs: default 5, at least 5)
#
# It records the run conditions (machine type, CPU model, kernel, power profile, temperature, load),
# runs criterion's wall benches (benches 7 and 8 at 100k and 1M rows; RH_WALL_ROWS overrides) <runs>
# times as separate processes, summarises each run from criterion's samples, and combines them: per
# case the median of the run medians and the run-to-run spread. That spread decides whether a
# wall-clock gate is possible on the machine (well under 10%; SPR-009 T-02b). It also measures the
# instruction counts (bin/perf_gate.py), which must equal the CI baseline's toolchain-for-toolchain.
#
# Everything goes to artifacts/perf/session_<machine>_<UTC time>/, which the adjutant workflow uploads.
# The session never writes a committed file: the trend baseline is recorded afterwards, deliberately,
# with `python -I bin/wall_trend.py record --measured <dir>/session.txt --trend
# crates/rh-bench/baselines/wall_<machine>_v1.txt`, in a [perf-baseline] pull request.
#
# The laptop must be on AC power with the performance profile (RUN-001 §6); the script refuses
# otherwise, because a throttled run would become the baseline.
set -euo pipefail
cd "$(dirname "$0")/.."
machine="${1:-}"; runs="${2:-5}"
case "$machine" in adjutant|laptop) ;; *) echo "usage: $0 <adjutant|laptop> [runs]" >&2; exit 2 ;; esac
[ "$runs" -ge 5 ] || { echo "RUN-002 §5: at least 5 runs" >&2; exit 2; }
if [ -n "${PYTHON:-}" ]; then PY="$PYTHON"; elif [ -x python/.venv/bin/python ]; then PY=python/.venv/bin/python; else PY=python3; fi
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
out="artifacts/perf/session_${machine}_$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$out"

# RUN-001 §6: on the laptop, AC power and the performance profile, or no session.
if [ "$machine" = laptop ]; then
  profile="$(powerprofilesctl get 2>/dev/null || echo unknown)"
  online="$(cat /sys/class/power_supply/AC*/online 2>/dev/null | head -1 || echo 0)"
  if [ "$profile" != performance ] || [ "$online" != 1 ]; then
    echo "laptop: power profile '$profile', AC online '$online'; RUN-001 §6 needs AC and 'performance'" >&2
    exit 2
  fi
fi

{
  echo "machine: $machine"
  echo "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "commit: $(git rev-parse HEAD) ($(git status --porcelain | wc -l | tr -d ' ') uncommitted files)"
  echo "product: $(cat /sys/class/dmi/id/product_name 2>/dev/null || echo unavailable)"
  # A GCE VM names its machine type in the metadata server (RUN-002 §5); elsewhere it is absent.
  echo "gce machine type: $(curl -s -m 2 -H 'Metadata-Flavor: Google' http://metadata.google.internal/computeMetadata/v1/instance/machine-type 2>/dev/null || echo unavailable)"
  echo "kernel: $(uname -srvm)"
  echo "power profile: $(powerprofilesctl get 2>/dev/null || echo unavailable)"
  echo "load average: $(cut -d' ' -f1-3 /proc/loadavg)"
  echo "rustc: $(rustc -V)"
  echo "valgrind: $(valgrind --version 2>/dev/null || echo unavailable)"
} > "$out/conditions.txt"
lscpu > "$out/lscpu.txt"
(sensors 2>/dev/null || echo "sensors unavailable") > "$out/temperature_before.txt"

for i in $(seq 1 "$runs"); do
  rm -rf "$TARGET_DIR/criterion"
  cargo bench --locked --quiet -p rh-bench --bench wall > "$out/criterion_run$i.log" 2>&1
  "$PY" -I bin/wall_trend.py summarize --criterion-dir "$TARGET_DIR/criterion" --machine "$machine" --out "$out/run$i.txt"
done
"$PY" -I bin/wall_trend.py combine "$out"/run*.txt --out "$out/session.txt"
(sensors 2>/dev/null || echo "sensors unavailable") > "$out/temperature_after.txt"

# Instruction counts on this machine: equal to CI's when the toolchain lines are equal (T-02a).
"$PY" -I bin/perf_gate.py measure --out "$out/instructions.txt"
"$PY" -I bin/perf_gate.py check --measured "$out/instructions.txt" > "$out/instructions_check.txt" || true

echo "session written to $out"
