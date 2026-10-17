"""Generate rh-manifest's authored-number vectors from an exact decimal reference (SPR-011 T-04).

The boundary of ``05.5`` §3.2: an authored number enters plane A as the exact decimal value of
its literal, rounded half to even to the fixed-point type, never through an ``f64``. This script
is the independent oracle for ``rh_manifest::exact``: each expected value is Python's
``decimal.Decimal`` of the literal, made a ``fractions.Fraction``, multiplied by ``2^F`` and
rounded once with ``round(Fraction)`` (half to even). No code of the engine is reused.

Literals are random over the three grammars the loaders hand over: a ``PyYAML`` float scalar
(``yaml``: underscores, ``.5``, ``6.``, signed exponents, sexagesimal ``1:30.5``, ``.inf``,
``.nan``), a string Pydantic coerces with Python's ``float()`` (``python``: whitespace, ``inf``),
and the text ``toml`` keeps for a TOML float (``toml``). Halfway cases are planted on purpose:
``(2j + 1) / 2^(F+1)`` written out exactly in decimal, with zeros after it, with one more digit
and just below it. A SplitMix64 with a fixed seed draws the inputs, so the file is defined by
arithmetic, not by a Python version.

Output: ``crates/rh-manifest/tests/golden/decimal_vectors.txt``, one vector per line,
``<grammar> <frac_bits> <literal> <expected>``: the rounded integer, ``none`` when its magnitude
is at least 2^126, ``notfinite`` for an infinity or NaN. Literals hold no spaces (a ``python``
literal's surrounding whitespace is written as ``~``). Read by
``crates/rh-manifest/tests/exact_golden.rs``.

Usage:  python -I bin/gen_decimal_vectors.py           (write the file)
        python -I bin/gen_decimal_vectors.py --check   (exit 1 if the committed file is stale)
        python -I bin/gen_decimal_vectors.py --check --output FILE   (check another file: a control)
Standard library only (GOV-003 §3). Refs: SPR-011 T-04, ruling 9; 05.5 §3.2.
"""

import argparse
import sys
from collections.abc import Sequence
from decimal import Decimal, localcontext
from fractions import Fraction
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
OUTPUT = REPO / "crates" / "rh-manifest" / "tests" / "golden" / "decimal_vectors.txt"
SEED = 0x5EED_DEC1_0A15
RANDOM_CASES = 400
FRAC_BITS = (16, 32)
LIMIT = 1 << 126
MASK64 = (1 << 64) - 1
MIN_UNDERSCORED = 2  # an underscore needs a digit on each side


class SplitMix64:
    """SplitMix64 (Steele, Lea and Flood 2014): a fixed, version-independent input stream."""

    def __init__(self, seed: int) -> None:
        """Start the stream at ``seed``."""
        self.state = seed & MASK64

    def next_u64(self) -> int:
        """Return the next output."""
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK64
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK64
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK64
        return z ^ (z >> 31)

    def below(self, n: int) -> int:
        """Return an integer in ``[0, n)``."""
        return self.next_u64() % n


def exact(grammar: str, literal: str) -> Fraction | None:
    """Return the literal's exact value under its grammar, or None for an infinity or NaN."""
    if grammar == "python":
        text = literal.replace("~", " ").strip().replace("_", "")
        if text.lower().lstrip("+-") in {"inf", "infinity", "nan"}:
            return None
    else:
        text = literal.replace("_", "").lower()
        if text.lstrip("+-") in {".inf", ".nan", "inf", "nan"}:
            return None
    if grammar == "yaml" and ":" in text:
        negative = text.startswith("-")
        groups = text.lstrip("+-").split(":")
        last, heads = groups[-1], groups[:-1]
        seconds, _, frac = last.partition(".")
        whole = 0
        for group in [*heads, seconds]:
            whole = whole * 60 + int(group)
        value = Fraction(whole) + Fraction(Decimal("0." + (frac or "0")))
        return -value if negative else value
    with localcontext() as context:
        context.prec = 10_000
        return Fraction(Decimal(text))


def expected(grammar: str, literal: str, frac_bits: int) -> str:
    """Return the expected vector value: the rounded integer, ``none`` or ``notfinite``."""
    value = exact(grammar, literal)
    if value is None:
        return "notfinite"
    rounded = round(value * (1 << frac_bits))
    return "none" if abs(rounded) >= LIMIT else str(rounded)


def digits(rng: SplitMix64, n: int) -> str:
    """Return ``n`` random decimal digits."""
    return "".join(str(rng.below(10)) for _ in range(n))


def underscored(rng: SplitMix64, run: str) -> str:
    """Insert single underscores between some digits (allowed in all three grammars)."""
    if len(run) < MIN_UNDERSCORED or rng.below(4):
        return run
    i = 1 + rng.below(len(run) - 1)
    return run[:i] + "_" + run[i:]


