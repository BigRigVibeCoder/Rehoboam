"""The gate inventory of bin/check_all.sh: deleting a gate's line must turn this test red.

Every check a sprint adds to bin/check_all.sh is listed here by its exact name, and must appear as a
`run "<name>"` line inside the fail-fast chain (not in a comment, not after `status=$?`). Without
this list, removing a line leaves the tier green: nothing else notices that a gate stopped running
(SPR-010 GAP-4, the golden `--check` line; VER-SPR-001-TEST §4 item 2).

EVERY SPRINT ADDS ITS GATES HERE, in the same commit that adds them to bin/check_all.sh. Keep each
list alphabetical. Renaming a gate means renaming it here too, which is the point: a reviewer sees
the change.

Refs: SPR-010 GAP-4 (VER-SPR-010-TEST); GOV-002 §1 laws 4 and 9, §4.
"""

import re
import shlex
from pathlib import Path

import check_all_selftest

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "bin" / "check_all.sh"
FIXED_POINT_FIXTURE = (
    Path(__file__).resolve().parents[2] / "tests" / "lint_fixtures" / "compile_fail" / "fixed_point"
)
RUN_NAME = re.compile(r'^\s*run "(?P<name>[^"]*)"')
NEXTEST_PACKAGE = re.compile(r"-p (\S+)")
NEXTEST_TARGET = re.compile(r"--test (\S+)")

