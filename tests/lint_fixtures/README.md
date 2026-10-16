# Lint fixtures: positive controls (SPR-001)

Each fixture plants one violation that a gate must catch. `bin/check_all.sh` runs every gate on the
real repository (it must pass) and on each fixture (it must FAIL, printing that gate's own diagnostic,
via `run "<name>" "fail:<text>"`). If a control ever passes, its gate has stopped working. The licence
ledger is not a gate (ADR-0021 as amended 2026-10-16, personal-use mode); its control must succeed and
list the planted licence (`pass:<text>`).

Fixture workspaces carry their own `[workspace]` table and sit under the root `Cargo.toml`
`exclude`, so they never join the Rehoboam workspace. Nothing here is built except the compile-fail
cases (into `target/lint-fixtures/`). Everything runs offline; the git-source and clippy-float controls are rendered into
`target/rendered-fixtures/` and fetches its dependency from a local `file://` repository.

| Fixture | Gate | Plants | Must fail with |
|---|---|---|---|
| `deny/git_source/` (template) | `cargo deny check sources` (crates.io only) | a dependency from a git repository | `error[source-not-allowed]` |
| `deny/wildcard_version/` | `cargo deny check bans` (`wildcards = "deny"`) | `version = "*"` in a publishable crate | `error[wildcard]` |
| `deny/gpl_dependency/` | licence ledger, `cargo deny list` (records, never gates) | a path dependency licensed `GPL-3.0-only` | must succeed and list `GPL-3.0-only` |
| `arch/system_to_system/` | `bin/lint_arch.py` (V-INSP-03) | system crate → system crate | `VIOLATION [V-INSP-03]` |
| `arch/system_to_disallowed/` | `bin/lint_arch.py` (V-INSP-03) | system crate → `rh-snapshot` | `VIOLATION [V-INSP-03]` |
| `arch/system_build_dependency/` | `bin/lint_arch.py` (V-INSP-03, DEF-5) | system crate → `rh-snapshot` as a build-dependency | `VIOLATION [V-INSP-03] … (as a build-dependency)` |
| `arch/system_package_alias/` | `bin/lint_arch.py` (V-INSP-03, DEF-5) | system crate → `rh-snapshot` under the alias `rh-core` (`package = "rh-snapshot"`) | `VIOLATION [V-INSP-03] rh-sys-a: depends on rh-snapshot` |
| `arch/plane_a_to_b/` | `bin/lint_arch.py` (V-PLN-01) | plane-A crate → plane-B crate | `VIOLATION [V-PLN-01]` |
| `arch/plane_a_to_c/` | `bin/lint_arch.py` (V-PLN-01) | plane-A crate → plane-C crate | `VIOLATION [V-PLN-01]` |
| `arch/missing_metadata/` | `bin/lint_arch.py` | a crate with no `[package.metadata.rehoboam]` | `VIOLATION [metadata]` |
| `arch/relabelled_snapshot/` | `bin/lint_arch.py` (pinned labels) | rh-snapshot declared `kind = "contract"` so a system may depend on it | `VIOLATION [metadata] rh-snapshot` (and V-INSP-03 for the edge) |
| `arch/contract_name/` | `bin/lint_arch.py` (contract naming) | `rh-shadow` declares `kind = "contract"` | `VIOLATION [metadata] rh-shadow` |
| `arch/plane_a_rng_crate/` | `bin/lint_arch.py` (rng-crate, DEF-1) | plane-A crate → `rand` (a registry dependency, read without fetching) | `VIOLATION [rng-crate]` |
| `arch/rng_building_block_outside_ids/` | `bin/lint_arch.py` (rng-crate, DEF-1) | `rand_chacha` in rh-core (allowed in rh-ids only) | `VIOLATION [rng-crate] rh-core` |
| `arch/plane_a_unlisted_dep/` | `bin/lint_arch.py` (plane-A-external, BCK-002 A-20) | plane-A crate → `itoa`, which is not on the reviewed list | `VIOLATION [plane-A-external] rh-core: depends on itoa` |
| `arch/plane_a_scoped_dep_outside/` | `bin/lint_arch.py` (plane-A-external, BCK-002 A-20) | `libm` in rh-core (the list allows it only in rh-sample) | `VIOLATION [plane-A-external] rh-core: depends on libm` |
| `arch/plane_a_to_manifest/` | `bin/lint_arch.py` (plane-A closure; SPR-004 ruling A) | plane-A `rh-registry` -> the tool crate `rh-manifest` | `VIOLATION [plane-A-closure] rh-registry` |
| `arch/relabelled_manifest/` | `bin/lint_arch.py` (pinned labels) | `rh-manifest` declares plane A | `VIOLATION [metadata] rh-manifest` |
| `compile_fail/plane_a_names_plane_b/` | `cargo check` (V-PLN-01, compile-fail) | plane-A code naming a plane-B type without a dependency edge | ``unlinked crate `fixture_observation` `` |
| `compile_fail/plane_a_names_plane_c/` | `cargo check` (V-PLN-01, compile-fail) | plane-A code naming a plane-C type without a dependency edge | ``unlinked crate `fixture_narrative` `` |
| `forbidden/planted/<api>.rs` | `bin/lint_forbidden.py --scan` (V-DET-19) | exactly one forbidden API per file (24 files, plus the 6 `float_*.rs` files in the row below): HashMap, HashSet, thread_rng, ThreadRng, rand::rng, StdRng, SmallRng, OsRng, from_entropy, from_os_rng, getrandom, rand::random, random_range, random_bool, random_ratio, random_iter, mul_add, Instant::now, SystemTime, unsafe; paths spaced around `::` (`std :: time :: Instant :: now()` in a macro body, `rand :: rng()`); `#[path]` and `include!` (DEF-4) | ``VIOLATION [V-DET-19/<rule>] `<token>` `` |
| `golden/altered_samplers_v1.txt` | `cargo nextest run -p rh-sample --test golden` with `RH_SAMPLE_GOLDEN` (V-DET-09, SPR-003) | the sampler golden file with one value changed (`gamma 65536 65536 key=0`: 1777 → 1778) | `golden vector differs` |
| `golden/stale_sampler_reference.txt` | `bin/gen_sampler_reference.py --check --output …` (SPR-003) | the exact reference file with its ziggurat table hash changed by one | `STALE:` |
| `golden/stale_rh_ids_v2.txt` | `bin/gen_rng_golden.py --check` (SPR-002 T-02) | an rh-ids golden file that differs from the reference implementation | `STALE:` |
| (rendered) `target/rendered-fixtures/doctests_one_missing.txt` | `doctest_count_gate` in `bin/check_all.sh` (SPR-002 T-01, DEF-2) | the real `cargo test --doc` output with one typed-ID compile-fail line removed | `crates/rh-ids/src/ids.rs: expected 7 compile-fail doctests, found 6` |
| `compile_fail/fixed_point/<crate>/` | `cargo check -p <crate>` (SPR-010 T-01, compile-fail) | plane-A code using `rh-core::fixed` wrongly, one rule per crate: `mix-q8-q16` (a `Q8_8` compared with a `Q16_16`), `mix-money-q16` (a `Q16_16` where `Money` is expected), `float-from` (`Q16_16::from(f64)`), `float-into` (`f64::from(Q16_16)`), `float-cast` (`Money as f64`) | the crate's own error: ``expected `Q8_8`, found `Q16_16` ``, ``expected `Money`, found `Q16_16` ``, ``expected `Q16_16`, found `f64` ``, ``the trait bound `f64: From<Q16_16>` is not satisfied``, ``non-primitive cast: `Money` as `f64` ``. Its sibling `fixed-ok` uses the API correctly and must compile |
| `forbidden/planted/float_*.rs` | `bin/lint_forbidden.py --scan` (V-DET-19 float rule, SPR-010 T-04) | one float per file: `float_type` (`f32`), `float_literal` (`0.5`), `float_suffix` (`2e-3f64`), `float_conversion` (`to_f64`); `float_escape_reason` (an escape whose reason is not `f32_field <entity.column>`); `float_undeclared_field` (`f32_field region.fertility`, which no system spec declares) | ``VIOLATION [V-DET-19/float] `<token>` `` |
| `forbidden/float_workspace/` | `bin/lint_forbidden.py` (V-DET-19 float rule, SPR-010 T-04) | a float famine threshold in a plane-A system crate's `src/`, and a float statistic in its `tests/` (exempt, never reported) | ``VIOLATION [V-DET-19/float] `f32` at …/fixture-sys/src/lib.rs`` |
| `forbidden/float_field/` | `bin/lint_forbidden.py --scan … --specs …/specs` (SPR-010 T-04) | **negative control**: an `f32` escaped as `f32_field region.soil_moisture`, which `specs/X1.fields.odd.toml` declares | must succeed: `0 violations; 1 escapes; 1 declared f32_field columns` |
| `clippy/float_unnamed/` (template) | `cargo clippy -p fixture-core -- -D warnings` with the root `[workspace.lints.*]` tables rendered in (SPR-010 GAP-3) | plane-A float arithmetic (`store * store`) on values a dependency returns as `f64`, with no float token for `bin/lint_forbidden.py` to see | `floating-point arithmetic detected` (`clippy::float_arithmetic`). Known limit: no clippy lint catches a float `<`/`>` (or, on this toolchain, `==`) between such values, so the control plants arithmetic |
| `arch/relabelled_sample/` | `bin/lint_arch.py` (pinned labels, SPR-003 T-00) | `rh-sample`, the float zone, declared (tool, tool), where `bin/lint_forbidden.py` would not scan it | `VIOLATION [metadata] rh-sample: declares (plane, kind) = ('tool', 'tool'), but rh-sample is pinned` |
| `arch/system_to_sample/` | `bin/lint_arch.py` (V-INSP-03 allow-list, SPR-003 T-00) | **negative control**: a system crate depending on `rh-sample` | must succeed |
| `forbidden/float_zone/` | `bin/lint_forbidden.py` (float zone, SPR-003 T-00) | floats in `rh-sample` (exempt) and in `rh-sampler`, a plane-A crate named like it | ``VIOLATION [V-DET-19/float] `f64` at …/rh-sampler/src/lib.rs``; nothing reported for `rh-sample` (checked in `python/tests`) |
| `forbidden/float_zone_core/` | `bin/lint_forbidden.py` (float zone, SPR-003 GAP-3) | a float in a plane-A crate named `rh-core`: the exemption is the set `{rh-sample}` and nothing else (`python/tests` repeats this for every real plane-A crate name and pins the set) | ``VIOLATION [V-DET-19/float] `f64` at …/rh-core/src/lib.rs`` |
| `forbidden/float_zone_ok/` | `bin/lint_forbidden.py` (float zone, SPR-003 T-00) | **negative control**: `rh-sample` alone with a float in `src/` | must succeed: `float rule exempts rh-sample only (SPR-003 float zone); 1 files scanned; 0 violations` |
| `forbidden/float_zone_other_rules/` | `bin/lint_forbidden.py` (float zone, SPR-003 T-00) | `mul_add` in `rh-sample`: the exemption covers the float rule only | ``VIOLATION [V-DET-19/mul-add] `mul_add` at …/rh-sample/src/lib.rs`` |
| `forbidden/float_api/` | `bin/lint_forbidden.py`, rule `float-api` (SPR-003 ruling item 4, GAP-1) | `rh-sample` exposing a float through its public API, once per file: `fn_return.rs` (`pub fn … -> f64`), `field.rs` (a `pub` `f64` field), `reexport.rs` (`pub use libm::log`), `impl_from.rs` (`impl From<Draw> for f64`); `internal.rs` keeps floats behind `pub(crate)` and is not reported | ``VIOLATION [V-DET-19/float-api] `f64` at …/<file>.rs`` (`libm` for `reexport.rs`), four controls |
| `clippy/std_transcendental/` (template) | `cargo clippy -p rh-sample -- -D warnings`, rendered with the root lint tables and `clippy.toml` (SPR-003 T-00) | the float zone's crate-level float allow, then `std`'s `x.ln()` and `x.exp()` | ``use of a disallowed method `f64::ln` `` and ``…`f64::exp` `` (two controls) |
| `uv_drift/` | `uv lock --check --offline` (T-02) | a `pyproject.toml` changed after its `uv.lock` was generated | `needs to be updated` |
| `forbidden/workspace/` | `bin/lint_forbidden.py` (V-DET-19) | HashMap in a plane-A crate, and also in a tool crate (skipped), comments and strings (ignored), plus one reasoned escape (counted) | `VIOLATION [V-DET-19/hash-collections]` |
| `parity/registry_drift.toml` | `bin/manifest_parity.py` (V-WC-04, SPR-004) | the Rust side reads a Registry whose M5 `r0` bound is widened, so `invalid_draft_style` loses an error | `PARITY MISMATCH specs/manifest/examples/invalid_draft_style.yaml` |
| `parity/registry_drift_message.toml` | `bin/manifest_parity.py` line comparison (SPR-004 GAP-3) | the Rust side reads a Registry whose M5 `r0` bound is `[1.0, 9.0]`: same verdict and counts, different message | `PARITY MISMATCH specs/manifest/examples/invalid_draft_style.yaml` |
| `ownership/unowned_write.access.toml` | `rh-cli ownership` (single writer, 05.1 §2; SPR-004 T-03) | `S5.daily_life` declares a write to `person.alive`, owned by `M2.demography` | `VIOLATION [single-writer] S5.daily_life: writes person.alive, which M2.demography owns` |
| `schema/drifted_bound.schema.json` | `rh-cli manifest schema --diff` (SPR-004 T-04) | the reference schema with `Horizon.years` maximum changed to 6000 | `SCHEMA DIFFERENCE $defs.Horizon.properties.years.maximum` |

Each compile-fail case also passes `bin/lint_arch.py` (its graph has no edge), which shows the failure
is the language barrier: plane-A code can reach a plane-B or plane-C type only through a dependency
edge, and `arch/plane_a_to_b/` and `arch/plane_a_to_c/` show the lint rejects that edge. The unit tests in `python/tests/` check each
rule in more detail, including the cases a gate must not flag.
