"""Reference implementation of the rh-ids keyed hash and RNG; writes their golden vectors (SPR-002).

This is an independent, standard-library re-implementation of the byte layouts that
``crates/rh-ids`` documents. It generates ``crates/rh-ids/tests/golden/rh_ids_v1.txt``; the Rust
tests (``crates/rh-ids/tests/golden.rs``) must reproduce every line bit for bit. Two independent
implementations agreeing is the known-answer test (V-DET-09 style); the ChaCha core itself is first
checked here against published test vectors. Golden vectors are generated, never hand-typed.

Layout, version 1 (all integers little-endian; ``i64`` as its two's-complement ``u64``):

ChaCha8
    The original (D. J. Bernstein) ChaCha with 8 rounds, a 64-bit block counter in state words
    12-13 and a 64-bit nonce ("stream") in words 14-15. This is what ``rand_chacha`` implements.

H (T-02)
    ``K0 = le64(world_seed) || b"rehoboam/rh-ids/H/v1" || 4 zero bytes`` (32 bytes)
    ``PRF(K, nonce, block)`` = the first 32 bytes of ChaCha8 block ``block`` under key ``K``
    ``K1 = PRF(K0, nonce=branch_salt, block=system_id)``
    ``K2 = PRF(K1, nonce=purpose_id, block=entity_id)``
    The stream for ``(world_seed, branch_salt, system_id, purpose_id, entity_id, t)`` is ChaCha8
    under ``K2`` with nonce ``t``; ``H(..., t, k)`` is the u64 made of stream words ``2k`` (low)
    and ``2k + 1`` (high).

SplitMix64 scalar hash (T-03)
    ``mix`` is the SplitMix64 output function, ``GAMMA = 0x9e3779b97f4a7c15``.
    ``h64_words(domain, w[0..n])``: ``h = mix(domain ^ n)``; then ``h = mix((h + GAMMA) ^ w)``.
    ``h64_bytes(domain, b)``: the same over 8-byte little-endian chunks (last zero-padded), with
    ``n`` the byte length. ``tag(name) = h64_bytes(0, name)``.
    ``purpose_id(name) = h64_bytes(tag("rehoboam/rh-ids/purpose/v1"), name)``.
    Light stream draw ``k`` = ``mix(base + (k + 1) * GAMMA)``, with
    ``base = h64_words(tag("rehoboam/rh-ids/light/v1"), [seed, salt, system, purpose, entity, t])``.

Usage:  python -I bin/gen_rng_golden.py           (rewrite the golden file)
        python -I bin/gen_rng_golden.py --check   (exit 1 if the committed file differs)
        python -I bin/gen_rng_golden.py --check --golden PATH   (check another file; the positive
                                                                 control uses a stale fixture)
Exit:   0 ok, 1 stale golden file, 2 the reference failed its own known-answer self-test.
Standard library only (GOV-003 §3).
"""

import argparse
import sys
from collections.abc import Iterator
from pathlib import Path

GOLDEN = Path(__file__).resolve().parent.parent / "crates/rh-ids/tests/golden/rh_ids_v1.txt"
EXIT_OK = 0
EXIT_STALE = 1
EXIT_SELFTEST = 2

MASK32 = 0xFFFF_FFFF
MASK64 = 0xFFFF_FFFF_FFFF_FFFF
# "expand 32-byte k", the ChaCha constant words.
CHACHA_CONSTANTS = (0x6170_7865, 0x3320_646E, 0x7962_2D32, 0x6B20_6574)
CHACHA8_ROUNDS = 8
CHACHA20_ROUNDS = 20
BLOCK_BYTES = 64
KEY_BYTES = 32
DOMAIN_H_V1 = b"rehoboam/rh-ids/H/v1\x00\x00\x00\x00"

# Published known answers for the all-zero key, nonce 0, block 0.
# REF: RFC 8439 Appendix A.1 test vector #1 (ChaCha20; the zero counter and nonce make the 64/64
#      and 32/96 layouts coincide); draft-strombergson-chacha-test-vectors-01 TC1 (ChaCha8,
#      256-bit key).
KAT_CHACHA20_ZERO = (
    "76b8e0ada0f13d90405d6ae55386bd28bdd219b8a08ded1aa836efcc8b770dc7"
    "da41597c5157488d7724e03fb8d84a376a43b8f41518a11cc387b669b2ee6586"
)
KAT_CHACHA8_ZERO = (
    "3e00ef2f895f40d67f5bb8e81f09a5a12c840ec3ce9a7f3b181be188ef711a1e"
    "984ce172b9216f419f445367456d5619314a42a3da86b001387bfdb80e0cfe42"
)


