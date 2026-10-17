#!/usr/bin/env bash
# The PR tier of GOV-002, fail-fast, with a forensic report (GOV-002 §4, §6: no report means no pass).
# Every sprint adds its own checks here in the same sprint that creates them.
# Usage: bin/check_all.sh            (uses $PYTHON; else python/.venv/bin/python; else python3)
#        bin/check_all.sh --part core|golden   (one part only; no flag runs both)
#        PYTHON=/path/to/python bin/check_all.sh
# The chain has two parts (SPR-009 review 2; BCK-002 A-38): `core` (lints, builds, workspace and
# by-name tests, Python, specs) and `golden` (release builds, golden runs with the vitals observer,
# performance gates). CI runs them as two parallel jobs so the PR tier fits its 15 minutes
# (V-PERF-04); every gate still runs on every PR, and a local run with no flag runs both.
# Needs the Rust toolchain from rust-toolchain.toml, cargo-nextest, cargo-deny, valgrind,
# iai-callgrind-runner (SPR-009), uv, and the uv environment from `uv sync --frozen --project python` (RUN-001 §2-4). A missing tool fails the
# first check by name; nothing is ever skipped silently.
set -u
cd "$(dirname "$0")/.." || { echo "check_all.sh: cannot enter the repository root" >&2; exit 2; }
if [ -n "${PYTHON:-}" ]; then PY="$PYTHON"; elif [ -x python/.venv/bin/python ]; then PY=python/.venv/bin/python; else PY="$(command -v python3 || command -v python)"; fi
PART=""
case "${1:-}" in
  "") ;;
  --part)
    case "${2:-}" in
      core|golden) PART="$2" ;;
      *) echo "check_all.sh: --part takes core or golden" >&2; exit 2 ;;
    esac ;;
  *) echo "usage: bin/check_all.sh [--part core|golden]" >&2; exit 2 ;;
esac
REPORT="artifacts/check_report.md"
# Built binaries live under $CARGO_TARGET_DIR when it is set (SPR-005 Tester OBS); never assume target/.
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
RH_CLI="$TARGET_DIR/debug/rh-cli"
mkdir -p artifacts
commit="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
dirty="$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
{
  echo "# Check report"
  echo
  echo "- **Time:** $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "- **Commit:** ${commit} (${dirty} uncommitted files) · **Branch:** ${branch}"
  echo "- **Python:** $(${PY} --version 2>&1)"
  echo "- **Part:** ${PART:-all (core and golden)}"
  # On GitHub Actions the checkout is detached; name the run so the report can be traced to it.
  if [ "${GITHUB_ACTIONS:-}" = true ]; then
    echo "- **CI run:** ${GITHUB_SERVER_URL:-}/${GITHUB_REPOSITORY:-}/actions/runs/${GITHUB_RUN_ID:-} (${GITHUB_EVENT_NAME:-} on ${GITHUB_REF:-}, head ${GITHUB_SHA:-})"
  fi
  echo
  echo "| # | Check | Result | Seconds |"
  echo "|---|---|---|---|"
} > "$REPORT"

n=0; failed=""
# run <name> <expect> <command...>
#   expect: pass | fail | fail:<text> | pass:<text>. A positive control uses fail:<text>: the command
#   must fail AND print <text>, so a control that fails for the wrong reason (a crash, a missing
#   lockfile) is caught. pass:<text> is the same for a command that must succeed and print <text>.
run() {
  # A gate of the part this run leaves out (--part) is neither run nor counted; `part` says so.
  if [ -n "${PART:-}" ] && [ "${current_part:-}" != "$PART" ]; then return 0; fi
  local name="$1" expect="$2"; shift 2
  n=$((n+1))
  local t0 out rc
  t0=$(date +%s)
  out="$("$@" 2>&1)"; rc=$?
  local dt=$(( $(date +%s) - t0 ))
  local ok=0
  case "$expect" in
    pass) if [ $rc -eq 0 ]; then ok=1; fi ;;
    fail) if [ $rc -ne 0 ]; then ok=1; fi ;;
    fail:*) if [ $rc -ne 0 ] && grep -qF -- "${expect#fail:}" <<<"$out"; then ok=1; fi ;;
    pass:*) if [ $rc -eq 0 ] && grep -qF -- "${expect#pass:}" <<<"$out"; then ok=1; fi ;;
  esac
  if [ $ok -eq 1 ]; then
    echo "| $n | $name | PASS | $dt |" >> "$REPORT"; echo "PASS  $name"
  else
    echo "| $n | $name | **FAIL** | $dt |" >> "$REPORT"; echo "FAIL  $name"
    { echo; echo "## First failure: $name"; echo; echo '```'; echo "$out" | tail -40; echo '```'; } >> "$REPORT"
    failed="$name"
    return 1
  fi
}

# part <name>: the chain's gates from here to the next `part` belong to part <name> (core or golden).
# Each gate belongs to exactly one part (test_check_all_inventory.py); --part runs one of them.
part() {
  current_part="$1"
  if [ -n "${PART:-}" ] && [ "$1" != "$PART" ]; then
    echo "SKIP  part $1: its gates run with --part $1 (in CI, a parallel job)"
    echo "| - | part $1: not run here (--part $PART); its gates run in their own job | SKIP | 0 |" >> "$REPORT"
  fi
  return 0
}

# Every tool the tier needs, checked up front so a missing one is named rather than mistaken for a
# failing check (RUN-001 says how to install each).
# require_tools <part>: valgrind and iai-callgrind-runner serve only the golden part's perf gates.
require_tools() {
  local missing=""
  command -v cargo >/dev/null 2>&1 || missing="$missing cargo(RUN-001 §2)"
  cargo fmt --version >/dev/null 2>&1 || missing="$missing rustfmt(RUN-001 §2)"
  cargo clippy --version >/dev/null 2>&1 || missing="$missing clippy(RUN-001 §2)"
  cargo nextest --version >/dev/null 2>&1 || missing="$missing cargo-nextest(RUN-001 §3)"
  cargo deny --version >/dev/null 2>&1 || missing="$missing cargo-deny(RUN-001 §3)"
  if [ "$1" = golden ]; then
    command -v valgrind >/dev/null 2>&1 || missing="$missing valgrind(RUN-001 §1)"
    command -v iai-callgrind-runner >/dev/null 2>&1 || missing="$missing iai-callgrind-runner(RUN-001 §3)"
  fi
  command -v uv >/dev/null 2>&1 || missing="$missing uv(RUN-001 §4)"
  local mod
  for mod in ruff mypy pytest pydantic yaml duckdb; do
    "$PY" -I -c "import $mod" >/dev/null 2>&1 || missing="$missing python:$mod(RUN-001 §4)"
  done
  if [ -n "$missing" ]; then echo "Missing tools:$missing"; return 1; fi
  echo "All required tools present."
}

# Licence ledger (ADR-0021 as amended 2026-10-16, personal-use mode): it records, it never gates.
# licence_ledger <Cargo.toml> <output file>: writes every crate in the graph with its licence(s), and
# prints the ledger so a control can look for a licence in it.
licence_ledger() {
  local manifest="$1" out="$2"
  mkdir -p "$(dirname "$out")" || return 1
  {
    echo "# Licence ledger for ${manifest} at ${commit}: every crate in the graph and its licence(s)."
    echo "# Personal-use mode (ADR-0021 as amended 2026-10-16): recorded for a future decision to share; not a gate."
    cargo deny --color never --locked --manifest-path "$manifest" list --layout crate
  } > "$out" || return 1
  cat "$out"
}

# The git-source control: copies tests/lint_fixtures/deny/git_source/ to target/, serves its dep/ from
# a local git repository (file://, no network), and runs `cargo deny check sources`, which must reject it.
git_source_control() {
  local dst=target/rendered-fixtures/git_source
  rm -rf "$dst" && mkdir -p "$dst" && cp -R tests/lint_fixtures/deny/git_source/. "$dst/" || return 1
  git -C "$dst/dep" init -q && git -C "$dst/dep" add . &&
    git -C "$dst/dep" -c user.name=fixture -c user.email=fixture@localhost -c commit.gpgsign=false \
      commit -q -m "git-source fixture" || return 1
  sed "s|@DEP_GIT_URL@|file://$PWD/$dst/dep|" "$dst/Cargo.toml.in" > "$dst/Cargo.toml" || return 1
  cargo generate-lockfile --quiet --manifest-path "$dst/Cargo.toml" || return 1
  cargo deny --color never --offline --locked --manifest-path "$dst/Cargo.toml" check sources
}

# Clippy controls: clippy_control <fixture> <package> renders tests/lint_fixtures/clippy/<fixture>/ into
# target/, with the [workspace.lints.*] tables copied from the repository's Cargo.toml and the repository's
# clippy.toml beside it, so it tests the real lint settings, then runs clippy -D warnings on <package>.
#   float_unnamed (SPR-010 GAP-3): plane-A float arithmetic on values that carry no float token.
#   std_transcendental (SPR-003 T-00): the float zone calling std ln and exp instead of libm.
clippy_control() {
  local src="tests/lint_fixtures/clippy/$1" dst="target/rendered-fixtures/$1" lints
  rm -rf "$dst" && mkdir -p "$dst" && cp -R "$src/." "$dst/" && cp clippy.toml "$dst/" || return 1
  lints="$(awk '/^\[workspace\.lints\./ {on=1} /^\[/ && !/^\[workspace\.lints\./ {on=0} on' Cargo.toml)"
  [ -n "$lints" ] || { echo "no [workspace.lints.*] tables in Cargo.toml"; return 1; }
  awk -v lints="$lints" '$0 == "@WORKSPACE_LINTS@" {print lints; next} {print}' "$dst/Cargo.toml.in" > "$dst/Cargo.toml" || return 1
  cargo generate-lockfile --quiet --offline --manifest-path "$dst/Cargo.toml" || return 1
  cargo clippy --offline --locked --quiet --manifest-path "$dst/Cargo.toml" -p "$2" --target-dir target/lint-fixtures -- -D warnings
}

