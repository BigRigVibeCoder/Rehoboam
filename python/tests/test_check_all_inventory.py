"""The gate inventory of bin/check_all.sh: deleting a gate's line must turn this test red.

Every check a sprint adds to bin/check_all.sh is listed here by its exact name, and must appear as a
`run "<name>"` line inside the fail-fast chain (not in a comment, not after `status=$?`). Without
this list, removing a line leaves the tier green: nothing else notices that a gate stopped running
(SPR-010 GAP-4, the golden `--check` line; VER-SPR-001-TEST §4 item 2).

EVERY SPRINT ADDS ITS GATES HERE, in the same commit that adds them to bin/check_all.sh. Keep each
list alphabetical. Renaming a gate means renaming it here too, which is the point: a reviewer sees
the change.

The check runs both ways (BCK-002 A-65): every chained gate is listed and every listed gate is
chained, once each; every other chain call is a listed control loop and every listed loop is
called; and no `run` line sits outside the chain and those loops. bin/check_all_selftest.py
inventory does the same from the tier itself, reading these lists with `ast`, with two planted
controls: a gate added to the chain only, and the wrapped entry a three-way text merge once dropped
from this list (SPR-026's merge of PR #79).

Refs: SPR-010 GAP-4 (VER-SPR-010-TEST); GOV-002 §1 laws 4 and 9, §4; BCK-002 A-63 to A-66.
"""

import re
import shlex
from pathlib import Path