def _rotl32(x: int, n: int) -> int:
    return ((x << n) | (x >> (32 - n))) & MASK32


def _quarter_round(s: list[int], a: int, b: int, c: int, d: int) -> None:
    s[a] = (s[a] + s[b]) & MASK32
    s[d] = _rotl32(s[d] ^ s[a], 16)
    s[c] = (s[c] + s[d]) & MASK32
    s[b] = _rotl32(s[b] ^ s[c], 12)
    s[a] = (s[a] + s[b]) & MASK32
    s[d] = _rotl32(s[d] ^ s[a], 8)
    s[c] = (s[c] + s[d]) & MASK32
    s[b] = _rotl32(s[b] ^ s[c], 7)


def chacha_block(key: bytes, counter: int, nonce: int, rounds: int = CHACHA8_ROUNDS) -> bytes:
    """Return one 64-byte ChaCha block (64-bit counter, 64-bit nonce)."""
    if len(key) != KEY_BYTES:
        raise ValueError("ChaCha key must be 32 bytes")
    key_words = [int.from_bytes(key[i : i + 4], "little") for i in range(0, KEY_BYTES, 4)]
    counter &= MASK64
    nonce &= MASK64
    state = [*CHACHA_CONSTANTS, *key_words, counter & MASK32, counter >> 32, nonce & MASK32]
    state.append(nonce >> 32)
    work = state.copy()
    for _ in range(rounds // 2):
        _quarter_round(work, 0, 4, 8, 12)
        _quarter_round(work, 1, 5, 9, 13)
        _quarter_round(work, 2, 6, 10, 14)
        _quarter_round(work, 3, 7, 11, 15)
        _quarter_round(work, 0, 5, 10, 15)
        _quarter_round(work, 1, 6, 11, 12)
        _quarter_round(work, 2, 7, 8, 13)
        _quarter_round(work, 3, 4, 9, 14)
    return b"".join(
        ((w + s) & MASK32).to_bytes(4, "little") for w, s in zip(work, state, strict=True)
    )


def prf(key: bytes, nonce: int, block: int) -> bytes:
    """The cascade step: the first 32 bytes of ChaCha8 block ``block`` with nonce ``nonce``."""
    return chacha_block(key, block, nonce)[:KEY_BYTES]


def seed_key(world_seed: int) -> bytes:
    """K0: the world seed and the version-1 domain tag."""
    return (world_seed & MASK64).to_bytes(8, "little") + DOMAIN_H_V1


def stream_key(world_seed: int, salt: int, system: int, purpose: int, entity: int) -> bytes:
    """K2, the key of the streams of one (salt, system, purpose, entity)."""
    k1 = prf(seed_key(world_seed), salt, system)
    return prf(k1, purpose, entity)


def stream_u64(key: bytes, t: int, k: int) -> int:
    """Draw ``k`` of the stream for tick ``t``: words 2k (low) and 2k + 1 (high)."""
    word = 2 * k
    block = chacha_block(key, word // 16, t)
    off = (word % 16) * 4
    return int.from_bytes(block[off : off + 8], "little")


# (world_seed, branch_salt, system_id, purpose_id, entity_id, t): every field of H but k.
RngKey = tuple[int, int, int, int, int, int]


def h(key: RngKey, k: int) -> int:
    """H(world_seed, branch_salt, system_id, purpose, entity_id, t, k) (ADR-0006 v1.1 item 2)."""
    world_seed, salt, system, purpose, entity, t = key
    return stream_u64(stream_key(world_seed, salt, system, purpose, entity), t, k)


GAMMA = 0x9E37_79B9_7F4A_7C15
MIX_MUL_1 = 0xBF58_476D_1CE4_E5B9
MIX_MUL_2 = 0x94D0_49BB_1331_11EB
WORD_BYTES = 8
# The first output of SplitMix64 from state 0 (REF: Vigna, splitmix64.c reference sequence).
KAT_SPLITMIX64_SEED0 = 0xE220_A839_7B1D_CDAF


def mix(z: int) -> int:
    """The SplitMix64 output function (Steele, Lea and Flood 2014)."""
    z &= MASK64
    z = ((z ^ (z >> 30)) * MIX_MUL_1) & MASK64
    z = ((z ^ (z >> 27)) * MIX_MUL_2) & MASK64
    return z ^ (z >> 31)


def h64_words(domain: int, words: list[int]) -> int:
    """The SplitMix64 scalar hash of 64-bit words under a domain tag."""
    acc = mix(domain ^ len(words))
    for w in words:
        acc = mix(((acc + GAMMA) & MASK64) ^ (w & MASK64))
    return acc


def h64_bytes(domain: int, data: bytes) -> int:
    """The SplitMix64 scalar hash of bytes (8-byte little-endian chunks, zero-padded)."""
    acc = mix(domain ^ len(data))
    for start in range(0, len(data), WORD_BYTES):
        chunk = data[start : start + WORD_BYTES].ljust(WORD_BYTES, b"\x00")
        acc = mix(((acc + GAMMA) & MASK64) ^ int.from_bytes(chunk, "little"))
    return acc


def tag(name: str) -> int:
    """A domain tag from an ASCII name."""
    return h64_bytes(0, name.encode("ascii"))


TAG_PURPOSE = tag("rehoboam/rh-ids/purpose/v1")
TAG_LIGHT = tag("rehoboam/rh-ids/light/v1")


def purpose_id(name: str) -> int:
    """The stable identifier of an RNG purpose name."""
    return h64_bytes(TAG_PURPOSE, name.encode("ascii"))


def light(key: RngKey, k: int) -> int:
    """Draw ``k`` of the light (SplitMix64) stream of ``key``."""
    base = h64_words(TAG_LIGHT, [x & MASK64 for x in key])
    return mix((base + (k + 1) * GAMMA) & MASK64)


def self_test() -> list[str]:
    """Check the ChaCha core against published vectors; return the failures."""
    failures = []
    if chacha_block(bytes(KEY_BYTES), 0, 0, CHACHA20_ROUNDS).hex() != KAT_CHACHA20_ZERO:
        failures.append("ChaCha20 all-zero block differs from RFC 8439 A.1 #1")
    if chacha_block(bytes(KEY_BYTES), 0, 0, CHACHA8_ROUNDS).hex() != KAT_CHACHA8_ZERO:
        failures.append("ChaCha8 all-zero block differs from Strombergson TC1")
    if mix(GAMMA) != KAT_SPLITMIX64_SEED0:
        failures.append("SplitMix64 first output from state 0 differs from the reference")
    return failures


def _splitmix_sequence(state: int) -> Iterator[int]:
    """A plain SplitMix64 sequence, used only to pick varied test keys for the vectors."""
    while True:
        state = (state + 0x9E37_79B9_7F4A_7C15) & MASK64
        z = state
        z = ((z ^ (z >> 30)) * 0xBF58_476D_1CE4_E5B9) & MASK64
        z = ((z ^ (z >> 27)) * 0x94D0_49BB_1331_11EB) & MASK64
        yield z ^ (z >> 31)


def _signed64(x: int) -> int:
    return x - (1 << 64) if x >> 63 else x


def lines_chacha() -> list[str]:
    """ChaCha8 blocks under varied keys, counters and nonces (checks rand_chacha's layout)."""
    pick = _splitmix_sequence(1)
    out = []
    cases = [
        (bytes(KEY_BYTES), 0, 0),
        (bytes(range(KEY_BYTES)), 1, 2),
        (bytes(KEY_BYTES), MASK64, MASK64),
    ]
    for _ in range(3):
        key = b"".join(next(pick).to_bytes(8, "little") for _ in range(4))
        cases.append((key, next(pick), next(pick)))
    for key, counter, nonce in cases:
        out.append(
            f"chacha8 {key.hex()} {counter} {nonce} {chacha_block(key, counter, nonce).hex()}"
        )
    return out


def lines_h() -> list[str]:
    """H over edge and varied keys, including adjacent entities, adjacent t and negative t."""
    keys = [
        (0, 0, 0, 0, 0, 0, 0),
        (0, 0, 0, 0, 0, 0, 1),
        (0, 0, 0, 0, 0, 1, 0),
        (0, 0, 0, 0, 1, 0, 0),
        (0, 0, 0, 1, 0, 0, 0),
        (0, 0, 1, 0, 0, 0, 0),
        (0, 1, 0, 0, 0, 0, 0),
        (1, 0, 0, 0, 0, 0, 0),
        (MASK64, MASK64, 0xFFFF_FFFF, MASK64, MASK64, -1, 7),
        (42, 0, 3, 0x1234_5678_9ABC_DEF0, 1_000_000, -525_600, 0),
        (42, 0, 3, 0x1234_5678_9ABC_DEF0, 1_000_000, -525_600, 8),  # crosses into block 1
        (42, 0, 3, 0x1234_5678_9ABC_DEF0, 1_000_000, -525_600, 1 << 40),
        (42, 0, 3, 0x1234_5678_9ABC_DEF0, 1_000_000, -(1 << 63), MASK64),
    ]
    pick = _splitmix_sequence(2)
    for _ in range(24):
        seed, salt, system, purpose, entity, t = (next(pick) for _ in range(6))
        keys.append((seed, salt, system & MASK32, purpose, entity, _signed64(t), next(pick) % 64))
    return [
        f"h {s} {b} {y} {p} {e} {t} {k} {h((s, b, y, p, e, t), k)}" for s, b, y, p, e, t, k in keys
    ]


def _varied_keys(seed: int, count: int) -> list[tuple[RngKey, int]]:
    pick = _splitmix_sequence(seed)
    out = []
    for _ in range(count):
        s, b, y, p, e, t = (next(pick) for _ in range(6))
        out.append(((s, b, y & MASK32, p, e, _signed64(t)), next(pick) % 1000))
    return out


def lines_splitmix() -> list[str]:
    """Scalar hashes, tags, purpose identifiers and light-stream draws."""
    out = [f"mix {z} {mix(z)}" for z in (0, 1, GAMMA, MASK64, 0x0123_4567_89AB_CDEF)]
    word_cases: list[tuple[int, list[int]]] = [(0, []), (0, [0]), (1, [0]), (0, [0, 0])]
    word_cases += [(7, [1, 2, 3]), (MASK64, [MASK64] * 5)]
    for domain, words in word_cases:
        fields = [domain, len(words), *words, h64_words(domain, words)]
        out.append("h64_words " + " ".join(map(str, fields)))
    names = ["", "a", "death_time", "activity_block", "encounter_outcome", "exactly8", "nine_char"]
    names += ["rehoboam/rh-ids/purpose/v1", "rehoboam/rh-ids/light/v1"]
    out += [f"tag {n or '-'} {tag(n)}" for n in names]
    purposes = ["death_time", "activity_block", "encounter_outcome", "birth_interval", "x"]
    out += [f"purpose {n} {purpose_id(n)}" for n in purposes]
    keys: list[tuple[RngKey, int]] = [((0, 0, 0, 0, 0, 0), 0), ((0, 0, 0, 0, 0, -1), MASK64)]
    keys += _varied_keys(3, 16)
    out += [f"light {' '.join(map(str, key))} {k} {light(key, k)}" for key, k in keys]
    return out


def render() -> str:
    """The whole golden file."""
    header = [
        "# rh-ids golden vectors, layout version 1.",
        "# GENERATED by bin/gen_rng_golden.py; do not edit.",
        "# Decimal integers; t is signed. Formats:",
        "#   chacha8 <key hex> <block counter> <nonce> <64-byte block hex>",
        "#   h <world_seed> <branch_salt> <system_id> <purpose_id> <entity_id> <t> <k> <u64>",
        "#   mix <z> <u64>",
        "#   h64_words <domain> <n> <w1> .. <wn> <u64>",
        "#   tag <name, '-' for empty> <u64>",
        "#   purpose <name> <purpose_id>",
        "#   light <world_seed> <branch_salt> <system_id> <purpose_id> <entity_id> <t> <k> <u64>",
    ]
    body = [*lines_chacha(), *lines_h(), *lines_splitmix()]
    return "\n".join([*header, *body]) + "\n"


def main(argv: list[str] | None = None) -> int:
    """Generate or check the golden file."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    parser.add_argument(
        "--golden", type=Path, default=GOLDEN, help="the golden file (default: %(default)s)"
    )
    args = parser.parse_args(argv)
    golden: Path = args.golden
    failures = self_test()
    if failures:
        for failure in failures:
            print(f"SELF-TEST FAILED: {failure}")
        return EXIT_SELFTEST
    text = render()
    if args.check:
        current = golden.read_text(encoding="utf-8") if golden.exists() else ""
        if current != text:
            print(
                f"STALE: {golden} differs from the reference; run python -I bin/gen_rng_golden.py"
            )
            return EXIT_STALE
        print(f"OK: {golden.name} matches the reference ({text.count(chr(10))} lines)")
        return EXIT_OK
    golden.parent.mkdir(parents=True, exist_ok=True)
    golden.write_text(text, encoding="utf-8")
    print(f"wrote {golden}")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
