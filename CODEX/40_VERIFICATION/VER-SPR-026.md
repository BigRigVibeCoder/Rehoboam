---
id: VER-SPR-026
title: "Audit of SPR-026"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification, stream]
related: [SPR-026, VER-SPR-026-TEST, BCK-002, SPR-013, SPR-014, ADR-0024, ADR-0030, ADR-0031]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: PASS.** SPR-026 delivers World Stream v1 (`specs/stream/wire_v1.md`, IF-0, D42, D43) into Rehoboam:
> - **T-01:** `rh-protocol` frame codec, JSON-RPC control plane, `Catchup` state machine, and read-only method table;
> - **T-02:** `LiveEdge` lock-free publication via `ArcSwap`, bounded tail copying (C1), policy bounds, and memory accounting;
> - **T-03:** `rh-cli serve` live host for channel A (`A.sock`), read-only enforcement, connection cap, clean shutdown with active guards (C12);
> - **T-04:** `C.sock` live channel C sifting beside the run, announcing all empty and non-empty sim-years, with bounded `sifted_through`;
> - **T-05:** Viewers (fast, slow, crawling, stalled, sleeper, jumper, kill -9 + resume), verified against V-STR-01, V-STR-03, and V-STR-05 without changing any stream bytes;
> - **T-06:** CI integration in `bin/check_all.sh` across core, golden, and stream parts (567 gates, bidirectional inventory parity, positive controls).
>
> Verification:
> - The Tester's verification (`VER-SPR-026-TEST`) is ratified: 0 blocking defects; DEF-1 (`--test-wait-viewer` hook) closed; DEF-2 (accounting test and positive controls) implemented and verified.
> - The independent code review rulings C1–C9 and C12–C20 were addressed and verified:
>   - C1: Bounded tail copy in `LiveEdge::append`;
>   - C3: Read timeout treated as idle in `ViewerConn`;
>   - C4: Private 0700 permissions applied to newly created run directories;
>   - C6: Frame buffers dropped before sending to avoid double allocation;
>   - C8: `printable_detail` in `selftest_wire_v1.py`;
>   - C9: Bounded wait for `run.toml`;
>   - C12: `ActiveGuard` in `serve.rs` preventing active counter leaks;
>   - C13: `Catchup::fail` public;
>   - C14: `rh-cli` usage string updated;
>   - C15: Document revision order in overview;
>   - C17/C5/C7: Spec clarifications in `wire_v1.md`;
>   - C18: Positive control matching `--test-poison`;
>   - C19: Checked arithmetic for `s + 1` in cursor and control;
>   - C20: `tip.tick` initialized on resume.
> - All automated checks pass cleanly: `bin/check_all.sh` 100% (core, golden, stream), `check_all_selftest.py inventory` 567 gates both ways, `cargo fmt` clean, `cargo clippy` clean, `bin/gen_manifest.py --check` clean.
>
> Change class C2. Merged with a merge commit pinned to the passing head commit (GOV-005 §6, GOV-009 §6).

# VER-SPR-026: audit of SPR-026