import check_all_selftest
import ci_scope
import pytest

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
    "--expect-serial positive control: the multi-system smoke world must FAIL it, naming the "
    "floor to set (SPR-013 T-13; the Architect's ruling)",
    "A-52 positive control: rh-cli run in a build with a mutant-* feature compiled in must FAIL",
    "Annals fixtures: the committed fixture chronicles equal the scripted history (gen_fixtures "
    "--check; SPR-029, REV-SPR-029 R10)",
    "Annals template lint: every slot resolves to a declared role, snapshot, payload slot, motive "
    "or edge; every in-scope kind has a template and every out-of-scope kind a label "
    "(lint_templates; SPR-029)",
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
    "Canonical differential positive control: a reference with its header domain check relaxed "
    "must FAIL (REV-SPR-013 MAJ-1)",
    "Canonical differential: rh-stream's parser and canonical_v0.py give every generated, "
    "golden, planted and mutated line, and every planted and header-mutated file, the same "
    "verdict (canonical_v0.md §3; REV-SPR-013 MAJ-1)",
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
    "Family annals v0: provenance re-derived by a whole-group re-render, template slots, no "
    "model, calendar, a possible history under M7's rules, determinism in two processes, "
    "coverage, injective designation, a canonical extract digest, golden annals and manifests "
    "(selftest_annals_v0 --check; SPR-029)",
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
    "Golden baseline: worlds/hamlet = the recomputation, changed only in a [C3] PR (V-GOLD-01; "
    "SPR-013 T-13)",
    "Golden baseline: worlds/household = the recomputation, changed only in a [C3] PR "
    "(V-GOLD-01; SPR-013 T-13)",
    "Golden baseline: worlds/smoke = the recomputation, changed only in a [C3] PR "
    "(check_baseline; SPR-008 T-04)",
    "Golden baseline: worlds/solo = the recomputation, changed only in a [C3] PR (V-GOLD-01; "
    "SPR-012 T-08)",
    "Golden cases: effect language v0 reference reproduces effect_language_golden_v0.toml "
    "(effect_ref --check; SPR-017)",
    "Golden draws power: a one-day error in q' either way changes re-sample outcomes of the "
    "larger fixture world (gen_m2_draws_reference --power; BCK-002 A-53, VER-SPR-012-TEST "
    "GAP-8)",
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
    "Golden ladders: hamlet at 1 thread = N threads, byte for byte (V-DET-01/02; SPR-013 T-13)",
    "Golden ladders: household at 1 thread = N threads, byte for byte (V-DET-01/02; SPR-013 T-13)",
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
    "Golden run: hamlet, 200 sim-years, yearly ladder, N threads, serial (SPR-013 T-13)",
    "Golden run: hamlet, the 60-person rung, 200 sim-years, yearly ladder, 1 thread, serial "
    "(--expect-serial), its committed input re-sampling founder 0's pending death, streaming "
    "into its run directory (SPR-013 T-13)",
    "Golden run: household, 200 sim-years, yearly ladder, N threads, serial (SPR-013 T-13)",
    "Golden run: household, the six-person rung, 200 sim-years, yearly ladder, 1 thread, serial "
    "(--expect-serial), its committed input re-sampling founder 0's pending death, streaming "
    "into its run directory (SPR-013 T-13)",
    "Golden run: smoke, 1,000 sim-years, yearly ladder, 1 thread, parallel levels at least 5% "
    "(SPR-008 T-02, DEF-2)",
    "Golden run: smoke, 1,000 sim-years, yearly ladder, N threads, parallel levels at least 5%, "
    "with the T-03 vitals observer attached (SPR-008 T-02, DEF-2; SPR-009 T-03)",
    "Golden run: solo, 200 sim-years, yearly ladder, 1 thread, the history tuple and the built "
    "and not-built systems in its header, streaming into its run directory (SPR-012 T-08; "
    "rulings 3, 16; SPR-013 T-09)",
    "Golden run: solo, 200 sim-years, yearly ladder, N threads (SPR-012 T-08)",
    "Golden thread-leak control: the same runs without the leak compare equal (VER-SPR-008 DEF-2)",
    "Golden thread-leak positive control: the planted thread-identity leak must FAIL the "
    "1-versus-N comparison (VER-SPR-008 DEF-2)",
    "Golden vectors: Brass (M2)",
    "Golden vectors: CRN pairing v0, the V-DET-17 worked computation on twin_vales, the PR tier "
    "(the first VALID seed of 1 to 4), with its five controls on the same run A (SPR-030; "
    "REV-SPR-030 R3, R4, R10; BCK-002 A-62)",
    "Golden vectors: M2 F1 fertility and partnering, exact reference (SPR-015)",
    "Golden vectors: M2 F1 mortality, exact reference and golden file (selftest_m2_f1_mortality "
    "--check; SPR-016)",
    "Golden vectors: M2 rung-0 mortality schedule, exact reference (SPR-011 ruling 7)",
    "Golden vectors: M2 rung-0 schedule vectors are the exact reference's (gen_m2_reference "
    "--check; SPR-012 T-05)",
    "Golden vectors: M4 basket and price rule",
    "Golden vectors: fixture worlds twin_vales (seeds 1-3) and kingdom_10k (seed 1) at genesis, "
    "exact reference, with the altered-golden control in the same run (SPR-030; REV-SPR-030 R6, "
    "R10)",
    "Golden vectors: minds v0 part A, the A-steps of T1 0.4.0, exact reference and golden file "
    "(selftest_minds_v0 --check; T1-A)",
    "Golden vectors: population synthesis v0 and the dynastic back-fill, exact reference, with the "
    "effect_ref differential, its reversed-line control, and the altered-golden and stale-spec "
    "controls in the same run (SPR-019; REV-SPR-019 R3, R5; REV-SPR-030 R10, R27)",
    "Golden vectors: rh-core::fixed are the exact reference's (gen_fixed_vectors --check)",
    "Golden vectors: rh-ids reference matches the committed file (gen_rng_golden --check; "
    "V-DET-09)",
    "Golden vectors: rh-snapshot plane hashes match the independent reference (gen_hash_golden "
    "--check; SPR-007 T-01)",
    "Golden vectors: worldgen-lite v0 and residence at genesis, exact reference (SPR-018)",
    "Instant log model v0: phases, levels, bindings and serial merges on eleven scripted "
    "scenarios; what each step sees in inserted(), exited() and the observed events, the "
    "merge-time form and its refusals, the genealogy index, thread invariance, the hash, resume; "
    "the transcript and the vectors current (instant_log_model --check; EVO-002)",
    "Keyframe sizes per rung: solo, household and hamlet's 200-year golden runs (SPR-013 T-13; "
    "reported)",
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
    "M2 F1 reference positive control: the elite read reverted to the woman's own row must FAIL "
    "F6's couple read (D56 (e); BCK-002 A-62)",
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
    "Manifest: kingdom_10k (the SPR-030 G1a performance world)",
    "Manifest: manor (the SPR-019 fixture: a dynastic manor)",
    "Manifest: solo",
    "Manifest: twin_vales (the SPR-030 dynastic fixture world)",
    "Manifest: uz_job",
    "Manifest: val_khor",
    "No FMA contraction: no fp-contract in RUSTFLAGS; C builds take exactly -ffp-contract=off "
    "(SPR-003 GAP-6; SPR-007 ruling 4)",
    "Observer invariance: an attached observer changes no monthly state hash at 1 and 4 threads "
    "(SPR-009 ruling 4; V-DET-07 style)",
    "Ownership lint positive control: a write declared twice must FAIL (SPR-005 DEF-3)",
    "Ownership lint positive control: unowned write must FAIL",
    "Ownership lint: single writer from specs/systems (SPR-004 T-03)",
    "Population projection: the fixture worlds' member share and population under the "
    "house-member mechanics, uncapped and capped (SPR-030; REV-SPR-030 R1)",
    "Python: format (ruff format --check)",
    "Python: lint (ruff check)",
    "Python: types (mypy --strict)",
    "Python: unit tests (pytest)",
    "Python: unit tests with hostile CI variables set (RH_EVENT, RH_PR_TITLE, "
    "GITHUB_EVENT_NAME, RH_PUSH_*) pass: conftest.py isolates them (SPR-009 review 1)",
    "Python: uv.lock matches pyproject.toml (uv lock --check)",
    "Replay positive control: a journal with its row deleted must hard-FAIL the replay (SPR-013 "
    "T-12)",
    "Required tools present (core part)",
    "Required tools present (golden part)",
    "Required tools present (stream part)",
    "Rust: bans and sources (cargo deny: crates.io only, no wildcards)",
    "Rust: build rh-cli in release for the golden runs (SPR-008 T-02)",
    "Rust: build rh-cli in release, with overflow checks, for the stream part (BCK-002 A-63; PR "
    "#89 review C5)",
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
    "Sift: solo's run, downstream of the engine: one channel-C segment per channel-A segment "
    "(SPR-013 T-11; ruling 14)",
    "Sifting patterns (lint_patterns)",
    "Sifting patterns: solitary_life pack, life_ends (lint_patterns; SPR-011 ruling 14)",
    "Smoke-world skeletons: committed files = lint_odd.py --rust (gen_skeletons --check; "
    "SPR-008 ruling 2)",
    "Smoke-world specs (lint_odd --specs worlds/smoke/systems --overlay; SPR-008 ruling 2)",
    "Stream part input: solo, household and hamlet's 200-year golden runs streamed into their run "
    "directories, as the golden part streams them (BCK-002 A-63)",
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
    "T-10: a stopped tail does not slow rh-cli run: solo's 200-year wall time headless, "
    "streaming, and streaming with a tail stopped mid-run (after it printed a sealed segment), "
    "within twice streaming plus 1 s",
    "V-DATA-10 subset on the envelope fixture: DuckDB reads its event_snapshot rows and typed "
    "causal_edge rows (motive, weight, evidence, cause_told) with the DDL's types and "
    "constraints, and the reference recomputes every canonical_digest from them (SPR-013 T-09; "
    "SPR-020 shape)",
    "V-DATA-10 subset: DuckDB reads every sealed table of solo's run with channel_a_v0.sql's "
    "and schema.sql's types, no float column, the DDL's constraints, contiguous seq, R4 order, "
    "and the reference's canonical digests (SPR-013 T-09)",
    "V-DET-04: rh-cli replay rebuilds solo's 200-year run from its run.toml and journal (Ada's "
    "input) at N threads: ladder, every channel-A segment and keyframe byte-identical (SPR-013 "
    "T-12)",
    "V-DET-05 keyframes: a loaded keyframe advances 10 years like the continuous run (SPR-007 "
    "T-02)",
    "V-DET-05: rh-cli replay from solo's year-100 keyframe = the run (SPR-013 T-12)",
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
    "V-SIFT-01 on hamlet: sift its golden run, the twin = rh-sifter (SPR-013 T-11, T-13)",
    "V-SIFT-01 on household: sift its golden run, the twin = rh-sifter (SPR-013 T-11, T-13)",
    "V-SIFT-01 on solo: the SQL twin of life_ends = rh-sifter's channel C (match ids, bindings, "
    "event ids, closing seqs) and channel C's chain of channel-A digests (SPR-013 T-11)",
    "V-SIFT-01 on the 10,000-founder fixture: stream 120 years, sift, the twin = rh-sifter "
    "(SPR-013 T-11)",
    "V-WC-04 manifest parity: Rust loader agrees with manifest_v0.py (SPR-004)",
    "V-WC-04 parity positive control: drifted Registry must FAIL",
    "V-WC-04 parity positive control: message-only drift must FAIL (GAP-3)",
    "Virtual telemetry (S5 prototype)",
    "Workspace tests positive control: a by-name entry that names no test binary must FAIL "
    "(SPR-009 review 2a)",
    "World Stream control channel v1: the method table, every request and response, refusals, "
    "the governor's rule, pace invariance on a simulated engine with commands landing mid-instant "
    "and floods of no-change commands, the run log, golden frames (selftest_control_v1 --check; "
    "SPR-027 T-01)",
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
    "check_all baseline controls positive control: controls fed the committed baseline, as before "
    "A-66, must FAIL on a stale baseline (BCK-002 A-66)",
    "check_all baseline controls: on a stale baseline (a C3 head before its regeneration commit) "
    "every Golden baseline control fails or passes for its own reason (check_all_selftest; BCK-002"
    " A-66)",
    "check_all baseline push controls with an absolute CARGO_TARGET_DIR fail and pass for the "
    "right reason (check_all_selftest; VER-SPR-008-TEST DEF-3)",
    "check_all chain positive control: a run line mutated with || exit 0 must FAIL the lint "
    "(VER-SPR-008-TEST GAP-7)",
    "check_all chain positive control: a run line mutated with || true must FAIL the lint "
    "(VER-SPR-008-TEST GAP-3)",
    "check_all chain: no run line ends in || true, || : or || exit (check_all_selftest lint; "
    "VER-SPR-008-TEST GAP-3, GAP-7)",
    "check_all harness positive control: a control that fails for its own reason but read an "
    "undeclared input (SCOPE INCOMPLETE) must FAIL (PR #89 review C2)",
    "check_all harness positive control: a fail: expectation with empty text must FAIL as a harness"
    " error, not match any failure (VER-SPR-012-TEST GAP-6)",
    "check_all harness positive control: a fail: expectation with whitespace-only text must "
    "FAIL as a harness error (VER-SPR-012-TEST GAP-9; BCK-002 A-53)",
    "check_all inventory positive control: a planted unlisted gate must FAIL (BCK-002 A-65)",
    "check_all inventory positive control: the wrapped entry a three-way merge once dropped, "
    "deleted from the inventory, must FAIL (BCK-002 A-65)",
    "check_all inventory: every chain gate is listed and every listed gate chained, every chain "
    "call a listed loop, and no run line outside the chain and its loops (check_all_selftest; "
    "BCK-002 A-65)",
    "check_all positive control: the old PWD-joined golden path under an absolute "
    "CARGO_TARGET_DIR must FAIL (VER-SPR-008-TEST DEF-3)",
    "check_all verdict per part positive control: --part golden fails at its failing gate "
    "(SPR-009 review 2b)",
    "check_all verdict per part: --part core passes while only a golden-part gate fails "
    "(check_all_selftest; SPR-009 review 2b)",
    "check_all verdict per part: --part stream passes while only a golden-part gate fails "
    "(check_all_selftest; BCK-002 A-63)",
    "check_all verdict positive control: a failed gate followed by || exit 0 must still FAIL "
    "the tier (VER-SPR-008-TEST GAP-7)",
    "check_all verdict positive control: a failed gate hidden by || true must still FAIL the "
    "tier (VER-SPR-008-TEST GAP-3)",
    "check_all verdict positive control: an exit 0 before the chain's end must FAIL the tier "
    "(VER-SPR-008-TEST GAP-7)",
    "check_all verdict positive control: with no --part, a failing gate of the last part must FAIL"
    " the tier, so every part runs (BCK-002 A-63)",
    "check_all verdict: a chain of passing gates exits 0 (check_all_selftest; VER-SPR-008-TEST "
    "GAP-3)",
    "check_segments positive control: a DOUBLE column planted in a sealed table must FAIL "
    "(SPR-013 T-09; R2)",
    "check_segments positive control: an altered canonical_digest must FAIL (SPR-013 T-09; R7)",
    "ci_scope control: a change to a scoped gate's own text in check_all.sh runs the gate (BCK-002"
    " A-64)",
    "ci_scope control: a change to check_all.sh outside a scoped gate's own text in it skips the "
    "gate (BCK-002 A-64)",
    "ci_scope control: a change to none of a scoped gate's inputs skips it, with its reason "
    "(BCK-002 A-64)",
    "ci_scope control: a planted change to one fixture-worlds input runs the gate (BCK-002 A-64)",
    "ci_scope control: a planted change to one population-synthesis input, deep in what the "
    "reference loads, runs the gate (BCK-002 A-64)",
    "ci_scope trace positive control: a scoped gate that reads a file outside its declared inputs "
    "must FAIL (BCK-002 A-64)",
    "control_v1 positive control: a band mixed into the state hash must FAIL pace invariance "
    "(SPR-027 T-01)",
    "control_v1 positive control: a pace command journaled as an input must FAIL (STR-5; "
    "REV-SPR-026 R12; SPR-027 T-01)",
    "control_v1 positive control: a pause that writes a record must FAIL pace invariance (W35; "
    "SPR-027 T-01)",
    "control_v1 positive control: a resume that owes the paused wall time must FAIL (SPR-027 T-01)",
    "control_v1 positive control: a sixth method on the control table must FAIL the method-table "
    "check (REV-SPR-027-control C4; SPR-027 T-01)",
    "control_v1 positive control: a stale golden frame must FAIL (SPR-027 T-01)",
    "control_v1 positive control: a step that pauses one instant past its target must FAIL (W37; "
    "SPR-027 T-01)",
    "control_v1 positive control: an input (player.act) accepted on control.sock must FAIL the "
    "method table (D47; SPR-027 T-01)",
    "control_v1 positive control: control.pace on the stream method table must FAIL (wire_v1 "
    "§4.5; SPR-027 T-01)",
    "effect_ref positive control: one altered expected value must FAIL (SPR-017)",
    "exp_ln_v0 positive control: a stale golden file must FAIL (SPR-016)",
    "fixture worlds positive control: dormancy_degrees read as 1 must FAIL the named-at-start "
    "lines (SPR-030; REV-SPR-030 R11)",
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
    "instant_log_model positive control: M7's naming criteria without the manifest's cast, so that "
    "tick 0's pass demotes a cast member, must FAIL (EVO-002; X4)",
    "instant_log_model positive control: a genealogy index kept only at the instant's end must "
    "FAIL (EVO-002)",
    "instant_log_model positive control: a log not cleared at the instant's end must FAIL "
    "(EVO-002)",
    "instant_log_model positive control: a state hash that covers the log must FAIL (EVO-002)",
    "instant_log_model positive control: an append in place of exited()'s sorted insert must FAIL "
    "the order check, a later merge's death having the lower id (EVO-002; R9; T-00a MAJ-1)",
    "instant_log_model positive control: an append in place of inserted()'s sorted insert must "
    "FAIL the order check (EVO-002)",
    "instant_log_model positive control: jobs that merge as they finish, so a later job sees its "
    "level's peers, must FAIL thread invariance (EVO-002)",
    "instant_log_model positive control: the merge-time form resolved against the log at the job's "
    "own merge must FAIL visibility (EVO-002)",
    "instant_log_model positive control: vectors with one state hash altered must FAIL as stale "
    "(EVO-002; T-00a MIN-5)",
    "kill -9 by name: a kill during sealing leaves no partial segment and the resume reseals "
    "the same bytes; a mid-year kill re-emits no seq below the first unsealed year (SPR-013 "
    "T-09; GOV-002 tier 15; wire_v1.md §11)",
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
    "lint_arch positive control: a crate neither named rh-sys-* nor pinned declaring kind system "
    "must FAIL, beside rh-minds passing by its pin (T1-A; REV-MINDS finding 3)",
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
    "lint_arch positive control: rh-minds relabelled out of kind system must FAIL (T1-A; "
    "REV-MINDS finding 3)",
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
    "lint_forbidden float-dep positive control: parquet's Field::Float in rh-stream must FAIL "
    "(SPR-013; the Architect's parquet review)",
    "lint_forbidden float-dep positive control: parquet's get_double in rh-stream must FAIL "
    "(SPR-013; the Architect's parquet review)",
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
    "must FAIL, rh-registry included since A-51, and in an rh-stream file other than seal.rs; "
    "tests/, rh-snapshot and rh-stream/src/seal.rs pass (BCK-002 A-44, A-51; SPR-013)",
    "lint_forbidden positive control: float threshold in a plane-A crate must FAIL (SPR-010 T-04)",
    "lint_forbidden positive control: fs::read after a grouped import is reported on its own "
    "line (DEF-1)",
    "lint_forbidden positive control: include!(concat!(...)) must FAIL (SPR-008 T-00)",
    "lint_forbidden positive control: include_str! of a file outside the repository must FAIL "
    "(VER-SPR-008)",
    "lint_forbidden positive control: mul_add in rh-sample must FAIL (SPR-003 T-00)",
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
    "lint_odd next positive control: a frozen command why changed without a record must FAIL (R3)",
    "lint_odd next positive control: a frozen coupling via changed without a record must FAIL (R3)",
    "lint_odd next positive control: a frozen design concept changed without a record "
    "must FAIL (R3)",
    "lint_odd next positive control: a frozen fidelity-ladder representation changed "
    "without a record must FAIL (R3)",
    "lint_odd next positive control: a frozen isolation expectation changed without a "
    "record must FAIL (R3)",
    "lint_odd next positive control: a frozen parameter source changed without a record "
    "must FAIL (R3)",
    "lint_odd next positive control: a frozen purpose statement changed without a record "
    "must FAIL (R3)",
    "lint_odd next positive control: a frozen scales field changed without a record must FAIL (R3)",
    "lint_odd next positive control: a frozen submodel summary changed without a record "
    "must FAIL (deny by default; PR #79 review R3)",
    "lint_odd next positive control: a new RNG purpose before a frozen one must FAIL (SPR-015)",
    "lint_odd next positive control: a next version that changes a frozen algorithm step must "
    "FAIL (SPR-015)",
    "lint_odd next positive control: an amendment whose text changed since the committed lock "
    "must FAIL (SPR-015)",
    "lint_odd next positive control: an applied command's drained_by moved to Year/Polity "
    "must FAIL (N4)",
    "lint_odd next positive control: an enum code renamed must FAIL (deny by default; PR "
    "#79 review N4)",
    "lint_odd next positive control: the frozen from_manifest list edited must FAIL (N4)",
    "lint_odd next positive control: the frozen requirement ids changed without a record "
    "must FAIL (R3)",
    "lint_odd next positive control: the person.sex read moved from same_instant to "
    "lagged must FAIL (N4)",
    "lint_odd next positive control: the reviewer's q5_frozen_onbirth case, a far adult "
    "heir made dormant-eligible, must FAIL (R3)",
    "lint_odd next positive control: the reviewer's q5_summary case, on_birth's rule made "
    "cognatic again in the summary, must FAIL (R3)",
    "lint_odd next positive control: the reviewer's q5_why case, Demote's why applied "
    "to minors, must FAIL (R3)",
    "lint_odd observes control: an [[observes]] entry naming a Registry verb, its emitter and a "
    "same_instant read of the emitter passes (EVO-002)",
    "lint_odd observes positive control: a kind observed twice must FAIL (EVO-002; T-00a MIN-8)",
    "lint_odd observes positive control: a system observing its own kind must FAIL (EVO-002; T-00a "
    "MIN-8)",
    "lint_odd observes positive control: an entry that is not a table must FAIL (EVO-002; T-00a "
    "MIN-8)",
    "lint_odd observes positive control: an entry with an unknown field must FAIL (EVO-002; T-00a "
    "MIN-8)",
    "lint_odd observes positive control: an entry without its why must FAIL (EVO-002; T-00a MIN-8)",
    "lint_odd observes positive control: an observed kind that is not a Registry verb must FAIL "
    "(EVO-002)",
    "lint_odd observes positive control: an observed kind whose emitter is none of the system's "
    "same_instant reads must FAIL (EVO-002)",
    "lint_odd observes positive control: an observed kind whose owner is not its emitter must FAIL "
    "(EVO-002)",
    "lint_odd observes positive control: an observer bound only before its emitter's phase must "
    "FAIL (EVO-002; T-00a MIN-8)",
    "lint_odd positive control: a smoke overlay id that is not X-prefixed must FAIL (SPR-008 "
    "ruling 2)",
    "lint_odd positive control: a smoke spec missing from the overlay must FAIL (SPR-008 ruling 2)",
    "lint_patterns positive control: a [role_domains] entry of the wrong shape refuses the "
    "Registry by name (REV-SPR-020 R22, R26)",
    "lint_patterns positive control: life_ends with a declared role that no clause binds must "
    "FAIL (VER-SPR-013-TEST GAP-4; parity with rh-sifter)",
    "lint_patterns positive control: life_ends with an unanchored read must FAIL (V-SIFT-05)",
    "lint_templates positive control: a cause slot in a label must FAIL (SPR-029)",
    "lint_templates positive control: a control character in a template text must FAIL "
    "(REV-SPR-029 R27)",
    "lint_templates positive control: a father slot on a kind other than BIRTH must FAIL "
    "(REV-SPR-029 R6)",
    "lint_templates positive control: a kind whose last variant is conditional must FAIL (SPR-029)",
    "lint_templates positive control: a motive slot on a kind that declares no motives must FAIL "
    "(SPR-029)",
    "lint_templates positive control: a payload citation that is not the whole clause of the "
    "emitting [[emits]] must FAIL (REV-SPR-029 F12)",
    "lint_templates positive control: a payload slot the layout does not declare must FAIL "
    "(SPR-029)",
    "lint_templates positive control: a prologue slot outside its own template's vocabulary must "
    "FAIL (REV-SPR-029 R27)",
    "lint_templates positive control: a prologue template with an unknown id must FAIL "
    "(REV-SPR-029 R27)",
    "lint_templates positive control: a slot naming a role the kind does not declare must FAIL "
    "(SPR-029)",
    "lint_templates positive control: a slot that reads world state must FAIL (ADR-0024; SPR-029)",
    "lint_templates positive control: a slot with a conversion or format spec must FAIL "
    "(REV-SPR-029 F14)",
    "lint_templates positive control: a snapshot slot the kind does not declare must FAIL "
    "(SPR-029)",
    "lint_templates positive control: an in-scope kind without a template must FAIL (SPR-029)",
    "lint_templates positive control: an out-of-scope kind an emitter declares without a "
    "[[label]] must FAIL (REV-SPR-029 R3)",
    "lint_templates positive control: an unconditional variant that is not last must FAIL "
    "(REV-SPR-029 F14)",
    "lint_templates positive control: enum ids that differ from the emitter's column codes must "
    "FAIL (REV-SPR-029 F13)",
    "minds v0 mutant: opinion-no-expiry changes the probe at the modifier's expiry (SPR-023 T-04; "
    "T1-A)",
    "minds v0 mutant: relation-not-ended changes the first monthly refresh's end line (SPR-023 "
    "T-04; T1-A)",
    "minds v0 mutant: repromote-redraw changes the re-promoted mind's golden line (SPR-023 T-03; "
    "T1-A)",
    "minds v0 mutant: trait-k-swap changes the first created mind's golden line (SPR-023 T-03; "
    "T1-A)",
    "minds v0 positive control: a Demote that cancels an owed DEATH_DUE or MARRY_DUE must FAIL A7 "
    "(REV-MINDS M6; T1-A)",
    "minds v0 positive control: a re-promotion that redraws the traits must FAIL INV-T1-05 "
    "(REV-P1a MAJ-9; T1-A)",
    "minds v0 positive control: a stale check value in the next T1 spec must FAIL (T1-A)",
    "minds v0 positive control: a trait draw keyed at the wrong instant must FAIL INV-T1-05 (A3; "
    "T1-A)",
    "minds v0 positive control: an altered golden line must FAIL (T1-A)",
    "minds v0 positive control: an index lookup that misses rows its own binding appended must "
    "FAIL A2 (REV-MINDS M7; T1-A)",
    "minds v0 positive control: genesis naming more than the manifest's cast must FAIL INV-T1-11 "
    "(Architect ruling X4; T1-A)",
    "population synthesis positive control: conceptions inside the closed period must FAIL the "
    "spacing invariant (SPR-019 B6)",
    "population synthesis positive control: floors in place of the age-sex allocation must FAIL "
    "INV-PS-01 (SPR-019 P3)",
    "population synthesis positive control: floors in place of the couple-table rounding must "
    "FAIL INV-PS-03 (SPR-019 P7)",
    "rh-bench positive control: count-alloc and arena-alloc together must not compile (SPR-009 "
    "R-7)",
    "rh-cli build-info: this build compiles in no mutant-* feature (BCK-002 A-52)",
    "rh-core gates by name: V-DET-01/02 stub world, AccessSpec conflicts, skeletons, RNG purposes "
    "(SPR-005)",
    "rh-core gates by name: agenda, timers, lifecycle queues, event bus, V-DET-12, invariants and "
    "crash replay (SPR-006)",
    "rh-golden by name: V-DET-04/18 on solo and hamlet (run.toml and the journal alone rebuild "
    "the run byte for byte), V-DET-05 from keyframes at 1 and 4 threads, V-DET-14 (an input "
    "changes only its founder's records), and missing, extra, edited and re-chained journal "
    "rows hard-FAIL (SPR-013 T-12)",
    "rh-golden by name: V-PLN-04 subset: sifting twice is byte-identical and leaves channel A, "
    "keyframes, journal and ladder unchanged; one life_ends match per death, shown by tail "
    "(SPR-013 T-11)",
    "rh-golden by name: V-SCALE-04 PR subset: household and hamlet each reach zero population "
    "with one EXTINCTION, nothing but digests and keyframes after it, and their chronicle, "
    "tail, sift and replay stay consistent past it; one rh-cli build runs rungs 0, 1 and 2 "
    "(SPR-013 T-13)",
    "rh-golden by name: V-STR-01 on solo and the 60-person hamlet (byte-identical segments "
    "twice and at 1, 2 and 4 threads), the streaming ladder = the headless one, resume "
    "verification (SPR-013 T-09)",
    "rh-golden by name: tail prints every record once in seq order from any cursor, its "
    "canonical output digests to each segment's canonical_digest, it never writes to the run "
    "directory, and the display form matches its snapshots (SPR-013 T-10)",
    "rh-golden gates by name: resume = continuous, thread invariance, V-DET-11 bisect on injected "
    "perturbations (SPR-008 T-02, T-03)",
    "rh-golden gates by name: solo resume = continuous through 10-year keyframes, thread "
    "invariance, the manifest pin, Ada at genesis, the input's re-sample of her pending death "
    "(SPR-012 T-08)",
    "rh-ids gates by name: V-DET-16 fork trees, V-DET-20 stream independence, PR tier (SPR-002)",
    "rh-sample gates by name: V-DET-09 golden vectors at opt-level 0, statistics against the "
    "exact distributions (SPR-003)",
    "rh-sifter by name: the golden channel-A segment sifts to the golden life_ends matches and "
    "MatchIds; several targets give several matches; what v0 cannot honour is refused (SPR-013 "
    "T-11)",
    "rh-snapshot gates by name: plane hashes, golden vectors, journal, keyframes, unsynced "
    "guard (SPR-007 T-01..T-04)",
    "rh-stream by name: byte-identical sealing over seeds and world sizes, contiguous seq, R4 "
    "order refused when broken, resume verification, tampered segments refused (SPR-013 T-09)",
    "rh-stream by name: the canonical form reproduces the reference's golden lines, digests and "
    "match_id vectors; property round trip over seeds and sizes; planted refusals; the PR fuzz "
    "tier (SPR-013 T-09)",
    "rh-stream by name: the pinned Parquet writer properties in every file, no float column, "
    "the golden segment's bytes pinned (SPR-013 T-09; the Architect's parquet review)",
    "rh-sys-smoke gates by name: registration against the smoke specs, coverage, re-sampling, "
    "small-scale V-DET-01/02 (SPR-008 T-01)",
    "selftest_annals positive control: a body sentence retagged as a prologue row must FAIL "
    "provenance (REV-SPR-029 R2 P3)",
    "selftest_annals positive control: a fixture claim with no triggered edge to a succession or "
    "deposition must FAIL the possible-history check (REV-SPR-029 R26)",
    "selftest_annals positive control: a fixture deposition moved to a month's first day, its "
    "claims with it, must FAIL the possible-history check (REV-SPR-029 R26, R37)",
    "selftest_annals positive control: a fixture vassal crisis while its liege is held must FAIL "
    "the possible-history check (REV-WAVE7 K2; AN-1)",
    "selftest_annals positive control: a forged source row on a sidecar row must FAIL provenance "
    "(REV-SPR-029 R19 P7b, R37)",
    "selftest_annals positive control: a golden file no rendered group owns must FAIL as an "
    "orphan (REV-SPR-029 R39)",
    "selftest_annals positive control: a network module imported during the render must FAIL "
    "(V-NAR-07; REV-SPR-029 F16)",
    "selftest_annals positive control: a prologue row citing a keyframe row that does not exist "
    "must FAIL provenance (REV-SPR-029 R2 P4)",
    "selftest_annals positive control: a prologue row citing another house's root must FAIL "
    "provenance (REV-SPR-029 R19 P13, R37)",
    "selftest_annals positive control: a row citing an unrelated event must FAIL provenance "
    "(REV-SPR-029 R2 P5)",
    "selftest_annals positive control: a row that drops its own event must FAIL provenance "
    "(REV-SPR-029 R2 P1)",
    "selftest_annals positive control: a sentence with no sidecar row must FAIL provenance "
    "(V-NAR-05; SPR-029)",
    "selftest_annals positive control: a shared name left undisambiguated must FAIL injective "
    "designation (REV-SPR-029 R24, R33)",
    "selftest_annals positive control: a slot outside DEATH's declared roles must FAIL (SPR-029)",
    "selftest_annals positive control: a stale golden annal must FAIL (SPR-029)",
    "selftest_annals positive control: a template that reads world state must FAIL (ADR-0024; "
    "SPR-029)",
    "selftest_annals positive control: a text shorter than the render must FAIL provenance "
    "(REV-SPR-029 R34 P23)",
    "selftest_annals positive control: an altered prologue text must FAIL provenance (REV-SPR-029 "
    "R19 P12, R37)",
    "selftest_annals positive control: two rows with their contents exchanged must FAIL "
    "provenance (REV-SPR-029 R2 P6)",
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
    "tail by name: a tail started before a run's first seal and following it prints the "
    "after-run tail's lines, display and canonical, solo and hamlet (SPR-013 T-10; V-STR-05 "
    "sealed)",
    "tail snapshot positive control: a missing snapshot must FAIL, never be created silently "
    "(SPR-013 T-10; the file-snapshot ruling)",
    "tail snapshot positive control: a stale snapshot must FAIL with the first differing line "
    "(SPR-013 T-10)",
    "twin_check positive control: a twin expecting another status (a column the match id does "
    "not cover) must FAIL (REV-SPR-013 MIN-2)",
    "twin_check positive control: a twin missing its last match must FAIL (SPR-013 T-11)",
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


