"""Reference printer and parser of the World Stream canonical text form v0 (SPR-013; PR #37 Q11).

The canonical text form is the lossless one-line text form of every channel-A record kind (INPUT,
EVENT, LIFE_ENTRY, LIFE_EXIT, STEP_DIGEST, KEYFRAME) and of channel C's MATCH. Its grammar is
``specs/stream/canonical_v0.md``; ``segment.canonical_digest`` (channel_a_v0.sql R7,
channel_c_v0.sql R8) is the XXH3-128 of a segment's lines in ``seq`` order. This file is the
independent reference ``rh-stream`` (SPR-013 T-09) must agree with, as ``bin/gen_hash_golden.py``
is for ``rh-snapshot``:

- :func:`render` and :func:`parse` print and read one record, checking the grammar and the DDL's
  domains (enum codes, the MATCH invariants); :func:`digest` hashes lines; :func:`digest_file`
  checks and digests a whole canonical file;
- :func:`match_id` is channel C's content-addressed MatchId (channel_c_v0.sql R5);
- the self-test is a property round trip over generated records of every kind, idempotence of the
  printer, planted non-canonical lines and files that must be refused, and known answers;
- the golden file ``specs/stream/golden/canonical_v0.txt`` holds hand-written and generated lines
  with their digests and ``match_id`` vectors; the Rust printer, parser and digest must
  reproduce every line bit for bit.

XXH3 comes from ``bin/gen_hash_golden.py``, the repository's standard-library XXH3-64/128
reference (seed 0, default secret), loaded by path so that ``python -I`` works.

Usage:  python -I specs/stream/canonical_v0.py            (self-test, then rewrite the golden file)
        python -I specs/stream/canonical_v0.py --check    (self-test; exit 1 if the golden differs)
        python -I specs/stream/canonical_v0.py --check --golden PATH   (check another file; the
                                                           positive control uses a stale fixture)
        python -I specs/stream/canonical_v0.py --digest FILE   (check a canonical file, header to
                                                           final LF, and print the digest of its
                                                           record lines)
Exit:   0 ok, 1 stale golden file or a file that is not canonical, 2 the self-test failed.
Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import importlib.util
import itertools
import random
import re
import sys
import tempfile
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GOLDEN = HERE / "golden" / "canonical_v0.txt"
HASH_REF = ROOT / "bin" / "gen_hash_golden.py"
FORMAT = "rehoboam/stream/canonical/v0"
D_MATCH = b"rehoboam/sift/match/v0"
EXIT_OK = 0
EXIT_STALE = 1
EXIT_SELFTEST = 2

MASK64 = (1 << 64) - 1
HEX64_LEN = 16
HEX128_LEN = 32
DIGEST_LEN = 16
INT_MAX_CHARS = 20  # "-9223372036854775808" and "18446744073709551615" are 20 characters
# The bytes a string token writes unescaped (canonical_v0.md §2). Everything else is %XX.
SAFE = frozenset(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-/:+@")
INT = re.compile(r"0|-?[1-9][0-9]*")
HEX64 = re.compile(r"[0-9a-f]{16}")
HEX128 = re.compile(r"[0-9a-f]{32}")
ESCAPE = re.compile(r"%[0-9A-F]{2}")
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
CLAUSE = re.compile(r"e[0-9]+")
NULL = "~"
EMPTY = "%"
LF = 0x0A
CR = 0x0D
ASCII_LIMIT = 0x80


def _load(path: Path) -> ModuleType:
    """Import a module by path (``python -I`` adds no directory to ``sys.path``)."""
    spec = importlib.util.spec_from_file_location(path.stem, path)
    if spec is None or spec.loader is None:
        raise ImportError(str(path))
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module  # dataclasses resolve their module through sys.modules
    spec.loader.exec_module(module)
    return module


_HASH = _load(HASH_REF)


def xxh3_64(data: bytes) -> int:
    """XXH3-64 of ``data``, seed 0, default secret (the engine's hash)."""
    return int(_HASH.xxh3_64(data))


def xxh3_128(data: bytes) -> bytes:
    """XXH3-128 of ``data`` in its canonical 16-byte form: high half first, each half big-endian."""
    return int(_HASH.xxh3_128(data)).to_bytes(DIGEST_LEN, "big")


class CanonicalError(ValueError):
    """A token, line, file or value that is not in the canonical form or outside its domain."""


@dataclass(frozen=True)
class Codec:
    """How one field type is printed, parsed and generated."""

    name: str
    render: Callable[[object], str]
    parse: Callable[[str], object]
    gen: Callable[[random.Random], object]


def _int_codec(name: str, lo: int, hi: int) -> Codec:
    """A decimal integer in ``[lo, hi]``: no leading zero, no ``+``, no ``-0``."""

    def render(v: object) -> str:
        if not isinstance(v, int) or isinstance(v, bool) or not lo <= v <= hi:
            raise CanonicalError(f"{name}: {v!r} is not an integer in [{lo}, {hi}]")
        return str(v)

    def parse(tok: str) -> object:
        if len(tok) > INT_MAX_CHARS or not INT.fullmatch(tok):
            raise CanonicalError(f"{name}: {tok!r} is not a canonical integer")
        v = int(tok)
        if not lo <= v <= hi:
            raise CanonicalError(f"{name}: {v} is outside [{lo}, {hi}]")
        return v

    def gen(rng: random.Random) -> object:
        return rng.choice([lo, hi, 0 if lo <= 0 <= hi else lo, rng.randint(lo, hi)])

    return Codec(name, render, parse, gen)


def _enum_codec(name: str, allowed: Sequence[int]) -> Codec:
    """An integer code from the DDL's enumeration ``allowed`` (canonical_v0.md §3)."""
    inner = _int_codec(name, 0, 255)
    codes = frozenset(allowed)

    def check(v: object) -> object:
        if v not in codes:
            raise CanonicalError(f"{name}: code {v!r} is not one of {sorted(codes)}")
        return v

    def gen(rng: random.Random) -> object:
        return rng.choice(sorted(codes))

    return Codec(name, lambda v: inner.render(check(v)), lambda t: check(inner.parse(t)), gen)


def _optional(inner: Codec) -> Codec:
    """``inner`` or SQL NULL, written ``~``."""

    def render(v: object) -> str:
        return NULL if v is None else inner.render(v)

    def parse(tok: str) -> object:
        return None if tok == NULL else inner.parse(tok)

    def gen(rng: random.Random) -> object:
        return None if rng.random() < 0.3 else inner.gen(rng)  # noqa: PLR2004

    return Codec(f"optional {inner.name}", render, parse, gen)


def _hex64_codec(name: str) -> Codec:
    """A u64 hash as exactly 16 lowercase hex digits."""

    def render(v: object) -> str:
        if not isinstance(v, int) or isinstance(v, bool) or not 0 <= v <= MASK64:
            raise CanonicalError(f"{name}: {v!r} is not a u64")
        return f"{v:016x}"

    def parse(tok: str) -> object:
        if not HEX64.fullmatch(tok):
            raise CanonicalError(f"{name}: {tok!r} is not 16 lowercase hex digits")
        return int(tok, 16)

    def gen(rng: random.Random) -> object:
        return rng.choice([0, MASK64, rng.getrandbits(64)])

    return Codec(name, render, parse, gen)


def _hex128_codec(name: str) -> Codec:
    """A 16-byte BLOB as exactly 32 lowercase hex digits."""

    def render(v: object) -> str:
        if not isinstance(v, bytes) or len(v) != DIGEST_LEN:
            raise CanonicalError(f"{name}: {v!r} is not 16 bytes")
        return v.hex()

    def parse(tok: str) -> object:
        if not HEX128.fullmatch(tok):
            raise CanonicalError(f"{name}: {tok!r} is not 32 lowercase hex digits")
        return bytes.fromhex(tok)

    def gen(rng: random.Random) -> object:
        return rng.choice([bytes(DIGEST_LEN), b"\xff" * DIGEST_LEN, rng.randbytes(DIGEST_LEN)])

    return Codec(name, render, parse, gen)


def encode_str(s: str) -> str:
    """The string token of ``s``: UTF-8 bytes, unsafe ones as ``%XX``; the empty string is ``%``."""
    try:
        data = s.encode("utf-8")
    except UnicodeEncodeError as e:
        raise CanonicalError(f"string: {s!r} is not encodable as UTF-8: {e}") from None
    out = "".join(chr(b) if b in SAFE else f"%{b:02X}" for b in data)
    return out or EMPTY


def _decode_bytes(tok: str) -> bytes:
    """The bytes a non-empty string token encodes, refusing every non-canonical spelling."""
    out = bytearray()
    i = 0
    while i < len(tok):
        ch = tok[i]
        if ch == "%":
            esc = tok[i : i + 3]
            if not ESCAPE.fullmatch(esc):
                raise CanonicalError(f"string: bad escape {esc!r} in {tok!r}")
            b = int(esc[1:], 16)
            if b in SAFE:
                raise CanonicalError(f"string: {esc} escapes a safe byte in {tok!r}")
            out.append(b)
            i += 3
        elif ord(ch) in SAFE:
            out.append(ord(ch))
            i += 1
        else:
            raise CanonicalError(f"string: unescaped {ch!r} in {tok!r}")
    return bytes(out)


def decode_str(tok: str) -> str:
    """The string a token encodes (the inverse of :func:`encode_str`)."""
    if tok == EMPTY:
        return ""
    if not tok:
        raise CanonicalError("string: empty token (the empty string is '%')")
    try:
        return _decode_bytes(tok).decode("utf-8")
    except UnicodeDecodeError as e:
        raise CanonicalError(f"string: {tok!r} is not UTF-8: {e}") from None


_STRING_POOL = (
    "",
    "M2.demography",
    "resample",
    "person",
    "a b",
    "%",
    "100%",
    "%25",
    "é",
    "日本",
    "tab\there",
    "new\nline",
    "\\",
    '"q"',
    "kf/y000012/keyframe.arrow",
    "rehoboam/rh-snapshot/keyframe/v1",
    "~",
    "[1,2]",
    "x" * 40,
    "\U0001f600",
    "\u0000",
    "\xa0",
)


def _gen_str(rng: random.Random) -> object:
    """A string from the pool, or random code points outside the surrogates."""
    if rng.random() < 0.5:  # noqa: PLR2004
        return rng.choice(_STRING_POOL)
    points = [
        rng.choice([rng.randint(0, 0x7F), rng.randint(0x80, 0xD7FF), rng.randint(0xE000, 0x10FFFF)])
        for _ in range(rng.randint(1, 12))
    ]
    return "".join(map(chr, points))


def _str_codec(name: str) -> Codec:
    def render(v: object) -> str:
        if not isinstance(v, str):
            raise CanonicalError(f"{name}: {v!r} is not a string")
        return encode_str(v)

    return Codec(name, render, decode_str, _gen_str)


def _pattern_codec(name: str, pattern: re.Pattern[str]) -> Codec:
    """A short identifier matching ``pattern`` (role variables, clause names), written as is."""

    def render(v: object) -> str:
        if not isinstance(v, str) or not pattern.fullmatch(v):
            raise CanonicalError(f"{name}: {v!r} does not match {pattern.pattern}")
        return v

    def parse(tok: str) -> object:
        if not pattern.fullmatch(tok):
            raise CanonicalError(f"{name}: {tok!r} does not match {pattern.pattern}")
        return tok

    def gen(rng: random.Random) -> object:
        return rng.choice(
            ["A", "B", "S", "kin_1", "e1"] if pattern is IDENT else ["e1", "e2", "e3"]
        )

    return Codec(name, render, parse, gen)


def _tuple_codec(name: str, parts: Sequence[Codec]) -> Codec:
    """A ``/``-joined tuple of ``parts`` (a list item)."""

    def render(v: object) -> str:
        if not isinstance(v, tuple) or len(v) != len(parts):
            raise CanonicalError(f"{name}: {v!r} is not a {len(parts)}-tuple")
        return "/".join(c.render(x) for c, x in zip(parts, v, strict=True))

    def parse(tok: str) -> object:
        bits = tok.split("/")
        if len(bits) != len(parts):
            raise CanonicalError(f"{name}: {tok!r} has {len(bits)} parts, not {len(parts)}")
        return tuple(c.parse(b) for c, b in zip(parts, bits, strict=True))

    def gen(rng: random.Random) -> object:
        return tuple(c.gen(rng) for c in parts)

    return Codec(name, render, parse, gen)


Order = Callable[[tuple[object, ...]], str | None]


def _list_codec(
    name: str,
    item: Codec,
    order: Order,
    sort_index: tuple[int, ...] | None = None,
    finish: Callable[[tuple[object, ...]], tuple[object, ...]] | None = None,
) -> Codec:
    """``[item,item,...]``; ``order`` names what is wrong with a non-canonical item order.

    Generated items are sorted and deduplicated by the tuple components ``sort_index`` (when
    given), then passed through ``finish``.
    """

    def check(items: tuple[object, ...]) -> tuple[object, ...]:
        problem = order(items)
        if problem:
            raise CanonicalError(f"{name}: {problem}")
        return items

    def render(v: object) -> str:
        if not isinstance(v, tuple):
            raise CanonicalError(f"{name}: {v!r} is not a tuple")
        return "[" + ",".join(item.render(x) for x in check(v)) + "]"

    def parse(tok: str) -> object:
        if len(tok) < 2 or tok[0] != "[" or tok[-1] != "]":  # noqa: PLR2004
            raise CanonicalError(f"{name}: {tok!r} is not a bracketed list")
        inner = tok[1:-1]
        return check(() if inner == "" else tuple(item.parse(x) for x in inner.split(",")))

    def gen(rng: random.Random) -> object:
        items = [item.gen(rng) for _ in range(rng.choice([0, 1, 1, 2, 3]))]
        out = tuple(items) if sort_index is None else _canonical_order(items, sort_index)
        return finish(out) if finish else out

    return Codec(name, render, parse, gen)


def _sort_key(item: object, indices: tuple[int, ...]) -> tuple[object, ...]:
    """The components ``indices`` of a tuple item, a NULL (None) ordered before any value."""
    assert isinstance(item, tuple)
    return tuple((0, 0) if item[i] is None else (1, item[i]) for i in indices)


def _canonical_order(items: Sequence[object], indices: tuple[int, ...]) -> tuple[object, ...]:
    """Generated list items sorted and deduplicated by the components ``indices``."""
    seen: dict[tuple[object, ...], object] = {}
    for it in items:
        seen.setdefault(_sort_key(it, indices), it)
    return tuple(seen[k] for k in sorted(seen))  # type: ignore[type-var,unused-ignore]


def _any_order(_: tuple[object, ...]) -> str | None:
    return None


def _ascending(indices: tuple[int, ...], what: str) -> Order:
    """Items strictly ascending by the components ``indices`` (a NULL before any value)."""

    def order(items: tuple[object, ...]) -> str | None:
        keys = [_sort_key(it, indices) for it in items if isinstance(it, tuple)]
        for a, b in itertools.pairwise(keys):
            if not a < b:
                return f"{what} not strictly ascending: {a!r} then {b!r}"
        return None

    return order


def _events_order(items: tuple[object, ...]) -> str | None:
    """Clause names distinct, event ids strictly ascending (kernel order, 05.4 §2.3 item 2)."""
    names = [it[0] for it in items if isinstance(it, tuple)]
    if len(set(names)) != len(names):
        return f"clause names repeat: {names}"
    return _ascending((1,), "event ids")(items)


def _renamed_clauses(items: tuple[object, ...]) -> tuple[object, ...]:
    """Generated event items renamed e1, e2, ... in order, so that clause names never repeat."""
    return tuple((f"e{i + 1}", *it[1:]) for i, it in enumerate(items) if isinstance(it, tuple))


I64 = _int_codec("i64", -(1 << 63), (1 << 63) - 1)
I32 = _int_codec("i32", -(1 << 31), (1 << 31) - 1)
I16 = _int_codec("i16", -(1 << 15), (1 << 15) - 1)
U64 = _int_codec("u64", 0, MASK64)
U32 = _int_codec("u32", 0, (1 << 32) - 1)
U16 = _int_codec("u16", 0, (1 << 16) - 1)
U8 = _int_codec("u8", 0, (1 << 8) - 1)
OPT_U64 = _optional(U64)
OPT_U32 = _optional(U32)
OPT_U16 = _optional(U16)
OPT_U8 = _optional(U8)
HASH64 = _hex64_codec("hash64")
BLOB16 = _hex128_codec("blob16")
STR = _str_codec("string")
ROLE_VAR = _pattern_codec("role variable", IDENT)
CLAUSE_NAME = _pattern_codec("clause", CLAUSE)
# The DDL's enumerations (channel_a_v0.sql, channel_c_v0.sql; 05.1 §4.2 for the twelve phases).
PHASE = _enum_codec("phase", range(12))
INPUT_CLASS = _enum_codec("class", [0])
SECRECY = _enum_codec("secrecy", range(4))
ORIGIN = _enum_codec("origin", [0, 1, 3, 4])
ENTITY_KIND = _enum_codec(
    "entity_kind", range(6)
)  # 5 = artifact, reserved for S6 (envelope_v0.md §1)
EVENT_ROLE = _enum_codec("role", range(4))
EDGE_KIND = _enum_codec("edge kind", range(9))
MOTIVATED_BY = 3  # the one edge kind that may carry a motive (envelope_v0.md §5)
DETERMINISTIC_KINDS = frozenset(
    {0, 1, 6, 7, 8}
)  # contingent, triggered, player, oracle, director: evidence is 1
UNWRITTEN_KINDS = frozenset(
    {6, 7, 8}
)  # player, oracle, director: no writer until the input-event format exists (§5; R22 NIT-5)
WRITTEN_KINDS = 6  # a generated edge's kind is in range(WRITTEN_KINDS)
WITNESS = 2  # the one role that may repeat on an event (envelope_v0.md §1)
EVIDENCE_ONE = 256  # q8_8 raw bits of 1
MAX_CAST = 256  # the ordinal is a u8
HALF = 0.5
CAUSE_TOLD = _enum_codec("cause_told", range(2))
EVIDENCE = _int_codec("evidence", 0, 256)  # q8_8 raw bits in [0, 1]
LIFE_ORIGIN = _enum_codec("life origin", range(3))
SEX = _enum_codec("sex", range(2))
EXIT_KIND = _enum_codec("exit_kind", range(2))
DEATH_CAUSE = _enum_codec("death_cause", range(6))
SELECTION = _enum_codec("selection", range(3))
ON_COMPLETE = _enum_codec("on_complete", range(3))
STATUS = _enum_codec("status", [1])
PAYLOAD = _list_codec("payload", I64, _any_order)
EVENT_IDS = _list_codec("event ids", U64, _any_order)
ROLE_KEY = (
    2,
    0,
    1,
)  # the ordinal order: role code, then (entity_kind, entity_id) (envelope_v0.md §1)
ROLES = _list_codec(
    "roles",
    _tuple_codec("role", (ENTITY_KIND, U64, EVENT_ROLE)),
    _ascending(ROLE_KEY, "roles (by role code, then entity)"),
    ROLE_KEY,
)
SNAPSHOT_KEY = (0, 1, 2, 3, 4)  # (attr_id, entity_kind, entity_id, partner_kind, partner_id)
SNAPSHOTS = _list_codec(
    "snapshots",
    _tuple_codec("snapshot", (U16, ENTITY_KIND, U64, OPT_U8, OPT_U64, I32)),
    _ascending(SNAPSHOT_KEY, "snapshot keys"),
    SNAPSHOT_KEY,
)
CAUSES = _list_codec(
    "causes",
    _tuple_codec("cause", (U64, EDGE_KIND, OPT_U16, I16, EVIDENCE, CAUSE_TOLD)),
    _ascending((0,), "cause ids"),
    (0,),
)
BINDINGS = _list_codec(
    "bindings",
    _tuple_codec("binding", (ROLE_VAR, ENTITY_KIND, U64)),
    _ascending((0,), "roles"),
    (0,),
)
EVENTS = _list_codec(
    "events", _tuple_codec("event", (CLAUSE_NAME, U64)), _events_order, (1,), _renamed_clauses
)

Fields = tuple[tuple[str, Codec], ...]
HEADER: Fields = (("timeline_id", U32), ("seq", U64), ("tick", I64))
# Every record kind's fields after the header, in line order (canonical_v0.md §3).
KINDS: dict[str, Fields] = {
    "INPUT": (
        *HEADER,
        ("journal_seq", U64),
        ("class", INPUT_CLASS),
        ("input_kind", STR),
        ("scope", STR),
        ("scope_id", OPT_U64),
        ("producer", STR),
        ("payload_hash", BLOB16),
        ("row_digest", BLOB16),
    ),
    "EVENT": (
        *HEADER,
        ("event_id", U64),
        ("phase", PHASE),
        ("verb_id", U16),
        ("verb", STR),
        ("act_classes", U64),
        ("place_id", OPT_U32),
        ("magnitude", I32),
        ("tellability", U8),
        ("secrecy", SECRECY),
        ("origin", ORIGIN),
        ("payload", PAYLOAD),
        ("roles", ROLES),
        ("snapshots", SNAPSHOTS),
        ("causes", CAUSES),
    ),
    "LIFE_ENTRY": (
        *HEADER,
        ("person_id", U64),
        ("origin", LIFE_ORIGIN),
        ("sex", SEX),
        ("mother_id", U64),
        ("father_id", U64),
        ("birth_tick", I64),
    ),
    "LIFE_EXIT": (
        *HEADER,
        ("person_id", U64),
        ("exit_kind", EXIT_KIND),
        ("death_cause", DEATH_CAUSE),
        ("event_id", OPT_U64),
    ),
    "STEP_DIGEST": (
        *HEADER,
        ("living", U64),
        ("births", U32),
        ("deaths", U32),
        ("state_hash", HASH64),
        ("observation_hash", HASH64),
        ("narrative_hash", HASH64),
    ),
    "KEYFRAME": (
        *HEADER,
        ("keyframe_format", STR),
        ("keyframe_path", STR),
        ("state_hash", HASH64),
        ("journal_position", U64),
    ),
    "MATCH": (
        *HEADER,
        ("match_id", HASH64),
        ("pack", STR),
        ("pattern_id", STR),
        ("pattern_version", U16),
        ("selection", SELECTION),
        ("on_complete", ON_COMPLETE),
        ("status", STATUS),
        ("opened_tick", I64),
        ("closed_by_event_id", U64),
        ("closed_by_seq", U64),
        ("bindings", BINDINGS),
        ("events", EVENTS),
    ),
}
CHANNEL = {kind: ("C" if kind == "MATCH" else "A") for kind in KINDS}
CHANNELS = frozenset(CHANNEL.values())


@dataclass(frozen=True)
class Record:
    """One record: its kind and its field values in line order (lists as tuples)."""

    kind: str
    fields: tuple[object, ...]

    def __getitem__(self, name: str) -> object:
        """The value of field ``name``."""
        return self.fields[[n for n, _ in KINDS[self.kind]].index(name)]

    def replace(self, **values: object) -> Record:
        """A copy with the named fields replaced."""
        names = [n for n, _ in KINDS[self.kind]]
        fields = list(self.fields)
        for name, value in values.items():
            fields[names.index(name)] = value
        return Record(self.kind, tuple(fields))


def _check_match(rec: Record) -> str | None:
    """The MATCH invariants of channel_c_v0.sql R6 (canonical_v0.md §3)."""
    events = rec["events"]
    assert isinstance(events, tuple)
    ids = [e[1] for e in events if isinstance(e, tuple)]
    if not ids:
        return "a match binds at least one event"
    if rec["closed_by_event_id"] not in ids:
        return f"closed_by_event_id {rec['closed_by_event_id']} is not among the events {ids}"
    opened, tick = rec["opened_tick"], rec["tick"]
    assert isinstance(opened, int) and isinstance(tick, int)
    if opened > tick:
        return f"opened_tick {opened} is after the closing tick {tick}"
    return None


def _check_life_exit(rec: Record) -> str | None:
    """A death has a cause; an emigration has none and no DEATH event (channel_a_v0.sql)."""
    death = rec["exit_kind"] == 0
    if death and rec["death_cause"] == 0:
        return "a death (exit_kind 0) needs a death_cause other than 0 (none)"
    if not death and (rec["death_cause"] != 0 or rec["event_id"] is not None):
        return "an emigration (exit_kind 1) has death_cause 0 and no event_id"
    return None


def _check_cast(roles: tuple[object, ...]) -> str | None:
    """A cast of at most 256 rows, every role but witness at most once (envelope_v0.md §1)."""
    if len(roles) > MAX_CAST:
        return f"a cast of {len(roles)} rows; the ordinal is a u8 (at most {MAX_CAST})"
    seen: set[object] = set()
    for row in roles:
        assert isinstance(row, tuple)
        role = row[2]
        if role != WITNESS and role in seen:
            return f"role {role} filled twice; only witness ({WITNESS}) may repeat"
        seen.add(role)
    return None


def _check_edge(edge: tuple[object, ...], event_id: object) -> str | None:
    """The edge domains of envelope_v0.md §5 (REV-SPR-020 R8, R22 NIT-5).

    A cause earlier than its effect; a written kind; a motive on motivated_by only; weight 0 off
    it; evidence 1 on a deterministic kind: what `EventLog::append` refuses, the parser refuses.
    """
    cause, kind, motive, weight, evidence = edge[0], edge[1], edge[2], edge[3], edge[4]
    assert isinstance(cause, int) and isinstance(event_id, int)
    if cause >= event_id:
        return f"cause {edge!r}: cause {cause} is not earlier than event {event_id} (ForwardCause)"
    if kind in UNWRITTEN_KINDS:
        return f"cause {edge!r}: kind {kind} (player, oracle, director) has no writer in P1a (§5)"
    if motive is not None and kind != MOTIVATED_BY:
        return f"cause {edge!r}: a motive refines motivated_by (kind {MOTIVATED_BY}) only"
    if kind != MOTIVATED_BY and weight != 0:
        return f"cause {edge!r}: weight is 0 on every kind but motivated_by"
    if kind in DETERMINISTIC_KINDS and evidence != EVIDENCE_ONE:
        return f"cause {edge!r}: evidence is 1 ({EVIDENCE_ONE}) on a deterministic kind"
    return None


def _check_event(rec: Record) -> str | None:
    """The envelope's domains (envelope_v0.md §1, §4, §5), applied by render and parse."""
    roles, snapshots, causes = rec["roles"], rec["snapshots"], rec["causes"]
    assert isinstance(roles, tuple) and isinstance(snapshots, tuple) and isinstance(causes, tuple)
    problem = _check_cast(roles)
    if problem:
        return problem
    for snap in snapshots:
        assert isinstance(snap, tuple)
        if (snap[3] is None) != (snap[4] is None):
            return f"snapshot {snap!r}: partner_kind and partner_id are both NULL or both set"
    for edge in causes:
        assert isinstance(edge, tuple)
        problem = _check_edge(edge, rec["event_id"])
        if problem:
            return problem
    return None


# Record-level domain checks beyond the token grammar, applied by render and parse.
KIND_CHECKS: dict[str, Callable[[Record], str | None]] = {
    "EVENT": _check_event,
    "MATCH": _check_match,
    "LIFE_EXIT": _check_life_exit,
}


def _fix_match(rec: Record, rng: random.Random) -> Record:
    """Make a generated MATCH obey its invariants."""
    events = rec["events"]
    assert isinstance(events, tuple)
    if not events:
        events = (("e1", rng.getrandbits(64)),)
    ids = [e[1] for e in events if isinstance(e, tuple)]
    tick = rec["tick"]
    assert isinstance(tick, int)
    opened = rng.choice([tick, max(tick - rng.randint(0, 1 << 20), -(1 << 63)), -(1 << 63)])
    return rec.replace(events=events, closed_by_event_id=rng.choice(ids), opened_tick=opened)


def _fix_life_exit(rec: Record, rng: random.Random) -> Record:
    """Make a generated LIFE_EXIT obey its invariant."""
    if rec["exit_kind"] == 0:
        return rec.replace(death_cause=rng.randint(1, 5))
    return rec.replace(death_cause=0, event_id=None)


def _fix_event(rec: Record, rng: random.Random) -> Record:
    """Make a generated EVENT obey the envelope's domains, re-sorted (a partner moves a row)."""
    snapshots, causes = rec["snapshots"], rec["causes"]
    assert isinstance(snapshots, tuple) and isinstance(causes, tuple)
    fixed_snapshots = []
    for snap in snapshots:
        assert isinstance(snap, tuple)
        fixed = snap
        if (snap[3] is None) != (snap[4] is None):
            kinds = range(6)  # entity kinds, artifact reserved
            partner = (
                (None, None) if rng.random() < HALF else (rng.choice(kinds), rng.getrandbits(64))
            )
            fixed = (*snap[:3], *partner, snap[5])
        fixed_snapshots.append(fixed)
    event_id = rec["event_id"]
    assert isinstance(event_id, int)
    by_cause: dict[int, tuple[object, ...]] = {}
    for edge in causes:  # a cause earlier than the event, one edge per cause, ascending
        assert isinstance(edge, tuple) and isinstance(edge[0], int)
        if event_id == 0:
            break
        by_cause.setdefault(edge[0] % event_id, _fix_edge((edge[0] % event_id, *edge[1:])))
    fixed_causes = tuple(by_cause[c] for c in sorted(by_cause))
    roles = rec["roles"]
    assert isinstance(roles, tuple)
    seen: set[object] = set()
    fixed_roles = []
    for row in roles:
        assert isinstance(row, tuple)
        if row[2] == WITNESS or row[2] not in seen:
            seen.add(row[2])
            fixed_roles.append(row)
    return rec.replace(
        roles=tuple(fixed_roles),
        snapshots=_canonical_order(fixed_snapshots, SNAPSHOT_KEY),
        causes=fixed_causes,
    )


def _fix_edge(edge: tuple[object, ...]) -> tuple[object, ...]:
    """A generated edge within its kind's domains (:func:`_check_edge`)."""
    assert isinstance(edge[1], int)
    kind = edge[1] % WRITTEN_KINDS  # 6-8 have no writer (R22 NIT-5)
    motive = edge[2] if kind == MOTIVATED_BY else None
    weight = edge[3] if kind == MOTIVATED_BY else 0
    evidence = EVIDENCE_ONE if kind in DETERMINISTIC_KINDS else edge[4]
    return (edge[0], kind, motive, weight, evidence, edge[5])


KIND_FIXES: dict[str, Callable[[Record, random.Random], Record]] = {
    "EVENT": _fix_event,
    "MATCH": _fix_match,
    "LIFE_EXIT": _fix_life_exit,
}


def _checked(rec: Record) -> Record:
    check = KIND_CHECKS.get(rec.kind)
    problem = check(rec) if check else None
    if problem:
        raise CanonicalError(f"{rec.kind}: {problem}")
    return rec


def render(rec: Record) -> str:
    """The canonical line of ``rec``, without its LF terminator."""
    spec = KINDS.get(rec.kind)
    if spec is None:
        raise CanonicalError(f"unknown kind {rec.kind!r}")
    if len(rec.fields) != len(spec):
        raise CanonicalError(f"{rec.kind}: {len(rec.fields)} fields, expected {len(spec)}")
    _checked(rec)
    return " ".join([rec.kind, *(c.render(v) for (_, c), v in zip(spec, rec.fields, strict=True))])


def parse(line: str) -> Record:
    """The record of one canonical line (given without its LF), or :class:`CanonicalError`."""
    if not line or not line.isascii() or "\n" in line or "\r" in line:
        raise CanonicalError(f"line is empty, not ASCII or holds a line break: {line!r}")
    tokens = line.split(" ")
    spec = KINDS.get(tokens[0])
    if spec is None:
        raise CanonicalError(f"unknown kind {tokens[0]!r}")
    if len(tokens) - 1 != len(spec):
        raise CanonicalError(f"{tokens[0]}: {len(tokens) - 1} fields, expected {len(spec)}")
    fields = tuple(c.parse(t) for (_, c), t in zip(spec, tokens[1:], strict=True))
    return _checked(Record(tokens[0], fields))


def digest(lines: Iterable[str]) -> bytes:
    """``canonical_digest``: XXH3-128 over the lines, each terminated by LF, in the order given."""
    return xxh3_128(b"".join(line.encode("ascii") + b"\n" for line in lines))


def _str_bytes(s: str) -> bytes:
    b = s.encode("utf-8")
    return len(b).to_bytes(4, "little") + b


Binding = tuple[str, int, int]


def match_id(
    pack: str,
    pattern_id: str,
    version: int,
    event_ids: Sequence[int],
    bindings: Sequence[Binding],
) -> int:
    """Channel C's MatchId (channel_c_v0.sql R5): XXH3-64 over the pattern, events and bindings."""
    ordered = sorted(bindings, key=lambda b: b[0].encode("utf-8"))
    body = (
        D_MATCH
        + _str_bytes(pack)
        + _str_bytes(pattern_id)
        + version.to_bytes(2, "little")
        + len(event_ids).to_bytes(4, "little")
        + b"".join(e.to_bytes(8, "little") for e in event_ids)
        + len(ordered).to_bytes(4, "little")
        + b"".join(
            _str_bytes(role) + kind.to_bytes(1, "little") + eid.to_bytes(8, "little")
            for role, kind, eid in ordered
        )
    )
    return xxh3_64(body)


# --- Canonical files ----------------------------------------------------------------------------

HEADER_LINE = re.compile(
    r"#rehoboam/stream/canonical/v0 timeline=(?P<timeline>0|[1-9][0-9]*) channel=(?P<channel>[AC]) "
    r"registry_version=(?P<registry>[^ ]+) format=(?P<format>[^ ]+)"
)


def header_line(timeline: int, channel: str, registry_version: str, segment_format: str) -> str:
    """The header line of a canonical file (canonical_v0.md §1)."""
    return (
        f"#{FORMAT} timeline={timeline} channel={channel} "
        f"registry_version={encode_str(registry_version)} format={encode_str(segment_format)}"
    )


def _file_lines(data: bytes, where: str) -> list[str]:
    """The lines of a canonical file's bytes: ASCII, LF-terminated, no CR."""
    if not data or data[-1] != LF:
        raise CanonicalError(f"{where}: the file is empty or does not end with LF")
    if any(b >= ASCII_LIMIT for b in data):
        raise CanonicalError(f"{where}: the file holds a byte outside ASCII")
    if CR in data:
        raise CanonicalError(f"{where}: the file holds a carriage return")
    return data[:-1].decode("ascii").split("\n")


def _check_records(records: list[tuple[int, Record]], where: str, timeline: int, chan: str) -> None:
    """One timeline, one channel, contiguous seq and non-decreasing ticks (R3)."""
    for n, rec in records:
        if CHANNEL[rec.kind] != chan:
            raise CanonicalError(f"{where}:{n}: a {rec.kind} record in a channel-{chan} file")
        if rec["timeline_id"] != timeline:
            raise CanonicalError(f"{where}:{n}: timeline {rec['timeline_id']} is not {timeline}")
    for (n, a), (m, b) in itertools.pairwise(records):
        seq_a, seq_b, tick_a, tick_b = a["seq"], b["seq"], a["tick"], b["tick"]
        assert isinstance(seq_a, int) and isinstance(seq_b, int)
        assert isinstance(tick_a, int) and isinstance(tick_b, int)
        if seq_b != seq_a + 1:
            raise CanonicalError(f"{where}:{m}: seq {seq_b} does not follow {seq_a} (line {n})")
        if tick_b < tick_a:
            raise CanonicalError(f"{where}:{m}: tick {tick_b} is before {tick_a} (line {n})")


def digest_file(path: Path) -> bytes:
    """Check a canonical file (header, grammar, domains, order) and digest its record lines."""
    where = str(path)
    lines = _file_lines(path.read_bytes(), where)
    header = HEADER_LINE.fullmatch(lines[0])
    if header is None:
        raise CanonicalError(f"{where}:1: not a canonical header line: {lines[0]!r}")
    timeline, chan = int(header["timeline"]), header["channel"]
    records: list[tuple[int, Record]] = []
    for n, raw in enumerate(lines[1:], 2):
        if raw.startswith("#"):
            continue
        try:
            records.append((n, parse(raw)))
        except CanonicalError as e:
            raise CanonicalError(f"{where}:{n}: {e}") from None
    _check_records(records, where, timeline, chan)
    return digest(lines[n - 1] for n, _ in records)


# --- Self-test ---------------------------------------------------------------------------------


def generate(rng: random.Random, kind: str) -> Record:
    """A random record of ``kind`` with canonical list orders and in-domain values."""
    rec = Record(kind, tuple(c.gen(rng) for _, c in KINDS[kind]))
    fix = KIND_FIXES.get(kind)
    return fix(rec, rng) if fix else rec


# Lines the parser must refuse, with the reason (canonical_v0.md §5).
REFUSED: tuple[tuple[str, str], ...] = (
    ("LIFE_EXIT 0 1 0 7 0 1 ~ ", "a trailing space"),
    (" LIFE_EXIT 0 1 0 7 0 1 ~", "a leading space"),
    ("LIFE_EXIT 0 1  0 7 0 1 ~", "two spaces between tokens"),
    ("LIFE_EXIT 0 1 0 7 0 1 ~\r", "a carriage return"),
    ("LIFE_EXIT 0 01 0 7 0 1 ~", "a leading zero"),
    ("LIFE_EXIT 0 +1 0 7 0 1 ~", "a plus sign"),
    ("LIFE_EXIT 0 1 -0 7 0 1 ~", "minus zero"),
    ("LIFE_EXIT 0 1 0 7 0 6 ~", "a death_cause of 6"),
    ("LIFE_EXIT 0 1 0 7 0 0 ~", "a death without a cause"),
    ("LIFE_EXIT 0 1 0 7 1 1 ~", "an emigration with a death cause"),
    ("LIFE_EXIT 0 1 0 7 1 0 3", "an emigration with a DEATH event"),
    ("LIFE_EXIT 0 1 0 18446744073709551616 0 1 ~", "a u64 past its range"),
    ("LIFE_EXIT 0 1 0 100000000000000000000000 0 1 ~", "a 24-digit integer"),
    ("LIFE_EXIT 0 1 0 7 0 1", "a missing field"),
    ("LIFE_EXIT 0 1 0 7 0 1 ~ 5", "an extra field"),
    ("LIFE_EXIT 0 ~ 0 7 0 1 ~", "NULL in a field that is not optional"),
    ("LIFE_EXIT 0 1 9223372036854775808 7 0 1 ~", "an i64 past its range"),
    ("DEATH 0 1 0 7 0 1 ~", "an unknown kind"),
    ("life_exit 0 1 0 7 0 1 ~", "a lowercase kind"),
    ("STEP_DIGEST 0 0 0 3 0 0 0 0 0", "a hash written in decimal"),
    ("STEP_DIGEST 0 0 0 3 0 0 00000000000000AB 0000000000000000 0000000000000000", "uppercase hex"),
    ("STEP_DIGEST 0 0 0 3 0 0 000000000000000 0000000000000000 0000000000000000", "15 hex digits"),
    ("KEYFRAME 0 0 0 a%2fb kf 0000000000000000 0", "a lowercase hex escape"),
    ("KEYFRAME 0 0 0 %41 kf 0000000000000000 0", "an escaped safe byte"),
    ("KEYFRAME 0 0 0 %FF kf 0000000000000000 0", "an escape that is not UTF-8"),
    ("KEYFRAME 0 0 0 a%2 kf 0000000000000000 0", "a truncated escape"),
    ("KEYFRAME 0 0 0 a\tb kf 0000000000000000 0", "an unescaped control byte"),
    ("KEYFRAME 0 0 0 é kf 0000000000000000 0", "a non-ASCII line"),
    ("KEYFRAME 0 0 0  kf 0000000000000000 0", "an empty string token"),
    (
        "INPUT 0 0 0 0 1 k s ~ p 00000000000000000000000000000000 00000000000000000000000000000000",
        "an input class other than push",
    ),
    ("EVENT 0 0 0 1 12 0 DEATH 32768 ~ 0 128 0 0 [1] [] [] []", "a phase of 12"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 4 0 [1] [] [] []", "a secrecy of 4"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 2 [1] [] [] []", "the retired origin 2"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 1 [] [] []", "a payload that is not a list"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [1,] [] [] []", "a trailing comma"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [1/7] [] []", "a role tuple with two parts"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [6/7/1] [] []", "an entity kind of 6"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [1/7/0,1/8/0] [] []", "two actors"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [1/7/1,1/8/1] [] []", "two targets"),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] ["
        + ",".join(f"1/{i}/2" for i in range(MAX_CAST + 1))
        + "] [] []",
        "a cast of 257 rows",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [0/1/~/5/256/1]",
        "a triggered edge with a weight",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [0/1/~/0/0/1]",
        "a triggered edge with evidence 0",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [0/0/~/0/255/1]",
        "a contingent edge with evidence below 1",
    ),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [1/7/4] [] []", "a role code of 4"),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [1/7/1,1/8/0] [] []",
        "roles out of role-code order",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [1/8/2,1/7/2] [] []",
        "witnesses out of entity order",
    ),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [1/7/1,1/7/1] [] []", "a repeated cast row"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [0/9/~/0/256/1]", "an edge kind of 9"),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [0/6/~/0/256/1]",
        "an edge of kind 6 (player), which has no writer (R22 NIT-5)",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [1/1/~/0/256/1]",
        "a cause at the event's own id (a forward cause; R22 NIT-5)",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [0/1/~/0/256/1,2/1/~/0/256/1]",
        "a cause after the event (a forward cause)",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [0/1]",
        "a cause tuple with two parts (the P1.0 shape)",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [0/1/2/0/256/1]",
        "a motive on a triggered edge",
    ),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [0/3/~/0/257/1]", "an evidence of 257"),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [0/3/~/-32769/256/1]",
        "a weight below i16",
    ),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [0/1/~/0/256/2]", "a cause_told of 2"),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [3/1/~/0/256/1,2/1/~/0/256/1]",
        "causes out of order",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [] [2/1/~/0/256/1,2/1/~/0/256/1]",
        "a repeated cause",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [25/1/7/~/8/100] []",
        "a partner id without a kind",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [25/1/7/1/~/100] []",
        "a partner kind without an id",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [25/6/7/~/~/100] []",
        "a snapshot entity kind of 6",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [25/1/8/~/~/100,25/1/7/~/~/100] []",
        "snapshots out of order",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [25/1/7/1/8/100,25/1/7/~/~/100] []",
        "a snapshot with a partner before the same key without one",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [25/1/7/~/~/100,25/1/7/~/~/100] []",
        "a repeated snapshot",
    ),
    (
        "EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [25/1/7/~/~/2147483648] []",
        "a snapshot value past i32",
    ),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [ ] [] [] []", "a space inside a list"),
    (
        "MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 1 1 [B/1/1,A/1/2] [e1/1]",
        "bindings out of order",
    ),
    (
        "MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 1 1 [1/1/1] [e1/1]",
        "a role that is not an identifier",
    ),
    ("MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 1 1 [A/6/1] [e1/1]", "a binding to entity kind 6"),
    ("MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 2 1 [] [e1/2,e2/1]", "event ids not ascending"),
    ("MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 2 1 [] [e1/1,e1/2]", "a repeated clause"),
    ("MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 1 1 [] [x1/1]", "a clause not named eN"),
    ("MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 1 1 [] []", "a match with no event"),
    (
        "MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 2 1 [] [e1/1]",
        "closed_by_event_id not among the events",
    ),
    (
        "MATCH 0 0 0 0000000000000000 p q 1 0 0 1 1 1 1 [] [e1/1]",
        "opened_tick after the closing tick",
    ),
    ("MATCH 0 0 0 0000000000000000 p q 1 3 0 1 0 1 1 [] [e1/1]", "a selection of 3"),
    ("MATCH 0 0 0 0000000000000000 p q 1 0 3 1 0 1 1 [] [e1/1]", "an on_complete of 3"),
    (
        "MATCH 0 0 0 0000000000000000 p q 1 0 0 0 0 1 1 [] [e1/1]",
        "a partial match (status 0) in v0",
    ),
)
ROUND_TRIP_RECORDS = 400
VALID_FILE = (
    "#rehoboam/stream/canonical/v0 timeline=0 channel=A registry_version=0.2.0 "
    "format=rehoboam/stream/channel-a/v0\n"
    "# a comment\n"
    "LIFE_ENTRY 0 5 0 7 1 0 0 0 -100\n"
    "LIFE_EXIT 0 6 1440 7 0 1 ~\n"
)
# Files `--digest` must refuse, with the reason (canonical_v0.md §1).
REFUSED_FILES: tuple[tuple[bytes, str], ...] = (
    (VALID_FILE.replace("\n", "\r\n").encode(), "CRLF line ends"),
    (VALID_FILE.encode()[:-1], "a missing final LF"),
    (VALID_FILE.encode() + b"\r", "a lone CR"),
    (VALID_FILE.encode().replace(b"# a comment", b"# caf\xc3\xa9"), "a non-ASCII comment"),
    (VALID_FILE.encode().split(b"\n", 1)[1], "a missing header"),
    (
        VALID_FILE.encode().replace(b"channel=A", b"channel=C"),
        "channel-A records in a channel-C file",
    ),
    (VALID_FILE.encode().replace(b"LIFE_EXIT 0 6", b"LIFE_EXIT 0 7"), "a gap in seq"),
    (VALID_FILE.encode().replace(b"LIFE_EXIT 0 6", b"LIFE_EXIT 1 6"), "a second timeline"),
    (VALID_FILE.encode().replace(b"LIFE_EXIT 0 6 1440", b"LIFE_EXIT 0 6 -1"), "a tick going back"),
    (
        VALID_FILE.encode().replace(
            b"LIFE_EXIT 0 6 1440 7 0 1 ~",
            b"MATCH 0 6 1440 0000000000000000 p q 1 0 0 1 0 1 1 [] [e1/1]",
        ),
        "a MATCH in a channel-A file",
    ),
    (b"", "an empty file"),
)


