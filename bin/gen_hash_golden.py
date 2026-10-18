"""Reference implementation of XXH3 and the rh-snapshot plane-hash layout; writes golden vectors.

This is an independent, standard-library re-implementation (SPR-007 T-01, Architect ruling 1) of
XXH3-64 and XXH3-128 (seed 0, default secret), written from the xxHash specification
(``doc/xxhash_spec.md`` and ``xxhash.h`` v0.8, Yann Collet, BSD-2-Clause), and of the plane-hash
layout that ``crates/rh-snapshot/src/hash.rs`` documents. It generates
``crates/rh-snapshot/tests/golden/rh_snapshot_hash_v1.txt``; the Rust test
(``crates/rh-snapshot/tests/golden.rs``) must reproduce every line bit for bit. Two independent
implementations agreeing is the known-answer test, as ``bin/gen_rng_golden.py`` is for rh-ids;
XXH3 itself is first checked here against the published empty-input values.

Layout ``rehoboam/rh-snapshot/hash/v1`` (all integers little-endian; ``XXH3`` = XXH3-64, seed 0):

Row
    A table's canonical row encoding. The stub ``i64`` column: 8 two's-complement bytes per row.
Chunk
    ``R = 1024`` rows; chunk ``c`` covers rows ``[c*R, min((c+1)*R, n))``.
    ``chunk(c) = XXH3(D_CHUNK || u64(c) || u64(m) || (u32(len(row)) || row)*)``
Table root
    ``XXH3(D_TABLE || u32(len(name)) || name || u64(n) || u64(k) || u64(chunk(0)) ..)``
Plane hash
    ``XXH3(D_PLANE || u8(plane letter) || u64(t) || (u32(len(name)) || name || u64(root))*)``
    over the tables in strictly increasing byte order of their names.
``D_CHUNK``, ``D_TABLE``, ``D_PLANE`` are ``b"rehoboam/rh-snapshot/hash/v1/"`` followed by
``chunk``, ``table`` and ``plane``.

Event digest v2 (``rehoboam/rh-events/log-hash/v2``; ``specs/chronicle/envelope_v0.md`` §6; SPR-020
T-02). ``crates/rh-events/tests/golden/event_digest_v2.txt``: every event's ``Event::digest`` and
the ``EventLog::hash`` of the events appended in order, from the SplitMix64 scalar hash
(``h64_words``, ``h64_bytes``, ``tag``; layout in ``crates/rh-ids/src/rng.rs``), re-implemented
here. The digest's words, little-endian ``u64`` through ``h64_words(T, words)`` with
``T = tag("rehoboam/rh-events/log-hash/v2")`` and ``h(name) = h64_bytes(T, name)``::

    id, tick, phase, h(emitter), h(kind),
    has_place, place (0 when absent), magnitude (i32 sign-extended), secrecy, origin,
    n_roles,     then per role row:  role, entity_kind_tag, entity_raw
    n_snapshots, then per snapshot:  h(attr), kind_tag, raw, has_partner, partner_kind_tag,
                                     partner_raw (0, 0 when absent), value (i32 sign-extended)
    n_causes,    then per edge:      cause_id, kind, has_motive, h(motive) (0 when absent),
                                     weight (i16 sign-extended), evidence
    n_payload,   then the payload words

The log: ``running = h64_words(T, [running, digest])`` per event from 0, and
``hash = h64_words(T, [n, running])``.

Usage:  python -I bin/gen_hash_golden.py           (rewrite the golden files)
        python -I bin/gen_hash_golden.py --check   (exit 1 if the committed file differs)
        python -I bin/gen_hash_golden.py --check --golden PATH   (check another file; the positive
                                                                  control uses a stale fixture)
        python -I bin/gen_hash_golden.py --check --events-golden PATH   (likewise, the event
                                                                  digest file)
Exit:   0 ok, 1 stale golden file, 2 the reference failed its own known-answer self-test.
Standard library only (GOV-003 §3).
"""

import argparse
import copy
import itertools
import sys
from collections.abc import Callable
from pathlib import Path
from typing import Any

GOLDEN = (
    Path(__file__).resolve().parent.parent
    / "crates/rh-snapshot/tests/golden/rh_snapshot_hash_v1.txt"
)
EVENTS_GOLDEN = (
    Path(__file__).resolve().parent.parent / "crates/rh-events/tests/golden/event_digest_v2.txt"
)
EXIT_OK = 0
EXIT_STALE = 1
EXIT_SELFTEST = 2

MASK32 = 0xFFFF_FFFF
MASK64 = 0xFFFF_FFFF_FFFF_FFFF

PRIME32_1 = 0x9E37_79B1
PRIME32_2 = 0x85EB_CA77
PRIME32_3 = 0xC2B2_AE3D
PRIME64_1 = 0x9E37_79B1_85EB_CA87
PRIME64_2 = 0xC2B2_AE3D_27D4_EB4F
PRIME64_3 = 0x1656_67B1_9E37_79F9
PRIME64_4 = 0x85EB_CA77_C2B2_AE63
PRIME64_5 = 0x27D4_EB2F_1656_67C5
PRIME_MX1 = 0x1656_6791_9E37_79F9
PRIME_MX2 = 0x9FB2_1C65_1E98_DF25

# The default 192-byte secret (kSecret) of the specification.
SECRET = bytes.fromhex(
    "b8fe6c3923a44bbe7c01812cf721ad1cded46de9839097db7240a4a4b7b3671f"
    "cb79e64eccc0e578825ad07dccff7221b8084674f743248ee03590e6813a264c"
    "3c2852bb91c300cb88d0658b1b532ea371644897a20df94e3819ef46a9deacd8"
    "a8fa763fe39c343ff9dcbbc7c70b4f1d8a51e04bcdb45931c89f7ec9d9787364"
    "eac5ac8334d3ebc3c581a0fffa1363eb170ddd51b7f0da49d316552629d4689e"
    "2b16be587d47a1fc8ff8b8d17ad031ce45cb3a8f95160428afd7fbcabb4b407e"
)
SECRET_SIZE_MIN = 136
STRIPE_LEN = 64
SECRET_CONSUME_RATE = 8
ACC_NB = 8
MIDSIZE_STARTOFFSET = 3
MIDSIZE_LASTOFFSET = 17
SECRET_LASTACC_START = 7
SECRET_MERGEACCS_START = 11