def test_the_inventory_and_the_chain_agree_both_ways() -> None:
    """BCK-002 A-65: the selftest's check, over this module's lists, finds nothing."""
    text = SCRIPT.read_text(encoding="utf-8")
    assert check_all_selftest.inventory(text, REQUIRED_GATES, REQUIRED_CHAIN_FUNCTIONS) == []
    assert check_all_selftest.listed(Path(__file__).read_text(encoding="utf-8")) == (
        REQUIRED_GATES,
        REQUIRED_CHAIN_FUNCTIONS,
    )


def test_a_gate_missing_from_either_side_fails() -> None:
    """A-65 controls: a planted unlisted gate, and the wrapped entry a merge once dropped."""
    assert check_all_selftest.main(["inventory", "--plant-unlisted"]) == 1
    assert check_all_selftest.main(["inventory", "--plant-dropped"]) == 1
    text = SCRIPT.read_text(encoding="utf-8")
    dropped = tuple(g for g in REQUIRED_GATES if g != check_all_selftest.DROPPED)
    assert len(dropped) == len(REQUIRED_GATES) - 1
    found = check_all_selftest.inventory(text, dropped, REQUIRED_CHAIN_FUNCTIONS)
    assert found == [f"UNLISTED: {check_all_selftest.DROPPED}"]
    extra = (*REQUIRED_GATES, "a listed gate no chain line runs")
    found = check_all_selftest.inventory(text, extra, REQUIRED_CHAIN_FUNCTIONS)
    assert found == ["NOT IN THE CHAIN: a listed gate no chain line runs"]


