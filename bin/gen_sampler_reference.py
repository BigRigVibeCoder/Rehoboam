"""Generate the exact reference distributions for rh-sample's statistical tests (SPR-003).

The golden vectors of rh-sample (V-DET-09) only freeze what the samplers return; whether they are
right comes from statistical tests against the exact distributions, computed here independently
of the Rust code, with ``fractions.Fraction`` and ``decimal.Decimal`` only, so the file is the same
on every machine and Python version:

    chi2      Binomial(n, p) and Poisson(λ): the pmf by its exact rational recurrence (p is a Q16_16
              = b/2^16, λ likewise), evaluated in 60-digit Decimal from (1 - p)^n and from e^-λ
              (Decimal's exp is correctly rounded), merged into bins of expected count at least
              MIN_EXPECTED for the tier's sample size, with an even number of degrees of freedom,
              and the 99.9% quantile of χ²(df) (closed form for even df, solved by bisection).
    moments   The exact mean, variance and fourth central moment of Poisson, Gamma, Beta and
              Normal, for a z-test of the sample mean and the sample variance.
    tail      P(Z_q > t) for the standard normal rounded to Q16_16: P(Z > t + 2^-17) exactly
              (rounding half to even moves the threshold by half a step), from an erf series.
    table     The SplitMix64 hash (rh-ids's h64_words, domain 0) of the bits of the ziggurat tables,
              parsed from crates/rh-sample/src/ziggurat_tables.rs with Python's correctly rounded
              float(): it pins that rustc reads the literals to the same bits.

Each statistic exists for two tiers: ``pr`` (bin/check_all.sh) and ``nightly`` (64 times the
samples, .github/workflows/nightly.yml), so each tier has its own bins and critical values.

Output: ``crates/rh-sample/tests/golden/reference.txt``, read by ``crates/rh-sample/tests/stats.rs``
and ``tests/golden.rs``. Parameters are written as Q16_16 raw bits; probabilities and moments as
decimal strings with 20 significant digits.

Usage:  python -I bin/gen_sampler_reference.py           (write the file)
        python -I bin/gen_sampler_reference.py --check   (exit 1 if the committed file is stale)
Standard library only (GOV-003 §3). Refs: SPR-003 T-01..T-04; V-DET-09; ADR-0006 item 3.
"""

import argparse
import re
import struct
import sys
from collections.abc import Iterator, Sequence
from dataclasses import dataclass
from decimal import Decimal, localcontext
from fractions import Fraction
from math import isqrt
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
OUTPUT = REPO / "crates" / "rh-sample" / "tests" / "golden" / "reference.txt"
TABLES = REPO / "crates" / "rh-sample" / "src" / "ziggurat_tables.rs"
ONE = 1 << 16  # Q16_16 ONE in raw bits
PRECISION = 60  # Decimal digits for every computation
DIGITS = 20  # significant digits written
MIN_EXPECTED = 20  # smallest expected count per chi-square bin
MIN_DF = 2  # a case with fewer degrees of freedom tests nothing
ALPHA = Fraction(1, 1000)  # chi-square critical values are the 99.9% quantiles (as V-DET-20)
MASK64 = (1 << 64) - 1
# Samples per statistic, by tier. The nightly tier is 64 times the PR tier.
TIERS = {"pr": 1 << 16, "nightly": 1 << 22}
TAIL_TIERS = {"pr": 1 << 20, "nightly": 1 << 26}

