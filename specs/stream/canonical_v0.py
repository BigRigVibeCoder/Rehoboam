"""Reference printer and parser of the World Stream canonical text form v0 (SPR-013; PR #37 Q11).

The canonical text form is the lossless one-line text form of every channel-A record kind (INPUT,
EVENT, LIFE_ENTRY, LIFE_EXIT, STEP_DIGEST, KEYFRAME) and of channel C's MATCH. Its grammar is
``specs/stream/canonical_v0.md``; ``segment.canonical_digest`` (channel_a_v0.sql R7,
channel_c_v0.sql R8) is the XXH3-128 of a segment's lines in ``seq`` order. This file is the
independent reference ``rh-stream`` (SPR-013 T-09) must agree with, as ``bin/gen_hash_golden.py``
is for ``rh-snapshot``:

- :func:`render` and :func:`parse` print and read one record; :func:`digest` hashes lines;
- :func:`match_id` is channel C's content-addressed MatchId (channel_c_v0.sql R5);
- the self-test is a property round trip over generated records of every kind, idempotence of the
  printer, and planted non-canonical lines that the parser must refuse;
- the golden file ``specs/stream/golden/canonical_v0.txt`` holds hand-written and generated lines
  with their segment digests and ``match_id`` vectors; the Rust printer, parser and digest must
  reproduce every line bit for bit.

XXH3 comes from ``bin/gen_hash_golden.py``, the repository's standard-library XXH3-64/128
reference (seed 0, default secret), loaded by path so that ``python -I`` works.

Usage:  python -I specs/stream/canonical_v0.py            (self-test, then rewrite the golden file)
        python -I specs/stream/canonical_v0.py --check    (self-test; exit 1 if the golden differs)
        python -I specs/stream/canonical_v0.py --check --golden PATH   (check another file; the
                                                           positive control uses a stale fixture)
        python -I specs/stream/canonical_v0.py --digest FILE   (parse a canonical file and print the
                                                           digest of its record lines; `#` lines are
                                                           not records and not hashed)
Exit:   0 ok, 1 stale golden file or a file that does not parse, 2 the self-test failed.
Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import importlib.util
import itertools
import random
import re
import sys
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
# The bytes a string token writes unescaped (canonical_v0.md §2.5). Everything else is %XX.
SAFE = frozenset(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-/:+@")
INT = re.compile(r"0|-?[1-9][0-9]*")
HEX64 = re.compile(r"[0-9a-f]{16}")
HEX128 = re.compile(r"[0-9a-f]{32}")
ESCAPE = re.compile(r"%[0-9A-F]{2}")
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
CLAUSE = re.compile(r"e[0-9]+")
NULL = "~"
EMPTY = "%"


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
    """A token, line or value that is not in the canonical form."""


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
        if not INT.fullmatch(tok):
            raise CanonicalError(f"{name}: {tok!r} is not a canonical integer")
        v = int(tok)
        if not lo <= v <= hi:
            raise CanonicalError(f"{name}: {v} is outside [{lo}, {hi}]")
        return v

    def gen(rng: random.Random) -> object:
        return rng.choice([lo, hi, 0 if lo <= 0 <= hi else lo, rng.randint(lo, hi)])

    return Codec(name, render, parse, gen)


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
    out = "".join(chr(b) if b in SAFE else f"%{b:02X}" for b in s.encode("utf-8"))
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


def _list_codec(
    name: str,
    item: Codec,
    order: Order,
    sort_index: int | None = None,
    finish: Callable[[tuple[object, ...]], tuple[object, ...]] | None = None,
) -> Codec:
    """``[item,item,...]``; ``order`` names what is wrong with a non-canonical item order.

    Generated items are sorted and deduplicated by tuple component ``sort_index`` (when given),
    then passed through ``finish``.
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