def _round_trip(rng: random.Random) -> list[str]:
    """Property: parse(render(r)) == r and render is idempotent, for records of every kind."""
    failures = []
    for kind in KINDS:
        for _ in range(ROUND_TRIP_RECORDS):
            rec = generate(rng, kind)
            line = render(rec)
            back = parse(line)
            if back != rec or render(back) != line or "  " in line or not line.isascii():
                failures.append(f"round trip failed for {kind}: {line!r} -> {back!r}")
                break
    return failures


def _refusals() -> list[str]:
    """Every planted non-canonical line raises CanonicalError."""
    failures = []
    for line, why in REFUSED:
        try:
            parse(line)
        except CanonicalError:
            continue
        failures.append(f"parser accepted {why}: {line!r}")
    return failures


def _file_refusals() -> list[str]:
    """``digest_file`` accepts the valid file and refuses every planted one."""
    failures = []
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "canonical.txt"
        path.write_bytes(VALID_FILE.encode())
        want = digest(line for line in VALID_FILE.split("\n") if line and not line.startswith("#"))
        if digest_file(path) != want:
            failures.append("digest_file: the valid file's digest is not its record lines' digest")
        for data, why in REFUSED_FILES:
            path.write_bytes(data)
            try:
                digest_file(path)
            except CanonicalError:
                continue
            failures.append(f"digest_file accepted {why}")
    return failures


