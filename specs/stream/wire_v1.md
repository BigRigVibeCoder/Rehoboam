# World Stream wire format v1 (IF-0 live edge) and `rh-protocol`

*Status: v1 draft, round 3 (REV-SPR-026 R1–R24 applied), a C2 contract for P1.17 (SPR-026, SPR-027). The director approved its two open parts on 2026-10-17: canonical lines on the live edge (§2.2, D43, amending ADR-0031 §1 item 4) and the window bound of option (b) (§6.1, D42, amending PERF-5 and V-PERF-09). Reference: `specs/stream/selftest_wire_v1.py` (frame codec, control messages, the host's dispatcher, the `Catchup` state machine, the live-edge rule simulated with fast, slow, crawling, stalled, reconnecting, jumping and late viewers over sealed and live segments, a seal-race check, golden frames); golden file: `specs/stream/golden/wire_v1.txt`. Records: `specs/stream/channel_a_v0.sql`, `channel_c_v0.sql`; their one-line form: `specs/stream/canonical_v0.md`. Contracts it implements: `docs/05_architecture/05.10_world_stream.md` §5 and §7; `05.9` IF-0 and the harness rules; ADR-0024, ADR-0025, ADR-0030, ADR-0031, ADR-0035 §6. Tests: `docs/09` V-STR-01, V-STR-03, V-STR-05, V-PERF-09 (§9 below). A change to a byte of the frame header, a control message's field or an error code is a new version (`rehoboam/stream/wire/v2`), announced like a Registry change (`05.9` IF-9).*

*Architect text rulings, 2026-10-17, at SPR-026's plan: the viewer's live mode is `rh-cli tail --live`, because SPR-013 and `05.10` use `--follow <person>` as a person filter; `sift --follow` keeps its name. Two sentences about SPR-020 are updated now that PR #61 is merged (§2.2, §5). No frame, message or golden line changes.*

## 0. What this is

The engine runs headless and writes the World Stream: channel A (the world, plane A) is written by the engine host, channel C (sift matches, plane C) by the sifter. Sealed segments are Parquet on disk, one per sim-year per channel (SPR-013): **the chronicle is the stream at rest**. This document is the rest of IF-0: how a viewer reads the **live edge**, the records of the newest sim-years that are still in the host's memory, over a local socket, and how a viewer that falls behind that window goes to the sealed segments and comes back with no gap and no duplicate, by `seq`.

Three facts shape the design and are not negotiable here (they are the ADRs'):

1. **The engine never waits for a reader** (`05.10` §7 item 3; `05.9` harness rules; PERF-5). Appending is a memory write on the engine host's thread. No socket write is on the tick path, the writer takes no lock a reader can hold (§6.2), nothing back-pressures the engine, and nothing is dropped: a reader that cannot keep up is sent to the sealed segments.
2. **A viewer is never an input to plane A** (ADR-0024 rule 2; ADR-0030). The stream sockets are read-only for the world, in every version (§4.5). They carry no material input and no pace command. Inputs go through the input journal (`05.9` IF-2; `rh-snapshot` `journal.rs`; `rh-cli run --inputs` in P1a; the control endpoint of §8 in P2). The pace command is a control command on the control endpoint (SPR-027), logged in the run log, never on a stream socket and never in the journal (STR-5).
3. **Pace never changes what happens** (`05.1` §4.4). The wire therefore carries no information that could feed back: a viewer's cursor, its rate and its existence are unknown to the simulation.

IF-0 is three sockets in the run directory: `A.sock` and `C.sock` (this document) and `control.sock` (§8; SPR-027, P2). That is the "one local control and stream endpoint" of ADR-0035 §6 and `05.10` §7a, spelled as files.

## 1. Transport

| Item | v1 |
|---|---|
| Socket | A **Unix domain stream socket** (`SOCK_STREAM`), one path per channel and timeline, under the timeline's stream directory (`channel_a_v0.sql` R5): `<run-dir>/stream/t<timeline_id, 10 digits>/A.sock`, served by the engine host (the process that runs `rh-stream`'s writer), and `<run-dir>/stream/t<timeline_id>/C.sock`, served by the sifter process (`rh-cli sift … --follow --socket`; SPR-013 ruling 14). A process serves the channels it writes and no other: one writer per channel (`05.10` §7 item 1) means one server per channel. |
| Platform | Linux (D5). Unix socket paths are limited to 107 bytes (`sun_path`); a host whose run directory makes the path longer refuses to serve, with an error naming the path, and never falls back to another transport silently. Windows and macOS are out of scope in phase 1; a later version may add a named pipe or TCP loopback behind the same framing, which is transport-agnostic (as IF-4's frames are, ADR-0023). |
| Modes and ownership | The run directory is created mode `0700`; every socket file is `0600`; both belong to the user running the host. Each server takes an exclusive `flock` on a lock file beside its socket for its lifetime: `A.lock` by the engine host, `C.lock` by the sifter (R20), so that the sifter can serve beside the run, a second server for the same socket cannot start, and nobody unlinks a live socket as "stale". A stale socket file (a crashed server) is removed by the next server that holds that socket's lock. |
| Lifetime | The host creates the socket file before its first append and removes it when it exits normally. A viewer that cannot connect reads the sealed segments only, which is `rh-cli tail` without `--live`. |
| Connections | Any number of viewers, each on its own connection; the host caps them (`too_many_viewers`, §4.4; default 64). A connection carries at most **one subscription** at a time. A viewer that wants both channels opens two connections, one per socket. |
| Direction | The viewer sends CONTROL frames only, of at most 4 KiB (§2). The host sends CONTROL and RECORDS frames. Nothing the viewer sends reaches the engine thread: the serving code runs on its own threads, in the host process, in one module outside the engine crates' plane-A code (§7). |
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
8       4     length    u32: payload length in bytes, 0 ≤ length ≤ 1 048 576 (1 MiB)
12      n     payload   as the type says (§2.1, §2.2)
```

The first six bytes, `magic` and `version`, are **frozen across all versions**: every version of this protocol starts a frame with them, so an implementation of any version can read the version of a frame it does not understand (§5).

Limits: every frame's payload is at most **1 MiB** (so a RECORDS frame and a per-connection buffer are bounded, §6.2; the RSS bound of §9 counts one frame per viewer). A frame **from a viewer** is at most **4 KiB** and is always CONTROL; the host judges the 12 header bytes first and refuses a longer one, a RECORDS type or an unknown version on the header alone, as `invalid_frame` or `unsupported_version`, without reading the payload (it answers, then closes). A control payload nests at most **4** levels deep (an object or array counts one level; the root is level 1).

A receiver that finds a wrong magic, an unknown version, an unknown type, a nonzero flag, a length over its limit, or a payload that does not decode as its type (§2.1, §2.2) has a **protocol error**: it stops reading the connection. A host sends one control frame first, an error response with `id` null and `invalid_frame` or `unsupported_version` (§4.4), then closes; a viewer closes and reconnects by §6.3. A protocol error is never silent and never resynchronised by scanning for the next magic.

### 2.1 CONTROL (type 1)

The payload is exactly one **JSON-RPC 2.0 object** (a request, a response, or a notification; never a batch array), encoded as **canonical JSON** (§3.1), UTF-8 without a byte-order mark, without a trailing newline. The methods, parameters, results, notifications and errors are §4.

### 2.2 RECORDS (type 2)

The payload is a 20-byte sub-header, then `count` records in the **canonical text form** (`canonical_v0.md`), each line terminated by one LF, in ascending `seq` with no gap:

```text
offset  size  field        value
0       4     timeline_id  u32: the records' timeline (the segment's)
4       8     first_seq    u64: the seq of the first record in this frame
12      4     count        u32 ≥ 1: the number of records
16      1     channel      u8: the ASCII byte of the channel, 0x41 'A' or 0x43 'C'
17      3     reserved     0 0 0
20      …     body         count canonical lines, each followed by LF (0x0A); nothing else
```

**What the wire reads of a line, and nothing more.** Every canonical line of every canonical version begins with the frozen prefix `<KIND> <timeline_id> <seq> <tick> ` (the kind name, then the three header ints, each followed by one space; `canonical_v0.md` §1 states the prefix as frozen across versions: the sentence is on `main` since PR #61 merged). The wire's checks on a RECORDS body are exactly these:

- the body is printable ASCII (0x20–0x7E) and LF, and ends with LF; it holds exactly `count` lines;
- the i-th line (from 0) begins with a kind the canonical format assigns to the frame's channel (`canonical_v0.py` `CHANNEL`: today every kind but `MATCH` is channel A's and `MATCH` is channel C's; a later canonical version may add kinds to either channel without a wire change), then the frame's `timeline_id`, then `first_seq + i`, then a canonical `i64` tick;
- within a subscription (§4.2), consecutive frames are contiguous: frame k+1's `first_seq` is frame k's `first_seq + count`. A viewer checks this and treats a gap or an overlap as a protocol error, never as something to skip or dedupe (V-STR-05 is proven, not patched).

Anything after the prefix is the canonical format's business: the writer validated the whole line when it rendered it (§6.2), serving threads copy bytes, and a viewer parses as much of a line as it needs (a filter on the kind, the tick or an id splits on spaces; `rh-cli tail --canonical` parses fully). A line that fails a prefix check, a line of another timeline or channel, and a frame whose line count differs from `count` are protocol errors. SPR-020's envelope changes the fields after the prefix and the golden lines; it changes no wire byte and no check here (R6).

The body is byte for byte what `segment.canonical_digest` hashes (`canonical_v0.md` §4): the concatenation, over a sealed sim-year's records in `seq` order, of the bodies of the frames that carried them is the bytes whose XXH3-128 is that year's `canonical_digest`. A viewer can therefore verify a whole year of live delivery against the sealed digest the host announces (`stream.sealed`, §4.3), which is how V-STR-05 is checked (§9).

How many records a frame holds is the host's choice (a frame is written when the viewer's connection is ready for one; at MAX a frame may hold thousands of records, at Live a single one), within the 1 MiB limit. Frame boundaries carry no meaning and are never aligned to anything; a viewer must not assume a sim-year, an instant or a kind ends at a frame boundary. `stream.sealed` marks the year.

**Why canonical lines and not Arrow IPC (D43).** `05.10` §5 (v1.1) and ADR-0031 §1 item 4 say "Arrow IPC with LZ4 on the live edge". This contract carries the canonical text form instead, for four reasons: (1) one spelling already exists, is lossless, has a stdlib Python reference and a Rust parser (SPR-013 T-09), and is the form the digest is defined over, so live delivery is verifiable against the sealed digest line by line; (2) the viewers of P1a parse lines anyway to print them, the Textual TUI stub (SPR-027) and the Tester's oracles stay standard-library Python for the live edge (the sealed-read path is a `rh-cli tail --canonical` subprocess, §6.3), and a viewer that only filters splits a line in about 0.3 µs; (3) bytes are no constraint on a local socket: a 10k world at the G1a pace is 4–14 MB/s of lines, a 1M world about 28 MB/s, against the GB/s a Unix socket carries; (4) a frame of lines is independent of every Arrow and Parquet version, so a Parquet upgrade changes no wire byte (the reason the canonical form exists, `canonical_v0.md` §0). An Arrow IPC payload can be added as a new frame type in a later version if a viewer ever needs columnar batches; nothing in v1 prevents it. The Arrow container remains the stream at rest (ADR-0031 §4 layer 3). Because ADR-0031 §1 item 4 is an accepted ADR, this is the director's decision: D43 in PR #60, recommended for approval; until it is ruled, this section, `05.10` §5 v1.4 and `05.9` IF-0 v1.4 are marked pending.

## 3. Encodings

### 3.1 Canonical JSON (control frames)

A control payload is the one JSON text that these rules produce from its value, so that a golden frame is a golden byte string and two implementations compare frames by `==`:

- Object keys are sorted by their byte values (every key in this document is ASCII, so code-point and byte order agree). No key is repeated.
- No whitespace outside strings: separators are `,` and `:` exactly.
- Numbers are **integers only**, written in decimal with no sign on zero, no leading zero, no fraction and no exponent, within `i64` for ticks and `u64` for seqs, counts, nonces and ids. There is no float anywhere on this wire (a `tick` is `i64` minutes, a `seq` is `u64`, a hash is a hex string).
- Strings hold only bytes 0x20–0x7E, with `"` written `\"` and `\` written `\\` and nothing else escaped; a value that needs any other byte is invalid for this wire (every string here is an identifier, an enum token, a version string, a hex digest or a path; a path that is not printable ASCII is reported percent-escaped as the canonical form does, `canonical_v0.md` §2).
- `true`, `false` and `null` as themselves.
- Nesting depth at most 4 (§2).

In Python this is `json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)` on values that respect the string rule. In Rust, `serde_json` with the default (sorted) map and no `preserve_order` feature produces it when structs declare their fields in byte order, or when values are built as `serde_json::Value`; the conformance test (§7) checks every golden frame byte for byte.

A receiver parses JSON by the grammar, so it accepts a non-canonical spelling from a peer (whitespace, another key order); it does not refuse one. It does refuse: a float, anywhere, as `invalid_params` (R24); and, as `parse_error` or `invalid_request`, a number outside `i64..u64`, a repeated key, a non-ASCII string, a nesting deeper than 4, and an **unknown member** of a JSON-RPC envelope (a request holds exactly `jsonrpc`, `id`, `method` and optionally `params`; a notification `jsonrpc`, `method`, `params`; a response `jsonrpc`, `id` and one of `result` or `error`; an error object `code`, `message`, `data`). A host refuses an unknown parameter as `invalid_params` (§4.2). A viewer, by contrast, **ignores** a field it does not know inside a `result` or a notification's `params` (§5, additive changes), but never one in the envelope.

### 3.2 Values

| Value | Encoding |
|---|---|
| `seq` | JSON integer, `u64` |
| `tick` | JSON integer, `i64` minutes (`SimTime`) |
| `timeline_id` | JSON integer, `u32` |
| `sim_year` | JSON integer, `i32 ≥ 0` (`floor(tick / minutes_per_year)`) |
| `run_nonce` | JSON integer, `u64`: a number the host draws at start from the OS random source or the wall clock in nanoseconds, never a process id alone (pid 1 repeats across restarts in a container) and never in plane A; new on every start of a host, so a viewer can tell a resumed host from the one it was talking to |
| `channel` | `"A"` or `"C"` |
| digest, hash | lowercase hex string: 32 digits for a 16-byte XXH3-128 canonical form, 16 for a `u64` hash |
| `world_seed` | JSON integer, `u64` |
| version strings | the format strings the segments carry (`segment.format`, `canonical_v0.md` §1's `format`), and this wire's `rehoboam/stream/wire/v1` |
| enums (`band`, `state`, `reason`) | lowercase ASCII tokens from the tables below; unknown tokens are refused by a host and ignored by a viewer where §5 says so |

## 4. Control messages

The control plane is JSON-RPC 2.0 (`05.9` IF-3's shape, so that the same methods move to `rh-server`'s WebSocket in P2 unchanged). The viewer's requests carry an `id` (a JSON integer or string); the host answers each with a response carrying the same `id`, in request order (a viewer may pipeline). Notifications carry no `id` and are never answered; a notification or a response **from a viewer** is `invalid_request`. The namespace on a stream socket is `stream.*` and nothing else (§4.5).

### 4.1 `stream.status`

Request: `{"id": 1, "jsonrpc": "2.0", "method": "stream.status"}` (`params` absent or `{}`; any member in it is `invalid_params`).

Result: the state of this channel's stream, read from **one snapshot** (§6.2) at the moment of the answer.

| Field | Type | Meaning |
|---|---|---|
| `wire` | `{"format": "rehoboam/stream/wire/v1", "version": 1}` | This document |
| `channel` | `"A"` / `"C"` | The channel this socket serves |
| `timeline_id` | `u32` | The one timeline this socket serves (P1a: 0; a fork has its own directory and sockets, `05.10` §5) |
| `run_nonce` | `u64` | This host's start (§3.2). A viewer that reconnects compares it with the one it saw: a different nonce with the same `history` is a resumed host, whose tip may be below the viewer's cursor until the resumed run has re-emitted the lost records (§6.2), so the viewer retries `future_seq` with backoff and, when that year's `stream.sealed` arrives, verifies what it had received of the year against the announced digest (§6.3, R21); a different `history` is another run, and the viewer stops with an error |
| `record_format` | string | The segments' `segment.format` (`rehoboam/stream/channel-a/v0` or `…/channel-c/v0`): which DDL the records follow |
| `canonical_format` | string | The lines' format (`rehoboam/stream/canonical/v0`): which grammar parses the bodies |
| `history` | `{"engine_build", "registry_version", "manifest_hash", "world_seed"}` | The history tuple the segments carry (`channel_a_v0.sql` `segment`); strings, a 32-hex digest, a `u64`. A viewer displays it and refuses to mix runs |
| `calendar` | `{"minutes_per_year": int}` | The world calendar's year (`05.1` §4.1), so a viewer maps `tick` to `sim_year` as the segments do |
| `tip` | `{"tick": i64 \| null, "next_seq": u64}` | The last instant whose records are written (null before the first), and the seq the next record will get (= the number of records written so far). Channel A's `tip.tick` is the engine's tip (`05.10` §2); channel C's is the closing tick of the last match written |
| `pace` | `{"band": token, "state": token}` \| `null` | Channel A: the engine's pace band, one of `live`, `1h`, `1d`, `1w`, `1mo`, `1y`, `10y`, `max` (`05.1` §4.4), and its state, `running`, `paused` or `finished`. The P1a host runs at `max`; the pace command is SPR-027's and lives on the control endpoint, never here. Channel C: `null` (the sifter has no pace) |
| `sealed` | `{"last_year": int \| null, "last_seq": u64 \| null}` | `last_year`: the newest sealed sim-year (empty or not), null before the first seal. `last_seq`: the last seq of the newest **non-empty** sealed segment, null while none exists; every record with `seq ≤ last_seq` is in a sealed segment on disk (`channel_a_v0.sql` R5, `channel_c_v0.sql` R7). Channel C seals a sim-year with no match (`record_count = 0`, R7), so `last_year` may run ahead of the year that holds `last_seq` |
| `live` | `{"first_seq": u64, "next_seq": u64}` | The host's in-memory window: the records with `first_seq ≤ seq < next_seq` can be served live now (§6). `next_seq` equals `tip.next_seq`. Both from the same snapshot as `sealed` |
| `sifted_through` | `{"a_seq": u64 \| null, "a_year": int \| null}` \| `null` | Channel C only: the last channel-A seq and sealed sim-year the sifter has processed, so a viewer can tell "no match yet" from "not sifted yet". Channel A: `null` |

Invariants a viewer may rely on and the reference checks: `sealed.last_seq + 1 ≥ live.first_seq` when `last_seq` is not null, and `live.first_seq = 0` while it is null (§6, I1); `live.first_seq ≤ live.next_seq`; `tip.next_seq = live.next_seq`.

### 4.2 `stream.subscribe` and `stream.unsubscribe`

Request: `{"id": 2, "jsonrpc": "2.0", "method": "stream.subscribe", "params": {"from": {"seq": 1200}}}` or `"from": "live"`.

- `{"seq": n}` asks for every record from `n` on, in order, with no gap.
- `"live"` asks for records from the tip on: `n = live.next_seq` at the moment of the answer. It is "jump to live" (`05.10` §5); what the viewer skips is in the sealed segments and the window, and the result says where live began.

Result: `{"first_seq": n}`, the seq of the first record the viewer will receive. From the response on, RECORDS frames follow on this connection, contiguous from `n`, until the subscription ends by `stream.unsubscribe`, `stream.behind`, `stream.end` or the connection closing. The answer is evaluated against one snapshot (§6.2).

Errors (§4.4): `behind_live_edge` if `n < live.first_seq` (the catch-up case, §6); `future_seq` if `n > live.next_seq`, carrying `finished` (retriable while the writer runs: a viewer that reconnected to a resumed host backs off and asks again; terminal when `finished` is true: the run is over and no record will ever reach that seq, so the viewer stops, §6.3); `already_subscribed` if this connection holds a subscription; `invalid_params` otherwise (a float, a negative number, both forms, neither, any other member such as a `filter`: v1 has **no server-side filter**; the `filter` of `05.9` IF-0's earlier sketch is viewer-side in P1a, in `rh-cli tail` (SPR-027), and a host does no work per viewer but copying bytes).

`stream.unsubscribe` (no params) ends the subscription. Result: `{"next_seq": m}`, the seq the viewer would have received next; after the response no RECORDS frame follows until the next `stream.subscribe`, so "jump to live" is `stream.unsubscribe`, then `stream.subscribe` with `"live"` (the `Jump` input of §6.3). Error: `not_subscribed`.

### 4.3 Notifications (host → viewer)

Sent only to a subscribed connection, in-band with its RECORDS frames. The serving thread **pulls** them from the host's shared seal log (§6.2); the writer never pushes anything to a connection.

| Method | Params | When |
|---|---|---|
| `stream.sealed` | `{"channel", "timeline_id", "sim_year", "first_seq", "last_seq", "canonical_digest"}` | A segment of this channel was sealed: its sim-year, its seq range and its `canonical_digest` (32 hex). For a non-empty year it is sent after the frame that carries `last_seq` (so the viewer holds every line of the year); its position relative to records of later seqs is not fixed, because sealing year *y* and writing year *y + 1* overlap. For an **empty** year (channel C, `channel_c_v0.sql` R7) `last_seq` is `null`, `first_seq` is the year's first seq (the seq its first record would have had, which is the next year's first seq), `canonical_digest` is the digest of the empty string, and it is sent at once. A viewer subscribed from mid-year gets the notification for that year and may verify only the part it received; a viewer that joined later than a year's seal never gets that year's notification and reads the digest from the segment row |
| `stream.behind` | `{"channel", "timeline_id", "next_seq", "sealed_last_seq", "sealed_last_year", "live_first_seq"}` | The subscription has ended because the viewer's next seq left the window (§6): `next_seq < live_first_seq`, with `next_seq` the viewer's cursor (a viewer checks it equals its own). Every record `next_seq ≤ seq ≤ sealed_last_seq` is sealed on disk; `sealed_last_seq` and `live_first_seq` come from one snapshot, so `sealed_last_seq + 1 ≥ live_first_seq` (I1). The viewer reads them there and subscribes again from `sealed_last_seq + 1` (§6.3). No RECORDS frame follows on this subscription |
| `stream.end` | `{"channel", "timeline_id", "next_seq", "reason"}` | The writer has stopped: no record will follow. `reason` is `finished` (the run ended normally and the last segment is sealed; channel C: the sifter sifted the last sealed year) or `aborted` (the host stopped on an error: a poisoned engine, BCK-002 A-42; the records up to `next_seq - 1` were written, the unsealed year is **not** sealed, and a resumed run re-emits it). The subscription has ended; the connection stays open for `stream.status` |

A viewer ignores a notification whose method it does not know (§5).

### 4.4 Errors

A response's `error` is `{"code": int, "message": string, "data": {"reason": token, …}}`; `data.reason` is the machine-readable name, `message` is for people. Codes follow IF-3's ranges: the JSON-RPC standard codes, then −32100 to −32199 for validation.

| Code | `reason` | Extra `data` fields | Meaning |
|---|---|---|---|
| −32700 | `parse_error` | `"detail"` string | The payload is not JSON, or exceeds the parser (depth, recursion) |
| −32600 | `invalid_request` | `"detail"` string | Not a JSON-RPC 2.0 request object (a batch, no `method`, a bad `id`, `jsonrpc ≠ "2.0"`, an unknown envelope member, a non-ASCII string anywhere, a notification or response from a viewer) |
| −32601 | `method_not_found` | — | A method outside `stream.status`, `stream.subscribe`, `stream.unsubscribe`. **Every method of another namespace (`player.*`, `oracle.*`, `timeline.*`, `console.*`, `world.*`, …, and any `stream.*` method of a later version) gets this code and the message "this socket carries the stream only and accepts no input; inputs go through the input journal"**: it is the socket's one answer to an attempt to act (§4.5) |
| −32602 | `invalid_params` | `"detail"` string | A parameter is missing, of the wrong type, out of range, or unknown (`filter`, a member of `stream.status`'s params); a float anywhere in the payload |
| −32110 | `unsupported_version` | `"supported": [1]` | The frame's `version` is not one the host speaks. Sent as a response with `id` null, at version 1, then the host closes (§5) |
| −32111 | `invalid_frame` | `"detail"` string | A bad magic, type, flag, length or sub-header, a RECORDS frame from a viewer, a frame from a viewer over 4 KiB, or a line that fails a prefix check in a frame the host received. `id` null, then close |
| −32112 | `behind_live_edge` | `"sealed_last_seq"` (`u64 \| null`), `"sealed_last_year"` (`int \| null`), `"live_first_seq"` | `stream.subscribe` from a seq below the window (§6); the three from one snapshot |
| −32113 | `future_seq` | `"next_seq"`, `"finished"` bool | `stream.subscribe` from a seq above the tip. **Retriable** while `finished` is false (a resumed host re-emitting); **terminal** when true: the writer has stopped and the seq will never exist, so the viewer stops (§6.3, R21) |
| −32114 | `already_subscribed` | — | This connection already holds a subscription |
| −32115 | `not_subscribed` | — | `stream.unsubscribe` with no subscription |
| −32116 | `too_many_viewers` | `"limit"` int | The host's connection cap; sent as the first frame on the new connection, `id` null, then close. The cap is the host's (default 64) and is not a back-pressure signal: it bounds threads, not the engine. **Retriable** (§6.3) |

### 4.5 What a viewer can never do

- **Send an input.** No method on a stream socket reaches the journal, the engine or the sifter's state. `method_not_found` is the only answer to `player.act`, an Oracle verb, a fork, a seek, a pace command or anything else. A material input reaches plane A only as a journaled input through IF-2's path (ADR-0024 rule 2; ADR-0025 §5; ADR-0030 §4): in P1a `rh-cli run --inputs` journals it before the run; in P2 the control endpoint (§8) validates and journals it at the tip. Replay never needs a stream socket: history is `f(engine_build, registry_version, manifest_hash, seed, input_journal)` and a stream viewer is in none of those.
- **An invariant for every version of this wire:** a stream socket (`A.sock`, `C.sock`) accepts exactly the read methods of the `stream.*` namespace, `stream.status`, `stream.subscribe`, `stream.unsubscribe` and whatever a later version adds that **reads**; a method that changes anything, in any namespace, belongs to `control.sock` (§8) and is never added to a stream socket. A conformance test enumerates the method table of every version and fails on any other entry (§7).
- **Change the stream.** Reading never writes: the host's serving threads hold the window read-only, and `rh-stream`'s writer is the only writer of channel A, the sifter the only writer of channel C (`05.10` §7 item 1). Two viewers at different cursors read the same bytes; the reference checks it (§7).
- **Slow the engine.** A connection that never reads blocks its own serving thread on its own socket and nothing else; the window moves on, and when the stalled viewer's next seq leaves it, the host owes it one `stream.behind` and nothing more. The engine's append has no path to a socket (§7 lints), the writer takes no lock a serving thread can hold (§6.2), and the host buffers nothing per viewer beyond the frame being written.
- **Know anything plane B or C knows, on channel A.** Channel A carries plane-A records only (`channel_a_v0.sql` R1). Channel C is a separate socket from a separate process.

## 5. Versioning

- This is `rehoboam/stream/wire/v1`: frame header version 1, the messages of §4, the error codes of §4.4. `stream.status` reports it (`wire`).
- **Unknown version.** A host that reads a frame whose `version` it does not speak answers with one control frame at the lowest version it speaks (v1), an `unsupported_version` error with `id` null and `data.supported`, then closes. A viewer that reads a frame of a version it does not speak closes. Neither guesses. Because `magic` and `version` are frozen (§2), a v1 viewer talking to a v3 host learns that from the host's first frame.
- **Additive within v1** (minor, `05.9` IF-9): a host may add a field to a result or a notification's params, and a new notification method, in a later minor; a viewer ignores result fields and notifications it does not know. A host never ignores: an unknown request parameter is `invalid_params`, an unknown method `method_not_found`, an unknown envelope member `invalid_request`. The minor version is not on the wire; the fields' presence is.
- **Breaking** (major): a change to the header, the sub-header, a field's type or meaning, an error code, or the removal of anything, is v2 with a migration note in `05.9`. A new **request** method on a stream socket is a major change and must be a read (§4.5's invariant).
- The record lines inside a RECORDS frame are versioned by `stream.status.canonical_format` and `record_format`, independently of the wire: a later format change bumps those and not this (SPR-020's envelope amended channel A v0 and the canonical form v0 in place before any segment was written, REV-SPR-020 R6, so neither was bumped); the wire reads only the frozen prefix (§2.2).

## 6. The live-edge rule

### 6.1 State and the window bound

Per channel and timeline, the host keeps:

- **The sealed set** on disk: every sealed segment (`channel_a_v0.sql` R5, `channel_c_v0.sql` R7). `s = sealed.last_seq` is the last seq of the newest non-empty one (null while none exists); every record with `seq ≤ s` is on disk. A sealed segment is never modified (atomic sealing, `segment.parquet` last).
- **The window** `W = [w0, w1)` in memory: `w0 = live.first_seq`, `w1 = live.next_seq`.
- **The tip** `w1`: the next seq. Every record ever written has `seq < w1`.

**The window policy.** A sim-year's records enter the window as they are appended and leave it as one block. Sealing a completed year (Parquet encoding, `fsync`, rename) runs **off the engine host's thread**, so a seal overlaps the writing of the next year; the year being sealed stays in the window until it is on disk, whatever the policy. The director decides the bound (D42, PR #60), between:

| Option | Rule | Bound on the window | Cost |
|---|---|---|---|
| (a) PERF-5 literal | retention 0: a year leaves memory when its seal is on disk, and the seal runs **synchronously on the host thread** at the year boundary so that no second year accumulates | the current sim-year | Parquet encoding and `fsync` on the tick path once per sim-year, against §6.2 and `05.10` §7 item 3 |
| (b) year-end drop (**recommended, 75%**) | a year leaves memory when the next year **completes** or, if its seal lands later than that, when the seal **lands** (R16); the seal runs off-thread | the current sim-year and the previous one, **provided a seal lands within a sim-year of writing**; a slower seal overruns the bound by its lateness (a seal that lands k sim-years after its year's end keeps that year k extra years, so the window holds 2 + ⌈k⌉ years while it lasts), and the host logs every seal's latency against the sim-year's wall time so the overrun is visible | a live viewer that is a few records behind at the moment of a seal is never sent to disk; two years of lines in memory, about 1–3 MB at 10k persons, 30–100 MB at 1M; at MAX on a tiny world a sim-year can be shorter than an `fsync`, which is the overrun case |
| (c) record-count tail | the window keeps the newest N records across year boundaries, with the sealed set behind it | N records | a fixed byte bound at any scale; the viewer's "behind" point no longer coincides with a sealed year, so catch-up reads a partial year from disk |

The reference implements (a) and (b) (`Writer.policy`, `sync` and `year_end`) and asserts each one's bound over every run (`peak_years`), the slow-seal overrun included (a scenario whose seal takes 20 of 12 ticks shows a 3-year window and nothing more); (c) is stated, not simulated. **The default is (b), marked pending D42**; a host setting selects (a). The RSS criterion of V-PERF-09 (§9) follows the option ruled.

Invariants, held at every instant a viewer can observe (every `stream.status` answer, every error carrying `(s, w0)`, and every frame), **when read from one snapshot** (§6.2):

- **I1 (no gap between disk and memory):** `s + 1 ≥ w0`; while `s` is null, `w0 = 0`. Every `seq < w1` is on disk or in `W` (or both).
- **I2 (contiguity):** seqs are assigned contiguously from 0 by the writer (`channel_a_v0.sql` R3); `w0 ≤ w1`; `W` has no hole.
- **I3 (immutability):** a seq's record never changes, in memory or on disk; a record in both places is the same line.
- **I4 (monotone, in order), within one `run_nonce`:** `s`, `w0` and `w1` never decrease while one host runs (a resumed host starts again from its disk, §6.2). `w0` moves only when a year is dropped from memory, which happens only after that year's seal is complete on disk, and the writer **publishes `s` before it moves `w0`**: a reader that loads `w0` and then `s`, or either from one snapshot, never sees `s + 1 < w0`. The reference's seal-race check interleaves a reader's loads with the writer's steps and shows that the atomic read and the `w0`-then-`s` order never break I1, while the reverse order and a writer that moves `w0` first can (R4).

### 6.2 The host's side: the writer never waits

Normative:

- **Appending never waits.** The writer appends a record to `W` and advances `w1`; this is the only thing on the engine host's tick path. The writer **takes no lock that a reader or a serving thread can hold, touches no per-connection state, and never waits on a serving thread**: not for a seal, not for a notification, not for memory. A reader observes the writer's state only through an atomic snapshot that the writer publishes; nothing a reader does can delay a publication.
- **Validation at append, or not at all.** The writer renders a line from a record it holds and validates it then (the canonical printer is total on valid records; `rh-stream`'s round-trip test keeps it so). Serving threads **copy bytes**: a RECORDS frame is the sub-header plus a byte range of the window, never a parse. A viewer parses what it needs.
- **One snapshot per answer, and the snapshot owns what it reads.** A snapshot carries `w0`, `next_seq`, `s`, `last_year`, the seal log's length **and the references to the chunks** that hold the window from `w0` (R17). `LiveEdge::read(from, max) -> Result<Lines, Behind { s, last_year, w0 }>` evaluates `from ≥ w0` and copies the lines **from the snapshot's chunks alone**, never from the live window, so a seal and a drop that complete between the snapshot and the copy change nothing the copy sees: a frame carries exactly the seqs it claims. A `stream.status`, a `behind_live_edge`, a `stream.behind` and a `stream.sealed` take every field they carry from one snapshot. A serving thread never checks `w0` in one load and copies in another (the check-then-copy race, R4); the reference's seal-race check runs a drop between a snapshot and its copy and shows the copy correct, and the live-window copy of the round-2 reference wrong.
- **Seal latency.** The host measures every seal from its year's end to its landing on disk and logs it against the sim-year's wall time; a seal slower than a sim-year is the overrun case of §6.1 (b), and V-PERF-09 prints the distribution (R16).
- **The seal log.** Every seal appends an entry `(sim_year, first_seq, last_seq | null, canonical_digest)` to a shared, append-only log that is part of the published state. Serving threads **pull** from it: each connection remembers the log index it has reached and emits `stream.sealed` for the entries its cursor has passed (an empty year at once). The writer never pushes anything to a connection.
- **Leaving the window.** Before copying, the serving thread's snapshot says whether `c ≥ w0`. If `c < w0` (the viewer was slower than the window: the host dropped the year `c` is in), it sends `stream.behind` with `next_seq = c` and `(s, last_year, w0)` from that snapshot, ends the subscription, and sends no more records. By I1 every seq in `[c, s]` is on disk, and `s + 1 ≥ w0`, so the viewer can resume at `s + 1` without a gap.
- **Waiting for records.** When `c = w1`, the serving thread waits for the writer's next publication by polling with a short sleep, or by a primitive the writer cannot block on (a futex wake the writer issues without waiting; never a mutex-guarded condition variable the reader holds). A `stream.subscribe` from `n` is answered by the same check: `n < w0` is `behind_live_edge`, `w0 ≤ n ≤ w1` is accepted with `first_seq = n` (`n = w1` means "wait for the next record"), `n > w1` is `future_seq`.
- **Bounded per viewer.** The host holds, per connection, its cursor, its seal-log index, and the one frame being written (at most 1 MiB). It never queues records per viewer, so a stalled viewer costs one thread and one frame, and the window plus the seal log is the host's only stream memory.
- **Aborting.** A host that stops on an error (a poisoned engine, BCK-002 A-42) never seals a partial year: it sends `stream.end { reason: "aborted" }` to every subscription and leaves the unsealed year unsealed, for the resumed run to re-emit (the reference simulates it).
- **A resumed host** (after a `kill -9` or an abort, R18) starts with an **empty window**: `w0 = w1` = the first seq of the first sim-year that is not on disk (which may be the previous year, if its seal was in flight when the host died: that seal is lost and the year is re-emitted), and `s` and `last_year` read from the sealed segments. It then replays from the nearest keyframe (every 10 sim-years, `05.10` §4) and emits nothing below `w0`; the records it emits from `w0` on are the same bytes the lost host emitted, **while every input was journaled before the run** (P1a: `rh-cli run --inputs`). For P2's tip inputs, journaled at issue time, a resumed run's `STEP_DIGEST` carries a `state_hash` that differs from the continuous run's until the last issue time (VER-SPR-007-TEST OBS-10), so byte-for-byte re-emission is not guaranteed there; a viewer detects it through the resumed year's `stream.sealed` (§6.3). The new host has a new `run_nonce`; I4 holds within one nonce.

**The mechanism (R22).** The property above is binding; the design that meets it under `#![forbid(unsafe_code)]` without a lock is one **`ArcSwap`** (crate `arc-swap`, MIT OR Apache-2.0, no dependencies, default features off, **pre-approved for `rh-stream` alone** and listed in `bin/lint_arch.py` `PLANE_A_EXTERNAL_DEPS`; SPR-026's Developer picks the exact pin, at a version at least two weeks old) of an immutable snapshot `(w0, next_seq, s, last_year, chunk list, seal log)`: the writer builds the next snapshot (a new chunk appended, or a seal landed and `s` published, or a year dropped and `w0` moved, in that order per I4) and stores it; a serving thread loads the `Arc`, reads and copies from it, and drops it; the writer never waits, a reader never blocks the writer, and a chunk lives as long as some snapshot references it. It holds no simulation state, so determinism is untouched. The documented alternative, standard-library only, is a fixed-capacity ring of `AtomicU64` words with a generation check after the copy, whose capacity is fixed at start. A `loom` model of `LiveEdge` is recommended for SPR-026, not required.

### 6.3 The viewer's side: catch-up with no gap and no duplicate

A viewer keeps one number, its cursor `c` (the next seq it needs), and runs the loop of `rh-protocol`'s `Catchup` state machine (§7; the reference's `Viewer`):

```text
loop:
  r = subscribe(from = {seq: c})
  if r is behind_live_edge(sealed_last_seq = s, live_first_seq = w0):
      assert s + 1 >= w0                      # I1, else protocol error
      read sealed segments for seqs [c, s]    # exactly s - c + 1 lines, else protocol error
      c = s + 1
      continue                                # the window may have moved again; loop
  if r is future_seq(finished = true): stop                              # the cursor is past the end of a finished run
  if r is future_seq or too_many_viewers:
      wait with exponential backoff (1, 2, 4 … 64 polls), then continue   # a resumed host, a full host
  if r is first_seq = c:
      while frames arrive:
          take the frame's records; check first_seq == c; c += count
      on stream.behind(next_seq, s, w0): check next_seq == c; read sealed [c, s]; c = s + 1; continue
      on stream.end(reason): done
  on a protocol error: close the connection (state Failed; nothing else is accepted until closed)
  on a closed connection: reconnect; stream.status; refuse another history (stop); on a new run_nonce,
      remember the resumed year and verify what was received of it against its stream.sealed; continue at c
  on a closed connection during a sealed read: finish the read, then reconnect (never resubscribe blindly)
  on a retry timer that fires after a reconnect: ignore it
  on the Jump input (while live): unsubscribe; frames still in flight are delivered as live ones;
      check unsubscribe.next_seq == c; subscribe(from = "live"); c = first_seq
      a stream.behind in flight ends the jump's subscription: the not_subscribed answer to the unsubscribe
      is expected, then subscribe live; a stream.end in flight ends the viewer
```

Why it is gap-free: every seq the viewer skips over is read from disk, and I1 guarantees the disk reaches the window (`s + 1 ≥ w0`), so `subscribe(from = s + 1)` is never `behind_live_edge` unless the window moved again, in which case the loop reads the newly sealed years; a sealed read that returns fewer lines than `[c, s]` (a gap on disk) is a protocol error, never skipped. Why it is duplicate-free: the viewer resumes exactly at `c`, the host serves from exactly `c` (I2, I3), and a sealed read stops at `s` exactly; nothing is delivered twice because nothing is ever requested twice. Why a restart is safe: a resumed host starts from its disk and re-emits the lost records from the first unsealed year (§6.2), so a cursor above the resumed tip waits (`future_seq`, backoff) and then continues, and the resumed year's `stream.sealed` lets the viewer verify that what it had already received of that year is what the resumed host sealed (R21; in P1a it is, because inputs are journaled before the run); a `future_seq` with `finished` means the seq will never exist and the viewer stops; a different `history` means another run and the viewer stops. Why the transport's orderings are safe (R15): a frame already in flight when the viewer sends a jump's unsubscribe is delivered like any live frame; a `stream.behind` or `stream.end` in flight under a jump ends the subscription, so the `not_subscribed` answer that follows is expected; a connection that closes during a sealed read is remembered and the read's end reconnects; a backoff timer that fires after a reconnect is a stale timer and is ignored. The golden transcript holds all four orderings. Why it terminates: the window moves only at a seal, once per sim-year, and reading a sealed year from disk is faster than simulating one, so a viewer that is faster than the simulation catches up, and one that is slower keeps reading sealed years for good, which is the "40 years behind at 1 yr/s" cursor of `05.10` §1.

The reference simulates fast, slow, crawling, stalled, reconnecting, jumping and late viewers over a writer with asynchronous seals, both window policies, a seal slower than a sim-year, empty channel-C years, a host restart that loses a seal in flight, an abort and a connection cap, and checks every delivery is exactly `0, 1, 2, …` (a jump excepted, where both sides are contiguous) with per-year digests equal to the sealed ones and to every `stream.sealed` (§7).

**The sealed-read path.** A viewer reads `[c, s]` from the sealed segments with `rh-stream`'s reader (a Rust viewer) or, for a standard-library Python viewer such as the Textual TUI stub, through a `rh-cli tail --canonical --from-seq c --to-seq s` subprocess, which prints exactly the canonical lines of that range in `seq` order (`--from-seq` is SPR-013 T-10's; `--to-seq` is SPR-026's, a small flag, R23). A Python viewer never needs `pyarrow` or DuckDB for catch-up.

### 6.4 Channel C live

Channel C's socket is served by the sifter process (SPR-013 ruling 14; `rh-cli sift <run-dir> --follow --socket` in SPR-026, with the same `serve.rs`), from its own window, with the same frames, messages and rule, and with the empty-year form of `stream.sealed`. "Live" on channel C means **match records as the sifter writes them**: in P1a the sifter is downstream of sealed channel-A segments, so channel C's edge trails channel A's by up to one sim-year plus the sift time, and `sifted_through` says how far it is. The engine process never serves channel C, never reads it and never knows whether a sifter runs (ADR-0024; V-PLN-04). A later in-process sifter (P2.1) changes when matches are written, not the wire.

## 7. `rh-protocol`: the crate SPR-026 builds

| Item | Ruling |
|---|---|
| Name, place | `crates/rh-protocol` (`05.0` §6: "wire formats (control schemas, binary frames)") |
| Labels | `[package.metadata.rehoboam] plane = "tool", kind = "tool"`, **pinned by name** in `bin/lint_arch.py` `PINNED_LABELS`, so it cannot be relabelled into plane A |
| Engine crate | Yes: it runs in the engine host's process, so it joins `ENGINE_CRATES` (V-INSP-05) and may not reach the media, HTTP-UI and LLM deny-list; it has no network stack of its own (no `tokio`, no `hyper`, no `mio`, no `socket2`): it is bytes in, bytes out |
| Dependencies | **No workspace crate, of any dependency kind** (normal, build, dev), and an **exact third-party allow-list**: `serde` and `serde_json` as normal dependencies, `proptest` as a dev-dependency, nothing else, nothing as a build-dependency. Rule `protocol-leaf` of `bin/lint_arch.py` refuses any other edge (`PROTOCOL_EXTERNAL_DEPS`). Why not `jsonrpsee` (docs/14 §K lists it for `rh-server`): it has no Unix-socket transport and needs an async runtime; the control plane here is one object per frame, which `serde_json` spells and §4 dispatches in a page of code; `jsonrpsee` stays an option for `rh-server`'s WebSocket in P2 |
| What the leaf rule buys | A viewer that depends on `rh-protocol` gets no path to `rh-core`, `rh-stream` or `rh-snapshot`, and no transport: the crate that spells the wire cannot write a record, open a journal, open a socket or hold an engine. Conversely the plane-A closure (`bin/lint_arch.py`) forbids every plane-A crate from depending on a tool crate, so `rh-stream` and the engine cannot even name a frame: the engine's output is lines in memory, and the wire is applied to them by the host, outside plane A |
| Modules | `wire`: `Header`, `FrameType`, `MAX_PAYLOAD`, `VIEWER_MAX_PAYLOAD`, `encode(frame) -> Vec<u8>`, `decode(&[u8], limit) -> Result<(Frame, usize), WireError>` (returns the bytes consumed; a short buffer is `Incomplete`, so a reader can accumulate). `records`: `RecordsFrame { timeline_id, channel, first_seq, lines }` with the §2.2 prefix checks only (the kind table comes from a `const` copied from `canonical_v0.py`'s `CHANNEL`, checked against the canonical golden file by the conformance test); a `RecordsBody` that a serving thread fills from a byte range without a parse. `control`: `Request`, `Response`, `Notification`, `Method` (the three), `SubscribeFrom`, `Status`, `Sealed`, `Behind`, `End { reason }`, `RpcError`, `ErrorCode`, `Reason`, each `Serialize`/`Deserialize` in canonical field order, with `deny_unknown_fields` on every envelope and request-parameter struct and **not** on results and notification params (§3.1); `dispatch(&mut Connection, &[u8]) -> Vec<Vec<u8>>`, the host's dispatcher of §4 (the method table, the parameter refusals, the caps). `cursor`: `Catchup`, the pure state machine of §6.3: `fn next(&mut self, event: Event) -> Action` with `Event::{Subscribed(first_seq), Behind { next_seq, s, w0 }, Records { first_seq, count }, Sealed, End(reason), Closed, SealedRead { first_seq, count }, Refused { reason, finished }, RetryElapsed, Status { same_run }, Jump, Unsubscribed(next_seq)}` and `Action::{Subscribe(seq), SubscribeLive, ReadSealed { from, to }, Deliver { first_seq, count }, Ignore, Wait(polls), Status, Unsubscribe, Done(reason), Close(detail), Stop(detail)}`, states `Idle, Connecting, Subscribing, Live, ReadingSealed, Retry, Jumping, JumpEnded, Failed, Done, Fatal` with a reconnect-pending flag for a close during a sealed read (R15), so `rh-cli tail --live`, the TUI stub and the Tester's harness share one implementation of the loop; the golden file's `scenario catchup` transcript is reproduced line for line |
| Host side (not in `rh-protocol`) | The Unix-socket server: accept, one serving thread per connection, the window reader, the dispatcher's caller. SPR-026 puts it in the tool-plane host that already runs `rh-stream`'s writer, `rh-cli run … --socket`, in **`crates/rh-cli/src/serve.rs`**, the one file of `rh-cli` allowed a socket (`bin/lint_forbidden.py` `NETWORK_FILES`); the same module serves `rh-cli sift … --follow --socket` for channel C. No `rh-server` crate before it has a consumer (R13); `rh-server` (P2, `05.0` §6) absorbs it when the WebSocket control plane arrives. `rh-stream` (plane A) exposes the window as `LiveEdge` (`snapshot()`, `read(from, max)`, `bytes()`, the seal log), built by the writer after each instant under the normative rules of §6.2; the server reads it and never writes it |
| Lints | `bin/lint_arch.py`: `PINNED_LABELS["rh-protocol"] = ("tool", "tool")`; `ENGINE_CRATES` gains `rh-protocol`; rule `protocol-leaf` (fixtures `arch/protocol_depends_on_plane_a`, `arch/protocol_third_party`); the closure already refuses a plane-A crate depending on it (fixture `arch/plane_a_to_protocol`). `bin/lint_forbidden.py`: rule `network` (V-DET-19): `std::net`, `std::os::unix::net`, `net::<Type>` through a renamed, glob-imported or `{self as x}` parent, and the listener, stream and datagram types are forbidden in plane-A `src/` (planted controls `forbidden/planted/net_socket.rs`, `net_parent_alias.rs`, `net_parent_glob.rs`, `net_parent_self.rs`); in the tool crate `rh-cli` (`NETWORK_FILES`, keyed by package name) the same rule holds everywhere but `src/serve.rs`, a `#[path]` is refused and a literal `include!` followed so a socket cannot be spliced in from another file, and in `serve.rs` itself a `pub use` or `pub type` of a socket type is refused (rule `network-reexport`; fixtures `forbidden/network_workspace` and `forbidden/network_bypasses`, the latter the six ways round the rule that compile, R19). The lint keeps socket **types** out of every file of the host but `serve.rs` and out of plane A altogether; that the tick path never waits is shown by probe (b) of §9, not by the lint alone |
| Conformance | `crates/rh-protocol/tests/golden.rs` reads `specs/stream/golden/wire_v1.txt`: it encodes every `frame` from its decoded description and compares the bytes, decodes every frame back, refuses every `refuse` line with the named reason, answers every `dispatch` line's request with exactly the reply bytes (on a fresh connection to an empty host), drives `Catchup` through the `scenario catchup` events and compares the actions; a proptest round trip over random frames and control messages; the method-table test of §4.5 (the dispatcher's table equals `{stream.status, stream.subscribe, stream.unsubscribe}` and every entry is a read) |

## 8. Reserved: the control endpoint (SPR-027, P2)

*Specified: `specs/stream/control_v1.md` (SPR-027 T-01; a reviewed spec PR of its own, REV-WAVE5 W30) is the method set `rehoboam/stream/control/v1` this section reserves: the pace command (`control.status`, `control.pace`, `control.pause`, `control.resume`, `control.step`; `control.pace` and `control.pause` are v1's spellings of IF-3's `timeline.run(band)` and `timeline.pause()`) over this document's framing (§2), canonical JSON (§3.1), error model (§4.4) and version rule (§5), with its own codes −32120 to −32129, its run log, and the P2 operations (`timeline.*`, `player.act`, `sift.set_autopause`) reserved under IF-3's names and refused by name. Nothing below changes; the paragraph stands as the reservation.*

`<run-dir>/stream/t<id>/control.sock`, mode `0600`, is reserved for the control plane of IF-0 (`05.10` §7a; ADR-0035 §6: "a control and stream endpoint"): the pace command (SPR-027; STR-5), and in P2 tip and scheduled inputs and forks (STR-4; `05.9` IF-3's `timeline.*`, `player.*`, `oracle.*` over the same framing and JSON-RPC carrier as §2.1 and §4). It is a different socket from the stream's so that the read-only property of §4.5 is a property of a file, not of a method table: a viewer given only `A.sock` and `C.sock` can act on nothing. Its method set carries **its own version string**, `rehoboam/stream/control/v1`, reported by its own `control.status`, and its own method table; this document fixes only that it uses this framing (§2), the canonical JSON (§3.1), the error model (§4.4) and the version rule (§5), that its methods are SPR-027's and P2's to specify, and that nothing it does is journaled unless IF-2 says so (the pace command is run-log only). BCK-002 A-36 (acceptance cursors in the journal) belongs to the first journaled input on `control.sock`, in P2; neither SPR-026 nor SPR-027 journals an input (R12).

## 9. Acceptance: V-STR rows to probes (SPR-026)

| Test (`docs/09`) | Probe in SPR-026 | Pass |
|---|---|---|
| V-STR-01 | `rh-cli run solo` and `hamlet` headless, then with four viewers attached (`tail --live` at the edge, one at a scripted 100× playback, one rewinding by re-subscribing from old seqs, one stalled: connected, never reads); compare every sealed segment's `canonical_digest` and the state ladder | Equal, byte for byte; the ladder equals the baseline |
| V-STR-03 | The same run: viewers at 1×, 100×, rewinding and stalled during a MAX run | Channel A unchanged (V-STR-01); throughput per V-PERF-09 |
| V-STR-05 | A viewer that deliberately sleeps past a seal (it subscribes, sleeps two sim-years of wall time at MAX, resumes): it gets `stream.behind`, reads the sealed years through `rh-cli tail --canonical --from-seq --to-seq`, subscribes again, and continues to `stream.end`; the viewer's per-year XXH3-128 over the bodies it received equals every `stream.sealed.canonical_digest` and every `segment.canonical_digest` on disk; its seqs are `0..next_seq` exactly. Then "jump to live" from a cursor 10 years behind: `first_seq = live.next_seq` at the answer, nothing duplicated after it. Then a `kill -9` of the host mid-year (a seal in flight) and a resume from the keyframe: the viewer gets `future_seq`, backs off, verifies the resumed year against its `stream.sealed`, and ends with the same seqs and digests | Contiguous, no gap, no duplicate; digests equal; `Catchup` reports no protocol error |
| V-PERF-09 (P1a variant; **SPR-030's row**, D34, PRJ-002 §5; SPR-026 supplies its ingredients, R23) | On `adjutant` (RUN-002): the 10k dynastic world headless, then with `rh-cli tail --live`, the Textual TUI stub (SPR-027) and one stalled consumer, **the viewers pinned to cores the engine does not use** (`taskset`), so a viewer's parsing cost never competes with the tick; wall time, the live bytes and the ladder. SPR-026 delivers the stalled consumer, `LiveEdge::bytes()`, the seal-latency log and the per-viewer frame cap, and counts the host's live bytes with SPR-009's counting allocator rather than measuring RSS | Throughput ≥ 95%; the host's live bytes ≤ peak `LiveEdge::bytes()` (the window as allocated, under the option D42 rules: two sim-years by (b), plus any seal overrun the latency log shows; and the seal queue's completed years as allocated, from queued to sealed: GAP-1 of VER-SPR-026-TEST, the Architect's ruling) + V × 1 MiB (one frame per viewer) + a stated allowance for threads, with every term and the seal-latency distribution printed; the ladder identical; the stalled consumer, resumed after the run, reads everything from sealed segments with nothing missing |
| "The engine never waits" | (a) `bin/lint_forbidden.py` rule `network` is green on the workspace and red on its four planted controls (the three parent-module bypasses included), on `forbidden/network_workspace` and on `forbidden/network_bypasses` (the six splices and re-exports); `bin/lint_arch.py` `protocol-leaf` (both fixtures) and the plane-A closure are green and red on their fixtures: the lints keep socket types out of plane A and out of every file of `rh-cli` but `serve.rs`. (b) **Deterministic, and the proof that the tick path never waits:** a unit test of `LiveEdge` freezes a serving thread inside `LiveEdge::read` on a test barrier, **across a year-end drop**, and stalls another connection's socket (never read); the writer must then append and seal 10 sim-years before a deadline (pass/fail, no statistics), its stream memory staying the window; once released, the frozen reader returns exactly the lines `[c, c + n)` it was asked for, or `Behind`, and the stalled connection's first read after the run is `stream.behind`. A `loom` model of `LiveEdge` is recommended. (c) `cargo nextest run -p rh-snapshot --test observer`-style: the host's `state_hash` ladder with and without viewers is one ladder | All green |
| Seal races | The reference's seal-race check reproduced in Rust against `LiveEdge`: a reader's two loads interleaved with a seal's steps never show `s + 1 < w0` through the snapshot API, and a seal and a drop completed between a snapshot and its copy leave the copy carrying exactly the seqs the frame claims | No I1 violation in any interleaving; every frame's lines are its `first_seq + i` |
| V-IF-02-style | Corrupt frames fed to the host (a fuzz seeded from the reference's `refuse` and `dispatch` lines): wrong magic, version 2, type 3, flag 1, length over 4 KiB, a truncated payload, a records frame from a viewer, a line with a bad prefix, a JSON batch, a float seq, a depth-5 object, an unknown member, 100 000 nested arrays | The host answers `invalid_frame`, `unsupported_version`, `parse_error` or `invalid_request` and closes the connection; the run continues, the ladder is unchanged, no panic |
| Read-only | `player.act`, `timeline.fork`, `timeline.run`, `oracle.whisper`, `stream.pace` sent on `A.sock`; the method-table test of §7 | `method_not_found` with the read-only message; the journal is unchanged (row count and chain head); the ladder is unchanged; the table has three read entries |
| Channel C live | `rh-cli sift --follow --socket` beside the run; a viewer on `C.sock` from seq 0 to `stream.end`, through sim-years with no match | Its lines equal `rh-cli tail --canonical` of the sealed channel-C segments; every empty year arrives as `stream.sealed` with `last_seq: null`; `sifted_through` never exceeds channel A's `sealed.last_seq`; the state ladder is unchanged (V-PLN-04) |
| Conformance | `crates/rh-protocol/tests/golden.rs` against `specs/stream/golden/wire_v1.txt` | Every golden frame reproduced and decoded; every `refuse` refused; every `dispatch` answered byte for byte; the `Catchup` transcript reproduced |

## 10. Gates

- `python -I specs/stream/selftest_wire_v1.py --check`: the self-test (frame round trips, every message encoded and decoded, the refusals, the host's dispatcher, the live-edge simulation under several seeds, both window policies, empty years, a restart and a connection cap, the seal-race check, the `Catchup` transcript), then the golden file compared with the reference's rendering. `bin/check_all.sh --part core` runs it, plus two positive controls: a stale golden frame must FAIL with `STALE:`, and `--control window-gap`, a host whose window moves before the year it drops is on disk (I4 broken), must FAIL the live-edge check with `I1 broken`, the viewer's own detection; a `--control` run never writes the golden file.
- `python -I specs/stream/selftest_wire_v1.py` rewrites the golden file after the self-test. The record lines in it come from `golden/canonical_v0.txt`; whichever of PR #60 and the SPR-020 envelope merges second regenerates it with this command (R6).
- `bin/lint_arch.py` and `bin/lint_forbidden.py` with their new fixtures (§7), in the same part.

## 11. Proposed for SPR-013 T-09 (the Architect applies it at the merge; R18)

Add to T-09's acceptance: *A mid-year `kill -9` of `rh-cli run` followed by a resume from the 10-year keyframe re-emits no seq below the first sim-year that is not on disk, and that year's lines, `STEP_DIGEST` included, are byte-identical to the unbroken run's; a seal in flight at the kill is lost and the year is resealed to the same bytes. (SPR-026's host starts its live edge from that same point, `specs/stream/wire_v1.md` §6.2.)*
