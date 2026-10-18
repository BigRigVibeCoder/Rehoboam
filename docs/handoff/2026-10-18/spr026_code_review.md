# SPR-026 independent code review (head `f610915`)

The review ran on a model different from the developer's, for the Architect's audit VER-SPR-026. Nothing in the repository was changed. The Architect recorded this from the reviewer's returned text. The rulings, including the Tester's DEF-2 from phase 4, are in `spr026_rulings.md` beside this file. The branch has since moved to `762bd84` (main merged by gate name, GAP-8) and `2de1841` (the Tester's phase-4 report).

## Verdict
**PASS AFTER FIXES. BLK 0, MAJ 1, MIN 9, NIT 9.**

The contract and determinism are sound:
- no panic is reachable from a hostile viewer;
- there is no gap or duplicate, and no lock the writer can wait on;
- plane A is unchanged;
- a `--socket` run directory is byte-identical to a headless one;
- the host survived raw-socket fuzzing.

C1 is a real defect no gate sees.

## Findings

| ID | Sev | Where (`f610915`) | Defect and failure scenario | Fix |
|---|---|---|---|---|
| C1 | MAJ | `rh-stream/src/live.rs:419–446` (`(*cur.chunks).clone()` at :441); `writer.rs` | Each `append` clones the whole `Vec<Arc<Chunk>>` inside `rcu`, on the host thread. At MAX the window is the whole unsealed backlog, so the run is quadratic in host pushes. **Measured, no viewer attached, `--ladder monthly`:** solo 3,000 y headless 18.4 s vs `--socket` 26.5 s (+44%); hamlet 6,000 y 45 s vs 82–100 s (+81–120%). The yearly ladder hides it. The Tester re-measured on 4 cores: solo +7.9%, hamlet +12.5% (still above the 5% target) | Merge a completed year's chunks into one; keep the open year append-only (paged list or bounded copy). Re-measure `memory.rs` (bytes() accounting changes). Add a printed, ungated monthly-ladder `--socket` vs headless line |
| C2 | MIN | `serve.rs:788–791`; `rh-golden/src/sift.rs:83–110` | `sift --follow` ends C `finished`, exit 0, when the engine died early | End `Aborted`, non-zero exit |
| C3 | MIN | `serve.rs:828`, `:854` | The product viewer treats 30 s of host silence as fatal | Treat a timeout as idle |
| C4 | MIN | `serve.rs:536–545`, `:669` | `chmod 0700` on any existing `--run-dir` | Only on a directory the host creates |
| C5 | MIN | `rh-protocol/src/control.rs:697`; `serve.rs:205–221` | `stream.sealed` is sent for years wholly before the subscription start (a spec gap; the reference does the same) | Amend `wire_v1.md` §4.3 |
| C6 | MIN | `serve.rs:189–266` | A stalled viewer blocked in a send pins a snapshot and a line buffer | Drop them before sending |
| C7 | MIN | `wire_v1.md` §4.4, §9 | The close rule (close iff the answer's id is null) is not normative | State it; list `invalid_params` |
| C8 | MIN | `selftest_wire_v1.py:311, 316–331` | The reference embeds a non-ASCII string raw in `detail` (raises) and does not check keys for ASCII | Fix the reference; regenerate goldens by tool |
| C9 | MIN | `serve.rs:744`, `:1124`; `sift.rs` | An unbounded silent wait for `run.toml` | Bound it; print the awaited file |
| C10 | MIN | `rh-golden/src/tail.rs:503` | `sealed_lines` builds the whole range in memory | Stream per segment (ruled: SPR-027 T-04) |
| C11–C20 | NIT | various | The seal-queue depth warning (to A-67); a `Drop` guard for `active`; `Catchup::fail` public; `USAGE`; `05.0` order; `run_nonce`/`world_seed` above 2^53 (P2 WebSocket); EOF semantics; the test-hooks control's line number; overflow checks (resolved by PR #89's stream part) with two host-fed `s + 1` sites to fix; `tip.tick` after a resume | See `spr026_rulings.md` |

## Checked and found sound (summary)
- **Dispatcher vs reference:** 40,114 mutated frames, byte-identical apart from C8.
- **Fuzz:** a 619-connection socket fuzz with no panic.
- **Panics:** no reachable `unwrap`/`expect`/`panic`.
- **Determinism:** hamlet 400 y monthly, 2 threads, headless and `--socket` byte-identical.
- **Recovery:** a kill -9 and resume with `tail --live` attached, 2,427 lines identical.
- **Lints, the inventory and gate placement:** all correct.
- **Dependencies:** only `arc-swap 1.7.1` and `rh-protocol` added.

## The Tester's "before" readings for the re-check (phase 4 on `762bd84`, 4 cores)
- **C1:** solo +7.9%, hamlet +12.5%.
- **C2:** the follow-sift exits 0.
- **C3:** exits after 30.1 s.
- **C4:** 0755 becomes 0700.
- **C9:** hangs for more than 100 s.
- **C19:** hangs for 40 s with no panic.

The probes (`p_c1.py`, `p_min.py`, `p_mem3.py`) were in the Tester's session; see `HANDOFF.md` for whether they were pushed.