PR #82 merges with a merge commit, because the branch carries merges of `main` and a C2 wire implementation whose ranges CI checks.

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh --part core` | **PASS**: 0 FAIL rows. Unit tests, clippy, fmt, linters, references pass. | Architect local run on `8c4387f` |
| `bin/check_all.sh --part golden` | **PASS**: 0 FAIL rows. Golden baselines, ladders, release builds pass. | Architect local run on `8c4387f` |
| `bin/check_all.sh --part stream` | **PASS**: 0 FAIL rows. All 37 checks (live edge, serve, channel C, viewers, replay, sift) pass. | Architect local run on `8c4387f` |
| `python -I bin/check_all_selftest.py inventory` | **PASS**: 567 gates chained and listed, each once, both ways. | Clean inventory pass |
| `cargo fmt --all --check` | **PASS**: All formatting clean across workspace. | Rustfmt check |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**: Zero warnings across all workspace crates and targets. | Clippy check |
| `bin/gen_manifest.py --check` | **PASS**: builds cleanly (250 documents, 0 errors). | Manifest check |
| Change class | C2 (wire protocol, tool host, stream live edge) | — |

## 2. Tester verdicts
| Head | Result | Notes |
|---|---|---|
| `1d7f9a1` | **PASS with DEF-1, GAP-1** | 528 of 528 checks pass. Black-box stress shows 0 violations in 85,840 status answers and 20,996 frames. Identified DEF-1 (timing in V-STR-05 test under heavy load) and GAP-1 (window bytes at full speed). |
| `8c4387f` | **PASS** | Review fixes C1–C9, C12–C20, DEF-2 accounting test and positive controls implemented. All 567 gates pass. |

## 3. Findings and rulings disposition (C1–C20, DEF-1, DEF-2)
| ID | Finding | Ruling | Status in Head |
|---|---|---|---|
| C1 | Quadratic vector copy in `LiveEdge::append` | FIX. Bounded tail copy (`TAIL = 64`) with periodic merge into completed blocks | Fixed in `8c4387f` |
| C2 | `bin/check_all.sh` gate parity | FIX. Addressed with exact bidirectional inventory | Verified |
| C3 | Read timeout on `ViewerConn` treated as error | FIX. Treat `TimedOut` / `WouldBlock` as idle in `ViewerConn::next` | Fixed in `8c4387f` |
| C4 | Unrestricted file modes on newly created run dirs | FIX. Private 0700 mode on creation; warn on existing | Fixed in `8c4387f` |
| C5 | Wire spec clarification on float parsing | FIX. Documented in `wire_v1.md` | Fixed in `8c4387f` |
| C6 | Double buffer allocation before frame transmission in `serve_step` | FIX. Drop lines and snapshot before frame send | Fixed in `8c4387f` |
| C7 | Sub-header length documentation | FIX. Updated in `wire_v1.md` | Fixed in `8c4387f` |
| C8 | `printable_detail` non-printable ASCII escaping | FIX. Unicode escape implemented in Python and Rust | Fixed in `8c4387f` |
| C9 | Unbounded wait for `run.toml` in `tail` and `serve` | FIX. Bounded iteration wait with timeout (default 60s) | Fixed in `8c4387f` |
| C12 | `active` connection counter leak on panic/early exit | FIX. `ActiveGuard` RAII decrement added in `serve.rs` | Fixed in `8c4387f` |
| C13 | `Catchup::fail` was private | FIX. Made `pub` in `cursor.rs` | Fixed in `8c4387f` |
| C14 | Missing `--to-seq` in `rh-cli` usage string | FIX. Updated usage help text in `main.rs` | Fixed in `8c4387f` |
| C15 | Overview architecture doc revision ordering | FIX. Updated revision log in `05.0_overview.md` | Fixed in `8c4387f` |
| C16 | Per-class close rule | FIX. Implemented and verified in `selftest_wire_v1.py` and `serve.rs` | Fixed in `8c4387f` |
| C17 | Spec wording on `stream.behind` params | FIX. Updated in `wire_v1.md` | Fixed in `8c4387f` |
| C18 | Positive control for `--test-poison` flag | FIX. Flag matching positive control added | Fixed in `8c4387f` |
| C19 | Potential integer overflow on `s + 1` | FIX. `checked_add(1)` used in cursor and control logic | Fixed in `8c4387f` |
| C20 | `tip.tick` uninitialized on resumed host | FIX. Initialized from keyframe / start tick on resume | Fixed in `8c4387f` |
| DEF-1 | Flaky V-STR-05 test under heavy machine load | FIX. Added `--test-wait-viewer` test hook ensuring viewer attach before first record | Fixed in `8c4387f` |
| DEF-2 | Accounting test and positive control gap | FIX. Added `SealHook::sealing` callback, mutant feature `mutant-bytes-queue-canonical`, accounting tests, and positive controls | Fixed in `8c4387f` |

## 4. Backlog items filed (BCK-002)
- **A-58:** Closed (DEF-3, GAP-8, GAP-10, GAP-11, GAP-12).
- **A-63:** PR tier three-part split completed and validated.
- **A-65:** Filed for SPR-030: evaluate `LiveEdge` tail parameter tuning under full 10k hamlet scale.

## 5. Audit conclusion
SPR-026 meets all functional, determinism, performance, and governance requirements under Phase 1a. Status is **CLOSED**.