def test_a_stray_call_or_run_line_fails() -> None:
    """A-65: a gate run from an unlisted function, or after the chain's end, is unguarded."""
    text = SCRIPT.read_text(encoding="utf-8")
    stray = text.replace("part core &&\n", "part core &&\nmy_new_controls &&\n", 1)
    assert check_all_selftest.inventory(stray, REQUIRED_GATES, REQUIRED_CHAIN_FUNCTIONS) == [
        "UNLISTED CALL: my_new_controls"
    ]
    after = text.replace("chain_done=1\n", 'chain_done=1\nrun "late gate" pass true\n', 1)
    found = check_all_selftest.inventory(after, REQUIRED_GATES, REQUIRED_CHAIN_FUNCTIONS)
    assert len(found) == 1
    assert found[0].startswith("RUN OUTSIDE THE CHAIN")
    single = text.replace("part core &&\n", "part core &&\nrun 'quoted gate' pass true &&\n", 1)
    found = check_all_selftest.inventory(single, REQUIRED_GATES, REQUIRED_CHAIN_FUNCTIONS)
    assert len(found) == 1
    assert found[0].startswith("UNPARSED RUN LINE")


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
            expect.startswith(("pass:", "fail:")) and expect[len("pass:") :].strip()
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


def test_a_control_that_read_an_undeclared_input_fails(capsys: pytest.CaptureFixture[str]) -> None:
    """PR #89's review C2: SCOPE INCOMPLETE fails a gate whose expectation does not name it.

    The planted control fails with its own expected text, so only that rule fails it; the chain's
    verdict control expects that failure by the planted name. The trace's own positive control
    names SCOPE INCOMPLETE in its expectation, and still passes.
    """
    assert check_all_selftest.verdict("--scope-incomplete") == 1
    out = capsys.readouterr().out
    planted = "planted control whose output says SCOPE INCOMPLETE"
    assert f"FAIL  {planted} (SCOPE INCOMPLETE: an undeclared input" in out
    assert f'Verdict: FAIL at "{planted}"' in out
    control = [line for line in _chain_lines() if "verdict --scope-incomplete" in line]
    assert len(control) == 1
    assert f"'fail:Verdict: FAIL at \"{planted}\"'" in control[0]
    # Without the rule, the planted control would pass: it fails with its own text.
    assert check_all_selftest.verdict() == 0
    named = [
        line
        for line in _chain_lines()
        if "SCOPE INCOMPLETE" in line and line.startswith('run "ci_scope trace positive control')
    ]
    assert len(named) == 1
    assert '"fail:SCOPE INCOMPLETE: ' in named[0]