def _canonical_order(items: list[object], sort_index: int) -> tuple[object, ...]:
    """Generated list items sorted and deduplicated by component ``sort_index``."""
    seen: dict[object, object] = {}
    for it in items:
        assert isinstance(it, tuple)
        seen.setdefault(it[sort_index], it)
    return tuple(seen[k] for k in sorted(seen))  # type: ignore[type-var,unused-ignore]


def _any_order(_: tuple[object, ...]) -> str | None:
    return None


Order = Callable[[tuple[object, ...]], str | None]


def _ascending(index: int, what: str) -> Order:
    """Items strictly ascending by component ``index``."""

    def order(items: tuple[object, ...]) -> str | None:
        keys = [it[index] for it in items if isinstance(it, tuple)]
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
    return _ascending(1, "event ids")(items)


def _renamed_clauses(items: tuple[object, ...]) -> tuple[object, ...]:
    """Generated event items renamed e1, e2, ... in order, so that clause names never repeat."""
    return tuple((f"e{i + 1}", *it[1:]) for i, it in enumerate(items) if isinstance(it, tuple))


I64 = _int_codec("i64", -(1 << 63), (1 << 63) - 1)
I32 = _int_codec("i32", -(1 << 31), (1 << 31) - 1)
U64 = _int_codec("u64", 0, MASK64)
U32 = _int_codec("u32", 0, (1 << 32) - 1)
U16 = _int_codec("u16", 0, (1 << 16) - 1)
U8 = _int_codec("u8", 0, (1 << 8) - 1)
OPT_U64 = _optional(U64)
OPT_U32 = _optional(U32)
HASH64 = _hex64_codec("hash64")
BLOB16 = _hex128_codec("blob16")
STR = _str_codec("string")
ROLE_VAR = _pattern_codec("role variable", IDENT)
CLAUSE_NAME = _pattern_codec("clause", CLAUSE)
PAYLOAD = _list_codec("payload", I64, _any_order)
EVENT_IDS = _list_codec("event ids", U64, _any_order)
ROLES = _list_codec("roles", _tuple_codec("role", (U8, U64, U8)), _any_order)
CAUSES = _list_codec("causes", _tuple_codec("cause", (U64, U8)), _ascending(0, "cause ids"), 0)
BINDINGS = _list_codec(
    "bindings", _tuple_codec("binding", (ROLE_VAR, U8, U64)), _ascending(0, "roles"), 0
)
EVENTS = _list_codec(
    "events", _tuple_codec("event", (CLAUSE_NAME, U64)), _events_order, 1, _renamed_clauses
)

Fields = tuple[tuple[str, Codec], ...]
HEADER: Fields = (("timeline_id", U32), ("seq", U64), ("tick", I64))
# Every record kind's fields after the header, in line order (canonical_v0.md §3).
KINDS: dict[str, Fields] = {
    "INPUT": (
        *HEADER,
        ("journal_seq", U64),
        ("class", U8),
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
        ("phase", U8),
        ("verb_id", U16),
        ("verb", STR),
        ("act_classes", U64),
        ("place_id", OPT_U32),
        ("magnitude", I32),
        ("tellability", U8),
        ("secrecy", U8),
        ("origin", U8),
        ("payload", PAYLOAD),
        ("roles", ROLES),
        ("causes", CAUSES),
    ),
    "LIFE_ENTRY": (
        *HEADER,
        ("person_id", U64),
        ("origin", U8),
        ("sex", U8),
        ("mother_id", U64),
        ("father_id", U64),
        ("birth_tick", I64),
    ),
    "LIFE_EXIT": (
        *HEADER,
        ("person_id", U64),
        ("exit_kind", U8),
        ("death_cause", U8),
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
        ("selection", U8),
        ("on_complete", U8),
        ("status", U8),
        ("opened_tick", I64),
        ("closed_by_event_id", U64),
        ("closed_by_seq", U64),
        ("bindings", BINDINGS),
        ("events", EVENTS),
    ),
}
CHANNEL = {kind: ("C" if kind == "MATCH" else "A") for kind in KINDS}


