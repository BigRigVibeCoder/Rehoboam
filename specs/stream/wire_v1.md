# World Stream wire format v1 (IF-0 live edge) and `rh-protocol`

*Status: v1 draft, a C2 contract for P1.17 (SPR-026, SPR-027). Reference: `specs/stream/selftest_wire_v1.py` (frame codec, control messages, the live-edge rule simulated with slow, fast, stalled and reconnecting viewers, golden frames); golden frames: `specs/stream/golden/wire_v1.txt`. Records: `specs/stream/channel_a_v0.sql`, `channel_c_v0.sql`; their one-line form: `specs/stream/canonical_v0.md`. Contracts it implements: `docs/05_architecture/05.10_world_stream.md` §5 and §7; `05.9` IF-0 and the harness rules; ADR-0024, ADR-0025, ADR-0030, ADR-0031, ADR-0035 §6. Tests: `docs/09` V-STR-01, V-STR-03, V-STR-05, V-PERF-09 (§9 below). A change to a byte of the frame header, a control message's field or an error code is a new version (`rehoboam/stream/wire/v2`), announced like a Registry change (`05.9` IF-9).*

## 0. What this is

The engine runs headless and writes the World Stream: channel A (the world, plane A) is written by the engine host, channel C (sift matches, plane C) by the sifter. Sealed segments are Parquet on disk, one per sim-year per channel (SPR-013): **the chronicle is the stream at rest**. This document is the rest of IF-0: how a viewer reads the **live edge**, the records of the newest sim-year that are not sealed yet, over a local socket, and how a viewer that falls behind the edge goes to the sealed segments and comes back with no gap and no duplicate, by `seq`.