# Every gate run directly by the chain, by its exact `run` name. Alphabetical (Python's str order).
REQUIRED_GATES = (
    "Architecture: crate boundaries, planes and engine purity (lint_arch; V-INSP-03, V-PLN-01, "
    "V-INSP-05)",
    "Authored numbers: rh-manifest's exact-decimal vectors are the decimal reference's "
    "(gen_decimal_vectors --check; SPR-011 T-04)",
    "Baseline integrity control: a count 40 instructions off passes inside the band and is "
    "printed (SPR-009 R-8)",
    "Baseline integrity control: a local run says the tag and reason are not checked, never the "
    "success wording (SPR-009 GAP-2)",
    "Baseline integrity control: a push whose commit ends in [perf-baseline] with a Reason: "
    "line passes (SPR-009 GAP-2)",
    "Baseline integrity control: a regenerated baseline in a [perf-baseline] PR with a reason "
    "passes (SPR-009 ruling 3)",
    "Baseline integrity positive control: [perf-baseline] outside the title's trailing tag "
    "group must FAIL (SPR-009 review 5)",
    "Baseline integrity positive control: a [perf-baseline] PR without a reason must FAIL "
    "(SPR-009 ruling 3)",
    "Baseline integrity positive control: a hand-typed baseline must FAIL (SPR-009 ruling 3)",
    "Baseline integrity positive control: a push with no [perf-baseline] commit and reason must "
    "FAIL (SPR-009 GAP-2)",
    "Baseline integrity positive control: a regenerated baseline without [perf-baseline] must "
    "FAIL (SPR-009 ruling 3)",
    "Baseline integrity: a changed instruction-count baseline equals the tool's output, in a "
    "[perf-baseline] PR (SPR-009 ruling 3)",
    "C dependencies built with -ffp-contract=off: cc build output of bundled SQLite and zstd "
    "(SPR-007 ruling 4)",
    "C fp-contract check positive control: a C build without the flag must FAIL (SPR-007 ruling 4)",
    "CI workflows parse (YAML)",
    "Canonical text form v0: round trip, refusals and golden lines with digests (canonical_v0 "
    "--check; SPR-013 T-09)",
    "Chronicle SQL-twin equivalence",
    "Chronicle envelope v0: anchored snapshots, typed edges, cause_told, provenance index, byte "
    "budget (selftest_envelope; SPR-020)",
    "Clippy float backstop positive control: unnamed float arithmetic in plane A must FAIL "
    "(SPR-010 GAP-3)",
    "Clippy libm-only positive control: std exp in rh-sample must FAIL (SPR-003 T-00)",
    "Clippy libm-only positive control: std ln in rh-sample must FAIL (SPR-003 T-00)",
    "DEF-3 forged keyframes: no bit flip in the Arrow body panics, aborts or loads another "
    "state (SPR-007 T-02)",
    "Doctest count gate positive control: one typed-ID case missing must FAIL",
    "Doctest count gate: every expected compile-fail case and twin ran (SPR-002 T-01; DEF-2)",
    "Document index builds (gen_manifest --check)",
    "Fixed-point exp and ln v0: constants, convergence, faithful rounding, refusals, golden "
    "vectors (exp_ln_v0 --check; 05.1 §6 item 4 v1.10; SPR-016)",
    "Forbidden APIs in plane A (lint_forbidden; V-DET-19)",
    "Generated skeletons: committed files = lint_odd.py --rust (gen_skeletons --check; SPR-005 "
    "T-03)",
    "Golden baseline control: a local run with no base says the C3 rule was not checked "
    "(VER-SPR-008 MAJ-1)",
    "Golden baseline control: a push that changes the baseline in a [C3] commit passes "
    "(VER-SPR-008 MAJ-1)",
    "Golden baseline control: a regenerated baseline in a [C3] PR passes (SPR-008 T-04)",
    "Golden baseline positive control: [C3] outside the title's trailing tag group must FAIL "
    "(VER-SPR-008 MAJ-1)",
    "Golden baseline positive control: a hand-edited baseline must FAIL (SPR-008 T-04)",
    "Golden baseline positive control: a hand-edited solo baseline must FAIL (SPR-012 T-08; SPR-008"
    " ruling 6)",
    "Golden baseline positive control: a pull_request run with no base must FAIL closed "
    "(VER-SPR-008 MAJ-1)",
    "Golden baseline positive control: a push that changes the baseline with no [C3] commit "
    "must FAIL (VER-SPR-008 MAJ-1)",
    "Golden baseline positive control: a regenerated baseline in a PR not marked [C3] must FAIL "
    "(SPR-008 T-04)",
    "Golden baseline positive control: a regenerated solo baseline in a PR not marked [C3] must "
    "FAIL (SPR-012 T-08; SPR-008 ruling 6)",
    "Golden baseline: worlds/smoke = the recomputation, changed only in a [C3] PR "
    "(check_baseline; SPR-008 T-04)",
    "Golden baseline: worlds/solo = the recomputation, changed only in a [C3] PR (V-GOLD-01; "
    "SPR-012 T-08)",
    "Golden draws: M2's death instants on fixture worlds are the independent model's, every S11 "
    "re-sample case covered (gen_m2_draws_reference --check; SPR-012, Tester's interim report)",
    "Golden ladder cut: the yearly cut of a 3-year monthly run = the 3-year yearly run, byte for "
    "byte (VER-SPR-008-TEST GAP-6)",
    "Golden ladder positive control: CRLF line ends must FAIL ladder compare (VER-SPR-008-TEST "
    "GAP-4)",
    "Golden ladder positive control: a missing final newline must FAIL ladder compare "
    "(VER-SPR-008-TEST GAP-4)",
    "Golden ladder positive control: an injected run never compares equal to a clean one "
    "(VER-SPR-008 MIN-2)",
    "Golden ladders: 1 thread = N threads, byte for byte (rh-cli ladder compare; V-DET-01)",
    "Golden ladders: solo at 1 thread = N threads, byte for byte (V-DET-01/02; SPR-012 T-08)",
    "Golden parallel-level positive control: a floor above the run's share of parallel levels "
    "must FAIL (VER-SPR-008 DEF-2)",
    "Golden resume control: past the journal's last input with --extend passes, and the header "
    "names the journal (VER-SPR-008 DEF-1)",
    "Golden resume control: the journal's own horizon with --until passes (VER-SPR-008 DEF-1)",
    "Golden resume positive control: a --years past the journal's last input must FAIL without "
    "--extend (VER-SPR-008 DEF-1)",
    "Golden resume positive control: a --years shorter than the journal's horizon must FAIL "
    "(VER-SPR-008 DEF-1)",
    "Golden run positive control: --strict refuses solo, whose manifest names systems this build "
    "lacks (SPR-011 ruling 3; SPR-012 T-08)",
    "Golden run: smoke, 1,000 sim-years, yearly ladder, 1 thread, parallel levels at least 5% "
    "(SPR-008 T-02, DEF-2)",
    "Golden run: smoke, 1,000 sim-years, yearly ladder, N threads, parallel levels at least 5%, "
    "with the T-03 vitals observer attached (SPR-008 T-02, DEF-2; SPR-009 T-03)",
    "Golden run: solo, 200 sim-years, yearly ladder, 1 thread, the history tuple and the built and "
    "not-built systems in its header (SPR-012 T-08; rulings 3, 16)",
    "Golden run: solo, 200 sim-years, yearly ladder, N threads (SPR-012 T-08)",
    "Golden thread-leak control: the same runs without the leak compare equal (VER-SPR-008 DEF-2)",
    "Golden thread-leak positive control: the planted thread-identity leak must FAIL the "
    "1-versus-N comparison (VER-SPR-008 DEF-2)",
    "Golden vectors: Brass (M2)",
    "Golden vectors: M2 F1 fertility and partnering, exact reference (SPR-015)",
    "Golden vectors: M2 F1 mortality, exact reference and golden file (selftest_m2_f1_mortality "
    "--check; SPR-016)",
    "Golden vectors: M2 rung-0 mortality schedule, exact reference (SPR-011 ruling 7)",
    "Golden vectors: M2 rung-0 schedule vectors are the exact reference's (gen_m2_reference "
    "--check; SPR-012 T-05)",
    "Golden vectors: M4 basket and price rule",
    "Golden vectors: population synthesis v0 and the dynastic back-fill, exact reference (SPR-019)",
    "Golden vectors: rh-core::fixed are the exact reference's (gen_fixed_vectors --check)",
    "Golden vectors: rh-ids reference matches the committed file (gen_rng_golden --check; "
    "V-DET-09)",
    "Golden vectors: rh-snapshot plane hashes match the independent reference (gen_hash_golden "
    "--check; SPR-007 T-01)",
    "Golden vectors: worldgen-lite v0 and residence at genesis, exact reference (SPR-018)",
    "Licence ledger control: the GPL fixture is listed as GPL-3.0-only",
    "Licence ledger, not a gate (artifacts/licence_ledger.txt; ADR-0021)",
    "M2 F1 mortality reference positive control: a stale check value in the next M2 spec must FAIL "
    "(SPR-016)",
    "M2 F1 positive control: a newborn drawn on rung 0's S8 must FAIL the first-draw rule (SPR-016 "
    "MR10)",
    "M2 F1 positive control: a stale golden file must FAIL (SPR-016)",
    "M2 F1 positive control: an infant table that misses l(1) must FAIL continuity at age 1 "
    "(SPR-016 MR7)",
    "M2 F1 positive control: the power form at beta = 1 must FAIL rung-0 equality (SPR-016 MR3)",
    "M2 F1 reference positive control: a stale check value in the next M2 spec must FAIL (SPR-015)",
    "M2 draws differential golden: the engine's death instants equal the independent model's, "
    "re-samples included (SPR-012, Tester's interim report)",
    "M2 draws positive control: one altered death line in the golden must FAIL (SPR-012)",
    "M2 draws positive control: the re-sample's Bernoulli taken from k = 1 must FAIL (SPR-012, the "
    "Tester's first mutant)",
    "M2 draws positive control: the re-sample's day taken from k = 0 must FAIL (SPR-012, the "
    "Tester's second mutant)",
    "Manifest JSON Schema from the Rust types = manifest_v0.schema.json (SPR-004 T-04)",
    "Manifest: aethelgard",
    "Manifest: hamlet (rung 2; SPR-011 ruling 18)",
    "Manifest: household (rung 1; SPR-011 ruling 18)",
    "Manifest: invalid_draft_style must FAIL",
    "Manifest: manor (the SPR-019 fixture: a dynastic manor)",
    "Manifest: solo",
    "Manifest: uz_job",
    "Manifest: val_khor",
    "No FMA contraction: no fp-contract in RUSTFLAGS; C builds take exactly -ffp-contract=off "
    "(SPR-003 GAP-6; SPR-007 ruling 4)",
    "Observer invariance: an attached observer changes no monthly state hash at 1 and 4 threads "
    "(SPR-009 ruling 4; V-DET-07 style)",
    "Ownership lint positive control: a write declared twice must FAIL (SPR-005 DEF-3)",
    "Ownership lint positive control: unowned write must FAIL",
    "Ownership lint: single writer from specs/systems (SPR-004 T-03)",
    "Python: format (ruff format --check)",
    "Python: lint (ruff check)",
    "Python: types (mypy --strict)",
    "Python: unit tests (pytest)",
    "Python: unit tests with hostile CI variables set (RH_EVENT, RH_PR_TITLE, "
    "GITHUB_EVENT_NAME, RH_PUSH_*) pass: conftest.py isolates them (SPR-009 review 1)",
    "Python: uv.lock matches pyproject.toml (uv lock --check)",
    "Required tools present (core part)",
    "Required tools present (golden part)",
    "Rust: bans and sources (cargo deny: crates.io only, no wildcards)",
    "Rust: build rh-cli in release for the golden runs (SPR-008 T-02)",
    "Rust: build the workspace (cargo build)",
    "Rust: doctests (cargo test --doc)",
    "Rust: format (cargo fmt --check)",
    "Rust: lint (cargo clippy -D warnings)",
    "Rust: lint rh-bench with the arena, the instruction-count targets included (cargo clippy "
    "-D warnings; SPR-009 R-7)",
    "Rust: lint rh-bench with the counting allocator (cargo clippy -D warnings; SPR-009 T-05)",
    "Rust: unit tests (cargo nextest), less the binaries the by-name gates run (SPR-009 review 2a)",
    "Sampler reference: exact distributions are current (gen_sampler_reference --check; SPR-003)",
    "Schema diff positive control: drifted bound must FAIL",
    "Sifting patterns (lint_patterns)",
    "Sifting patterns: solitary_life pack, life_ends (lint_patterns; SPR-011 ruling 14)",
    "Smoke-world skeletons: committed files = lint_odd.py --rust (gen_skeletons --check; "
    "SPR-008 ruling 2)",
    "Smoke-world specs (lint_odd --specs worlds/smoke/systems --overlay; SPR-008 ruling 2)",
    "System specs (lint_odd)",
    "System specs, next version: the M2 F1 draft supersedes the frozen rung-0 slice and "
    "preserves it (lint_odd --next; SPR-015)",
    "T-02b wall-clock control: a median 25% above the trend is an ALERT that --gate fails "
    "(SPR-009)",
    "T-02b wall-clock control: another CPU model must report machine changed, never compare "
    "(RUN-002 §5)",
    "T-02b wall-clock harness: criterion benches 7 and 8 at a smoke size, medians and spreads "
    "from the samples (SPR-009)",
    "T-02b wall-clock positive control: a NaN median must be refused, never compared (SPR-009 "
    "DEF-3)",
    "T-05 M2 schedule in fixed point: bit-equal to the exact reference, K = 1 at the standard's e0,"
    " Table 19, e0 within 0.06 years, the q(15) band (SPR-012)",
    "T-05 crash replay positive control: an artifact with an altered instant must FAIL (SPR-006)",
    "T-05 positive control: one altered schedule vector must FAIL (SPR-012)",
    "T-06 V-INV-11: founders' negative birth ticks round-trip through a keyframe, and the loaded "
    "engine advances as the continuous one (SPR-012)",
    "T-06 rh-worldgen: V-DATA-05 founder ids and a forced collision, bit-equal genesis twice and "
    "across threads, Ada at 25, refused writes (SPR-012)",
    "T-07 M2.demography in release: V-DATA-04 over 1,000 years, one founder entry per person and a "
    "death record iff absent (SPR-012; review MIN-1)",
    "T-07 M2.demography in release: q(15) of 10,000 founders aged 0 inside the binomial 99.9% band "
    "for each of 32 seeds and pooled (SPR-012; review MAJ-1, MIN-1)",
    "T-07 M2.demography: registration against the frozen spec, alpha and beta refused, V-DET-12 and"
    " V-INV-01 on random worlds, EXTINCTION once, re-sampling (SPR-012)",
    "V-DET-05 keyframes: a loaded keyframe advances 10 years like the continuous run (SPR-007 "
    "T-02)",
    "V-DET-09 at opt-level 3: rh-sample golden vectors under --release (SPR-003 T-05)",
    "V-DET-09 positive control: one altered sampler golden vector must FAIL (SPR-003)",
    "V-DET-10 release build: the unsynced guard is compiled out (SPR-007 ruling 5)",
    "V-DET-10 unsynced guard: read-only APIs fuzzed on the stub world (SPR-007 T-04)",
    "V-DET-12 positive control: a back-dated event in the golden log must FAIL (SPR-006)",
    "V-INV-08 positive control: a missing crate directory must FAIL, never pass as nothing found "
    "(SPR-012; review MIN-2)",
    "V-INV-08 positive control: a planted Poisson death draw must FAIL (SPR-012 T-07)",
    "V-INV-08: M2 draws deaths by Bernoulli only (no Poisson or binomial draw; SPR-012 T-07)",
    "V-PERF-04 control: a speed-up passes and never re-baselines (docs/08 §9)",
    "V-PERF-04 instruction counts: benches 7 and 8 within 10% of the committed baseline "
    "(SPR-009 T-02a)",
    "V-PERF-04 positive control: a deliberate 15% slowdown in instructions must FAIL (SPR-009 "
    "T-02)",
    "V-PERF-04 repeatability: a second run on the same commit gives identical counts (SPR-009 "
    "T-02)",
    "V-PERF-04 toolchain control: a baseline from another valgrind must report re-baseline "
    "required (SPR-009 ruling 2)",
    "V-PERF-11 allocation budget: smoke-world steady-state ticks within the recorded budget at "
    "1 and 4 threads (SPR-009 T-05)",
    "V-PERF-11 baseline integrity: a changed allocation baseline equals the tool's output "
    "exactly (SPR-009 ruling 3, R-8)",
    "V-PERF-11 counting allocator self-test: a known pattern is counted exactly (SPR-009 T-05)",
    "V-PERF-11 integrity positive control: an allocation baseline one allocation off must FAIL "
    "(SPR-009 R-8)",
    "V-PERF-11 positive control: a system that zero-fills a buffer in its tick (vec![0; n]) "
    "must FAIL (SPR-009 T-05)",
    "V-PERF-11 repeatability: a second run on the same commit gives identical allocation counts "
    "(SPR-009 T-05)",
    "V-PERF-11 toolchain control: an allocation baseline from another rustc must report "
    "re-baseline required (SPR-009 review 7)",
    "V-PLN-01 compile-fail control: plane A naming a plane-B type must FAIL",
    "V-PLN-01 compile-fail control: plane A naming a plane-C type must FAIL",
    "V-PLN-01 compile-fail fixture (plane C) has a clean dependency graph",
    "V-PLN-01 compile-fail fixture has a clean dependency graph",
    "V-SCALE-03: solo's 1,000-year wall time, reported (SPR-012 T-08)",
    "V-WC-04 manifest parity: Rust loader agrees with manifest_v0.py (SPR-004)",
    "V-WC-04 parity positive control: drifted Registry must FAIL",
    "V-WC-04 parity positive control: message-only drift must FAIL (GAP-3)",
    "Virtual telemetry (S5 prototype)",
    "Workspace tests positive control: a by-name entry that names no test binary must FAIL "
    "(SPR-009 review 2a)",
    "World Stream v0 channel-A segment schema: no floats, headers, order, sealed round trip "
    "(SPR-011 ruling 13)",
    "World Stream v0 channel-C segment schema: no floats, header, order, sealed round trip, "
    "digests, life_ends twin (SPR-013 T-11)",
    "World Stream wire format v1: framing, control messages, refusals, the live-edge rule with "
    "slow, stalled and reconnecting viewers, golden frames (selftest_wire_v1 --check; SPR-026)",
    "canonical_v0 positive control: a stale golden digest must FAIL",
    "cargo deny positive control: git source must FAIL",
    "cargo deny positive control: wildcard version must FAIL",
    "channel-C self-test positive control: a float column in the DDL must FAIL",
    "check_all baseline push controls with an absolute CARGO_TARGET_DIR fail and pass for the "
    "right reason (check_all_selftest; VER-SPR-008-TEST DEF-3)",
    "check_all chain positive control: a run line mutated with || exit 0 must FAIL the lint "
    "(VER-SPR-008-TEST GAP-7)",
    "check_all chain positive control: a run line mutated with || true must FAIL the lint "
    "(VER-SPR-008-TEST GAP-3)",
    "check_all chain: no run line ends in || true, || : or || exit (check_all_selftest lint; "
    "VER-SPR-008-TEST GAP-3, GAP-7)",
    "check_all harness positive control: a fail: expectation with empty text must FAIL as a harness"
    " error, not match any failure (VER-SPR-012-TEST GAP-6)",
    "check_all positive control: the old PWD-joined golden path under an absolute "
    "CARGO_TARGET_DIR must FAIL (VER-SPR-008-TEST DEF-3)",
    "check_all verdict per part positive control: --part golden fails at its failing gate "
    "(SPR-009 review 2b)",
    "check_all verdict per part: --part core passes while only a golden-part gate fails "
    "(check_all_selftest; SPR-009 review 2b)",
    "check_all verdict positive control: a failed gate followed by || exit 0 must still FAIL "
    "the tier (VER-SPR-008-TEST GAP-7)",
    "check_all verdict positive control: a failed gate hidden by || true must still FAIL the "
    "tier (VER-SPR-008-TEST GAP-3)",
    "check_all verdict positive control: an exit 0 before the chain's end must FAIL the tier "
    "(VER-SPR-008-TEST GAP-7)",
    "check_all verdict: a chain of passing gates exits 0 (check_all_selftest; VER-SPR-008-TEST "
    "GAP-3)",
    "exp_ln_v0 positive control: a stale golden file must FAIL (SPR-016)",
    "fp-contract guard control: CC, CXX and CXXFLAGS with exactly -ffp-contract=off pass "
    "(SPR-007 GAP-2)",
    "fp-contract guard control: a .cargo/config.toml with exactly CFLAGS = -ffp-contract=off "
    "passes (SPR-007 ruling 4)",
    "fp-contract guard positive control: CC with -ffp-contract=fast must FAIL (SPR-007 GAP-2)",
    "fp-contract guard positive control: CFLAGS with -ffp-contract=fast must FAIL (SPR-007 "
    "ruling 4)",
    "fp-contract guard positive control: CXXFLAGS with -ffp-contract=on must FAIL (SPR-007 GAP-2)",
    "fp-contract guard positive control: RUSTFLAGS with fp-contract must FAIL",
    "fp-contract guard positive control: a .cargo/config.toml CFLAGS with -ffp-contract=fast "
    "must FAIL (SPR-007 ruling 4)",
    "fp-contract guard positive control: a .cargo/config.toml CFLAGS without force = true must "
    "FAIL (SPR-007 ruling 4)",
    "fp-contract guard positive control: a .cargo/config.toml with fp-contract must FAIL",
    "fp-contract guard positive control: a target-specific CXX with -ffp-contract=fast must "
    "FAIL (SPR-007 GAP-2)",
    "gen_decimal_vectors positive control: one altered vector must FAIL (SPR-011 T-04)",
    "gen_hash_golden positive control: one changed hash must FAIL",
    "gen_m2_draws_reference positive control: one altered death line must FAIL (SPR-012)",
    "gen_m2_reference positive control: one altered vector must FAIL (SPR-012 T-05)",
    "gen_rng_golden positive control: stale golden file must FAIL",
    "gen_sampler_reference positive control: stale reference must FAIL",
    "gen_skeletons positive control: a hand-edited smoke skeleton must FAIL (SPR-008 ruling 2)",
    "gen_skeletons positive control: hand-edited skeleton must FAIL",
    "ladder compare positive control: one changed table root must FAIL (SPR-008 T-02)",
    "lint_arch control: a bench target requiring arena-alloc, with its allocator in benches/, "
    "passes (SPR-009 R-7)",
    "lint_arch control: a deny-listed dev-dependency of an engine crate, and a viewer's normal "
    "one, pass (V-INSP-05; SPR-011 T-00)",
    "lint_arch control: a system may depend on rh-sample (SPR-003 T-00)",
    "lint_arch positive control: a #[global_allocator] behind cfg_attr must FAIL (SPR-009 GAP-6)",
    "lint_arch positive control: a #[global_allocator] outside rh-bench's benches must FAIL "
    "(SPR-009 T-05)",
    "lint_arch positive control: a #[global_allocator] under a source directory named target "
    "must FAIL (VER-SPR-009-TEST GAP-21)",
    "lint_arch positive control: a clippy.toml reached through a symlinked member must FAIL "
    "(SPR-003 GAP-13)",
    "lint_arch positive control: a crate enabling rh-bench/count-alloc must FAIL (SPR-009 T-05)",
    "lint_arch positive control: a crate-local clippy.toml must FAIL (SPR-003 GAP-12)",
    "lint_arch positive control: a non-bench target requiring count-alloc must FAIL (SPR-009 T-05)",
    "lint_arch positive control: a plane-A crate enabling rh-bench/arena-alloc must FAIL "
    "(SPR-009 R-7)",
    "lint_arch positive control: a second, target-specific libm entry must FAIL (SPR-003 GAP-10)",
    "lint_arch positive control: a symlinked workspace member must FAIL (SPR-003 GAP-13)",
    "lint_arch positive control: a test target requiring arena-alloc must FAIL (SPR-009 R-7)",
    "lint_arch positive control: a third-party crate in rh-sample must FAIL (SPR-003 GAP-4)",
    "lint_arch positive control: an allow-listed crate whose lints drift from the workspace's "
    "must FAIL (SPR-009 T-05)",
    "lint_arch positive control: an engine crate depending directly on a deny-listed crate must "
    "FAIL (V-INSP-05; SPR-011 T-00)",
    "lint_arch positive control: an engine crate reaching a deny-listed crate transitively must "
    "FAIL (V-INSP-05; SPR-011 T-00)",
    "lint_arch positive control: kind contract outside rh-contract-* must FAIL",
    "lint_arch positive control: libm not pinned exactly must FAIL (SPR-003 GAP-4)",
    "lint_arch positive control: libm outside rh-sample must FAIL",
    "lint_arch positive control: libm with default features must FAIL (SPR-003 GAP-4)",
    "lint_arch positive control: libm with feature arch must FAIL (SPR-003 GAP-10)",
    "lint_arch positive control: missing plane/kind must FAIL",
    "lint_arch positive control: own [lints.clippy] table in rh-sample must FAIL (SPR-003 GAP-9)",
    "lint_arch positive control: plane A -> plane B must FAIL",
    "lint_arch positive control: plane A -> plane C must FAIL",
    "lint_arch positive control: plane A -> rand must FAIL",
    "lint_arch positive control: plane A -> rh-manifest (tool) must FAIL (SPR-004)",
    "lint_arch positive control: plane A -> rh-protocol must FAIL (plane-A closure; SPR-026)",
    "lint_arch positive control: plane A -> unreviewed third-party crate must FAIL",
    "lint_arch positive control: rand_chacha outside rh-ids must FAIL",
    "lint_arch positive control: relabelled rh-snapshot must FAIL",
    "lint_arch positive control: rh-manifest relabelled as plane A must FAIL (SPR-004)",
    "lint_arch positive control: rh-protocol depending on a workspace crate must FAIL "
    "(protocol-leaf; SPR-026)",
    "lint_arch positive control: rh-protocol depending on tokio must FAIL (protocol-leaf "
    "allow-list; SPR-026 R7)",
    "lint_arch positive control: rh-sample relabelled out of plane A must FAIL (SPR-003 T-00)",
    "lint_arch positive control: rh-sample without [lints] workspace = true must FAIL (SPR-003 "
    "GAP-9)",
    "lint_arch positive control: system -> rh-snapshot aliased as rh-core must FAIL",
    "lint_arch positive control: system -> rh-snapshot as a build-dependency must FAIL",
    "lint_arch positive control: system -> rh-snapshot must FAIL",
    "lint_arch positive control: system -> system must FAIL",
    "lint_arch positive control: unsafe outside the allow-listed file must FAIL (SPR-009 T-05)",
    "lint_arch positive control: unsafe_code relaxed by a crate not on the allow-list must FAIL "
    "(GOV-003 §2; SPR-009 T-05)",
    "lint_forbidden control: a declared f32_field escape is accepted and counted (SPR-010 T-04)",
    "lint_forbidden control: a float in rh-sample passes (SPR-003 T-00)",
    "lint_forbidden control: a literal in-repository include! of a clean skeleton is followed "
    "and passes (SPR-008 T-00)",
    "lint_forbidden control: std::env::consts, env! and option_env! pass (DEF-5)",
    "lint_forbidden float-api positive control: a private float alias in rh-sample must FAIL "
    "(SPR-003 GAP-11)",
    "lint_forbidden float-api positive control: an imported float alias in rh-sample must FAIL "
    "(SPR-003 GAP-11b)",
    "lint_forbidden float-api positive control: field in rh-sample's public API must FAIL "
    "(SPR-003 GAP-1)",
    "lint_forbidden float-api positive control: fn_return in rh-sample's public API must FAIL "
    "(SPR-003 GAP-1)",
    "lint_forbidden float-api positive control: impl_from in rh-sample's public API must FAIL "
    "(SPR-003 GAP-1)",
    "lint_forbidden float-api positive control: reexport in rh-sample's public API must FAIL "
    "(SPR-003 GAP-1)",
    "lint_forbidden float-dep positive control: accessor in rh-snapshot must FAIL (SPR-007 GAP-1)",
    "lint_forbidden float-dep positive control: arrow_float in rh-snapshot must FAIL (SPR-007 "
    "T-02)",
    "lint_forbidden float-dep positive control: real_bits in rh-snapshot must FAIL (SPR-007 GAP-1)",
    "lint_forbidden float-dep positive control: real_cmp in rh-snapshot must FAIL (SPR-007 GAP-1)",
    "lint_forbidden float-dep positive control: real_floor in rh-snapshot must FAIL (SPR-007 "
    "GAP-1)",
    "lint_forbidden positive control: a float in a crate named rh-core must FAIL (SPR-003 GAP-3)",
    "lint_forbidden positive control: a float outside rh-sample must FAIL (SPR-003 T-00)",
    "lint_forbidden positive control: a forbidden API behind a nested include! must FAIL (SPR-008 "
    "T-00)",
    "lint_forbidden positive control: a forbidden API in an included skeleton must FAIL (SPR-008 "
    "T-00)",
    "lint_forbidden positive control: a socket in rh-cli outside src/serve.rs must FAIL; "
    "src/serve.rs and tests/ pass, and the tool crate's float is not scanned (SPR-026 R7)",
    "lint_forbidden positive control: a use that renames the include macro must FAIL (VER-SPR-008)",
    "lint_forbidden positive control: an environment read in a plane-A crate's src/ must FAIL, "
    "its tests/ and a builder's args pass (SPR-011 A-4)",
    "lint_forbidden positive control: an include! outside the repository must FAIL (SPR-008 T-00)",
    "lint_forbidden positive control: an include! reached through a symlink resolves from the "
    "link, as rustc does (VER-SPR-008 MIN-3)",
    "lint_forbidden positive control: env::current_dir after a grouped import is reported on "
    "its own line (DEF-5)",
    "lint_forbidden positive control: file I/O in a plane-A crate's src/ off FILE_IO_CRATES "
    "must FAIL, its tests/ and rh-snapshot pass (BCK-002 A-44)",
    "lint_forbidden positive control: float threshold in a plane-A crate must FAIL (SPR-010 T-04)",
    "lint_forbidden positive control: fs::read after a grouped import is reported on its own "
    "line (DEF-1)",
    "lint_forbidden positive control: include!(concat!(...)) must FAIL (SPR-008 T-00)",
    "lint_forbidden positive control: include_str! of a file outside the repository must FAIL "
    "(VER-SPR-008)",
    "lint_forbidden positive control: mul_add in rh-sample must FAIL (SPR-003 T-00)",
    "lint_forbidden positive control: rh-registry's spec loaders named in plane-A src/ outside "
    "their home must FAIL in all seven forms, alias and macro included; tests/ and ownership.rs "
    "pass (BCK-002 A-47; review MAJ-2)",
    "lint_forbidden positive control: the six ways round sockets-only-in-serve.rs (re-exports, "
    "#[path], include!) must FAIL (SPR-026 R19)",
    "lint_forbidden positive control: workspace plane-A crate must FAIL",
    "lint_odd applies control: a sender's payload matching a typed [[applies]] field for field, "
    "with its drain binding, passes (REV-P1a section 7; EVO-001)",
    "lint_odd applies positive control: a sender's payload fields differ from [[applies]], "
    "frozen, must FAIL (REV-P1a section 7)",
    "lint_odd applies positive control: an [[applies]] drained_by naming no binding of "
    "[[schedule]], frozen, must FAIL (EVO-001)",
    "lint_odd applies positive control: an [[applies]] payload type outside the closed "
    "vocabulary, frozen, must FAIL (EVO-001)",
    "lint_odd applies positive control: an outcome-carrying command whose sender names no "
    "timer, frozen, must FAIL (REV-EVO-001 R9)",
    "lint_odd applies positive control: two [[applies]] entries for one command must FAIL "
    "(REV-EVO-001 MIN-11)",
    "lint_odd emits control: a Registry overlay declares a world's own event kinds (SPR-011 "
    "ruling 11)",
    "lint_odd emits control: a draft spec emitting a non-verb warns and passes (SPR-011 ruling 11)",
    "lint_odd emits positive control: a frozen spec emitting a non-verb must FAIL (SPR-011 "
    "ruling 11)",
    "lint_odd envelope control: a draft spec's [[emits]] entry without roles warns and passes "
    "(SPR-020; REV-SPR-020 R7)",
    "lint_odd envelope control: roles, motives, snapshots, an algorithm source and a "
    "cause_column that all resolve pass (SPR-020)",
    "lint_odd envelope positive control: a cause_column on a state column without a declared "
    "default must FAIL (REV-SPR-020 R2)",
    "lint_odd envelope positive control: a cause_column that is not an owned event_ref of the "
    "same entity must FAIL (SAGA-15; SPR-020, REV-SPR-020 R17)",
    "lint_odd envelope positive control: a frozen spec's [[emits]] entry without roles must "
    "FAIL (SPR-020; REV-SPR-020 R7)",
    "lint_odd envelope positive control: a motive listed twice must FAIL (REV-SPR-020 R7)",
    "lint_odd envelope positive control: a motive the Registry [motive_edges] lacks must FAIL "
    "(SPR-020)",
    "lint_odd envelope positive control: a non-witness role listed twice must FAIL (SPR-020)",
    "lint_odd envelope positive control: a q8_8 attribute filled from a bool column must FAIL "
    "(SPR-020)",
    "lint_odd envelope positive control: a role the Registry [event_roles] lacks must FAIL "
    "(SPR-020)",
    "lint_odd envelope positive control: a snapshot declared twice must FAIL (REV-SPR-020 R7)",
    "lint_odd envelope positive control: a snapshot from a column the spec neither owns nor "
    "reads must FAIL (SPR-020)",
    "lint_odd envelope positive control: a snapshot of a role the kind does not fill must FAIL "
    "(SPR-020)",
    "lint_odd envelope positive control: a snapshot of an attribute the Registry lacks must "
    "FAIL (SPR-020)",
    "lint_odd envelope positive control: a snapshot of witness must FAIL (REV-SPR-020 R7)",
    "lint_odd envelope positive control: a snapshot without a source column must FAIL (SPR-020)",
    "lint_odd envelope positive control: an algorithm source without its type must FAIL "
    "(REV-SPR-020 R7)",
    "lint_odd envelope positive control: an enum attribute filled from a column with other "
    "codes must FAIL (SPR-020)",
    "lint_odd envelope positive control: an enum attribute filled from a foreign column with "
    "other codes must FAIL (REV-SPR-020 R7)",
    "lint_odd neutral control: a draft spec's rate read of an absent owner with no neutral "
    "warns and passes (EVO-001)",
    "lint_odd neutral control: a frozen substrate reading a non-substrate's column as identity "
    "warns and passes; the rule is advisory, registration enforces it (REV-EVO-001 R12, R16)",
    "lint_odd neutral control: an identity read without a neutral and a rate read with a typed "
    "neutral stated by its coupling pass (EVO-001)",
    "lint_odd neutral positive control: a frozen spec's rate read of an absent owner with no "
    "neutral must FAIL (EVO-001)",
    "lint_odd neutral positive control: a neutral of the wrong type must FAIL (EVO-001)",
    "lint_odd neutral positive control: a neutral that cannot be typed, frozen, must FAIL "
    "(EVO-001)",
    "lint_odd neutral positive control: an enum neutral outside the column's codes must FAIL "
    "(EVO-001)",
    "lint_odd neutral positive control: an identity read carrying a neutral must FAIL "
    "(REV-EVO-001 R12)",
    "lint_odd next control: a next version with no amendment lints and plants nothing in the lock "
    "(REV-SPR-020 R26)",
    "lint_odd next positive control: a Registry overlay that moves a frozen parameter bound "
    "must FAIL (SPR-015)",
    "lint_odd next positive control: a new RNG purpose before a frozen one must FAIL (SPR-015)",
    "lint_odd next positive control: a next version that changes a frozen algorithm step must "
    "FAIL (SPR-015)",
    "lint_odd next positive control: an amendment whose text changed since the committed lock "
    "must FAIL (SPR-015)",
    "lint_odd positive control: a smoke overlay id that is not X-prefixed must FAIL (SPR-008 "
    "ruling 2)",
    "lint_odd positive control: a smoke spec missing from the overlay must FAIL (SPR-008 ruling 2)",
    "lint_patterns positive control: a [role_domains] entry of the wrong shape refuses the "
    "Registry by name (REV-SPR-020 R22, R26)",
    "lint_patterns positive control: life_ends with an unanchored read must FAIL (V-SIFT-05)",
    "population synthesis positive control: a spec check value off by one must FAIL the "
    "reference's spec check (SPR-019; REV-SPR-019 R5)",
    "population synthesis positive control: an altered golden output must FAIL (SPR-019)",
    "population synthesis positive control: conceptions inside the closed period must FAIL the "
    "spacing invariant (SPR-019 B6)",
    "population synthesis positive control: floors in place of the age-sex allocation must FAIL "
    "INV-PS-01 (SPR-019 P3)",
    "population synthesis positive control: floors in place of the couple-table rounding must "
    "FAIL INV-PS-03 (SPR-019 P7)",
    "rh-bench positive control: count-alloc and arena-alloc together must not compile (SPR-009 "
    "R-7)",
    "rh-core gates by name: V-DET-01/02 stub world, AccessSpec conflicts, skeletons, RNG purposes "
    "(SPR-005)",
    "rh-core gates by name: agenda, timers, lifecycle queues, event bus, V-DET-12, invariants and "
    "crash replay (SPR-006)",
    "rh-golden gates by name: resume = continuous, thread invariance, V-DET-11 bisect on injected "
    "perturbations (SPR-008 T-02, T-03)",
    "rh-golden gates by name: solo resume = continuous through 10-year keyframes, thread "
    "invariance, the manifest pin, Ada at genesis, the input's re-sample of her pending death "
    "(SPR-012 T-08)",
    "rh-ids gates by name: V-DET-16 fork trees, V-DET-20 stream independence, PR tier (SPR-002)",
    "rh-sample gates by name: V-DET-09 golden vectors at opt-level 0, statistics against the "
    "exact distributions (SPR-003)",
    "rh-snapshot gates by name: plane hashes, golden vectors, journal, keyframes, unsynced "
    "guard (SPR-007 T-01..T-04)",
    "rh-sys-smoke gates by name: registration against the smoke specs, coverage, re-sampling, "
    "small-scale V-DET-01/02 (SPR-008 T-01)",
    "selftest_envelope positive control: a motive on a triggered edge must FAIL (V-DATA-09; "
    "SPR-020)",
    "selftest_envelope positive control: a snapshot that is not the emitter's value at emission "
    "must FAIL (V-DATA-08; SPR-020)",
    "selftest_envelope positive control: a state value whose cause is a later event must FAIL "
    "(V-SAGA-21; SPR-020)",
    "selftest_envelope positive control: an input's consequence that loses the origin after one "
    "hop must FAIL (05.2 §4.0 criterion 4; REV-SPR-020 R14)",
    "selftest_envelope positive control: fifty witnesses on every event must FAIL the byte "
    "budget (V-SAGA-15; REV-SPR-020 R11)",
    "uv lock --check positive control: drifted lock must FAIL",
    "wire_v1 positive control: a host whose window moves before the dropped year is on disk must "
    "FAIL the live-edge rule (SPR-026)",
    "wire_v1 positive control: a stale golden frame must FAIL (SPR-026)",
    "worldgen-lite positive control: an altered golden output must FAIL (SPR-018)",
    "worldgen-lite positive control: an altered template number must FAIL against the golden "
    "outputs (SPR-018)",
)

