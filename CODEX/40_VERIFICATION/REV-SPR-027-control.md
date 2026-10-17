---
id: REV-SPR-027-control
title: "Independent review of PR #80: specs/stream/control_v1.md, the IF-0 control channel v1 (SPR-027 T-01)"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [SPR-027, SPR-026, PRJ-002, PRJ-003, REV-WAVE5, REV-SPR-026, BCK-002]
created: 2026-10-17
updated: 2026-10-17
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing commit of PR #80 (2026-10-17), after four review rounds and the Architect's check of round 5.** `specs/stream/control_v1.md` is the IF-0 control channel v1.
>
> **What it specifies:**
> - five methods on `control.sock`;
> - a pace governor in the host's run loop, built on one atomic decision point before each instant, a reply slot per request, and the queue fold;
> - re-anchoring on a change of the effective rate;
> - a step that runs at `max`;
> - finishing in one critical section;
> - IF-3's names reserved;
> - the wire's close rule;
> - a stdlib reference with 14 scenarios and a 1,207-line golden.
>
> **The rounds:**
> - **Round 1** (`ef15522`): 0 BLK, 4 MAJ, 13 MIN, 11 NIT (§1). A command could land while an instant runs; no-change commands starved the engine.
> - **Round 2** (`f95abc9`): 0 BLK, 2 MAJ, 6 MIN, 7 NIT (§2). The reference's instant was atomic, and the generation CAS was vacuous.
> - **Round 3** (`f0d6b62`): 0 BLK, 0 MAJ, 3 MIN, 7 NIT (§3).
> - **Round 4** (`e65249d`): 0 BLK, 0 MAJ, 2 MIN, 1 NIT (§4).
> - **Round 5:** checked by the Architect (§0d).
>
> **Result:** 45 of 46 mutants killed; the 46th is argued equivalent. SPR-027 is amended at close.
>
> **Reviewer:** one session on a different model from the drafter's, across all four rounds.

# REV-SPR-027-control: independent review of PR #80

## 0. Architect rulings, round 1 (2026-10-17), on `ef15522`

An independent reviewer on a different model found **0 BLK, 4 MAJ, 13 MIN and 11 NIT**.
- The non-negotiables hold: nothing in v1 is journaled, recorded, keyframed or hashed, a pace changes only when the engine runs, and the P2.7 names are refused by name.
- The nine new gates pass, and so do pytest (679), mypy, ruff and the wire self-test.

The defects are all host-side: a command that lands while an instant runs, commands that change nothing, and coverage. **Every finding is accepted unless a row says otherwise.** The drafter applies these rulings in one push, and a second independent reviewer re-checks the result.

### MAJ
| # | Ruling | Confidence |
|---|---|---|
| C1 | **Accepted: the engine applies commands, and the answer reports the state the engine applied.**<ul><li>**Generation.** The governor cell carries a `generation`. Every command that changes the cell (C2) bumps it.</li><li>**Decision points.** Only the engine makes decisions that depend on the tip, and it makes them at its decision points: before an instant (§3.3 item 1) and after one (item 5).</li><li>**Item 1.** It re-reads the cell and re-checks a pending step: if the next instant's tick is past the target, the engine pauses now and writes `step.reached`, and runs nothing.</li><li>**Item 5.** It writes `paused` for a reached step only by compare-and-swap on the generation it read. If a command has landed since, the command wins and no `step.reached` line is written.</li><li>**Publication.** At each decision point the engine publishes `applied_generation`, the tip, and `next_tick` (the agenda's next instant).</li><li>**Answers wait for the engine.** A mutating command's answer waits until `applied_generation` is at least the command's generation. The engine never waits on a controller.</li><li>**What the answer means.** The answer's `tick` and `seq` are where the command took effect, so a paused tip is exactly the answer's tip.</li><li>**How long the wait is.** At `max` it is at most one instant. Under a governor wait it is immediate, because the wait is interruptible (Q8). Before the first instant it is the genesis time.</li><li>**Tick checks happen at application.** `"instant"`, `past_tick` and "already reached" are resolved by the engine against `next_tick`, and the refusal comes back in the answer.</li><li>**Run end.** If the run finishes before the command is applied, the answer is `run_finished`.</li><li>**`control.status`.** It never waits; it reads the published snapshot.</li><li>**Reference.** The reference splits each instant into a start and an end. A scenario lands a pause, a resume, and a step `"instant"` mid-instant, and the golden pins the three answers and the three tips. The reviewer's three probes (below) become self-test cases.</li></ul> | 75% |
| C2 | **Accepted.**<ul><li>The anchor resets only when the band or the run state changes.</li><li>A command that changes nothing (`changed: false`) touches neither the anchor nor the generation, and its answer returns at once.</li><li>A scenario sends `control.resume` and `control.pace 1y` every 50 ms at `1y` and asserts band `1y`'s release times exactly. A second scenario does the same with a resume every 0.9 s at `1mo`.</li></ul> | 85% |
| C3 | **Accepted.**<ul><li>Add a two-controller scenario: with a step pending, b pauses, b resumes, then b replaces the step target.</li><li>Add dispatch lines `pause_finished`, `resume_finished`, one per reserved name (after C12), and `not_json` (`{`).</li><li>The reserved-detail check hard-codes the §5.1 table instead of iterating `RESERVED`.</li><li>Each of the reviewer's surviving mutants must die on the self-test or the golden: `pause-keeps-step`, `resume-keeps-step`, `step-keeps-first-target`, `pause-after-finish-ok`, `reserved-name-generic`, `resume-anchor-tick-stale`.</li></ul> | 90% |
| C4 | **Accepted.** Add `--control extra-method`: `METHODS` gains `control.seek`, and the check must FAIL with "the control method table is not exactly the five methods of §4". Add its `check_all` gate and its inventory entry. | 90% |

### MIN
| # | Ruling | Confidence |
|---|---|---|
| C5 | **The Architect amends SPR-027** in an Architect commit when this PR closes: five methods; T-01 names `selftest_control_v1.py` and "a planted sixth method"; `tip: {tick, seq}` plus `next_seq`; T-03's V-STR-01 probe gets the step from a pause and the `materialize` check, after T-02; C25's `to_argv` note; C28's governor order. **Do not edit SPR-027 in this PR.** | 90% |
| C6 | **Accepted.** Merge the current `origin/main` once, just before your push. PR #81 may have merged by then; it moves the CRN gate between `check_all` parts and touches the inventory test. Renumber your `05.10` line to the next free version, and resolve the inventory test so that it keeps both sides. | 95% |
| C7 | **Accepted.** While paused, "the tip does not move". A seal already in flight may still land and move `sealed.last_seq`, and the window. The pause probe asserts four things: no record, no keyframe, no journal row, and `next_seq` unchanged. The only seals it allows are for years completed before the pause. | 85% |
| C8 | **Accepted: `--start paused`** (`running` is the default), a command-line option that is never read from the log. Without `--socket` it is refused, because nothing could ever resume the run. Every gate and baseline keeps running at `max`. | 80% |
| C9 | **Accepted.** Before the first instant, `past_tick` refuses only a T below the first instant's tick, and the anchor is that tick. The first instant is at `SimTime::EPOCH` (tick 0), and ticks may be negative (`05.1` §4.1). The reference's first instant moves to genesis. | 85% |
| C10 | **Accepted.** The governor uses a monotonic clock (Rust `Instant`). Realtime appears only in the run log. Deadline arithmetic is in `i128` nanoseconds; say so in §3.3. | 90% |
| C11 | **Accepted.** The `pace` line carries `conn=N`, a per-host counter starting at 1, and `pid=P`, the peer pid from `SO_PEERCRED`, or `-` where the platform gives none. | 80% |
| C12 | **Ruled: one spelling per operation, and IF-3's names win.**<ul><li>§10 records that `control.pace` and `control.pause` are this socket's v1 spellings of IF-3's `timeline.run(band)` and `timeline.pause()` (W30).</li><li>The §5.1 reserved set becomes IF-3's existing names for the P2 operations (`timeline.run_until`, `timeline.fork`, `player.act`, `sift.set_autopause`, …), each refused by name with its detail.</li><li>An invented `control.*` name stays only where IF-3 has no name for the operation, marked provisional.</li><li>Note the `05.9` IF-3 amendment (the two v1 spellings, and the −32120 to −32129 allocation of Q9) as T-06's.</li></ul> | 70% |
| C13 | **Accepted, control side only.** Every −32601 from this socket carries `data.socket: "control"`. There is no `wire_v1` change in this PR: SPR-026 is building the wire now. I record the optional wire detail as a backlog candidate. | 80% |
| C14 | **Accepted.**<ul><li>A pause before the last instant holds the run.</li><li>Once the last instant is done, finishing is never held.</li><li>A pending step is cleared at the end, with no `step.reached`.</li><li>Commands after the end are answered `run_finished`.</li><li>§7 states that a pause outlives its connection, and that any controller can resume it.</li></ul> | 85% |
| C15 | **Accepted.** The grace period applies to `control.sock` only, and it ends early when no controller is connected. `A.sock`'s end-of-run lifetime stays exactly what `wire_v1.md` §1 says; this spec must not change it. | 75% |
| C16 | **Accepted.** List, for each refusal reason, whether the connection closes, matching the wire reference's behaviour: header and frame errors close; `parse_error` and `invalid_request` are answered and keep the connection open. If `wire_v1.md`'s text disagrees with its own reference, tell me; do not edit the wire. | 80% |
| C17 | **Accepted.** `05.1` §4.4 gains `live` (1 sim-minute per wall second, 60:1) and `1mo` (a twelfth of the year), with a note that the 20–60:1 street clock is viewer playback (`05.10` §3). Bump its version. | 90% |