Three facts shape the design and are not negotiable here (they are the ADRs'):

1. **The engine never waits for a reader** (`05.10` §7 item 3; `05.9` harness rules; PERF-5). Appending is a memory write on the engine host's thread. No socket write is on the tick path, nothing back-pressures the engine, and nothing is dropped: a reader that cannot keep up is sent to the sealed segments.
2. **A viewer is never an input to plane A** (ADR-0024 rule 2; ADR-0030). This socket is read-only for the world. It carries no material input and no pace command. Inputs go through the input journal (`05.9` IF-2; `rh-snapshot` `journal.rs`; `rh-cli run --inputs` in P1a; the control endpoint of §8 in P2). The pace command is a control command on the control endpoint (SPR-027), logged in the run log, never on this socket and never in the journal (STR-5).
3. **Pace never changes what happens** (`05.1` §4.4). The wire therefore carries no information that could feed back: a viewer's cursor, its rate and its existence are unknown to the simulation.

## 1. Transport

| Item | v1 |
|---|---|
| Socket | A **Unix domain stream socket** (`SOCK_STREAM`), one path per channel, in the run directory: `<run-dir>/stream/A.sock` (served by the engine host, the process that runs `rh-stream`'s writer) and `<run-dir>/stream/C.sock` (served by the sifter process, `rh-cli sift … --follow`; SPR-013 ruling 14). A process serves the channels it writes and no other: one writer per channel (`05.10` §7 item 1) means one server per channel. |
| Platform | Linux (D5). Unix socket paths are limited to 107 bytes (`sun_path`); a host whose run directory makes the path longer refuses to serve, with an error naming the path, and never falls back to another transport silently. Windows and macOS are out of scope in phase 1; a later version may add a named pipe or TCP loopback behind the same framing, which is transport-agnostic (as IF-4's frames are, ADR-0023). |
| Lifetime | The host creates the socket file before its first append and removes it when it exits normally. A stale socket file (a crashed host) is removed by the next host that owns the run directory. A viewer that cannot connect reads the sealed segments only, which is `rh-cli tail` without `--follow`. |
| Connections | Any number of viewers, each on its own connection; the host may cap them (`too_many_viewers`, §4.4). A connection carries at most **one subscription** at a time. A viewer that wants both channels opens two connections, one per socket. |
| Direction | The viewer sends control frames. The host sends control frames and records frames. Nothing the viewer sends reaches the engine thread: the serving code runs on its own threads, in the host process, outside the engine crates' plane-A code (§7). |
| Byte order | Little-endian everywhere, as IF-4. |
| Reliability | A Unix stream socket is ordered and lossless, so frames carry no sequence number or checksum of their own; `seq` inside the records is the only ordering that matters, and the magic in every header catches a framing desync. |

## 2. Framing

Every message is one **frame**: a fixed 12-byte header, then `length` bytes of payload.

```text
offset  size  field     value
0       4     magic     the ASCII bytes "RHWS" (0x52 0x48 0x57 0x53)
4       2     version   u16 = 1                       (rehoboam/stream/wire/v1)
6       1     type      u8: 1 = CONTROL, 2 = RECORDS
7       1     flags     u8: 0 in v1 (reserved; a nonzero value is refused)
8       4     length    u32: payload length in bytes, 0 ≤ length ≤ 16 777 216 (16 MiB)
12      n     payload   as the type says (§2.1, §2.2)
```

The first six bytes, `magic` and `version`, are **frozen across all versions**: every version of this protocol starts a frame with them, so an implementation of any version can read the version of a frame it does not understand (§5).

A receiver that finds a wrong magic, an unknown version, an unknown type, a nonzero flag, a length over the maximum, or a payload that does not decode as its type (§2.1, §2.2) has a **protocol error**: it stops reading the connection. A host sends one control frame first, an error response with `id` null and `invalid_frame` or `unsupported_version` (§4.4), then closes; a viewer closes and may reconnect. A protocol error is never silent and never resynchronised by scanning for the next magic.

### 2.1 CONTROL (type 1)

The payload is exactly one **JSON-RPC 2.0 object** (a request, a response, or a notification; never a batch array), encoded as **canonical JSON** (§3.1), UTF-8 without a byte-order mark, without a trailing newline. The methods, parameters, results, notifications and errors are §4.

### 2.2 RECORDS (type 2)

The payload is a 20-byte sub-header, then `count` records in the **canonical text form v0** (`canonical_v0.md`), each line terminated by one LF, in ascending `seq` with no gap:

```text
offset  size  field        value
0       4     timeline_id  u32: the records' timeline (the segment's)
4       8     first_seq    u64: the seq of the first record in this frame
12      4     count        u32 ≥ 1: the number of records
16      1     channel      u8: the ASCII byte of the channel, 0x41 'A' or 0x43 'C'
17      3     reserved     0 0 0
20      …     body         count canonical lines, each followed by LF (0x0A); nothing else
```

Rules:
- The i-th line of the body (from 0) is the record with `seq = first_seq + i`, and its own `tl seq tick` header agrees with the sub-header (the timeline, and that seq). A frame whose body has more or fewer lines than `count`, a line whose seq is not `first_seq + i`, a line of another timeline, a line that is not canonical (the parser of `canonical_v0.md` refuses it), or a record kind of the other channel (a `MATCH` on channel A, anything but `MATCH` on channel C) is a protocol error.
- Within a subscription (§4.2), consecutive frames are contiguous: frame k+1's `first_seq` is frame k's `first_seq + count`. A viewer checks this and treats a gap or an overlap as a protocol error, never as something to skip or dedupe (V-STR-05 is proven, not patched).
- The body is byte for byte what `segment.canonical_digest` hashes (`canonical_v0.md` §4): the concatenation, over a sealed sim-year's records in `seq` order, of the bodies of the frames that carried them is the bytes whose XXH3-128 is that year's `canonical_digest`. A viewer can therefore verify a whole year of live delivery against the sealed digest the host announces (`stream.sealed`, §4.3), which is how V-STR-05 is checked (§9).
- How many records a frame holds is the host's choice (a frame is written when the viewer's connection is ready for one; at MAX a frame may hold thousands of records, at Live a single one). Frame boundaries carry no meaning and are never aligned to anything; a viewer must not assume a sim-year, an instant or a kind ends at a frame boundary. `stream.sealed` marks the year.
- The records' fields are the canonical form's business, not this document's. SPR-020's envelope (roles, anchored snapshots, typed and motive edges) changes the canonical form and its version string; it changes nothing here. The wire carries segment records; it never defines them.

**Why canonical lines and not Arrow IPC.** `05.10` §5 (v1.1) said "Arrow IPC record batches with LZ4 on the live edge". This contract carries the canonical text form instead, for four reasons, and `05.10` §5 and `05.9` IF-0 are amended with it: (1) one spelling already exists, is lossless, has a stdlib Python reference and a Rust parser (SPR-013 T-09), and is the form the digest is defined over, so live delivery is verifiable against the sealed digest line by line; (2) the viewers of P1a parse lines anyway to print them, and the Textual TUI stub (SPR-027) and the Tester's oracles stay standard-library Python, with no `pyarrow`; (3) an event is 30–80 bytes as a line against 20–60 as a Parquet row (`05.10` §5), and a local socket moves hundreds of MB/s, so the 2–3× is not a cost on this transport; at MAX a viewer reads a filtered view and never the raw rain (`05.10` §4); (4) a frame of lines is independent of every Arrow and Parquet version, so a Parquet upgrade changes no wire byte (the reason the canonical form exists, `canonical_v0.md` §0). An Arrow IPC payload can be added as a new frame type in a later version if a viewer ever needs columnar batches; nothing in v1 prevents it. The Arrow container remains the stream at rest (ADR-0031 §4 layer 3).

## 3. Encodings

### 3.1 Canonical JSON (control frames)

A control payload is the one JSON text that these rules produce from its value, so that a golden frame is a golden byte string and two implementations compare frames by `==`:

- Object keys are sorted by their byte values (every key in this document is ASCII, so code-point and byte order agree). No key is repeated.
- No whitespace outside strings: separators are `,` and `:` exactly.
- Numbers are **integers only**, written in decimal with no sign on zero, no leading zero, no fraction and no exponent, within `i64` for ticks and `u64` for seqs, counts and ids. There is no float anywhere on this wire (a `tick` is `i64` minutes, a `seq` is `u64`, a hash is a hex string).
- Strings hold only bytes 0x20–0x7E, with `"` written `\"` and `\` written `\\` and nothing else escaped; a value that needs any other byte is invalid for this wire (every string here is an identifier, an enum token, a version string, a hex digest or a path; a path that is not printable ASCII is reported percent-escaped as the canonical form does, `canonical_v0.md` §2).
- `true`, `false` and `null` as themselves.

In Python this is `json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)` on values that respect the string rule. In Rust, `serde_json` with the default (sorted) map and no `preserve_order` feature produces it when structs declare their fields in byte order, or when values are built as `serde_json::Value`; the conformance test (§7) checks every golden frame byte for byte.

A receiver parses JSON by the grammar, so it accepts a non-canonical spelling from a peer (whitespace, another key order); it does not refuse one. It does refuse a float, a number outside its field's range, a repeated key and a non-ASCII string, as `invalid_params` or `invalid_request`.

### 3.2 Values

| Value | Encoding |
|---|---|
| `seq` | JSON integer, `u64` |
| `tick` | JSON integer, `i64` minutes (`SimTime`) |
| `timeline_id` | JSON integer, `u32` |
| `sim_year` | JSON integer, `i32 ≥ 0` (`floor(tick / minutes_per_year)`) |
| `channel` | `"A"` or `"C"` |
| digest, hash | lowercase hex string: 32 digits for a 16-byte XXH3-128 canonical form, 16 for a `u64` hash |
| `world_seed` | JSON integer, `u64` |
| version strings | the format strings the segments carry (`segment.format`, `canonical_v0.md` §1's `format`), and this wire's `rehoboam/stream/wire/v1` |
| enums (`band`, `state`, `reason`) | lowercase ASCII tokens from the tables below; unknown tokens are refused by a host and ignored by a viewer where §5 says so |

## 4. Control messages

The control plane is JSON-RPC 2.0 (`05.9` IF-3's shape, so that the same methods move to `rh-server`'s WebSocket in P2 unchanged). The viewer's requests carry an `id` (a JSON integer or string); the host answers each with a response carrying the same `id`, in request order (a viewer may pipeline). Notifications carry no `id` and are never answered. The namespace on this socket is `stream.*` and nothing else (§4.5).

### 4.1 `stream.status`

Request: `{"id": 1, "jsonrpc": "2.0", "method": "stream.status"}` (`params` absent or `{}`).

Result: the state of this channel's stream at the moment of the answer.

| Field | Type | Meaning |
|---|---|---|
| `wire` | `{"format": "rehoboam/stream/wire/v1", "version": 1}` | This document |
| `channel` | `"A"` / `"C"` | The channel this socket serves |
| `timeline_id` | `u32` | The one timeline this socket serves (P1a: 0; a fork has its own run directory and sockets, `05.10` §5) |
| `record_format` | string | The segments' `segment.format` (`rehoboam/stream/channel-a/v0` or `…/channel-c/v0`): which DDL the records follow |
| `canonical_format` | string | The lines' format (`rehoboam/stream/canonical/v0`): which grammar parses the bodies |
| `history` | `{"engine_build", "registry_version", "manifest_hash", "world_seed"}` | The history tuple the segments carry (`channel_a_v0.sql` `segment`); strings, a 32-hex digest, a `u64`. A viewer displays it and uses it to refuse mixing runs |
| `calendar` | `{"minutes_per_year": int}` | The world calendar's year (`05.1` §4.1), so a viewer maps `tick` to `sim_year` as the segments do |
| `tip` | `{"tick": i64 \| null, "next_seq": u64}` | The last instant whose records are written (null before the first), and the seq the next record will get (= the number of records written so far). Channel A's `tip.tick` is the engine's tip (`05.10` §2); channel C's is the closing tick of the last match written |
| `pace` | `{"band": token, "state": token}` \| `null` | Channel A: the engine's pace band, one of `live`, `1h`, `1d`, `1w`, `1mo`, `1y`, `10y`, `max` (`05.1` §4.4), and its state, `running`, `paused` or `finished`. The P1a host runs at `max`; the pace command is SPR-027's and lives on the control endpoint, never here. Channel C: `null` (the sifter has no pace) |
| `sealed` | `{"last_year": int \| null, "last_seq": u64 \| null}` | Every record with `seq ≤ last_seq` is in a sealed segment on disk (`channel_a_v0.sql` R5, `channel_c_v0.sql` R7), in a directory whose `sim_year ≤ last_year`. Both null before the first seal |
| `live` | `{"first_seq": u64, "next_seq": u64}` | The host's in-memory window: the records with `first_seq ≤ seq < next_seq` can be served live now (§6). `next_seq` equals `tip.next_seq` |
| `sifted_through` | `{"a_seq": u64 \| null, "a_year": int \| null}` \| `null` | Channel C only: the last channel-A seq and sealed sim-year the sifter has processed, so a viewer can tell "no match yet" from "not sifted yet". Channel A: `null` |

Invariants a viewer may rely on and the reference checks: `sealed.last_seq + 1 ≥ live.first_seq` when both exist, and `live.first_seq = 0` before the first seal (§6, I1); `live.first_seq ≤ live.next_seq`; `tip.next_seq = live.next_seq`.

### 4.2 `stream.subscribe` and `stream.unsubscribe`

Request: `{"id": 2, "jsonrpc": "2.0", "method": "stream.subscribe", "params": {"from": {"seq": 1200}}}` or `"from": "live"`.

- `{"seq": n}` asks for every record from `n` on, in order, with no gap.
- `"live"` asks for records from the tip on: `n = live.next_seq` at the moment of the answer. It is "jump to live" (`05.10` §5); what the viewer skips is in the sealed segments and the window, and the result says where live began.

Result: `{"first_seq": n}`, the seq of the first record the viewer will receive. From the response on, RECORDS frames follow on this connection, contiguous from `n`, until the subscription ends by `stream.unsubscribe`, `stream.behind`, `stream.end` or the connection closing.

Errors (§4.4): `behind_live_edge` if `n < live.first_seq` (the catch-up case, §6); `future_seq` if `n > live.next_seq`; `already_subscribed` if this connection holds a subscription; `invalid_params` otherwise (a float, a negative number, both forms, neither, a filter: v1 has **no server-side filter**; the `filter` of `05.9` IF-0's sketch is viewer-side in P1a, in `rh-cli tail` (SPR-027), and a host does no work per viewer but copying bytes).

`stream.unsubscribe` (no params) ends the subscription. Result: `{"next_seq": m}`, the seq the viewer would have received next; after the response no RECORDS frame follows until the next `stream.subscribe`, so "jump to live" is `stream.unsubscribe`, then `stream.subscribe` with `"live"`. Error: `not_subscribed`.

### 4.3 Notifications (host → viewer)

Sent only to a subscribed connection, in-band with its RECORDS frames.

| Method | Params | When |
|---|---|---|
| `stream.sealed` | `{"channel", "timeline_id", "sim_year", "first_seq", "last_seq", "canonical_digest"}` | A segment of this channel was sealed: its sim-year, its seq range and its `canonical_digest` (32 hex). Sent after the frame that carries `last_seq` (so the viewer holds every line of the year); its position relative to records of later seqs is not fixed, because sealing year *y* and writing year *y + 1*'s first records overlap. A viewer subscribed from mid-year gets the notification for that year and may verify only the part it received; a viewer that joined later than a year's seal never gets that year's notification and reads the digest from the segment row |
| `stream.behind` | `{"channel", "timeline_id", "next_seq", "sealed_last_seq", "sealed_last_year", "live_first_seq"}` | The subscription has ended because the viewer's next seq left the window (§6): `next_seq < live_first_seq`. Every record `next_seq ≤ seq ≤ sealed_last_seq` is sealed on disk. The viewer reads them there and subscribes again from `sealed_last_seq + 1` (§6.3). No RECORDS frame follows on this subscription |
| `stream.end` | `{"channel", "timeline_id", "next_seq"}` | The writer has finished (the run ended; the sifter sifted the last sealed year and exited its loop): no record will follow and the last segment is sealed. The subscription has ended; the connection stays open for `stream.status` |

A viewer ignores a notification whose method it does not know (§5).

### 4.4 Errors

A response's `error` is `{"code": int, "message": string, "data": {"reason": token, …}}`; `data.reason` is the machine-readable name, `message` is for people. Codes follow IF-3's ranges: the JSON-RPC standard codes, then −32100 to −32199 for validation.

| Code | `reason` | Extra `data` fields | Meaning |
|---|---|---|---|
| −32700 | `parse_error` | — | The payload is not JSON |
| −32600 | `invalid_request` | — | Not a JSON-RPC 2.0 request object (a batch, no `method`, a bad `id`, `jsonrpc ≠ "2.0"`) |
| −32601 | `method_not_found` | — | A method outside `stream.status`, `stream.subscribe`, `stream.unsubscribe`. **Every method of another namespace (`player.*`, `oracle.*`, `timeline.*`, `console.*`, `world.*`, …, and any `stream.*` method of a later version) gets this code and the message "this socket carries the stream only and accepts no input; inputs go through the input journal"**: it is the socket's one answer to an attempt to act (§4.5) |
| −32602 | `invalid_params` | `"detail"` string | A parameter is missing, of the wrong type, a float, out of range, or unknown (`filter`) |
| −32110 | `unsupported_version` | `"supported": [1]` | The frame's `version` is not one the host speaks. Sent as a response with `id` null, at version 1, then the host closes (§5) |
| −32111 | `invalid_frame` | `"detail"` string | A bad magic, type, flag, length or sub-header, or a non-canonical record line in a frame the host received. `id` null, then close |
| −32112 | `behind_live_edge` | `"sealed_last_seq"`, `"sealed_last_year"`, `"live_first_seq"` | `stream.subscribe` from a seq below the window (§6) |
| −32113 | `future_seq` | `"next_seq"` | `stream.subscribe` from a seq above the tip |
| −32114 | `already_subscribed` | — | This connection already holds a subscription |
| −32115 | `not_subscribed` | — | `stream.unsubscribe` with no subscription |
| −32116 | `too_many_viewers` | `"limit"` int | The host's connection cap; sent as the first frame on the new connection, `id` null, then close. The cap is the host's (default 64) and is not a back-pressure signal: it bounds threads, not the engine |

### 4.5 What a viewer can never do

- **Send an input.** No method on this socket reaches the journal, the engine or the sifter's state. `method_not_found` is the only answer to `player.act`, an Oracle verb, a fork, a seek, a pace command or anything else. A material input reaches plane A only as a journaled input through IF-2's path (ADR-0024 rule 2; ADR-0025 §5; ADR-0030 §4): in P1a `rh-cli run --inputs` journals it before the run; in P2 the control endpoint (§8) validates and journals it at the tip. Replay never needs this socket: history is `f(engine_build, registry_version, manifest_hash, seed, input_journal)` and a stream viewer is in none of those.
- **Change the stream.** Reading never writes: the host's serving threads hold the window read-only, and `rh-stream`'s writer is the only writer of channel A, the sifter the only writer of channel C (`05.10` §7 item 1). Two viewers at different cursors read the same bytes; the reference checks it (§7).
- **Slow the engine.** A connection that never reads blocks its own serving thread on its own socket and nothing else; the window moves on, and when the stalled viewer's next seq leaves it, the host owes it one `stream.behind` and nothing more. The engine's append has no path to a socket (§7 lints) and the host buffers nothing per viewer beyond the frame being written (§6.2).
- **Know anything plane B or C knows, on channel A.** Channel A carries plane-A records only (`channel_a_v0.sql` R1). Channel C is a separate socket from a separate process.

## 5. Versioning

- This is `rehoboam/stream/wire/v1`: frame header version 1, the messages of §4, the error codes of §4.4. `stream.status` reports it (`wire`).
- **Unknown version.** A host that reads a frame whose `version` it does not speak answers with one control frame at the lowest version it speaks (v1), an `unsupported_version` error with `id` null and `data.supported`, then closes. A viewer that reads a frame of a version it does not speak closes. Neither guesses. Because `magic` and `version` are frozen (§2), a v1 viewer talking to a v3 host learns that from the host's first frame.
- **Additive within v1** (minor, `05.9` IF-9): a host may add a field to a result or a notification, and a new notification method, in a later minor; a viewer ignores result fields and notifications it does not know. A host never ignores: an unknown request parameter is `invalid_params`, an unknown method `method_not_found`. The minor version is not on the wire; the fields' presence is.
- **Breaking** (major): a change to the header, the sub-header, a field's type or meaning, an error code, or the removal of anything, is v2 with a migration note in `05.9`.
- The record lines inside a RECORDS frame are versioned by `stream.status.canonical_format` and `record_format`, independently of the wire: SPR-020's envelope bumps those and not this.

## 6. The live-edge rule

### 6.1 State

Per channel and timeline, the host keeps:

- **The sealed set** `S = [0, s]` on disk: every record with `seq ≤ s` is in a sealed segment (`s = sealed.last_seq`; `S` is empty before the first seal). A sealed segment is never modified (`channel_a_v0.sql` R5: atomic sealing, `segment.parquet` last).
- **The window** `W = [w0, w1)` in memory: `w0 = live.first_seq`, `w1 = live.next_seq`. It holds the records of the unsealed segment (the current sim-year) and **retained** sealed segments behind it (a host setting, `retain_sealed_years`, default **1**: the last sealed year stays in memory until the next seal, so a live viewer that is a few records behind at the moment of a seal is not sent to disk once per sim-year; at MAX on a small world a year seals every few wall-seconds). A retained year is dropped only after it is sealed, so retention never delays a seal, and `w0` moves only at a seal. The window is the host's whole stream memory; with the default it is two sim-years of records at most, which is the bound V-PERF-09 measures (PERF-5's "the in-memory live-edge segment" is read as this window; the Architect message carries the D-item that makes the wording say so). `retain_sealed_years = 0` is allowed and keeps the bound at one segment; the reference simulates both.
- **The tip** `w1`: the next seq. Every record ever written has `seq < w1`.

Invariants, held at every instant a viewer can observe (every `stream.status` answer and every frame):

- **I1 (no gap between disk and memory):** `s + 1 ≥ w0`; before the first seal `w0 = 0`. Every `seq < w1` is in `S` or in `W` (or both).
- **I2 (contiguity):** seqs are assigned contiguously from 0 by the writer (`channel_a_v0.sql` R3); `w0 ≤ w1`; `W` has no hole.
- **I3 (immutability):** a seq's record never changes, in memory or on disk; a record in both places is the same line.
- **I4 (monotone):** `s`, `w0` and `w1` never decrease. `w0` moves only when a sealed year is dropped from memory, which happens only after its seal is complete on disk (so I1 is kept through the seal: the year exists on disk before it leaves memory).

### 6.2 The host's side

- **Appending never waits.** The writer appends a record to `W` and advances `w1`; this is the only thing on the engine host's tick path. No socket, no viewer state, no lock held across a socket write.
- **Serving reads `W` at the viewer's cursor.** A subscribed connection holds a cursor `c` (its next seq). The serving thread copies the lines `[c, min(w1, c + batch))` into one RECORDS frame, writes it to the socket (blocking only itself), advances `c`, and repeats. When `c = w1` it waits for the writer's next append (a condition variable or a short sleep; nothing the writer waits on).
- **Leaving the window.** Before copying, the thread checks `c ≥ w0`. If `c < w0` (the viewer was slower than the window: the host sealed and dropped the year `c` is in), it sends `stream.behind` with `next_seq = c`, `sealed_last_seq = s`, `live_first_seq = w0`, ends the subscription, and sends no more records. By I1 every seq in `[c, s]` is on disk, and `s + 1 ≥ w0`, so the viewer can resume at `s + 1` without a gap.
- **A seal** sends `stream.sealed` to every subscription whose cursor has passed the year's `last_seq` (and to those still inside the year, after the frame that carries `last_seq`).
- **Bounded per viewer.** The host holds, per connection, its cursor, the sealed notifications it owes, and the one frame being written. It never queues records per viewer, so a stalled viewer costs one thread and one frame, and the window's size is the host's only stream memory (V-PERF-09's RSS bound).
- **A `stream.subscribe` from `n`** is answered by the same check: `n < w0` is `behind_live_edge` (with `s` and `w0`), `w0 ≤ n ≤ w1` is accepted with `first_seq = n` (`n = w1` means "wait for the next record"), `n > w1` is `future_seq`.

### 6.3 The viewer's side: catch-up with no gap and no duplicate

A viewer keeps one number, its cursor `c` (the next seq it needs), and runs this loop (`rh-protocol`'s `Catchup` state machine, §7; the reference's `Viewer`):

```text
loop:
  r = subscribe(from = {seq: c})
  if r is behind_live_edge(sealed_last_seq = s, live_first_seq = w0):
      assert s + 1 >= w0                      # I1, else protocol error
      read sealed segments for seqs [c, s]    # the chronicle on disk, in seq order
      c = s + 1
      continue                                # the window may have moved again; loop
  if r is first_seq = c:
      while frames arrive:
          take the frame's records; check first_seq == c; c += count
      on stream.behind(next_seq = c, sealed_last_seq = s): read sealed [c, s]; c = s + 1; continue
      on stream.end: done
  on a closed connection: reconnect and continue (c is the viewer's; nothing is lost)
```

Why it is gap-free: every seq the viewer skips over is read from disk, and I1 guarantees the disk reaches the window (`s + 1 ≥ w0`), so `subscribe(from = s + 1)` is never `behind_live_edge` unless the window moved again, in which case the loop reads the newly sealed years. Why it is duplicate-free: the viewer resumes exactly at `c`, the host serves from exactly `c` (I2, I3), and a sealed read stops at `s` exactly; nothing is delivered twice because nothing is ever requested twice. Why it terminates: the window moves only at a seal, once per sim-year, and reading a sealed year from disk is faster than simulating one, so a viewer that is faster than the simulation catches up, and one that is slower keeps reading sealed years for good, which is the "40 years behind at 1 yr/s" cursor of `05.10` §1. The reference simulates slow, fast, stalled, reconnecting and jumping viewers over a writer with random seal timing and checks every delivery is exactly `0, 1, 2, …` with per-year digests equal to the sealed ones (§7).

"Jump to live" is `subscribe(from = "live")`: the viewer sets `c = first_seq` from the answer, accepting that `[old c, first_seq)` is skipped on purpose and sits in the sealed segments and the window for later.

### 6.4 Channel C live

Channel C's socket is served by the sifter process (SPR-013 ruling 14; `rh-cli sift <run-dir> --follow --socket` in SPR-026), from its own window, with the same frames, messages and rule. "Live" on channel C means **match records as the sifter writes them**: in P1a the sifter is downstream of sealed channel-A segments, so channel C's edge trails channel A's by up to one sim-year plus the sift time, and `sifted_through` says how far it is. The engine process never serves channel C, never reads it and never knows whether a sifter runs (ADR-0024; V-PLN-04). A later in-process sifter (P2.1) changes when matches are written, not the wire.

## 7. `rh-protocol`: the crate SPR-026 builds

| Item | Ruling |
|---|---|
| Name, place | `crates/rh-protocol` (`05.0` §6: "wire formats (control schemas, binary frames)") |
| Labels | `[package.metadata.rehoboam] plane = "tool", kind = "tool"`, **pinned by name** in `bin/lint_arch.py` `PINNED_LABELS`, so it cannot be relabelled into plane A |
| Engine crate | Yes: it runs in the engine host's process, so it joins `ENGINE_CRATES` (V-INSP-05) and may not reach the media, HTTP-UI and LLM deny-list; it has no network stack of its own (no `tokio`, no `hyper`): it is bytes in, bytes out |
| Dependencies | **No workspace crate, of any dependency kind** (normal, build, dev): rule `protocol-leaf` of `bin/lint_arch.py` refuses an edge from `rh-protocol` to any member. Third-party: `serde` (derives) and `serde_json` (the canonical JSON; already in `rh-cli`), pinned exactly; `proptest` as a dev-dependency for the round trip (GOV-002 tier 3). Nothing else |
| What the leaf rule buys | A viewer that depends on `rh-protocol` gets no path to `rh-core`, `rh-stream` or `rh-snapshot`: the crate that spells the wire cannot write a record, open a journal or hold an engine. Conversely the plane-A closure (`bin/lint_arch.py`) forbids every plane-A crate from depending on a tool crate, so `rh-stream` and the engine cannot even name a frame: the engine's output is lines in memory, and the wire is applied to them by the host, outside plane A |
| Modules | `wire`: `Header`, `FrameType`, `MAX_PAYLOAD`, `encode(frame) -> Vec<u8>`, `decode(&[u8]) -> Result<(Frame, usize), WireError>` (returns the bytes consumed; a short buffer is `Incomplete`, so a reader can accumulate). `records`: `RecordsFrame { timeline_id, channel, first_seq, lines: Vec<String> }` with the §2.2 checks (the line check uses `rh-stream`'s parser **in the host and the viewer**, not here: `rh-protocol` checks the sub-header, the count, each line's `tl seq` prefix and ASCII-ness, and leaves the grammar to the caller, which keeps the leaf a leaf). `control`: `Request`, `Response`, `Notification`, `Method` (the three), `SubscribeFrom`, `Status`, `Sealed`, `Behind`, `End`, `RpcError`, `ErrorCode`, `Reason`, each `Serialize`/`Deserialize` in canonical field order. `cursor`: `Catchup`, the pure state machine of §6.3: `fn next(&mut self, event: Event) -> Action` with `Event::{Subscribed(first_seq), Behind{..}, Records{first_seq, count}, Sealed{..}, End, Closed, SealedRead{first_seq, count}}` (the last reports what the sealed read returned; it must cover exactly `[cursor, sealed_last_seq]`) and `Action::{Subscribe(seq), ReadSealed{from, to}, Deliver{first_seq, count}, Ignore, Done(next_seq), ProtocolError(detail)}`, the transcript of the golden file's `scenario catchup` block reproduced line for line, so `rh-cli tail --follow`, the TUI stub and the Tester's harness share one implementation of the loop |
| Host side (not in `rh-protocol`) | The Unix-socket server: accept, one serving thread per connection, the window reader. SPR-026 puts it in the tool-plane host that already runs `rh-stream`'s writer, `rh-cli run … --socket` (`crates/rh-cli/src/serve.rs`, std `UnixListener`, no async runtime), and the same module in `rh-cli sift … --follow --socket` for channel C. `rh-server` (P2, `05.0` §6) absorbs it when the WebSocket control plane arrives. `rh-stream` (plane A) exposes the window as a read-only, shared, append-only `LiveEdge` (`first_seq`, `next_seq`, `lines(from, max)`), built by the writer after each instant; the server reads it and never writes it |
| Lints | `bin/lint_arch.py`: `PINNED_LABELS["rh-protocol"] = ("tool", "tool")`; `ENGINE_CRATES` gains `rh-protocol`; rule `protocol-leaf` (fixture `arch/protocol_depends_on_plane_a`); the closure already refuses a plane-A crate depending on it (fixture `arch/plane_a_to_protocol`). `bin/lint_forbidden.py`: rule `network` (V-DET-19): `std::net`, `std::os::unix::net`, `UnixListener`, `UnixStream`, `TcpListener`, `TcpStream` are forbidden in plane-A `src/` (planted control `forbidden/planted/net_socket.rs`), so no plane-A crate, `rh-stream` included, can open or serve a socket: the engine's append has no path to the wire by construction, which is "the engine never waits" as a lint, not a promise |
| Conformance | `crates/rh-protocol/tests/golden.rs` reads `specs/stream/golden/wire_v1.txt`, encodes every `frame` from its decoded description and compares the bytes, decodes every frame back, and refuses every `refuse` line with the named reason; a proptest round trip over random frames and control messages; `Catchup` driven by the reference's scenarios (its `scenario` lines, seqs in, actions out) |

## 8. Reserved: the control endpoint (SPR-027, P2)

`<run-dir>/stream/control.sock` is reserved for the control plane of IF-0 (`05.10` §7a: "a control and stream endpoint"): the pace command (SPR-027; STR-5), and in P2 tip and scheduled inputs and forks (STR-4; `05.9` IF-3's `timeline.*`, `player.*`, `oracle.*` over the same framing and JSON-RPC carrier as §2.1 and §4). It is a different socket from the stream's so that the read-only property of §4.5 is a property of a file, not of a method table: a viewer given only `A.sock` and `C.sock` can act on nothing. Its methods are SPR-027's and P2's to specify; this document fixes only that they use this framing (§2), the canonical JSON (§3.1), the error model (§4.4) and the version rule (§5), and that nothing they do is journaled unless IF-2 says so (the pace command is run-log only).

## 9. Acceptance: V-STR rows to probes (SPR-026)

| Test (`docs/09`) | Probe in SPR-026 | Pass |
|---|---|---|
| V-STR-01 | `rh-cli run solo` and `hamlet` headless, then with four viewers attached (`tail --follow` at the edge, one at a scripted 100× playback, one rewinding by re-subscribing from old seqs, one stalled: connected, never reads); compare every sealed segment's `canonical_digest` and the state ladder | Equal, byte for byte; the ladder equals the baseline |
| V-STR-03 | The same run: viewers at 1×, 100×, rewinding and stalled during a MAX run | Channel A unchanged (V-STR-01); throughput per V-PERF-09 |
| V-STR-05 | A viewer that deliberately sleeps past a seal (it subscribes, sleeps two sim-years of wall time at MAX, resumes): it gets `stream.behind`, reads the sealed years, subscribes again, and continues to `stream.end`; the viewer's per-year XXH3-128 over the bodies it received equals every `stream.sealed.canonical_digest` and every `segment.canonical_digest` on disk; its seqs are `0..next_seq` exactly. Then "jump to live" from a cursor 10 years behind: `first_seq = live.next_seq` at the answer, nothing duplicated after it | Contiguous, no gap, no duplicate; digests equal; `Catchup` reports no protocol error |
| V-PERF-09 (P1a variant) | On `adjutant` (RUN-002): the 10k dynastic world headless, then with `rh-cli tail --follow`, the Textual TUI stub and one stalled consumer; wall time, peak RSS and the ladder | Throughput ≥ 95%; RSS growth ≤ the window (the unsealed segment and the last sealed one, `retain_sealed_years = 1`; measured as the sum of the two segments' line bytes); the ladder identical; the stalled consumer, resumed after the run, reads everything from sealed segments with nothing missing |
| "The engine never waits" | (a) `bin/lint_forbidden.py` rule `network` is green on the workspace and red on its planted control; `bin/lint_arch.py` `protocol-leaf` and the plane-A closure are green and red on their fixtures: no plane-A crate can reach a socket or a frame. (b) A unit test of the host: a connection whose socket is never read; the writer appends 10 sim-years; the host's stream memory stays the window (measured by `LiveEdge` size), the writer's append never blocks (its per-append wall time is sampled and bounded by the no-viewer run's maximum), and the stalled connection's first read after the run is `stream.behind`. (c) `cargo nextest run -p rh-snapshot --test observer`-style: the host's `state_hash` ladder with and without viewers is one ladder | All green |
| V-IF-02-style | Corrupt frames fed to the host (a fuzz seeded from the reference's `refuse` lines): wrong magic, version 2, type 3, flag 1, length over the maximum, a truncated payload, a records frame from a viewer, a non-canonical line, a JSON batch, a float seq | The host answers `invalid_frame` or `unsupported_version` and closes the connection; the run continues, the ladder is unchanged, no panic |
| Read-only | `player.act`, `timeline.fork`, `timeline.run`, `oracle.whisper`, `stream.pace` sent on `A.sock` | `method_not_found` with the read-only message; the journal is unchanged (row count and chain head); the ladder is unchanged |
| Channel C live | `rh-cli sift --follow --socket` beside the run; a viewer on `C.sock` from seq 0 to `stream.end` | Its lines equal `rh-cli tail --canonical` of the sealed channel-C segments; `sifted_through` never exceeds channel A's `sealed.last_seq`; the state ladder is unchanged (V-PLN-04) |
| Conformance | `crates/rh-protocol/tests/golden.rs` against `specs/stream/golden/wire_v1.txt` | Every golden frame reproduced and decoded; every `refuse` refused |

## 10. Gates

- `python -I specs/stream/selftest_wire_v1.py --check`: the self-test (frame round trips, every message encoded and decoded, the refusals, the live-edge simulation under several seeds and viewer mixes), then the golden file compared with the reference's rendering. `bin/check_all.sh --part core` runs it, plus two positive controls: a stale golden frame must FAIL with `STALE:`, and `--control window-gap`, a host that drops a year from memory before its seal is on disk (I1 broken), must FAIL the live-edge check.
- `python -I specs/stream/selftest_wire_v1.py` rewrites the golden file after the self-test.
- `bin/lint_arch.py` and `bin/lint_forbidden.py` with their new fixtures (§7), in the same part.