# Functions the chain calls, each running a loop of controls. Their lists are pinned below
# (fixed point) and in test_lint_forbidden.py (FORBIDDEN_API_CONTROLS). Alphabetical.
REQUIRED_CHAIN_FUNCTIONS = (
    "fixed_point_compile_controls",
    "forbidden_api_controls",
)


def _chain_lines() -> list[str]:
    """Return the non-comment lines from the first top-level `run` to `status=$?`."""
    lines = SCRIPT.read_text(encoding="utf-8").splitlines()
    start = next(i for i, line in enumerate(lines) if line.startswith(("run ", "part ")))
    end = lines.index("status=$?")
    return [line for line in lines[start:end] if not line.lstrip().startswith("#")]


def test_the_inventory_lists_are_alphabetical_and_unique() -> None:
    """Sorted lists make a missing or duplicated entry easy to see in review."""
    for listed in (REQUIRED_GATES, REQUIRED_CHAIN_FUNCTIONS):
        assert list(listed) == sorted(set(listed))


def test_every_required_gate_runs_in_the_chain() -> None:
    """Deleting, commenting out or moving a gate out of the chain fails here."""
    chained = {m.group("name") for line in _chain_lines() if (m := RUN_NAME.match(line))}
    assert sorted(set(REQUIRED_GATES) - chained) == []