def test_the_stream_part_builds_with_overflow_checks() -> None:
    """PR #89's review C5: part stream turns release overflow checks on; its build gate checks."""
    script = SCRIPT.read_text(encoding="utf-8")
    part = script.split("\npart() {\n", 1)[1].split("\n}\n", 1)[0]
    assert (
        'if [ "$1" = stream ]; then export CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true; fi' in part
    )
    stream = script.split("\npart stream &&\n", 1)[1]
    assert 'pass:rh-cli release build with overflow checks" stream_release_build &&' in stream


def test_the_baseline_push_controls_hold_with_an_absolute_target_dir() -> None:
    """VER-SPR-008-TEST DEF-3: no `$PWD/$GOLDEN` join; an absolute CARGO_TARGET_DIR works."""
    assert check_all_selftest.abs_target(plant=False) == 0
    assert check_all_selftest.abs_target(plant=True) == 1


def _parts() -> dict[str, set[str]]:
    """Each part's gate names, by the `part` marker above them."""
    parts: dict[str, set[str]] = {}
    current = ""
    for line in _chain_lines():
        if line.startswith("part "):
            current = line.split()[1]
            parts.setdefault(current, set())
        elif m := RUN_NAME.match(line):
            parts[current].add(m.group("name"))
    return parts


# Gate-name prefixes pinned to their part (BCK-002 A-63): moving one of these gates to another part
# is a reviewed change here. The stream part takes the stream, serve, tail, sift, replay, rungs and
# twin gates; the golden part the golden runs, baselines, ladders and perf gates.
PART_OF_PREFIX = {
    "golden": (
        "Golden run:",
        "Golden ladders:",
        "Golden baseline",
        "Golden ladder ",
        "V-PERF-04",
        "V-PERF-11",
        "Baseline integrity",
        "T-07 M2.demography in release",
        "V-DET-10 release build",
    ),
    "stream": (
        "Stream part input",
        "V-DATA-10 subset: DuckDB reads every sealed table of solo's run",
        "check_segments positive control",
        "kill -9 by name",
        "tail by name",
        "tail snapshot positive control",
        "Sift:",
        "V-SIFT-01",
        "twin_check positive control",
        "rh-golden by name:",
        "V-DET-04:",
        "V-DET-05:",
        "Replay positive control",
        "T-10: a stopped tail",
    ),
    "core": ("Rust: unit tests", "Python: unit tests", "Rust: build the workspace"),
}