def _known_answers() -> list[str]:
    """Fixed values the grammar pins: the empty digest, string escapes, a lone surrogate."""
    failures = []
    if digest([]).hex() != "99aa06d3014798d86001c324468d497f":
        failures.append("digest of no lines is not XXH3-128 of the empty string")
    if encode_str("") != EMPTY or decode_str(EMPTY) != "" or encode_str("%") != "%25":
        failures.append("string escaping of '' or '%'")
    if encode_str("a b/é") != "a%20b/%C3%A9":
        failures.append(f"string escaping: {encode_str('a b/é')!r}")
    try:
        render(Record("KEYFRAME", (0, 0, 0, "\ud800", "kf", 0, 0)))
        failures.append("render accepted a lone surrogate")
    except CanonicalError:
        pass
    return failures


def self_test() -> list[str]:
    """Return the failures of the self-test (empty when it passes)."""
    rng = random.Random(0x5013)
    return [*_known_answers(), *_round_trip(rng), *_refusals(), *_file_refusals()]


# --- The golden file --------------------------------------------------------------------------

D_TEST = b"\x00" * DIGEST_LEN
DEATH, EXTINCTION = 0, 1
DEATH_CLASS = 1 << 15
ACTOR, TARGET = 0, 1
TRIGGERED = 1
RETALIATION_FOR = 0  # [motive_edges] index
STRESS_ATTR = 25  # an [attributes] index (illustrative; the segment's dictionary names it)
ONE = 256  # q8_8 raw bits of 1
# The P1.0 edge in its typed shape: a deterministic link, no motive, weight 0, evidence 1, told.
LINK = (None, 0, ONE, 1)
A_YEAR = (-10_368_000, -(1 << 40))
# life_ends matches of the hand-written segment, in R4 order:
# (seq, tick, closing event id, its channel-A seq, the deceased).
LIFE_ENDS_VECTORS: tuple[tuple[int, int, int, int, int], ...] = (
    (0, 0, 0, 1, 0x1111),
    (2, 216_000, 3, 9, 0x2222),
)
BETRAYAL_BINDINGS: tuple[Binding, ...] = (("A", 1, 0x2222), ("B", 1, 0x3333))