def test_every_chained_gate_is_in_the_inventory() -> None:
    """A new gate must be added to the inventory in the same change, or it is unguarded."""
    chained = {m.group("name") for line in _chain_lines() if (m := RUN_NAME.match(line))}
    assert sorted(chained - set(REQUIRED_GATES)) == []


def _first_word(rest: str) -> str:
    """Return the first shell word of ``rest``: bare, single-quoted, or double-quoted."""
    if rest.startswith("'"):
        return rest[1:].split("'", 1)[0]
    if not rest.startswith('"'):
        return rest.split(" ", 1)[0]
    out, i = [], 1
    while i < len(rest) and rest[i] != '"':
        if rest[i] == "\\" and i + 1 < len(rest):
            i += 1
        out.append(rest[i])
        i += 1
    return "".join(out)


def test_every_gate_states_a_non_empty_expectation() -> None:
    """VER-SPR-012-TEST GAP-6: a `pass:` or `fail:` with no text matches any output.

    Every `run` line of the script (the chain and the control loops) names an expectation that is
    `pass`, `fail`, or `pass:`/`fail:` followed by text; `run` itself refuses an empty text as a
    harness error (bin/check_all_selftest.py verdict --empty-fail).
    """
    seen = 0
    for line in SCRIPT.read_text(encoding="utf-8").splitlines():
        if not RUN_NAME.match(line):
            continue
        match = RUN_NAME.match(line)
        assert match is not None
        expect = _first_word(line[match.end() :].strip())
        assert expect in {"pass", "fail"} or (
            expect.startswith(("pass:", "fail:")) and len(expect) > len("pass:")
        ), f"{line.strip()[:80]}: expectation {expect!r}"
        seen += 1
    assert seen > 300, f"only {seen} run lines read"


