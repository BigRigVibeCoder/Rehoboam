# Parity corpus (V-WC-04, SPR-004)

Every `*.yaml` here is validated by the Python reference (`specs/manifest/manifest_v0.py`) and the Rust loader (`rh-cli manifest validate`), and `bin/manifest_parity.py` requires the two to agree: same verdict, exit code and counts, and, line for line, the same `[ERROR]`/`[WARNING]` lines (exits 0 and 1) or the same `(loc, type)` pairs (exit 2). A file `NAME.registry.toml` beside `NAME.yaml` is the Registry both sides use for that case; it is a fixture, never a contract.

A fixture may declare a ruled divergence in its first lines, `# parity-divergence: PYTHON-VERDICT / RUST-VERDICT`; it then passes only with exactly those verdicts.

File names say what a case exercises: `valid_*` must be VALID, `registry_*` reach the Registry checks, `schema_*` fail the structural checks, `load_error_*` are not loadable YAML.

## Out of contract

These inputs are outside the manifest language (`docs/05_architecture/05.5` §3) and are deliberately not in the corpus:

- **Explicit `!!binary`, `!!set` and `!!timestamp` tags (SPR-004 DEF-3, deferred).** PyYAML constructs them; the Rust loader refuses them with a LOAD ERROR, which fails closed. The expander never emits them.
- **Unindented continuation lines (GAP-8 / DEF-12, deferred; BCK-002 A-23).** A flow collection or quoted scalar continued on a line at its parent key's column (`a: [1,` then `2]`). PyYAML reads it; the Rust loader refuses it (YAML 1.2 indentation), which fails closed. The expander indents its YAML.
- **LS or PS inside a folded (`>`) block scalar (DEF-11 ruling).** The reference accepts it (PyYAML keeps the character) and the Rust loader refuses it with a LOAD ERROR: an accepted divergence, pinned by `load_error_linebreak_ls_folded.yaml`. Its header line `# parity-divergence: VALID / LOAD ERROR` tells `bin/manifest_parity.py` to require exactly those two verdicts.
- **Alias expansion beyond a few thousand nodes (GAP-6, deferred).** Neither validator limits it; manifest input is trusted today.
