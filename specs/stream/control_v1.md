# World Stream control channel v1 (IF-0 `control.sock`): the pace command

*Status: v1 draft, round 1, a C2 contract for P1.17 b (SPR-027 T-01; a reviewed spec PR of its own before SPR-027 builds the socket: REV-WAVE5 W30, D54 item 5). Reference: `specs/stream/selftest_control_v1.py` (the method table, every request and response, the refusals, a simulated engine clock under a governor with a fake wall clock showing that pacing changes no stream byte, the run log, golden frames); golden file: `specs/stream/golden/control_v1.txt`. The carrier it reuses: `specs/stream/wire_v1.md` §2 (framing), §3.1 (canonical JSON), §4.4 (the error model) and §5 (the version rule), which §8 of that document reserves for this one. Contracts it implements: `docs/05_architecture/05.10_world_stream.md` §3 ("pace is your setting") and §7 item 4 ("pace is a control command, not an input"); `05.1` §4.4 (the speed bands); `05.9` IF-0; `docs/03` STR-5, the P1a half (D-5); ADR-0030 item 5; ADR-0024; ADR-0025. Rulings it is bound by: REV-SPR-026 R12 (BCK-002 A-36 moves to P2: neither SPR-026 nor SPR-027 journals an input; pace is run-log only), REV-WAVE5 W35 (a pause writes no record and no keyframe) and W37 ("advance to tick T" is the state after Bookkeeping of T). A change to a method's parameters or result, an error code, the band tokens or the run-log line is a new version (`rehoboam/stream/control/v2`), announced like a Registry change (`05.9` IF-9). Tests: `docs/09` V-STR-01 ("at any pace"), V-STR-03, V-IF-02 (style) (§11).*

## 0. What this is

The engine runs headless at the pace the director sets. `05.10` §3: "the engine's pace is a speed band (Live, 1 h/s … 1 decade/s) or MAX; pace never changes what happens, only how fast it happens." STR-5: "the engine's pace shall be set only by control commands, logged in the run log and never in the input journal." This document is the command: how a controller connects to the engine host, reads the pace, sets a band, pauses, resumes and advances the run to a tick, what the host answers, what it refuses, and what it writes where.

Four facts shape it and are not negotiable here (they are the ADRs' and the rulings'):

1. **The pace is host-side state, never history.** History is `f(engine_build, registry_version, manifest_hash, seed, input_journal)` (ADR-0006; CLAUDE.md). The band, the state and every command here live in the host's run loop, outside plane A: they are never journaled, never a record of channel A or C, never in a keyframe, and never in `state_hash`. A paused run writes no record and no keyframe (W35). A command here is therefore **not an input** and needs no acceptance cursor (A-36 is P2's, with the first journaled input on this socket).
2. **Pacing changes what is seen, never what happens.** A run paced through this socket, with any sequence of commands, produces the same channel-A bytes, the same `canonical_digest` per sealed year, the same keyframe set, the same ladder and the same journal as the headless run at `max` (V-STR-01 "at any pace"; `05.1` §4.4). The reference proves it on a simulated engine (§9) and SPR-027 T-03 proves it on `solo` (§11).
3. **The engine never waits on a controller** (`05.10` §7 item 3; PERF-5). A controller waits on the engine when it wants to (it polls `control.status`); the engine's loop reads one cell between instants and never a socket.
4. **`control.sock` carries the pace command and nothing else in v1.** Material inputs (tip and scheduled inputs, forks: STR-4) are P2.7's, over this same socket (D47). Their names are reserved here and refused by name (§5); until they exist, nothing on this socket reaches plane A, and the read-only property of the stream sockets (`wire_v1.md` §4.5) extends to the control socket in v1 as a fact about the method table (§7).

IF-0 is three sockets in the run directory (`wire_v1.md` §0): `A.sock` and `C.sock` carry the stream, read-only in every version; `control.sock`, this document, is the one socket on which the world can be told anything, and in v1 it can be told only how fast to go.

## 1. Transport and permissions

| Item | v1 |
|---|---|
| Socket | A **Unix domain stream socket** (`SOCK_STREAM`) at `<run-dir>/stream/t<timeline_id, 10 digits>/control.sock`, beside `A.sock`, served by the **engine host** (the process that runs the engine's loop and `rh-stream`'s writer: `rh-cli run … --socket`), from the same module that serves `A.sock` (`crates/rh-cli/src/serve.rs`, the one file of `rh-cli` allowed a socket: `bin/lint_forbidden.py` `NETWORK_FILES`). The sifter never serves it (it has no pace). A fork, when forks exist (P2), has its own directory and its own control socket. |
| When it exists | Whenever the host serves its stream socket: `--socket` creates `A.sock` and `control.sock` together, before the first instant, and removes both at a normal exit. A headless run (no `--socket`) has no control socket, runs at `max` and can be neither paused nor paced; it is the run `bin/check_all.sh`'s golden gates and the performance baselines measure, and it pays nothing for this document (one branch per instant in the loop, §6). |
| Platform | Linux (D5). The 107-byte `sun_path` limit applies as in `wire_v1.md` §1: a host whose path is longer refuses to serve, naming the path, and never falls back silently. |
| Modes and ownership | The run directory is `0700`; `control.sock` is **`0600`** and belongs to the user running the host, like every socket of IF-0. The host holds an exclusive `flock` on `control.lock` beside the socket for its lifetime (the lock layout of `wire_v1.md` §1 and REV-SPR-026 R20: one lock file per served socket), so a second host for the same timeline cannot serve, a stale socket file after a crash is removed by the next host that holds the lock, and nobody unlinks a live socket. |
| Connections | Any number of **controllers**, each on its own connection, capped by the host (`too_many_controllers`, §5; default 8). Every connection may send every command: there is no owner, no lease and no lock on the pace (§7). |
| Direction | Both sides send **CONTROL frames only** (§2). The host never sends a RECORDS frame on this socket; a RECORDS frame from either side is `invalid_frame`. A frame from a controller is at most 4 KiB, as a viewer's is on the stream sockets. |
| What reaches the engine thread | **One cell.** The serving thread of a connection validates the request, applies it to the governor's cell under the host's own lock (never a lock the engine thread takes on the tick path: §6), appends the run-log line, and answers. The engine's loop reads the cell between instants. Nothing a controller sends is parsed, stored or seen on the engine thread. |
| Lifetime | The socket file is created before the first instant and unlinked at a normal exit, after the last `finished` answer the host gives (§8). A controller that cannot connect has no run to pace: it reads the sealed segments, or starts one. |