# The fp-contract guard (ADR-0006 item 4: no FMA contraction; VER-SPR-003-TEST GAP-6, narrowed by
# SPR-007 ruling 4). rustc never fuses a * b + c by default, but -C llvm-args=-fp-contract=fast would,
# and no golden vector can see the 1-ulp effect; C compilers may contract unless told not to.
# fp_contract_guard <root> fails if
#   - any *RUSTFLAGS* variable (RUSTFLAGS, CARGO_ENCODED_RUSTFLAGS, CARGO_BUILD_RUSTFLAGS,
#     CARGO_TARGET_*_RUSTFLAGS) mentions fp-contract at all;
#   - any C or C++ compiler variable the cc crate reads holds an fp-contract token other than exactly
#     -ffp-contract=off: *CFLAGS* (CFLAGS, HOST_CFLAGS, TARGET_CFLAGS, CFLAGS_<target>), *CXXFLAGS*,
#     and CC, CXX with their HOST_/TARGET_ and _<target> variants (a compiler given as
#     `CC="gcc -ffp-contract=fast"`; VER-SPR-007-TEST GAP-2);
#   - any .cargo/config or .cargo/config.toml under <root> (target/, .git/ and the lint fixtures
#     excluded) mentions fp-contract on a line that is not a CFLAGS setting, or with any token other
#     than exactly -ffp-contract=off, or in a CFLAGS entry without `force = true` (without it, a CFLAGS
#     already in the environment beats the config; Architect ruling, 2026-10-16). Whole-line TOML
#     comments set nothing and are skipped.
# The one accepted form is the repository's own `CFLAGS = { value = "-ffp-contract=off", ... }`.
FP_CONTRACT_OK="-ffp-contract=off"
# The variable names the guard reads as compiler settings (an extended regex over `compgen -v`).
C_COMPILER_VARS='CFLAGS|CXXFLAGS|^(HOST_|TARGET_)?(CC|CXX)(_.+)?$'
# fp_contract_bad_tokens <text>: prints every fp-contract token of <text> that is not $FP_CONTRACT_OK.
fp_contract_bad_tokens() {
  grep -oE -- "[^[:space:]\"',]*fp-contract[^[:space:]\"',]*" <<<"$1" | grep -vxF -- "$FP_CONTRACT_OK"
}
fp_contract_guard() {
  local root="$1" bad=0 var file line
  for var in $(compgen -v | grep RUSTFLAGS); do
    if grep -q -- 'fp-contract' <<<"${!var}"; then echo "$var sets fp-contract: ${!var}"; bad=1; fi
  done
  for var in $(compgen -v | grep -E "$C_COMPILER_VARS"); do
    if fp_contract_bad_tokens "${!var}" >/dev/null; then echo "$var sets fp-contract other than $FP_CONTRACT_OK: ${!var}"; bad=1; fi
  done
  while IFS= read -r file; do
    while IFS= read -r line; do
      if ! grep -qE '^[[:space:]]*CFLAGS[[:space:]]*=' <<<"$line" || fp_contract_bad_tokens "$line" >/dev/null; then
        echo "$file sets fp-contract other than CFLAGS = $FP_CONTRACT_OK: $line"; bad=1
      elif ! grep -qE 'force[[:space:]]*=[[:space:]]*true' <<<"$line"; then
        echo "$file sets CFLAGS without force = true, so an environment CFLAGS would replace it: $line"; bad=1
      fi
    done < <(grep -- 'fp-contract' "$file" | grep -vE '^[[:space:]]*#')
  done < <(find "$root" \( -path "$root/target" -o -path "$root/.git" -o -path "$root/tests/lint_fixtures" \) -prune \
             -o -type f \( -path '*/.cargo/config' -o -path '*/.cargo/config.toml' \) -print)
  [ $bad -eq 0 ] && echo "no fp-contract in any RUSTFLAGS variable; none but CFLAGS = $FP_CONTRACT_OK in C/C++ compiler variables (CFLAGS, CXXFLAGS, CC, CXX) or .cargo/config under $root"
  return $bad
}
# Its environment controls: the guard with RUSTFLAGS or CFLAGS set as a contracting build would set them.
fp_contract_env_control() {
  RUSTFLAGS="-C llvm-args=-fp-contract=fast" fp_contract_guard .
}
fp_contract_cflags_env_control() {
  CFLAGS="-O2 -ffp-contract=fast" fp_contract_guard .
}
# GAP-2: the same through CXXFLAGS, a CC wrapper and a target-specific CXX.
fp_contract_cxxflags_env_control() {
  CXXFLAGS="-ffp-contract=on" fp_contract_guard .
}
fp_contract_cc_env_control() {
  CC="gcc -ffp-contract=fast" fp_contract_guard .
}
fp_contract_cxx_target_env_control() {
  CXX_x86_64_unknown_linux_gnu="g++ -ffp-contract=fast" fp_contract_guard .
}
# And the accepted value passes through every one of them.
fp_contract_compiler_vars_ok_control() {
  CC="gcc -ffp-contract=off" CXX="g++" CXXFLAGS="-O2 -ffp-contract=off" fp_contract_guard .
}
# The C dependencies really were built with it (SPR-007 ruling 4): the repository's .cargo/config.toml
# sets CFLAGS to exactly $FP_CONTRACT_OK, and the cc build-script output of every C-building crate
# (each `output` file under <target dir>/{debug,release}/build/<crate>-*/) records CFLAGS = Some(that).
C_BUILD_CRATES="libsqlite3-sys zstd-sys"
c_fp_contract_check() {
  local dir="$1" crate out found bad=0
  grep -qxF "CFLAGS = { value = \"$FP_CONTRACT_OK\", force = true }" .cargo/config.toml ||
    { echo ".cargo/config.toml does not set CFLAGS = { value = \"$FP_CONTRACT_OK\", force = true }"; return 1; }
  for crate in $C_BUILD_CRATES; do
    found=0
    for out in "$dir"/debug/build/"$crate"-*/output "$dir"/release/build/"$crate"-*/output; do
      [ -f "$out" ] || continue
      found=1
      if grep -qxF "CFLAGS = Some($FP_CONTRACT_OK)" "$out"; then echo "$out: CFLAGS = Some($FP_CONTRACT_OK)"
      else echo "$out: the C build of $crate did not get CFLAGS = $FP_CONTRACT_OK"; bad=1; fi
    done
    [ $found -eq 1 ] || { echo "no build-script output for $crate under $dir: build first"; bad=1; }
  done
  return $bad
}
# Its control: the same check over a rendered build tree whose cc output lacks the flag.
c_fp_contract_control() {
  local dst=target/rendered-fixtures/c_fp_contract
  rm -rf "$dst" && mkdir -p "$dst/debug/build/libsqlite3-sys-0000/" || return 1
  printf 'cargo:rerun-if-env-changed=CFLAGS\nCFLAGS = None\n' > "$dst/debug/build/libsqlite3-sys-0000/output" || return 1
  c_fp_contract_check "$dst"
}

# Hash golden-vector control (SPR-007 T-01): the committed rh-snapshot golden file with one hash changed.
hash_golden_control() {
  local planted=target/rendered-fixtures/stale_rh_snapshot_hash_v1.txt
  mkdir -p "$(dirname "$planted")" || return 1
  sed '0,/^xxh3_64 0 /s/^xxh3_64 0 .*/xxh3_64 0 0000000000000000/' crates/rh-snapshot/tests/golden/rh_snapshot_hash_v1.txt > "$planted" || return 1
  "$PY" -I bin/gen_hash_golden.py --check --golden "$planted"
}

# Positive controls for the forbidden-API gate (V-DET-19): one planted file per API, holding exactly
# that API, which must be reported under its own rule with its own token (tests/lint_fixtures/README.md).
# Each entry is file:rule:token; a token may itself contain "::".
FORBIDDEN_API_CONTROLS="hash_map:hash-collections:HashMap hash_set:hash-collections:HashSet
  thread_rng:thread-rng:thread_rng thread_rng_type:thread-rng:ThreadRng rand_rng:thread-rng:rand::rng
  std_rng:std-rng:StdRng small_rng:std-rng:SmallRng os_rng:std-rng:OsRng from_entropy:std-rng:from_entropy
  from_os_rng:std-rng:from_os_rng getrandom:std-rng:getrandom rand_random:rand-random:rand::random
  random_range:rand-random:random_range random_bool:rand-random:random_bool
  random_ratio:rand-random:random_ratio random_iter:rand-random:random_iter mul_add:mul-add:mul_add
  instant:wall-clock:Instant::now system_time:wall-clock:SystemTime unsafe:unsafe:unsafe
  spaced_path:wall-clock:time::Instant spaced_rand:thread-rng:rand::rng
  float_type:float:f32 float_literal:float:0.5 float_suffix:float:2e-3f64 float_conversion:float:to_f64
  float_escape_reason:float:f64 float_undeclared_field:float:f32
  allow_disallowed:lint-allow:allow(clippy::disallowed_methods
  path_attr:source-include:#[path include_macro:source-include:include!("
forbidden_api_controls() {
  local - spec file rest rule token tick='`'
  set -f  # tokens such as #[path must not be glob-expanded; `local -` restores the option
  for spec in $FORBIDDEN_API_CONTROLS; do
    file="${spec%%:*}"; rest="${spec#*:}"; rule="${rest%%:*}"; token="${rest#*:}"
    run "Forbidden-API positive control: $file must FAIL" "fail:VIOLATION [V-DET-19/$rule] ${tick}${token}${tick}" \
      "$PY" -I bin/lint_forbidden.py --scan "tests/lint_fixtures/forbidden/planted/$file.rs" || return 1
  done
}

# Golden runs (SPR-008): the release rh-cli, its output directory and the N of "N threads" (at least 2).
RH_CLI_RELEASE="$TARGET_DIR/release/rh-cli"
mkdir -p "$TARGET_DIR/golden"
# Absolute (VER-SPR-008-TEST DEF-3): $CARGO_TARGET_DIR may be absolute or relative, and some controls
# run from another directory. Never join it to $PWD.
GOLDEN="$(cd "$TARGET_DIR/golden" && pwd -P)"
GOLDEN_THREADS=$(( $(nproc 2>/dev/null || echo 2) > 2 ? $(nproc 2>/dev/null || echo 2) : 2 ))
# DEF-2: the floor of the share of scheduler levels that hold two or more jobs (the smoke world
# runs at about 7%: Environment and Activity every day; the gate names say 5%, keep them in step).
PARALLEL_FLOOR=5
# The ladder-compare control: the 1-thread ladder with rung 500's first table root changed must differ there.
ladder_compare_control() {
  local edited="$TARGET_DIR/rendered-fixtures/ladder_one_root.ladder"
  mkdir -p "$TARGET_DIR/rendered-fixtures" || return 1
  awk 'BEGIN{r=0} /^rung 500 /{r=1; print; next} r==1 && /^root /{sub(/[0-9a-f]$/, ($NF ~ /0$/ ? "1" : "0")); r=0} {print}' \
    "$GOLDEN/smoke_yearly_t1.ladder" > "$edited" || return 1
  "$RH_CLI_RELEASE" ladder compare "$GOLDEN/smoke_yearly_t1.ladder" "$edited"
}
# The hand-edited baseline control: the committed baseline with one hex digit of rung 1's hash typed over.
baseline_hand_edited_control() {
  local edited="$TARGET_DIR/rendered-fixtures/baseline_hand_edited.ladder"
  mkdir -p "$TARGET_DIR/rendered-fixtures" || return 1
  awk '/^rung 1 /{n=split($0, w, " "); h=w[5]; c=substr(h,16,1); w[5]=substr(h,1,15) (c=="0" ? "1" : "0"); $0=w[1]; for(i=2;i<=n;i++) $0=$0 " " w[i]} {print}' \
    worlds/smoke/baselines/smoke_yearly_1000y.ladder > "$edited" || return 1
  "$PY" -I bin/check_baseline.py --baseline "$edited" --recomputed "$GOLDEN/smoke_yearly_t1.ladder" --changed no
}

# The push form of the C3 rule (VER-SPR-008 MAJ-1): a two-commit repository whose second commit, with
# subject $1, adds the baseline; the gate runs over before..after there.
baseline_push_control() {
  local repo="$TARGET_DIR/rendered-fixtures/baseline_push" ladder="$GOLDEN/smoke_yearly_t1.ladder"
  local g=(git -C "$repo" -c user.name=check_all -c user.email=check_all@localhost -c commit.gpgsign=false)
  rm -rf "$repo" && mkdir -p "$repo" || return 1
  "${g[@]}" init -q && echo x > "$repo/other.txt" && "${g[@]}" add . && "${g[@]}" commit -qm base || return 1
  local before; before=$("${g[@]}" rev-parse HEAD) || return 1
  cp worlds/smoke/baselines/smoke_yearly_1000y.ladder "$repo/baseline.ladder" && "${g[@]}" add . && "${g[@]}" commit -qm "$1" || return 1
  local after; after=$("${g[@]}" rev-parse HEAD) || return 1
  local py="$PY" script="$PWD/bin/check_baseline.py"
  [[ "$py" == /* || "$py" != */* ]] || py="$PWD/$py"
  (cd "$repo" && "$py" -I "$script" --baseline baseline.ladder --recomputed "$ladder" \
    --event push --before "$before" --after "$after")
}

# MIN-2: two years clean and two years with the V-DET-11 hook; the ladders (which record the hook in
# their header) must differ.
injected_ladder_control() {
  local dir="$TARGET_DIR/rendered-fixtures/injected"
  rm -rf "$dir" && mkdir -p "$dir" || return 1
  "$RH_CLI_RELEASE" run smoke --years 2 --out "$dir/clean.ladder" >/dev/null || return 1
  "$RH_CLI_RELEASE" run smoke --years 2 --out "$dir/injected.ladder" --inject 24:harvest.stock:0 >/dev/null || return 1
  grep -q '^inject 24:harvest.stock:0$' "$dir/injected.ladder" || { echo "no inject line in the header"; return 0; }
  "$RH_CLI_RELEASE" ladder compare "$dir/clean.ladder" "$dir/injected.ladder"
}

# GAP-4: the 1-thread golden ladder with CRLF line ends (crlf) or without its final newline
# (unterminated), compared with itself: the strict parser must refuse it.
ladder_bytes_control() {
  local dir="${TARGET_DIR:?}/rendered-fixtures/ladder_bytes"
  mkdir -p "$dir" || return 1
  case "$1" in
    crlf) sed 's/$/\r/' "$GOLDEN/smoke_yearly_t1.ladder" > "$dir/$1.ladder" || return 1 ;;
    unterminated) head -c -1 "$GOLDEN/smoke_yearly_t1.ladder" > "$dir/$1.ladder" || return 1 ;;
  esac
  "$RH_CLI_RELEASE" ladder compare "$GOLDEN/smoke_yearly_t1.ladder" "$dir/$1.ladder"
}

# GAP-6: the nightly compares the yearly cut of its monthly run with the baseline; here the cut
# of a 3-year monthly run must equal the 3-year yearly run byte for byte.
ladder_cut_control() {
  local dir="${TARGET_DIR:?}/rendered-fixtures/ladder_cut"
  mkdir -p "$dir" || return 1
  "$RH_CLI_RELEASE" run smoke --years 3 --ladder monthly --out "$dir/monthly.ladder" >/dev/null || return 1
  "$RH_CLI_RELEASE" run smoke --years 3 --ladder yearly --out "$dir/yearly.ladder" >/dev/null || return 1
  "$RH_CLI_RELEASE" ladder cut-yearly "$dir/monthly.ladder" "$dir/cut.ladder" || return 1
  "$RH_CLI_RELEASE" ladder compare "$dir/cut.ladder" "$dir/yearly.ladder" || return 1
  cmp "$dir/cut.ladder" "$dir/yearly.ladder" && echo "CUT EQUAL: 3 yearly rungs, byte for byte"
}