@dataclass(frozen=True)
class Record:
    """One record: its kind and its field values in line order (lists as tuples)."""

    kind: str
    fields: tuple[object, ...]

    def __getitem__(self, name: str) -> object:
        """The value of field ``name``."""
        return self.fields[[n for n, _ in KINDS[self.kind]].index(name)]


def render(rec: Record) -> str:
    """The canonical line of ``rec``, without its LF terminator."""
    spec = KINDS.get(rec.kind)
    if spec is None:
        raise CanonicalError(f"unknown kind {rec.kind!r}")
    if len(rec.fields) != len(spec):
        raise CanonicalError(f"{rec.kind}: {len(rec.fields)} fields, expected {len(spec)}")
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
    return Record(tokens[0], tuple(c.parse(t) for (_, c), t in zip(spec, tokens[1:], strict=True)))


def digest(lines: Iterable[str]) -> bytes:
    """``canonical_digest``: XXH3-128 over the lines, each terminated by LF, in the order given."""
    return xxh3_128(b"".join(line.encode("ascii") + b"\n" for line in lines))


def _str_bytes(s: str) -> bytes:
    b = s.encode("utf-8")
    return len(b).to_bytes(4, "little") + b


def match_id(pack: str, pattern_id: str, version: int, event_ids: Sequence[int]) -> int:
    """Channel C's MatchId (channel_c_v0.sql R5): XXH3-64 over the pattern and its event tuple."""
    body = (
        D_MATCH
        + _str_bytes(pack)
        + _str_bytes(pattern_id)
        + version.to_bytes(2, "little")
        + len(event_ids).to_bytes(4, "little")
        + b"".join(e.to_bytes(8, "little") for e in event_ids)
    )
    return xxh3_64(body)


# --- Self-test ---------------------------------------------------------------------------------


def generate(rng: random.Random, kind: str) -> Record:
    """A random record of ``kind`` with canonical list orders."""
    return Record(kind, tuple(c.gen(rng) for _, c in KINDS[kind]))


# Lines the parser must refuse, with the reason (canonical_v0.md §5).
REFUSED: tuple[tuple[str, str], ...] = (
    ("LIFE_EXIT 0 1 0 7 0 1 ~ ", "a trailing space"),
    (" LIFE_EXIT 0 1 0 7 0 1 ~", "a leading space"),
    ("LIFE_EXIT 0 1  0 7 0 1 ~", "two spaces between tokens"),
    ("LIFE_EXIT 0 1 0 7 0 1 ~\r", "a carriage return"),
    ("LIFE_EXIT 0 01 0 7 0 1 ~", "a leading zero"),
    ("LIFE_EXIT 0 +1 0 7 0 1 ~", "a plus sign"),
    ("LIFE_EXIT 0 1 -0 7 0 1 ~", "minus zero"),
    ("LIFE_EXIT 0 1 0 7 0 256 ~", "a u8 of 256"),
    ("LIFE_EXIT 0 1 0 7 0 1 18446744073709551616", "a u64 past its range"),
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
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 1 [] []", "a payload that is not a list"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [1,] [] []", "a trailing comma"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [1/7] []", "a role tuple with two parts"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [3/1,2/1]", "causes out of order"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [] [] [2/1,2/1]", "a repeated cause"),
    ("EVENT 0 0 0 1 4 0 DEATH 32768 ~ 0 128 0 0 [ ] [] []", "a space inside a list"),
    (
        "MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 1 1 [B/1/1,A/1/2] [e1/1]",
        "bindings out of order",
    ),
    (
        "MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 1 1 [1/1/1] [e1/1]",
        "a role that is not an identifier",
    ),
    ("MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 1 1 [] [e1/2,e2/1]", "event ids not ascending"),
    ("MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 1 1 [] [e1/1,e1/2]", "a repeated clause"),
    ("MATCH 0 0 0 0000000000000000 p q 1 0 0 1 0 1 1 [] [x1/1]", "a clause not named eN"),
)
ROUND_TRIP_RECORDS = 400


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