def test_every_control_loop_is_called_from_the_chain() -> None:
    """The loop functions run many controls; each must still be called by the chain."""
    calls = {line.strip().removesuffix("&&").strip() for line in _chain_lines()}
    assert sorted(set(REQUIRED_CHAIN_FUNCTIONS) - calls) == []


def test_the_fixed_point_loop_covers_every_compile_fail_crate() -> None:
    """Each fixture crate of tests/lint_fixtures/compile_fail/fixed_point/ has its control."""
    script = SCRIPT.read_text(encoding="utf-8")
    listed = script.split("FIXED_POINT_COMPILE_FAIL='", 1)[1].split("'", 1)[0]
    crates = {line.split("|", 1)[0] for line in listed.splitlines() if line.strip()}
    on_disk = {p.parent.name for p in FIXED_POINT_FIXTURE.glob("*/Cargo.toml")}
    assert crates == on_disk - {"fixed-ok"}
    assert "-p fixed-ok --target-dir" in script, "the well-typed control must still compile"


def test_no_run_line_ends_in_or_true_or_exit() -> None:
    """GAP-3, GAP-7: `|| true`, `|| :` or `|| exit` on a `run` line would hide its failure."""
    text = SCRIPT.read_text(encoding="utf-8")
    assert check_all_selftest.lint(text) == []
    for plant in check_all_selftest.PLANTS.values():
        planted = check_all_selftest.lint(text, plant=plant)
        assert len(planted) == 1
        assert plant.removesuffix(" &&").strip() in planted[0]
    for suffix in (" || true &&", " || : &&", " ||true", " || :", " || exit 0 &&", " || exit"):
        lines = text.splitlines()
        first = next(i for i, line in enumerate(lines) if line.startswith("run "))
        lines[first] = lines[first].removesuffix(" &&") + suffix
        assert check_all_selftest.lint("\n".join(lines)) != [], suffix


