"""Generate the golden vectors of rh-core::fixed from an exact rational reference (SPR-010 T-02).

Every expected value is computed with ``fractions.Fraction``: the exact result of the operation on
the exact values the bits stand for, rounded once with Python's ``round(Fraction)`` (round half to
even, the DECISION in ``crates/rh-core/src/fixed/round.rs``), then clamped to the storage type. No
rounding code of the engine is reused, so the vectors are an independent oracle.

Inputs are the edge values of each type (bounds, zero, one step, one half, ties) crossed with each
other, plus pseudo-random values from a SplitMix64 with a fixed seed. SplitMix64 is used instead of
``random`` because its sequence is defined by its arithmetic, not by a Python version.

Output: ``crates/rh-core/tests/golden/fixed_vectors.txt``, one vector per line,
``<op> <input>... <expected>``, all as decimal integers (raw bits for Q types, minor units for
money); ``none`` is an expected ``None``. Read by ``crates/rh-core/tests/fixed_golden.rs``.

Usage:  python -I bin/gen_fixed_vectors.py           (write the file)
        python -I bin/gen_fixed_vectors.py --check   (exit 1 if the committed file is stale)
Standard library only (GOV-003 §3). Refs: SPR-010 T-02, T-03; DET-3; V-DET-19; GOV-002 §2 tier 5.
"""

import argparse
import sys
from collections.abc import Callable, Iterator, Sequence
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
OUTPUT = REPO / "crates" / "rh-core" / "tests" / "golden" / "fixed_vectors.txt"
SEED = 0x5EED_F1CE_D010
RANDOM_CASES = 192
MASK64 = (1 << 64) - 1


class SplitMix64:
    """SplitMix64 (Steele, Lea and Flood 2014): a fixed, version-independent input stream."""

    def __init__(self, seed: int) -> None:
        """Start the stream at ``seed``."""
        self.state = seed & MASK64

    def next_u64(self) -> int:
        """Return the next 64-bit output."""
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK64
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK64
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK64
        return z ^ (z >> 31)

    def signed(self, bits: int) -> int:
        """Return a uniform signed integer of ``bits`` bits."""
        value = self.next_u64() >> (64 - bits)
        return value - (1 << bits) if value >= 1 << (bits - 1) else value


@dataclass(frozen=True)
class Kind:
    """A storage type: its name in the vector file, its bit width and its fractional bits."""

    name: str
    bits: int
    frac: int

    @property
    def lo(self) -> int:
        """The smallest raw value."""
        return -(1 << (self.bits - 1))

    @property
    def hi(self) -> int:
        """The largest raw value."""
        return (1 << (self.bits - 1)) - 1

    def clamp(self, value: int) -> int:
        """Saturate an exact raw value to the type."""
        return max(self.lo, min(self.hi, value))

    def value(self, raw: int) -> Fraction:
        """Return the exact value that raw bits stand for."""
        return Fraction(raw, 1 << self.frac)

    def nearest(self, exact: Fraction) -> int:
        """Return the raw bits nearest ``exact`` (half to even), unclamped."""
        return round(exact * (1 << self.frac))

    def edges(self) -> list[int]:
        """Return the edge raw values: bounds, zero, one step, one half, one, and ties."""
        one, half = 1 << self.frac, 1 << (self.frac - 1)
        core = [0, 1, 2, 3, half - 1, half, half + 1, one - 1, one, one + 1, 3 * one + half]
        bounds = [self.lo, self.lo + 1, self.hi - 1, self.hi]
        return sorted(v for v in {*core, *(-c for c in core), *bounds} if self.lo <= v <= self.hi)


Q8 = Kind("q8", 16, 8)
Q16 = Kind("q16", 32, 16)
MONEY = Kind("money", 64, 0)


def fmt(value: int | None) -> str:
    """Format one expected value."""
    return "none" if value is None else str(value)


def pairs(kind: Kind, rng: SplitMix64) -> Iterator[tuple[int, int]]:
    """Yield every pair of edge values, then random pairs over the whole range and near one."""
    edges = kind.edges()
    yield from ((a, b) for a in edges for b in edges)
    small = kind.frac + 3  # values within about +-4, where products do not all saturate
    for i in range(RANDOM_CASES):
        width = kind.bits if i % 2 == 0 else small
        yield rng.signed(width), rng.signed(kind.bits if i % 3 == 0 else small)


def q_binary(kind: Kind, a: int, b: int) -> list[str]:
    """Return the add, sub, mul and div vectors of one pair of Q values."""
    va, vb = kind.value(a), kind.value(b)
    div = None if b == 0 else kind.clamp(kind.nearest(va / vb))
    return [
        f"{kind.name}.add {a} {b} {kind.clamp(a + b)}",
        f"{kind.name}.sub {a} {b} {kind.clamp(a - b)}",
        f"{kind.name}.mul {a} {b} {kind.clamp(kind.nearest(va * vb))}",
        f"{kind.name}.div {a} {b} {fmt(div)}",
    ]