# Binomial (n, p bits): BINV and BTPE, both orientations, and every BTPE branch (squeeze and
# recurrence) are reached.
BINOMIAL = [
    (5, 58982),
    (20, 32768),
    (30, 6554),
    (60, 16384),
    (4000, 64),
    (1000, 49152),
    (5000, 19661),
]
# Poisson λ bits: inversion (below 10, including the largest value below 10) and PTRS.
POISSON = [32768, 3 * ONE, 10 * ONE - 1, 10 * ONE, 25 * ONE, 300 * ONE, 5000 * ONE]
# Gamma (shape bits, scale bits): the boost (k < 1), k = 1, and k > 1.
GAMMA = [(ONE // 4, 2 * ONE), (ONE, ONE), (5 * ONE // 2, ONE // 2), (30 * ONE, 3 * ONE)]
# Beta (alpha bits, beta bits): U-shaped, skewed, concentrated, and a small shape (log path).
BETA = [(ONE // 2, ONE // 2), (2 * ONE, 5 * ONE), (30 * ONE, 10 * ONE), (ONE // 4, 3 * ONE)]
# Normal (mean bits, sd bits).
NORMAL = [(0, ONE), (5 * ONE, 3 * ONE), (-1000 * ONE, ONE // 8)]
# Standard-normal tail thresholds (bits): inside and beyond the ziggurat's base box (R ≈ 3.654).
TAIL = [ONE, 2 * ONE, 3 * ONE, 7 * ONE // 2, 15 * ONE // 4, 4 * ONE]
# The first 70 digits of pi.
PI = Decimal("3.141592653589793238462643383279502884197169399375105820974944592307816")


def fmt(value: Fraction | Decimal) -> str:
    """Return ``value`` with DIGITS significant digits, the same text everywhere."""
    with localcontext() as ctx:
        ctx.prec = PRECISION
        d = value if isinstance(value, Decimal) else Decimal(value.numerator) / value.denominator
        return f"{d:.{DIGITS - 1}e}"


def q(bits: int) -> Fraction:
    """Return the exact value of Q16_16 raw bits."""
    return Fraction(bits, ONE)


# --- Exact distributions ----------------------------------------------------------------------


def binomial_pmf(n: int, p_bits: int) -> list[Decimal]:
    """Return P(X = k) for k = 0..n: p(0) = (c/2^16)^n, p(k+1) = p(k)·(n-k)/(k+1)·b/c.

    With p = b/2^16 and c = 2^16 - b, every factor is an exact rational; Decimal evaluates the
    chain at PRECISION digits (relative error below 10^-50 for n = 5000).
    """
    b, c = p_bits, ONE - p_bits
    with localcontext() as ctx:
        ctx.prec = PRECISION
        term = (Decimal(c) / ONE) ** n
        pmf = [term]
        for k in range(n):
            term = term * (n - k) * b / ((k + 1) * c)
            pmf.append(term)
    return pmf


def poisson_pmf(lam_bits: int) -> tuple[list[Decimal], Decimal]:
    """Return P(X = k) for k = 0..K, K past the mean by 40 sd, and the mass above K.

    p(0) = e^-λ (Decimal's exp is correctly rounded), p(k) = p(k-1)·λ/k with λ = bits/2^16 exact.
    """
    lam = Decimal(lam_bits) / ONE  # exact: a dyadic rational
    top = lam_bits // ONE + 40 * (isqrt(lam_bits // ONE + 1) + 1) + 40
    with localcontext() as ctx:
        ctx.prec = PRECISION
        term = (-lam).exp()
        pmf = [term]
        for k in range(1, top + 1):
            term = term * lam / k
            pmf.append(term)
        rest = Decimal(1) - sum(pmf, Decimal(0))
    return pmf, rest if rest > 0 else Decimal(0)


def chi2_survival_even(df: int, x: Decimal) -> Decimal:
    """P(χ²(df) > x) for an even df: e^(-x/2) Σ_{i < df/2} (x/2)^i / i!."""
    half = x / 2
    term, total = Decimal(1), Decimal(1)
    for i in range(1, df // 2):
        term = term * half / i
        total += term
    return (-half).exp() * total


def chi2_critical(df: int) -> Decimal:
    """The (1 - ALPHA) quantile of χ²(df), df even, by bisection."""
    target = Decimal(ALPHA.numerator) / ALPHA.denominator
    lo, hi = Decimal(0), Decimal(df) + 100 * Decimal(df).sqrt() + 100
    for _ in range(200):
        mid = (lo + hi) / 2
        if chi2_survival_even(df, mid) > target:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


@dataclass(frozen=True)
class Bin:
    """An inclusive range of counts and its exact probability."""

    lo: int
    hi: int
    prob: Decimal


def bins(pmf: Sequence[Decimal], rest: Decimal, samples: int) -> list[Bin]:
    """Merge the pmf into bins of expected count >= MIN_EXPECTED; the last bin is open above.

    The number of bins is made odd (df = bins - 1 even) by merging the last two if needed.
    """
    out: list[Bin] = []
    lo, mass = 0, Decimal(0)
    for k, p in enumerate(pmf):
        mass += p
        if mass * samples >= MIN_EXPECTED:
            out.append(Bin(lo, k, mass))
            lo, mass = k + 1, Decimal(0)
    tail = mass + rest
    last = out.pop()
    out.append(Bin(last.lo, 0xFFFF_FFFF, last.prob + tail))
    if len(out) % 2 == 0:
        a, b = out.pop(-2), out.pop()
        out.append(Bin(a.lo, b.hi, a.prob + b.prob))
    return out


def erfc(x: Decimal) -> Decimal:
    """Return 1 - erf(x), with erf from its Maclaurin series at PRECISION + 20 digits."""
    with localcontext() as ctx:
        ctx.prec = PRECISION + 20
        total, term, n = Decimal(0), x, 0
        while True:
            piece = term / (2 * n + 1)
            total += piece
            if abs(piece) < Decimal(10) ** -(PRECISION + 15):
                break
            n += 1
            term = -term * x * x / n
        return 1 - 2 * total / PI.sqrt()


def normal_upper(t_bits: int) -> Decimal:
    """P(round_q16(Z) > t) = P(Z > t + 2^-17) = erfc((t + 2^-17)/√2)/2."""
    with localcontext() as ctx:
        ctx.prec = PRECISION + 20
        x = Decimal(2 * t_bits + 1) / (2 * ONE)
        return erfc(x / Decimal(2).sqrt()) / 2


# --- Moments ------------------------------------------------------------------------------------


def poisson_moments(lam: Fraction) -> tuple[Fraction, Fraction, Fraction]:
    """Mean, variance and fourth central moment of Poisson(λ)."""
    return lam, lam, lam * (1 + 3 * lam)


def gamma_moments(k: Fraction, theta: Fraction) -> tuple[Fraction, Fraction, Fraction]:
    """Mean, variance and fourth central moment of Gamma(k, θ): 3kθ⁴(k + 2)."""
    return k * theta, k * theta**2, 3 * k * (k + 2) * theta**4


def beta_moments(a: Fraction, b: Fraction) -> tuple[Fraction, Fraction, Fraction]:
    """Mean, variance and fourth central moment of Beta(a, b), from the excess kurtosis."""
    s = a + b
    var = a * b / (s**2 * (s + 1))
    excess = 6 * ((a - b) ** 2 * (s + 1) - a * b * (s + 2)) / (a * b * (s + 2) * (s + 3))
    return a / s, var, var**2 * (3 + excess)


def normal_moments(mu: Fraction, sd: Fraction) -> tuple[Fraction, Fraction, Fraction]:
    """Mean, variance and fourth central moment of N(μ, σ²)."""
    return mu, sd**2, 3 * sd**4


# --- The ziggurat tables ------------------------------------------------------------------------


def mix(z: int) -> int:
    """The SplitMix64 output function (rh-ids rng::mix)."""
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK64
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK64
    return z ^ (z >> 31)


def h64_words(domain: int, words: Sequence[int]) -> int:
    """rh-ids rng::h64_words: h = mix(domain ^ n), then h = mix((h + GAMMA) ^ w) per word."""
    h = mix(domain ^ len(words))
    for w in words:
        h = mix(((h + 0x9E3779B97F4A7C15) & MASK64) ^ w)
    return h


def table_hash() -> int:
    """Hash the bits of ZIG_NORM_R, ZIG_NORM_X and ZIG_NORM_F, in that order."""
    text = TABLES.read_text(encoding="utf-8")
    code = "\n".join(line for line in text.splitlines() if not line.lstrip().startswith("//"))
    values: list[str] = []
    for name in ("ZIG_NORM_R", "ZIG_NORM_X", "ZIG_NORM_F"):
        match = re.search(rf"{name}\s*:[^=]*=\s*(\[[^\]]*\]|[^;]*);", code)
        if match is None:
            raise SystemExit(f"gen_sampler_reference: {name} not found in {TABLES}")
        values += re.findall(r"\d+\.\d+", match.group(1))
    if len(values) != 1 + 2 * 257:
        raise SystemExit(f"gen_sampler_reference: expected 515 table entries, found {len(values)}")
    return h64_words(0, [struct.unpack("<Q", struct.pack("<d", float(v)))[0] for v in values])


# --- Rendering ----------------------------------------------------------------------------------


def chi2_block(
    tier: str, dist: str, params: str, pmf: list[Decimal], rest: Decimal
) -> Iterator[str]:
    """One chi-square case: its header line, then its bins."""
    samples = TIERS[tier]
    merged = bins(pmf, rest, samples)
    df = len(merged) - 1
    if df < MIN_DF:
        raise SystemExit(f"gen_sampler_reference: {dist} {params} has {df} degrees of freedom")
    yield f"chi2 {tier} {dist} {params} {samples} {len(merged)} {df} {fmt(chi2_critical(df))}"
    for b in merged:
        yield f"bin {b.lo} {b.hi} {fmt(b.prob)}"


def moments_line(tier: str, dist: str, params: str, m: tuple[Fraction, Fraction, Fraction]) -> str:
    """One moments case."""
    return f"moments {tier} {dist} {params} {TIERS[tier]} {fmt(m[0])} {fmt(m[1])} {fmt(m[2])}"


def render() -> str:
    """Return the whole reference file."""
    lines = [
        "# rh-sample exact reference distributions (SPR-003).",
        "# Generated by bin/gen_sampler_reference.py; never edit by hand.",
        "# Parameters are Q16_16 raw bits (binomial n is a count).",
        "# chi2 <tier> <dist> <params> <samples> <bins> <df> <critical 99.9%>, then <bins> lines:",
        "#   bin <lo> <hi> <probability>   (the last bin's hi is 4294967295: open above)",
        "# moments <tier> <dist> <params> <samples> <mean> <variance> <fourth central moment>",
        "# tail <tier> <t bits> <samples> <P(Z_q > t)>   (standard normal rounded to Q16_16)",
        "# table <hash>   (h64_words(0, bits of ZIG_NORM_R, ZIG_NORM_X, ZIG_NORM_F))",
        f"table {table_hash()}",
    ]
    binomial = {(n, p): binomial_pmf(n, p) for n, p in BINOMIAL}
    poisson = {lam: poisson_pmf(lam) for lam in POISSON}
    for tier in TIERS:
        for n, p in BINOMIAL:
            lines += chi2_block(tier, "binomial", f"{n} {p}", binomial[n, p], Decimal(0))
        for lam in POISSON:
            pmf, rest = poisson[lam]
            lines += chi2_block(tier, "poisson", f"{lam}", pmf, rest)
        for lam in POISSON:
            lines.append(moments_line(tier, "poisson", f"{lam}", poisson_moments(q(lam))))
        for k, theta in GAMMA:
            lines.append(moments_line(tier, "gamma", f"{k} {theta}", gamma_moments(q(k), q(theta))))
        for a, b in BETA:
            lines.append(moments_line(tier, "beta", f"{a} {b}", beta_moments(q(a), q(b))))
        for mu, sd in NORMAL:
            lines.append(moments_line(tier, "normal", f"{mu} {sd}", normal_moments(q(mu), q(sd))))
        for t in TAIL:
            lines.append(f"tail {tier} {t} {TAIL_TIERS[tier]} {fmt(normal_upper(t))}")
    return "\n".join(lines) + "\n"


def main(argv: Sequence[str] | None = None) -> int:
    """Write the reference file, or with ``--check`` verify that it is current."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    parser.add_argument("--output", type=Path, default=OUTPUT, help="the file to write or check")
    args = parser.parse_args(argv)
    text = render()
    cases = sum(line.split(" ", 1)[0] in ("chi2", "moments", "tail") for line in text.splitlines())
    if args.check:
        current = args.output.read_text(encoding="utf-8") if args.output.exists() else ""
        if current != text:
            print(f"gen_sampler_reference: STALE: {args.output}; run python -I {sys.argv[0]}")
            return 1
        print(f"gen_sampler_reference: current ({cases} cases)")
        return 0
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(text, encoding="utf-8")
    print(f"gen_sampler_reference: wrote {args.output} ({cases} cases)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