### NIT
C18–C28 are **accepted as proposed**. C25 means specifying three things: the merged CLI's resume (`--run-dir D --resume K`), the `pace` line's method token on a resume, and the tick that `host resume` carries. The `to_argv` note goes into SPR-027 (C5). C28 is also mine (C5).

### The questions the table missed
| Question | Ruling | Confidence |
|---|---|---|
| Does a step run at the band in force or at `max`? | **At `max`.** A step is a seek forward. It runs unthrottled to its target and then pauses. The band in force is kept, and applies again after the next resume, which re-anchors (C2). The alternative waits 24 wall-minutes for one instant at `live` on `solo`'s daily instants. | 75% |
| Which clock? | C10 | 90% |
| Commands that land mid-instant | C1 | 75% |
| Do idempotent commands re-anchor? | C2 | 85% |
| Starting held | C8 | 80% |
| IF-3 spellings | C12 | 70% |
| The run's end under a pause or a step | C14 | 85% |

### The drafter's Q1–Q11
| # | Ruling | Confidence |
|---|---|---|
| Q1 | Agree: include `control.step`. A later request method would be a major version, and W37 defines the operation. It depends on C1, C4 and C5. | 75% |
| Q2 | Agree: extend W30's run-log line, adding C11's identity. | 75% |
| Q3 | Agree: many controllers, last write wins, a cap of 8. §7 says that one controller's step or resume un-pauses another controller's pause. | 75% |
| Q4 | Agree: the log is never read back. The pace restarts at the command line's band, and C8 gives a way to start held. | 75% |
| Q5 | Agree: `1mo` is a twelfth of the year, and `live` is 60:1. `05.1` §4.4 says so (C17). | 85% |
| Q6 | Agree: `control.sock` exists only with `--socket`. `--pace` without `--socket` is allowed: a paced run feeding `tail --wait` is legitimate. `--start paused` without `--socket` is refused (C8). Apply C24. | 70% |
| Q7 | Agree: no notifications in v1, now that answers report the applied state (C1). | 75% |
| Q8 | Agree: the wait is interruptible and paused time is never owed, with C2 and C10. | 85% |
| Q9 | Agree: codes −32120 to −32129. They are recorded in `05.9` IF-3 at T-06 (C12). | 85% |
| Q10 | Agree: a 2 s grace period, with C15. | 70% |
| Q11 | Agree, with the size corrected: `wire_v1.md` is 63 KB. | 90% |

**Round 2.** The drafter applies everything above on `arch/SPR-027-control` and pushes once, after `check_all --part core` and a merge of `origin/main`. A second independent reviewer re-checks. Then the Architect closes the REV, amends SPR-027 (C5), and merges.