def q_ratio(kind: Kind, numerator: int, denominator: int) -> str:
    """Return the from_ratio vector: None for a zero denominator or an out-of-range result."""
    if denominator == 0:
        return f"{kind.name}.ratio {numerator} {denominator} none"
    raw = kind.nearest(Fraction(numerator, denominator))
    in_range = kind.lo <= raw <= kind.hi
    return f"{kind.name}.ratio {numerator} {denominator} {fmt(raw if in_range else None)}"


def q_vectors(kind: Kind, other: Kind, rng: SplitMix64) -> list[str]:
    """Return every vector of one Q type, including its conversion to the other Q type."""
    lines: list[str] = []
    for a, b in pairs(kind, rng):
        lines += q_binary(kind, a, b)
    singles = [*kind.edges(), *(rng.signed(kind.bits) for _ in range(RANDOM_CASES))]
    convert = "widen" if other.frac > kind.frac else "narrow"
    for a in singles:
        lines.append(f"{kind.name}.neg {a} {kind.clamp(-a)}")
        lines.append(f"{kind.name}.{convert} {a} {other.clamp(other.nearest(kind.value(a)))}")
    i64 = (1 << 63) - 1
    numerators = [0, 1, -1, 2, 3, 7, -7, 127, 128, -128, 32767, 32768, -32768, i64, -i64 - 1]
    denominators = [0, 1, -1, 2, -2, 3, 7, 256, 512, 65536, 131072, i64, -i64 - 1]
    lines += [q_ratio(kind, n, d) for n in numerators for d in denominators]
    for _ in range(RANDOM_CASES):
        lines.append(q_ratio(kind, rng.signed(24), rng.signed(16) or 1))
    return lines


def money_vectors(rng: SplitMix64) -> list[str]:
    """Return every Money vector: add, sub, neg, mul_int, mul_rate and div_int."""
    clamp = MONEY.clamp
    edges = [MONEY.lo, MONEY.lo + 1, -(10**12), -7, -5, -1, 0, 1, 2, 5, 7, 10**12, MONEY.hi - 1]
    amounts = [*edges, MONEY.hi]
    amounts += [rng.signed(64 if i % 2 else 40) for i in range(RANDOM_CASES // 3)]
    counts = [0, 1, -1, 2, -2, 3, 4, 7, -7, 1000, MONEY.lo, MONEY.hi]
    rates = [*Q16.edges(), *(rng.signed(20) for _ in range(16))]
    lines: list[str] = []
    for i, a in enumerate(amounts):
        b = amounts[(i * 7 + 3) % len(amounts)]
        lines.append(f"money.add {a} {b} {clamp(a + b)}")
        lines.append(f"money.sub {a} {b} {clamp(a - b)}")
        lines.append(f"money.neg {a} {clamp(-a)}")
        for n in counts:
            lines.append(f"money.mul_int {a} {n} {clamp(a * n)}")
            div = None if n == 0 else clamp(round(Fraction(a, n)))
            lines.append(f"money.div_int {a} {n} {fmt(div)}")
        for r in rates:
            lines.append(f"money.mul_rate {a} {r} {clamp(round(a * Q16.value(r)))}")
    return lines


def draw_vectors(rng: SplitMix64) -> list[str]:
    """Return the draw-against-probability vectors: hit iff ``draw / 2^64 < p``, exactly (T-03)."""
    probabilities = [*Q16.edges(), *(rng.signed(18) for _ in range(32))]
    lines: list[str] = []
    for p in probabilities:
        block = max(0, min(p, 1 << 16)) << 48  # the first draw that misses, for 0 < p < 1
        draws = {0, 1, (1 << 48) - 1, 1 << 48, MASK64 - 1, MASK64}
        draws |= {rng.next_u64() for _ in range(8)}
        draws |= {d for d in (block - 1, block, block + 1) if 0 <= d <= MASK64}
        for d in sorted(draws):
            hit = Fraction(d, 1 << 64) < Q16.value(p)
            lines.append(f"draw {d} {p} {int(hit)}")
    return lines


SECTIONS: tuple[Callable[[SplitMix64], list[str]], ...] = (
    lambda rng: q_vectors(Q8, Q16, rng),
    lambda rng: q_vectors(Q16, Q8, rng),
    money_vectors,
    draw_vectors,
)


def render() -> str:
    """Return the full text of the vector file."""
    rng = SplitMix64(SEED)
    header = [
        "# Golden vectors for rh-core::fixed. GENERATED by bin/gen_fixed_vectors.py; never edit.",
        "# Exact reference: fractions.Fraction, rounded half to even, then saturated (SPR-010).",
        "# <op> <input>... <expected>; Q values are raw bits, money is minor units, none is None.",
    ]
    body = [line for section in SECTIONS for line in section(rng)]
    return "\n".join([*header, *body]) + "\n"


def main(argv: Sequence[str] | None = None) -> int:
    """Write the vector file, or with ``--check`` verify that it is current."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    args = parser.parse_args(argv)
    text = render()
    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != text:
            print(f"gen_fixed_vectors: {OUTPUT} is stale; run python -I bin/gen_fixed_vectors.py")
            return 1
        print(f"gen_fixed_vectors: current ({text.count(chr(10)) - 3} vectors)")
        return 0
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(text, encoding="utf-8")
    print(f"gen_fixed_vectors: wrote {OUTPUT} ({text.count(chr(10)) - 3} vectors)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
