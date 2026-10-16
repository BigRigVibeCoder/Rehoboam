#!/usr/bin/env bash
# The PR tier of GOV-002, fail-fast, with a forensic report (GOV-002 §4, §6: no report means no pass).
# Every sprint adds its own checks here in the same sprint that creates them.
# Usage: bin/check_all.sh            (uses $PYTHON; else python/.venv/bin/python; else python3)
#        PYTHON=/path/to/python bin/check_all.sh
# Needs the Rust toolchain from rust-toolchain.toml, cargo-nextest, cargo-deny, uv, and the uv
# environment from `uv sync --frozen --project python` (RUN-001 §2-4). A missing tool fails the
# first check by name; nothing is ever skipped silently.
set -u
cd "$(dirname "$0")/.." || { echo "check_all.sh: cannot enter the repository root" >&2; exit 2; }
if [ -n "${PYTHON:-}" ]; then PY="$PYTHON"; elif [ -x python/.venv/bin/python ]; then PY=python/.venv/bin/python; else PY="$(command -v python3 || command -v python)"; fi
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

# Every tool the tier needs, checked up front so a missing one is named rather than mistaken for a
# failing check (RUN-001 says how to install each).
require_tools() {
  local missing=""
  command -v cargo >/dev/null 2>&1 || missing="$missing cargo(RUN-001 §2)"
  cargo fmt --version >/dev/null 2>&1 || missing="$missing rustfmt(RUN-001 §2)"
  cargo clippy --version >/dev/null 2>&1 || missing="$missing clippy(RUN-001 §2)"
  cargo nextest --version >/dev/null 2>&1 || missing="$missing cargo-nextest(RUN-001 §3)"
  cargo deny --version >/dev/null 2>&1 || missing="$missing cargo-deny(RUN-001 §3)"
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

# Effect-language golden control (SPR-017 spec): the committed golden file with one expected value
# altered, which the reference must reject by the case's name.
effect_golden_control() {
  local planted=target/rendered-fixtures/altered_effect_language_golden_v0.toml
  mkdir -p "$(dirname "$planted")" || return 1
  sed '0,/^expect = "bool:true"$/s//expect = "bool:false"/' specs/registry/effect_language_golden_v0.toml > "$planted" || return 1
  "$PY" -I specs/registry/effect_ref.py --check --golden "$planted"
}

# The checks: ONE fail-fast chain. Define every function above this line; a definition placed between
# two `&&` lines splits the chain, and later checks would run after a failure (DEF-2).
run "Required tools present" pass require_tools &&
# Tier 1 — static and contract lints (fastest first)
run "Document index builds (gen_manifest --check)" pass "$PY" -I bin/gen_manifest.py --check &&
run "CI workflows parse (YAML)" pass "$PY" -I -c 'import sys, yaml; [yaml.safe_load(open(f, encoding="utf-8")) for f in sys.argv[1:]]' .github/workflows/ci.yml .github/workflows/advisories.yml .github/workflows/nightly.yml &&
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
run "Rust: format (cargo fmt --check)" pass cargo fmt --all --check &&
run "Rust: bans and sources (cargo deny: crates.io only, no wildcards)" pass cargo deny --color never --locked check bans sources &&
run "cargo deny positive control: git source must FAIL" "fail:error[source-not-allowed]" git_source_control &&
run "cargo deny positive control: wildcard version must FAIL" "fail:error[wildcard]" cargo deny --color never --offline --locked --manifest-path tests/lint_fixtures/deny/wildcard_version/Cargo.toml check bans &&
run "Licence ledger, not a gate (artifacts/licence_ledger.txt; ADR-0021)" pass licence_ledger Cargo.toml artifacts/licence_ledger.txt &&
run "Licence ledger control: the GPL fixture is listed as GPL-3.0-only" "pass:GPL-3.0-only" licence_ledger tests/lint_fixtures/deny/gpl_dependency/Cargo.toml target/rendered-fixtures/licence_ledger_gpl.txt &&
run "Architecture: crate boundaries and planes (lint_arch; V-INSP-03, V-PLN-01)" pass "$PY" -I bin/lint_arch.py &&
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
run "Clippy float backstop positive control: unnamed float arithmetic in plane A must FAIL (SPR-010 GAP-3)" "fail:floating-point arithmetic detected" clippy_control float_unnamed fixture-core &&
run "Clippy libm-only positive control: std ln in rh-sample must FAIL (SPR-003 T-00)" 'fail:use of a disallowed method `f64::ln`' clippy_control std_transcendental rh-sample &&
run "Clippy libm-only positive control: std exp in rh-sample must FAIL (SPR-003 T-00)" 'fail:use of a disallowed method `f64::exp`' clippy_control std_transcendental rh-sample &&
run "System specs (lint_odd)" pass "$PY" -I specs/systems/lint_odd.py &&
run "Generated skeletons: committed files = lint_odd.py --rust (gen_skeletons --check; SPR-005 T-03)" "pass:up to date" "$PY" -I bin/gen_skeletons.py --check &&
run "gen_skeletons positive control: hand-edited skeleton must FAIL" "fail:STALE: tests/lint_fixtures/skeletons/stale/M2_demography.rs" "$PY" -I bin/gen_skeletons.py --check --out tests/lint_fixtures/skeletons/stale &&
run "Sifting patterns (lint_patterns)" pass "$PY" -I specs/sifting/lint_patterns.py specs/sifting/patterns.example.toml specs/manifest/registry_v0.toml &&
run "Manifest: solo" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/solo.yaml &&
run "Manifest: val_khor" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/val_khor.yaml &&
run "Manifest: aethelgard" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/aethelgard.yaml &&
run "Manifest: uz_job" pass "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/uz_job.yaml &&
run "Manifest: invalid_draft_style must FAIL" fail "$PY" -I specs/manifest/manifest_v0.py validate specs/manifest/examples/invalid_draft_style.yaml &&
run "Rust: lint (cargo clippy -D warnings)" pass cargo clippy --workspace --all-targets --locked --quiet -- -D warnings &&
# Tier 2 — unit tests
run "Rust: build the workspace (cargo build)" pass cargo build --workspace --locked --quiet &&
run "Rust: unit tests (cargo nextest)" pass cargo nextest run --workspace --locked --no-fail-fast --status-level fail &&
run "rh-snapshot gates by name: plane hashes, golden vectors, journal, keyframes, unsynced guard (SPR-007 T-01..T-04)" "pass:every_golden_line_reproduces" cargo nextest run -p rh-snapshot --locked --test hash --test golden --test journal --test keyframe --test guard --test stub_world --status-level pass &&
run "V-DET-05 keyframes: a loaded keyframe advances 10 years like the continuous run (SPR-007 T-02)" "pass:V-DET-05: two keyframes loaded and advanced 10 years, 120 monthly hashes equal" cargo nextest run -p rh-snapshot --locked --test keyframe --no-capture v_det_05 &&
run "DEF-3 forged keyframes: no bit flip in the Arrow body panics, aborts or loads another state (SPR-007 T-02)" "pass:accepted_other: 0, panicked: 0 }" cargo nextest run -p rh-snapshot --locked --test keyframe --no-capture def_3_no_forged &&
run "V-DET-10 unsynced guard: read-only APIs fuzzed on the stub world (SPR-007 T-04)" "pass:v_det_10_read_only_apis_never_change_plane_a" cargo nextest run -p rh-snapshot --locked --test unsynced_guard --status-level pass &&
run "V-DET-10 release build: the unsynced guard is compiled out (SPR-007 ruling 5)" "pass:release_builds_compile_the_guard_out" cargo nextest run -p rh-snapshot --locked --release --test guard --status-level pass &&
run "C dependencies built with -ffp-contract=off: cc build output of bundled SQLite and zstd (SPR-007 ruling 4)" "pass:CFLAGS = Some(-ffp-contract=off)" c_fp_contract_check "$TARGET_DIR" &&
run "C fp-contract check positive control: a C build without the flag must FAIL (SPR-007 ruling 4)" "fail:did not get CFLAGS = -ffp-contract=off" c_fp_contract_control &&
run "rh-ids gates by name: V-DET-16 fork trees, V-DET-20 stream independence, PR tier (SPR-002)" "pass:V-DET-20: 1000 pairs" cargo nextest run -p rh-ids --locked --test fork_trees --test stream_independence --no-capture &&
run "rh-sample gates by name: V-DET-09 golden vectors at opt-level 0, statistics against the exact distributions (SPR-003)" "pass:V-DET-09: 136 golden vectors" cargo nextest run -p rh-sample --locked --test golden --test stats --no-capture &&
run "V-DET-09 at opt-level 3: rh-sample golden vectors under --release (SPR-003 T-05)" "pass:V-DET-09: 136 golden vectors" cargo nextest run -p rh-sample --locked --release --test golden --no-capture &&
run "V-DET-09 positive control: one altered sampler golden vector must FAIL (SPR-003)" "fail:golden vector differs" env RH_SAMPLE_GOLDEN="$PWD/tests/lint_fixtures/golden/altered_samplers_v1.txt" cargo nextest run -p rh-sample --locked --test golden --no-capture &&
run "rh-core gates by name: V-DET-01/02 stub world, AccessSpec conflicts, skeletons, RNG purposes (SPR-005)" "pass:V-DET-01: 600 monthly states equal at 1, 2, 4 and 8 threads" cargo nextest run -p rh-core -p rh-registry --locked --test determinism --test registration --test real_specs --test phase_table --test time_properties --test system_id_golden --test engine_contract --no-capture &&
run "rh-core gates by name: agenda, timers, lifecycle queues, event bus, V-DET-12, invariants and crash replay (SPR-006)" "pass:V-DET-12: golden event log of 196 events checked" cargo nextest run -p rh-core -p rh-events --locked --test agenda --test agenda_events --test invariants --lib --no-capture &&
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
run "Python: unit tests (pytest)" pass "$PY" -I -m pytest -c python/pyproject.toml --rootdir=. -q &&
# Tier 5 — golden vectors
run "Golden vectors: rh-ids reference matches the committed file (gen_rng_golden --check; V-DET-09)" pass "$PY" -I bin/gen_rng_golden.py --check &&
run "gen_rng_golden positive control: stale golden file must FAIL" "fail:STALE:" "$PY" -I bin/gen_rng_golden.py --check --golden tests/lint_fixtures/golden/stale_rh_ids_v2.txt &&
run "Golden vectors: rh-snapshot plane hashes match the independent reference (gen_hash_golden --check; SPR-007 T-01)" "pass:OK:" "$PY" -I bin/gen_hash_golden.py --check &&
run "gen_hash_golden positive control: one changed hash must FAIL" "fail:STALE:" hash_golden_control &&
run "Sampler reference: exact distributions are current (gen_sampler_reference --check; SPR-003)" pass "$PY" -I bin/gen_sampler_reference.py --check &&
run "gen_sampler_reference positive control: stale reference must FAIL" "fail:STALE:" "$PY" -I bin/gen_sampler_reference.py --check --output tests/lint_fixtures/golden/stale_sampler_reference.txt &&
run "Golden vectors: rh-core::fixed are the exact reference's (gen_fixed_vectors --check)" pass "$PY" -I bin/gen_fixed_vectors.py --check &&
run "Golden cases: effect language v0 reference reproduces effect_language_golden_v0.toml (effect_ref --check; SPR-017)" "pass:EFFECT LANGUAGE v0.1.0: PASS" "$PY" -I specs/registry/effect_ref.py --check &&
run "effect_ref positive control: one altered expected value must FAIL (SPR-017)" "fail:FAIL lit_true" effect_golden_control &&
run "Golden vectors: Brass (M2)" pass "$PY" -I specs/demography/selftest_brass.py &&
run "Golden vectors: M4 basket and price rule" pass "$PY" -I specs/economy/selftest_economy.py &&
# Tier 6/8 — determinism-style and contract prototypes
run "Virtual telemetry (S5 prototype)" pass "$PY" -I specs/stream/selftest_virtual_telemetry.py &&
run "Chronicle SQL-twin equivalence" pass "$PY" -I specs/chronicle/selftest.py
status=$?

{
  echo
  if [ $status -eq 0 ]; then echo "## Verdict: PASS ($n checks)"; else echo "## Verdict: FAIL at \"$failed\" (later checks skipped: fail-fast)"; fi
} >> "$REPORT"
echo "Report: $REPORT"
exit $status