def _known_answers() -> list[str]:
    """Fixed values the grammar pins: the empty digest, string escapes, the MatchId of a vector."""
    failures = []
    if digest([]).hex() != "99aa06d3014798d86001c324468d497f":
        failures.append("digest of no lines is not XXH3-128 of the empty string")
    if encode_str("") != EMPTY or decode_str(EMPTY) != "" or encode_str("%") != "%25":
        failures.append("string escaping of '' or '%'")
    if encode_str("a b/é") != "a%20b/%C3%A9":
        failures.append(f"string escaping: {encode_str('a b/é')!r}")
    return failures


def self_test() -> list[str]:
    """Return the failures of the self-test (empty when it passes)."""
    rng = random.Random(0x5013)
    return [*_known_answers(), *_round_trip(rng), *_refusals()]


# --- The golden file --------------------------------------------------------------------------

D_TEST = b"\x00" * DIGEST_LEN
DEATH, EXTINCTION = 0, 1
DEATH_CLASS = 1 << 15
TARGET = 1
TRIGGERED = 1
# life_ends matches of the hand-written segment:
# (seq, tick, closing event id, its channel-A seq, the deceased).
LIFE_ENDS_VECTORS: tuple[tuple[int, int, int, int, int], ...] = (
    (0, 0, 0, 1, 0x1111),
    (1, 216_000, 3, 9, 0x2222),
)


def _hand_written_a() -> list[Record]:
    """Channel-A records of one synthetic instant sequence, with the grammar's edge cases."""
    return [
        Record(
            "INPUT",
            (0, 0, 0, 0, 0, "resample", "person", 0x1111, "script", D_TEST, b"\x01" * DIGEST_LEN),
        ),
        Record(
            "EVENT",
            (
                0,
                1,
                0,
                0,
                4,
                DEATH,
                "DEATH",
                DEATH_CLASS,
                None,
                0,
                128,
                0,
                0,
                (1,),
                ((1, 0x1111, TARGET),),
                (),
            ),
        ),
        Record("LIFE_ENTRY", (0, 2, 0, 0x1111, 1, 0, 0, 0, -10_368_000)),
        Record("LIFE_ENTRY", (0, 3, 0, 0x2222, 1, 1, 0, 0, -(1 << 40))),
        Record("LIFE_EXIT", (0, 4, 0, 0x1111, 0, 1, 0)),
        Record("STEP_DIGEST", (0, 5, 0, 1, 0, 1, 0x0123_4567_89AB_CDEF, 0, MASK64)),
        Record(
            "INPUT",
            (
                0,
                6,
                43_200,
                1,
                0,
                "ForkOverride",
                "none",
                None,
                "Player",
                b"\xff" * DIGEST_LEN,
                D_TEST,
            ),
        ),
        Record(
            "EVENT",
            (
                0,
                7,
                43_200,
                1,
                4,
                7,
                "ISSUE_DECREE",
                1 << 4,
                12,
                -5,
                96,
                3,
                1,
                (),
                ((1, 0x2222, 0), (3, 9, 1)),
                ((0, TRIGGERED),),
            ),
        ),
        Record(
            "EVENT",
            (
                0,
                8,
                43_200,
                2,
                9,
                1,
                "EXTINCTION",
                DEATH_CLASS,
                None,
                0,
                255,
                0,
                0,
                (-1, 0, (1 << 63) - 1, -(1 << 63)),
                (),
                ((0, 0), (1, 5)),
            ),
        ),
        Record(
            "EVENT",
            (
                0,
                9,
                216_000,
                3,
                4,
                DEATH,
                "DEATH",
                DEATH_CLASS,
                None,
                0,
                128,
                0,
                0,
                (2,),
                ((1, 0x2222, TARGET), (1, 0x3333, 0)),
                (),
            ),
        ),
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
        Record("KEYFRAME", (0, 14, 216_000, "", 'a b%é/日本\t\n"\\', MASK64, MASK64)),
    ]


