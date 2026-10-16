# Lint fixtures: positive controls (SPR-001)

Each fixture plants one violation that a gate must catch. `bin/check_all.sh` runs every gate on the
real repository (it must pass) and on each fixture (it must FAIL, printing that gate's own diagnostic,
via `run "<name>" "fail:<text>"`). If a control ever passes, its gate has stopped working. The licence
ledger is not a gate (ADR-0021 as amended 2026-10-16, personal-use mode); its control must succeed and
list the planted licence (`pass:<text>`).

Fixture workspaces carry their own `[workspace]` table and sit under the root `Cargo.toml`
`exclude`, so they never join the Rehoboam workspace. Nothing here is built except the compile-fail
cases (into `target/lint-fixtures/`). Everything runs offline; the git-source control is rendered into
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
| `compile_fail/plane_a_names_plane_b/` | `cargo check` (V-PLN-01, compile-fail) | plane-A code naming a plane-B type without a dependency edge | ``unlinked crate `fixture_observation` `` |
| `compile_fail/plane_a_names_plane_c/` | `cargo check` (V-PLN-01, compile-fail) | plane-A code naming a plane-C type without a dependency edge | ``unlinked crate `fixture_narrative` `` |
| `forbidden/planted/<api>.rs` | `bin/lint_forbidden.py --scan` (V-DET-19) | exactly one forbidden API per file (24 files): HashMap, HashSet, thread_rng, ThreadRng, rand::rng, StdRng, SmallRng, OsRng, from_entropy, from_os_rng, getrandom, rand::random, random_range, random_bool, random_ratio, random_iter, mul_add, Instant::now, SystemTime, unsafe; paths spaced around `::` (`std :: time :: Instant :: now()` in a macro body, `rand :: rng()`); `#[path]` and `include!` (DEF-4) | ``VIOLATION [V-DET-19/<rule>] `<token>` `` |
| `golden/stale_rh_ids_v1.txt` | `bin/gen_rng_golden.py --check` (SPR-002 T-02) | an rh-ids golden file that differs from the reference implementation | `STALE:` |
| (rendered) `target/rendered-fixtures/doctests_one_missing.txt` | `doctest_count_gate` in `bin/check_all.sh` (SPR-002 T-01, DEF-2) | the real `cargo test --doc` output with one typed-ID compile-fail line removed | `crates/rh-ids/src/ids.rs: expected 7 compile-fail doctests, found 6` |
| `uv_drift/` | `uv lock --check --offline` (T-02) | a `pyproject.toml` changed after its `uv.lock` was generated | `needs to be updated` |
| `forbidden/workspace/` | `bin/lint_forbidden.py` (V-DET-19) | HashMap in a plane-A crate, and also in a tool crate (skipped), comments and strings (ignored), plus one reasoned escape (counted) | `VIOLATION [V-DET-19/hash-collections]` |

Each compile-fail case also passes `bin/lint_arch.py` (its graph has no edge), which shows the failure
is the language barrier: plane-A code can reach a plane-B or plane-C type only through a dependency
edge, and `arch/plane_a_to_b/` and `arch/plane_a_to_c/` show the lint rejects that edge. The unit tests in `python/tests/` check each
rule in more detail, including the cases a gate must not flag.