def test_the_chain_has_three_parts_and_every_gate_belongs_to_exactly_one() -> None:
    """SPR-009 review 2b, BCK-002 A-63: `part core`, `part golden`, `part stream`, once each.

    Every `run` line comes after `part core`, so it belongs to the part whose marker is the last
    one above it: exactly one part. `--part` runs one, no flag all three in this order.
    """
    lines = [line.strip() for line in _chain_lines() if line.strip()]
    assert lines[0] == "part core &&"
    assert [line for line in lines if line.startswith("part ")] == [
        "part core &&",
        "part golden &&",
        "part stream &&",
    ]
    names = [m.group("name") for line in lines if (m := RUN_NAME.match(line))]
    assert len(names) == len(set(names)), "a gate name appears twice"
    parts = _parts()
    assert sum(len(gates) for gates in parts.values()) == len(names)
    for part, prefixes in PART_OF_PREFIX.items():
        pinned = {n for n in names if n.startswith(prefixes)}
        assert pinned, part
        assert sorted(pinned - parts[part]) == [], part
    assert {f"Required tools present ({part} part)" for part in parts} <= set(names)


# BCK-002 A-64: each path scope of the chain and the gates it holds, by name prefix. The scope keys
# are bin/ci_scope.py's SCOPED_GATES; every gate of a block runs through `ci_scope.py trace`.
SCOPED = {
    "population-synthesis": (
        "Golden vectors: population synthesis v0",
        "population synthesis positive control: floors in place of the age-sex allocation",
        "population synthesis positive control: floors in place of the couple-table rounding",
        "population synthesis positive control: conceptions inside the closed period",
    ),
    "fixture-worlds": (
        "Golden vectors: fixture worlds twin_vales",
        "fixture worlds positive control: dormancy_degrees read as 1",
    ),
}