def test_the_verdict_fails_when_any_gate_failed_or_the_chain_exits_early() -> None:
    """GAP-3, GAP-7: hidden failures and early exits still fail the tier; a clean chain passes.

    A failed gate hidden by `|| true` or `|| exit 0`, or an early `exit 0`, must each be FAIL.
    """
    assert check_all_selftest.verdict() == 0
    for mode in ("--or-true", "--or-exit", "--early-exit"):
        assert check_all_selftest.verdict(mode) == 1, mode


def test_the_baseline_push_controls_hold_with_an_absolute_target_dir() -> None:
    """VER-SPR-008-TEST DEF-3: no `$PWD/$GOLDEN` join; an absolute CARGO_TARGET_DIR works."""
    assert check_all_selftest.abs_target(plant=False) == 0
    assert check_all_selftest.abs_target(plant=True) == 1


def test_the_chain_has_two_parts_and_every_gate_belongs_to_exactly_one() -> None:
    """SPR-009 review 2b: `part core` opens the chain and `part golden` follows it, once each.

    Every `run` line comes after `part core`, so it belongs to the part whose marker is the last
    one above it: exactly one part. `--part` runs one, no flag both.
    """
    lines = [line.strip() for line in _chain_lines() if line.strip()]
    assert lines[0] == "part core &&"
    assert [line for line in lines if line.startswith("part ")] == [
        "part core &&",
        "part golden &&",
    ]
    golden = lines.index("part golden &&")
    names = [m.group("name") for line in lines if (m := RUN_NAME.match(line))]
    in_golden = {m.group("name") for line in lines[golden:] if (m := RUN_NAME.match(line))}
    assert len(names) == len(set(names)), "a gate name appears twice"
    # The golden runs and the perf gates are the golden part; the debug tests and Python the core.
    assert {n for n in names if n.startswith(("Golden run:", "V-PERF-04 instruction"))} <= in_golden
    assert {
        n for n in names if n.startswith(("Rust: unit tests", "Python: unit tests"))
    }.isdisjoint(in_golden)