def _event(
    seq: int, tick: int, eid: int, verb: tuple[int, str, int], rest: tuple[object, ...]
) -> Record:
    """An EVENT record: ``verb`` is (verb_id, name, tellability); ``rest`` follows the verb."""
    verb_id, name, tellability = verb
    phase, place, magnitude, secrecy, origin, payload, roles, snapshots, causes = rest
    fields = (0, seq, tick, eid, phase, verb_id, name, DEATH_CLASS if verb_id <= 1 else 1 << 4)
    return Record(
        "EVENT",
        (
            *fields,
            place,
            magnitude,
            tellability,
            secrecy,
            origin,
            payload,
            roles,
            snapshots,
            causes,
        ),
    )


def _hand_written_a() -> list[Record]:
    """Channel-A records of one synthetic instant sequence in R4 order, with the grammar's edges."""
    d1 = (4, None, 0, 0, 0, (1,), ((1, 0x1111, TARGET),), (), ())
    decree = (
        4,
        12,
        -5,
        3,
        1,
        (),
        ((1, 0x2222, ACTOR), (3, 9, TARGET)),
        (),
        ((0, TRIGGERED, *LINK),),
    )
    extinction = (
        9,
        None,
        0,
        0,
        0,
        (-1, 0, (1 << 63) - 1, -(1 << 63)),
        (),
        (),
        ((0, 0, *LINK), (1, 5, None, 0, 128, 0)),
    )
    # A murder: the culprit is the actor, a stress snapshot of the culprit, a RETALIATION_FOR motive
    # edge to the decree (weight 0.5, margin 0.25) and a triggered edge to the first death.
    d2 = (
        4,
        None,
        0,
        0,
        0,
        (2,),
        ((1, 0x3333, ACTOR), (1, 0x2222, TARGET)),
        ((STRESS_ATTR, 1, 0x3333, None, None, 0x3F00),),
        ((0, TRIGGERED, *LINK), (1, MOTIVATED_BY, RETALIATION_FOR, 128, 64, 1)),
    )
    return [
        Record(
            "INPUT", (0, 0, 0, 0, 0, "resample", "person", 0x1111, "script", D_TEST, b"\x01" * 16)
        ),
        _event(1, 0, 0, (DEATH, "DEATH", 128), d1),
        Record("LIFE_ENTRY", (0, 2, 0, 0x1111, 1, 0, 0, 0, A_YEAR[0])),
        Record("LIFE_EXIT", (0, 3, 0, 0x1111, 0, 1, 0)),
        Record("LIFE_ENTRY", (0, 4, 0, 0x2222, 1, 1, 0, 0, A_YEAR[1])),
        Record("STEP_DIGEST", (0, 5, 0, 1, 0, 1, 0x0123_4567_89AB_CDEF, 0, MASK64)),
        Record(
            "INPUT",
            (0, 6, 43_200, 1, 0, "ForkOverride", "none", None, "Player", b"\xff" * 16, D_TEST),
        ),
        _event(7, 43_200, 1, (7, "ISSUE_DECREE", 96), decree),
        _event(8, 43_200, 2, (EXTINCTION, "EXTINCTION", 255), extinction),
        _event(9, 216_000, 3, (DEATH, "DEATH", 128), d2),
        Record("LIFE_EXIT", (0, 10, 216_000, 0x2222, 0, 2, 3)),
        Record("LIFE_EXIT", (0, 11, 216_000, 0x3333, 1, 0, None)),
        Record("STEP_DIGEST", (0, 12, 216_000, 0, 0, 1, 1, 2, 3)),
        Record(
            "KEYFRAME",
            (
                0,
                13,
                216_000,
                "rehoboam/rh-snapshot/keyframe/v1",
                "kf/y000000/t000216000.arrow",
                1,
                2,
            ),
        ),
        Record("KEYFRAME", (0, 14, 259_200, "", 'a b%é/日本\t\n"\\', MASK64, MASK64)),
    ]