# Published known answers: XXH3_64bits("") and XXH3_128bits("") (canonical, high half first).
KAT_XXH3_64_EMPTY = 0x2D06_8005_38D3_94C2
KAT_XXH3_128_EMPTY = 0x99AA_06D3_0147_98D8_6001_C324_468D_497F

CHUNK_ROWS = 1024
D_CHUNK = b"rehoboam/rh-snapshot/hash/v1/chunk"
D_TABLE = b"rehoboam/rh-snapshot/hash/v1/table"
D_PLANE = b"rehoboam/rh-snapshot/hash/v1/plane"
GAMMA = 0x9E37_79B9_7F4A_7C15


def _r32(b: bytes, i: int) -> int:
    return int.from_bytes(b[i : i + 4], "little")


def _r64(b: bytes, i: int) -> int:
    return int.from_bytes(b[i : i + 8], "little")


def _rotl64(x: int, n: int) -> int:
    return ((x << n) | (x >> (64 - n))) & MASK64


def _rotl32(x: int, n: int) -> int:
    return ((x << n) | (x >> (32 - n))) & MASK32


def _swap32(x: int) -> int:
    return int.from_bytes(x.to_bytes(4, "little"), "big")


def _swap64(x: int) -> int:
    return int.from_bytes(x.to_bytes(8, "little"), "big")


def _mul128(a: int, b: int) -> tuple[int, int]:
    """The 128-bit product of two u64 as (low, high)."""
    p = a * b
    return p & MASK64, p >> 64


def _fold64(a: int, b: int) -> int:
    lo, hi = _mul128(a, b)
    return lo ^ hi


def _xxh64_avalanche(h: int) -> int:
    h ^= h >> 33
    h = (h * PRIME64_2) & MASK64
    h ^= h >> 29
    h = (h * PRIME64_3) & MASK64
    return h ^ (h >> 32)


def _avalanche(h: int) -> int:
    h ^= h >> 37
    h = (h * PRIME_MX1) & MASK64
    return h ^ (h >> 32)


def _rrmxmx(h: int, length: int) -> int:
    h ^= _rotl64(h, 49) ^ _rotl64(h, 24)
    h = (h * PRIME_MX2) & MASK64
    h ^= (h >> 35) + length
    h = (h * PRIME_MX2) & MASK64
    return h ^ (h >> 28)


def _mix16(data: bytes, i: int, s: int, seed: int = 0) -> int:
    lo = _r64(data, i) ^ ((_r64(SECRET, s) + seed) & MASK64)
    hi = _r64(data, i + 8) ^ ((_r64(SECRET, s + 8) - seed) & MASK64)
    return _fold64(lo, hi)


def _long_accumulators(data: bytes) -> list[int]:
    """The eight accumulators of the long-input loop (inputs over 240 bytes)."""
    acc = [PRIME32_3, PRIME64_1, PRIME64_2, PRIME64_3, PRIME64_4, PRIME32_2, PRIME64_5, PRIME32_1]
    n = len(data)
    stripes_per_block = (len(SECRET) - STRIPE_LEN) // SECRET_CONSUME_RATE
    block_len = STRIPE_LEN * stripes_per_block

    def accumulate_512(i: int, s: int) -> None:
        for lane in range(ACC_NB):
            value = _r64(data, i + 8 * lane)
            key = value ^ _r64(SECRET, s + 8 * lane)
            acc[lane ^ 1] = (acc[lane ^ 1] + value) & MASK64
            acc[lane] = (acc[lane] + (key & MASK32) * (key >> 32)) & MASK64

    def scramble() -> None:
        s = len(SECRET) - STRIPE_LEN
        for lane in range(ACC_NB):
            a = acc[lane]
            a ^= a >> 47
            a ^= _r64(SECRET, s + 8 * lane)
            acc[lane] = (a * PRIME32_1) & MASK64

    blocks = (n - 1) // block_len
    for b in range(blocks):
        for k in range(stripes_per_block):
            accumulate_512(b * block_len + k * STRIPE_LEN, k * SECRET_CONSUME_RATE)
        scramble()
    stripes = ((n - 1) - block_len * blocks) // STRIPE_LEN
    for k in range(stripes):
        accumulate_512(blocks * block_len + k * STRIPE_LEN, k * SECRET_CONSUME_RATE)
    accumulate_512(n - STRIPE_LEN, len(SECRET) - STRIPE_LEN - SECRET_LASTACC_START)
    return acc


def _merge(acc: list[int], s: int, start: int) -> int:
    result = start & MASK64
    for i in range(4):
        lo = acc[2 * i] ^ _r64(SECRET, s + 16 * i)
        hi = acc[2 * i + 1] ^ _r64(SECRET, s + 16 * i + 8)
        result = (result + _fold64(lo, hi)) & MASK64
    return _avalanche(result)


# The length ranges of XXH3: 0, 1-3, 4-8, 9-16, 17-128, 129-240 and long.
LEN_TINY = 3
LEN_SMALL = 8
LEN_SHORT = 16
LEN_MID = 128
LEN_MIDSIZE = 240
# The 17-128 range mixes 16-byte pairs above these lengths.
MID_STEPS = (32, 64, 96)


def _h64_empty(_: bytes) -> int:
    return _xxh64_avalanche(_r64(SECRET, 56) ^ _r64(SECRET, 64))


def _h64_1to3(data: bytes) -> int:
    n = len(data)
    combined = (data[0] << 16) | (data[n >> 1] << 24) | data[n - 1] | (n << 8)
    return _xxh64_avalanche(combined ^ (_r32(SECRET, 0) ^ _r32(SECRET, 4)))


def _h64_4to8(data: bytes) -> int:
    n = len(data)
    input64 = _r32(data, n - 4) + (_r32(data, 0) << 32)
    return _rrmxmx(input64 ^ (_r64(SECRET, 8) ^ _r64(SECRET, 16)), n)


def _h64_9to16(data: bytes) -> int:
    n = len(data)
    lo = _r64(data, 0) ^ (_r64(SECRET, 24) ^ _r64(SECRET, 32))
    hi = _r64(data, n - 8) ^ (_r64(SECRET, 40) ^ _r64(SECRET, 48))
    return _avalanche((n + _swap64(lo) + hi + _fold64(lo, hi)) & MASK64)


def _h64_17to128(data: bytes) -> int:
    n = len(data)
    acc = n * PRIME64_1
    # Pairs (front offset, back offset from the end, secret offset), innermost first.
    pairs = [(0, 16, 0), (16, 32, 32), (32, 48, 64), (48, 64, 96)]
    for (front, back, s), above in zip(pairs, (0, *MID_STEPS), strict=True):
        if n > above:
            acc += _mix16(data, front, s) + _mix16(data, n - back, s + 16)
    return _avalanche(acc & MASK64)