# nextest options that take a value; any other bare word after `nextest run` is a test filter.
NEXTEST_VALUE_OPTIONS = {"-p", "--package", "--test", "--status-level", "-E", "--features"}


def _whole_binaries(line: str) -> bool:
    """Whether a by-name gate runs its binaries whole: a passing gate with no test filter.

    A positive control (a run that must fail) or a filtered run covers only part of a binary, so
    it never stands for the whole of it (VER-SPR-009-TEST GAP-20).
    """
    words = shlex.split(line.strip().removesuffix("&&"))
    if len(words) < 3 or not words[2].startswith("pass") or "nextest" not in words:
        return False
    args = words[words.index("nextest") + 2 :]
    bare = [
        w
        for i, w in enumerate(args)
        if not w.startswith("-") and (i == 0 or args[i - 1] not in NEXTEST_VALUE_OPTIONS)
    ]
    return not bare


def _by_name_test_binaries(*, release: bool) -> set[str]:
    """Nextest binary IDs the by-name gates run whole (with or without --release lines)."""
    found = set()
    for line in _chain_lines():
        if "cargo nextest run -p" not in line or ("--release" in line and not release):
            continue
        if not _whole_binaries(line):
            continue
        for package in NEXTEST_PACKAGE.findall(line):
            tests = NEXTEST_TARGET.findall(line)
            found |= {
                f"{package}::{test}"
                for test in tests
                if (REPO / "crates" / package / "tests" / f"{test}.rs").exists()
            }
            if "--lib" in line:
                found.add(package)
    return found