def _match(
    header: tuple[int, int],
    pattern: tuple[str, str, int],
    closing: tuple[int, int, int, int, int],
    bindings: tuple[Binding, ...],
    events: tuple[tuple[str, int], ...],
) -> Record:
    """A MATCH record with its content-addressed id.

    ``header`` is (seq, tick); ``closing`` is (selection, on_complete, opened_tick, closing event
    id, its channel-A seq).
    """
    seq, tick = header
    pack, pid, version = pattern
    selection, on_complete, opened, event, a_seq = closing
    mid = match_id(pack, pid, version, [e for _, e in events], bindings)
    fields = (
        0,
        seq,
        tick,
        mid,
        pack,
        pid,
        version,
        selection,
        on_complete,
        1,
        opened,
        event,
        a_seq,
    )
    return Record("MATCH", (*fields, bindings, events))


def _hand_written_c() -> list[Record]:
    """The matches of :func:`_hand_written_a` in R4 order: by closing seq, then pack."""
    life_ends = ("solitary_life", "life_ends", 1)
    out = {
        seq: _match(
            (seq, tick),
            life_ends,
            (0, 0, tick, event, a_seq),
            (("A", 1, person),),
            (("e1", event),),
        )
        for seq, tick, event, a_seq, person in LIFE_ENDS_VECTORS
    }
    betrayal = ("betrayal", "trust_then_betrayal", 3)
    events = (("e1", 1), ("e2", 3))
    out[1] = _match((1, 216_000), betrayal, (2, 2, 43_200, 3, 9), BETRAYAL_BINDINGS, events)
    return [out[k] for k in sorted(out)]