# DEF-2: 2 years at 1 and N threads, with the planted thread-identity leak when given
# --leak-thread, then the 1-versus-N comparison. With the leak the header must say so.
thread_leak_control() {
  local dir="${TARGET_DIR:?}/rendered-fixtures/thread_leak"
  rm -rf "$dir" && mkdir -p "$dir" || return 1
  "$RH_CLI_RELEASE" run smoke --years 2 --threads 1 --out "$dir/t1.ladder" "$@" >/dev/null || return 1
  "$RH_CLI_RELEASE" run smoke --years 2 --threads "$GOLDEN_THREADS" --out "$dir/tN.ladder" "$@" >/dev/null || return 1
  if [ "$*" = "--leak-thread" ]; then
    grep -q '^leak thread$' "$dir/t1.ladder" || { echo "no leak line in the header"; return 0; }
  fi
  "$RH_CLI_RELEASE" ladder compare "$dir/t1.ladder" "$dir/tN.ladder"
}

# DEF-1: a 24-year run with keyframes (made once per check_all run), then a run resumed from its
# year-10 keyframe with --years $1 and any further flags; prints the resumed ladder's header.
def1_resume() {
  local dir="${TARGET_DIR:?}/rendered-fixtures/def1" years="$1"; shift
  # Keyed to this check_all run ($$): keyframes of an earlier run, or of another world, are stale.
  if [ "$(cat "$dir/run.id" 2>/dev/null)" != "$$" ] || [ ! -f "$dir/kf/year-0010.rhkf" ]; then
    rm -rf "$dir" && mkdir -p "$dir" && echo "$$" > "$dir/run.id" || return 1
    "$RH_CLI_RELEASE" run smoke --years 24 --out "$dir/full.ladder" --journal "$dir/journal.sqlite" --keyframes "$dir/kf" >/dev/null || return 1
  fi
  "$RH_CLI_RELEASE" run smoke --years "$years" --resume "$dir/kf/year-0010.rhkf" --journal "$dir/journal.sqlite" \
    --out "$dir/resumed.ladder" "$@" || return 1
  head -9 "$dir/resumed.ladder"
}

# Doctests (nextest skips them). The compile-fail cases are acceptance tests (SPR-002 T-01), so the
# gate counts them per file: a case demoted to plain text, deleted or renamed must turn it red, not
# just vanish (VER-SPR-002-TEST DEF-2). Each entry is file:compile_fail:compiling, the number of
# `compile_fail` doctests and of compiling doctests (the twins that show each case fails for its
# stated reason). Update an entry in the same commit that adds or removes a case.
DOCTEST_EXPECT="crates/rh-ids/src/ids.rs:7:7 crates/rh-ids/src/rng.rs:1:1"
DOCTEST_LOG=target/doctests.txt
doctests_run() {
  mkdir -p target && rm -f "$DOCTEST_LOG" || return 1
  cargo test --doc --workspace --locked > "$DOCTEST_LOG" 2>&1
  local rc=$?
  cat "$DOCTEST_LOG"
  return $rc
}
# doctest_count_gate <cargo test --doc output>: fails on any file whose counts differ from DOCTEST_EXPECT.
doctest_count_gate() {
  local log="$1" spec file rest want_cf want_ok got_cf got_ok bad=0
  [ -s "$log" ] || { echo "no doctest output at $log"; return 1; }
  for spec in $DOCTEST_EXPECT; do
    file="${spec%%:*}"; rest="${spec#*:}"; want_cf="${rest%%:*}"; want_ok="${rest#*:}"
    got_cf=$(grep -F -- "test $file - " "$log" | grep -cF -- " - compile fail ... ok")
    got_ok=$(grep -F -- "test $file - " "$log" | grep -F -- ") ... ok" | grep -cvF -- "compile fail")
    if [ "$got_cf" -ne "$want_cf" ]; then echo "$file: expected $want_cf compile-fail doctests, found $got_cf"; bad=1; fi
    if [ "$got_ok" -ne "$want_ok" ]; then echo "$file: expected $want_ok compiling doctests, found $got_ok"; bad=1; fi
    [ "$bad" -eq 0 ] && echo "$file: $got_cf compile-fail and $got_ok compiling doctests, as expected"
  done
  return $bad
}
# The positive control: the real doctest output with one typed-ID compile-fail line removed.
doctest_count_control() {
  local planted=target/rendered-fixtures/doctests_one_missing.txt
  mkdir -p "$(dirname "$planted")" || return 1
  awk '!done && /^test crates\/rh-ids\/src\/ids\.rs - .* - compile fail \.\.\. ok$/ { done = 1; next } { print }' \
    "$DOCTEST_LOG" > "$planted" || return 1
  doctest_count_gate "$planted"
}

# T-01 compile-fail controls for rh-core::fixed (SPR-010): plane-A code that mixes Q8_8, Q16_16 and Money,
# or converts one from or to a float, must not compile, each with its own error; fixed-ok must compile.
# Each entry is crate|expected text.
FIXED_POINT_COMPILE_FAIL='mix-q8-q16|expected `Q8_8`, found `Q16_16`
mix-money-q16|expected `Money`, found `Q16_16`
float-from|expected `Q16_16`, found `f64`
float-into|the trait bound `f64: From<Q16_16>` is not satisfied
float-cast|non-primitive cast: `Money` as `f64`'
fixed_point_compile_controls() {
  local fixture=tests/lint_fixtures/compile_fail/fixed_point/Cargo.toml crate text
  run "rh-core::fixed compile control: well-typed plane-A use compiles" pass \
    cargo check --offline --locked --quiet --manifest-path "$fixture" -p fixed-ok --target-dir target/lint-fixtures || return 1
  while IFS='|' read -r crate text; do
    run "rh-core::fixed compile-fail control: $crate must FAIL" "fail:$text" \
      cargo check --offline --locked --quiet --manifest-path "$fixture" -p "$crate" --target-dir target/lint-fixtures || return 1
  done <<<"$FIXED_POINT_COMPILE_FAIL"
}

# SPR-009 T-02a: the instruction-count gate (V-PERF-04) and its controls. The gate measures benches 7
# and 8 under callgrind (bin/perf_gate.py) and compares them with the committed baseline; a second
# run must give identical counts; a changed baseline must equal the tool's output (ruling 3).
PERF_BASELINE=crates/rh-bench/baselines/instructions_v1.txt
PERF_RUN=artifacts/perf/instructions.txt
PERF_FIXTURES=target/rendered-fixtures/perf
# perf_plant <name> <awk program>: the measured counts of this run, rewritten by <awk program>, as
# $PERF_FIXTURES/<name>.txt (a planted baseline for a control).
perf_plant() {
  mkdir -p "$PERF_FIXTURES" && awk "$2" "$PERF_RUN" > "$PERF_FIXTURES/$1.txt"
}
# A baseline 25% above this run's counts: this run is a speed-up. It passes, and nothing rewrites the
# baseline it was checked against (docs/08 §9: a speed-up never re-baselines automatically).
perf_speedup_control() {
  local before
  perf_plant speedup '$1 == "bench" { $3 = int($3 * 5 / 4) } { print }' || return 1
  before="$(cksum < "$PERF_FIXTURES/speedup.txt")"
  "$PY" -I bin/perf_gate.py check --baseline "$PERF_FIXTURES/speedup.txt" --measured "$PERF_RUN" || return 1
  [ "$(cksum < "$PERF_FIXTURES/speedup.txt")" = "$before" ] || { echo "the check rewrote its baseline"; return 1; }
  echo "speed-up passed; the baseline is unchanged"
}
# A baseline recorded under another valgrind: the gate must say re-baseline, not pass or fail on counts.
perf_toolchain_control() {
  perf_plant toolchain '$1 == "toolchain" && $2 == "valgrind" { $3 = "valgrind-3.21.0" } { print }' || return 1
  "$PY" -I bin/perf_gate.py check --baseline "$PERF_FIXTURES/toolchain.txt" --measured "$PERF_RUN"
}
# perf_integrity_control <case>: ruling 3, both ways. Each case is a baseline that differs from its
# base revision (the base is this run's counts with one count changed):
#   typed     one count typed by hand, 65 off (outside the R-8 band), in a [perf-baseline] PR with
#             a reason: must FAIL
#   inband    one count 40 off: inside the band, passes and is printed (R-8)
#   untagged  regenerated (= the tool's output), in a PR without [perf-baseline]: must FAIL
#   midtag    regenerated, [perf-baseline] mid-title, outside the trailing tag group: must FAIL
#   noreason  regenerated, [perf-baseline], no reason line: must FAIL
#   regen     regenerated, [perf-baseline], with a reason: passes
perf_integrity_control() {
  local title="SPR-009: control [perf-baseline]" body="perf-baseline-reason: the control's" baseline="$PERF_RUN"
  perf_plant base '$1 == "bench" && !done { $3 = $3 + 7; done = 1 } { print }' || return 1
  case "$1" in
    typed) perf_plant typed '$1 == "bench" && $2 == "plane_hashes/state_full/rows_10k" { $3 = $3 + 65 } { print }' || return 1
           baseline="$PERF_FIXTURES/typed.txt" ;;
    inband) perf_plant inband '$1 == "bench" && $2 == "plane_hashes/state_full/rows_10k" { $3 = $3 + 40 } { print }' || return 1
           baseline="$PERF_FIXTURES/inband.txt" ;;
    untagged) title="perf: control" ;;
    midtag) title="docs: explain the [perf-baseline] rule" ;;
    noreason) body="no reason given" ;;
    regen) ;;
    *) echo "unknown case $1"; return 1 ;;
  esac
  RH_PR_TITLE="$title" RH_PR_BODY="$body" "$PY" -I bin/perf_gate.py integrity \
    --baseline "$baseline" --measured "$PERF_RUN" --base-file "$PERF_FIXTURES/base.txt"
}

# SPR-009 T-02b: the wall-clock harness at a smoke size, so it keeps working between adjutant
# sessions. Wall time never gates here (RUN-001 §6); the controls check wall_trend.py's verdicts.
WALL_RUN=artifacts/perf/wall_smoke.txt
wall_smoke() {
  rm -rf "$TARGET_DIR/criterion" || return 1
  RH_WALL_ROWS=1000 cargo bench --locked --quiet -p rh-bench --bench wall -- \
    --warm-up-time 0.05 --measurement-time 0.1 >/dev/null || return 1
  "$PY" -I bin/wall_trend.py summarize --criterion-dir "$TARGET_DIR/criterion" --machine runner --out "$WALL_RUN"
}
# wall_control <case>: a trend file planted from this run's summary.
#   slower   medians 20% below this run's (this run is slower): an ALERT, which --gate fails
#   machine  another CPU model: "machine changed", never a silent comparison
wall_control() {
  local planted="$PERF_FIXTURES/wall_$1.txt"
  mkdir -p "$PERF_FIXTURES" || return 1
  case "$1" in
    slower) awk '$1 == "bench" { $3 = sprintf("%.1f", $3 * 0.8) } { print }' "$WALL_RUN" > "$planted" ;;
    machine) awk '$1 == "condition" && $2 == "cpu" { $0 = "condition cpu Another CPU" } { print }' "$WALL_RUN" > "$planted" ;;
    nan) awk '$1 == "bench" && !done { $3 = "nan"; done = 1 } { print }' "$WALL_RUN" > "$planted" ;;
    *) echo "unknown case $1"; return 1 ;;
  esac
  "$PY" -I bin/wall_trend.py check --trend "$planted" --measured "$WALL_RUN" --gate
}

# SPR-009 GAP-2 (VER-SPR-009-TEST): on a push, a changed perf baseline needs a commit in before..after
# whose subject ends in [perf-baseline] and whose body has a Reason: line. perf_push_control <case>
# builds a scratch repository: the base commit holds a planted baseline, the next commit the tool's
# output, with a tagged subject and a reason (tagged: passes) or neither (untagged: must FAIL).
perf_push_control() {
  local repo="$PERF_FIXTURES/push_$1" subject="SPR-009: re-record the baseline" body="Reason: the control's" c1 c2
  perf_plant base '$1 == "bench" && !done { $3 = $3 + 7; done = 1 } { print }' || return 1
  rm -rf "$repo" && mkdir -p "$repo" || return 1
  local -a g=(git -C "$repo" -c user.name=fixture -c user.email=fixture@localhost -c commit.gpgsign=false)
  "${g[@]}" init -q && cp "$PERF_FIXTURES/base.txt" "$repo/baseline.txt" && "${g[@]}" add . && "${g[@]}" commit -qm base || return 1
  c1="$("${g[@]}" rev-parse HEAD)"
  [ "$1" = tagged ] && subject="$subject [perf-baseline]"
  [ "$1" = untagged ] && body="no reason line"
  cp "$PERF_RUN" "$repo/baseline.txt" && "${g[@]}" add . && "${g[@]}" commit -qm "$subject" -m "$body" || return 1
  c2="$("${g[@]}" rev-parse HEAD)"
  env -u RH_PR_TITLE -u RH_PR_BODY RH_EVENT=push RH_PUSH_BEFORE="$c1" RH_PUSH_AFTER="$c2" \
    "$PY" -I bin/perf_gate.py integrity --repo "$repo" --baseline "$repo/baseline.txt" --measured "$PERF_RUN"
}
# GAP-2: with no pull-request or push context (a local run, workflow_dispatch) the tag and reason are
# not checked, and the gate says so instead of the success wording.
perf_local_control() {
  perf_plant base '$1 == "bench" && !done { $3 = $3 + 7; done = 1 } { print }' || return 1
  # workflow_dispatch, not "local": inside Actions an event the gate does not know is an error.
  env -u RH_PR_TITLE -u RH_PR_BODY -u GITHUB_EVENT_NAME RH_EVENT=workflow_dispatch "$PY" -I bin/perf_gate.py integrity \
    --baseline "$PERF_RUN" --measured "$PERF_RUN" --base-file "$PERF_FIXTURES/base.txt"
}