def test_a_filtered_run_or_a_control_is_not_coverage() -> None:
    """GAP-20: the V-DET-12 positive control (a filtered run that must fail) covers nothing."""
    control = next(line for line in _chain_lines() if "V-DET-12 positive control" in line)
    assert not _whole_binaries(control)
    filtered = next(line for line in _chain_lines() if line.startswith('run "V-DET-05 keyframes'))
    assert not _whole_binaries(filtered)
    whole = next(line for line in _chain_lines() if line.startswith('run "rh-sys-smoke gates'))
    assert _whole_binaries(whole)


def test_the_workspace_run_leaves_out_exactly_the_by_name_binaries() -> None:
    """Review 2a: each test binary runs once, in the workspace run or in its by-name gate.

    Every entry of BY_NAME_TESTS is run by a by-name gate (else it would run nowhere), and every
    binary a debug by-name gate runs is an entry (else it would run twice).
    """
    script = SCRIPT.read_text(encoding="utf-8")
    listed = set(script.split('BY_NAME_TESTS="', 1)[1].split('"', 1)[0].split())
    assert sorted(listed - _by_name_test_binaries(release=True)) == []
    assert sorted(_by_name_test_binaries(release=False) - listed) == []
    assert '-E "not ($filter)"' in script


def test_the_python_checks_run_before_any_rust_build() -> None:
    """GAP-18: the Python checks come before the first cargo build or clippy run.

    ruff, mypy, pytest and the hostile-CI pytest lead the core part, so a Python failure shows in
    about a minute.
    """
    names = [m.group("name") for line in _chain_lines() if (m := RUN_NAME.match(line))]
    python = [i for i, n in enumerate(names) if n.startswith("Python: ")]
    rust_builds = [
        i for i, n in enumerate(names) if n.startswith(("Rust: lint", "Rust: build", "Rust: unit"))
    ]
    assert len(python) == 6
    assert max(python) < min(rust_builds)