MATCH_ID_VECTORS: tuple[tuple[str, str, int, tuple[int, ...], tuple[Binding, ...]], ...] = (
    ("solitary_life", "life_ends", 1, (0,), (("A", 1, 0x1111),)),
    ("solitary_life", "life_ends", 1, (3,), (("A", 1, 0x2222),)),
    ("solitary_life", "life_ends", 1, (3,), (("A", 1, 0x3333),)),
    ("betrayal", "trust_then_betrayal", 3, (1, 3), BETRAYAL_BINDINGS),
    ("", "", 0, (), ()),
    ("pé", "q", 65535, (MASK64,), (("Z", 4, MASK64), ("a", 0, 0))),
)
GENERATED_LINES = 24


def _block(head: str, records: Sequence[Record]) -> list[str]:
    lines = [render(r) for r in records]
    return [f"{head} {len(lines)}", *lines, f"digest {digest(lines).hex()}"]


def render_golden() -> str:
    """The golden file's full text."""
    rng = random.Random(2026)
    a_kinds = [k for k in KINDS if CHANNEL[k] == "A"]
    header = [
        "# World Stream canonical text form v0: golden lines, digests and MatchId vectors.",
        "# GENERATED by specs/stream/canonical_v0.py; do not edit. Grammar: canonical_v0.md.",
        "# `segment <timeline> <channel> <sim_year> <n>`: n canonical lines of one segment in R4",
        "# order, then `digest <xxh3-128, 32 lowercase hex>`: canonical_digest (each line + LF).",
        "# `lines <n>`: n generated lines in no particular order, then their digest the same way.",
        "# The generated lines come from CPython 3.12's `random` with a fixed seed; another",
        "# interpreter may generate different lines, which is why they are committed here.",
        "# `match_id <pack> <pattern_id> <version> <event ids> <bindings> <xxh3-64>`: R5 vectors.",
    ]
    body = [
        *_block("segment 0 A 0", _hand_written_a()),
        *_block("segment 0 C 0", _hand_written_c()),
        *_block("lines", [generate(rng, rng.choice(a_kinds)) for _ in range(GENERATED_LINES)]),
        *_block("lines", [generate(rng, "MATCH") for _ in range(GENERATED_LINES // 2)]),
        *_block("segment 3 A 2", []),
    ]
    vectors = [
        " ".join(
            [
                "match_id",
                encode_str(pack),
                encode_str(pid),
                str(v),
                EVENT_IDS.render(ids),
                BINDINGS.render(bindings),
                f"{match_id(pack, pid, v, ids, bindings):016x}",
            ]
        )
        for pack, pid, v, ids, bindings in MATCH_ID_VECTORS
    ]
    return "\n".join([*header, *body, *vectors]) + "\n"


def _check_golden(golden: Path, text: str) -> int:
    """Compare the committed golden file with the reference's text."""
    try:
        current = golden.read_text(encoding="utf-8")
    except OSError as e:
        print(f"STALE: cannot read {golden}: {e}", file=sys.stderr)
        return EXIT_STALE
    if current != text:
        print(f"STALE: {golden} differs from the reference", file=sys.stderr)
        return EXIT_STALE
    print(f"OK: {golden} matches the reference ({text.count(chr(10))} lines)")
    return EXIT_OK


def _digest_command(path: Path) -> int:
    """Print the digest of a canonical file, or say what is not canonical in it."""
    try:
        print(digest_file(path).hex())
    except CanonicalError as e:
        print(f"NOT CANONICAL: {e}", file=sys.stderr)
        return EXIT_STALE
    return EXIT_OK


def main(argv: list[str] | None = None) -> int:
    """Self-test, then write or check the golden file, or digest a canonical file."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the file to write or check")
    parser.add_argument("--digest", type=Path, help="print the digest of this canonical file")
    args = parser.parse_args(argv)
    failures = self_test()
    for f in failures:
        print(f"SELF-TEST FAIL: {f}", file=sys.stderr)
    if failures:
        return EXIT_SELFTEST
    print(
        f"self-test: {len(KINDS)} kinds x {ROUND_TRIP_RECORDS} records round-trip; "
        f"{len(REFUSED)} planted lines and {len(REFUSED_FILES)} planted files refused"
    )
    if args.digest is not None:
        return _digest_command(args.digest)
    text = render_golden()
    if args.check:
        return _check_golden(args.golden, text)
    args.golden.parent.mkdir(parents=True, exist_ok=True)
    args.golden.write_text(text, encoding="utf-8")
    print(f"wrote {args.golden}")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