# SPR-009 T-05: V-PERF-11, the smoke world's heap allocations per steady-state tick (bin/perf_gate.py
# alloc-*). The budget is the recorded count and any increase fails (ruling 5); counts are exact (R-8).
ALLOC_BASELINE=crates/rh-bench/baselines/allocations_v1.txt
ALLOC_RUN=artifacts/perf/allocations.txt
alloc_repeat() {
  "$PY" -I bin/perf_gate.py alloc-measure --out artifacts/perf/allocations_run2.txt >/dev/null || return 1
  cmp "$ALLOC_RUN" artifacts/perf/allocations_run2.txt && echo "identical allocation counts in two runs"
}
# A hand-typed allocation baseline (one allocation more), in a [perf-baseline] PR with a reason: must FAIL.
alloc_typed_control() {
  mkdir -p "$PERF_FIXTURES" || return 1
  awk '$1 == "budget" && $3 == "1" && $6 == "allocations" && !done { $7 = $7 + 1; done = 1 } { print }' "$ALLOC_RUN" > "$PERF_FIXTURES/alloc_typed.txt" || return 1
  RH_PR_TITLE="SPR-009: control [perf-baseline]" RH_PR_BODY="perf-baseline-reason: control" "$PY" -I bin/perf_gate.py integrity \
    --exact --baseline "$PERF_FIXTURES/alloc_typed.txt" --measured "$ALLOC_RUN" --base-file "$ALLOC_BASELINE"
}
# Review 7: an allocation baseline recorded under another rustc must report re-baseline required (exit 3),
# never OVER BUDGET or a pass.
alloc_toolchain_control() {
  mkdir -p "$PERF_FIXTURES" || return 1
  awk '$1 == "toolchain" && $2 == "rustc" { $0 = "toolchain rustc 1.96.0 0000000000000000000000000000000000000000" } { print }' \
    "$ALLOC_RUN" > "$PERF_FIXTURES/alloc_toolchain.txt" || return 1
  "$PY" -I bin/perf_gate.py alloc-check --baseline "$PERF_FIXTURES/alloc_toolchain.txt" --measured "$ALLOC_RUN"
}

# SPR-009 T-03, folded into the N-thread golden run (review 2c): the 1,000-year run at N threads with
# the host's timing observer attached prints the vitals line; its ladder then goes through the same
# gates as the 1-thread run (= the golden baseline, = the 1-thread ladder), so the observer changes
# no history at N threads (GOV-004 law 2; review finding 7).
golden_vitals_run() {
  local out
  out="$("$RH_CLI_RELEASE" run smoke --years 1000 --threads "$GOLDEN_THREADS" --ladder yearly \
    --require-parallel "$PARALLEL_FLOOR" --vitals --out "$GOLDEN/smoke_yearly_tN.ladder")" || { echo "$out"; return 1; }
  echo "$out" | grep -v '^vitals: system '
  grep -q '^vitals: tip .* | pace .* ticks/s | serial .* | state ' <<<"$out" || { echo "no vitals line"; return 1; }
}

# The debug workspace run leaves out the test binaries a by-name gate runs (review 2a): each is run
# once, by the gate that checks its output. Entries are nextest binary IDs: <crate>::<test target>,
# or <crate> for its library's unit tests (`--lib`). rh-golden::golden runs in release only (golden
# part). test_check_all_inventory.py proves every entry is run by a by-name gate.
BY_NAME_TESTS="rh-snapshot::hash rh-snapshot::golden rh-snapshot::journal rh-snapshot::keyframe
  rh-snapshot::guard rh-snapshot::stub_world rh-snapshot::unsynced_guard rh-snapshot::observer
  rh-ids::fork_trees rh-ids::stream_independence rh-sample::golden rh-sample::stats
  rh-core::determinism rh-core::registration rh-registry::real_specs rh-core::phase_table
  rh-core::time_properties rh-core::system_id_golden rh-core::engine_contract
  rh-core::agenda rh-core::agenda_events rh-core::invariants rh-core rh-events
  rh-sys-smoke::smoke rh-golden::golden"
workspace_tests() {
  local id filter=""
  for id in $BY_NAME_TESTS; do filter="${filter:+$filter | }binary_id($id)"; done
  echo "left to the by-name gates: $filter"
  cargo nextest run --workspace --locked --no-fail-fast --status-level fail -E "not ($filter)"
}
# Its control: a typo in an entry must fail, never silently exclude nothing.
workspace_tests_typo_control() {
  BY_NAME_TESTS="rh-snapshot::no_such_test" workspace_tests
}

# SPR-009 review 1: the Python suite with hostile CI variables set must still pass (conftest.py clears
# every RH_* and GITHUB_* variable before each test).
pytest_hostile_env() {
  RH_EVENT=pull_request RH_PR_TITLE='x [perf-baseline] [C3]' RH_PR_BODY='perf-baseline-reason: hostile' \
    GITHUB_EVENT_NAME=push GITHUB_ACTIONS=true RH_PUSH_BEFORE=garbage RH_PUSH_AFTER=garbage \
    RH_BASELINE_BASE=origin/no-such-branch "$PY" -I -m pytest -c python/pyproject.toml --rootdir=. -q
}

# The checks: ONE fail-fast chain. Define every function above this line; a definition placed between
# two `&&` lines splits the chain, and later checks would run after a failure (DEF-2).
# The verdict (VER-SPR-008-TEST GAP-3, GAP-7), computed in an EXIT trap so that no exit, early or
# late, ends the tier without one: a gate line ending in `|| true` or `|| exit 0`, or a stray exit,
# cannot turn failed or unrun gates into a pass. The tier passes only when the chain ran to its end,
# no gate failed and the chain's last status is 0. bin/check_all_selftest.py runs this code on
# planted chains; its lint refuses `|| true`, `|| :` and `|| exit` on a run line.
finish() {
  local status=$? verdict
  if [ -n "${chain_done:-}" ] && [ -z "$failed" ] && [ "$status" -eq 0 ]; then
    verdict="PASS ($n checks${PART:+, part $PART})"; status=0
  elif [ -n "$failed" ]; then
    verdict="FAIL at \"$failed\" (later checks skipped: fail-fast)"; status=1
  elif [ -z "${chain_done:-}" ]; then
    verdict="FAIL (the chain exited early, status $status, after $n checks: later gates unrun)"; status=1
  else
    verdict="FAIL (the chain stopped with no gate named)"; status=1
  fi
  { echo; echo "## Verdict: $verdict"; } >> "$REPORT"
  echo "Report: $REPORT"
  exit "$status"
}
chain_done=""
trap finish EXIT

