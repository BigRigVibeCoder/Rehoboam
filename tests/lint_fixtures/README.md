# Lint fixtures: positive controls (SPR-001)

Each fixture plants one violation that a gate must catch. `bin/check_all.sh` runs every gate on the
real repository (it must pass) and on each fixture (it must FAIL, printing that gate's own diagnostic,
via `run "<name>" "fail:<text>"`). If a control ever passes, its gate has stopped working.

Fixture workspaces carry their own `[workspace]` table and sit under the root `Cargo.toml`
`exclude`, so they never join the Rehoboam workspace. Nothing here is built except the compile-fail
case (into `target/lint-fixtures/`). Everything runs offline.

| Fixture | Gate | Plants | Must fail with |
|---|---|---|---|
| `deny/gpl_dependency/` | `cargo deny check licenses` (ADR-0021) | a path dependency licensed `GPL-3.0-only`, publishable | `error[rejected]` |
| `deny/banned_crate/` | `cargo deny check bans` (`docs/14` licence traps) | a path dependency named `xxhash-rust`, licensed MIT | `error[banned]` |
| `arch/system_to_system/` | `bin/lint_arch.py` (V-INSP-03) | system crate → system crate | `VIOLATION [V-INSP-03]` |
| `arch/system_to_disallowed/` | `bin/lint_arch.py` (V-INSP-03) | system crate → `rh-snapshot` | `VIOLATION [V-INSP-03]` |
| `arch/plane_a_to_b/` | `bin/lint_arch.py` (V-PLN-01) | plane-A crate → plane-B crate | `VIOLATION [V-PLN-01]` |
| `arch/plane_a_to_c/` | `bin/lint_arch.py` (V-PLN-01) | plane-A crate → plane-C crate | `VIOLATION [V-PLN-01]` |
| `arch/missing_metadata/` | `bin/lint_arch.py` | a crate with no `[package.metadata.rehoboam]` | `VIOLATION [metadata]` |
| `compile_fail/plane_a_names_plane_b/` | `cargo check` (V-PLN-01, compile-fail) | plane-A code naming a plane-B type without a dependency edge | ``unlinked crate `fixture_observation` `` |
| `forbidden/planted/<api>.rs` | `bin/lint_forbidden.py --scan` (V-DET-19) | one forbidden API per file: HashMap, HashSet, thread_rng, StdRng, rand::random, mul_add, Instant::now, SystemTime, unsafe | `VIOLATION [V-DET-19/<rule>]` |
| `forbidden/workspace/` | `bin/lint_forbidden.py` (V-DET-19) | HashMap in a plane-A crate, and also in a tool crate (skipped), comments and strings (ignored), plus one reasoned escape (counted) | `VIOLATION [V-DET-19/hash-collections]` |

The compile-fail case also passes `bin/lint_arch.py` (its graph has no edge), which shows the failure
is the language barrier: plane-A code can reach a plane-B type only through a dependency edge, and
`arch/plane_a_to_b/` shows the lint rejects that edge. The unit tests in `python/tests/` check each
rule in more detail, including the cases a gate must not flag.
