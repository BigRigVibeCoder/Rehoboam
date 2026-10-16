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
    ``XXH3(D_TABLE || u64(n) || u64(k) || u64(chunk(0)) .. u64(chunk(k-1)))``
Plane hash
    ``XXH3(D_PLANE || u8(plane letter) || u64(t) || (u32(len(name)) || name || u64(root))*)``
    over the tables in strictly increasing byte order of their names.
``D_CHUNK``, ``D_TABLE``, ``D_PLANE`` are ``b"rehoboam/rh-snapshot/hash/v1/"`` followed by
``chunk``, ``table`` and ``plane``.

Usage:  python -I bin/gen_hash_golden.py           (rewrite the golden file)
        python -I bin/gen_hash_golden.py --check   (exit 1 if the committed file differs)
        python -I bin/gen_hash_golden.py --check --golden PATH   (check another file; the positive
                                                                  control uses a stale fixture)
Exit:   0 ok, 1 stale golden file, 2 the reference failed its own known-answer self-test.
Standard library only (GOV-003 §3).
"""

import argparse
import itertools
import sys
from pathlib import Path

GOLDEN = (
    Path(__file__).resolve().parent.parent
    / "crates/rh-snapshot/tests/golden/rh_snapshot_hash_v1.txt"
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


def table_digest(rows: list[bytes]) -> tuple[list[int], int]:
    """Layout level 3: the chunk hashes and the root of a table given as encoded rows."""
    chunks = [chunk_hash(rows, c) for c in range(-(-len(rows) // CHUNK_ROWS))]
    body = b"".join(_u64(c) for c in chunks)
    root = xxh3_64(D_TABLE + _u64(len(rows)) + _u64(len(chunks)) + body)
    return chunks, root


def plane_hash(plane: str, tables: list[tuple[str, int]]) -> int:
    """Layout level 4 over ``(name, root)`` pairs in strictly increasing byte order of name."""
    names = [name.encode() for name, _ in tables]
    if any(a >= b for a, b in itertools.pairwise(names)):
        raise ValueError("table names out of byte order")
    pairs = zip(names, tables, strict=True)
    body = b"".join(_u32(len(n)) + n + _u64(root) for n, (_, root) in pairs)
    return xxh3_64(D_PLANE + plane.encode() + _u64(len(tables)) + body)


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
    for salt, n in COLUMN_CASES:
        chunks, root = table_digest(column_rows(salt, n))
        hexes = [f"{root:016x}", *(f"{c:016x}" for c in chunks)]
        out.append(" ".join(["column", str(salt), str(n), *hexes]))
    for n in BYTES_CASES:
        chunks, root = table_digest(bytes_rows(n))
        hexes = [f"{root:016x}", *(f"{c:016x}" for c in chunks)]
        out.append(" ".join(["bytes", str(n), *hexes]))
    return out


def lines_planes() -> list[str]:
    """``plane`` vectors over synthetic columns."""
    out = []
    for plane, tables in PLANE_CASES:
        roots = [(name, table_digest(column_rows(salt, n))[1]) for name, salt, n in tables]
        fields = [f"{name} {salt} {n}" for name, salt, n in tables]
        h = plane_hash(plane, roots)
        out.append(" ".join(["plane", plane, str(len(tables)), *fields, f"{h:016x}"]))
    return out


def render() -> str:
    """The golden file's full text."""
    header = [
        "# rh-snapshot golden vectors: XXH3 and the hash layout rehoboam/rh-snapshot/hash/v1.",
        "# GENERATED by bin/gen_hash_golden.py; do not edit.",
        "# Hashes are lowercase hex; XXH3-128 is high half first. Formats:",
        "#   xxh3_64 <len> <hash of input(len)>     input byte j = (131 j + len) mod 256",
        "#   xxh3_128 <len> <hash of input(len)>",
        "#   column <salt> <rows> <root> <chunks..> row i = ((i + 1) * 0x9e3779b97f4a7c15) ^ salt",
        "#   bytes <rows> <root> <chunks..>         row i = (i mod 7) bytes of value i mod 256",
        "#   plane <A|B|C> <t> (<name> <salt> <rows>)*t <plane hash>  (tables are columns)",
    ]
    return "\n".join([*header, *lines_xxh3(), *lines_tables(), *lines_planes()]) + "\n"


def main(argv: list[str] | None = None) -> int:
    """Write or check the golden file."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the file to write or check")
    args = parser.parse_args(argv)
    failures = self_test()
    if failures:
        for f in failures:
            print(f"SELF-TEST FAIL: {f}", file=sys.stderr)
        return EXIT_SELFTEST
    text = render()
    if args.check:
        try:
            current = args.golden.read_text(encoding="utf-8")
        except OSError as e:
            print(f"STALE: cannot read {args.golden}: {e}", file=sys.stderr)
            return EXIT_STALE
        if current != text:
            print(f"STALE: {args.golden} differs from the reference", file=sys.stderr)
            return EXIT_STALE
        print(f"OK: {args.golden} matches the reference ({text.count(chr(10))} lines)")
        return EXIT_OK
    args.golden.parent.mkdir(parents=True, exist_ok=True)
    args.golden.write_text(text, encoding="utf-8")
    print(f"wrote {args.golden}")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