def random_literal(rng: SplitMix64) -> str:
    """Return a random decimal literal valid in every grammar: ``[-]d+.d+[e(+|-)d+]``."""
    sign = "-" if rng.below(3) == 0 else ""
    int_part = underscored(rng, digits(rng, 1 + rng.below(6)).lstrip("0") or "0")
    frac = underscored(rng, digits(rng, 1 + rng.below(24)))
    exponent = ""
    if rng.below(3) == 0:
        exponent = f"e{'+-'[rng.below(2)]}{rng.below(40)}"
    return f"{sign}{int_part}.{frac}{exponent}"


def ties(rng: SplitMix64, frac_bits: int) -> list[str]:
    """Return literals at exactly half a step, and just around it."""
    out = []
    for _ in range(24):
        odd = 2 * rng.below(1 << 20) + 1
        half = Fraction(odd, 1 << (frac_bits + 1))
        with localcontext() as context:
            context.prec = 200  # 2^-(F+1) has F+1 decimals: exact
            text = format(Decimal(half.numerator) / Decimal(half.denominator), "f")
        if "E" in text or "." not in text:
            continue
        sign = "-" if rng.below(2) else ""
        out += [f"{sign}{text}", f"{sign}{text}000", f"{sign}{text}1"]
        out.append(f"{sign}{text[:-1]}{int(text[-1]) - 1}9")  # just below the tie
    return out


def edge_literals() -> list[str]:
    """Return fixed literals: zero, signs, forms, and the range edges."""
    return [
        "0.0",
        "-0.0",
        "1.0",
        ".5",
        "6.",
        "+2.5",
        "30.0",
        "0.1",
        "1.0e+9",
        "1.0e-9",
        "32767.99998474121",
        "32768.0",
        "-32768.0",
        "1.0e+40",
        "1.0e+38",
        "1.0e-400",
        "123_456.789_012",
    ]


def yaml_only(rng: SplitMix64) -> list[str]:
    """Return PyYAML-only forms: sexagesimal floats and the special values."""
    out = [".inf", "-.Inf", ".NaN", "1:30.5", "-1:00:00.25", "190:20:30.15"]
    for _ in range(16):
        groups = [str(1 + rng.below(99))] + [f"{rng.below(60):02d}" for _ in range(rng.below(3))]
        out.append(f"{':'.join(groups)}:{rng.below(60):02d}.{digits(rng, 1 + rng.below(8))}")
    return out


def python_only(rng: SplitMix64) -> list[str]:
    """Return float(str)-only forms: whitespace (written ``~``), no point, infinities."""
    out = ["inf", "-Infinity", "nan", "~1.5~", "1e3", "7", "-12e-2"]
    out += [f"~{random_literal(rng)}" for _ in range(8)]
    return out


def toml_only(rng: SplitMix64) -> list[str]:
    """Return TOML-only forms: an exponent without a point, inf and nan."""
    out = ["inf", "-inf", "nan", "1e9", "6e-3"]
    out += [f"{1 + rng.below(9)}e{rng.below(30)}" for _ in range(8)]
    return out


def render() -> str:
    """Return the full text of the vector file."""
    rng = SplitMix64(SEED)
    shared = [*edge_literals(), *(random_literal(rng) for _ in range(RANDOM_CASES))]
    lines = [
        "# Authored-number vectors for rh-manifest::exact.",
        "# GENERATED by bin/gen_decimal_vectors.py; never edit.",
        "# Exact reference: decimal.Decimal -> Fraction, times 2^F, round half to even.",
        "# <grammar> <frac_bits> <literal> <expected>; `none` = |result| >= 2^126; `~` = a space.",
    ]
    for frac_bits in FRAC_BITS:
        literals = {
            "yaml": [*shared, *ties(rng, frac_bits), *yaml_only(rng)],
            "python": [*shared, *ties(rng, frac_bits), *python_only(rng)],
            "toml": [*shared, *ties(rng, frac_bits), *toml_only(rng)],
        }
        for grammar, items in literals.items():
            for literal in items:
                lines.append(
                    f"{grammar} {frac_bits} {literal} {expected(grammar, literal, frac_bits)}"
                )
    return "\n".join(lines) + "\n"


def vector_count(text: str) -> int:
    """Return the number of vectors (non-comment lines) in the file text."""
    return sum(1 for line in text.splitlines() if line and not line.startswith("#"))


def main(argv: Sequence[str] | None = None) -> int:
    """Write the vector file, or with ``--check`` verify that it is current."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    parser.add_argument(
        "--output", type=Path, default=OUTPUT, help="the vector file (default: the committed one)"
    )
    args = parser.parse_args(argv)
    output: Path = args.output
    text = render()
    if args.check:
        current = output.read_text(encoding="utf-8") if output.exists() else ""
        if current != text:
            print(f"STALE: {output}; run python -I bin/gen_decimal_vectors.py")
            return 1
        print(f"gen_decimal_vectors: current ({vector_count(text)} vectors)")
        return 0
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(text, encoding="utf-8")
    print(f"gen_decimal_vectors: wrote {output} ({vector_count(text)} vectors)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