part core &&
run "Required tools present (core part)" pass require_tools core &&
# Tier 1 — static and contract lints, fastest first. The Python checks (ruff, mypy, pytest, the
# hostile-CI pytest) run before any Rust build, so a Python failure shows in about a minute (GAP-18).
run "Document index builds (gen_manifest --check)" pass "$PY" -I bin/gen_manifest.py --check &&
run "CI workflows parse (YAML)" pass "$PY" -I -c 'import sys, yaml; [yaml.safe_load(open(f, encoding="utf-8")) for f in sys.argv[1:]]' .github/workflows/ci.yml .github/workflows/advisories.yml .github/workflows/nightly.yml .github/workflows/adjutant.yml &&
run "No FMA contraction: no fp-contract in RUSTFLAGS; C builds take exactly -ffp-contract=off (SPR-003 GAP-6; SPR-007 ruling 4)" pass fp_contract_guard . &&
run "fp-contract guard positive control: RUSTFLAGS with fp-contract must FAIL" "fail:RUSTFLAGS sets fp-contract" fp_contract_env_control &&
run "fp-contract guard positive control: a .cargo/config.toml with fp-contract must FAIL" "fail:.cargo/config.toml sets fp-contract" fp_contract_guard tests/lint_fixtures/fp_contract &&
run "fp-contract guard positive control: CFLAGS with -ffp-contract=fast must FAIL (SPR-007 ruling 4)" "fail:CFLAGS sets fp-contract other than -ffp-contract=off" fp_contract_cflags_env_control &&
run "fp-contract guard positive control: CXXFLAGS with -ffp-contract=on must FAIL (SPR-007 GAP-2)" "fail:CXXFLAGS sets fp-contract other than -ffp-contract=off" fp_contract_cxxflags_env_control &&
run "fp-contract guard positive control: CC with -ffp-contract=fast must FAIL (SPR-007 GAP-2)" "fail:CC sets fp-contract other than -ffp-contract=off" fp_contract_cc_env_control &&
run "fp-contract guard positive control: a target-specific CXX with -ffp-contract=fast must FAIL (SPR-007 GAP-2)" "fail:CXX_x86_64_unknown_linux_gnu sets fp-contract other than -ffp-contract=off" fp_contract_cxx_target_env_control &&
run "fp-contract guard control: CC, CXX and CXXFLAGS with exactly -ffp-contract=off pass (SPR-007 GAP-2)" "pass:no fp-contract in any RUSTFLAGS variable" fp_contract_compiler_vars_ok_control &&
run "fp-contract guard positive control: a .cargo/config.toml CFLAGS with -ffp-contract=fast must FAIL (SPR-007 ruling 4)" "fail:.cargo/config.toml sets fp-contract other than CFLAGS = -ffp-contract=off" fp_contract_guard tests/lint_fixtures/fp_contract_cflags &&
run "fp-contract guard positive control: a .cargo/config.toml CFLAGS without force = true must FAIL (SPR-007 ruling 4)" "fail:.cargo/config.toml sets CFLAGS without force = true" fp_contract_guard tests/lint_fixtures/fp_contract_cflags_unforced &&
run "fp-contract guard control: a .cargo/config.toml with exactly CFLAGS = -ffp-contract=off passes (SPR-007 ruling 4)" "pass:none but CFLAGS = -ffp-contract=off" fp_contract_guard tests/lint_fixtures/fp_contract_cflags_ok &&
run "Python: uv.lock matches pyproject.toml (uv lock --check)" pass uv lock --check --offline --project python &&
run "uv lock --check positive control: drifted lock must FAIL" "fail:needs to be updated" uv lock --check --offline --project tests/lint_fixtures/uv_drift &&
run "Python: lint (ruff check)" pass "$PY" -I -m ruff check --config python/pyproject.toml --no-cache . &&
run "Python: format (ruff format --check)" pass "$PY" -I -m ruff format --config python/pyproject.toml --no-cache --check . &&
run "Python: types (mypy --strict)" pass "$PY" -I -m mypy --config-file python/pyproject.toml &&
run "Python: unit tests (pytest)" pass "$PY" -I -m pytest -c python/pyproject.toml --rootdir=. -q &&
run "Python: unit tests with hostile CI variables set (RH_EVENT, RH_PR_TITLE, GITHUB_EVENT_NAME, RH_PUSH_*) pass: conftest.py isolates them (SPR-009 review 1)" pass pytest_hostile_env &&
run "Rust: format (cargo fmt --check)" pass cargo fmt --all --check &&
run "Rust: bans and sources (cargo deny: crates.io only, no wildcards)" pass cargo deny --color never --locked check bans sources &&
run "cargo deny positive control: git source must FAIL" "fail:error[source-not-allowed]" git_source_control &&
run "cargo deny positive control: wildcard version must FAIL" "fail:error[wildcard]" cargo deny --color never --offline --locked --manifest-path tests/lint_fixtures/deny/wildcard_version/Cargo.toml check bans &&
run "Licence ledger, not a gate (artifacts/licence_ledger.txt; ADR-0021)" pass licence_ledger Cargo.toml artifacts/licence_ledger.txt &&
run "Licence ledger control: the GPL fixture is listed as GPL-3.0-only" "pass:GPL-3.0-only" licence_ledger tests/lint_fixtures/deny/gpl_dependency/Cargo.toml target/rendered-fixtures/licence_ledger_gpl.txt &&
run "Architecture: crate boundaries and planes (lint_arch; V-INSP-03, V-PLN-01)" pass "$PY" -I bin/lint_arch.py &&
run "lint_arch positive control: a #[global_allocator] outside rh-bench's benches must FAIL (SPR-009 T-05)" "fail:VIOLATION [global-allocator] rh-tool: src/lib.rs declares a #[global_allocator]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/global_allocator_elsewhere/Cargo.toml &&
run "lint_arch positive control: a #[global_allocator] under a source directory named target must FAIL (VER-SPR-009-TEST GAP-21)" "fail:VIOLATION [global-allocator] rh-tool: src/target/alloc.rs declares a #[global_allocator]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/global_allocator_src_target/Cargo.toml &&
run "lint_arch positive control: a #[global_allocator] behind cfg_attr must FAIL (SPR-009 GAP-6)" "fail:VIOLATION [global-allocator] rh-tool: src/lib.rs declares a #[global_allocator]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/global_allocator_cfg_attr/Cargo.toml &&
run "lint_arch positive control: a crate enabling rh-bench/count-alloc must FAIL (SPR-009 T-05)" "fail:VIOLATION [count-alloc] rh-tool: enables rh-bench/count-alloc" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/count_alloc_enabled/Cargo.toml &&
run "lint_arch positive control: a plane-A crate enabling rh-bench/arena-alloc must FAIL (SPR-009 R-7)" "fail:VIOLATION [count-alloc] rh-sys-x: enables rh-bench/arena-alloc" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/arena_alloc_plane_a/Cargo.toml &&
run "lint_arch positive control: a test target requiring arena-alloc must FAIL (SPR-009 R-7)" "fail:VIOLATION [count-alloc] rh-bench: a [test] target requires arena-alloc" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/arena_alloc_test_target/Cargo.toml &&
run "lint_arch control: a bench target requiring arena-alloc, with its allocator in benches/, passes (SPR-009 R-7)" "pass:0 violations" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/arena_alloc_bench_ok/Cargo.toml &&
run "lint_arch positive control: a non-bench target requiring count-alloc must FAIL (SPR-009 T-05)" "fail:VIOLATION [count-alloc] rh-bench: a [bin] target requires count-alloc" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/count_alloc_bin/Cargo.toml &&
run "lint_arch positive control: unsafe_code relaxed by a crate not on the allow-list must FAIL (GOV-003 §2; SPR-009 T-05)" "fail:VIOLATION [unsafe-allow] rh-tool: declares unsafe_code = 'allow'" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/unsafe_unlisted/Cargo.toml &&
run "lint_arch positive control: an allow-listed crate whose lints drift from the workspace's must FAIL (SPR-009 T-05)" "fail:VIOLATION [unsafe-allow] rh-bench: its own [lints] must equal [workspace.lints]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/unsafe_drift/Cargo.toml &&
run "lint_arch positive control: unsafe outside the allow-listed file must FAIL (SPR-009 T-05)" "fail:VIOLATION [unsafe-allow] rh-bench: src/lib.rs holds unsafe code" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/unsafe_outside_file/Cargo.toml &&
run "lint_arch positive control: system -> system must FAIL" "fail:VIOLATION [V-INSP-03]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/system_to_system/Cargo.toml &&
run "lint_arch positive control: system -> rh-snapshot must FAIL" "fail:VIOLATION [V-INSP-03]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/system_to_disallowed/Cargo.toml &&
run "lint_arch positive control: system -> rh-snapshot as a build-dependency must FAIL" "fail:VIOLATION [V-INSP-03] rh-sys-a: depends on rh-snapshot (as a build-dependency)" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/system_build_dependency/Cargo.toml &&
run "lint_arch positive control: system -> rh-snapshot aliased as rh-core must FAIL" "fail:VIOLATION [V-INSP-03] rh-sys-a: depends on rh-snapshot;" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/system_package_alias/Cargo.toml &&
run "lint_arch positive control: plane A -> plane B must FAIL" "fail:VIOLATION [V-PLN-01]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_to_b/Cargo.toml &&
run "lint_arch positive control: plane A -> plane C must FAIL" "fail:VIOLATION [V-PLN-01]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_to_c/Cargo.toml &&
run "lint_arch positive control: missing plane/kind must FAIL" "fail:VIOLATION [metadata]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/missing_metadata/Cargo.toml &&
run "lint_arch positive control: relabelled rh-snapshot must FAIL" "fail:VIOLATION [metadata] rh-snapshot: declares (plane, kind) = ('A', 'contract'), but rh-snapshot is pinned" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/relabelled_snapshot/Cargo.toml &&
run "lint_arch positive control: kind contract outside rh-contract-* must FAIL" 'fail:VIOLATION [metadata] rh-shadow: kind "contract" belongs only to' "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/contract_name/Cargo.toml &&
run "lint_arch positive control: rh-sample relabelled out of plane A must FAIL (SPR-003 T-00)" "fail:VIOLATION [metadata] rh-sample: declares (plane, kind) = ('tool', 'tool'), but rh-sample is pinned" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/relabelled_sample/Cargo.toml &&
run "lint_arch control: a system may depend on rh-sample (SPR-003 T-00)" pass "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/system_to_sample/Cargo.toml &&
run "lint_arch positive control: plane A -> rand must FAIL" "fail:VIOLATION [rng-crate] rh-core: plane-A crate depends on rand;" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_rng_crate/Cargo.toml &&
run "lint_arch positive control: rand_chacha outside rh-ids must FAIL" "fail:VIOLATION [rng-crate] rh-core: depends on rand_chacha;" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/rng_building_block_outside_ids/Cargo.toml &&
run "lint_arch positive control: plane A -> unreviewed third-party crate must FAIL" "fail:VIOLATION [plane-A-external] rh-core: depends on itoa, which is not on the reviewed list" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_unlisted_dep/Cargo.toml &&
run "lint_arch positive control: libm outside rh-sample must FAIL" "fail:VIOLATION [plane-A-external] rh-core: depends on libm, which plane A may use only in rh-sample" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_scoped_dep_outside/Cargo.toml &&
run "lint_arch positive control: plane A -> rh-manifest (tool) must FAIL (SPR-004)" "fail:VIOLATION [plane-A-closure] rh-registry: plane-A crate depends on plane-\"tool\" crate rh-manifest" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_to_manifest/Cargo.toml &&
run "lint_arch positive control: rh-manifest relabelled as plane A must FAIL (SPR-004)" "fail:VIOLATION [metadata] rh-manifest: declares (plane, kind) = ('A', 'machinery'), but rh-manifest is pinned" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/relabelled_manifest/Cargo.toml &&
run "lint_arch positive control: a crate-local clippy.toml must FAIL (SPR-003 GAP-12)" "fail:VIOLATION [clippy-config] crates/rh-sample/clippy.toml: a clippy configuration other than the workspace root's" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/clippy_config/crate_local/Cargo.toml &&
run "lint_arch positive control: a symlinked workspace member must FAIL (SPR-003 GAP-13)" "fail:VIOLATION [clippy-config] rh-sample: its manifest directory" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/clippy_config/symlinked_member/Cargo.toml &&
run "lint_arch positive control: a clippy.toml reached through a symlinked member must FAIL (SPR-003 GAP-13)" "fail:VIOLATION [clippy-config] node_modules/rh-sample/clippy.toml: a clippy configuration other than" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/clippy_config/symlinked_member/Cargo.toml &&
run "lint_arch positive control: own [lints.clippy] table in rh-sample must FAIL (SPR-003 GAP-9)" "fail:VIOLATION [lints] rh-sample: declares its own lint table(s) [lints.clippy]" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_own_lints/Cargo.toml &&
run "lint_arch positive control: rh-sample without [lints] workspace = true must FAIL (SPR-003 GAP-9)" "fail:VIOLATION [lints] rh-sample: does not declare \`[lints] workspace = true\`" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_no_workspace_lints/Cargo.toml &&
run "lint_arch positive control: libm not pinned exactly must FAIL (SPR-003 GAP-4)" 'fail:VIOLATION [plane-A-external] rh-sample: requires libm as `^0.2`; a plane-A third-party dependency is pinned exactly' "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_loose_pin/Cargo.toml &&
run "lint_arch positive control: libm with default features must FAIL (SPR-003 GAP-4)" "fail:VIOLATION [plane-A-external] rh-sample: uses libm with its default features" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_default_features/Cargo.toml &&
run "lint_arch positive control: libm with feature arch must FAIL (SPR-003 GAP-10)" "fail:VIOLATION [plane-A-external] rh-sample: enables feature(s) arch of libm; the review allows none" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_libm_features/Cargo.toml &&
run "lint_arch positive control: a second, target-specific libm entry must FAIL (SPR-003 GAP-10)" 'fail:VIOLATION [plane-A-external] rh-sample: requires libm (for target cfg(unix)) as `^0.2`' "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_libm_target_edge/Cargo.toml &&
run "lint_arch positive control: a third-party crate in rh-sample must FAIL (SPR-003 GAP-4)" "fail:VIOLATION [plane-A-external] rh-sample: depends on zerocopy, which is not on the reviewed list" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_sample_third_party/Cargo.toml &&
run "V-PLN-01 compile-fail fixture has a clean dependency graph" pass "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/compile_fail/plane_a_names_plane_b/Cargo.toml &&
run "V-PLN-01 compile-fail control: plane A naming a plane-B type must FAIL" 'fail:unlinked crate `fixture_observation`' cargo check --offline --locked --quiet --manifest-path tests/lint_fixtures/compile_fail/plane_a_names_plane_b/Cargo.toml --target-dir target/lint-fixtures &&
run "V-PLN-01 compile-fail fixture (plane C) has a clean dependency graph" pass "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/compile_fail/plane_a_names_plane_c/Cargo.toml &&
run "V-PLN-01 compile-fail control: plane A naming a plane-C type must FAIL" 'fail:unlinked crate `fixture_narrative`' cargo check --offline --locked --quiet --manifest-path tests/lint_fixtures/compile_fail/plane_a_names_plane_c/Cargo.toml --target-dir target/lint-fixtures &&
fixed_point_compile_controls &&
run "Forbidden APIs in plane A (lint_forbidden; V-DET-19)" pass "$PY" -I bin/lint_forbidden.py &&
forbidden_api_controls &&
run "lint_forbidden positive control: workspace plane-A crate must FAIL" "fail:VIOLATION [V-DET-19/hash-collections] \`HashMap\` at tests/lint_fixtures/forbidden/workspace/fixture-core/" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/workspace/Cargo.toml &&
run "lint_forbidden positive control: float threshold in a plane-A crate must FAIL (SPR-010 T-04)" "fail:VIOLATION [V-DET-19/float] \`f32\` at tests/lint_fixtures/forbidden/float_workspace/fixture-sys/src/lib.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_workspace/Cargo.toml &&
run "lint_forbidden positive control: a float outside rh-sample must FAIL (SPR-003 T-00)" "fail:VIOLATION [V-DET-19/float] \`f64\` at tests/lint_fixtures/forbidden/float_zone/rh-sampler/src/lib.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_zone/Cargo.toml &&
run "lint_forbidden positive control: a float in a crate named rh-core must FAIL (SPR-003 GAP-3)" "fail:VIOLATION [V-DET-19/float] \`f64\` at tests/lint_fixtures/forbidden/float_zone_core/rh-core/src/lib.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_zone_core/Cargo.toml &&
run "lint_forbidden control: a float in rh-sample passes (SPR-003 T-00)" "pass:float rule exempts rh-sample only (SPR-003 float zone); 1 files scanned; 0 violations" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_zone_ok/Cargo.toml &&
run "lint_forbidden positive control: mul_add in rh-sample must FAIL (SPR-003 T-00)" "fail:VIOLATION [V-DET-19/mul-add] \`mul_add\` at tests/lint_fixtures/forbidden/float_zone_other_rules/rh-sample/src/lib.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_zone_other_rules/Cargo.toml &&
run "lint_forbidden float-api positive control: fn_return in rh-sample's public API must FAIL (SPR-003 GAP-1)" "fail:VIOLATION [V-DET-19/float-api] \`f64\` at tests/lint_fixtures/forbidden/float_api/rh-sample/src/fn_return.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_api/Cargo.toml &&
run "lint_forbidden float-api positive control: a private float alias in rh-sample must FAIL (SPR-003 GAP-11)" "fail:VIOLATION [V-DET-19/float-api] \`f64\` at tests/lint_fixtures/forbidden/float_api/rh-sample/src/alias.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_api/Cargo.toml &&
run "lint_forbidden float-api positive control: an imported float alias in rh-sample must FAIL (SPR-003 GAP-11b)" "fail:VIOLATION [V-DET-19/float-api] \`f64\` at tests/lint_fixtures/forbidden/float_api/rh-sample/src/import_alias.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_api/Cargo.toml &&
run "lint_forbidden float-api positive control: field in rh-sample's public API must FAIL (SPR-003 GAP-1)" "fail:VIOLATION [V-DET-19/float-api] \`f64\` at tests/lint_fixtures/forbidden/float_api/rh-sample/src/field.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_api/Cargo.toml &&
run "lint_forbidden float-api positive control: reexport in rh-sample's public API must FAIL (SPR-003 GAP-1)" "fail:VIOLATION [V-DET-19/float-api] \`libm\` at tests/lint_fixtures/forbidden/float_api/rh-sample/src/reexport.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_api/Cargo.toml &&
run "lint_forbidden float-api positive control: impl_from in rh-sample's public API must FAIL (SPR-003 GAP-1)" "fail:VIOLATION [V-DET-19/float-api] \`f64\` at tests/lint_fixtures/forbidden/float_api/rh-sample/src/impl_from.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_api/Cargo.toml &&
run "lint_forbidden float-dep positive control: accessor in rh-snapshot must FAIL (SPR-007 GAP-1)" "fail:VIOLATION [V-DET-19/float-dep] \`as_f64_or_null\` at tests/lint_fixtures/forbidden/float_dep/rh-snapshot/src/accessor.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_dep/Cargo.toml &&
run "lint_forbidden float-dep positive control: arrow_float in rh-snapshot must FAIL (SPR-007 T-02)" "fail:VIOLATION [V-DET-19/float-dep] \`Float64Array\` at tests/lint_fixtures/forbidden/float_dep/rh-snapshot/src/arrow_float.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_dep/Cargo.toml &&
run "lint_forbidden float-dep positive control: real_bits in rh-snapshot must FAIL (SPR-007 GAP-1)" "fail:VIOLATION [V-DET-19/float-dep] \`Real\` at tests/lint_fixtures/forbidden/float_dep/rh-snapshot/src/real_bits.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_dep/Cargo.toml &&
run "lint_forbidden float-dep positive control: real_cmp in rh-snapshot must FAIL (SPR-007 GAP-1)" "fail:VIOLATION [V-DET-19/float-dep] \`Real\` at tests/lint_fixtures/forbidden/float_dep/rh-snapshot/src/real_cmp.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_dep/Cargo.toml &&
run "lint_forbidden float-dep positive control: real_floor in rh-snapshot must FAIL (SPR-007 GAP-1)" "fail:VIOLATION [V-DET-19/float-dep] \`Real\` at tests/lint_fixtures/forbidden/float_dep/rh-snapshot/src/real_floor.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_dep/Cargo.toml &&
run "lint_forbidden control: a declared f32_field escape is accepted and counted (SPR-010 T-04)" "pass:0 violations; 1 escapes; 1 declared f32_field columns" "$PY" -I bin/lint_forbidden.py --scan tests/lint_fixtures/forbidden/float_field/declared.rs --specs tests/lint_fixtures/forbidden/float_field/specs &&
run "lint_forbidden control: a literal in-repository include! of a clean skeleton is followed and passes (SPR-008 T-00)" "pass:2 files scanned; 0 violations; 0 escapes; 0 declared f32_field columns; 1 literal include! followed" "$PY" -I bin/lint_forbidden.py --scan tests/lint_fixtures/forbidden/include/clean.rs &&
run "lint_forbidden positive control: a forbidden API in an included skeleton must FAIL (SPR-008 T-00)" "fail:VIOLATION [V-DET-19/hash-collections] \`HashMap\` at tests/lint_fixtures/forbidden/include/skeletons/dirty_skeleton.rs:5:" "$PY" -I bin/lint_forbidden.py --scan tests/lint_fixtures/forbidden/include/dirty.rs &&
run "lint_forbidden positive control: a forbidden API behind a nested include! must FAIL (SPR-008 T-00)" "fail:VIOLATION [V-DET-19/hash-collections] \`HashMap\` at tests/lint_fixtures/forbidden/include/skeletons/dirty_skeleton.rs:5:" "$PY" -I bin/lint_forbidden.py --scan tests/lint_fixtures/forbidden/include/nested.rs &&
run "lint_forbidden positive control: include!(concat!(...)) must FAIL (SPR-008 T-00)" "fail:VIOLATION [V-DET-19/source-include] \`include!(\` at tests/lint_fixtures/forbidden/include/concat.rs:5:" "$PY" -I bin/lint_forbidden.py --scan tests/lint_fixtures/forbidden/include/concat.rs &&
run "lint_forbidden positive control: an include! outside the repository must FAIL (SPR-008 T-00)" "fail:resolves outside the repository" "$PY" -I bin/lint_forbidden.py --scan tests/lint_fixtures/forbidden/include/outside.rs &&
run "lint_forbidden positive control: an include! reached through a symlink resolves from the link, as rustc does (VER-SPR-008 MIN-3)" "fail:VIOLATION [V-DET-19/hash-collections] \`HashMap\` at tests/lint_fixtures/forbidden/include/symlink/gen.rs:3:" "$PY" -I bin/lint_forbidden.py --scan tests/lint_fixtures/forbidden/include/symlink/link.rs &&
run "lint_forbidden positive control: a use that renames the include macro must FAIL (VER-SPR-008)" "fail:VIOLATION [V-DET-19/source-include] \`use core::include\` at tests/lint_fixtures/forbidden/include/renamed.rs:3:" "$PY" -I bin/lint_forbidden.py --scan tests/lint_fixtures/forbidden/include/renamed.rs &&
run "lint_forbidden positive control: include_str! of a file outside the repository must FAIL (VER-SPR-008)" "fail:VIOLATION [V-DET-19/data-include] \`include_str!(\` at tests/lint_fixtures/forbidden/include/data_outside.rs:3:" "$PY" -I bin/lint_forbidden.py --scan tests/lint_fixtures/forbidden/include/data_outside.rs &&
run "Clippy float backstop positive control: unnamed float arithmetic in plane A must FAIL (SPR-010 GAP-3)" "fail:floating-point arithmetic detected" clippy_control float_unnamed fixture-core &&
run "Clippy libm-only positive control: std ln in rh-sample must FAIL (SPR-003 T-00)" 'fail:use of a disallowed method `f64::ln`' clippy_control std_transcendental rh-sample &&
run "Clippy libm-only positive control: std exp in rh-sample must FAIL (SPR-003 T-00)" 'fail:use of a disallowed method `f64::exp`' clippy_control std_transcendental rh-sample &&
run "System specs (lint_odd)" pass "$PY" -I specs/systems/lint_odd.py &&
run "lint_odd emits positive control: a frozen spec emitting a non-verb must FAIL (SPR-011 ruling 11)" "fail:ERROR M1.regions.odd.toml: emits NOT_A_VERB, which is not a Registry verb" "$PY" -I specs/systems/lint_odd.py --extra-specs tests/lint_fixtures/odd_emits/frozen &&
run "lint_odd emits control: a draft spec emitting a non-verb warns and passes (SPR-011 ruling 11)" "pass:WARN  M1.regions.odd.toml: emits SUCCEED, which the Registry gives to M7.dynasty" "$PY" -I specs/systems/lint_odd.py --extra-specs tests/lint_fixtures/odd_emits/draft &&
run "lint_odd emits control: a Registry overlay declares a world's own event kinds (SPR-011 ruling 11)" "pass:PASS: 7 system specs linted" "$PY" -I specs/systems/lint_odd.py --extra-specs tests/lint_fixtures/odd_emits/overlay --overlay tests/lint_fixtures/odd_emits/overlay.toml &&
run "lint_odd applies positive control: a sender's payload fields differ from [[applies]], frozen, must FAIL (REV-P1a section 7)" "fail:has payload fields ['a', 'c', 'b'], but [[applies]] declares ['a', 'b', 'c']" "$PY" -I specs/systems/lint_odd.py --extra-specs tests/lint_fixtures/odd_applies/mismatch &&
run "lint_odd applies control: a sender's payload matching [[applies]] field for field passes (REV-P1a section 7)" "pass:PASS: 7 system specs linted" "$PY" -I specs/systems/lint_odd.py --extra-specs tests/lint_fixtures/odd_applies/match &&
run "Generated skeletons: committed files = lint_odd.py --rust (gen_skeletons --check; SPR-005 T-03)" "pass:up to date" "$PY" -I bin/gen_skeletons.py --check &&
run "gen_skeletons positive control: hand-edited skeleton must FAIL" "fail:STALE: tests/lint_fixtures/skeletons/stale/M2_demography.rs" "$PY" -I bin/gen_skeletons.py --check --out tests/lint_fixtures/skeletons/stale &&
run "Smoke-world specs (lint_odd --specs worlds/smoke/systems --overlay; SPR-008 ruling 2)" "pass:PASS: 7 system specs linted" "$PY" -I specs/systems/lint_odd.py --specs worlds/smoke/systems --overlay worlds/smoke/registry_overlay.toml &&
run "lint_odd positive control: a smoke overlay id that is not X-prefixed must FAIL (SPR-008 ruling 2)" "fail:overlay system 'S9.smoke_chronicle' must be X-prefixed" "$PY" -I specs/systems/lint_odd.py --specs worlds/smoke/systems --overlay tests/lint_fixtures/smoke/overlay_not_x.toml &&
run "lint_odd positive control: a smoke spec missing from the overlay must FAIL (SPR-008 ruling 2)" "fail:system 'X6.smoke_chronicle' is not in the overlay" "$PY" -I specs/systems/lint_odd.py --specs worlds/smoke/systems --overlay tests/lint_fixtures/smoke/overlay_missing.toml &&
run "Smoke-world skeletons: committed files = lint_odd.py --rust (gen_skeletons --check; SPR-008 ruling 2)" "pass:SKELETONS: 7 up to date in crates/rh-sys-smoke/src/generated" "$PY" -I bin/gen_skeletons.py --specs worlds/smoke/systems --overlay worlds/smoke/registry_overlay.toml --out crates/rh-sys-smoke/src/generated --check &&
run "gen_skeletons positive control: a hand-edited smoke skeleton must FAIL (SPR-008 ruling 2)" "fail:STALE: tests/lint_fixtures/skeletons/smoke_stale/X3_smoke_vital.rs" "$PY" -I bin/gen_skeletons.py --specs worlds/smoke/systems --overlay worlds/smoke/registry_overlay.toml --out tests/lint_fixtures/skeletons/smoke_stale --check &&
run "Sifting patterns (lint_patterns)" pass "$PY" -I specs/sifting/lint_patterns.py specs/sifting/patterns.example.toml specs/manifest/registry_v0.toml &&
run "Sifting patterns: solitary_life pack, life_ends (lint_patterns; SPR-011 ruling 14)" "pass:PASS: 1 patterns linted." "$PY" -I specs/sifting/lint_patterns.py specs/sifting/patterns.solitary_life.toml specs/manifest/registry_v0.toml &&
run "lint_patterns positive control: life_ends with an unanchored read must FAIL (V-SIFT-05)" "fail:ERROR life_ends.e1: UNANCHORED state read" "$PY" -I specs/sifting/lint_patterns.py tests/lint_fixtures/sifting/life_ends_unanchored.toml specs/manifest/registry_v0.toml &&
run "Manifest: solo" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/solo.yaml &&
run "Manifest: val_khor" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/val_khor.yaml &&
run "Manifest: aethelgard" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/aethelgard.yaml &&
run "Manifest: uz_job" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/uz_job.yaml &&
run "Manifest: household (rung 1; SPR-011 ruling 18)" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/household.yaml &&
run "Manifest: hamlet (rung 2; SPR-011 ruling 18)" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/hamlet.yaml &&
run "Manifest: invalid_draft_style must FAIL" fail "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/invalid_draft_style.yaml &&
run "Rust: lint (cargo clippy -D warnings)" pass cargo clippy --workspace --all-targets --locked --quiet -- -D warnings &&
run "Rust: lint rh-bench with the counting allocator (cargo clippy -D warnings; SPR-009 T-05)" pass cargo clippy -p rh-bench --all-targets --features count-alloc --locked --quiet -- -D warnings &&
run "Rust: lint rh-bench with the arena, the instruction-count targets included (cargo clippy -D warnings; SPR-009 R-7)" pass cargo clippy -p rh-bench --all-targets --features arena-alloc --locked --quiet -- -D warnings &&
run "rh-bench positive control: count-alloc and arena-alloc together must not compile (SPR-009 R-7)" "fail:count-alloc and arena-alloc are for separate bench targets" cargo check -p rh-bench --lib --features count-alloc,arena-alloc --locked --quiet &&
# Tier 2 — unit tests
run "Rust: build the workspace (cargo build)" pass cargo build --workspace --locked --quiet &&
run "Rust: unit tests (cargo nextest), less the binaries the by-name gates run (SPR-009 review 2a)" "pass:left to the by-name gates" workspace_tests &&
run "Workspace tests positive control: a by-name entry that names no test binary must FAIL (SPR-009 review 2a)" "fail:no binary IDs matched" workspace_tests_typo_control &&
run "rh-snapshot gates by name: plane hashes, golden vectors, journal, keyframes, unsynced guard (SPR-007 T-01..T-04)" "pass:every_golden_line_reproduces" cargo nextest run -p rh-snapshot --locked --test hash --test golden --test journal --test keyframe --test guard --test stub_world --status-level pass &&
run "V-DET-05 keyframes: a loaded keyframe advances 10 years like the continuous run (SPR-007 T-02)" "pass:V-DET-05: two keyframes loaded and advanced 10 years, 120 monthly hashes equal" cargo nextest run -p rh-snapshot --locked --test keyframe --no-capture v_det_05 &&
run "DEF-3 forged keyframes: no bit flip in the Arrow body panics, aborts or loads another state (SPR-007 T-02)" "pass:accepted_other: 0, panicked: 0 }" cargo nextest run -p rh-snapshot --locked --test keyframe --no-capture def_3_no_forged &&
run "V-DET-10 unsynced guard: read-only APIs fuzzed on the stub world (SPR-007 T-04)" "pass:v_det_10_read_only_apis_never_change_plane_a" cargo nextest run -p rh-snapshot --locked --test unsynced_guard --status-level pass &&
run "V-DET-10 release build: the unsynced guard is compiled out (SPR-007 ruling 5)" "pass:release_builds_compile_the_guard_out" cargo nextest run -p rh-snapshot --locked --release --test guard --status-level pass &&
run "Observer invariance: an attached observer changes no monthly state hash at 1 and 4 threads (SPR-009 ruling 4; V-DET-07 style)" "pass:observer invariance: 73 monthly hashes equal at 4 threads" cargo nextest run -p rh-snapshot --locked --test observer --no-capture &&
run "C dependencies built with -ffp-contract=off: cc build output of bundled SQLite and zstd (SPR-007 ruling 4)" "pass:CFLAGS = Some(-ffp-contract=off)" c_fp_contract_check "$TARGET_DIR" &&
run "C fp-contract check positive control: a C build without the flag must FAIL (SPR-007 ruling 4)" "fail:did not get CFLAGS = -ffp-contract=off" c_fp_contract_control &&
run "rh-ids gates by name: V-DET-16 fork trees, V-DET-20 stream independence, PR tier (SPR-002)" "pass:V-DET-20: 1000 pairs" cargo nextest run -p rh-ids --locked --test fork_trees --test stream_independence --no-capture &&
run "rh-sample gates by name: V-DET-09 golden vectors at opt-level 0, statistics against the exact distributions (SPR-003)" "pass:V-DET-09: 136 golden vectors" cargo nextest run -p rh-sample --locked --test golden --test stats --no-capture &&
run "V-DET-09 at opt-level 3: rh-sample golden vectors under --release (SPR-003 T-05)" "pass:V-DET-09: 136 golden vectors" cargo nextest run -p rh-sample --locked --release --test golden --no-capture &&
run "V-DET-09 positive control: one altered sampler golden vector must FAIL (SPR-003)" "fail:golden vector differs" env RH_SAMPLE_GOLDEN="$PWD/tests/lint_fixtures/golden/altered_samplers_v1.txt" cargo nextest run -p rh-sample --locked --test golden --no-capture &&
run "rh-core gates by name: V-DET-01/02 stub world, AccessSpec conflicts, skeletons, RNG purposes (SPR-005)" "pass:V-DET-01: 600 monthly states equal at 1, 2, 4 and 8 threads" cargo nextest run -p rh-core -p rh-registry --locked --test determinism --test registration --test real_specs --test phase_table --test time_properties --test system_id_golden --test engine_contract --no-capture &&
run "rh-core gates by name: agenda, timers, lifecycle queues, event bus, V-DET-12, invariants and crash replay (SPR-006)" "pass:V-DET-12: golden event log of 196 events checked" cargo nextest run -p rh-core -p rh-events --locked --test agenda --test agenda_events --test invariants --lib --no-capture &&
run "rh-sys-smoke gates by name: registration against the smoke specs, coverage, re-sampling, small-scale V-DET-01/02 (SPR-008 T-01)" "pass:SMOKE: 6 yearly states" cargo nextest run -p rh-sys-smoke --locked --test smoke --no-capture &&
run "V-DET-12 positive control: a back-dated event in the golden log must FAIL (SPR-006)" "fail:V-DET-12 violated" env RH_CORE_LIFE_GOLDEN="$PWD/tests/lint_fixtures/golden/backdated_life_events.txt" cargo nextest run -p rh-core --locked --test agenda_events --no-capture v_det_12_golden_event_log &&
run "T-05 crash replay positive control: an artifact with an altered instant must FAIL (SPR-006)" "fail:does not replay" env RH_CORE_CRASH_ARTIFACT="$PWD/tests/lint_fixtures/golden/crash_altered_tick.json" cargo nextest run -p rh-core --locked --test invariants --no-capture t05_the_committed_crash_artifact_replays &&
run "Rust: doctests (cargo test --doc)" pass doctests_run &&
run "Doctest count gate: every expected compile-fail case and twin ran (SPR-002 T-01; DEF-2)" pass doctest_count_gate "$DOCTEST_LOG" &&
run "Doctest count gate positive control: one typed-ID case missing must FAIL" "fail:crates/rh-ids/src/ids.rs: expected 7 compile-fail doctests, found 6" doctest_count_control &&
run "V-WC-04 manifest parity: Rust loader agrees with manifest_v0.py (SPR-004)" "pass:PARITY: all" "$PY" -I bin/manifest_parity.py --rust-bin "$RH_CLI" specs/manifest/examples/*.yaml tests/manifest_parity/*.yaml &&
run "V-WC-04 parity positive control: drifted Registry must FAIL" "fail:PARITY MISMATCH specs/manifest/examples/invalid_draft_style.yaml" "$PY" -I bin/manifest_parity.py --rust-bin "$RH_CLI" --rust-registry tests/lint_fixtures/parity/registry_drift.toml specs/manifest/examples/invalid_draft_style.yaml &&
run "V-WC-04 parity positive control: message-only drift must FAIL (GAP-3)" "fail:PARITY MISMATCH specs/manifest/examples/invalid_draft_style.yaml" "$PY" -I bin/manifest_parity.py --rust-bin "$RH_CLI" --rust-registry tests/lint_fixtures/parity/registry_drift_message.toml specs/manifest/examples/invalid_draft_style.yaml &&
run "Ownership lint: single writer from specs/systems (SPR-004 T-03)" "pass:0 violations" "$RH_CLI" ownership &&
run "Ownership lint positive control: unowned write must FAIL" "fail:VIOLATION [single-writer] S5.daily_life: writes person.alive, which M2.demography owns" "$RH_CLI" ownership tests/lint_fixtures/ownership/unowned_write.access.toml &&
run "Ownership lint positive control: a write declared twice must FAIL (SPR-005 DEF-3)" "fail:VIOLATION [duplicate] M2.demography: declares a write to person.alive twice" "$RH_CLI" ownership tests/lint_fixtures/ownership/duplicate_write.access.toml &&
run "Manifest JSON Schema from the Rust types = manifest_v0.schema.json (SPR-004 T-04)" "pass:SCHEMA: no unexplained difference" "$RH_CLI" manifest schema --diff specs/manifest/manifest_v0.schema.json &&
run "Schema diff positive control: drifted bound must FAIL" "fail:SCHEMA DIFFERENCE \$defs.Horizon.properties.years.maximum" "$RH_CLI" manifest schema --diff tests/lint_fixtures/schema/drifted_bound.schema.json &&
run "check_all verdict: a chain of passing gates exits 0 (check_all_selftest; VER-SPR-008-TEST GAP-3)" "pass:Verdict: PASS (2 checks)" "$PY" -I bin/check_all_selftest.py verdict &&
run "check_all verdict positive control: a failed gate hidden by || true must still FAIL the tier (VER-SPR-008-TEST GAP-3)" "fail:Verdict: FAIL at \"planted failing gate\"" "$PY" -I bin/check_all_selftest.py verdict --or-true &&
run "check_all chain: no run line ends in || true, || : or || exit (check_all_selftest lint; VER-SPR-008-TEST GAP-3, GAP-7)" "pass:NO OR-TRUE" "$PY" -I bin/check_all_selftest.py lint bin/check_all.sh &&
run "check_all chain positive control: a run line mutated with || true must FAIL the lint (VER-SPR-008-TEST GAP-3)" "fail:OR-TRUE ON A RUN LINE" "$PY" -I bin/check_all_selftest.py lint bin/check_all.sh --plant-or-true &&
run "check_all chain positive control: a run line mutated with || exit 0 must FAIL the lint (VER-SPR-008-TEST GAP-7)" "fail:OR-TRUE ON A RUN LINE" "$PY" -I bin/check_all_selftest.py lint bin/check_all.sh --plant-or-exit &&
run "check_all verdict positive control: a failed gate followed by || exit 0 must still FAIL the tier (VER-SPR-008-TEST GAP-7)" "fail:Verdict: FAIL at \"planted failing gate\"" "$PY" -I bin/check_all_selftest.py verdict --or-exit &&
run "check_all verdict per part: --part core passes while only a golden-part gate fails (check_all_selftest; SPR-009 review 2b)" "pass:Verdict: PASS (1 checks, part core)" "$PY" -I bin/check_all_selftest.py verdict --part-core &&
run "check_all verdict per part positive control: --part golden fails at its failing gate (SPR-009 review 2b)" 'fail:Verdict: FAIL at "planted failing golden gate"' "$PY" -I bin/check_all_selftest.py verdict --part-golden &&
run "check_all verdict positive control: an exit 0 before the chain's end must FAIL the tier (VER-SPR-008-TEST GAP-7)" "fail:Verdict: FAIL (the chain exited early" "$PY" -I bin/check_all_selftest.py verdict --early-exit &&
run "check_all baseline push controls with an absolute CARGO_TARGET_DIR fail and pass for the right reason (check_all_selftest; VER-SPR-008-TEST DEF-3)" "pass:ABSOLUTE TARGET DIR: both baseline push controls fail and pass for the right reason" "$PY" -I bin/check_all_selftest.py abs-target &&
run "check_all positive control: the old PWD-joined golden path under an absolute CARGO_TARGET_DIR must FAIL (VER-SPR-008-TEST DEF-3)" "fail:a baseline push control behaved wrongly" "$PY" -I bin/check_all_selftest.py abs-target --plant-pwd-join &&
# Tier 5 — golden vectors
run "Golden vectors: rh-ids reference matches the committed file (gen_rng_golden --check; V-DET-09)" pass "$PY" -I bin/gen_rng_golden.py --check &&
run "gen_rng_golden positive control: stale golden file must FAIL" "fail:STALE:" "$PY" -I bin/gen_rng_golden.py --check --golden tests/lint_fixtures/golden/stale_rh_ids_v2.txt &&
run "Golden vectors: rh-snapshot plane hashes match the independent reference (gen_hash_golden --check; SPR-007 T-01)" "pass:OK:" "$PY" -I bin/gen_hash_golden.py --check &&
run "gen_hash_golden positive control: one changed hash must FAIL" "fail:STALE:" hash_golden_control &&
run "Sampler reference: exact distributions are current (gen_sampler_reference --check; SPR-003)" pass "$PY" -I bin/gen_sampler_reference.py --check &&
run "gen_sampler_reference positive control: stale reference must FAIL" "fail:STALE:" "$PY" -I bin/gen_sampler_reference.py --check --output tests/lint_fixtures/golden/stale_sampler_reference.txt &&
run "Golden vectors: rh-core::fixed are the exact reference's (gen_fixed_vectors --check)" pass "$PY" -I bin/gen_fixed_vectors.py --check &&
run "Golden vectors: Brass (M2)" pass "$PY" -I specs/demography/selftest_brass.py &&
run "Golden vectors: M2 rung-0 mortality schedule, exact reference (SPR-011 ruling 7)" "pass:M2 RUNG-0 SCHEDULE: PASS" "$PY" -I specs/demography/selftest_m2_rung0.py &&
run "Golden vectors: M4 basket and price rule" pass "$PY" -I specs/economy/selftest_economy.py &&
# Tier 6/8 — determinism-style and contract prototypes
run "Virtual telemetry (S5 prototype)" pass "$PY" -I specs/stream/selftest_virtual_telemetry.py &&
run "World Stream v0 channel-A segment schema: no floats, headers, order, sealed round trip (SPR-011 ruling 13)" "pass:CHANNEL A v0: PASS" "$PY" -I specs/stream/selftest_channel_a.py &&
run "Chronicle SQL-twin equivalence" pass "$PY" -I specs/chronicle/selftest.py &&
part golden &&
run "Required tools present (golden part)" pass require_tools golden &&
# Tier 10 — performance: instruction counts (SPR-009 T-02a; GOV-002 tier 10)
run "V-PERF-04 instruction counts: benches 7 and 8 within 10% of the committed baseline (SPR-009 T-02a)" "pass:V-PERF-04 PASS" "$PY" -I bin/perf_gate.py gate --baseline "$PERF_BASELINE" --out "$PERF_RUN" &&
run "V-PERF-04 repeatability: a second run on the same commit gives identical counts (SPR-009 T-02)" "pass:identical counts in two runs" sh -c '"$1" -I bin/perf_gate.py measure --out artifacts/perf/instructions_run2.txt && "$1" -I bin/perf_gate.py same "$2" artifacts/perf/instructions_run2.txt' sh "$PY" "$PERF_RUN" &&
run "Baseline integrity: a changed instruction-count baseline equals the tool's output, in a [perf-baseline] PR (SPR-009 ruling 3)" pass "$PY" -I bin/perf_gate.py integrity --baseline "$PERF_BASELINE" --measured "$PERF_RUN" &&
run "Baseline integrity positive control: a hand-typed baseline must FAIL (SPR-009 ruling 3)" "fail:plane_hashes/state_full/rows_10k: the baseline's" perf_integrity_control typed &&
run "Baseline integrity control: a count 40 instructions off passes inside the band and is printed (SPR-009 R-8)" "pass:within band plane_hashes/state_full/rows_10k" perf_integrity_control inband &&
run "Baseline integrity positive control: a regenerated baseline without [perf-baseline] must FAIL (SPR-009 ruling 3)" "fail:needs [perf-baseline] in the pull request's title" perf_integrity_control untagged &&
run "Baseline integrity positive control: [perf-baseline] outside the title's trailing tag group must FAIL (SPR-009 review 5)" "fail:needs [perf-baseline] in the pull request's title" perf_integrity_control midtag &&
run "Baseline integrity positive control: a [perf-baseline] PR without a reason must FAIL (SPR-009 ruling 3)" "fail:needs a \`perf-baseline-reason: <why>\` line" perf_integrity_control noreason &&
run "Baseline integrity control: a regenerated baseline in a [perf-baseline] PR with a reason passes (SPR-009 ruling 3)" "pass:equals the tool's output" perf_integrity_control regen &&
run "Baseline integrity control: a push whose commit ends in [perf-baseline] with a Reason: line passes (SPR-009 GAP-2)" "pass:with the [perf-baseline] tag and a reason" perf_push_control tagged &&
run "Baseline integrity positive control: a push with no [perf-baseline] commit and reason must FAIL (SPR-009 GAP-2)" "fail:no commit in the range has a subject ending in [perf-baseline]" perf_push_control untagged &&
run "Baseline integrity control: a local run says the tag and reason are not checked, never the success wording (SPR-009 GAP-2)" "pass:tag and reason not checked here" perf_local_control &&
run "V-PERF-04 control: a speed-up passes and never re-baselines (docs/08 §9)" "pass:speed-up passed; the baseline is unchanged" perf_speedup_control &&
run "V-PERF-04 toolchain control: a baseline from another valgrind must report re-baseline required (SPR-009 ruling 2)" "fail:toolchain changed: re-baseline required" perf_toolchain_control &&
run "V-PERF-04 positive control: a deliberate 15% slowdown in instructions must FAIL (SPR-009 T-02)" "fail:REGRESSION plane_hashes/state_full/rows_10k" "$PY" -I bin/perf_gate.py gate --features slowdown-control --baseline "$PERF_BASELINE" --out artifacts/perf/instructions_slowdown.txt &&
run "V-PERF-11 counting allocator self-test: a known pattern is counted exactly (SPR-009 T-05)" "pass:as expected" cargo bench --locked --quiet -p rh-bench --features count-alloc --bench alloc_budget &&
run "T-02b wall-clock harness: criterion benches 7 and 8 at a smoke size, medians and spreads from the samples (SPR-009)" "pass:8 cases on runner" wall_smoke &&
run "T-02b wall-clock control: a median 25% above the trend is an ALERT that --gate fails (SPR-009)" "fail:wall-clock ALERT above +10% (gating)" wall_control slower &&
run "T-02b wall-clock control: another CPU model must report machine changed, never compare (RUN-002 §5)" "fail:machine changed: new trend baseline required" wall_control machine &&
run "T-02b wall-clock positive control: a NaN median must be refused, never compared (SPR-009 DEF-3)" "fail:not a rehoboam/rh-bench/wall/v1 line" wall_control nan &&
run "V-PERF-11 allocation budget: smoke-world steady-state ticks within the recorded budget at 1 and 4 threads (SPR-009 T-05)" "pass:V-PERF-11 PASS" "$PY" -I bin/perf_gate.py alloc-gate --out "$ALLOC_RUN" &&
run "V-PERF-11 repeatability: a second run on the same commit gives identical allocation counts (SPR-009 T-05)" "pass:identical allocation counts in two runs" alloc_repeat &&
run "V-PERF-11 baseline integrity: a changed allocation baseline equals the tool's output exactly (SPR-009 ruling 3, R-8)" pass "$PY" -I bin/perf_gate.py integrity --exact --baseline "$ALLOC_BASELINE" --measured "$ALLOC_RUN" &&
run "V-PERF-11 integrity positive control: an allocation baseline one allocation off must FAIL (SPR-009 R-8)" "fail:does not equal the tool's output" alloc_typed_control &&
run "V-PERF-11 toolchain control: an allocation baseline from another rustc must report re-baseline required (SPR-009 review 7)" "fail:toolchain changed: re-baseline required" alloc_toolchain_control &&
run "V-PERF-11 positive control: a system that zero-fills a buffer in its tick (vec![0; n]) must FAIL (SPR-009 T-05)" "fail:OVER BUDGET threads 1 allocations" "$PY" -I bin/perf_gate.py alloc-gate --control --out artifacts/perf/allocations_control.txt &&
# Golden runs (SPR-008)
run "Rust: build rh-cli in release for the golden runs (SPR-008 T-02)" pass cargo build --release --locked --quiet -p rh-cli &&
run "Golden run: smoke, 1,000 sim-years, yearly ladder, 1 thread, parallel levels at least 5% (SPR-008 T-02, DEF-2)" "pass:smoke: 1000 yearly rungs to year 1000 at 1 thread(s)" "$RH_CLI_RELEASE" run smoke --years 1000 --threads 1 --ladder yearly --require-parallel "$PARALLEL_FLOOR" --out "$GOLDEN/smoke_yearly_t1.ladder" &&
run "Golden run: smoke, 1,000 sim-years, yearly ladder, N threads, parallel levels at least 5%, with the T-03 vitals observer attached (SPR-008 T-02, DEF-2; SPR-009 T-03)" "pass:smoke: 1000 yearly rungs to year 1000 at $GOLDEN_THREADS thread(s)" golden_vitals_run &&
run "Golden ladders: 1 thread = N threads, byte for byte (rh-cli ladder compare; V-DET-01)" "pass:LADDERS EQUAL: 1000 rungs" "$RH_CLI_RELEASE" ladder compare "$GOLDEN/smoke_yearly_t1.ladder" "$GOLDEN/smoke_yearly_tN.ladder" &&
run "ladder compare positive control: one changed table root must FAIL (SPR-008 T-02)" "fail:LADDERS DIFFER: first differing rung: 500" ladder_compare_control &&
run "Golden baseline: worlds/smoke = the recomputation, changed only in a [C3] PR (check_baseline; SPR-008 T-04)" "pass:BASELINE OK" "$PY" -I bin/check_baseline.py --baseline worlds/smoke/baselines/smoke_yearly_1000y.ladder --recomputed "$GOLDEN/smoke_yearly_t1.ladder" "$GOLDEN/smoke_yearly_tN.ladder" &&
run "Golden baseline positive control: a hand-edited baseline must FAIL (SPR-008 T-04)" "fail:BASELINE MISMATCH" baseline_hand_edited_control &&
run "Golden baseline positive control: a regenerated baseline in a PR not marked [C3] must FAIL (SPR-008 T-04)" "fail:BASELINE CHANGED in a PR not marked [C3]" "$PY" -I bin/check_baseline.py --baseline worlds/smoke/baselines/smoke_yearly_1000y.ladder --recomputed "$GOLDEN/smoke_yearly_t1.ladder" --changed yes --title "SPR-008: Golden-run CI" &&
run "Golden baseline control: a regenerated baseline in a [C3] PR passes (SPR-008 T-04)" "pass:regenerated in a [C3] PR" "$PY" -I bin/check_baseline.py --baseline worlds/smoke/baselines/smoke_yearly_1000y.ladder --recomputed "$GOLDEN/smoke_yearly_t1.ladder" --changed yes --title "SPR-008: Golden-run CI [C3]" &&
run "Golden baseline positive control: a pull_request run with no base must FAIL closed (VER-SPR-008 MAJ-1)" "fail:needs its base" "$PY" -I bin/check_baseline.py --baseline worlds/smoke/baselines/smoke_yearly_1000y.ladder --recomputed "$GOLDEN/smoke_yearly_t1.ladder" --event pull_request --base-ref "" &&
run "Golden baseline control: a local run with no base says the C3 rule was not checked (VER-SPR-008 MAJ-1)" "pass:C3 rule not checked (no base ref)" "$PY" -I bin/check_baseline.py --baseline worlds/smoke/baselines/smoke_yearly_1000y.ladder --recomputed "$GOLDEN/smoke_yearly_t1.ladder" --event local --base-ref "" &&
run "Golden baseline positive control: [C3] outside the title's trailing tag group must FAIL (VER-SPR-008 MAJ-1)" "fail:BASELINE CHANGED in a PR not marked [C3]" "$PY" -I bin/check_baseline.py --baseline worlds/smoke/baselines/smoke_yearly_1000y.ladder --recomputed "$GOLDEN/smoke_yearly_t1.ladder" --changed yes --title "docs: explain the [C3] rule" &&
run "Golden baseline positive control: a push that changes the baseline with no [C3] commit must FAIL (VER-SPR-008 MAJ-1)" "fail:BASELINE CHANGED in a PR not marked [C3]" baseline_push_control "feat(SPR-008): regenerate the smoke baseline" &&
run "Golden baseline control: a push that changes the baseline in a [C3] commit passes (VER-SPR-008 MAJ-1)" "pass:regenerated in a [C3] PR (the commit subjects of" baseline_push_control "feat(SPR-008): regenerate the smoke baseline [C3]" &&
run "Golden ladder positive control: an injected run never compares equal to a clean one (VER-SPR-008 MIN-2)" "fail:LADDERS DIFFER" injected_ladder_control &&
run "Golden ladder positive control: CRLF line ends must FAIL ladder compare (VER-SPR-008-TEST GAP-4)" "fail:a carriage return" ladder_bytes_control crlf &&
run "Golden ladder positive control: a missing final newline must FAIL ladder compare (VER-SPR-008-TEST GAP-4)" "fail:does not end in a newline" ladder_bytes_control unterminated &&
run "Golden ladder cut: the yearly cut of a 3-year monthly run = the 3-year yearly run, byte for byte (VER-SPR-008-TEST GAP-6)" "pass:CUT EQUAL" ladder_cut_control &&
run "Golden thread-leak positive control: the planted thread-identity leak must FAIL the 1-versus-N comparison (VER-SPR-008 DEF-2)" "fail:LADDERS DIFFER" thread_leak_control --leak-thread &&
run "Golden thread-leak control: the same runs without the leak compare equal (VER-SPR-008 DEF-2)" "pass:LADDERS EQUAL: 2 rungs" thread_leak_control &&
run "Golden parallel-level positive control: a floor above the run's share of parallel levels must FAIL (VER-SPR-008 DEF-2)" "fail:PARALLEL LEVELS BELOW THE FLOOR" "$RH_CLI_RELEASE" run smoke --years 2 --threads 1 --require-parallel 50 --out "$TARGET_DIR/rendered-fixtures/parallel_floor.ladder" &&
run "Golden resume positive control: a --years shorter than the journal's horizon must FAIL (VER-SPR-008 DEF-1)" "fail:disagrees with the journal's horizon" def1_resume 13 &&
run "Golden resume positive control: a --years past the journal's last input must FAIL without --extend (VER-SPR-008 DEF-1)" "fail:past the journal's last input" def1_resume 30 &&
run "Golden resume control: past the journal's last input with --extend passes, and the header names the journal (VER-SPR-008 DEF-1)" "pass:journal 24 " def1_resume 30 --extend &&
run "Golden resume control: the journal's own horizon with --until passes (VER-SPR-008 DEF-1)" "pass:until 18" def1_resume 24 --until 18 &&
run "rh-golden gates by name: resume = continuous, thread invariance, V-DET-11 bisect on injected perturbations (SPR-008 T-02, T-03)" "pass:V-DET-11: 4 injected perturbations named exactly" cargo nextest run -p rh-golden --release --locked --test golden --no-capture
status=$?
chain_done=1
exit "$status"
