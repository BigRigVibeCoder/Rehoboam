# World Stream canonical text form v0 and `segment.canonical_digest`

*Status: v0, a C2 contract for P1.0 (SPR-013 T-09, T-11; PR #37 Q11). Reference: `specs/stream/canonical_v0.py` (printer, parser, digest, MatchId, self-test); golden lines: `specs/stream/golden/canonical_v0.txt`. Record definitions: `specs/stream/channel_a_v0.sql`, `specs/stream/channel_c_v0.sql`. A change to a token, a field or the digest is a new version (`rehoboam/stream/canonical/v1`), announced like a Registry change.*

## 0. Why a text form

A sealed segment is Parquet. Its bytes depend on the Parquet library, its version and its encoder settings, so "byte-identical segments" (V-STR-01, `05.10` §4) can only be promised within one build. The canonical text form is the record itself, spelled once: every record kind has exactly one line, every line parses back to exactly one record (lossless), and the segment's `canonical_digest` is a hash of those lines. The digest therefore survives a Parquet upgrade, an encoder change or a re-write of the same records on another machine, and two implementations (rh-stream in Rust, this reference in Python) can be checked against each other line by line. `rh-cli tail --canonical` prints it; the Tester's fuzz tier feeds it to the parser (SPR-013 T-09).

Design rules:
- **One spelling.** Every value has exactly one token. The parser refuses every other spelling (a leading zero, a trailing space, an uppercase hex digit, an escaped safe byte), so that `parse(render(r)) = r` and `render(parse(l)) = l` both hold; a line that parses is already canonical.
- **ASCII only.** Strings are escaped to ASCII, so a line is bytes = characters and a digest needs no text encoding step.
- **Positional.** Fields are in a fixed order per kind, so the grammar is a table (§3), not a language.
- **Readable.** Ids and counts are decimal, hashes are hex, lists are bracketed; a human reads the line without the schema.

## 1. Lines and files

- A **line** is a sequence of **tokens** separated by exactly one space (0x20). It holds no other byte below 0x21 and no byte above 0x7E. It ends with one LF (0x0A); there is no CR. Where a line is quoted below, its LF is left out.
- The first token is the **kind name** (`INPUT`, `EVENT`, `LIFE_ENTRY`, `LIFE_EXIT`, `STEP_DIGEST`, `KEYFRAME`, `MATCH`), then the kind's fields in the order of §3. Nothing else: no trailing fields, no comments on a record line.
- A **canonical file** (what `rh-cli tail --canonical` writes, what `canonical_v0.py --digest` reads) is record lines, in `seq` order, plus any number of lines starting with `#`. A `#` line is a header or a comment: it is not a record and is never hashed. The first line of a file written by a tool is its header, `#rehoboam/stream/canonical/v0 timeline=<u32> channel=<A|C> registry_version=<string token> format=<string token>`, so a reader knows which dictionaries (the segment's `verb`, `record_kind`, `pattern` tables) the ids in the lines refer to. The header carries the Registry version; the record lines do not repeat it.

## 2. Tokens

| Token | Form | Notes |
|---|---|---|
| **int** (`i64`, `i32`, `u64`, `u32`, `u16`, `u8`) | `0`, or `-?[1-9][0-9]*` within the type's range | Decimal, no leading zero, no `+`, no `-0`, no `_`. Ids (`u64`) are unsigned decimal, never hex or signed. Out of range is an error |
| **optional int** | an int, or `~` for SQL NULL | Only where §3 says `?` |
| **hash64** | exactly 16 lowercase hex digits | A `UBIGINT` hash column (`state_hash`, `observation_hash`, `narrative_hash`, `match_id`) |
| **blob16** | exactly 32 lowercase hex digits | A 16-byte `BLOB` (`payload_hash`, `row_digest`): the bytes in storage order |
| **string** | the UTF-8 bytes of the value, each byte in the safe set written as itself, every other byte as `%` + two **uppercase** hex digits; the empty string is the single character `%` | Safe set: `A-Z a-z 0-9 . _ - / : + @`. `%` is never safe (`%` is `%25`), so a lone `%` token is unambiguous. An escaped safe byte, a lowercase hex escape, a truncated escape, an unescaped byte outside the set and a byte sequence that is not UTF-8 are errors |
| **identifier** | `[A-Za-z_][A-Za-z0-9_]*` | A pattern role variable (`A`); written as is |
| **clause** | `e[0-9]+` | A pattern clause's `bind` (`e1`) |
| **list(T)** | `[` items joined by `,` `]`; the empty list is `[]` | No spaces inside. An item is an int or a **tuple**: components joined by `/` |

Enumerations (`phase`, `secrecy`, `origin`, `class`, `entity_kind`, `role`, `exit_kind`, `death_cause`, `selection`, `on_complete`, `status`) are their integer codes, exactly the codes the segment DDL fixes; names would tie the line to a Registry version. Verbs appear twice on an `EVENT` line, as `verb_id` (the Registry index, `u16`) and as the verb's name (a string), so that a line is readable and lossless without the segment's `verb` dictionary; a parser given the dictionary checks the pair. Fixed-point quantities (`magnitude`, Q16_16 payload values) are their raw integers. Nothing is ever a float.

## 3. Records

`tl seq tick` is the record header (`timeline_id u32`, `seq u64`, `tick i64`), the same for every kind. `?` marks an optional int. Child rows (roles, causes, bindings, events) are inlined as lists, so a record's line is complete.

| Kind | Fields after the header, in order |
|---|---|
| `INPUT` | `journal_seq u64` `class u8` `input_kind string` `scope string` `scope_id u64?` `producer string` `payload_hash blob16` `row_digest blob16` |
| `EVENT` | `event_id u64` `phase u8` `verb_id u16` `verb string` `act_classes u64` `place_id u32?` `magnitude i32` `tellability u8` `secrecy u8` `origin u8` `payload list(i64)` `roles list(entity_kind u8 / entity_id u64 / role u8)` `causes list(cause_id u64 / kind u8)` |
| `LIFE_ENTRY` | `person_id u64` `origin u8` `sex u8` `mother_id u64` `father_id u64` `birth_tick i64` |
| `LIFE_EXIT` | `person_id u64` `exit_kind u8` `death_cause u8` `event_id u64?` |
| `STEP_DIGEST` | `living u64` `births u32` `deaths u32` `state_hash hash64` `observation_hash hash64` `narrative_hash hash64` |
| `KEYFRAME` | `keyframe_format string` `keyframe_path string` `state_hash hash64` `journal_position u64` |
| `MATCH` (channel C) | `match_id hash64` `pack string` `pattern_id string` `pattern_version u16` `selection u8` `on_complete u8` `status u8` `opened_tick i64` `closed_by_event_id u64` `closed_by_seq u64` `bindings list(role identifier / entity_kind u8 / entity_id u64)` `events list(clause / event_id u64)` |

List orders, which the parser enforces:
- `roles`: in `ordinal` order; the position in the list *is* the ordinal, so ordinals are contiguous from 0 (`event_role` rows of a segment therefore never have a gap).
- `causes`: strictly ascending `cause_id` (one edge per cause, as `causal_edge`'s key says).
- `bindings`: strictly ascending role, bytewise.
- `events`: in clause order (`ordinal`), with distinct clause names and strictly ascending event ids (kernel order, `05.4` §2.3 item 2).
- `payload`: as emitted (its order is meaning; `DEATH`: `[cause]`).

Examples (from the golden file):

```text
EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 0 [1] [1/4369/1] []
LIFE_EXIT 0 11 216000 13107 1 0 ~
STEP_DIGEST 0 12 216000 0 0 1 0000000000000001 0000000000000002 0000000000000003
KEYFRAME 0 14 216000 % a%20b%25%C3%A9/%E6%97%A5%E6%9C%AC%09%0A%22%5C ffffffffffffffff 18446744073709551615
MATCH 0 0 0 e0d979477f64bc68 solitary_life life_ends 1 0 0 1 0 0 1 [A/1/4369] [e1/0]
```

The first is a natural `DEATH` of person 4369 at tick 0 (phase 4 Vital, verb 0, act-class bit 15, no place, tellability 128, payload `[cause 1]`, one role: a person, the target, no causes). The fourth has an empty `keyframe_format` and a path holding a space, `%`, `é`, two CJK characters, a tab, a newline, a quote and a backslash.

## 4. The digest

```text
canonical_digest(segment) = XXH3-128( line(r_0) || LF || line(r_1) || LF || ... || line(r_{n-1}) || LF )
```

over the segment's records in `seq` order, where `line(r)` is the record's canonical line without its LF and `||` is byte concatenation. Nothing else is hashed: no header, no dictionary, no segment row (those are compared as values). XXH3-128 is seed 0 with the default secret (`xxhash.h` v0.8), the hash `rh_snapshot::hash::xxh3_128` already computes for the journal (`05.9` IF-2) and `bin/gen_hash_golden.py` references in the standard library. The 16 stored bytes are XXH3's **canonical form**: the high 64 bits big-endian, then the low 64 bits big-endian; printed, they are 32 lowercase hex digits. A segment with no record (channel C, a sim-year with no match) digests the empty string: `99aa06d3014798d86001c324468d497f`.

Why this and not an alternative:
- **XXH3-128 rather than XXH3-64:** 64 bits is a fingerprint for equality within one run (the plane hashes); the digest is kept across builds, Registry versions and machines for years, where a 128-bit identity costs nothing and makes an accidental collision not worth discussing. SPR-013's plan fixed it (head of Tasks).
- **Lines rather than Parquet bytes:** §0.
- **Not a Merkle tree over records:** a segment holds at most a few hundred thousand lines; one pass is cheap, and the twin check and `tail` read every line anyway. Per-record hashes would be a second contract to keep.

## 5. Refusals (what the parser rejects)

The reference's self-test plants these and requires `CanonicalError`: a leading, trailing or doubled space; a CR; a leading zero, a `+`, `-0`; an int outside its type; a missing or extra field; `~` in a non-optional field; an unknown or lowercase kind; a hash in decimal, in uppercase or of the wrong length; a lowercase, truncated or non-UTF-8 escape; an escaped safe byte; an unescaped control byte; a non-ASCII line; an empty string token; a scalar where a list is due; a trailing comma; a tuple with the wrong arity; causes or bindings out of order or repeated; event ids not ascending; a repeated or misnamed clause; a space inside a list.

## 6. MatchId (channel C)

```text
match_id = XXH3-64( "rehoboam/sift/match/v0" || str(pack) || str(pattern_id) || u16(pattern_version) || u32(n) || u64(event_id_1) || ... || u64(event_id_n) )
```

with `str(s) = u32(len(bytes)) || UTF-8 bytes`, every integer little-endian, the event ids in clause order, seed 0 and the default secret. It is `05.4` §2.3 item 10's `H64(pattern_id, pattern_version, event_ids)` with the pack added (a pattern id is unique within its pack). It is content-addressed, so a re-sift and the SQL twin agree on it without sharing state (V-SIFT-01); bindings are not hashed because, for one pattern, they are a function of the event tuple. Vectors are in the golden file (`match_id` lines).

## 7. Reference, golden file and gates

- `python -I specs/stream/canonical_v0.py --check` runs the self-test (a property round trip over generated records of every kind, printer idempotence, the §5 refusals, the known answers of §4 and §2), then compares the committed golden file with the reference's rendering. The golden file holds a hand-written channel-A segment that exercises every kind and edge (a negative tick, NULLs, an empty payload, i64 extremes, every string escape), its `life_ends` matches, generated segments, an empty segment, and the MatchId vectors. `rh-stream`'s test (SPR-013 T-09) parses the golden file and reproduces every line and digest; `rh-sifter`'s (T-11) reproduces the `match_id` lines.
- `bin/check_all.sh` runs `--check` and a positive control: a stale copy of the golden file must FAIL with `STALE:` (`tests/lint_fixtures/golden/stale_canonical_v0.txt`).
- `python -I specs/stream/canonical_v0.py --digest FILE` parses a canonical file and prints its digest, for the Tester: `rh-cli tail --canonical` of one segment, piped to a file, must give the segment's `canonical_digest`.