def _h64_129to240(data: bytes) -> int:
    n = len(data)
    acc = n * PRIME64_1
    for i in range(8):
        acc += _mix16(data, 16 * i, 16 * i)
    acc = _avalanche(acc & MASK64)
    for i in range(8, n // 16):
        acc += _mix16(data, 16 * i, 16 * (i - 8) + MIDSIZE_STARTOFFSET)
    acc += _mix16(data, n - 16, SECRET_SIZE_MIN - MIDSIZE_LASTOFFSET)
    return _avalanche(acc & MASK64)


def _h64_long(data: bytes) -> int:
    return _merge(_long_accumulators(data), SECRET_MERGEACCS_START, len(data) * PRIME64_1)


def xxh3_64(data: bytes) -> int:
    """XXH3-64 of ``data``, seed 0, default secret."""
    n = len(data)
    for limit, f in (
        (0, _h64_empty),
        (LEN_TINY, _h64_1to3),
        (LEN_SMALL, _h64_4to8),
        (LEN_SHORT, _h64_9to16),
        (LEN_MID, _h64_17to128),
        (LEN_MIDSIZE, _h64_129to240),
    ):
        if n <= limit:
            return f(data)
    return _h64_long(data)


def _mix32(acc: tuple[int, int], data: bytes, i1: int, i2: int, s: int) -> tuple[int, int]:
    """XXH128_mix32B with seed 0 (the 128-bit variant's 32-byte mixing step)."""
    lo, hi = acc
    lo = (lo + _mix16(data, i1, s)) & MASK64
    lo ^= (_r64(data, i2) + _r64(data, i2 + 8)) & MASK64
    hi = (hi + _mix16(data, i2, s + 16)) & MASK64
    hi ^= (_r64(data, i1) + _r64(data, i1 + 8)) & MASK64
    return lo, hi


def _finish128(acc: tuple[int, int], n: int) -> int:
    lo, hi = acc
    low = _avalanche((lo + hi) & MASK64)
    high = (lo * PRIME64_1 + hi * PRIME64_4 + n * PRIME64_2) & MASK64
    high = (-_avalanche(high)) & MASK64
    return (high << 64) | low


def _h128_empty(_: bytes) -> int:
    low = _xxh64_avalanche(_r64(SECRET, 64) ^ _r64(SECRET, 72))
    high = _xxh64_avalanche(_r64(SECRET, 80) ^ _r64(SECRET, 88))
    return (high << 64) | low


def _h128_1to3(data: bytes) -> int:
    n = len(data)
    lo = (data[0] << 16) | (data[n >> 1] << 24) | data[n - 1] | (n << 8)
    hi = _rotl32(_swap32(lo), 13)
    low = _xxh64_avalanche(lo ^ (_r32(SECRET, 0) ^ _r32(SECRET, 4)))
    high = _xxh64_avalanche(hi ^ (_r32(SECRET, 8) ^ _r32(SECRET, 12)))
    return (high << 64) | low


def _h128_4to8(data: bytes) -> int:
    n = len(data)
    input64 = _r32(data, 0) + (_r32(data, n - 4) << 32)
    keyed = input64 ^ (_r64(SECRET, 16) ^ _r64(SECRET, 24))
    lo, hi = _mul128(keyed, (PRIME64_1 + (n << 2)) & MASK64)
    hi = (hi + (lo << 1)) & MASK64
    lo ^= hi >> 3
    lo ^= lo >> 35
    lo = (lo * PRIME_MX2) & MASK64
    lo ^= lo >> 28
    return (_avalanche(hi) << 64) | lo


def _h128_9to16(data: bytes) -> int:
    n = len(data)
    bitflip_lo = _r64(SECRET, 32) ^ _r64(SECRET, 40)
    bitflip_hi = _r64(SECRET, 48) ^ _r64(SECRET, 56)
    in_lo = _r64(data, 0)
    in_hi = _r64(data, n - 8)
    lo, hi = _mul128(in_lo ^ in_hi ^ bitflip_lo, PRIME64_1)
    lo = (lo + ((n - 1) << 54)) & MASK64
    in_hi ^= bitflip_hi
    hi = (hi + in_hi + (in_hi & MASK32) * (PRIME32_2 - 1)) & MASK64
    lo ^= _swap64(hi)
    rlo, rhi = _mul128(lo, PRIME64_2)
    rhi = (rhi + hi * PRIME64_2) & MASK64
    return (_avalanche(rhi) << 64) | _avalanche(rlo)


def _h128_17to128(data: bytes) -> int:
    n = len(data)
    acc = ((n * PRIME64_1) & MASK64, 0)
    # Outermost first, as the specification nests them: (front, back from the end, secret).
    steps = [(48, 64, 96), (32, 48, 64), (16, 32, 32), (0, 16, 0)]
    for (front, back, s), above in zip(steps, (*reversed(MID_STEPS), 0), strict=True):
        if n > above:
            acc = _mix32(acc, data, front, n - back, s)
    return _finish128(acc, n)


def _h128_129to240(data: bytes) -> int:
    n = len(data)
    acc = ((n * PRIME64_1) & MASK64, 0)
    for i in range(32, 160, 32):
        acc = _mix32(acc, data, i - 32, i - 16, i - 32)
    acc = (_avalanche(acc[0]), _avalanche(acc[1]))
    for i in range(160, n + 1, 32):
        acc = _mix32(acc, data, i - 32, i - 16, MIDSIZE_STARTOFFSET + i - 160)
    acc = _mix32(acc, data, n - 16, n - 32, SECRET_SIZE_MIN - MIDSIZE_LASTOFFSET - 16)
    return _finish128(acc, n)


def _h128_long(data: bytes) -> int:
    n = len(data)
    acc = _long_accumulators(data)
    low = _merge(acc, SECRET_MERGEACCS_START, n * PRIME64_1)
    high_start = len(SECRET) - STRIPE_LEN - SECRET_MERGEACCS_START
    high = _merge(acc, high_start, ~(n * PRIME64_2) & MASK64)
    return (high << 64) | low


def xxh3_128(data: bytes) -> int:
    """XXH3-128 of ``data``, seed 0, default secret, as ``high << 64 | low``."""
    n = len(data)
    for limit, f in (
        (0, _h128_empty),
        (LEN_TINY, _h128_1to3),
        (LEN_SMALL, _h128_4to8),
        (LEN_SHORT, _h128_9to16),
        (LEN_MID, _h128_17to128),
        (LEN_MIDSIZE, _h128_129to240),
    ):
        if n <= limit:
            return f(data)
    return _h128_long(data)


def self_test() -> list[str]:
    """Return the failures of the published known-answer values (empty when all pass)."""
    failures = []
    if xxh3_64(b"") != KAT_XXH3_64_EMPTY:
        got = xxh3_64(b"")
        failures.append(f"XXH3-64(empty) = {got:016x}, expected {KAT_XXH3_64_EMPTY:016x}")
    if xxh3_128(b"") != KAT_XXH3_128_EMPTY:
        got = xxh3_128(b"")
        failures.append(f"XXH3-128(empty) = {got:032x}, expected {KAT_XXH3_128_EMPTY:032x}")
    return failures


# --- The layout -------------------------------------------------------------------------------


def _u32(x: int) -> bytes:
    return x.to_bytes(4, "little")


def _u64(x: int) -> bytes:
    return x.to_bytes(8, "little")


def chunk_hash(rows: list[bytes], chunk: int) -> int:
    """Layout level 2: chunk ``chunk`` of a table given as encoded rows."""
    part = rows[chunk * CHUNK_ROWS : (chunk + 1) * CHUNK_ROWS]
    body = b"".join(_u32(len(r)) + r for r in part)
    return xxh3_64(D_CHUNK + _u64(chunk) + _u64(len(part)) + body)


def table_digest(name: str, rows: list[bytes]) -> tuple[list[int], int]:
    """Layout level 3: the chunk hashes and the root of table ``name`` given as encoded rows."""
    chunks = [chunk_hash(rows, c) for c in range(-(-len(rows) // CHUNK_ROWS))]
    body = b"".join(_u64(c) for c in chunks)
    encoded = name.encode()
    header = D_TABLE + _u32(len(encoded)) + encoded + _u64(len(rows)) + _u64(len(chunks))
    root = xxh3_64(header + body)
    return chunks, root


def plane_hash(plane: str, tables: list[tuple[str, int]]) -> int:
    """Layout level 4 over ``(name, root)`` pairs in strictly increasing byte order of name."""
    names = [name.encode() for name, _ in tables]
    if any(a >= b for a, b in itertools.pairwise(names)):
        raise ValueError("table names out of byte order")
    pairs = zip(names, tables, strict=True)
    body = b"".join(_u32(len(n)) + n + _u64(root) for n, (_, root) in pairs)
    return xxh3_64(D_PLANE + plane.encode() + _u64(len(tables)) + body)


# --- Engine tables (layout v1, SPR-007 T-02) --------------------------------------------------
#
# The plane-A state hash covers the world columns and eight engine tables named "rh-core/...";
# the rows are encoded as crates/rh-snapshot/src/state.rs documents. Little-endian integers;
# str = u32 len || UTF-8; opt(x) = u8 0 | u8 1 || x; entity = u8 kind tag || u64 raw; phase = u8
# index; list(x) = u32 count || x...

ENTITY_TAGS = {"world": 0, "person": 1, "household": 2, "settlement": 3, "region": 4}
TableRows = dict[str, list[bytes]]
# An engine state as plain data: nested dicts, lists and tuples of ints and strings.
State = dict[str, Any]


def _u8(x: int) -> bytes:
    return x.to_bytes(1, "little")


def _i64(x: int) -> bytes:
    return (x & MASK64).to_bytes(8, "little")


def _i32(x: int) -> bytes:
    return (x & MASK32).to_bytes(4, "little")


def _i16(x: int) -> bytes:
    return (x & 0xFFFF).to_bytes(2, "little")


def _str(text: str) -> bytes:
    b = text.encode()
    return _u32(len(b)) + b


def _entity(e: tuple[str, int]) -> bytes:
    return _u8(ENTITY_TAGS[e[0]]) + _u64(e[1])


def _opt_u64(x: int | None) -> bytes:
    return _u8(0) if x is None else _u8(1) + _u64(x)


def _opt_i64(x: int | None) -> bytes:
    return _u8(0) if x is None else _u8(1) + _i64(x)


def _list_i64(xs: list[int]) -> bytes:
    return _u32(len(xs)) + b"".join(_i64(x) for x in xs)


def engine_rows(st: State) -> TableRows:
    """The eight engine tables of a state given as plain data (see SYNTHETIC_STATE)."""
    ag = st["agenda"]
    timers = [
        _i64(t["time"])
        + _u8(t["phase"])
        + _entity(t["entity"])
        + _u64(t["seq"])
        + _u32(t["owner"])
        + _u64(t["purpose"])
        + _u32(t["kind"])
        + _i64(t["payload"])
        + _u8(1 if t["seq"] in ag["tombstones"] else 0)
        # EVO-001 R15 (SPR-014 T-02): the cause tail, u8 1 || u64 id, only with a cause
        + (_u8(1) + _u64(t["cause"]) if t.get("cause") is not None else b"")
        for t in ag["entries"]
    ]
    commands = [
        _str(owner)
        + _i64(c["tick"])
        + _u8(c["phase"])
        + _u64(c["person"])
        + _u64(c["seq"])
        + _str(c["sender"])
        + _opt_u64(c["cause"])
        + _str(c["kind"])
        + _list_i64(c["args"])
        + _opt_u64(c["draft"])
        for owner, queue in st["commands"]["queues"]
        for c in queue
    ]
    epochs = [_entity(e) + _u32(n) for e, n in st["epochs"]]
    inputs = [
        _i64(t) + _u32(i) + _u32(len(scope)) + b"".join(_entity(e) for e in scope)
        for t, scopes in st["inputs"]
        for i, scope in enumerate(scopes)
    ]
    meta = (
        _opt_i64(st["cursor"])
        + _u32(st["timeline"])
        + _u64(st["accepted_inputs"])
        + _u64(st["commands"]["next_seq"])
    )
    ev = st["events"]
    held = [
        _u64(e["id"])
        + _i64(e["tick"])
        + _u8(e["phase"])
        + _str(e["emitter"])
        + _str(e["kind"])
        + (_u8(0) if e["place"] is None else _u8(1) + _u32(e["place"]))
        + _i32(e["magnitude"])
        + _u8(e["secrecy"])
        + _u8(e["origin"])
        + _u8(e["admitted"])
        + _u32(len(e["roles"]))
        + b"".join(_u8(r) + _entity(x) for r, x in e["roles"])
        + _u32(len(e["snapshots"]))
        + b"".join(
            _str(a) + _entity(x) + (_u8(0) if p is None else _u8(1) + _entity(p)) + _i32(v)
            for a, x, p, v in e["snapshots"]
        )
        + _u32(len(e["causes"]))
        + b"".join(
            _u64(c) + _u8(k) + (_u8(0) if m is None else _u8(1) + _str(m)) + _i16(w) + _i16(ev)
            for c, k, m, w, ev in e["causes"]
        )
        + _list_i64(e["payload"])
        for e in ev["events"]
    ]
    last = _u8(0) if ev["last"] is None else _u8(1) + _i64(ev["last"][0]) + _u8(ev["last"][1])
    return {
        "rh-core/agenda.meta": [_u64(ag["next_seq"])],
        "rh-core/agenda.timers": timers,
        "rh-core/engine.commands": commands,
        "rh-core/engine.epochs": epochs,
        "rh-core/engine.inputs": inputs,
        "rh-core/engine.meta": [meta],
        "rh-core/events.held": held,
        "rh-core/events.meta": [_u64(ev["next_seq"]) + last + _u64(ev["running"])],
    }


# The engine tables a keyframe carries that state_hash does not cover (state.rs CARRIED_TABLES).
CARRIED_TABLES = frozenset({"rh-core/events.held"})


def state_hash(world: list[tuple[str, int, int]], st: State) -> tuple[TableRows, int]:
    """The plane-A hash of synthetic world columns ``(name, salt, rows)`` plus engine tables."""
    tables: TableRows = {name: column_rows(salt, n) for name, salt, n in world}
    tables.update(engine_rows(st))
    # Carried tables (undrained events) are in a keyframe but outside state_hash: draining is the
    # host's, not history (Architect ruling, VER-SPR-007-TEST round 2 GAP-6).
    roots = [
        (name, table_digest(name, rows)[1])
        for name, rows in sorted(tables.items())
        if name not in CARRIED_TABLES
    ]
    return tables, plane_hash("A", sorted(roots, key=lambda r: r[0].encode()))


DAY = 1_440
# A synthetic engine state that every engine table has rows in; crates/rh-snapshot/tests/golden.rs
# builds the same state with the Rust types.
SYNTHETIC_WORLD = [("pop.count", 21, 5), ("vital.alive", 22, 1030)]
SYNTHETIC_STATE: State = {
    "cursor": 40 * DAY + 7,
    "timeline": 3,
    "accepted_inputs": 2,
    "agenda": {
        "next_seq": 6,
        "entries": [
            {
                "time": 100,
                "phase": 4,
                "entity": ("person", 7),
                "seq": 2,
                "owner": 11,
                "purpose": 0xABC,
                "kind": 1,
                "payload": -5,
            },
            {
                "time": 100,
                "phase": 4,
                "entity": ("person", 9),
                "seq": 0,
                "owner": 11,
                "purpose": 0xABC,
                "kind": 2,
                "payload": 7,
            },
            {
                "time": 250,
                "phase": 1,
                "entity": ("household", 3),
                "seq": 5,
                "owner": 12,
                "purpose": 0xDEF,
                "kind": 3,
                "payload": 0,
            },
        ],
        "tombstones": [5],
    },
    "commands": {
        "next_seq": 9,
        "queues": [
            (
                "X2.estate",
                [
                    {
                        "tick": DAY,
                        "phase": 3,
                        "person": 7,
                        "seq": 4,
                        "sender": "X1.vital",
                        "cause": 12,
                        "kind": "Inherit",
                        "args": [1, -2],
                        "draft": 0,
                    },
                    {
                        "tick": DAY,
                        "phase": 3,
                        "person": 8,
                        "seq": 8,
                        "sender": "X5.war",
                        "cause": None,
                        "kind": "Inherit",
                        "args": [],
                        "draft": None,
                    },
                ],
            ),
        ],
    },
    "epochs": [(("person", 7), 1), (("household", 3), 2)],
    "inputs": [
        (45 * DAY + 3, [[("person", 1), ("person", 6)], []]),
        (400 * DAY, [[("region", 2)]]),
    ],
    "events": {
        "next_seq": 20,
        "last": (57_603, 9),
        "running": 0x1234_5678_9ABC_DEF0,
        "events": [
            {
                "id": 18,
                "tick": 57_600,
                "phase": 4,
                "emitter": "X1.vital",
                "kind": "DEATH",
                "place": None,
                "magnitude": 0,
                "secrecy": 0,
                "origin": 0,
                "admitted": 1,
                "roles": [(1, ("person", 7))],
                "snapshots": [],
                "causes": [(3, 1, None, 0, 256)],
                "payload": [1],
            },
            {
                "id": 19,
                "tick": 57_603,
                "phase": 9,
                "emitter": "X4.census",
                "kind": "CENSUS",
                "place": 12,
                "magnitude": -7,
                "secrecy": 2,
                "origin": 3,
                "admitted": 0,
                "roles": [(0, ("person", 9)), (2, ("household", 3))],
                "snapshots": [
                    ("stress", ("person", 9), None, 18_000),
                    ("trust", ("person", 9), ("person", 7), -300),
                ],
                "causes": [(3, 3, "GRIEF_FOR", -120, 40), (18, 1, None, 0, 256)],
                "payload": [-1, 40],
            },
        ],
    },
}


def lines_engine() -> list[str]:
    """``engine_table`` vectors and the ``engine`` plane-A hash of the synthetic state."""
    tables, h = state_hash(SYNTHETIC_WORLD, SYNTHETIC_STATE)
    out = []
    for name, rows in sorted(engine_rows(SYNTHETIC_STATE).items()):
        chunks, root = table_digest(name, rows)
        out.append(
            " ".join(
                [
                    "engine_table",
                    name,
                    str(len(rows)),
                    f"{root:016x}",
                    *(f"{c:016x}" for c in chunks),
                ]
            )
        )
    assert len(tables) == len(SYNTHETIC_WORLD) + 8
    out.append(f"engine {h:016x}")
    return out


# The synthetic state with causes on two timers (SPR-014 T-02; 05.1 §2 rule 2): the first timer's
# cause is event 18, the third's event 0 (the first id: a cause is present even when its id is 0).
# Only rh-core/agenda.timers and the plane hash differ from the state above; a timer without a
# cause is encoded as before.
SYNTHETIC_STATE_CAUSE: State = {
    **SYNTHETIC_STATE,
    "agenda": {
        **SYNTHETIC_STATE["agenda"],
        "entries": [
            {**e, "cause": c}
            for e, c in zip(SYNTHETIC_STATE["agenda"]["entries"], [18, None, 0], strict=True)
        ],
    },
}


def lines_engine_cause() -> list[str]:
    """``engine_cause_table`` (the timers, with cause tails) and ``engine_cause`` (the hash)."""
    _, h = state_hash(SYNTHETIC_WORLD, SYNTHETIC_STATE_CAUSE)
    rows = engine_rows(SYNTHETIC_STATE_CAUSE)["rh-core/agenda.timers"]
    chunks, root = table_digest("rh-core/agenda.timers", rows)
    line = " ".join(
        [
            "engine_cause_table",
            "rh-core/agenda.timers",
            str(len(rows)),
            f"{root:016x}",
            *(f"{c:016x}" for c in chunks),
        ]
    )
    return [line, f"engine_cause {h:016x}"]


# --- The vectors -------------------------------------------------------------------------------


def input_bytes(length: int) -> bytes:
    """The hashed input of a ``xxh3`` vector: byte ``j`` is ``(131 j + length) mod 256``."""
    return bytes((131 * j + length) & 0xFF for j in range(length))


def column_value(salt: int, i: int) -> int:
    """Row ``i`` of a synthetic ``i64`` column: ``((i + 1) * GAMMA) xor salt``, two's complement."""
    v = (((i + 1) * GAMMA) & MASK64) ^ salt
    return v - (1 << 64) if v >= 1 << 63 else v


def column_rows(salt: int, n: int) -> list[bytes]:
    """A synthetic ``i64`` column encoded as rows (8 little-endian bytes each)."""
    return [(column_value(salt, i) & MASK64).to_bytes(8, "little") for i in range(n)]


def bytes_rows(n: int) -> list[bytes]:
    """A synthetic variable-length table: row ``i`` is ``i mod 7`` bytes of ``i mod 256``."""
    return [bytes([i & 0xFF]) * (i % 7) for i in range(n)]


XXH3_LENGTHS = [*range(0, 257), 511, 512, 513, 1023, 1024, 1025, 2047, 2048, 2049, 4099, 10_000]
COLUMN_CASES = [
    (0, 0),
    (1, 1),
    (2, 1023),
    (3, 1024),
    (4, 1025),
    (5, 3000),
    (0xFFFF_FFFF_FFFF_FFFF, 2048),
]
BYTES_CASES = [0, 1, 1024, 1500]
# Table names of the column and bytes vectors, cycled; one is non-ASCII (its UTF-8 length counts).
TABLE_NAMES = ["pop.count", "ledger.wealth", "région.sol", "t"]
# Plane vectors: (plane, [(name, column salt, rows)]), names given in byte order.
PLANE_CASES: list[tuple[str, list[tuple[str, int, int]]]] = [
    ("A", []),
    ("B", []),
    ("C", []),
    ("A", [("agenda.due", 3, 1000), ("ledger.wealth", 1, 3000), ("pop.count", 2, 3000)]),
    ("B", [("Z", 9, 1), ("a", 9, 1)]),
    ("C", [("matches", 11, 2049), ("visibility", 12, 0)]),
]


def lines_xxh3() -> list[str]:
    """``xxh3_64`` and ``xxh3_128`` vectors over :func:`input_bytes`."""
    out = []
    for n in XXH3_LENGTHS:
        out.append(f"xxh3_64 {n} {xxh3_64(input_bytes(n)):016x}")
    for n in XXH3_LENGTHS:
        out.append(f"xxh3_128 {n} {xxh3_128(input_bytes(n)):032x}")
    return out


def lines_tables() -> list[str]:
    """``column`` and ``bytes`` table vectors: root, then every chunk hash."""
    out = []
    for k, (salt, n) in enumerate(COLUMN_CASES):
        name = TABLE_NAMES[k % len(TABLE_NAMES)]
        chunks, root = table_digest(name, column_rows(salt, n))
        hexes = [f"{root:016x}", *(f"{c:016x}" for c in chunks)]
        out.append(" ".join(["column", name, str(salt), str(n), *hexes]))
    for k, n in enumerate(BYTES_CASES):
        name = TABLE_NAMES[k % len(TABLE_NAMES)]
        chunks, root = table_digest(name, bytes_rows(n))
        hexes = [f"{root:016x}", *(f"{c:016x}" for c in chunks)]
        out.append(" ".join(["bytes", name, str(n), *hexes]))
    return out


def lines_planes() -> list[str]:
    """``plane`` vectors over synthetic columns."""
    out = []
    for plane, tables in PLANE_CASES:
        roots = [(name, table_digest(name, column_rows(salt, n))[1]) for name, salt, n in tables]
        fields = [f"{name} {salt} {n}" for name, salt, n in tables]
        h = plane_hash(plane, roots)
        out.append(" ".join(["plane", plane, str(len(tables)), *fields, f"{h:016x}"]))
    return out


# --- Event digest v2 (SPR-020 T-02) ------------------------------------------------------------

SM_GAMMA = 0x9E37_79B9_7F4A_7C15
SM_MUL_1 = 0xBF58_476D_1CE4_E5B9
SM_MUL_2 = 0x94D0_49BB_1331_11EB
# The first output of SplitMix64 from state 0 (Vigna, splitmix64.c): mix(GAMMA).
KAT_SPLITMIX64_SEED0 = 0xE220_A839_7B1D_CDAF


def sm_mix(z: int) -> int:
    """The SplitMix64 output function (Steele, Lea and Flood 2014)."""
    z = ((z ^ (z >> 30)) * SM_MUL_1) & MASK64
    z = ((z ^ (z >> 27)) * SM_MUL_2) & MASK64
    return z ^ (z >> 31)


def h64_words(domain: int, words: list[int]) -> int:
    """``h = mix(domain ^ n)``, then ``h = mix((h + GAMMA) ^ w)`` per word."""
    h = sm_mix(domain ^ len(words))
    for w in words:
        h = sm_mix(((h + SM_GAMMA) & MASK64) ^ (w & MASK64))
    return h


def h64_bytes(domain: int, data: bytes) -> int:
    """:func:`h64_words` over 8-byte little-endian chunks (the last zero-padded), ``n`` bytes."""
    h = sm_mix(domain ^ len(data))
    for i in range(0, len(data), 8):
        w = int.from_bytes(data[i : i + 8].ljust(8, b"\0"), "little")
        h = sm_mix(((h + SM_GAMMA) & MASK64) ^ w)
    return h


def sm_tag(name: str) -> int:
    """``tag(name) = h64_bytes(0, name)``."""
    return h64_bytes(0, name.encode("ascii"))


T_EVENT = sm_tag("rehoboam/rh-events/log-hash/v2")
EVENT_ENTITY_TAGS = {"world": 0, "person": 1, "household": 2, "settlement": 3, "region": 4}

Event = dict[str, Any]


def event_digest(e: Event) -> int:
    """``Event::digest`` v2 of an event given as plain data (see EVENT_CASES)."""

    def h(name: str) -> int:
        return h64_bytes(T_EVENT, name.encode())

    place = e["place"]
    words = [
        e["id"],
        e["tick"],
        e["phase"],
        h(e["emitter"]),
        h(e["kind"]),
        0 if place is None else 1,
        0 if place is None else place,
        e["magnitude"],
        e["secrecy"],
        e["origin"],
        len(e["roles"]),
    ]
    for role, (kind, raw) in e["roles"]:
        words += [role, EVENT_ENTITY_TAGS[kind], raw]
    words.append(len(e["snapshots"]))
    for attr, (kind, raw), partner, value in e["snapshots"]:
        pk, praw = (0, 0) if partner is None else (EVENT_ENTITY_TAGS[partner[0]], partner[1])
        words += [h(attr), EVENT_ENTITY_TAGS[kind], raw, int(partner is not None), pk, praw, value]
    words.append(len(e["causes"]))
    for cause, kind, motive, weight, evidence in e["causes"]:
        words += [cause, kind, int(motive is not None), 0 if motive is None else h(motive)]
        words += [weight, evidence]
    words.append(len(e["payload"]))
    words += e["payload"]
    return h64_words(T_EVENT, [w & MASK64 for w in words])


def _event(**fields: object) -> Event:
    base: Event = {
        "place": None,
        "magnitude": 0,
        "secrecy": 0,
        "origin": 0,
        "roles": [],
        "snapshots": [],
        "causes": [],
        "payload": [],
    }
    base.update(fields)
    return base


# Events in a valid emission order (ids 0.., (tick, phase) non-decreasing, causes earlier), so the
# Rust test appends them to an EventLog too. They cover a place present and absent (and u32::MAX), a
# negative magnitude and both i32 extremes, each secrecy (0-3), each origin (0, 1, 3, 4), all four
# role codes with a repeated witness, snapshots with and without a partner and of the event, edge
# kinds 0-5 with and without a motive, a negative weight, and i64 payload extremes.
EVENT_CASES: list[Event] = [
    _event(
        id=0,
        tick=0,
        phase=0,
        emitter="M2.demography",
        kind="DEATH",
        roles=[(1, ("person", 7))],
        payload=[1],
    ),
    _event(
        id=1,
        tick=1_440,
        phase=4,
        emitter="M2.demography",
        kind="EXTINCTION",
        origin=3,
        causes=[(0, 1, None, 0, 256)],
    ),
    _event(
        id=2,
        tick=1_440,
        phase=5,
        emitter="T1.minds",
        kind="MENTAL_BREAK",
        place=17,
        magnitude=-42,
        secrecy=1,
        origin=1,
        roles=[(0, ("person", 9)), (2, ("person", 3)), (2, ("person", 4))],
        snapshots=[("stress", ("person", 9), None, 18_000)],
        causes=[(0, 3, "GRIEF_FOR", 300, 120), (1, 0, None, 0, 256)],
        payload=[-5, 7],
    ),
    _event(
        id=3,
        tick=2_000,
        phase=5,
        emitter="T1.minds",
        kind="MURDER_ATTEMPT",
        place=0,
        secrecy=2,
        origin=4,
        roles=[(0, ("person", 9)), (1, ("person", 7)), (3, ("household", 2))],
        snapshots=[
            ("stress", ("person", 9), None, -1),
            ("trust", ("person", 9), ("person", 7), -300),
        ],
        causes=[(1, 3, None, -77, 64), (2, 1, None, 0, 256)],
    ),
    _event(
        id=4,
        tick=2_000,
        phase=11,
        emitter="X1.weather",
        kind="FROST",
        magnitude=-(2**31),
        secrecy=3,
        roles=[(1, ("region", 2))],
        snapshots=[("weather", ("world", 0), None, 2)],
        causes=[(1, 2, None, 0, 200), (2, 5, None, 0, 0), (3, 4, None, 0, 100)],
    ),
    _event(
        id=5,
        tick=2**40,
        phase=0,
        emitter="X4.census",
        kind="CENSUS",
        place=2**32 - 1,
        magnitude=2**31 - 1,
        roles=[(1, ("settlement", 2**64 - 1))],
        causes=[(4, 3, "RETALIATION_FOR", -(2**15), 256), (0, 3, "CORRUPTION_BY", 2**15 - 1, 0)],
        payload=[-(2**63), 2**63 - 1, 0],
    ),
]
for _e in EVENT_CASES:
    _e["causes"].sort(key=lambda c: c[0])


def _event_line(e: Event) -> str:
    """One ``event`` line: the fields as tokens (``-`` for none), then the digest."""

    def ent(x: tuple[str, int]) -> str:
        return f"{x[0]}:{x[1]}"

    roles = ",".join(f"{r}/{ent(x)}" for r, x in e["roles"]) or "-"
    snaps = (
        ",".join(
            f"{a}/{ent(x)}/{'-' if p is None else ent(p)}/{v}" for a, x, p, v in e["snapshots"]
        )
        or "-"
    )
    causes = (
        ",".join(f"{c}/{k}/{'-' if m is None else m}/{w}/{ev}" for c, k, m, w, ev in e["causes"])
        or "-"
    )
    payload = ",".join(str(v) for v in e["payload"]) or "-"
    place = "-" if e["place"] is None else str(e["place"])
    fields = [
        "event",
        str(e["id"]),
        str(e["tick"]),
        str(e["phase"]),
        e["emitter"],
        e["kind"],
        place,
        str(e["magnitude"]),
        str(e["secrecy"]),
        str(e["origin"]),
        roles,
        snaps,
        causes,
        payload,
        f"{event_digest(e):016x}",
    ]
    return " ".join(fields)


# One-field mutations of each field kind: each must change the digest (checked here, and by the
# Rust test on the same mutations).
def _mutations(e: Event) -> list[tuple[str, Event]]:
    out = []

    def mut(name: str, f: Callable[[Event], object]) -> None:
        m = copy.deepcopy(e)
        f(m)
        out.append((name, m))

    mut("id", lambda m: m.update(id=m["id"] + 1))
    mut("tick", lambda m: m.update(tick=m["tick"] + 1))
    mut("phase", lambda m: m.update(phase=m["phase"] ^ 1))
    mut("emitter", lambda m: m.update(emitter=m["emitter"] + "x"))
    mut("kind", lambda m: m.update(kind=m["kind"] + "X"))
    mut("place", lambda m: m.update(place=None if m["place"] is not None else 0))
    mut("magnitude", lambda m: m.update(magnitude=m["magnitude"] ^ 1))
    mut("secrecy", lambda m: m.update(secrecy=(m["secrecy"] + 1) % 4))
    mut("origin", lambda m: m.update(origin=1 if m["origin"] == 0 else 0))
    mut("payload", lambda m: m.update(payload=[*m["payload"], 0]))
    if e["roles"]:
        r, x = e["roles"][0]
        mut("role", lambda m: m["roles"].__setitem__(0, ((r + 1) % 4, x)))
    if e["snapshots"]:
        a, x, p, v = e["snapshots"][0]
        mut("snapshot value", lambda m: m["snapshots"].__setitem__(0, (a, x, p, v ^ 1)))
        mut("snapshot attr", lambda m: m["snapshots"].__setitem__(0, (a + "x", x, p, v)))
        mut(
            "snapshot partner",
            lambda m: m["snapshots"].__setitem__(0, (a, x, None if p else ("world", 0), v)),
        )
    if e["causes"]:
        c, k, mo, w, ev = e["causes"][0]
        mut("edge kind", lambda m: m["causes"].__setitem__(0, (c, (k + 1) % 6, mo, w, ev)))
        mut("edge weight", lambda m: m["causes"].__setitem__(0, (c, k, mo, w ^ 1, ev)))
        mut("edge evidence", lambda m: m["causes"].__setitem__(0, (c, k, mo, w, ev ^ 1)))
        mut(
            "edge motive",
            lambda m: m["causes"].__setitem__(0, (c, k, None if mo else "FEAR_OF", w, ev)),
        )
    return out


def event_self_test() -> list[str]:
    """The SplitMix64 known answer and the one-field mutation rule (empty when both hold)."""
    failures = []
    if sm_mix(SM_GAMMA) != KAT_SPLITMIX64_SEED0:
        failures.append(f"SplitMix64(0) first output = {sm_mix(SM_GAMMA):016x}")
    for e in EVENT_CASES:
        d = event_digest(e)
        for name, m in _mutations(e):
            if event_digest(m) == d:
                failures.append(f"event {e['id']}: a mutated {name} keeps the digest")
    return failures


def render_events() -> str:
    """The event digest golden file's full text."""
    running = 0
    for e in EVENT_CASES:
        running = h64_words(T_EVENT, [running, event_digest(e)])
    header = [
        "# rh-events golden vectors: Event::digest v2 and EventLog::hash (envelope_v0.md §6).",
        "# GENERATED by bin/gen_hash_golden.py; do not edit.",
        "#   event <id> <tick> <phase> <emitter> <kind> <place|-> <magnitude> <secrecy> <origin>",
        "#         <roles: role/kind:raw,..|-> <snapshots: attr/kind:raw/partner|-/value,..|->",
        "#         <causes: cause/kind/motive|-/weight/evidence,..|-> <payload: v,..|-> <digest>",
        "#   log <n> <EventLog::hash of the events appended in order>",
    ]
    lines = [_event_line(e) for e in EVENT_CASES]
    log = h64_words(T_EVENT, [len(EVENT_CASES), running])
    return "\n".join([*header, *lines, f"log {len(EVENT_CASES)} {log:016x}"]) + "\n"


def render() -> str:
    """The golden file's full text."""
    header = [
        "# rh-snapshot golden vectors: XXH3 and the hash layout rehoboam/rh-snapshot/hash/v1.",
        "# GENERATED by bin/gen_hash_golden.py; do not edit.",
        "# Hashes are lowercase hex; XXH3-128 is high half first. Formats:",
        "#   xxh3_64 <len> <hash of input(len)>     input byte j = (131 j + len) mod 256",
        "#   xxh3_128 <len> <hash of input(len)>",
        "#   column <name> <salt> <rows> <root> <chunks..>  row i = ((i + 1) * GAMMA) ^ salt",
        "#   bytes <name> <rows> <root> <chunks..>   row i = (i mod 7) bytes of value i mod 256",
        "#   (GAMMA = 0x9e3779b97f4a7c15; names are UTF-8)",
        "#   plane <A|B|C> <t> (<name> <salt> <rows>)*t <plane hash>  (tables are columns)",
        "#   engine_table <name> <rows> <root> <chunks..>  the synthetic engine state's tables",
        "#   engine <plane hash>   its state_hash: the synthetic world columns and engine tables,",
        "#   rh-core/events.held excepted (a carried table: in a keyframe, outside the hash)",
        "#   engine_cause_table, engine_cause: two timers with a cause (SPR-014 T-02)",
    ]
    return (
        "\n".join(
            [
                *header,
                *lines_xxh3(),
                *lines_tables(),
                *lines_planes(),
                *lines_engine(),
                *lines_engine_cause(),
            ]
        )
        + "\n"
    )


def _check(path: Path, text: str) -> bool:
    try:
        current = path.read_text(encoding="utf-8")
    except OSError as e:
        print(f"STALE: cannot read {path}: {e}", file=sys.stderr)
        return False
    if current != text:
        print(f"STALE: {path} differs from the reference", file=sys.stderr)
        return False
    print(f"OK: {path} matches the reference ({text.count(chr(10))} lines)")
    return True


def main(argv: list[str] | None = None) -> int:
    """Write or check the golden files."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if a committed file is stale")
    parser.add_argument(
        "--golden", type=Path, default=GOLDEN, help="the hash file to write or check"
    )
    parser.add_argument(
        "--events-golden",
        type=Path,
        default=EVENTS_GOLDEN,
        help="the event digest file to write or check",
    )
    args = parser.parse_args(argv)
    failures = self_test() + event_self_test()
    if failures:
        for f in failures:
            print(f"SELF-TEST FAIL: {f}", file=sys.stderr)
        return EXIT_SELFTEST
    files = [(args.golden, render()), (args.events_golden, render_events())]
    if args.check:
        ok = [_check(path, text) for path, text in files]
        return EXIT_OK if all(ok) else EXIT_STALE
    for path, text in files:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        print(f"wrote {path}")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
