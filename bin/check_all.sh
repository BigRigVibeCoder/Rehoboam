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

# The clippy float backstop control (SPR-010 GAP-3): renders tests/lint_fixtures/clippy/float_unnamed/ into
# target/, with the [workspace.lints.*] tables copied from the repository's Cargo.toml, so it tests the real
# lint settings; clippy must reject plane-A float arithmetic on values that carry no float token.
clippy_float_control() {
  local src=tests/lint_fixtures/clippy/float_unnamed dst=target/rendered-fixtures/float_unnamed lints
  rm -rf "$dst" && mkdir -p "$dst" && cp -R "$src/." "$dst/" || return 1
  lints="$(awk '/^\[workspace\.lints\./ {on=1} /^\[/ && !/^\[workspace\.lints\./ {on=0} on' Cargo.toml)"
  [ -n "$lints" ] || { echo "no [workspace.lints.*] tables in Cargo.toml"; return 1; }
  awk -v lints="$lints" '$0 == "@WORKSPACE_LINTS@" {print lints; next} {print}' "$dst/Cargo.toml.in" > "$dst/Cargo.toml" || return 1
  cargo generate-lockfile --quiet --offline --manifest-path "$dst/Cargo.toml" || return 1
  cargo clippy --offline --locked --quiet --manifest-path "$dst/Cargo.toml" -p fixture-core --target-dir target/lint-fixtures -- -D warnings
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

# The checks: ONE fail-fast chain. Define every function above this line; a definition placed between
# two `&&` lines splits the chain, and later checks would run after a failure (DEF-2).
run "Required tools present" pass require_tools &&
# Tier 1 — static and contract lints (fastest first)
run "Document index builds (gen_manifest --check)" pass "$PY" -I bin/gen_manifest.py --check &&
run "CI workflows parse (YAML)" pass "$PY" -I -c 'import sys, yaml; [yaml.safe_load(open(f, encoding="utf-8")) for f in sys.argv[1:]]' .github/workflows/ci.yml .github/workflows/advisories.yml .github/workflows/nightly.yml &&
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
run "lint_arch positive control: plane A -> rand must FAIL" "fail:VIOLATION [rng-crate] rh-core: plane-A crate depends on rand;" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_rng_crate/Cargo.toml &&
run "lint_arch positive control: rand_chacha outside rh-ids must FAIL" "fail:VIOLATION [rng-crate] rh-core: depends on rand_chacha;" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/rng_building_block_outside_ids/Cargo.toml &&
run "lint_arch positive control: plane A -> unreviewed third-party crate must FAIL" "fail:VIOLATION [plane-A-external] rh-core: depends on itoa, which is not on the reviewed list" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_unlisted_dep/Cargo.toml &&
run "lint_arch positive control: libm outside rh-sample must FAIL" "fail:VIOLATION [plane-A-external] rh-core: depends on libm, which plane A may use only in rh-sample" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_scoped_dep_outside/Cargo.toml &&
run "lint_arch positive control: plane A -> rh-manifest (tool) must FAIL (SPR-004)" "fail:VIOLATION [plane-A-closure] rh-registry: plane-A crate depends on plane-\"tool\" crate rh-manifest" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/plane_a_to_manifest/Cargo.toml &&
run "lint_arch positive control: rh-manifest relabelled as plane A must FAIL (SPR-004)" "fail:VIOLATION [metadata] rh-manifest: declares (plane, kind) = ('A', 'machinery'), but rh-manifest is pinned" "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/arch/relabelled_manifest/Cargo.toml &&
run "V-PLN-01 compile-fail fixture has a clean dependency graph" pass "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/compile_fail/plane_a_names_plane_b/Cargo.toml &&
run "V-PLN-01 compile-fail control: plane A naming a plane-B type must FAIL" 'fail:unlinked crate `fixture_observation`' cargo check --offline --locked --quiet --manifest-path tests/lint_fixtures/compile_fail/plane_a_names_plane_b/Cargo.toml --target-dir target/lint-fixtures &&
run "V-PLN-01 compile-fail fixture (plane C) has a clean dependency graph" pass "$PY" -I bin/lint_arch.py --manifest-path tests/lint_fixtures/compile_fail/plane_a_names_plane_c/Cargo.toml &&
run "V-PLN-01 compile-fail control: plane A naming a plane-C type must FAIL" 'fail:unlinked crate `fixture_narrative`' cargo check --offline --locked --quiet --manifest-path tests/lint_fixtures/compile_fail/plane_a_names_plane_c/Cargo.toml --target-dir target/lint-fixtures &&
fixed_point_compile_controls &&
run "Forbidden APIs in plane A (lint_forbidden; V-DET-19)" pass "$PY" -I bin/lint_forbidden.py &&
forbidden_api_controls &&
run "lint_forbidden positive control: workspace plane-A crate must FAIL" "fail:VIOLATION [V-DET-19/hash-collections] \`HashMap\` at tests/lint_fixtures/forbidden/workspace/fixture-core/" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/workspace/Cargo.toml &&
run "lint_forbidden positive control: float threshold in a plane-A crate must FAIL (SPR-010 T-04)" "fail:VIOLATION [V-DET-19/float] \`f32\` at tests/lint_fixtures/forbidden/float_workspace/fixture-sys/src/lib.rs" "$PY" -I bin/lint_forbidden.py --manifest-path tests/lint_fixtures/forbidden/float_workspace/Cargo.toml &&
run "lint_forbidden control: a declared f32_field escape is accepted and counted (SPR-010 T-04)" "pass:0 violations; 1 escapes; 1 declared f32_field columns" "$PY" -I bin/lint_forbidden.py --scan tests/lint_fixtures/forbidden/float_field/declared.rs --specs tests/lint_fixtures/forbidden/float_field/specs &&
run "Clippy float backstop positive control: unnamed float arithmetic in plane A must FAIL (SPR-010 GAP-3)" "fail:floating-point arithmetic detected" clippy_float_control &&
run "System specs (lint_odd)" pass "$PY" -I specs/systems/lint_odd.py &&
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
run "rh-ids gates by name: V-DET-16 fork trees, V-DET-20 stream independence, PR tier (SPR-002)" "pass:V-DET-20: 1000 pairs" cargo nextest run -p rh-ids --locked --test fork_trees --test stream_independence --no-capture &&
run "Rust: doctests (cargo test --doc)" pass doctests_run &&
run "Doctest count gate: every expected compile-fail case and twin ran (SPR-002 T-01; DEF-2)" pass doctest_count_gate "$DOCTEST_LOG" &&
run "Doctest count gate positive control: one typed-ID case missing must FAIL" "fail:crates/rh-ids/src/ids.rs: expected 7 compile-fail doctests, found 6" doctest_count_control &&
run "V-WC-04 manifest parity: Rust loader agrees with manifest_v0.py (SPR-004)" "pass:PARITY: all" "$PY" -I bin/manifest_parity.py specs/manifest/examples/*.yaml tests/manifest_parity/*.yaml &&
run "V-WC-04 parity positive control: drifted Registry must FAIL" "fail:PARITY MISMATCH specs/manifest/examples/invalid_draft_style.yaml" "$PY" -I bin/manifest_parity.py --rust-registry tests/lint_fixtures/parity/registry_drift.toml specs/manifest/examples/invalid_draft_style.yaml &&
run "Ownership lint: single writer from specs/systems (SPR-004 T-03)" "pass:0 violations" target/debug/rh-cli ownership &&
run "Ownership lint positive control: unowned write must FAIL" "fail:VIOLATION [single-writer] S5.daily_life: writes person.alive, which M2.demography owns" target/debug/rh-cli ownership tests/lint_fixtures/ownership/unowned_write.access.toml &&
run "Manifest JSON Schema from the Rust types = manifest_v0.schema.json (SPR-004 T-04)" "pass:SCHEMA: no unexplained difference" target/debug/rh-cli manifest schema --diff specs/manifest/manifest_v0.schema.json &&
run "Schema diff positive control: drifted bound must FAIL" "fail:SCHEMA DIFFERENCE \$defs.Horizon.properties.years.maximum" target/debug/rh-cli manifest schema --diff tests/lint_fixtures/schema/drifted_bound.schema.json &&
run "Python: unit tests (pytest)" pass "$PY" -I -m pytest -c python/pyproject.toml --rootdir=. -q &&
# Tier 5 — golden vectors
run "Golden vectors: rh-ids reference matches the committed file (gen_rng_golden --check; V-DET-09)" pass "$PY" -I bin/gen_rng_golden.py --check &&
run "gen_rng_golden positive control: stale golden file must FAIL" "fail:STALE:" "$PY" -I bin/gen_rng_golden.py --check --golden tests/lint_fixtures/golden/stale_rh_ids_v2.txt &&
run "Golden vectors: rh-core::fixed are the exact reference's (gen_fixed_vectors --check)" pass "$PY" -I bin/gen_fixed_vectors.py --check &&
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