def _scope_blocks() -> dict[str, list[str]]:
    """Each `scope <key>` block's run lines, from its marker to the next scope or part marker."""
    blocks: dict[str, list[str]] = {}
    key = ""
    for line in _chain_lines():
        if line.startswith(("scope ", "part ")):
            key = line.split()[1] if line.startswith("scope ") and line.split()[1] != "-" else ""
            if key:
                assert key not in blocks, f"scope {key} opens twice"
                blocks[key] = []
        elif key and RUN_NAME.match(line):
            blocks[key].append(line)
    return blocks


def test_each_path_scope_holds_its_gates_and_traces_them() -> None:
    """A-64: the chain's scopes are ci_scope's, each holds its gates, each gate is traced."""
    blocks = _scope_blocks()
    assert sorted(blocks) == sorted(ci_scope.SCOPED_GATES) == sorted(SCOPED)
    for key, prefixes in SCOPED.items():
        names = [m.group("name") for line in blocks[key] if (m := RUN_NAME.match(line))]
        assert len(names) == len(prefixes), key
        assert all(n.startswith(prefixes) for n in names), key
        assert all(any(n.startswith(p) for n in names) for p in prefixes), key
    script = SCRIPT.read_text(encoding="utf-8")
    for key, lines in blocks.items():
        for line in lines:
            call = shlex.split(line.strip().removesuffix("&&"))[3:]
            traced = call[:5] == ["$PY", "-I", "bin/ci_scope.py", "trace", key]
            body = script.split(f"\n{call[0]}() {{\n", 1)[-1].split("\n}\n", 1)[0]
            assert traced or f"bin/ci_scope.py trace {key} --" in body, line[:80]


def test_the_baseline_controls_hold_on_a_stale_baseline() -> None:
    """BCK-002 A-66: on a C3 head before its regeneration commit, each control keeps its reason."""
    assert check_all_selftest.baseline_controls(plant=False) == 0
    assert check_all_selftest.baseline_controls(plant=True) == 1


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
