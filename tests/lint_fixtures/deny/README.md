# cargo-deny positive controls (SPR-001 T-03)

Each directory is a self-contained workspace outside the main one (root `Cargo.toml` `exclude`).
`bin/check_all.sh` runs `cargo deny` on each with the repository's `deny.toml` and expects it to FAIL
with the named diagnostic. If a control ever passes, the gate has stopped working.

| Fixture | Plants | Must fail with |
|---|---|---|
| `gpl_dependency/` | a path dependency licensed `GPL-3.0-only` | `rejected` (licences, ADR-0021) |
| `banned_crate/` | a path dependency named `xxhash-rust`, licensed MIT | `banned` (bans, docs/14 licence trap) |

The crates have no code and no registry dependencies, so the checks run offline against the committed `Cargo.lock`.