## 2. Framing and encodings: the wire's, reused

The control channel reuses `wire_v1.md`'s frame layer **as it is**, because a second framing would cost a second codec, a second fuzz corpus and a second conformance test for one more method table, and `wire_v1.md` §8 already fixes that this socket uses that framing:

- **Frames** (§2 there): the 12-byte header `RHWS`, `version` 1, `type` 1 (CONTROL), `flags` 0, `length`; a frame from a controller is at most 4 KiB and is judged on its header first; a RECORDS type (2) is `invalid_frame` **in both directions** on this socket (the stream's §2.2 sub-header never appears here); a nonzero flag, a bad magic, a length over the limit and a payload that does not decode are protocol errors, answered once with `id` null and then the connection is closed, exactly as there.
- **Canonical JSON** (§3.1 there): one JSON-RPC 2.0 object per frame, keys sorted, no whitespace, integers only (a float anywhere is `invalid_params`, R24), printable ASCII strings, depth at most 4, no repeated key, no unknown envelope member, no batch.
- **The error model** (§4.4 there): `{"code", "message", "data": {"reason", …}}`; the standard codes and the wire's −32110 and −32111 keep their numbers and meanings; this document adds codes in **−32120 to −32129** (§5), its own range inside IF-3's validation band, so that a code never means two things on two sockets of one run.
- **The version rule** (§5 there): the frame header's `version` is the wire's; an unknown one is answered `unsupported_version` at v1 and closed. The **method set** has its own version string, `rehoboam/stream/control/v1`, reported by `control.status` (§4.1), which every control version answers with the same two members `control.format` and `control.version` (frozen, so a controller of any version learns from `control.status` which method set it is talking to). Additive changes (a new result field, a new notification method) are minors and are not on the wire; a controller ignores a result field and a notification it does not know and the host never ignores anything (an unknown parameter is `invalid_params`, an unknown method `method_not_found`).

The values are the wire's too (§3.2 there): `tick` an `i64` of minutes, `seq` a `u64`, `timeline_id` a `u32`, `run_nonce` a `u64`, the history tuple's strings and digest, `band` and `state` tokens from the tables below, a `wall_ns` an integer of nanoseconds (host-side; it appears in the run log, never on the wire: §8).

`rh-protocol` therefore gains no module for this socket but a second **dispatcher** over the same `wire` and `control` codecs, with its own method table (§7): `dispatch_control(&mut Connection, &[u8]) -> Vec<Vec<u8>>` beside the stream's `dispatch`, both in the crate's `control` module. The conformance test reads this document's golden file as it reads the wire's.

## 3. The pace: bands, states and the governor's rule

### 3.1 Bands (`05.1` §4.4)

The band is the rate of sim-time per wall-second. The tokens are those `stream.status.pace.band` already carries (`wire_v1.md` §4.1); the rates are integers derived from the world's calendar (`05.1` §4.1: `minutes_per_year`, 518,400 for a 360-day year, 525,600 for a 365-day one), so that the governor's arithmetic has no float and no rounding:

| Token | Meaning | Rate `r`, in sim-minutes per wall-second |
|---|---|---|
| `live` | real time: one sim-minute per wall-second (the band where the street bubble exists, `05.1` §4.4; SPR-027's specs table: "`live` is one sim-minute per wall-second") | 1 |
| `1h` | one sim-hour per wall-second | 60 |
| `1d` | one sim-day per wall-second | 1,440 |
| `1w` | one sim-week per wall-second | 10,080 |
| `1mo` | one sim-month per wall-second: a twelfth of the year | `minutes_per_year / 12` (43,200 or 43,800) |
| `1y` | one sim-year per wall-second | `minutes_per_year` |
| `10y` | one sim-decade per wall-second | `10 × minutes_per_year` |
| `max` | as fast as the hardware allows: the governor never sleeps | ∞ (no rate) |

`1mo` is a twelfth of the year, not the current month's length, so that a band is one number for the whole run and a controller can predict the wall time of a tick. A host starts at `max` unless its command line says otherwise (`rh-cli run … --socket --pace <band>`; the default is `max` because every gate and every baseline runs headless at `max`, and a paced default would make the first run with a viewer attached look slow for no reason).

### 3.2 States

| Token | Meaning |
|---|---|
| `running` | The host advances the engine instant by instant, sleeping between instants as the band says (§3.3). A pending **step** (§4.5) ends in `paused` when its tick is reached. |
| `paused` | The host holds the engine between two instants: the tip does not move, **no record and no keyframe are written** (W35), the stream's window does not move, no segment is sealed except one whose seal was already in flight off-thread when the pause landed (`wire_v1.md` §6.1: a seal runs off the engine thread and completes on its own), and the journal is untouched. Every command is still answered. |
| `finished` | The run has ended: the last instant's Bookkeeping ran, the last year is sealed, `stream.end { reason: finished }` was sent on the stream socket. The pace can no longer change (`run_finished`, §5); `control.status` is answered until the host exits (§8). A host that stops on an error (`aborted`, `wire_v1.md` §4.3) is `finished` too, with `control.status.end` saying which. |

The state machine: `running` ⇄ `paused` (by `control.pause` and `control.resume`; a step reaching its tick is a pause the host issues itself), and either → `finished` (the run's end or an abort, never a command). There is no transition out of `finished`.

### 3.3 The governor's rule (binding on the host, SPR-027 T-03)

The host keeps one cell, written by serving threads under the host's lock and read by the engine's loop between instants: `(band, state, step_target: tick | null, anchor_tick, anchor_wall_ns)`. The **anchor** is the pair (the tip's tick, the wall clock in nanoseconds) at the moment the cell was last written by a command or by the host's start.

Before the engine runs an instant at tick `t`, the loop:

1. **reads the cell once** (one load, no lock the engine thread can wait on: the cell is an atomic snapshot like `LiveEdge`'s, `wire_v1.md` §6.2, or a mutex only serving threads and this one load touch for nanoseconds; never a lock a controller can hold across a socket write);
2. if `state = paused`: waits, by polling with a short sleep or on a wake the serving thread issues without waiting, until the cell changes; then goes to item 1;
3. if `band ≠ max` and the deadline `anchor_wall_ns + ⌈(t − anchor_tick) × 10⁹ / r⌉` (`r` from §3.1; integers, rounded up) is still ahead: **waits until the deadline or until the cell changes**, whichever is first, then goes to item 1 (so a command that lands during a sleep is applied before the instant, and a pause lands at once rather than one sleep later); `max` has no deadline;
4. runs the instant to the end of its Bookkeeping, and the writer appends its records as always;
5. if `step_target` is not null and the **next** instant's tick is above `step_target` (or there is no next instant): the last instant at or below the target has run; writes `state = paused`, `step_target = null` into the cell, resets the anchor and appends the run-log line (`step.reached`, §8); the next iteration waits at item 2.

Consequences a controller may rely on and the reference checks (§9):

- **An instant is never split.** The host waits only between instants; a command that lands while an instant runs is applied before the next one, and one that lands during a wait ends the wait. So a `control.pause` answered at tip `t` holds the tip at `t` exactly, and `control.status.tip.tick` after a pause is the tick of the last instant whose Bookkeeping ran.
- **Paused time is not owed.** The anchor is reset by every command (a resume at tick `t` anchors at `t` and now), so a run paused for an hour at `1y` does not race through the lost hour when resumed. For the same reason a band change does not race or lag: it starts fresh from the tip.
- **The band bounds the wall time from below, never from above.** At `1y` a sim-year takes at least one wall-second; on a world that simulates slower than the band it takes as long as it takes (the engine's speed is PERF budgets' business, `08` §1). The host measures nothing here and reports nothing here; the TUI shows the lag.
- **The sleep is in the host's loop and in no plane-A crate.** The wall clock is read by `rh-golden`'s run loop (tool plane), never inside `rh-core`, `rh-snapshot` or any `rh-sys-*` (`bin/lint_forbidden.py`'s clock rule; SPR-027 T-03's lint line). Plane A has no notion that time passed.
- **Publish cadence is unchanged in v1.** `05.1` §4.4 says bands "change only the publish cadence and how far `advance_until` goes per wall interval"; the P1a writer appends after every instant at every band (one frame may hold one record at `live` and thousands at `max`, `wire_v1.md` §2.2), so v1 changes the second and leaves the first alone. A later version may coalesce publication at slow bands; nothing here prevents it and nothing in the records would show it.

## 4. Methods

The namespace on this socket is `control.*`. The requests of v1 are the five below; every other method, in any namespace, is `method_not_found` (§5), with a message that names what v1 accepts and, for a reserved name, why it is refused (§5.1). Requests carry an `id`; responses carry the same `id`, in request order per connection (a controller may pipeline); a notification or a response from a controller is `invalid_request`. The host sends no notification in v1 (what the alternative would add: §10).

Every command (§4.2 to §4.5) returns the same **pace object** after the command was applied, so a controller never needs a second round trip to know the state it caused:

```json
{"band": "1y", "state": "running", "step": null, "tick": 216000}
```

| Field | Type | Meaning |
|---|---|---|
| `band` | token (§3.1) | The band in force after the command |
| `state` | token (§3.2) | The state after the command |
| `step` | `{"until_tick": i64}` \| `null` | The pending step's target, if a step is pending (§4.5) |
| `tick` | `i64` \| `null` | The tip: the tick of the last instant whose Bookkeeping ran when the command was applied (`null` before the first instant; the same number as `stream.status.tip.tick` on `A.sock`, from the same cell) |

A command is **accepted** when it is well-formed and the state admits it; an accepted command changes the cell, appends exactly one run-log line (§8; a step whose target is reached at once also appends its `step.reached` line, §4.5), and is answered with the pace object. A **refused** request (§5) changes nothing: no cell write, no run-log line, no anchor reset. An accepted command whose effect is already in force (`control.pause` while paused, `control.pace` to the band in force, `control.resume` while running with no step pending) is still accepted, still answered and still logged, with `changed: false` in the result (§4.2 to §4.4), so that a controller's commands are idempotent and the run log says what was asked.

### 4.1 `control.status`

Request: `{"id": 1, "jsonrpc": "2.0", "method": "control.status"}` (`params` absent or `{}`; any member is `invalid_params`). Accepted in every state, logged never (a read).

Result, from one read of the cell and one snapshot of the stream (`wire_v1.md` §6.2), so that `tip` and `pace` agree:

| Field | Type | Meaning |
|---|---|---|
| `control` | `{"format": "rehoboam/stream/control/v1", "version": 1}` | This document. **Frozen across control versions**: every version answers `control.status` with these two members first, so a controller learns the method set from the one method every version has |
| `timeline_id` | `u32` | The timeline this socket controls (P1a: 0) |
| `run_nonce` | `u64` | This host's start (`wire_v1.md` §3.2): a controller that reconnects and finds another nonce with the same `history` is talking to a resumed host, whose pace is the start pace again (§8); another `history` is another run |
| `history` | `{"engine_build", "registry_version", "manifest_hash", "world_seed"}` | The history tuple (`wire_v1.md` §4.1). The pace is not in it |
| `calendar` | `{"minutes_per_year": int}` | The year, so a controller computes a band's rate (§3.1) and a date's tick |
| `pace` | the pace object | `band`, `state`, `step`, `tick` as above |
| `tip` | `{"tick": i64 \| null, "next_seq": u64}` | The stream's tip (`wire_v1.md` §4.1): `tick` equals `pace.tick`; `next_seq` is channel A's next seq, from the stream snapshot, so a controller can confirm that nothing was written while paused (`next_seq` unchanged across the pause) |
| `end` | `{"reason": "finished" \| "aborted"}` \| `null` | Null while the run goes on; the reason when `state = finished` (the stream's `stream.end.reason`) |
| `controllers` | `int` | Connections open on this socket, this one included (§7) |

Invariants the reference checks: `pace.tick = tip.tick`; `state = finished` ⇔ `end ≠ null`; `step ≠ null` ⇒ `state = running` and `step.until_tick > tip.tick`.

### 4.2 `control.pace`

Request: `{"id": 2, "jsonrpc": "2.0", "method": "control.pace", "params": {"band": "1y"}}`.

- `params` is exactly `{band}`; `band` is one of the eight tokens of §3.1. A missing or extra member, a non-string, an unknown token (`"1yr"`, `"MAX"`, `"1 y"`) or a float anywhere is `invalid_params`, with `data.detail` naming the member.
- Accepted in `running` and `paused`: the band changes, the state does not (a band set while paused takes effect at the resume), the anchor is reset, a pending step stays pending. Refused in `finished` with `run_finished`.

Result: the pace object plus `"changed": bool` (false when the band was already in force).

### 4.3 `control.pause`

Request: `{"id": 3, "jsonrpc": "2.0", "method": "control.pause"}` (no params).

- Accepted in `running`: the state becomes `paused` before the next instant (§3.3: the instant in progress, if any, completes; its records are written; then the tip holds). A pending step is cleared (the pause is the controller's, now; `step` is null in the result). Accepted in `paused` with `changed: false`. Refused in `finished` with `run_finished`.
- **What a pause does not do** (W35; `05.10` §4's "keyframe markers at auto-pauses" is P2's Time-Machine, not this): it writes no record of any kind on any channel, no keyframe, no journal row, and no segment is sealed by it (a year completed before the pause seals on its own schedule, off-thread, as it would have). `stream.status.tip.next_seq` on `A.sock` is the same number before the pause and at the resume; the reference and SPR-027 T-03 assert it.

Result: the pace object plus `changed`.

### 4.4 `control.resume`

Request: `{"id": 4, "jsonrpc": "2.0", "method": "control.resume"}` (no params).

- Accepted in `paused`: the state becomes `running` at the band in force, the anchor is reset to (the tip, now), so no paused time is owed. Accepted in `running` with `changed: false` when no step is pending; when a step is pending, `control.resume` **clears it** (the run goes on past the target) and reports `changed: true` with `step: null`. Refused in `finished` with `run_finished`.

Result: the pace object plus `changed`.

### 4.5 `control.step`: advance to a tick

Request: `{"id": 5, "jsonrpc": "2.0", "method": "control.step", "params": {"until": {"tick": 432000}}}` or `"until": "instant"`.

- `params` is exactly `{until}`; `until` is `{"tick": i64}` (advance to tick `T`) or the string `"instant"` (advance one instant: `T` is the tick of the next instant, which the host knows and the controller does not). Anything else is `invalid_params`.
- **Meaning (W37):** the host runs, at the band in force, until **after Bookkeeping of the last instant whose tick is at or below `T`**, then pauses. When `T` is an instant's tick, that is the state after Bookkeeping of `T`: the state a `STEP_DIGEST` at `T` hashes (`channel_a_v0.sql`: "once per Month pulse, after Bookkeeping") and the state `rh_golden::materialize(run_dir, T)` rebuilds (SPR-027 T-02). When `T` falls between two instants, the host pauses after the earlier one and `pace.tick < T`; a controller that wants an exact tick names an instant (a day start, in P1a) or reads `pace.tick` from the result of the pause.
- Accepted in `paused` and `running`: the state becomes (or stays) `running`, the step's target is set (a second `control.step` **replaces** a pending target; the result shows the one in force), the anchor is reset. When no instant lies at or below `T` beyond the tip (`T` is between the tip and the next instant), the target is reached at once: the command is accepted, logged (`control.step:<T>` then `step.reached`, §8), and the result already shows `state: paused`, `step: null`. A `control.pause` before the target is reached cancels it (§4.3); a `control.resume` clears it (§4.4). Refused: `past_tick` when `T` is at or below the tip (`T ≤ pace.tick`, or `T ≤ 0` before the first instant): the step is not an input and cannot fork, so a tick in the past is simply not somewhere a run can advance to (ADR-0025 §5 is about inputs; a controller that wants the state at a past tick materializes it, `05.10` §4); `run_finished` in `finished`.
- When the target is reached the host itself pauses (§3.3 item 5) and logs it (§8); the controller sees it as `state: paused` and `step: null` in `control.status`, with `pace.tick` the tick reached. **The host sends no notification**: a controller polls `control.status` (a read of one cell and one snapshot, cheap at any rate), which keeps the serving thread from holding anything across a wait and keeps v1 to requests and responses. A `control.reached` notification is the additive change a later minor would make (§10).

Result: the pace object (`state: running`, `step: {until_tick: T}` with `T` resolved for `"instant"`) plus `changed` (false when the same target was already pending).

### 4.6 Worked sequence

A controller that wants to watch year 10 at one sim-day per second and then let the run go:

```text
control.status                       -> pace {band: max, state: running, step: null, tick: 4665600}, tip.next_seq 8112
control.step {until: {tick: 5184000}} -> pace {band: max, state: running, step: {until_tick: 5184000}, tick: 4665600}, changed true
  (poll) control.status              -> pace {band: max, state: paused, step: null, tick: 5184000}   # after Bookkeeping of 5184000
control.pace {band: 1d}              -> pace {band: 1d, state: paused, step: null, tick: 5184000}, changed true
control.resume                       -> pace {band: 1d, state: running, step: null, tick: 5184000}, changed true
  … 360 wall-seconds later, year 11 begins …
control.pace {band: max}             -> pace {band: max, state: running, step: null, tick: 5702400}, changed true
```

The run log then holds six `pace` lines (§8); channel A holds exactly the records the headless run holds, at the same seqs, with the same digests (§9).

## 5. Refusals

A refused request is answered with an error response carrying the request's `id` (or `null` where the wire says so) and **changes nothing**: no cell write, no run-log line, no anchor reset, no effect on any other connection. The wire's codes keep their numbers and meanings; this socket's own reasons take −32120 to −32129.

| Code | `reason` | Extra `data` fields | When |
|---|---|---|---|
| −32700 | `parse_error` | `"detail"` | The payload is not JSON, or exceeds the parser (`wire_v1.md` §4.4) |
| −32600 | `invalid_request` | `"detail"` | Not a JSON-RPC 2.0 request object; a batch; an unknown envelope member; a non-ASCII string; a notification or a response from a controller |
| −32601 | `method_not_found` | `"detail"` (present for a reserved name, §5.1) | A method outside the five of §4. **The message is this socket's**: "this socket carries the control channel; v1 accepts the pace command only (control.status, control.pace, control.pause, control.resume, control.step) and no input". A `stream.*` method sent here gets it too (the stream is on `A.sock` and `C.sock`), and a `control.*` method sent on a stream socket gets the stream's read-only message (`wire_v1.md` §4.4) |
| −32602 | `invalid_params` | `"detail"` | A parameter missing, of the wrong type, out of range or unknown; an unknown band token; a float anywhere in the payload (R24); a member in the params of a method that takes none |
| −32110 | `unsupported_version` | `"supported": [1]` | The frame header's version (the wire's rule; `id` null, then close) |
| −32111 | `invalid_frame` | `"detail"` | A bad magic, type, flag or length; **a RECORDS frame in either direction**; a frame from a controller over 4 KiB (`id` null, then close) |
| −32120 | `too_many_controllers` | `"limit"` int | The host's connection cap on this socket (default 8); sent as the first frame on the new connection, `id` null, then close. **Retriable**. It bounds serving threads, never the engine |
| −32121 | `run_finished` | `"end": "finished" \| "aborted"` | `control.pace`, `control.pause`, `control.resume` or `control.step` after the run ended. **Terminal**: the run will not move again; `control.status` still answers |
| −32122 | `past_tick` | `"tip_tick": i64 \| null` | `control.step` to a tick at or below the tip |

Every message is for people and may change in a minor; `data.reason` and the code are the contract. The golden file holds one frame per reason (§9).

### 5.1 Reserved names, refused by name (P2.7; D47)

The following methods are **reserved** for the first version that journals an input on this socket (P2.7, `docs/11`; STR-4; BCK-002 A-36). In v1 every one of them is `method_not_found` with `data.detail` naming the reservation, so a P2 client talking to a v1 host learns at once that it is early, and so that no v1 host ever answers one of them by accident:

| Reserved method | What it will be (P2, not here) | v1 `data.detail` |
|---|---|---|
| `control.input` | A material input landing at the next tick boundary after the tip (STR-4; `05.10` §2 "now") | `reserved for P2.7 (D47): a material input is journaled through IF-2 with its acceptance cursor (BCK-002 A-36) before it is applied; v1 accepts no input` |
| `control.schedule` | A scheduled input, journaled when issued and applied at its tick (`05.10` §2 "scheduled") | the same |
| `control.fork` | A fork at a past tick (ADR-0025 §5; `05.9` IF-3 `timeline.fork`) | `reserved for P2.7 (D47): a fork is a new timeline with its own directory and sockets; v1 has one timeline` |
| `control.autopause` | The Watch and Play auto-pause (STR-5's P2 half, D-5; `05.4` §2.3) | `reserved for P2 (D-5): the auto-pause is the sifter's and the viewer's, not v1's` |
| `control.run_until` | Run-until on a predicate or the next match (`05.1` §4.4) | `reserved for P2: a predicate over aggregates needs the sifter; advancing to a tick is control.step` |
| `player.*`, `oracle.*`, `timeline.*`, `console.*`, `sift.*`, `world.*`, `chronicle.*`, `entity.*`, `foresight.*`, `bubble.*`, `map.*`, `dialogue.*`, `compile.*`, `narrative.*` | IF-3's namespaces (`05.9`), for `rh-server`'s WebSocket in P2 | `reserved for IF-3 (05.9), not carried by control v1` |

The reference's dispatcher answers each reserved name with the detail above, and its method-table check fails on any dispatcher that accepts one (§9). A later version that implements one of them bumps the method set to v2 (a new request method is a major change, `wire_v1.md` §5) and brings its journaling with it: **nothing is journaled on this socket until IF-2 says how** (`wire_v1.md` §8).

## 6. What the engine sees

Nothing. Normative, for SPR-027 T-03 and every later version:

- **Plane A is untouched.** `rh-core`, `rh-events`, `rh-snapshot`, `rh-stream` and every `rh-sys-*` crate have no type, no field and no function for the pace. The governor is in `rh-golden`'s run loop (tool plane), the socket in `serve.rs`, the method table in `rh-protocol` (tool plane, a leaf). `bin/lint_arch.py`'s plane-A closure and `bin/lint_forbidden.py`'s `network`, `network-reexport` and clock rules hold as they are; SPR-027 adds no escape.
- **The pace is in no hash.** `state_hash` covers plane-A state (`05.1` §6; SPR-007); the cell is not plane-A state. The reference's planted control `band-in-hash` mixes the band into the digest and must FAIL pace invariance (§9).
- **The pace is in no record and no keyframe.** Channel A's kinds are `channel_a_v0.sql`'s; none is a pace. A keyframe is written when `05.1` §4.2's Bookkeeping trigger says (every 10 sim-years) and never because the run paused (W35). The reference's `pause-writes-record` control appends a line at a pause and must FAIL.
- **The pace is not an input.** The journal (`rh-snapshot` `journal.rs`; IF-2) gains no row from this socket in v1; its row count and chain head are the same after any command sequence (SPR-027 T-03's test). The reference's `pace-in-journal` control journals a band change and must FAIL.
- **One branch per instant.** The headless cost of the governor is the read of one cell before each instant and a comparison; V-PERF-04 and V-PERF-11 (instructions and allocations per steady-state tick at `max` with no socket) are unchanged (SPR-027 T-03's acceptance).
- **No socket on the tick path.** The serving threads of this socket write the cell under the host's lock and never touch the writer, the window or the engine; the engine's loop reads the cell and never a socket. "The engine never waits for a reader" (`wire_v1.md` §0 fact 1) becomes here "the engine never waits for a controller": it waits at a pause because it was told to, which is the point, and for nothing else.

## 7. Concurrency: many controllers, one cell, last write wins

- **Any connection may command.** The socket is `0600`, so every controller is the one user, and the host serializes commands under its lock in arrival order: the cell holds the last accepted command, and the run log holds them all in that order (§8). A second controller gets the same methods and the same answers as the first; `control.status.controllers` tells it how many there are. There is no owner, no lease, no `control_held` error.
- **Why not one controller at a time.** The P1a controllers are short-lived tools: `rh-cli pace <run-dir> <band>`, `rh-cli pause <run-dir>`, `rh-cli resume <run-dir>`, `rh-cli step <run-dir> --tick T` (each one connection, one request, one answer, exit) and, later, a long-lived harness (`rh-tui`, P2.7; `rh-view`, ADR-0036). A lease would make the one-shot tools fail whenever the harness is open, for no safety gained: the pace is not history, a wrong band costs nothing, and the run log says who asked what. The alternative is §10's.
- **Ordering.** Commands from one connection are applied in request order; commands from two connections are applied in the order the host's lock admits them, and each answer reports the state its own command left. A controller that reads `control.status` after another controller's command sees that command's effect (one cell, one lock).
- **The method table is the read-only property, inverted.** A conformance test enumerates the method table of every control version: v1's is exactly `{control.status, control.pace, control.pause, control.resume, control.step}`, and the stream sockets' tables never gain a `control.*` entry (`wire_v1.md` §4.5's invariant; its planted control `stream-pace` and this document's `pace-on-stream` both fail). The two tables are disjoint for every version of both.
- **The cap.** `too_many_controllers` at the host's cap (default 8, a host setting) bounds serving threads, as `too_many_viewers` does on the stream sockets, and is retriable.

## 8. The run log; writer exit, kill and resume

### 8.1 `<run-dir>/run.log`

The host appends one line per accepted command to `<run-dir>/run.log` (`05.10` §7 item 4: "logged in the run log, never in the input journal"; STR-5), a text file in the run directory beside `run.toml`, LF-terminated, ASCII, append-only across the life of the run and across resumes:

```text
pace <wall_ns> <band> <state> <tick> <method> <run_nonce>
host <wall_ns> <event> <tick> <run_nonce>
```

| Line | Fields |
|---|---|
| `pace` | `wall_ns`: the host's wall clock in integer nanoseconds since the Unix epoch when the command was applied (host-side only; never on the wire, never in plane A); `band` and `state`: the pace **after** the command (so the line says what the run is doing from here); `tick`: the tip at the command (`~` before the first instant); `method`: `control.pace`, `control.pause`, `control.resume`, `control.step` (with the target appended as `control.step:<until_tick>`), or `step.reached` for the pause the host issues itself at a step's target (§3.3 item 5); `run_nonce`: the host's |
| `host` | `event`: `start` (the host began serving, with the start band and state in a `pace` line that follows at once), `finish`, `abort` (`wire_v1.md` §4.3's reasons) or `resume` (a host that started from the sealed segments and a keyframe after a kill, §8.2); `tick`: the tip at the event |

The first two lines of a fresh run are therefore `host … start ~ <nonce>` and `pace … max running ~ host.start <nonce>` (the start band; `control.status` would answer the same). A refused request writes no line. `control.status` writes no line.

**What the run log is not.** It is not part of history: `history = f(build, registry, manifest, seed, journal)` does not read it; `rh-cli replay`, `rh_golden::materialize` and `rh-cli tail` never open it; a run directory with it deleted replays and materializes to the same hashes. It is not in any hash and not in any segment. It is a record for people (the director reading why year 300 took an hour) and for SPR-027 T-03's probe (which asserts one `pace` line per accepted command and none per refusal). The read-only test (SPR-013 T-10's form) excludes it from nothing: `tail` and `materialize` do not write it, and the host is the only writer; a test that diffs the run directory's tree after a `tail` sees the same `run.log` bytes it saw before.

**Its format is this document's.** The line above (the W30 row's `pace <wall_ns> <band> <state>` with the tip, the method and the nonce appended: specs table) is versioned with the method set; a change to it is a change to this document.

### 8.2 Writer exit, kill and resume

| Event | What the host does | What a controller sees |
|---|---|---|
| **Normal end** (the horizon reached) | Bookkeeping of the last instant, the last seal, `stream.end { finished }` on the stream socket, `host … finish` in the run log, `state = finished` in the cell; the host then answers `control.status` and refuses the four commands with `run_finished` **for a grace period** (a host setting; default 2 seconds, so a controller that polls sees `finished` rather than a closed socket), then unlinks `control.sock` and `A.sock`, releases the locks and exits | `control.status.pace.state = finished`, `end.reason = finished`; then the connection closes |
| **Abort** (a poisoned engine, BCK-002 A-42; `wire_v1.md` §6.2) | As above with `aborted`: no partial seal, `stream.end { aborted }`, `host … abort`, the grace period, exit | `state = finished`, `end.reason = aborted`, `run_finished { end: aborted }` |
| **`kill -9`** | Nothing: the socket file and the lock file stay on disk, the lock is released by the kernel, the run log ends where it ended (its last line may be the last accepted command; a line is written whole or not at all: the host writes each line with one `write` of the full line and `fsync`s nothing, so a torn line is impossible and a lost line possible, and neither matters to history) | The connection closes without a frame. A controller that reconnects fails to connect (no host) until a host resumes |
| **Resume** (`rh-cli run … --socket` on a run directory with sealed segments and a keyframe: SPR-013 T-09's line, `wire_v1.md` §6.2's resumed host) | A new `run_nonce`; the stale socket files are removed under the locks and new ones created; **the pace starts again at the command line's band (default `max`) and `running`**: the pace was never state, so there is nothing to restore, and the run log is not read back (a host setting the band from a log line would make the log an input to the host's behaviour, which it is not; the controller that paused the old host sets the new one). The run log gains `host … resume <tick> <new nonce>` and the start `pace` line, after the old lines; the resumed host re-emits the lost year's records as the same bytes (`wire_v1.md` §6.2), at whatever pace it is now told | A controller that reconnects reads `control.status`: a new `run_nonce` with the same `history` is the resumed host, at the start pace; it sets the band again if it wants one. A different `history` is another run |

A pause therefore does not survive a kill, and a controller that needs a run held across a restart holds it by not restarting it. What changes under the alternative (restore the band from the log): §10.

## 9. The reference, the golden file and the controls

`specs/stream/selftest_control_v1.py` (standard library only; it loads `selftest_wire_v1.py` by path for the frame codec, the canonical JSON, the request and error builders and the stream's method table, and `canonical_v0.py` through it) is the independent reference `rh-protocol`'s control dispatcher and `rh-golden`'s governor must agree with. It holds:

1. **The method table and the dispatcher.** `METHODS` is exactly the five methods of §4; `RESERVED` is §5.1's table; `Host.dispatch` answers every request of §4 and every refusal of §5 over a simulated engine and a fake wall clock. A check asserts the table, the disjointness from the wire's `METHODS`, the detail of every reserved name, and that no accepted command's name is in `RESERVED`.
2. **The simulated engine.** `Engine` is a deterministic record generator: instants at fixed ticks (months of a 360-day year, 12 per sim-year, four years), each emitting a fixed number of canonical-shaped lines derived from the tick alone, a `STEP_DIGEST` whose `state_hash` is a hash of the state so far, a `KEYFRAME` every two years (the simulation's cadence; the engine's is ten), a journal with one row before the run, and a ladder of per-year hashes. It knows nothing of the pace: `Engine.run_instant()` takes no honest argument (the `band-in-hash` control is the one caller that passes it one).
3. **The governor** (`Governor`, §3.3) over a `FakeClock` whose `sleep_until` advances it, so that the rule's arithmetic (integers only) and the anchor resets are exercised exactly, with the wall time at which each instant was released recorded.
4. **Pace invariance.** The same engine is run headless at `max` and under five scripted command sequences (`1y` throughout; `10y`; a pause of two wall-seconds after year 1 and a resume, with `control.status` read inside the pause; a pause at the start, a step to a tick between two instants, a band change while paused, a step of one instant, a step to a later tick and a resume; two controllers interleaving band changes, a pause, a resume and a step), and the check asserts: the concatenated channel-A lines are byte-identical; the per-year digests are equal; the keyframe set is the same; the journal is unchanged (row count and chain head); the ladder is identical; **and** that the governor really slept (the released wall times at `1y` are at least one wall-second apart per sim-year; at `max` the clock did not move between instants), that the pause really held the tip (no instant ran between the pause and the resume, `next_seq` unchanged in the status read inside it), that the resume owed no paused time, and that each step paused exactly after Bookkeeping of the last instant at or below its target.
5. **The run log** rendered by the host for each sequence: one `pace` line per accepted command, none per refusal, the `host` lines, in order; compared with the golden `scenario` lines.
6. **Refusals:** every reason of §5 produced by a planted request, the detail checked; the reserved names; a RECORDS frame; a 5 KiB frame; a float band; an unknown band; an extra member; `control.step` to the tip and to the past; the commands after `finished`.
7. **The golden file** `specs/stream/golden/control_v1.txt`: `frame <name> <hex>` with its decoded `control <canonical json>` line for one frame of every request, result and error; `dispatch <name> <request hex> <reply hex>` for every planted request on a fresh connection to a host at a known tip (the engine run to year 1); `scenario <name>` followed by `clock <wall_ns> <tick> <instant>` lines (when each instant was released) and `log <run-log line>` lines for each paced sequence; and `invariance <sha256 of the channel-A lines> <keyframe ticks> <journal head> <last ladder hash>` once, the value every sequence reproduced. `rh-protocol`'s conformance test reproduces every `frame` and `dispatch` line; SPR-027 T-03's governor test reproduces the `clock` lines of each scenario on the same fake clock.

**Planted controls** (`--control <name>`), each failing for its own reason, its marker appearing literally in the source, and never writing the golden file:

| `--control` | What it plants | Must FAIL with |
|---|---|---|
| `pause-writes-record` | The host appends a `PAUSE` line to channel A at every pause (W35 broken) | `a pause wrote a record` |
| `pace-in-journal` | The host journals every band change as an input (STR-5, R12 broken) | `the journal changed under a pace command` |
| `band-in-hash` | The engine mixes the band into `state_hash` (history reads the pace) | `the state hash differs from the headless run` |
| `input-accepted` | The dispatcher accepts `control.input` and journals it (§5.1 broken) | `an input was accepted on control.sock` |
| `step-overshoot` | The governor pauses after the instant following the target (W37 broken) | `the step overshot its target` |
| `pace-on-stream` | The stream's method table (the wire reference's `METHODS`) gains `control.pace` (`wire_v1.md` §4.5 broken) | `the stream method table carries control.pace` |
| `paused-time-owed` | The resume keeps the old anchor, so the run races through the paused wall time | `paused time was owed` |

`bin/check_all.sh --part core` runs `--check`, a stale-golden control (the first frame's hex altered, must say `STALE:`) and the seven controls above; each run takes about a quarter of a second, the nine gates about two seconds.

## 10. What changes under the alternative

Choices the director or the Architect might reverse, and what each reversal costs here:

| Choice made here | The alternative | What changes |
|---|---|---|
| **Five methods**: the W30 row's four plus `control.step` (advance to a tick), because the task brief asks for "step / advance-to-tick" and because W37's "after Bookkeeping of T" is a sentence about exactly this command | The four of the W30 row; a step is `control.pace` to a slow band plus a `control.pause` by the controller at the right moment | §4.5, §3.3 item 5, the `past_tick` code, two golden frames, the `step-overshoot` control and the `step` field of the pace object go; a controller can no longer stop at an exact tick (at `max` the tip moves years per second), so V-STR-02's "advance to tick T" stays a materializer-only notion and SPR-035's scrubber seeks only by materializing |
| **A step is a request, the arrival is polled** (`control.status`) | A `control.reached {tick, seq}` notification to the connection that asked, or to every controller | Additive (a new notification method is a minor change, §2): no frame changes; the serving thread must then watch the cell for the step's end, which is the one place a serving thread would wait on the engine's progress. Recommended if the TUI (P2.7) wants it; not in v1 |
| **Many controllers, last write wins** (§7) | One controller holds the pace: the first connection to command gets it, others get `control_held` (−32123) and may only read; the holder's close releases it | A new code and a lease in the cell; the one-shot `rh-cli pace` tools fail while a harness is open, or the harness must release between keystrokes. No change to the stream, the log or the engine |
| **The pace starts again at `max` after a resume** (§8.2) | The resumed host reads the last `pace` line of `run.log` and starts at that band and state (a paused run stays paused across a restart) | The run log becomes an input to the host's behaviour (never to history), so its format becomes a parsed contract with a stale-line rule; a corrupt log must be refused or ignored by a stated rule; `host … resume` is followed by a `pace` line echoing the restored band. The alternative is the safer one for a director who paused a run at a moment that mattered and then restarted the machine; it is not v1 because the one-shot tools can re-pause in one command and because the log has no integrity check |
| **`1mo` is a twelfth of the year** (§3.1) | `1mo` is the current calendar month's length (28 to 31 days on a 365-day year) | The rate changes at month boundaries, so the anchor must reset at each, or the governor must integrate month by month; a controller can no longer predict a tick's wall time from the band alone. No wire change |
| **`control.sock` is created by `--socket` with `A.sock`** (§1) | A separate `--control` flag, so a run can be paced with no stream served, or streamed with no control | One more flag and one more combination to test; a host that serves `control.sock` without `A.sock` cannot show `tip.next_seq` from a stream snapshot and reports it from the writer instead |
| **The run-log line carries the tip, the method and the nonce** after the W30 row's four fields (§8.1) | The W30 row's `pace <wall_ns> <band> <state>` exactly | A reader cannot tell which command produced the line, nor at what tick, nor which host; the step's target has nowhere to go; SPR-027 T-03's "one line per accepted command" is checkable by count only |
| **The cap is 8 and retriable** | No cap, or the stream's 64 | A cap bounds threads; 8 is more than every P1a tool together opens. No wire change but the `limit` number |
| **The wire's frame layer, reused** (§2) | A line-oriented text protocol on `control.sock` (one JSON object per LF), on the grounds that a controller is a human-driven tool | A second codec and fuzz corpus in `rh-protocol`; the TUI's stdlib client would speak two protocols to one host; the version rule would need restating. Nothing gained but readability in `socat` |

## 11. Acceptance: what SPR-027 proves (T-03, T-06)

| Test (`docs/09`, SPR-027) | Probe | Pass |
|---|---|---|
| V-STR-01 "at any pace" | `solo` for 50 years at `max`, then at `10y`, then with a `control.pause` of two wall-seconds at year 20 and a `control.resume`, then with a `control.step` to a month-pulse tick of year 30 and a resume: the ladders, every sealed segment's `canonical_digest`, the keyframe set and the journal (rows and chain head) compared; the probe asserts the `10y` run took at least 4 wall-seconds, the pause held the tip (`stream.status.tip` unchanged during it, `next_seq` equal before and after) and nothing was written while paused; the stepped run's `materialize(run_dir, T).state_hash` equals the `STEP_DIGEST` at `T` and the tip at the pause is `T` | Byte-identical; the probe's four assertions hold |
| V-IF-02 style | The control endpoint under a fuzz seeded from this document's golden `dispatch` lines and the wire's `refuse` lines: wrong magic and version, a RECORDS frame, a float band, an unknown band, an extra member, depth 5, 100,000 nested arrays, a 5 KiB frame, every reserved name | Each gets its named error; the connection closes where the wire says; the host does not panic; the run continues and the ladder is unchanged |
| Read-only, both ways (`wire_v1.md` §4.5; §7 here) | `control.pace` on `A.sock` → `method_not_found` with the stream's message; `stream.subscribe` on `control.sock` → `method_not_found` with this socket's message; `player.act` and `control.input` on `control.sock` → `method_not_found` with the reservation detail; the journal unchanged after every test | As stated; the two method tables are exactly §4's and `wire_v1.md` §4's |
| The run log | After the V-STR-01 sequence above: one `pace` line per accepted command, in order, with the tip, the method and the nonce; no line per refused request; the `host` lines; the journal untouched | As §8.1 |
| Lints | `network`, `network-reexport`, `protocol-leaf`, the plane-A closure, the clock rule: green on the workspace with the governor in `rh-golden` and the socket in `serve.rs` | Green |
| Headless cost | V-PERF-04 and V-PERF-11 at `max` with no socket | Unchanged |
| Conformance | `crates/rh-protocol/tests/golden.rs` against `specs/stream/golden/control_v1.txt`: every `frame` reproduced and decoded, every `dispatch` answered byte for byte, the method-table test (five entries, disjoint from the stream's three), a planted sixth method and a planted `control.pace` on the stream table each failing by name; `rh-golden`'s governor test reproducing the `clock` lines of every `scenario` on the same fake clock | All green |

## 12. Gates

- `python -I specs/stream/selftest_control_v1.py --check`: the self-test (the method table, every message encoded and decoded, the refusals, the governor's arithmetic, pace invariance over the scripted sequences, the run log), then the golden file compared with the reference's rendering. `bin/check_all.sh --part core` runs it, a stale-golden control (must FAIL with `STALE:`) and the seven `--control` cases of §9, each with its message; a `--control` run never writes the golden file.
- `python -I specs/stream/selftest_control_v1.py` rewrites the golden file after the self-test. It depends on `selftest_wire_v1.py`'s codec and on no golden line of the wire's or the canonical form's, so a canonical change regenerates nothing here.