**Correction to C16 (2026-10-17, before round 2's push).** SPR-026's implementation showed that `wire_v1.md` §9's text and `selftest_wire_v1.py` disagreed. The text wins:
- **close** the connection after every answer whose `id` is null;
- **keep it open** for `method_not_found` and for an `invalid_params` that carries the id.

SPR-026 fixed the wire reference, and `control_v1.md` follows the same rule.

## 0b. Architect rulings, round 2 (2026-10-17), on `f95abc9`

The second independent review found **0 BLK, 2 MAJ, 6 MIN and 7 NIT**.

What holds:
- Part core passes 424 checks, and the ten control gates each fail with their own marker.
- C2's starvation fix holds: release times are exact to the nanosecond.
- C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C15, C16, C17 and C18–C28 are applied.
- The reserved names equal IF-3's fourteen namespaces.
- The error codes, frame limit, canonical JSON and close rule all match SPR-026's `a652bb6`.
- A Rust host built from one mutex and two condvars has no deadlock and no lost wake-up.

**C1 is not yet verified.** The reference moves the tip at an instant's *start*, so a "mid-instant" command lands after the records. The round-1 design, where the serving thread applies a command at once, passes the self-test and the golden (mutant N2). The spec's item-5 compare-and-swap cannot fail under its own definition of the generation. Two implementers would also diverge on K3, K5 and K6.

**Every finding is accepted unless a row says otherwise.**

| # | Ruling | Confidence |
|---|---|---|
| K1 (MAJ) | **Accepted.**<ul><li>The reference's instant moves the tip at its **end**.</li><li>A command landing in `(release, release + INSTANT_WALL]` is delivered while the instant computes, before its records are appended.</li><li>`mid-instant` gains three commands landing inside a step's reaching instant: a resume, a pause, and a no-change command.</li><li>Regenerate the golden.</li><li>§9 states the fake clock's delivery rule, ties included: a command at exactly a release time is delivered after the instant.</li><li>Under this harness, the reviewer's surviving mutants must die: N2 (answer does not wait), N1 and N1b (the round-1 lost update) or their equivalents after K2, and N4c (no-change commands queued).</li></ul> | 85% |
| K2 (MAJ) | **Ruled: drop the generation and item 5's step check. Each command gets its own reply slot.**<ul><li>A step is reached only at item 1, after the engine has applied the queue. A step runs at `max`, so item 1 follows item 5 with no wait, and the delay is unobservable.</li><li>A command landing in the reaching instant is applied first and wins by construction. A resume clears the step; a pause is logged as `control.pause`, and no `step.reached` is written.</li><li>The engine answers **every** queued command when it applies it, whether the command changed something, changed nothing, or was refused. The serving thread waits on that command's own slot (a one-shot), never on a counter.</li><li>Remove "generation", "applied_generation" and the CAS from the text. §3.3 has one decision point: item 1, which applies the queue, re-checks a pending step against `next_tick`, and publishes the snapshot.</li><li>The reviewer's "item 5 never reaches" mutant (N1c) is then the honest design.</li></ul> | 80% |
| K3 (MIN) | **Accepted.**<ul><li>A mutating command is answered at once only when the queue is empty **and** it changes nothing against the published, applied state. Otherwise it is queued, and the engine computes `changed` at application.</li><li>A connection's requests are served one at a time, in request order, and §4 says so.</li><li>Pin P4, P5, P13 and P16 as cases: a second controller's pause while the first's pause is queued; two identical steps; two pipelined pauses; a step `"instant"` behind a queued one.</li><li>N18 (fold ignored) must die.</li></ul> | 80% |
| K4 (MIN) | **Accepted.**<ul><li>Re-anchor at (tip, now) on any change of the **effective** rate: a step's start, end or clearing, a band change, a run-state change.</li><li>Make §3.3 and §4 say this once, and remove the contradictory sentences (:78 against :174; :98 against :164).</li><li>Add a scenario: at `1y`, a step started while running and cleared by a resume 95 ms later. The next instant comes at the band's period after the clearing, not after an 810 ms stall.</li><li>N15 and N16 must die.</li></ul> | 80% |
| K5 (MIN) | **Accepted.**<ul><li>The engine drains the queue, answers `run_finished`, and sets `finished` in **one critical section**. Every command arriving after that section is refused at submission.</li><li>Add a scenario: a step beyond the horizon, with a pause landing in the last instant.</li><li>N12 (finish writes `step.reached`) and N13 (finish applies the queue) must die.</li></ul> | 85% |
| K6 (MIN) | **Accepted.**<ul><li>`control.status` reads the snapshot the engine published at item 1. Reword :117, the "same number" sentence.</li><li>Status answers go into the scenarios' `answer` lines, so a status that waits (N3) fails the golden.</li><li>§4.1's invariant (:140) holds against the published snapshot.</li></ul> | 85% |
| K7 (MIN) | **Accepted, aligned to SPR-026's actual surface.**<ul><li>§2 says the reason set and the −32601 message are per socket. The control socket adds its −32120… reasons, and its −32601 carries `data.socket: "control"` with its own message.</li><li>The dispatcher's shape is `dispatch_control(&mut ControlConn, &dyn ControlHost, &[u8]) -> Reply { frames, close, used }`, mirroring `rh_protocol::control::dispatch(conn, &dyn Source, data) -> Reply`. `ControlHost` provides the snapshot, submit, and the wait on the reply slot.</li><li>Name what SPR-027 T-03 must extend in `rh_protocol`: the closed `REASONS` table, `classify`'s code set, and `MethodNotFound`'s fixed message. The Architect carries this into SPR-027 (C5).</li></ul> | 80% |
| K8 (MIN) | **Accepted.** One connection per controller (`if cmd.conn not in conns`). Regenerate. The golden then logs `conn=2` for controller b, and one-controller runs report `controllers: 1`. | 95% |
| K9–K15 (NIT) | **Accepted.**<ul><li>K9: four, not five.</li><li>K10: `conn=-` for the engine's lines, and N19 dies.</li><li>K11: correct the two claims. `resume-anchor-tick-stale` is an equivalent mutant: say so, and drop "dies".</li><li>K12: update `05.10`'s v1.7 note.</li><li>K13: `05.1` takes **"the next free version at merge"**, as v1.12 did, because SPR-014, SPR-015 and SPR-016 may also bump it.</li><li>K14: a §11 acceptance row for `--start paused` refused without `--socket`, built as a Rust CLI probe in SPR-027 T-03.</li><li>K15: a deadline beyond `Instant`'s range means no deadline; the engine waits for a command.</li></ul> | 90% |

**Round 3.** Apply everything above on `arch/SPR-027-control`, and push once after `check_all --part core` and a merge of the current `origin/main`. The same reviewer then re-checks with its mutant set (`mutants2.py`, `faithful.py`, `race_mutants.py`). **Every survivor listed above must die, or be argued equivalent in the PR body.**

## 0c. Architect rulings, round 3 (2026-10-17), on `f0d6b62`

The round-3 verification found **0 BLK, 0 MAJ, 3 MIN and 7 NIT**. What holds:
- K1, K2, K4, K5, K7, K8 and K9–K15 are applied.
- Round 2's majors are closed: the tip moves at the end of an instant, and the round-1 design (N2) and the lost update (N1, N1b) are killed.
- Core passes 424 checks. 38 of the mutants are killed.
- `git merge-tree` with SPR-026's PR #82 is conflict-free.

What remains is the atomicity of item 1, the order of answering within it, and the snapshot that a status read at once uses. Two Rust implementers would still diverge on these. **Every finding is accepted unless a row says otherwise.**

| # | Ruling | Confidence |
|---|---|---|
| L1 (MIN) | **Accepted. Item 1 is atomic.** The engine holds the host lock from taking the queue until it publishes. Equivalently, a request counts as queued until its slot is filled, and slots are released only after the publish. `queue-fold` gains two cases:<ul><li>opposite commands from two controllers in one instant (a's pause, b's resume): b is answered `running, changed:true`, and the cell ends `running`;</li><li>a pipelined (pause, status) pair on one connection: the status is answered `paused`.</li></ul>Make P13's order check non-vacuous by asserting answer order against arrival order with ids that are not sorted. N4d and N4e must die. | 85% |
| L2 (MIN) | **Accepted.**<ul><li>A step whose target has already been passed at application is reached **inside its own application**, as §4.5 says. It is answered `paused, step: null`, and `step.reached` is logged.</li><li>Each command's answer reflects the cell right after that command's own application, in queue order.</li><li>A status queued in item 1 is answered with the snapshot published at the end of item 1.</li><li>Pin this with a `dispatch_step_reached_at_once` line and a `queue-fold` case: a status queued in the item 1 where a step is reached. §4.1's invariant (:140) then holds.</li><li>N22 and N23 must die.</li></ul> | 80% |
| L3 (MIN) | **Accepted.** A status answered at once reads the **published** snapshot (`self.published`) on the serving side. Pin the window between an instant's end and the next item 1. N3b must die; N24 is then the honest design. | 85% |
| L4 (NIT) | Accepted. Item 3 reads: "a wait that reaches its deadline runs the instant at once". A command landing exactly at the deadline lands during the instant, as §9 says. | 90% |
| L5–L8 (NIT) | Accepted.<ul><li>L5: "decision point", singular; §9 item 1 names the current functions.</li><li>L6: the `tick` row reads "the tip at the decision point that applied it".</li><li>L7: one definition of `finished`, consistent with K5's critical section and §8.2.</li><li>L8: "never waits on the engine; it waits only behind its own connection's earlier requests".</li></ul> | 90% |
| L9 (NIT) | **Ruled:** the serving thread reads to EOF and submits every complete request frame it has received. Requests are applied whether or not the peer is still there, and answers to a closed peer are dropped. A partial frame at EOF is discarded. §7 and §3.2 say so. | 70% |
| L10 (NIT) | Accepted. Correct the PR body: N16 (no re-anchor at a step's start while running) and N16b (a band change during a step) are **equivalent** mutants, because a step's deadline is `anchor_wall` and its end or clearing re-anchors. They are argued equivalent, not "killed". | 90% |

**Round 4.** One push. The same reviewer re-checks with `mutants3.py` and `fold_probe.py`. N4d, N4e, N22, N23 and N3b must die. N16 and N16b are equivalent mutants.

## 0d. Architect rulings, round 4 (2026-10-17), on `e65249d`, and the close

The round-4 verification found **0 BLK, 0 MAJ, 2 MIN and 1 NIT**:
- core passes 441 checks;
- L1's atomicity, L2 and L4 are unambiguous in the text alone;
- N4d, N4e, N22, N3b and N24 die;
- N16 and N16b are accepted as equivalent.

| # | Ruling | Confidence |
|---|---|---|
| M2 (MIN) | **A queued `control.status` ends its connection's batch.** That connection's later requests stay queued for the next item 1. The status is answered after the publish, with the published snapshot, and answers on a connection are always in request order. Other connections are unaffected. The triple (`pause`, `status`, `pace`) pipelined mid-instant is pinned in `queue-fold` | 75% |
| M1 (MIN) | Pin L2's status half: P19 (a status queued behind a `pace` in a step's reaching instant) joins `queue-fold`. N23 must die | 90% |
| M3 (NIT) | The published snapshot is read without the host lock, by an atomic swap (as `LiveEdge` does through `arc-swap`). A status answered at once never waits out an item 1 | 85% |

**Round 5 (`51df13a`) was verified by the Architect:**
- `selftest_control_v1.py --check` PASS (1,207 golden lines);
- the eight controls each fail;
- the reviewer's 46 mutants (`mutants4.py`): 45 killed, N23 included. N16b survives and is accepted as equivalent;
- the triple is pinned in the golden (e's pause, then status `paused` with band `1y`, then pace `max`, answered after the status by the host's answer counter). The reviewer's `triple_probe.py` stops early, as M2 intends: the later request waits for the next item 1;
- the drafter's part core: 441 PASS.

**SPR-027 is amended by the Architect in the closing commit (C5, K7, K14, C25, C28):**
- T-01 is done;
- five methods;
- T-03 extends `rh-protocol` (per-socket `REASONS`, `classify`'s code set, the `MethodNotFound` message);
- the governor follows §3.3;
- `--start paused`;
- `to_argv` never forwards the control flags;
- `control_v1.md` §11 is binding on T-03's acceptance;
- blocker 4 is done.

## 1. Round 1: the review of `ef15522`

Worktree: `ef15522` on `c92d2ec`. Nothing in the repository was edited, and `git status` is clean. My scripts are in `the Architect's scratchpad: ctl_review/work/`.

#### BLUF
**Verdict: ready after fixes.** Findings: **0 BLK, 4 MAJ, 13 MIN, 11 NIT.**

The non-negotiables hold:
- Nothing in v1 is journaled, written as a record or keyframe, or hashed.
- Every v1 method changes only when the engine runs, never what happens.
- The P2.7 names are refused by name.

The defects are host-side:
- **Mid-instant race.** A command that lands while an instant runs breaks three things the contract promises:
  - a pause answered at tip t actually holds at t+1;
  - a resume can be overwritten by the host's own `step.reached`;
  - a step overshoots.
  The reference cannot show this because its instants are atomic.
- **Starvation.** A command that changes nothing (`changed: false`) still resets the anchor, so a controller repeating one can stop the engine.
- **Coverage.** The reference and the golden file do not pin four transitions §4 defines.
- **T-01 acceptance.** Its planted extra-method control is missing.

#### Findings
| ID | Sev | Where | Problem | Evidence | Fix |
|---|---|---|---|---|---|
| C1 | MAJ | control_v1.md:79, :83, :87, :145, :162–164; ref :481–492 | **A command can land while an instant runs.** The engine reads the cell once (item 1), and item 5 writes `paused` from that read. Three effects: (a) a pause answered at tip t0 holds at t1, the instant then running, which contradicts "holds the tip at t exactly"; at `max` the engine is almost always mid-instant. (b) A `control.resume` that lands during a step's last instant is lost to `step.reached`, so last-write-wins fails against the engine. (c) `"instant"`, `past_tick` and "reached at once" are judged against a stale tip, and a step overshoots. The reference never models this. | `probes.py` on the reference's own classes: (a) pause answered at tick 518400, tip held at 561600. (b) resume answered `running`; the cell then says `paused`, and the log ends `control.resume, step.reached`. (c) `"instant"` resolved to 561600 paused at 604800. | 1. The engine loop decides everything that depends on the tip. The cell carries a generation. Item 1 re-checks a pending step before running (next tick > target means pause now). Item 5 re-reads the cell and writes only by compare-and-swap on the generation it read; a command that landed wins, and no `step.reached` line is written. 2. A command's answer waits for the engine's acknowledgement (at most one instant; the engine never waits on it), so `pace.tick` is where the command took effect. Otherwise, weaken the guarantee and add `pace.applied`. 3. The engine publishes the next instant's tick for `"instant"`. 4. The reference splits an instant into start and end, and a scenario lands a pause, a resume and a step `"instant"` mid-instant. |
| C2 | MAJ | :88, :110; ref :405–460 | **Commands that change nothing reset the anchor, so a controller can starve the engine.** Re-sending the band, or `resume` while running, each push the next deadline out by a full instant. | `probes.py`: at `1y`, `control.resume` or `control.pace 1y` every 50 ms gives **0 instants in 3 s** (35 unperturbed). At `1mo`, a resume every 0.9 s also gives 0. `control.status` does not starve. | Re-anchor only when the band or the state changes. A `control.step` while running at the same band keeps the anchor. Add a scenario that repeats `resume` every 50 ms at `1y` and asserts band-1y's release times. |
| C3 | MAJ | :145, :154, :164, :198, :216; ref :768 | **The reference and the golden do not pin transitions that §4 and §5 define.** These are: a pause clears a pending step; a resume clears it; a second step replaces the target; `run_finished` for pause and resume. No golden line produces `parse_error` (which contradicts §5 and §9 item 6). The reserved-detail check iterates `RESERVED` itself, so it cannot notice a missing entry. The Rust conformance test, which reproduces golden lines, would hold the host to none of this. | My mutants survive both the self-test and the golden: `pause-keeps-step`, `resume-keeps-step`, `step-keeps-first-target`, `pause-after-finish-ok`, `reserved-name-generic` (`control.schedule` answered with no detail). `decode_golden.py`: no dispatch line yields `parse_error`. | Add a two-controller scenario with a step pending in which b pauses, resumes, then replaces the target. Add dispatch lines: `pause_finished`, `resume_finished`, one per reserved name, and `not_json` (`{`). Hard-code the §5.1 table in the check. |
| C4 | MAJ | SPR-027:102; ref `CONTROLS` | **T-01's acceptance requires a planted extra method on the control table that fails by name. None exists.** `_table_checks` compares `METHODS` to a set in the same file and has no positive control. `input-accepted` plants acceptance in the refusal path, not in the table. | `CONTROLS` has 7 entries; none adds to `METHODS`. | Add `--control extra-method` (`METHODS` gains `control.seek`), which must FAIL with "the control method table is not exactly the five methods of §4". Add its `check_all` gate and inventory entry. |
| C5 | MIN | SPR-027:17, :51, :101–102, :122, :126 | **SPR-027 does not match the PR.** It still says four methods. T-01 names `selftest_wire_v1.py` as the reference and "a planted fifth method". The ruled `control.status` shape is `tip: {tick, seq}`; the spec uses `next_seq`, which is the wire's and correct but unlisted. T-03's V-STR-01 probe lacks the step and the `materialize` check that §11 adds, and that check depends on T-02. | The PR diff does not touch SPR-027. | The Architect amends the BLUF, the specs row, T-01 (`selftest_control_v1.py`, "a planted sixth method") and T-03 ("exactly the five"; the step probe issued from a pause; after T-02). |
| C6 | MIN | PR base | **The base predates SPR-013's merge** (`8f7edb4`). `git merge-tree` reports conflicts in `05.10` (main already has v1.6) and in `test_check_all_inventory.py`. | merge-tree output. The control golden still matches when built on `origin/main`'s `specs/stream` (checked in a scratch copy). | Merge `origin/main`, renumber the `05.10` line to v1.7, and resolve the inventory test. |
| C7 | MIN | :68, :146, :316 | **"The stream's window does not move" while paused is not exact under D42(b).** A seal already in flight that lands during a pause moves `sealed.last_seq` and can drop the previous year (`w0` moves). A slow viewer can then get `stream.behind` mid-pause, and T-03's "nothing was written while paused" can fail if a year has just closed. | `wire_v1.md` §6.1(b): a year leaves when the next year completes or its seal lands, whichever is later. | Say that the tip does not move. Make the probe assert: no record, keyframe or journal row; `next_seq` unchanged; only seals of years completed before the pause. |
| C8 | MIN | :266, :268 | **Q4: there is no way to start a host held.** A killed paused run that resumes at `max` can reach the end before any controller reconnects. | `solo` runs 50 years in seconds at `max`. | Add a command-line `--start paused` (or `--pace <band> --paused`), never read from the log. Keep `running` as the default. |
| C9 | MIN | :75, :164; ref :224, :300 | **Ticks before the first instant.** The real engine's first instant is at `SimTime::EPOCH` (tick 0), and ticks may be negative (`05.1` §4.1). The spec refuses `T ≤ 0` before the first instant, and the reference anchors at tick 0 with its first instant at 43,200. On the real engine, `"instant"` or a step to tick 0 before the first instant would be refused as `past_tick`. | `origin/main` rh-core `engine.rs:350` (`cursor: Some(start)`); rh-golden `run.rs:1062` (`SimTime::EPOCH`). | Before the first instant: refuse `past_tick` only when T is below the first instant's tick, and anchor at that tick. Put the reference's first instant at genesis. |
| C10 | MIN | :75, :81, :250 | **The governor's clock is unspecified.** The run log uses Unix-epoch time; if the governor used the same realtime clock, an NTP step would stall or race the run. The deadline arithmetic's integer width is also unstated. | §3.3 calls the anchor "the wall clock in nanoseconds". | The governor uses a monotonic clock (`Instant`); realtime is for the log only; the deadline is computed in i128. |
| C11 | MIN | :232, :244 | **"The run log says who asked what", but the line has no controller identity.** | Golden `log` lines. | Add a connection number, or the peer pid from `SO_PEERCRED`, to the `pace` line (part of Q2). |
| C12 | MIN | :207–214; 05.9:96; wire §8 | **Two spellings for P2's operations.** IF-3 already names `timeline.run(band)`, `timeline.pause()`, `timeline.run_until`, `timeline.fork`, `player.act` and `sift.set_autopause`. wire §8 says P2 brings IF-3's `timeline.*`, `player.*` and `oracle.*` to `control.sock`. §5.1 reserves invented `control.input/schedule/fork/autopause/run_until` and assigns IF-3's namespaces to the WebSocket. | 05.9 IF-3 table; wire_v1.md:304. | Record in §10 that `control.pace` and `control.pause` are this socket's spelling of IF-3's verbs (W30). Reserve one name each for inputs and forks, or say which name wins, and note an IF-3 amendment for P2. |
| C13 | MIN | :193, :201 | **`method_not_found` on this socket can be told apart only by `message`**, which §5 says is not part of the contract. | `stream.subscribe` here returns −32601 with no detail. | Add `data.socket: "control"` on every −32601 from this socket. Optionally, a wire minor adds a detail for `control.*` sent on `A.sock`. |
| C14 | MIN | :67–71, :83 | **The spec does not say what happens at the run's end under a pause or a step.** A step beyond the horizon pauses after the last instant, then finishes. | `probes.py`: state `paused`, then `finish`; the log reads `step.reached`, `finish`. | State that the end is never held by a pause; a pending step is cleared with no `step.reached`. State in §7 that a pause outlives its connection and any controller can resume it. |
| C15 | MIN | :263 | **The grace period also sets `A.sock`'s end-of-run lifetime,** which is `wire_v1.md` §1's business, and adds 2 s to every `--socket` test run. | wire §1 says only "removes it when it exits normally". | Cite it as a wire amendment, or end the grace early when no controller is connected. |
| C16 | MIN | :35, :191–192 | **When does a bad payload close the connection?** §2 says a payload that does not decode is answered and the connection closed, "exactly as there". Both references answer `parse_error` and `invalid_request` without closing (only header and frame errors close). T-03's fuzz asserts closes "where the wire says". | `selftest_wire_v1.py:1020–1050`; control ref `dispatch`. | List per reason whether the connection closes. |
| C17 | MIN | 05.1 §4.4 | **`05.1` §4.4 was not updated**, though SPR-027:179 says it "gains the `live` definition in T-01's spec". | Not in the PR diff. | Add `live` = 1 sim-min/s (60:1) and `1mo` = a twelfth of the year. Note that the 20–60:1 street clock is viewer playback (`05.10` §3). |
| C18 | NIT | :120 | "These two members first": canonical JSON sorts keys, so `calendar` comes before `control`. | Golden `status_result` | Say "always present". |
| C19 | NIT | :3 vs :38 | The header says any change to a result is v2; §2 says a new result field is a minor. | — | Use wire §5's wording ("a field's type or meaning"). |
| C20 | NIT | :234 | No planted control called `stream-pace` exists in `wire_v1.md`. | grep | Cite SPR-026's "a dispatcher with a fourth method (`stream.pace`)". |
| C21 | NIT | :282; ref :756 | The marker `the stream method table carries control.pace` does not appear literally in the source: an f-string builds it. | grep | Make it literal, or drop the claim. |
| C22 | NIT | golden `frame status_result` | It shows `controllers: 0`, which §4.1 makes impossible ("this one included"). | Decoded; the dispatch line shows 1. | Build the frame from a connected host. |
| C23 | NIT | :327; ref :895 | The control golden does depend on a canonical golden line: `dispatch_records_frame` embeds `golden_records("A", 1)`. | Source | Use a literal line, or reword. |
| C24 | NIT | :24 | `control.sock`'s path is 6 bytes longer than `A.sock`'s, so the 107-byte check could pass for one and fail for the other. | — | Check the longer path and serve both or neither. |
| C25 | NIT | :266 | Several details are unspecified: the merged CLI resumes with `--run-dir D --resume K`; the `pace` line's method token on a resume; which tick `host resume` carries. | `origin/main` `cli.rs` | Specify them. For T-03: `cli.rs` `to_argv` must not forward `--socket` or `--pace` to spawned builds. |
| C26 | NIT | :108 | "From the same cell": the tip comes from the stream snapshot, not the governor's cell. | — | Reword. |
| C27 | NIT | ref :130 | `CONTROL_CODE_LO` and `CONTROL_CODE_HI` are inverted. | — | Rename. |
| C28 | NIT | SPR-027:55 | SPR-027's governor rule sleeps after the instant; the spec waits before it. The spec's version is better: a pause lands before the next instant. | — | Align T-03's text. |

#### My verdicts on Q1–Q11
- **Q1, `control.step`:** include it (7/10). Adding a request method later is a major version, and W37 defines it exactly. It depends on C1, C4 and C5.
- **Q2, extend W30's run-log line:** accept (7/10), and add the controller id (C11).
- **Q3, many controllers, last write wins, cap 8:** accept (7/10).
  - Ordering between serving threads is defined; between a serving thread and the engine it is not (C1).
  - §7 should say that one controller's step or resume un-pauses another controller's pause.
  - A disconnect cannot wedge the engine: a pause outlives its controller by design (one-shot tools), and any connection can resume.
- **Q4, the pace restarts at the command line's band after a resume:** accept that the log is never read back (7/10). Reject having no way to start held (C8). Do not default to paused: every gate and baseline runs at `max`.
- **Q5, `1mo` is a twelfth of the year; `live` is 60:1:** accept (8/10). `05.1` §4.4 must say so (C17).
- **Q6, `control.sock` only with `--socket`:** accept (7/10). Say whether `--pace` without `--socket` is refused; apply C24.
- **Q7, no notifications in v1:** accept (7/10), once answers reflect the applied state (C1).
- **Q8, interruptible wait, paused time never owed:** accept (8/10), only with C2 and C10.
- **Q9, codes −32120 to −32129:** accept (8/10). They do not collide with the wire's −32110 to −32116, and they sit in IF-3's −32100 to −32199 band. Record the allocation in `05.9` IF-3 at T-06.
- **Q10, 2 s grace period:** accept (6/10), with C15.
- **Q11, the readings:** accept (8/10). One correction: `wire_v1.md` is 63 KB, not 53 KB.

**Questions the table misses:**
- commands that land mid-instant (C1);
- whether idempotent commands re-anchor (C2);
- whether a step runs at the band in force or at `max` (a one-instant step at `live` on `solo`'s daily instants waits 24 wall-minutes);
- starting held (C8);
- the IF-3 spellings (C12);
- the run's end under a pause or step (C14);
- which clock the governor uses (C10).

#### Checked and sound
- **Non-negotiables.** Pace never reaches the journal, a record, a keyframe or a hash; the controls `pause-writes-record`, `pace-in-journal` and `band-in-hash` each fail with their marker. A pause leaves `next_seq` unchanged. A step issued from a pause stops after Bookkeeping of the last instant at or below T (target 650,000 paused at 648,000). This matches W37 and W35.
- **The P2.7 names** `control.input`, `control.fork`, `player.act` and `timeline.fork` are refused with their details in the golden.
- **The invariance check is circular for the honest host:** the simulated engine knows nothing of the pace, so the paced and headless runs cannot differ. Its value is in catching the planted leaks and pinning the clock lines. T-03 on `solo` is the real proof.
- **Wire reuse** is consistent: framing, canonical JSON, error model, `unsupported_version`, and `invalid_frame` for a RECORDS frame in either direction or a frame over 4 KiB. The two method tables are disjoint, and `control.*` on `A.sock` gets the stream's message.
- **Permissions:** `0600`, the `control.lock` file and the stale-socket rule mirror the wire's R20 layout.
- **The integer rule is deterministic and testable.** My `floor-remainder` and `floor-seconds` mutants are killed.
- **Other mutants (17 written):**
  - 9 killed by the self-test;
  - 2 killed by the golden file only (`band-change-no-anchor`, `status-next-seq-lags`);
  - 6 survive: the 5 in C3, plus `resume-anchor-tick-stale`, which behaves identically in every scripted scenario.
- **Docs:** the `05.10` v1.6 and `wire_v1.md` §8 pointers are accurate, apart from C6 and C12.

#### Gates
| Gate | Result | Time |
|---|---|---|
| The nine new `check_all` gates, run with check_all's exact commands and markers (`nine_gates.sh`) | 9/9 pass | 1.23 s total, 0.10–0.21 s each |
| `selftest_control_v1.py --check` | PASS, golden matches (410 lines) | 0.22 s |
| `selftest_control_v1.py --check` on `origin/main`'s stream references | PASS | — |
| `selftest_wire_v1.py --check`, `gen_manifest --check`, `lint_forbidden`, `lint_arch` | pass | — |
| `pytest` | 679 passed | 13.9 s |
| Inventory test | 13 passed | 0.85 s |
| ruff, ruff format | clean | — |
| mypy | 60 files clean | — |

`bin/check_all.sh --part core` was not completed. 254 gates passed in 170 s, then the nextest gate failed with ENOSPC: this sandbox has 3.2 GB free and the Rust test build filled it. That is not caused by the PR. The nine new gates come after that point in the chain, which is why I ran them separately. I removed my scratch worktree's `target/debug` and `target/release` to recover the space.

Files are in `the Architect's scratchpad: ctl_review/work/`:
- `mutants.py`
- `probes.py`
- `nine_gates.sh`
- `decode_golden.py`
- `check_core.log`

## 2. Round 2: the verification of `f95abc9`

#### BLUF
**Ready after fixes.** 0 BLK, 2 MAJ, 6 MIN, 7 NIT.

The text and the gates are in good shape: `check_all --part core` gives 424 PASS, all ten control gates pass and SPR-027 is untouched. C1 is still not verified, for two reasons:
- **The reference's simulated instant moves the tip at its start.** A "mid-instant" command therefore lands after the records are appended, so the round-1 design (the serving thread applies the command at once) passes every check and the golden.
- **The spec's generation compare-and-swap is vacuous under its own definition of the generation.**

The item-5 race (the round-1 lost resume) is pinned by nothing: the mutants that reproduce it survive.

#### Findings
| ID | Sev | Where | Problem | Evidence | Fix |
|---|---|---|---|---|---|
| K1 | MAJ | selftest:279 (tip set at the instant's start), :939-943 (mid-instant commands issued after `host.run_instant()`), :670-682 (status reads the live engine), :875 | The start/end split of each instant is cosmetic, so C1 is not exercised. The `mid-instant` resume lands while the host is paused, not mid-instant, so round-1 probe (b) is unpinned. | **N2** (the serving thread applies at once, the round-1 design) **survives** both the self-test and the golden.<br>Under a faithful harness (`work2/faithful.py`: commands in (release, release+10 ms] delivered before the append):<br>• honest: PASS;<br>• N2: FAIL with the round-1 symptoms ("pause … answered tick None"; step "instant" paused at 43200, not 86400);<br>• 12 golden lines differ, L636–L746 (e.g. `answer 175000000 a control.pace 1y running - 86400 9 False` becomes `43200 6`). The committed lines are what **N4c** (no-change commands queued) produces.<br>**N1b** (the round-1 lost update: resume answered `running`, cell `paused`, `step.reached` written after it; `race_mutants.py`) **survives**.<br>P12: a status mid the reaching instant shows `step {until_tick 518400}` with tip 518400, which breaks §4.1's invariant (control_v1.md:140). | • Deliver mid-instant commands before the instant's records (the tip moves at the end).<br>• Add to `mid-instant` a resume and a pause landing in a step's reaching instant, plus a no-change command mid-instant.<br>• Regenerate the golden.<br>• State the fake-clock delivery rule in §9, ties included: a command at exactly a release time is delivered after the instant (:934, :940 use a strict `<`; N20 is killed only by the golden). |
| K2 | MAJ | control_v1.md:76 vs :88 vs :90 | The generation text contradicts itself. :76 says only the engine bumps the generation, and only when it applies a change. Nothing can bump it during item 4, so item 5's "CAS on the generation it read" never fails and cannot see a queued command. :90's "applied_generation has passed it" has no per-command generation to compare: a command applied as a no-change, or refused `past_tick`, bumps nothing. The drafter's "queue empty ⇔ generation unchanged" holds only for generations assigned at enqueue. | The reference uses an empty queue (selftest:641) and per-command reply slots (`Pending.reply`).<br>A literal implementer builds **N1**: a pause landing in the reaching instant is answered `changed:false`, and the log shows `step.reached` instead of `control.pause`. N1 **survives**. | • Define a ticket assigned at enqueue under the host lock, and `applied_ticket`, advanced for every applied command (changed, unchanged or refused).<br>• Item 5's condition is "no ticket issued since item 1"; an answer waits for its own reply slot.<br>• Simpler: drop item 5's step check. A step runs at `max`, so item 1 follows with no wait and re-checks after applying the queue; N1c (item 5 never reaches) is observationally equivalent and survives. |
| K3 | MIN | control_v1.md:120; selftest:479-503, :517 | A no-change command is judged against the cell with the queue folded in, but answered with the published, unapplied state. | • P4: B's pause → `{state: running, changed: false}` while A's pause is still queued.<br>• P13: one connection pipelines two pauses; id 2 is answered before id 1, against "in request order" (:104).<br>• P5: B's step to T → `changed:false`, then A's identical step is refused `past_tick`; B's request is silently void.<br>• P16: B's step "instant" is answered with no target.<br>• **N18** (no fold; queue when the queue is non-empty) **survives**. | Answer at once only when the queue is empty, otherwise queue it (the engine already computes `changed` at application, :545/:557/:568/:587). Say that a connection's requests are served one at a time. |
| K4 | MIN | control_v1.md:78 vs :174 vs :98 vs :164; selftest:588, :573 | The anchor text contradicts itself for a step started while running and for a resume that clears it: :78 says a step's start re-anchors, :174 says only on a state change; :98 says the next resume re-anchors, :164 says this one does not. | P9: at `1y`, a step from a running state, cleared by a resume 95 ms later, keeps the old anchor. The next instant comes at 1000 ms, not 278 ms: an **810 ms stall**. A 10-year step would stall about 10 s.<br>N15 and N16 both survive (unpinned either way). | Re-anchor on any change of the effective rate (a step's start, end or clearing, a band, the run state). Add a scenario. |
| K5 | MIN | control_v1.md:277; selftest:647-654 | The §8.2 order (queued commands answered `run_finished`; then the last seal and `stream.end`; then `state = finished`) leaves a window. A command arriving in it sees `running`, is queued, and has no decision point left, so it is never answered. The reference's `finish()` is atomic and cannot show this. C14 is unpinned. | **N12** (finish writes `step.reached`) and **N13** (finish applies the queue) **survive**: no scenario ends with a step pending or a command queued. P11 shows the honest behaviour. | Refuse commands with `run_finished` from the same critical section that drains the queue. Add a scenario: a step beyond the horizon plus a pause landing in the last instant. |
| K6 | MIN | control_v1.md:76 vs :117/:136 (C26) | Two sources for the tip: the snapshot the engine publishes at decision points, or the stream's snapshot. They differ between the writer's append and item 5. With the stream's tip, :140's invariant fails in that window; with the engine's, :117's "same number at that moment" fails. "`control.status` never waits" is untested. | **N3** (status waits for the next decision point) **survives**: status answers are left out of the golden (selftest:1712) and no check times them. | Choose the engine-published snapshot and reword :117. Put status answers in the scenarios' `answer` lines. |
| K7 | MIN | control_v1.md:43; SPR-026 `a652bb6` control.rs:106, :163, :224, :384, :603 | Not buildable as written on SPR-026's module:<br>• `REASONS: [Reason; 11]` is closed;<br>• `MethodNotFound` always carries `READ_ONLY_MESSAGE`, and `error()` takes the message from the reason;<br>• `classify` rejects an error whose code is not one of the 11, so control's −32120…−32122 error frames fail `decode_control` (the reference patches the wire table at runtime, selftest:147);<br>• §2's `dispatch_control(&mut Connection, &[u8]) -> Vec<Vec<u8>>` has no host handle (snapshot, submit and wait) and no close flag; SPR-026's is `dispatch(conn, &dyn Source, data) -> Reply{frames, close, used}`. | Read on origin; nothing checked out. | In §2, make the reason set per socket (three reasons added; the −32601 message chosen per socket), and give `dispatch_control(&mut ControlConn, &dyn ControlHost, &[u8]) -> Reply`. |
| K8 | MIN | selftest:907 | `conns.setdefault(cmd.conn, host.connect())` opens a connection on every command. | • Golden L510–515 log controller b as `conn=4`; §8.1 (:264) gives 2.<br>• One-controller `starve-1y`: status says `controllers: 8`, with 181 connections opened. | `if cmd.conn not in conns: conns[cmd.conn] = host.connect()`, then regenerate. |
| K9 | NIT | control_v1.md:194 | "five `pace` lines for the commands": the sequence has four changing commands. | | Say four. |
| K10 | NIT | selftest:594 | A step reached at once logs `step.reached … conn=1`; §8.1 (:264) says `conn=-` for the engine's lines. | P10; N19 survives. | Pass `None`. |
| K11 | NIT | control_v1.md:291, :309; PR body | Two claims are inaccurate:<br>• "a resume … landing mid-instant": it lands while paused (:875);<br>• `resume-anchor-tick-stale` "dies": it is now an equivalent mutant (every way into `paused` re-anchors at the tip, :565/:599). It survives (R13). | | Correct the text. |
| K12 | NIT | 05.10:9 | The v1.7 note still says "Pending the independent review and the Architect's rulings". | | Update it. |
| K13 | NIT | 05.1 v1.16 | Main is at v1.15, whose own note says SPR-015 T-02 and SPR-016 T-03 add 05.1 text in their PRs. There is no SPR-014 branch on origin. A collision is likely. | | Take "the next free number at merge", as v1.12 did. |
| K14 | NIT | control_v1.md:24 vs §11 (:333-344) | C8's refusal (`--start paused` without `--socket`) has no acceptance probe. The reference has no CLI, so it cannot carry one. | | Add a §11 row. |
| K15 | NIT | control_v1.md:80 | The i128 arithmetic is safe, but `Instant + Duration` can overflow (`from_nanos` takes a u64). | | Say a deadline beyond `Instant`'s range means no deadline: wait for a command. |

#### Per-ruling verdicts
| Ruling | Verdict | Evidence |
|---|---|---|
| C1 | **Partly** | Applied:<br>• the §3.3 model is in the text;<br>• the reference answers after application (`Pending`);<br>• "instant" and `past_tick` are resolved at application;<br>• `run_finished` for a command queued at the end;<br>• golden L568/L570 pin the mid-instant pause and the step "instant".<br>Gaps: K1 (the split is cosmetic; the resume race is unpinned), K2 (the CAS text), K6. |
| C2 | Applied, with gaps | Text :90/:96/:120; `submit` :517.<br>Starve releases equal the unperturbed schedule to the nanosecond (`_exact_releases(…, 0, FIRST_TICK, 0)`).<br>P1: 37 instants in 3 s at `1y` (a 1 ms flood too), 4 at `1mo`.<br>N4 and N4b killed; N4c survives (K1); K3. |
| C3 | Applied | Contest scenario; the `_finished`, reserved-name and `not_json` dispatch lines; `RESERVED_TABLE` (:1025). Round-1 survivors: 5 killed; `resume-anchor-tick-stale` is equivalent (K11). |
| C4 | Applied | The gate fails with "…exactly the five methods of §4"; inventory entry present. |
| C5 | Applied | The `git diff` of SPR-027_Materializer.md is empty. |
| C6 | Applied | Merges 772478f; `05.10` is at v1.7; the inventory keeps both sides; pytest of the inventory: 13 passed. |
| C7 | Applied | :69, :156, :337; `_held`. |
| C8 | Applied | :24, :72, :280; `start-paused` scenario; K14. |
| C9 | Applied | :78, :175; `FIRST_TICK=0`; engine.rs:434 starts at `SimTime::EPOCH`.<br>P8: "instant" before the first instant → 0, held at 0; −1 is refused.<br>N7 killed. |
| C10 | Applied | :80; governor check at far ticks; K15. |
| C11 | Applied | :258, :264; K8 and K10. |
| C12 | Applied | §5.1 and :320; T-06 note at :230. |
| C13 | Applied | `error()` :366; check :1064. |
| C14 | **Partly** | Text applied; unpinned (N12, N13); K5. |
| C15 | Applied | :30, :277. |
| C16 | Applied | :36, :198; `CLOSES`; N8a/b/c killed; equals SPR-026's control.rs:630 (`close = id null`). |
| C17 | Applied | 05.1 §4.4 matches 05.10 §3 L57; K13. |
| C18–C27 | Applied | Key order in the :109 example; "always present"; the major-change list; SPR-026's `stream.pace` control; literal markers; `"controllers":1` in `status_result`; the literal RECORDS line; the path check on `control.sock`; the resume line (matches `rh-cli run … --resume K --run-dir D`); C27 HI/LO.<br>C26 conflicts with :76 (K6). |
| C28 / step at `max` | Applied | :98, :173; `deadline()` :623; N5 killed. The anchor contradiction is K4. |
| Clock (missing question) | Applied | Per C10. |
| Q6 | Applied | :24. |

#### Race walk-through
| Interleaving | Specified | Reference |
|---|---|---|
| A command lands in the instant (between item 1 and item 5) | Applied at item 5; the command wins | Queued, applied by `after_instant` :645. The tip has already moved (K1). |
| A resume, or a pause, in a step's reaching instant | The command wins; no `step.reached` | Honest: resume → running; pause → `control.pause` logged (P2b, P7). **Unpinned:** N1 and N1b survive. |
| A pause just after item 5 | No change; answered at once with `changed:false` | As specified (P7). |
| A step replaced in the old target's instant | The new target wins | Paused only at the new target (P6); unpinned. |
| Two controllers, different commands | Applied in lock order | `two-controllers` at 2.1 s. |
| Two controllers, the same command | Ill-defined (:120) | K3 (P4, P5, P16). |
| A command during a governor wait | Ends the wait (item 3) | :934; pinned (the pause at 1.05 s). |
| A no-change command during a wait | Not queued; the deadline stands | Releases exact. |
| A command during the last instant | `run_finished` | P11. |
| After finish | `run_finished` | `dispatch_*_finished`. |
| Between the final drain and `finished` | Stranded (§8.2 order) | Not modelled (K5). |
| A connection drops with its command queued | Still applied (C14) | `close()` leaves the queue alone. |
| `control.status` mid-instant | Published snapshot, no wait | Live read: invariant broken (P12); N3 survives (K6). |
| "instant" before the first instant | Resolves to tick 0 | P8. |

**Rust host.** A mutex with two condvars works with no deadlock and no lost wake-up:
- serving threads enqueue and wait on their reply;
- the engine tests the queue under the lock before `wait` or `wait_timeout` (std's `Condvar` is monotonic on Linux) and never waits on a serving thread;
- an answer is bounded by one instant at `max`, or by genesis before the first instant;
- EPIPE from a closed controller is harmless, since Rust ignores SIGPIPE.

The spec does not yet pin K2 (what the generation counts), K3, K5 and K6; two implementers would diverge on each.

#### Mutants
`work2/mutants2.py`: round 1's 17 re-expressed on `_pace`/`_pause`/`_resume`/`_step`/`submit`/`_step_reached` (R4 inverted, since round 2 logs no no-change), plus 21 new ones.

**Round-1 set:**
- Killed: R1, R2, R3, R4, R5, R6, R7 (golden), R8, R9, R10, R11, R12, R14, R15, R16, R17.
- Survives: R13 `resume-anchor-tick-stale`, which is equivalent.

**New:**
| Mutant | Result |
|---|---|
| N1 item 5 with no CAS | **Survives** |
| N1b stale target at item 5 | **Survives** |
| N1c item 5 never reaches | Survives (equivalent) |
| N2 answer does not wait | **Survives** (killed by the faithful harness) |
| N2b predicted answer | Killed |
| N3 `control.status` waits | **Survives** |
| N4 no-change bumps the generation | Killed |
| N4b no-change re-anchors | Killed |
| N4c no-change queued | **Survives** |
| N5 step at the band | Killed |
| N7 first anchor not at tick 0 | Killed |
| N8a/b/c C16 inverted per class | Killed |
| N10 "instant" resolved at submit | Killed |
| N12 finish writes `step.reached` | **Survives** |
| N13 finish applies the queue | **Survives** |
| N15, N16 anchor variants | **Survive** |
| N18 fold ignored | **Survives** |
| N19 reached-at-once logs `conn=-` | **Survives** |
| N20 tie rule | Killed (golden) |

`--start paused` without `--socket` cannot be expressed in the reference (K14).

#### Checked and sound
- **Gates:** the ten control gates each fail with their own marker (`extra-method` also prints "control.seek: ok"). The golden has 913 lines: 18 frames, 40 `dispatch`, 11 scenarios. The `answer` lines pin answers and tips for the mid-instant pause and step.
- **Probe results the drafter reported**, reproduced at their ticks (P2): (a) a pause answered at 518400 holds at 518400; (b) running, no `step.reached`; (c) "instant" → 561600, paused at 561600.
- **1y→10y→1y:** each is a change: two log lines, generation +2, re-anchor at (tip, now) (P14). Specified per command (:78, :120); not in the golden.
- **Reserved names (E):** IF-3's 14 namespaces equal `RESERVED_NAMESPACES`.
  - Explicit names cover `timeline.run`, `pause`, `run_until`, `seek` and `fork` (05.9 L97), `sift.set_autopause` (L100) and `player.act`.
  - `list` and `diff` fall to the namespace detail. Nothing is missing.
  - `control.schedule` is a provisional name that C12 allows; it is acceptable, though `timeline.*` would avoid a second spelling later.
- **F, against SPR-026:** no code collision (SPR-026 uses −32110…−32116). The 4 KiB limit equals `VIEWER_MAX_PAYLOAD` 4096 (wire.rs:28). Canonical JSON is shared. The C16 rule is identical. K7 lists the gaps.

#### Gates (worktree at `f95abc9`)
- **`bin/check_all.sh --part core`:** Verdict PASS, 424 checks, 0 FAIL, 736 s (12:16). It ran with `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only` to stay inside the disk allowance; this changes no behaviour.
- **The ten control gates** (`work2/ten_gates.sh`): 10/10, 0.16–0.29 s each.
- **Inventory test:** 13 passed.

**Round-1 tools, as adapted:**
- `probes.py` runs unchanged, but its probe 2 calls the engine directly with its own item 5, so it reproduces round 1 by construction. I rewrote it as `probes2.py` on the round-2 API: P1–P3 are round 1's, P4–P17 are new.
- `mutants.py` crashes (`Host.step` no longer exists). It is replaced by `mutants2.py`.
- `nine_gates.sh` became `ten_gates.sh`, adding `extra-method`.
- `decode_golden.py` is unchanged.

The worktree's `target/` is deleted; the worktree is clean.

Files are in the Architect's scratchpad: ctl_review/work2/:
- probes2.py
- mutants2.py
- faithful.py
- faithful_diff.py
- race_mutants.py
- ten_gates.sh
- check_core.log

## 3. Round 3: the verification of `f0d6b62`

#### BLUF
**Verdict: ready after small fixes. 0 BLK, 0 MAJ, 3 MIN, 7 NIT.**

All K1–K15 rulings are applied. Round 2's two MAJ are closed:
- **K1 (mid-instant):** the reference now moves the tip at the end of an instant, so commands that land mid-instant really are mid-instant. The round-1 design (the serving thread applies a command at once, N2) and the lost update (N1, N1b) now die.
- **K2:** the generation and the CAS are gone; there is one decision point and one reply slot per request.

What remains:
- **Two K3 conditions are unpinned.** "The queue is empty" and "nothing of the connection in flight" can each be removed without any test failing, and the text does not say item 1 is atomic.
- **Answers are computed before item 1's step re-check.** The text contradicts itself on this.
- **The reference reads live state** for a status answered at once while another connection's request is queued.

#### Findings
| ID | Sev | Where | Problem | Evidence | Fix |
|---|---|---|---|---|---|
| L1 | MIN | control_v1.md:84, :90, :120; selftest:529-555, :1734 | K3's two immediate-answer conditions are unpinned.<br>The text never says item 1 is atomic. A Rust host could take the queue under the lock, then apply outside it, or wake slots before publishing. A serving thread would then see an empty queue and a stale snapshot, and the round-2 lost answer comes back.<br>P13's order check is vacuous: the request ids are sorted by construction (:1734). | `fold_probe.py`:<br>• **N4d** (answer at once even with the queue non-empty) **survives**. With a's pause and b's resume landing in one instant, b is told `running, changed:false` and the cell ends `paused`; the honest host answers `running, changed:true`.<br>• **N4e** (in-flight ignored) **survives**. A pipelined (pause, status) pair is answered `running`; honest is `paused`.<br>`queue-fold` has no opposite pair and no pipelined status. | State that item 1 holds the host lock from taking the queue to publishing. Equivalently: a request counts as queued until its slot is filled, and slots are released after the publish.<br>Add both cases to `queue-fold`. |
| L2 | MIN | §4.5 :174, :178 vs §3.3 :84; selftest:610-622, :706 | Answers are computed before item 1's step re-check.<br>• §4.5 says a step reached at once is answered `paused, step: null`. The reference, and item 1 as written ("answer as it applies it", then re-check), answer `running, step {until_tick: T}`.<br>• A status or command queued in a reaching item 1 is answered with `step T` at `tip T`. That breaks §4.1's invariant (:140), because :124 has a queued status read "the snapshot being built". | P10, P19.<br>**N22** (answer `paused` when reached at once) and **N23** (a queued status reads the end-of-item-1 snapshot) **survive**. | Reach at once inside the step's application (as §4.5 says), and answer a queued status with the snapshot published at the end of item 1. Pin both: a `dispatch_step_reached_at_once` line and a `queue-fold` case. |
| L3 | MIN | selftest:706 | A status answered at once reads the live cell and tip whenever any other request is queued. §3.3 :76 and §4.1 :124 say it reads the published snapshot. | P18: published tip 475200; the status answered 518400.<br>**N3b** (every status live) and **N24** (always published) **survive**: no scenario has a status between an instant's end and the next decision point. | Use `self.published` on the serving side, and pin the window. |
| L4 | NIT | §3.3 :86 vs §9 :289 | Ties: item 3 "waits until the deadline … then goes to item 1". Read literally, that creates a decision point at the deadline, which would apply a command landing exactly at the deadline before the instant. §9 and the reference say it lands during the instant.<br>N20 is killed only by golden line 390. | | Item 3: "a wait that reaches its deadline runs the instant at once". |
| L5 | NIT | :13, :29, :288 | Stale after K2: "decision points" (plural); §9 item 1 still names `before_instant, after_instant` (`after_instant` now only finishes). | | |
| L6 | NIT | :117 | The `tick` row still says "both read from the stream's snapshot (C26)". K6 and :76 say the engine-published snapshot; the two differ between an instant's append and the next item 1. | | Reword: "the tip at the decision point that applied it". |
| L7 | NIT | §3.2 :70 vs :277 | §3.2 says `finished` means the last year is sealed and `stream.end` was sent. K5's critical section marks the cell `finished` before both, and §8.2 then repeats "state = finished in the cell". | | |
| L8 | NIT | :124 | "**never waits**" sits beside "queued behind a command on its own connection", where it does wait. | | |
| L9 | NIT | §7, §3.2 | Not stated: whether requests already received before a peer's EOF, but not yet read, are applied. A blocking server that stops on EPIPE drops them; a read-ahead server applies them. | | State it. |
| L10 | NIT | PR body ("N16 dies … none argued equivalent") | The anchor at a step's start, and a band change during a step, are never observable: a step's deadline is `anchor_wall`, and the step's end or clearing re-anchors. **N16** (no re-anchor at a step's start while running) and **N16b** (a band change during a step re-anchors) survive as **equivalent** mutants. Harmless, but not pinned. | | |

#### Per-ruling verdicts
| Ruling | Verdict | Evidence |
|---|---|---|
| K1 | Applied | Tip moves at the end (selftest:654-664); `simulate` delivers before the instant (~:1027).<br>`mid-instant` lands a resume, a pause and a no-change pace in reaching instants, plus a status and a tie (golden L568-578).<br>P17 now gives `43200 6`, which is round 2's faithful-harness value. N2, N1, N1b and N4c are killed. |
| K2 | Applied | No `generation` in the reference. One decision point (:639-644); `after_instant` only finishes. One slot per request; N2b and N2c are killed. L2 remains. |
| K3 | **Partly** | P4, P5, P13 and P16 are pinned and match the drafter's reading. P5' (the target is the computing instant's tick): both requests are refused `past_tick`. L1 remains. |
| K4 | Applied | `Cell.effective`; `step-clear` (golden L1002: `383333334`). P9's largest gap is 93.3 ms (it was 810 ms). N15 and R13 are killed. |
| K5 | Applied | `finish()` is atomic (:671-684); `finish-race`; N12 and N13 are killed; P11. L7 remains. |
| K6 | **Partly** | Status reads the published snapshot and status answers are in the golden (N3 killed); P12's invariant holds. L3 and L6 remain. |
| K7 | Applied | §2 :43 matches SPR-026 head `03b29ef` (rh-protocol unchanged since `a652bb6`): `REASONS` :106, `classify` :384, `MethodNotFound` message :163, `dispatch` :603, `Reply` :577.<br>A `git merge-tree` of PR #80 with PR #82 is conflict-free. On that merge, the inventory test's gate-list tests pass; 3 other tests fail only because my partial extract lacked their files. |
| K8 | Applied | `_issue` :986; the golden has `conn=1/2/3` and `controllers:1`. |
| K9, K10, K12–K15 | Applied | :194; N19 killed; 05.10 note; 05.1 v1.16 "next free number at merge"; §11 :342; :80. |
| K11 | Applied | Except L10. |

#### Races, read from the text alone
| Interleaving | Text | Reference |
|---|---|---|
| A resume or a pause lands in a step's reaching instant | Applied first; it wins; no `step.reached` (:84) | Matches; pinned (`race_mutants.py`: N1 and N1b killed) |
| A no-change command in a reaching instant, queue empty | Answered at once from the published snapshot | Pinned (golden L577) |
| Opposite commands from two controllers in one instant | Queued, applied in order | Matches; **unpinned** (N4d); atomicity unstated (L1) |
| A pipelined (command, status) pair | The status is queued behind the command | Matches; **unpinned** (N4e) |
| A step reached at once | **The text contradicts itself** (§4.5 vs item 1) | `running` (L2) |
| A status queued in the item 1 where a step is reached | "the snapshot being built" | Breaks :140 (L2) |
| A status answered at once while another's request is queued | Published snapshot | Live snapshot (L3) |
| A tie at a band deadline | Item 3 vs §9 (L4) | §9; golden pins it |
| A tie at a decision point at `max` | Applied before the instant | Pinned (N21 killed) |
| A command during the last instant | `run_finished` | Pinned (`finish-race`) |
| A request after finish | Refused at submission | `dispatch_*_finished` |
| A connection drops while its command is queued | Applied; the slot is filled and the answer dropped | P21 matches; buffered requests unspecified (L9) |
| A command during a governor wait | Cuts the wait | Pinned |

A mutex plus a per-request slot and a condvar has no deadlock and no lost wake-up, provided the engine tests the queue under the lock before waiting. What two Rust implementers would still disagree on is L1's atomicity, L2 and L4.

#### Mutants
`work3/mutants3.py`:
- the 17 round-1 mutants re-expressed;
- round-2's N-series rebuilt for one decision point: N1 checks the step before the queue; N1b pauses on the stale target; N1c keeps the end-of-instant check; N2b answers with the final state; N2c swaps slots;
- new ones aimed at round 3 (N3b, N4d, N4e, N16b, N21–N24).

| Result | Mutants |
|---|---|
| Killed by the self-test | R1–R6, R8–R16; N1, N1b, N1c, N2, N2c, N3, N4 (a no-change re-anchors), N4c, N5, N7, N8a/b/c, N10, N12, N13, N15, N19, N21 |
| Killed by the golden only | R7, R17, N2b, N20 |
| **Survive** | **N4d, N4e** (L1); **N22, N23** (L2); **N3b, N24** (L3); N16 and N16b (equivalent, L10) |

Round 1's `resume-anchor-tick-stale` (R13) now dies on `step-clear`.

The other round-2 tools:
- `faithful.py` is obsolete: the reference now behaves as that harness did.
- `race_mutants.py` and `probes2.py` are adapted as `race_mutants.py` and `probes3.py` (P1–P22; P1 floods: 37 instants in 3 s, releases exact to the nanosecond).

#### Gates (on `f0d6b62`)
- **`bin/check_all.sh --part core`:** PASS, 424 checks, 0 FAIL, 739 s. Run with `CARGO_INCREMENTAL=0` and `CARGO_PROFILE_DEV_DEBUG=line-tables-only` to save disk.
- **The ten control gates:** 10/10, 0.16–0.23 s each, each failing with its own marker.
- **Golden:** 1,171 lines, 14 scenarios, 40 dispatch lines, 18 frames.
- **Cleanup:** the worktree's `target/` is deleted and the worktree is clean.

Files are in the Architect's scratchpad: ctl_review/work3/:
- probes3.py
- mutants3.py
- race_mutants.py
- fold_probe.py
- ten_gates.sh
- check_core.log

## 4. Round 4: the verification of `e65249d`

#### BLUF
**Verdict: mergeable after two small fixes. 0 BLK, 0 MAJ, 2 MIN, 1 NIT.**

L1–L10 are applied:
- **Part core** on `e65249d`: PASS, 441 checks, 0 FAIL.
- **Text-only re-read:** L1's atomicity, L2 and L4 are now unambiguous.
- **Mutants:** N4d, N4e, N22, N3b and N24 die. Everything killed in round 3 is still killed.
- **N16 and N16b:** I accept the equivalence argument. While a step is pending the effective rate is `max`, so the deadline is just `anchor_wall` and no anchor value is read. Every way out of a pending step (reaching it, clearing it, a pause) re-anchors first. A replacement or a band change keeps the step pending, so nothing reads the overwritten value.

Two gaps remain:
- **M1:** L2's status half is implemented but not pinned. My N23 survives.
- **M2 (new):** a status pipelined between two commands of its own connection breaks "in request order".

#### Findings
| ID | Sev | Where | Problem | Evidence | Fix |
|---|---|---|---|---|---|
| M1 | MIN | `queue-fold`; selftest `before_instant` | The rule that a queued status gets the snapshot published at the end of item 1 is implemented but unpinned. The pinned case (a step to tip + 2,000 with a status behind it) cannot tell the end-of-item-1 snapshot from the one being built: both say `paused`. | **N23** (round 3's behaviour: the status gets the snapshot being built, in queue position) **survives**.<br>Probe P19 on the honest host (a status queued behind a `pace` in a step's reaching instant) answers `paused`, and the invariant at :140 holds. N23 answers `running, step T, tip T` there, which breaks :140. | Add P19 to `queue-fold`. |
| M2 | MIN | §3.3 item 1 ("only then answers the `control.status` requests that were queued") vs §3.3 "Answers" and §4 "in request order per connection"; selftest `_apply_pending`/`before_instant` | One connection pipelines `pause`, `status`, `pace 1y` while an instant computes. The status is answered **after** the later `pace`, and it reports `band: 1y`. | `triple_probe.py`: `answered_seq` comes out as pause 1, pace 2, status 3; the status shows `band=1y`.<br>The reference's own L1 arrival-order check (the answer counter) would fail on this triple; no scenario has it.<br>Two implementers will differ: wire order (request order, with a status reporting a later request) or the counter's order. | Make a queued status end its connection's batch: that connection's later requests wait for the next item 1. Or state that the status reports every command applied in its item 1 and that the wire order stays request order. Then pin the triple. |
| M3 | NIT | §3.3 item 1 (host lock held until the publish) vs §4.1 ("never waits on the engine") | If the published snapshot is read under the host lock, a status can wait out one item 1, including its run-log writes. | — | Say the snapshot is readable without the host lock (an atomic swap), or say that a status may wait one item 1. |

**Text re-read:**
- **L1:** unambiguous ("holds the host lock from taking the queue until it publishes; … slots are released only after the publish").
- **L2:** unambiguous (each command's answer is the cell right after its own application; a step already passed is reached inside its own application; a queued status is answered after the publish). A command answered in a reaching item 1 can still show its step pending at its own tick, as the text says; the published snapshot is what holds the invariant. Only M2 remains.
- **L4:** unambiguous ("a wait that reaches its deadline runs the instant at once … no decision point at the deadline").
- **L9:** clear.

#### Mutants
I adapted `mutants4.py` to round 4's API:
- `_apply_pending` now returns the deferred status requests, and `_fill` orders the answers.
- R7, R13, N4, N15 and N16 are re-implemented against the new `_apply_pending`.
- N1 and N1b now answer deferred status requests after the publish.
- N22, N23 and N24 are inverted, because round 4 made their round-3 forms the honest behaviour. N22 is now "answered `running`", N23 is "status gets the snapshot being built", and N24 is "answers numbered in queue position".

| Result | Mutants |
|---|---|
| Killed by the self-test | R1–R6, R8–R16; N1, N1b, N1c, N2, N2c, N3, **N3b**, N4, N4c, **N4d**, **N4e**, N5, N7, N8a/b/c, N10, N12, N13, N15, N19, N21, **N22**, **N24** |
| Killed by the golden only | R7, R17, N2b, N20 |
| Survives | **N23** (M1); N16 and N16b (equivalent, accepted) |

- **`fold_probe.py`** (honest host): b's resume opposite a's pause is answered `running, changed:true`, and the pipelined (pause, status) pair answers `paused`. Under N4d, b is told `changed:false`. Under N4e, the status says `running`. Both mutants are now caught by `queue-fold`.
- **`race_mutants.py`:** the honest host lets a resume or a pause in the reaching instant win. N1 and N1b reproduce the old faults and are killed.
- **`probes3.py`:**
  - P10 is answered `paused, step:null`;
  - P18's status shows the published tip, 475200;
  - P19's queued status is `paused`.

#### Gates (on `e65249d`)
- **`bin/check_all.sh --part core`:** PASS, 441 checks, 0 FAIL, 706 s. Same environment tweaks as before (`CARGO_INCREMENTAL=0`, dev debuginfo `line-tables-only`).
- **Ten control gates:** 10/10, each about 0.2 s, each failing with its own marker.
- **Golden:** 1,190 lines, 41 `dispatch` lines.
- **Merge of `39b8d76`:** it touches no control file, and `05.1` v1.16 is still the only v1.16.
- **Cleanup:** the worktree's `target/` is deleted and the worktree is clean.

Files are in the Architect's scratchpad: ctl_review/work4/:
- mutants4.py
- fold_probe.py
- triple_probe.py
- probes3.py
- race_mutants.py
- ten_gates.sh
- check_core.log
