---
id: VER-SPR-004
title: "Audit of SPR-004"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-004, VER-SPR-004-TEST]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** PASS. SPR-004 loads the Registry and world manifests in Rust with line-for-line parity to the reference (`manifest_v0.py`), generates the manifest JSON Schema from the Rust types with zero structural differences, and enforces single-writer column ownership from the system specs. The independent Tester passed all four gating tests on the first round and then drove five fix rounds through more than 7,500 generated, fuzzed and hand-built manifests: 15 defects (DEF-1 to DEF-15) were found and fixed. What remains is either deferred by ruling or a documented divergence where Rust fails closed. The sprint also produced three contract rulings: the tool crate `rh-manifest`, the fixed-point boundary, and the manifest language. Change class C2. The Architect merges (GOV-005 §6).

# VER-SPR-004 — Audit of SPR-004

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh` | PASS (104 checks) | Developer and Tester at `b4e8c12` (Tester evidence `e90bfe7`); Architect at the audit commit |
| CI on GitHub-hosted runners | PASS at `b4e8c12` (Tester evidence `e90bfe7`) (run 38876514123) | PR #6 |
| Change class | C2: new loaders, a new tool crate, a new lint; no golden world | — |

## 2. Tester verdicts
| Round | Head | Result |
|---|---|---|
| 1 | `7a08d15` | The four gating tests PASS (V-WC-04 parity on 21 manifests; Registry round trip 35/35 sections; ownership lint; schema 0 structural differences). DEF-1 to DEF-8, GAP-1 to GAP-7 |
| §8 | `5f3939d` | 9 of 11 items closed; DEF-9 (byte vs character offsets after non-ASCII text), DEF-10 |
| §9 | `58aa4d8` | DEF-9, DEF-10, DEF-8 residual closed; DEF-11 (NEL/LS/PS) from a non-ASCII fuzz run |
| §10 | `3559bf5` | DEF-11 closed; DEF-12 (= GAP-8, ruled out of contract), DEF-13 |
| §11 | `5bcae8d` (rebased) | DEF-13 closed; rebase diff clean; DEF-14, DEF-15 |
| §12 | `b4e8c12` | Clean. DEF-14 and DEF-15 are closed; 0 differences in `cases17` (332), `cases18` (80) and `cases19` (100). Header confinement holds against symlinks and `..` paths. One observation is accepted (§3) |

The Tester ran on a different model from the Developer and wrote its probes from the contracts before reading the implementation.

## 3. Defects and gaps: disposition
| Item | Disposition |
|---|---|
| DEF-1 BOM | Fixed |
| DEF-2 integers beyond 64 bits | Fixed. **Ruling:** every manifest integer is bounded to i64; the reference gained `Int64` (Architect, PR #14) |
| DEF-3 explicit `!!binary`/`!!set`/`!!timestamp` | Out of contract: Rust refuses (fail closed); documented in `05.5` §3 and the corpus README |
| DEF-4 error kinds and locations | Fixed (datetime keys with a time part render differently; kept out of the corpus) |
| DEF-5 two-element bounds | Fixed |
| DEF-6 untyped saga tables | Deferred: typed when their specs freeze (BCK-002 A-24) |
| DEF-7 default Registry path | Fixed |
| DEF-8 ownership lint field validation | Fixed, including the residual (a read's named owner must declare the column) |
| DEF-9 refusals after non-ASCII text | Fixed. Root cause: `saphyr-parser` 0.2.2's `Marker::index` counts characters, not bytes |
| DEF-10 `?` as a flow key indicator | Fixed (block `?x` is a string in PyYAML too) |
| DEF-11 NEL, LS, PS | Fixed. **Ruling:** they are line breaks, as in YAML 1.1; LS/PS inside a folded scalar is refused (pinned divergence) |
| DEF-12 / GAP-8 unindented continuations | Out of contract: Rust refuses (fail closed, YAML 1.2 rule); BCK-002 A-23 |
| DEF-13 multi-line implicit flow key | Fixed (was fail-open for free-key maps) |
| DEF-14 implicit flow key with `:` on the next line | Fixed (`cases18`: 48 differences before, 0 after; `cases19` confirms the forms that must still load) |
| DEF-15 divergence header not confined | Fixed. The header is honoured only under `tests/manifest_parity/` (resolved path; symlinks and `..` fail closed). Equal verdicts are malformed, Rust's diagnostic must be quoted, and only the first 5 lines are searched. **Observation, accepted:** a quoted diagnostic as generic as the verdict word would match any refusal. The one fixture quotes a specific message, and headers are read in review |
| GAP-1 manifest language | **Ruling:** YAML 1.1 as PyYAML reads it; Rust refuses the forms PyYAML refuses |
| GAP-2 "typed section" | **Ruling:** a section is typed when its spec is frozen (A-24) |
| GAP-3 parity oracle compared counts only | Fixed: ordered diagnostic lines and `(loc, type)` pairs; 53 of 53 check sites killed |
| GAP-4 Unicode digits in versions | Fixed in the reference (PR #14) and Rust |
| GAP-5 Registry strictness unspecified | Fixed: `05.5` §3.1 |
| GAP-6 alias expansion unbounded | Deferred: trusted input today (BCK-002 A-25) |
| GAP-7 ownership output overstated | Fixed |

**Architecture ruling during the sprint (A):** SPR-010's float gate made float-based validation illegal in the plane-A crate `rh-registry`. The load-time front end moved to the tool crate `rh-manifest`, which is pinned by name. Authored numbers enter plane A only as fixed point, compiled from the literal's exact decimal value (`05.5` §3.2).

**Escapes found later:** none so far.

## 4. Acceptance criteria
| Task | Criterion | Met |
|---|---|---|
| T-01 | Registry v0 loads into typed structures with bounds; a new system needs no code | Yes (35/35 sections; `X9.new` accepted) |
| T-02 | Manifest loader agrees with `manifest_v0.py` (V-WC-04) | Yes: line for line on 58 (57 line for line, plus 1 ruled divergence pinned by its header) corpus manifests; ruled divergences pinned |
| T-03 | Ownership lint rejects an unowned write, an undeclared read, a plane-B read from plane A, a double claim | Yes (42 of 43 probes; the 43rd a correctly refused non-UTF-8 spec) |
| T-04 | JSON Schema from the Rust types equals the reference schema | Yes (0 structural differences; annotations only) |

## 5. Contract conformance and edits
- ADR-0010 (mechanics only from the Registry), ADR-0024 (plane checks in the ownership lint), `05.1` §2 (single writer).
- Contract edits, authorised by the Architect:
  - `05.0` §6 and the engine-crate lists (`rh-manifest`);
  - `05.5` §3 (the manifest language), §3.1 (Registry well-formedness) and §3.2 (the fixed-point boundary);
  - `docs/09` V-WC-04;
  - `docs/13` §3.
- Reference edits (`manifest_v0.py`, by the Architect in PR #14): `Int64` and ASCII version digits.

## 6. Verdict
| Field | Value |
|---|---|
| Verdict | **PASS** |
| Merge route | The Architect merges (GOV-005 §6) |
| Carried forward | BCK-002 A-23 (flow-continuation indentation), A-24 (typed saga tables), A-25 (alias limit) |
| Escapes found later | — |