def _hand_written_c() -> list[Record]:
    """The life_ends matches of :func:`_hand_written_a`, with their content-addressed ids."""
    out = []
    for seq, tick, event, a_seq, person in LIFE_ENDS_VECTORS:
        mid = match_id("solitary_life", "life_ends", 1, [event])
        out.append(
            Record(
                "MATCH",
                (
                    0,
                    seq,
                    tick,
                    mid,
                    "solitary_life",
                    "life_ends",
                    1,
                    0,
                    0,
                    1,
                    tick,
                    event,
                    a_seq,
                    (("A", 1, person),),
                    (("e1", event),),
                ),
            )
        )
    out.append(
        Record(
            "MATCH",
            (
                0,
                2,
                216_000,
                match_id("betrayal", "trust_then_betrayal", 3, [1, 3]),
                "betrayal",
                "trust_then_betrayal",
                3,
                2,
                2,
                1,
                43_200,
                3,
                9,
                (("A", 1, 0x2222), ("B", 1, 0x3333)),
                (("e1", 1), ("e2", 3)),
            ),
        )
    )
    return out


MATCH_ID_VECTORS: tuple[tuple[str, str, int, tuple[int, ...]], ...] = (
    ("solitary_life", "life_ends", 1, (0,)),
    ("solitary_life", "life_ends", 1, (3,)),
    ("betrayal", "trust_then_betrayal", 3, (1, 3)),
    ("", "", 0, ()),
    ("pé", "q", 65535, (MASK64,)),
)
GENERATED_PER_SEGMENT = 24


def _segment(timeline: int, channel: str, year: int, records: Sequence[Record]) -> list[str]:
    lines = [render(r) for r in records]
    return [
        f"segment {timeline} {channel} {year} {len(lines)}",
        *lines,
        f"digest {digest(lines).hex()}",
    ]


def render_golden() -> str:
    """The golden file's full text."""
    rng = random.Random(2026)
    a_kinds = [k for k in KINDS if CHANNEL[k] == "A"]
    header = [
        "# World Stream canonical text form v0: golden lines, segment digests and MatchId vectors.",
        "# GENERATED by specs/stream/canonical_v0.py; do not edit. Grammar: canonical_v0.md.",
        "# `segment <timeline> <channel> <sim_year> <n>` is followed by n canonical lines, then",
        "# `digest <xxh3-128, 32 lowercase hex>`: canonical_digest over those lines, each with LF.",
        "# `match_id <pack> <pattern_id> <version> <event ids> <xxh3-64>`: channel_c_v0.sql R5.",
    ]
    body = [
        *_segment(0, "A", 0, _hand_written_a()),
        *_segment(0, "C", 0, _hand_written_c()),
        *_segment(
            0, "A", 1, [generate(rng, rng.choice(a_kinds)) for _ in range(GENERATED_PER_SEGMENT)]
        ),
        *_segment(0, "C", 1, [generate(rng, "MATCH") for _ in range(GENERATED_PER_SEGMENT // 2)]),
        *_segment(3, "A", 2, []),
    ]
    vectors = [
        " ".join(
            [
                "match_id",
                encode_str(pack),
                encode_str(pid),
                str(v),
                EVENT_IDS.render(ids),
                f"{match_id(pack, pid, v, ids):016x}",
            ]
        )
        for pack, pid, v, ids in MATCH_ID_VECTORS
    ]
    return "\n".join([*header, *body, *vectors]) + "\n"


def digest_file(path: Path) -> bytes:
    """Parse every record line of a canonical file (``#`` lines skipped) and return their digest."""
    lines = []
    for n, raw in enumerate(path.read_text(encoding="utf-8").split("\n")[:-1], 1):
        if raw.startswith("#"):
            continue
        try:
            parse(raw)
        except CanonicalError as e:
            raise CanonicalError(f"{path}:{n}: {e}") from None
        lines.append(raw)
    return digest(lines)


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
        f"{len(REFUSED)} planted lines refused"
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
