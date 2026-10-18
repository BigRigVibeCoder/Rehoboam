"""Exact reference and self-test of population synthesis v0 and the dynastic back-fill (SPR-019).

The algorithm is stated in ``specs/worldgen/population_synthesis.toml``:
[algorithm.population_synthesis] steps P0 to P12 (the anonymous rest of a people's ``population``:
a stationary age-sex marginal, couples by iterative proportional fitting, dependent children,
households, residence) and [algorithm.dynastic_backfill] steps B0 to B11 (each manifest dynasty's
pre-epoch genealogy, marriages, deaths and tenure log back to its ``generations``). This file
implements those steps with Python integers only (no float anywhere; ``fractions`` appears in one
check against the births reference's exact chain), on the keyed RNG of ``bin/gen_rng_golden.py``
(the independent rh-ids reference), the F1 mortality schedule of
``specs/demography/selftest_m2_f1_mortality.py`` (read as knots), the births constants of
``selftest_m2_f1.py`` and the worldgen-lite reference beside it, so it reproduces the engine's
integers bit for bit. It checks:

  1. the committed golden outputs (``specs/worldgen/golden/population_synthesis_v0.txt``) equal
     this reference's, for two manifests (hamlet, manor) at two seeds each, plus the manor's tenure
     logs under each of the seven succession laws; ``--write`` regenerates the file, never by hand;
  2. the invariants of the spec on every case (INV-PS-01..07, INV-BF-01..06): the allocated tables
     meet their marginals exactly and each cell is within one of its exact value; every id is
     unique; every parent is inside the fertility window (a back-filled mother at the conception
     pulse, a synthesised one at the birth) and alive; one mother's conceptions are more than the
     closed period apart; every back-filled birth and every closed row's death precede the epoch;
     each title's tenure rows are contiguous, held by living house members, and end at genesis
     with the title's holder; the living count at tick 0 equals the population; every living
     person has a household with a living head and a settlement;
  3. the law sweep: the manor's back-fill under each of the seven laws over twenty seeds keeps the
     back-fill invariants; the successions and fallbacks are counted and pinned;
  4. the spec file itself: its ``constants`` equal this reference's and its ``check_values`` hold
     (a stale value fails: ``--spec PATH`` points the check at another copy); the widths the spec
     states (every allocation product fits u128, every tick fits i64); E(a) in Q0.32 equals the
     births reference's exact chain rounded once.

Run: python -I specs/worldgen/selftest_population_synthesis.py           (check; exit 1 on failure)
     python -I specs/worldgen/selftest_population_synthesis.py --write   (regenerate the golden)
     python -I specs/worldgen/selftest_population_synthesis.py --golden PATH  (check another file)
     python -I specs/worldgen/selftest_population_synthesis.py --spec PATH   (another spec copy)
     python -I specs/worldgen/selftest_population_synthesis.py --altered-golden PATH
         --stale-spec PATH (the two controls that need no second build run on the same reference
         run; both must be reported as required, else the run FAILS: REV-SPR-030 R10)
     python -I specs/worldgen/selftest_population_synthesis.py --control NAME (a planted violation;
         NAME is ``spacing`` (B6's closed period skipped), ``rounding-cells`` (P3's age-sex
         allocation replaced by floors) or ``rounding-table`` (P7's rounding replaced by floors):
         each must FAIL the invariant that guards it)
     python -I specs/worldgen/selftest_population_synthesis.py --effect-ref DIR (the differential
         gate of REV-SPR-019 R3: DIR holds effect_ref.py and effect_language_golden_v0.toml; every
         pre-epoch succession of the genealogical laws in the sweep is re-evaluated by the effect
         language's programs and must give the same line)
Exit: 0 pass, 1 fail, 2 the reference failed its own checks. Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import functools
import hashlib
import importlib.util
import itertools
import sys
import tomllib
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from pathlib import Path
from types import ModuleType
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GOLDEN = HERE / "golden" / "population_synthesis_v0.txt"
SPEC = HERE / "population_synthesis.toml"
EXIT_OK, EXIT_FAIL, EXIT_REFERENCE = 0, 1, 2


def _load(name: str, path: Path) -> ModuleType:
    """Load a sibling reference by path (python -I adds no script directory to sys.path)."""
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SystemExit(f"{path} not found")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module  # dataclasses resolve annotations through sys.modules
    spec.loader.exec_module(module)
    return module


wl = _load("selftest_worldgen_lite", HERE / "selftest_worldgen_lite.py")
m2f1 = _load("selftest_m2_f1", ROOT / "specs" / "demography" / "selftest_m2_f1.py")
m2mort = _load(
    "selftest_m2_f1_mortality", ROOT / "specs" / "demography" / "selftest_m2_f1_mortality.py"
)
rng = wl.rng  # the rh-ids reference (bin/gen_rng_golden.py), through the worldgen reference

# ----------------------------------------------------------------------------- constants
ONE = 1 << 32  # Prob32's 1; l(x) and E(a) carry 32 fraction bits
Q64_ONE = 1 << 64  # the unmarried chain's 1 (Q0.64)
MASK64 = (1 << 64) - 1
I64_MAX = (1 << 63) - 1
I64_MIN = -(1 << 63)
DAY = 1440
MONTHS = 12
DAYS_360 = 360
MONTHS_365 = (31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)  # rh-core time.rs
SIBLING_GAP_DAYS = 609  # P10: at least M2's least birth-to-birth gap on either calendar (R10)
Q16 = 1 << 16
MAX_POPULATION = 10_000_000  # the manifest's bound on a people's population (< 2^24)
# rh-ids layouts (person/founder/v1 exists; household/v1 is SPR-015's H1; house and title are new).
TAG_FOUNDER = rng.TAG_FOUNDER
TAG_HOUSEHOLD = rng.tag("rehoboam/rh-ids/household/v1")
TAG_HOUSE = rng.tag("rehoboam/rh-ids/house/v1")
TAG_TITLE = rng.tag("rehoboam/rh-ids/title/v1")
# rh-worldgen purposes (P0, B0): the two of SPR-011/SPR-018 that are reused, and the new ones.
PURPOSE_BIRTHDAY = "founder_birthday"
PURPOSE_RESIDENCE = "founder_residence"
PURPOSE_OFFSET = "synth_offset"
PURPOSE_SPINE = "backfill_spine"
PURPOSE_MARRIAGE = "backfill_marriage"
PURPOSE_CONCEPTION = "backfill_conception"
PURPOSE_SEX = "backfill_sex"
PURPOSE_DEATH = "backfill_death"
NEW_PURPOSES = (
    PURPOSE_OFFSET,
    PURPOSE_SPINE,
    PURPOSE_MARRIAGE,
    PURPOSE_CONCEPTION,
    PURPOSE_SEX,
    PURPOSE_DEATH,
)
# M2 F1 constants: read from specs/systems/next/M2.demography.odd.toml ([algorithm.fertility_f1] and
# [algorithm.partnering_f1] `constants`) through the births reference: a changed constant moves the
# goldens here. The schedule is [algorithm.mortality_f1]'s, through the mortality reference (knots).
M2_SPEC = m2f1.load_spec(m2f1.NEXT_SPEC)
STANDARDS = m2f1.Standards(M2_SPEC)
NATURAL = tuple(int(n) for n in STANDARDS.natural)  # Coale-Trussell n(a), thousandths
GESTATION = int(STANDARDS.gestation)
POSTPARTUM = int(STANDARDS.postpartum)
CLOSED = GESTATION + POSTPARTUM  # pulses after a conception during which a woman is not susceptible
SEX_RATIO = (int(STANDARDS.sex_ratio[0]), int(STANDARDS.sex_ratio[1]))  # male births per 205
NUPTIALITY = tuple(STANDARDS.nuptiality)  # monthly first-marriage probability, thousandths, by band
_PARTNERING = M2_SPEC["algorithm"]["partnering_f1"]["constants"]
MIN_AGE_F = int(_PARTNERING["min_age_f_years"])
MIN_AGE_M = int(_PARTNERING["min_age_m_years"])
BANDS = int(m2f1.BANDS)
BAND_YEARS = int(m2f1.BAND_YEARS)
BAND_MONTHS = int(m2f1.BAND_MONTHS)
FERTILE_END = MIN_AGE_F + BANDS * BAND_YEARS  # 50
CHAIN_MONTHS = BANDS * BAND_MONTHS  # 420: the F3 chain's months from exact age 15
TFR_DEFAULT = "4.9"
MULT_DEFAULT = "1.0"
E0_DEFAULT = "30"
BETA_DEFAULT = "1"
# [algorithm.population_synthesis] constants (design values; the companion says why).
CHILD_MAX = 15  # a dependant: aged 0..14 at the epoch; everyone older heads a household or marries
WIDE_BANDS = 14  # five-year bands 15-19 .. 80+ (the last is open), for the couple table
HUSBAND_SHIFT = 3  # a man's marriage pattern is a woman's three years younger (the F1 chain's gap)
MARRIAGE_REF_AGE = 19  # the widowhood factor starts at the women's mean age at first marriage
GAP_SEED = {0: 8, 1: 8, 2: 8, -1: 4, 3: 4}  # IPF seed by husband band minus wife band; else 1
GAP_SEED_OTHER = 1
IPF_SWEEPS = 32
IPF_SCALE = 1 << 32  # cells carry 32 fraction bits
CHILD_BANDS = 3  # 0-4, 5-9, 10-14
SIBLING_GAP = SIBLING_GAP_DAYS * DAY  # P10: two genesis siblings' births, in minutes (876,960)
BAND_REPRESENTATIVE = 2  # P9: a band's representative age is its lower bound plus two years
OFFSET_BITS = 32  # P3: the offset phi is the draw's high 32 bits over 2^32
WEIGHT_PRODUCT_BITS = 96  # P3: an allocation needs sum(weights) x total < 2^96 (then u128 suffices)
PEOPLE_INDEX = 0  # the first human people (P1); a later people would be 1, 2, ...
SLOT_STRIDE = 16  # P3: synth_offset's entity = 16 x people index + slot
# [algorithm.dynastic_backfill] constants (design values).
YOUNGEST_YEARS = 20  # generation G of a dynasty is born in the 20 years before the epoch
FATHER_AGE = (19, 45)  # a spine father's age in whole years at the heir's birth
MOTHER_AGE = (16, 35)  # a spine mother's age at the heir's birth
GROOM_AGE = (18, 30)  # a younger son's age at his marriage
BRIDE_AGE = (15, 25)  # a bride's age in whole years at her marriage (spine wives and brides alike)
MAX_ROWS = 4096  # life-course rows per dynasty; the lineage relation's bound (effect language §5.2)
LINEAGE_HEIGHT = 3  # lineage(holder, 3): the effect language's laws (§8)
DESCENT_MAX = 6  # descends and descendants reach six generations (§5.3)
KIN_MAX = 6  # up(x, p) is at most six, else 7 with no common ancestor (§5.3)
HOLDERS_WINDOW = 16  # holders_of(title, 16): the title-blood rule (§8, REV-SPR-017 N5)
LAWS = (  # the Registry's succession_laws enum, in code order
    "primogeniture",
    "ultimogeniture",
    "seniority",
    "partition",
    "elective",
    "tanistry",
    "sortition",
)
COURT_LAWS = ("elective", "sortition")  # candidates from court(title): the genesis row only (B9)
GENEALOGICAL_LAWS = tuple(law for law in LAWS if law not in COURT_LAWS)
SWEEP_SEEDS = 20
SEX_MALE, SEX_FEMALE = 0, 1
ORIGIN_FOUNDER = 1
CAUSE_NATURAL = 1
EXIT_DEATH = 1
SOURCE_GENESIS = 0
EXIT_NONE, EXIT_NATURAL = 0, 1
TIER_SOVEREIGN, TIER_MAJOR = 0, 1
TIER_NAMES = ("sovereign", "major")
# Offset slots of the allocations of P3, P5 and P9 (purpose synth_offset, entity = 16 i + slot).
SLOT_AGESEX = 0
SLOT_WOMEN = 1
SLOT_MEN = 2
SLOT_CHILDREN = 3  # + the child band
# Draw indices of backfill_marriage (B8), entity = the groom.
DRAW_GROOM_AGE, DRAW_GROOM_DAY, DRAW_BRIDE_AGE, DRAW_BRIDE_DAY = 0, 1, 2, 3

Cells = dict[tuple[int, int], list["Person"]]


def rhe(n: int, d: int) -> int:
    """Return n / d rounded to the nearest integer, ties to even (S0); d > 0."""
    return int(m2mort.rhe(n, d))


def pick(u: int, n: int) -> int:
    """Return floor(u n / 2^64) for a u64 draw: an index in [0, n), n >= 1 (W0)."""
    return int(wl.pick(u, n))


def hits(draw: int, p: int) -> bool:
    """Return the Prob32 mapping: hits iff p = 2^32 or draw < p 2^32."""
    return p == ONE or draw < p << 32


def ceil_day(t: int) -> int:
    """Return the day start at or after t."""
    return -(-t // DAY) * DAY


def ceil_div(n: int, d: int) -> int:
    """Return the least integer at or above n / d; d > 0, n any sign."""
    return -(-n // d)


def q16(literal: str) -> int:
    """Return the Q16_16 bits of a decimal literal, rounded half to even (SPR-011 ruling 9)."""
    return int(m2mort.q16(literal))


def spec_constants() -> tuple[dict[str, Any], dict[str, Any]]:
    """Return this reference's constants in the form of the spec's two ``constants`` tables."""
    synthesis = {
        "child_max": CHILD_MAX,
        "wide_bands": WIDE_BANDS,
        "husband_shift_years": HUSBAND_SHIFT,
        "marriage_ref_age": MARRIAGE_REF_AGE,
        "gap_seed": [GAP_SEED[d] for d in (0, 1, 2, -1, 3)] + [GAP_SEED_OTHER],
        "ipf_sweeps": IPF_SWEEPS,
        "ipf_fraction_bits": IPF_SCALE.bit_length() - 1,
        "child_bands": [BAND_YEARS * (c + 1) for c in range(CHILD_BANDS)],
        "sibling_gap_minutes": SIBLING_GAP,
        "band_representative_years": BAND_REPRESENTATIVE,
        "offset_bits": OFFSET_BITS,
        "weight_product_bits": WEIGHT_PRODUCT_BITS,
        "slot_stride": SLOT_STRIDE,
        "max_population": MAX_POPULATION,
    }
    backfill = {
        "youngest_years": YOUNGEST_YEARS,
        "father_age_years": list(FATHER_AGE),
        "mother_age_years": list(MOTHER_AGE),
        "groom_age_years": list(GROOM_AGE),
        "bride_age_years": list(BRIDE_AGE),
        "max_rows": MAX_ROWS,
        "lineage_height": LINEAGE_HEIGHT,
        "descent_max": DESCENT_MAX,
        "kin_max": KIN_MAX,
        "holders_window": HOLDERS_WINDOW,
    }
    return synthesis, backfill


# ----------------------------------------------------------------------------- calendar (B2)
@dataclass(frozen=True)
class Calendar:
    """A 360- or 365-day year of twelve months; month m may be negative (pre-epoch)."""

    days_per_year: int

    @property
    def year(self) -> int:
        """Minutes per year, Y."""
        return self.days_per_year * DAY

    def month_lengths(self) -> tuple[int, ...]:
        """The twelve month lengths in days."""
        return (30,) * MONTHS if self.days_per_year == DAYS_360 else MONTHS_365

    def month_start(self, m: int) -> int:
        """Return t_m, the first minute of month m, for any integer m (floor division)."""
        y, j = divmod(m, MONTHS)
        return y * self.year + sum(self.month_lengths()[:j]) * DAY

    def month_days(self, m: int) -> int:
        """Return n_m, the length of month m in days."""
        return self.month_lengths()[m % MONTHS]

    def month_of(self, t: int) -> int:
        """Return the month index m with t_m <= t < t_(m+1)."""
        y, rem = divmod(t, self.year)
        offs = 0
        for j, days in enumerate(self.month_lengths()):
            if rem < offs + days * DAY:
                return y * MONTHS + j
            offs += days * DAY
        raise AssertionError("unreachable: rem < year")


# ----------------------------------------------------------------------------- schedules (P2, B3)
@functools.cache
def keyed_draw(seed: int, purpose: str, entity: int, k: int) -> int:
    """Return the rh-ids reference's keyed draw, once per key (the sweep repeats genealogies)."""
    return int(wl.draw(seed, purpose, entity, k))


@functools.cache
def compile_fertility(tfr_bits: int, mult_bits: int) -> Fertility:
    """Return the compiled fertility constants, once per (tfr, elite_fertility_mult)."""
    return Fertility(tfr_bits, mult_bits)


@functools.cache
def compile_schedule(e0: str, beta: str) -> Any:  # noqa: ANN401
    """Return the mortality reference's compiled F1 schedule, once per (e0, beta)."""
    return m2mort.compile_schedule(e0, beta)


class Schedule:
    """The compiled F1 mortality schedule (MR2-MR9) as knots, and the inverse death-age draw."""

    def __init__(self, e0: str, beta: str, cal: Calendar) -> None:
        """Compile K and the knots for the manifest literals ``e0`` and ``beta`` on ``cal``."""
        self.cal = cal
        self.compiled = compile_schedule(e0, beta)
        self.k = int(self.compiled.k)
        self.knots: list[int] = [int(v) for v in self.compiled.knot_l]
        self.ages: list[int] = [int(a) for a in self.compiled.ages(cal.year)]
        self.end_years = ceil_div(int(self.compiled.zero_twelfths), MONTHS)  # A: the schedule's end

    def survivors(self, x: int) -> int:
        """Return MR9's l(x) for an age of x minutes (32 fraction bits)."""
        return int(m2mort.ell(self.compiled, x, self.cal.year))

    def death_age(self, u: int, y: int) -> int:
        """Return B3's death age (minutes) of a person alive at age y, from the draw u.

        T = floor(u l(y) / 2^64) is the survivorship level at death, in [0, l(y)); the death age is
        the least whole minute x at which the linear interpolation of the knots falls to T or
        below: x = A_(i-1) + ceil((L_(i-1) - T)(A_i - A_(i-1)) / (L_(i-1) - L_i)) for the first
        knot index i with L_i <= T. P(x >= a) = l(a) / l(y) up to the interpolation; x > y always.
        """
        ly = self.survivors(y)
        assert ly > 0, "a death cannot be drawn for an age the schedule has already ended"
        target = (u * ly) >> 64
        i = next(i for i in range(1, len(self.knots)) if self.knots[i] <= target)
        a0, a1 = self.ages[i - 1], self.ages[i]
        l0, l1 = self.knots[i - 1], self.knots[i]
        x = a0 + ceil_div((l0 - target) * (a1 - a0), l0 - l1)
        assert x > y
        return x


class Fertility:
    """F1's compiled fertility and nuptiality constants (F2-F4, N1), from the births reference."""

    def __init__(self, tfr_bits: int, mult_bits: int) -> None:
        """Compile the level M, the elite level, the conception table and the unmarried chain."""
        st = STANDARDS
        self.nuptiality_bits = [int(b) for b in m2f1.nuptiality_bits(st)]
        self.tfr1_bits = int(m2f1.tfr1_bits(st))
        self.level_bits = int(m2f1.level_bits(st, tfr_bits))
        self.elite_bits = int(m2f1.elite_level_bits(self.level_bits, mult_bits))
        self.conception = [int(m2f1.conception_bits(st, self.level_bits, i)) for i in range(BANDS)]
        self.conception_elite = [
            int(m2f1.conception_bits(st, self.elite_bits, i)) for i in range(BANDS)
        ]
        self.sex_male_bits = int(m2f1.sex_male_bits(st))
        # P5: the F3 chain's unmarried share at the start of each month from exact age 15, Q0.64.
        self.unmarried = [Q64_ONE]
        for a in range(CHAIN_MONTHS - 1):
            nu = self.nuptiality_bits[a // BAND_MONTHS]
            self.unmarried.append(rhe(self.unmarried[-1] * (ONE - nu), ONE))

    def ever_married(self, age_years: int) -> int:
        """Return P5's E(a) in Q0.32: the share ever married at the middle month of age a."""
        if age_years < MIN_AGE_F:
            return 0
        month = min((age_years - MIN_AGE_F) * MONTHS + MONTHS // 2, CHAIN_MONTHS - 1)
        return ONE - rhe(self.unmarried[month], ONE)


def band_of(age_minutes: int, year: int) -> int | None:
    """Return F0's band i for an age in minutes: 15 Y <= x < 50 Y, else None."""
    if age_minutes < MIN_AGE_F * year or age_minutes >= FERTILE_END * year:
        return None
    return (age_minutes // year - MIN_AGE_F) // BAND_YEARS


def wide_band(age_years: int) -> int:
    """Return P5's band of a whole-year age 15 and over: 0 = 15-19, ..., 13 = 80 and over."""
    assert age_years >= MIN_AGE_F
    return min((age_years - MIN_AGE_F) // BAND_YEARS, WIDE_BANDS - 1)


def natural_at(age_years: int) -> int:
    """Return n(a) in thousandths at a whole-year age, 0 outside 15-49 (F1)."""
    if MIN_AGE_F <= age_years < FERTILE_END:
        return NATURAL[(age_years - MIN_AGE_F) // BAND_YEARS]
    return 0


# ----------------------------------------------------------------------------- the world built
@dataclass
class Person:
    """One person of genesis: a ledger row (if alive at tick 0) and a life-course row."""

    id: int
    k: int
    sex: int
    birth: int
    death: int | None = None  # the death tick when drawn (B3); None for a synthesised commoner
    mother: int = 0
    father: int = 0
    spouse: int = 0
    household: int = 0
    house: int = 0
    generation: int = -1
    spine: bool = False
    conception: int | None = None  # the conception pulse index of a back-filled child (checks)
    parity: int = 0
    last_birth: int = I64_MIN
    children: list[int] = field(default_factory=list)  # as a mother, in creation order

    @property
    def alive(self) -> bool:
        """Alive at tick 0: no drawn death, or one at or after the epoch."""
        return self.death is None or self.death >= 0

    def alive_at(self, t: int) -> bool:
        """Born and not yet dead at tick t."""
        return self.birth <= t and (self.death is None or t < self.death)


@dataclass
class Marriage:
    """A marriage row (M2 marriage.*)."""

    husband: int
    wife: int
    start: int
    end: int
    source: int = SOURCE_GENESIS


@dataclass
class Household:
    """A household row (M2 household.*) with its members for the bookkeeping of H4 at genesis."""

    id: int
    head: int
    formed: int
    dissolved: int = I64_MAX
    members: list[int] = field(default_factory=list)
    residence: int = 0


@dataclass
class Tenure:
    """A tenure row (M7 tenure.*)."""

    title: int
    holder: int
    start: int
    end: int


@dataclass
class Succession:
    """One pre-epoch succession of B9: what the law selected (the golden, the differential)."""

    tick: int
    holder: int
    rows: int  # tenure rows written before it (the closed one included)
    line: list[int]  # the law's line; empty when the stand-in rule decided
    chosen: int
    adult_branch: bool  # the stand-in's adult branch fired (an empty line, a living adult male)
    minor: bool  # the seated holder is under the majority at the succession


@dataclass
class Title:
    """A title (M7 title.*) at genesis."""

    id: int
    k: int
    tier: int
    law: str
    liege: int
    polity: int
    holder: int = 0
    held_since: int = 0
    predecessor_exit: int = EXIT_NONE
    tenures: list[Tenure] = field(default_factory=list)
    successions: list[Succession] = field(default_factory=list)

    @property
    def fallbacks(self) -> int:
        """Successions the stand-in rule decided because the law's line was empty."""
        return sum(1 for s in self.successions if not s.line)

    @property
    def minor_fallbacks(self) -> int:
        """Stand-in successions that seated a minor (no living adult male in the house)."""
        return sum(1 for s in self.successions if not s.line and s.minor)

    @property
    def adult_fallbacks(self) -> int:
        """Stand-in successions decided by the adult branch."""
        return sum(1 for s in self.successions if not s.line and s.adult_branch)


@dataclass
class House:
    """A house (M7 house.*) with its dynasty's genealogy."""

    id: int
    k: int
    generations: int
    title: int
    members: list[int] = field(default_factory=list)  # house_member rows, in creation order
    root: int = 0
    rows: int = 0  # life-course rows written for the dynasty (members and in-married wives)


@dataclass(frozen=True)
class Dynasty:
    """A manifest dynasty: ``polities[].dynasties[]``."""

    id: str
    generations: int


@dataclass(frozen=True)
class Polity:
    """A manifest polity, as the back-fill reads it."""

    id: str
    succession_law: str
    dynasties: tuple[Dynasty, ...]


@dataclass(frozen=True)
class Manifest:
    """The fields of a manifest the two algorithms read (hardcoded cases mirror the YAML files)."""

    id: str
    seed: int
    days_per_year: int
    template: str
    raster: int
    regions: int
    population: int
    manifest_founders: int  # cast + every manifest household's children and servants
    e0: str
    brass_beta: str
    tfr: str
    elite_fertility_mult: str
    majority_age: str | None  # M7.dynasty's parameter; None when M7 is not in the manifest
    polities: tuple[Polity, ...]


@dataclass
class Allocation:
    """One systematic allocation (P3): its integer weights, total and result, for INV-PS-01..03."""

    weights: list[int]
    total: int
    cells: list[int]

    def within_one(self) -> bool:
        """Every cell is within one of its exact target w_k total / sum(w)."""
        s = sum(self.weights)
        pairs = zip(self.cells, self.weights, strict=True)
        return all(abs(c * s - w * self.total) < s for c, w in pairs)


@dataclass
class Report:
    """What the synthesis computed on the way, for the golden lines and the invariants."""

    synthesised: int = 0
    living_dynasts: int = 0
    agesex: Allocation | None = None
    married: int = 0
    married_women: int = 0
    married_men: int = 0
    women_by_age: Allocation | None = None
    men_by_band: Allocation | None = None
    women_by_band: list[int] = field(default_factory=list)
    ipf: list[list[int]] = field(default_factory=list)
    ipf_fixed: list[list[int]] = field(default_factory=list)
    children_alloc: list[list[int]] = field(default_factory=list)
    leftover: int = 0
    orphans: int = 0


@dataclass
class Spine:
    """B4's anchors of one dynasty: births of S_0..S_G, births of W_0..W_(G-1), marriage ticks."""

    births: list[int]
    wives: list[int]
    marriages: list[int]


# ----------------------------------------------------------------------------- B9: the laws
class Genealogy:
    """The back-filled log as the effect language's prelude reads it at tick t (§5.3).

    A person exists when born at or before t (the runtime has no row for the unborn); ``alive``
    is person.alive at t: born, and not dead at t (the holder, dead at t, is not alive).
    """

    def __init__(self, persons: dict[int, Person], t: int) -> None:
        """Index the children of every parent among the persons born at or before t."""
        self.ps = persons
        self.t = t
        self.kids: dict[int, list[int]] = {}
        for p in persons.values():
            if p.birth <= t:
                for parent in (p.mother, p.father):
                    if parent:
                        self.kids.setdefault(parent, []).append(p.id)
        for v in self.kids.values():
            v.sort()

    def alive(self, x: int) -> bool:
        """Whether x is alive at t: the prelude's person.alive(x)."""
        return self.ps[x].alive_at(self.t)

    def parents(self, x: int, agnatic: bool) -> list[int]:
        """The parents from the life-course row, father first; the father only if agnatic."""
        p = self.ps[x]
        return [q for q in ((p.father,) if agnatic else (p.father, p.mother)) if q]

    def children(self, x: int) -> list[int]:
        """Every existing person with mother or father = x, ascending id (the prelude's)."""
        return self.kids.get(x, [])

    def ancestors_or_self(self, x: int, depth: int, agnatic: bool) -> set[int]:
        """Ancestors-or-self within ``depth`` generations."""
        seen = {x}
        frontier = [x]
        for _ in range(depth):
            frontier = [q for y in frontier for q in self.parents(y, agnatic) if q not in seen]
            seen.update(frontier)
        return seen

    def descendants(self, x: int, depth: int) -> list[int]:
        """The descendants of x within ``depth`` generations: by generation, then ascending id."""
        seen = {x}
        frontier = [x]
        out: list[int] = []
        for _ in range(depth):
            nxt: set[int] = set()
            for y in frontier:
                nxt.update(z for z in self.children(y) if z not in seen)
            seen.update(nxt)
            frontier = sorted(nxt)
            out.extend(frontier)
        return out

    def descends(self, x: int, a: int) -> bool:
        """Whether a is an ancestor-or-self of x within six generations (the prelude's)."""
        return x != 0 and a != 0 and a in self.ancestors_or_self(x, DESCENT_MAX, False)

    def lineage(self, p: int, height: int) -> list[int]:
        """The descendants of p's ancestors-or-self up to ``height``, p excluded (the prelude's).

        A dead member stays only with a living descendant among the members (N2); the set is
        bounded by max_rows (B7), so the relation's bound of 4096 never truncates it.
        """
        members: set[int] = set()
        level = [p]
        for _ in range(height + 1):
            for a in level:
                members.update(self.descendants(a, DESCENT_MAX))
            level = [q for y in level for q in self.parents(y, False)]
        members.discard(p)
        kept = {
            x
            for x in members
            if self.alive(x)
            or any(z in members and self.alive(z) for z in self.descendants(x, DESCENT_MAX))
        }
        assert len(kept) <= MAX_ROWS
        return sorted(kept)

    def up_and_common(self, x: int, p: int, agnatic: bool) -> tuple[int, int]:
        """up(x, p) and common(x, p): the least height of p's ancestor that is x's, father first."""
        x_anc = self.ancestors_or_self(x, DESCENT_MAX, agnatic)
        level = [p]
        for height in range(KIN_MAX + 1):
            for candidate in level:
                if candidate in x_anc:
                    return height, candidate
            nxt: list[int] = []
            for y in level:
                nxt.extend(q for q in self.parents(y, agnatic) if q not in nxt)
            level = nxt
        return KIN_MAX + 1, 0

    def line(self, a: int, x: int, agnatic: bool) -> list[int]:
        """The shortest father-first parent chain from x up to a, listed downward (prelude)."""
        if a in (x, 0) or x == 0:
            return []
        queue: list[list[int]] = [[x]]
        while queue:
            path = queue.pop(0)
            if len(path) > DESCENT_MAX:
                return []
            for parent in self.parents(path[-1], agnatic):
                if parent == a:
                    return list(reversed(path))
                if parent not in path:
                    queue.append([*path, parent])
        return []


def holders_of(rows: list[Tenure], n: int) -> list[int]:
    """The holders of the last n tenure rows, most recent first, each once (the prelude's)."""
    out: list[int] = []
    for r in reversed(rows[-n:]):
        if r.holder and r.holder not in out:
            out.append(r.holder)
    return out


Key = tuple[Any, ...]


class Law:
    """One succession law's program (effect language §8), on a genealogy at the holder's death."""

    def __init__(self, gen: Genealogy, holder: int, rows: list[Tenure], majority: int) -> None:
        """Bind the holder, the tenure log so far and the majority in minutes (tanistry)."""
        self.g = gen
        self.holder = holder
        self.holders = holders_of(rows, HOLDERS_WINDOW)
        self.majority = majority

    def blood(self, x: int) -> bool:
        """The title-blood rule: x descends from the holder or from one of the last 16 holders."""
        return self.g.descends(x, self.holder) or any(self.g.descends(x, h) for h in self.holders)

    def collateral(self, x: int) -> bool:
        """The lineage laws' common clause: alive, not the holder, not an ancestor, of the blood."""
        return (
            self.g.alive(x)
            and x != self.holder
            and not self.g.descends(self.holder, x)
            and self.blood(x)
        )

    def representation(self, x: int, youngest_first: bool) -> Key:
        """Primogeniture's key: (up, [(female, birth) along the line from the common ancestor])."""
        up, common = self.g.up_and_common(x, self.holder, False)
        sign = -1 if youngest_first else 1
        line = self.g.line(common, x, False)
        return (up, [(self.g.ps[y].sex == SEX_FEMALE, sign * self.g.ps[y].birth) for y in line])

    def agnatic(self, x: int) -> tuple[bool, int, int]:
        """The agnatic clauses: male with up_agnatic <= 3, up_agnatic, count(line_agnatic)."""
        up, common = self.g.up_and_common(x, self.holder, True)
        depth = len(self.g.line(common, x, True))
        return self.g.ps[x].sex == SEX_MALE and up <= LINEAGE_HEIGHT, up, depth

    def line_for(self, law: str) -> list[int]:
        """The law's line: the eligible candidates in key order, ties ascending PersonId."""
        keyed: list[tuple[Key, int]] = []
        for x in self.candidates(law):
            key = self.key(law, x)
            if key is not None:
                keyed.append((key, x))
        return [x for _, x in sorted(keyed)]

    def candidates(self, law: str) -> list[int]:
        """children(holder) for partition, lineage(holder, 3) for the other genealogical laws."""
        if law == "partition":
            return self.g.children(self.holder)
        return self.g.lineage(self.holder, LINEAGE_HEIGHT)

    def key(self, law: str, x: int) -> Key | None:
        """The order key of an eligible candidate, or None when x is not eligible."""
        p = self.g.ps[x]
        if law == "partition":
            return (p.sex == SEX_FEMALE, p.birth) if self.g.alive(x) and x != self.holder else None
        if not self.collateral(x):
            return None
        if law in ("primogeniture", "ultimogeniture"):
            return self.representation(x, law == "ultimogeniture")
        male_near, up, depth = self.agnatic(x)
        if not male_near:
            return None
        if law == "seniority":
            return (depth - up, p.birth)
        if law == "tanistry":
            adult = self.g.t - p.birth >= self.majority
            return (p.birth,) if depth <= LINEAGE_HEIGHT and adult else None
        raise AssertionError(f"not a genealogical law: {law}")


class Genesis:
    """The state the two algorithms build for one manifest and seed."""

    def __init__(self, m: Manifest, control: str | None = None) -> None:
        """Compile the schedules; nothing is generated yet."""
        self.m = m
        self.control = control
        self.seed = m.seed
        self.cal = Calendar(m.days_per_year)
        self.schedule = Schedule(m.e0, m.brass_beta, self.cal)
        self.fert = compile_fertility(q16(m.tfr), q16(m.elite_fertility_mult))
        self.persons: dict[int, Person] = {}
        self.order: list[int] = []  # ids in genesis order (k)
        self.next_k = m.manifest_founders
        self.marriages: list[Marriage] = []
        self.households: dict[int, Household] = {}
        self.houses: list[House] = []
        self.titles: list[Title] = []
        self.report = Report()
        self.majority = 0  # M7's majority_age in minutes (B9), when a dynasty needs it
        if any(p.dynasties for p in m.polities):
            if m.majority_age is None:
                raise AssertionError(f"{m.id}: a dynasty needs M7.dynasty's majority_age")
            self.majority = ((q16(m.majority_age) * m.days_per_year) >> 16) * DAY

    # -- keys and ids ------------------------------------------------------------------------
    def draw(self, purpose: str, entity: int, k: int) -> int:
        """Return H(world_seed, canonical salt, rh-worldgen, epoch 0, purpose, entity, t = 0, k)."""
        return keyed_draw(self.seed, purpose, entity, k)

    def new_person(self, sex: int, birth: int, **kw: object) -> Person:
        """Issue the next founder id (ruling 10's k order) and record the person."""
        k = self.next_k
        self.next_k += 1
        pid = rng.h64_words(TAG_FOUNDER, [self.seed, k])
        if pid in self.persons or pid == 0:
            raise AssertionError(f"founder id collision at k = {k}: the rehash would fire")
        p = Person(pid, k, sex, birth, **kw)  # type: ignore[arg-type]
        self.persons[pid] = p
        self.order.append(pid)
        return p

    def new_household(self, head: int, formed: int) -> Household:
        """Return a new household: H1's H64(household/v1, [head, formation tick as u64])."""
        hid = rng.h64_words(TAG_HOUSEHOLD, [head, formed & MASK64])
        if hid in self.households or hid == 0:
            raise AssertionError("household id collision")
        h = Household(hid, head, formed)
        self.households[hid] = h
        return h

    def join(self, pid: int, hid: int) -> None:
        """Put a person in a household, leaving the one it was in."""
        p = self.persons[pid]
        if p.household:
            self.households[p.household].members.remove(pid)
        p.household = hid
        self.households[hid].members.append(pid)

    # -- B: the dynastic back-fill -----------------------------------------------------------
    def backfill(self) -> None:
        """B1 to B10 for every dynasty, in manifest order (B1: houses and titles)."""
        house_k = 0
        title_k = 0
        for p_index, pol in enumerate(self.m.polities, start=1):
            if not pol.dynasties:
                continue
            sovereign = self.new_title(title_k, TIER_SOVEREIGN, pol.succession_law, 0, p_index)
            title_k += 1
            for d_index, dyn in enumerate(pol.dynasties):
                title = sovereign
                if d_index > 0:
                    title = self.new_title(title_k, TIER_MAJOR, pol.succession_law, sovereign.id, 0)
                    title_k += 1
                house_id = rng.h64_words(TAG_HOUSE, [self.seed, house_k])
                house = House(house_id, house_k, dyn.generations, title.id)
                house_k += 1
                self.houses.append(house)
                self.backfill_house(house, title)

    def new_title(self, k: int, tier: int, law: str, liege: int, polity: int) -> Title:
        """Return a new title: TitleId = H64(title/v1, [seed, k])."""
        t = Title(rng.h64_words(TAG_TITLE, [self.seed, k]), k, tier, law, liege, polity)
        self.titles.append(t)
        return t

    def backfill_house(self, house: House, title: Title) -> None:
        """B4 to B10: one dynasty."""
        rows_before = len(self.persons)
        anchors = self.spine_anchors(house)
        spine, wives = self.issue_spine(house, anchors)
        for g in range(house.generations):
            self.marry(spine[g], wives[g], anchors.marriages[g])
            self.bear(spine[g], wives[g], anchors.marriages[g], house, heir=spine[g + 1])
        self.younger_sons(house)
        house.rows = len(self.persons) - rows_before
        if house.rows > MAX_ROWS:
            raise AssertionError(
                f"dynasty {house.k}: {house.rows} rows exceed the bound {MAX_ROWS}"
            )
        self.tenures(house, title)

    def spine_anchors(self, house: House) -> Spine:
        """B4: the spine's births and marriages, youngest first, under backfill_spine (the house).

        Draws, in order: k = 0 the youngest generation's birth day; then for g = G-1 down to 0 four
        draws: the father's age at the heir's birth, the mother's, the wife's age at her marriage,
        the marriage day inside its month.
        """
        g_max = house.generations
        y, days = self.cal.year, self.cal.days_per_year
        counter = itertools.count()

        def sd() -> int:
            return self.draw(PURPOSE_SPINE, house.id, next(counter))

        def years(lo_hi: tuple[int, int]) -> int:
            return lo_hi[0] * days + pick(sd(), (lo_hi[1] - lo_hi[0]) * days)

        births = [0] * (g_max + 1)
        births[g_max] = -DAY * (1 + pick(sd(), YOUNGEST_YEARS * days))
        wives = [0] * g_max
        marriages = [0] * g_max
        for g in range(g_max - 1, -1, -1):
            heir = births[g + 1]
            births[g] = heir - DAY * years(FATHER_AGE)
            wives[g] = heir - DAY * years(MOTHER_AGE)
            c_h = self.cal.month_of(heir) - GESTATION  # the heir's conception pulse
            earliest = self.cal.month_of(births[g] + MIN_AGE_M * y) + 1  # the husband is 18
            wanted = self.cal.month_of(wives[g] + DAY * years(BRIDE_AGE)) + 1
            m_marry = max(min(wanted, c_h - 1), earliest)
            assert m_marry <= c_h - 1, "the age bounds keep the marriage before the conception"
            n = self.cal.month_days(m_marry)
            marriages[g] = self.cal.month_start(m_marry) + DAY * (1 + pick(sd(), n - 1))
        return Spine(births, wives, marriages)

    def issue_spine(self, house: House, a: Spine) -> tuple[list[Person], list[Person]]:
        """B4, B3: issue S_0, W_0, S_1, W_1, ..., S_G; link them; draw their conditional deaths."""
        g_max = house.generations
        spine: list[Person] = []
        wives: list[Person] = []
        for g in range(g_max + 1):
            s = self.new_person(SEX_MALE, a.births[g], generation=g, spine=True, house=house.id)
            spine.append(s)
            if g < g_max:
                wives.append(self.new_person(SEX_FEMALE, a.wives[g], generation=g))
        for g in range(1, g_max + 1):
            spine[g].mother, spine[g].father = wives[g - 1].id, spine[g - 1].id
            spine[g].conception = self.cal.month_of(a.births[g]) - GESTATION
            wives[g - 1].children.append(spine[g].id)
        house.root = spine[0].id
        house.members = [s.id for s in spine]
        for g in range(g_max):
            self.draw_death(spine[g], a.births[g + 1])
            self.draw_death(wives[g], a.births[g + 1])
        self.draw_death(spine[g_max], 0)
        return spine, wives

    def draw_death(self, p: Person, alive_at: int) -> None:
        """B3: the death tick of ``p``, alive at tick ``alive_at``: the day start at or after."""
        x = self.schedule.death_age(self.draw(PURPOSE_DEATH, p.id, 0), alive_at - p.birth)
        p.death = ceil_day(p.birth + x)

    def marry(self, h: Person, w: Person, tick: int) -> None:
        """B5: a marriage row, both spouse columns while both live, and the couple's household."""
        assert h.death is not None and w.death is not None
        assert tick < h.death and tick < w.death and tick < 0
        end = min(h.death, w.death)
        self.marriages.append(Marriage(h.id, w.id, tick, end if end < 0 else I64_MAX))
        if h.alive and w.alive:
            h.spouse, w.spouse = w.id, h.id
        hh = self.new_household(h.id, tick)
        self.join(h.id, hh.id)
        self.join(w.id, hh.id)

    def susceptible(self, m: int, last: int | None, c_h: int | None) -> bool:
        """B6: may the wife conceive at pulse m? (closed periods; the forced heir's window)."""
        if c_h is not None and c_h - CLOSED <= m <= c_h + CLOSED:
            return False
        return self.control == "spacing" or last is None or m > last + CLOSED

    def bear(self, h: Person, w: Person, tick: int, house: House, heir: Person | None) -> None:
        """B6: the couple's children by monthly draws from the first pulse after the marriage.

        A conception at pulse m closes (m, m + 20]; the forced heir's conception pulse c_h forbids
        any other conception in [c_h - 20, c_h + 20]. A pregnancy whose birth would fall at or
        after the epoch, or whose mother dies before the birth, bears no child. The table is F6's,
        the elite read on the couple (D56 (e)): c_i^e when the wife or the husband has a
        house_member row, so every back-filled wife, whose husband is of the house, draws c_i^e.
        """
        assert h.death is not None and w.death is not None
        m0 = self.cal.month_of(tick) + 1
        c_h = heir.conception if heir is not None else None
        if heir is not None and heir.household == 0:
            self.join(heir.id, w.household)
        last: int | None = None
        child_index = len(w.children)
        end = min(0, w.death, h.death, w.birth + FERTILE_END * self.cal.year)
        elite = m2f1.draws_elite(w.house != 0, h.house != 0)  # B6 = F6; rows never change
        table = self.fert.conception_elite if elite else self.fert.conception
        for m in itertools.count(m0):
            t_m = self.cal.month_start(m)
            if t_m >= end:
                return
            if m == c_h:
                last = m  # the forced heir, conceived now (B4); no draw
            band = band_of(t_m - w.birth, self.cal.year)
            if band is None or not self.susceptible(m, last, c_h):
                continue
            if not hits(self.draw(PURPOSE_CONCEPTION, w.id, 2 * (m - m0)), table[band]):
                continue
            last = m
            if self.conceive((h, w), m, 2 * (m - m0) + 1, child_index, house):
                child_index += 1

    def conceive(
        self, couple: tuple[Person, Person], m: int, k: int, child_index: int, house: House
    ) -> bool:
        """B6: the birth of a child conceived at pulse m, nine months on; False if none is born."""
        h, w = couple
        assert w.death is not None
        due = m + GESTATION
        n = self.cal.month_days(due)
        day = 1 + pick(self.draw(PURPOSE_CONCEPTION, w.id, k), n - 1)
        birth = self.cal.month_start(due) + DAY * day
        if birth >= 0 or birth > w.death:
            return False
        male = hits(self.draw(PURPOSE_SEX, w.id, child_index), self.fert.sex_male_bits)
        child = self.new_person(
            SEX_MALE if male else SEX_FEMALE,
            birth,
            mother=w.id,
            father=h.id,
            generation=h.generation + 1,
            house=house.id,
            conception=m,
        )
        w.children.append(child.id)
        house.members.append(child.id)
        self.draw_death(child, birth)
        self.join(child.id, w.household)
        return True

    def younger_sons(self, house: House) -> None:
        """B8: every non-spine son of a generation below G marries and bears, in creation order."""
        ps = self.persons
        queue = [pid for pid in self.order if ps[pid].house == house.id and not ps[pid].spine]
        i = 0
        while i < len(queue):
            p = ps[queue[i]]
            i += 1
            if p.sex != SEX_MALE or p.generation >= house.generations:
                continue
            start = len(self.order)
            self.marry_son(p, house)
            queue += [pid for pid in self.order[start:] if ps[pid].house == house.id]

    def marry_son(self, p: Person, house: House) -> None:
        """B8: a son who lives to his drawn marriage age, before the epoch, marries a new bride."""
        assert p.death is not None
        days = self.cal.days_per_year

        def draw(k: int) -> int:
            return self.draw(PURPOSE_MARRIAGE, p.id, k)

        ag = GROOM_AGE[0] + pick(draw(DRAW_GROOM_AGE), GROOM_AGE[1] - GROOM_AGE[0])
        tick = p.birth + DAY * (ag * days + pick(draw(DRAW_GROOM_DAY), days))
        if tick >= p.death or tick >= 0:
            return
        aw = BRIDE_AGE[0] + pick(draw(DRAW_BRIDE_AGE), BRIDE_AGE[1] - BRIDE_AGE[0])
        w_birth = tick - DAY * (aw * days + pick(draw(DRAW_BRIDE_DAY), days))
        w = self.new_person(SEX_FEMALE, w_birth, generation=p.generation)
        self.draw_death(w, tick)
        self.marry(p, w, tick)
        self.bear(p, w, tick, house, heir=None)

    # -- B9: tenures ---------------------------------------------------------------------------
    def stand_in(self, house: House, t: int) -> tuple[int, bool]:
        """B9's stand-in: the oldest living adult male of the house at t, else its oldest male.

        Returns the person and whether the adult branch fired; the other branch seats a minor.
        """
        living = [
            self.persons[pid]
            for pid in house.members
            if self.persons[pid].sex == SEX_MALE and self.persons[pid].alive_at(t)
        ]
        assert living, "a spine member is always alive at a pre-epoch holder death (B4)"
        adults = [p for p in living if t - p.birth >= self.majority]
        return min(adults or living, key=lambda p: (p.birth, p.id)).id, bool(adults)

    def tenures(self, house: House, title: Title) -> None:
        """B9, B10: the tenure log from the root's majority to genesis, and the title at tick 0."""
        rows: list[Tenure] = []
        if title.law in COURT_LAWS:
            rows.append(Tenure(title.id, self.stand_in(house, 0)[0], 0, I64_MAX))
        else:
            holder = self.persons[house.root]
            heir_birth = self.persons[house.members[1]].birth
            start = min(ceil_day(holder.birth + self.majority), heir_birth)
            while holder.death is not None and holder.death < 0:
                rows.append(Tenure(title.id, holder.id, start, holder.death))
                start = holder.death
                holder = self.persons[self.succeed(house, title, holder.id, start, rows)]
            rows.append(Tenure(title.id, holder.id, start, I64_MAX))
        title.tenures = rows
        title.holder = rows[-1].holder
        title.held_since = rows[-1].start
        title.predecessor_exit = EXIT_NATURAL if len(rows) > 1 else EXIT_NONE

    def succeed(self, house: House, title: Title, holder: int, t: int, rows: list[Tenure]) -> int:
        """B9: the law's line at the holder's death at t; its first, else the stand-in."""
        law = Law(Genealogy(self.persons, t), holder, rows, self.majority)
        line = law.line_for(title.law)
        chosen, adult = (line[0], False) if line else self.stand_in(house, t)
        minor = t - self.persons[chosen].birth < self.majority
        title.successions.append(Succession(t, holder, len(rows), line, chosen, adult, minor))
        return chosen

    # -- P: population synthesis -----------------------------------------------------------
    def synthesise(self) -> None:
        """P1 to P12 for the anonymous rest of the human population."""
        n = self.synthesised_count()
        by_age_sex = self.agesex_persons(n)
        age_of = {p.id: a for (a, _s), ps in by_age_sex.items() for p in ps}
        women, men = self.marital_marginals(by_age_sex)
        couples = self.couples(by_age_sex, women, men)
        self.children(by_age_sex, couples, age_of)
        self.singles()
        self.vitals()
        self.headship()
        self.place()

    def synthesised_count(self) -> int:
        """P1: population less the manifest founders and the living back-filled persons."""
        living = sum(1 for p in self.persons.values() if p.alive)
        n = self.m.population - self.m.manifest_founders - living
        if n < 0:
            raise AssertionError(
                f"{self.m.id}: population {self.m.population} is below the manifest founders "
                f"({self.m.manifest_founders}) plus the living dynasts ({living})"
            )
        self.report.synthesised, self.report.living_dynasts = n, living
        return n

    def person_years(self) -> list[int]:
        """Return P2's L_a = l(a Y) + l((a + 1) Y) for a = 0..A-1 (twice the person-years at a)."""
        y = self.cal.year
        return [
            self.schedule.survivors(a * y) + self.schedule.survivors((a + 1) * y)
            for a in range(self.schedule.end_years)
        ]

    def agesex_persons(self, n: int) -> Cells:
        """P2 to P4: the stationary age-sex marginal, its allocation and the persons by cell."""
        y = self.cal.year
        person_years = self.person_years()
        weights_by_sex = (SEX_RATIO[0], SEX_RATIO[1] - SEX_RATIO[0])
        weights = [
            person_years[a] * weights_by_sex[s]
            for a in range(self.schedule.end_years)
            for s in (SEX_MALE, SEX_FEMALE)
        ]
        alloc = self.allocate(weights, n, SLOT_AGESEX)
        self.report.agesex = alloc
        out: Cells = {}
        for a in range(self.schedule.end_years):
            for s in (SEX_MALE, SEX_FEMALE):
                for _ in range(alloc.cells[2 * a + s]):
                    p = self.new_person(s, 0)
                    p.birth = -(a * y) - pick(self.draw(PURPOSE_BIRTHDAY, p.id, 0), y)
                    out.setdefault((a, s), []).append(p)
        return out

    def married_share(self, age_years: int) -> int:
        """Return P5's C(a) in Q0.32: ever married, times the husband's survival since age 19."""
        e = self.fert.ever_married(age_years)
        if age_years <= MARRIAGE_REF_AGE:
            return e
        y = self.cal.year
        l_ref = self.schedule.survivors((MARRIAGE_REF_AGE + HUSBAND_SHIFT) * y)
        return rhe(e * self.schedule.survivors((age_years + HUSBAND_SHIFT) * y), l_ref)

    def marital_marginals(self, by: Cells) -> tuple[list[int], list[int]]:
        """P5: married women per whole-year age and married men per wide band, equal totals."""
        a_max = self.schedule.end_years
        women_w = [len(by.get((a, SEX_FEMALE), [])) * self.married_share(a) for a in range(a_max)]
        men_w = [0] * WIDE_BANDS
        for a in range(MIN_AGE_M, a_max):
            men_here = len(by.get((a, SEX_MALE), []))
            men_w[wide_band(a)] += men_here * self.married_share(a - HUSBAND_SHIFT)
        r = self.report
        r.married_women, r.married_men = sum(women_w) >> 32, sum(men_w) >> 32
        r.married = min(r.married_women, r.married_men)
        r.women_by_age = self.allocate(women_w, r.married, SLOT_WOMEN)
        r.men_by_band = self.allocate(men_w, r.married, SLOT_MEN)
        rows = [0] * WIDE_BANDS
        for a in range(MIN_AGE_F, a_max):
            rows[wide_band(a)] += r.women_by_age.cells[a]
        r.women_by_band = rows
        return r.women_by_age.cells, r.men_by_band.cells

    def couples(self, by: Cells, women: list[int], men: list[int]) -> list[tuple[int, int]]:
        """P6 to P8: the couple table by IPF and its rounding; pairing; marriages and households."""
        rows = self.report.women_by_band
        seed = [
            [GAP_SEED.get(j - i, GAP_SEED_OTHER) for j in range(WIDE_BANDS)]
            for i in range(WIDE_BANDS)
        ]
        fitted = self.ipf(seed, rows, men)
        table = self.round_table(fitted, rows, men)
        self.report.ipf, self.report.ipf_fixed = table, fitted
        married_women: list[list[int]] = [[] for _ in range(WIDE_BANDS)]
        for a in range(MIN_AGE_F, self.schedule.end_years):
            ids = sorted(p.id for p in by.get((a, SEX_FEMALE), []))
            married_women[wide_band(a)] += ids[: women[a]]
        married_men: list[list[int]] = [[] for _ in range(WIDE_BANDS)]
        for a in range(MIN_AGE_M, self.schedule.end_years):
            married_men[wide_band(a)] += [p.id for p in by.get((a, SEX_MALE), [])]
        for j in range(WIDE_BANDS):
            married_men[j] = sorted(married_men[j])[: men[j]]
            married_women[j].sort()
        wi, mj = [0] * WIDE_BANDS, [0] * WIDE_BANDS
        couples: list[tuple[int, int]] = []
        for i in range(WIDE_BANDS):
            for j in range(WIDE_BANDS):
                for _ in range(table[i][j]):
                    couples.append((married_men[j][mj[j]], married_women[i][wi[i]]))
                    wi[i] += 1
                    mj[j] += 1
        for h, w in couples:
            self.persons[h].spouse, self.persons[w].spouse = w, h
            self.marriages.append(Marriage(h, w, 0, I64_MAX))
            hh = self.new_household(h, 0)
            self.join(h, hh.id)
            self.join(w, hh.id)
        return couples

    def child_seed(self) -> list[list[int]]:
        """Return P9's seed B_(i,c): children of age band c a band-i mother bore and still has.

        For each year of age d in band c, n(A_i - d) at the band's representative age A_i (its
        lower bound plus two years), times L_d, the child's person-years at age d.
        """
        person_years = self.person_years()
        return [
            [
                sum(
                    natural_at(MIN_AGE_F + BAND_YEARS * i + BAND_REPRESENTATIVE - d)
                    * person_years[d]
                    for d in range(c * BAND_YEARS, (c + 1) * BAND_YEARS)
                )
                for c in range(CHILD_BANDS)
            ]
            for i in range(WIDE_BANDS)
        ]

    def children(self, by: Cells, couples: list[tuple[int, int]], age_of: dict[int, int]) -> None:
        """P9, P10: dependants allocated over mother bands, then assigned to mothers."""
        by_band: list[list[int]] = [[] for _ in range(CHILD_BANDS)]
        for a in range(CHILD_MAX):
            for s in (SEX_MALE, SEX_FEMALE):
                by_band[a // BAND_YEARS] += sorted(p.id for p in by.get((a, s), []))
        mothers = [
            sorted(w for _h, w in couples if wide_band(age_of[w]) == i) for i in range(WIDE_BANDS)
        ]
        seed = self.child_seed()
        allocation: list[list[int]] = [[0] * WIDE_BANDS for _ in range(CHILD_BANDS)]
        leftover: list[int] = []
        for c in range(CHILD_BANDS):
            weights = [len(mothers[i]) * seed[i][c] for i in range(WIDE_BANDS)]
            if sum(weights) == 0 or not by_band[c]:
                leftover += by_band[c]
                continue
            allocation[c] = self.allocate(weights, len(by_band[c]), SLOT_CHILDREN + c).cells
        self.report.children_alloc = allocation
        assign = self.assigner()
        for c in range(CHILD_BANDS):
            kids = iter(sorted(by_band[c], key=lambda pid: (-age_of[pid], pid)))
            for i in range(WIDE_BANDS):
                for _ in range(allocation[c][i]):
                    child = next(kids)
                    if not assign(mothers[i], child):
                        leftover.append(child)
        all_mothers = sorted(w for _h, w in couples)
        orphans = sum(
            0 if assign(all_mothers, child) else 1
            for child in sorted(leftover, key=lambda pid: (-age_of[pid], pid))
        )
        self.report.leftover, self.report.orphans = len(leftover), orphans

    def eligible(self, mother: int, child: int) -> bool:
        """P10: the mother in the fertility window at the birth, the father 18, siblings apart."""
        mo, ch = self.persons[mother], self.persons[child]
        fa = self.persons[mo.spouse]
        y = self.cal.year
        if not MIN_AGE_F * y <= ch.birth - mo.birth < FERTILE_END * y:
            return False
        if ch.birth - fa.birth < MIN_AGE_M * y:
            return False
        return all(abs(self.persons[c].birth - ch.birth) >= SIBLING_GAP for c in mo.children)

    def assigner(self) -> Callable[[list[int], int], bool]:
        """Return P10's assignment: the eligible mother with the fewest children (ties: id)."""
        count: dict[int, int] = {}

        def assign(mothers: list[int], child: int) -> bool:
            options = [w for w in mothers if self.eligible(w, child)]
            if not options:
                return False
            mother = min(options, key=lambda w: (count.get(w, 0), w))
            mo, ch = self.persons[mother], self.persons[child]
            ch.mother, ch.father = mo.id, mo.spouse
            mo.children.append(ch.id)
            count[mother] = count.get(mother, 0) + 1
            self.join(ch.id, mo.household)
            return True

        return assign

    def singles(self) -> None:
        """P11: everyone living and still without a household heads a one-person household (H2)."""
        for pid in self.order:
            p = self.persons[pid]
            if p.alive and p.household == 0:
                self.join(pid, self.new_household(pid, 0).id)

    def marriage_partner(self, pid: int) -> int:
        """Return the spouse of a person's last marriage row, dead or alive, else 0."""
        for mr in reversed(self.marriages):
            if pid in (mr.husband, mr.wife):
                return mr.wife if mr.husband == pid else mr.husband
        return 0

    def vitals(self) -> None:
        """P10, B11: a mother's parity and last_birth_tick (D-11), synthesised or back-filled."""
        for p in self.persons.values():
            if p.sex == SEX_FEMALE and p.children:
                p.parity = len(p.children)
                p.last_birth = max(self.persons[c].birth for c in p.children)

    def headship(self) -> None:
        """P12, B11: H4 at genesis for a dead head's household."""
        for hh in self.households.values():
            living = [self.persons[pid] for pid in hh.members if self.persons[pid].alive]
            if not living:
                hh.dissolved = max(self.persons[pid].death or 0 for pid in hh.members)
                hh.head = 0
            elif not self.persons[hh.head].alive:
                partner = self.marriage_partner(hh.head)
                candidates = [q for q in living if q.id == partner] or living
                hh.head = min(candidates, key=lambda q: (q.birth, q.id)).id

    def place(self) -> None:
        """S3 R1 to R3: one land-weighted draw per living household, keyed by its head."""
        templates = wl.load_templates(wl.TEMPLATES)
        world = wl.generate(templates, self.m.template, self.seed, self.m.raster, self.m.regions)
        cum = list(itertools.accumulate(world.land_count))
        for hh in self.households.values():
            if hh.head:
                x = pick(self.draw(PURPOSE_RESIDENCE, hh.head, 0), cum[-1])
                hh.residence = 1 + next(i for i, c in enumerate(cum) if c > x)

    # -- integerisation (P3, P6, P7) -----------------------------------------------------------
    def allocate(self, weights: list[int], total: int, slot: int) -> Allocation:
        """P3: systematic allocation of ``total`` units over cells in index order, keyed offset.

        The exact targets are T_k = w_k total / sum(w); with C_k their running sums and
        phi = (u >> 32) / 2^32 the offset (purpose synth_offset, entity = 16 i + slot, k = 0), cell
        k receives the points phi, phi + 1, ... that fall in [C_(k-1), C_k):
        ceil(C_k - phi) - ceil(C_(k-1) - phi) units, its floor or its ceiling. Every quantity is
        unsigned and below 2^128 when sum(w) (total + 1) < 2^96. A target at an integer value
        gets exactly it.
        """
        assert all(w >= 0 for w in weights)
        if total == 0:
            return Allocation(weights, 0, [0] * len(weights))
        s = sum(weights)
        assert s > 0, "nothing to allocate over"
        assert s * (total + 1) < 1 << WEIGHT_PRODUCT_BITS, "an allocation product does not fit u128"
        if self.control == "rounding-cells" and slot == SLOT_AGESEX:
            return Allocation(weights, total, [w * total // s for w in weights])
        phi = self.draw(PURPOSE_OFFSET, SLOT_STRIDE * PEOPLE_INDEX + slot, 0) >> (64 - OFFSET_BITS)
        scale = 1 << OFFSET_BITS
        # ceil(C_k - phi) = ceil(C_k + 1 - phi) - 1: every term unsigned, below 2^128 (R15).
        bounds = [
            ceil_div(c * total * scale + (scale - phi) * s, s * scale) - 1
            for c in itertools.accumulate(weights, initial=0)
        ]
        cells = [b - a for a, b in itertools.pairwise(bounds)]
        assert sum(cells) == total
        return Allocation(weights, total, cells)

    @staticmethod
    def ipf(seed: list[list[int]], rows: list[int], cols: list[int]) -> list[list[int]]:
        """P6: iterative proportional fitting in fixed point, 32 sweeps (a row step, a column step).

        Cells carry 32 fraction bits; a step rescales a line to its marginal with one rounding
        (half to even) per cell; a line whose sum is 0 stays 0. The result is in 2^-32 units.
        """
        assert sum(rows) == sum(cols), "the two marginals differ"
        x = [[z * IPF_SCALE for z in row] for row in seed]
        n_r, n_c = len(rows), len(cols)
        for _ in range(IPF_SWEEPS):
            for i in range(n_r):
                s = sum(x[i])
                x[i] = [rhe(v * rows[i] * IPF_SCALE, s) if s else 0 for v in x[i]]
            for j in range(n_c):
                s = sum(x[i][j] for i in range(n_r))
                for i in range(n_r):
                    x[i][j] = rhe(x[i][j] * cols[j] * IPF_SCALE, s) if s else 0
        return x

    def round_table(self, x: list[list[int]], rows: list[int], cols: list[int]) -> list[list[int]]:
        """P7: the two-marginal rounding of a fitted table (2^-32 units) to integers.

        Every cell is floored; the units each row and column still lack (d_i, e_j; their sums
        agree) are placed one per cell as a bipartite flow from rows to columns, found by shortest
        augmenting paths in canonical order (each row with a deficit, ascending; columns
        ascending; a used cell traversed backwards), first over the cells with a positive
        fractional part, then over any cell. Both marginals are met exactly and every cell is
        within one of its fitted value.
        """
        n_r, n_c = len(rows), len(cols)
        out = [[v >> 32 for v in row] for row in x]
        if self.control == "rounding-table":
            return out
        d = [rows[i] - sum(out[i]) for i in range(n_r)]
        e = [cols[j] - sum(out[i][j] for i in range(n_r)) for j in range(n_c)]
        assert all(v >= 0 for v in d) and all(v >= 0 for v in e), (
            "the fit's residual exceeds a unit"
        )
        assert sum(d) == sum(e), "the two marginals differ"
        flow = Flow(n_r, n_c, d, e)
        positive = {(i, j) for i in range(n_r) for j in range(n_c) if x[i][j] & (IPF_SCALE - 1)}
        everything = {(i, j) for i in range(n_r) for j in range(n_c)}
        for allowed in (positive, everything):
            while sum(flow.d) > 0 and flow.augment(allowed):
                pass
        assert sum(flow.d) == 0, "the rounding could not place every unit"
        for i, j in flow.used:
            out[i][j] += 1
        return out


Node = tuple[str, int]


class Flow:
    """P7's unit-capacity bipartite flow from rows (deficits d) to columns (deficits e)."""

    def __init__(self, n_r: int, n_c: int, d: list[int], e: list[int]) -> None:
        """Start with no unit placed."""
        self.n_r, self.n_c, self.d, self.e = n_r, n_c, d, e
        self.used: set[tuple[int, int]] = set()

    def path(self, i0: int, allowed: set[tuple[int, int]]) -> dict[Node, Node | None] | None:
        """Return the BFS tree of the shortest augmenting path from row i0, by node, or None."""
        parent: dict[Node, Node | None] = {("r", i0): None}
        queue: list[Node] = [("r", i0)]
        while queue:
            kind, v = queue.pop(0)
            if kind == "r":
                for j in range(self.n_c):
                    if (v, j) in allowed and (v, j) not in self.used and ("c", j) not in parent:
                        parent[("c", j)] = ("r", v)
                        if self.e[j] > 0:
                            parent[("end", 0)] = ("c", j)
                            return parent
                        queue.append(("c", j))
            else:
                for i in range(self.n_r):
                    if (i, v) in self.used and ("r", i) not in parent:
                        parent[("r", i)] = ("c", v)
                        queue.append(("r", i))
        return None

    def augment(self, allowed: set[tuple[int, int]]) -> bool:
        """Place one unit from the first row with a deficit that has an augmenting path."""
        for i0 in range(self.n_r):
            if self.d[i0] == 0:
                continue
            parent = self.path(i0, allowed)
            if parent is None:
                continue
            node = parent[("end", 0)]
            assert node is not None
            self.e[node[1]] -= 1
            while parent[node] is not None:
                prev = parent[node]
                assert prev is not None
                if node[0] == "c":
                    self.used.add((prev[1], node[1]))
                else:
                    self.used.discard((node[1], prev[1]))
                node = prev
            self.d[i0] -= 1
            return True
        return False


# ----------------------------------------------------------------------------- the cases
# The manifests, as specs/manifest/examples/*.yaml give them
# (python/tests/test_population_synthesis_cases.py checks these mirrors against the YAML). hamlet:
# 60 persons, Job's household of 40 and six cast members (manifest founders 46), no dynasty.
# manor: 400 persons, no cast, one polity with two dynasties, M7's majority_age 16.
HAMLET = Manifest(
    "hamlet",
    42,
    DAYS_360,
    "desert_fringe",
    256,
    4,
    60,
    46,
    E0_DEFAULT,
    BETA_DEFAULT,
    TFR_DEFAULT,
    MULT_DEFAULT,
    None,
    (),
)
MANOR = Manifest(
    "manor",
    1,
    DAYS_360,
    "flat",
    64,
    2,
    400,
    0,
    E0_DEFAULT,
    BETA_DEFAULT,
    "4.9",
    "1.2",
    "16",
    (Polity("manor_of_ash", "primogeniture", (Dynasty("ashcombe", 3), Dynasty("thorne", 2))),),
)
CASES: tuple[Manifest, ...] = (
    HAMLET,
    Manifest(**{**HAMLET.__dict__, "seed": 7}),
    MANOR,
    Manifest(**{**MANOR.__dict__, "seed": 2}),
)
# The law sweep's second configuration (REV-SPR-019 R12): the manor's two houses at the manifest's
# deepest generations (6) on the 365-day calendar, so the tenure logs reach past half the blood
# window and the stand-in's adult branch fires; a sweep fixture, not a golden case (no YAML).
MANOR_G6 = Manifest(
    **{
        **MANOR.__dict__,
        "id": "manor_g6",
        "days_per_year": 365,
        "polities": (
            Polity(
                "manor_of_ash",
                "primogeniture",
                (Dynasty("ashcombe", 6), Dynasty("thorne", 6)),
            ),
        ),
    }
)
SWEEP_CONFIGS: tuple[Manifest, ...] = (MANOR, MANOR_G6)


def build(m: Manifest, control: str | None = None, *, synthesise: bool = True) -> Genesis:
    """B then P: the dynasties first (their living members count against the population)."""
    g = Genesis(m, control)
    g.backfill()
    if synthesise:
        g.synthesise()
    else:
        g.vitals()
    return g


def with_law(m: Manifest, law: str, seed: int | None = None) -> Manifest:
    """Return the manifest with every polity's succession law replaced, and a seed if given."""
    pols = tuple(Polity(p.id, law, p.dynasties) for p in m.polities)
    return Manifest(**{**m.__dict__, "polities": pols, "seed": m.seed if seed is None else seed})


# ----------------------------------------------------------------------------- invariants
class Checker:
    """Check 2: the spec's invariants on one built world."""

    def __init__(self, g: Genesis, *, backfill_only: bool = False) -> None:
        """Collect failures for ``g``; ``backfill_only`` skips the synthesis invariants."""
        self.g = g
        self.f: list[str] = []
        self.y = g.cal.year
        self.backfill_only = backfill_only

    def fail(self, msg: str) -> None:
        """Record a failure."""
        self.f.append(f"{self.g.m.id} seed {self.g.seed}: {msg}")

    def run(self) -> list[str]:
        """Run every check and return the failures."""
        self.ids()
        if not self.backfill_only:
            self.marginals()
            self.population()
        for p in self.g.persons.values():
            self.person(p)
            self.parents(p)
            self.siblings(p)
        self.titles()
        self.houses()
        return self.f

    def ids(self) -> None:
        """Ids are unique across persons, houses and titles; 0 is never issued."""
        g = self.g
        if 0 in g.persons:
            self.fail("PersonId 0 issued")
        houses, titles = {h.id for h in g.houses}, {t.id for t in g.titles}
        if len(houses) != len(g.houses) or len(titles) != len(g.titles):
            self.fail("house or title ids collide")

    def marginals(self) -> None:
        """P3, P5, P6, P7: the allocations meet their marginals and stay within a unit per cell."""
        r = self.g.report
        assert r.agesex is not None and r.women_by_age is not None and r.men_by_band is not None
        if sum(r.agesex.cells) != r.synthesised:
            self.fail("the age-sex cells do not sum to the synthesised count")
        if not r.agesex.within_one():
            self.fail("an age-sex cell is a unit or more from its exact value")
        women_caps = [len(ps) for ps in self.women_by_age()]
        if sum(r.women_by_age.cells) != r.married or sum(r.men_by_band.cells) != r.married:
            self.fail("the married totals differ")
        if any(c > cap for c, cap in zip(r.women_by_age.cells, women_caps, strict=True)):
            self.fail("an age holds more married women than women")
        if not (r.women_by_age.within_one() and r.men_by_band.within_one()):
            self.fail("a married marginal cell is a unit or more from its exact value")
        if [sum(row) for row in r.ipf] != r.women_by_band:
            self.fail("the couple table's row sums are not the married women per band")
        cols = [sum(r.ipf[i][j] for i in range(WIDE_BANDS)) for j in range(WIDE_BANDS)]
        if cols != r.men_by_band.cells:
            self.fail("the couple table's column sums are not the married men per band")
        cells = (
            (r.ipf[i][j], r.ipf_fixed[i][j]) for i in range(WIDE_BANDS) for j in range(WIDE_BANDS)
        )
        if any(abs(a * IPF_SCALE - b) > IPF_SCALE for a, b in cells):
            self.fail("a couple-table cell is more than a unit from its fitted value")

    def women_by_age(self) -> list[list[Person]]:
        """The synthesised women by whole-year age at the epoch (P4's cells)."""
        out: list[list[Person]] = [[] for _ in range(self.g.schedule.end_years)]
        for p in self.g.persons.values():
            if p.sex == SEX_FEMALE and p.generation < 0 and p.death is None:
                out[-p.birth // self.y].append(p)
        return out

    def person(self, p: Person) -> None:
        """A living person has a household with a living head and a settlement; the dead, before."""
        g = self.g
        if not p.alive:
            if p.death is not None and (p.death >= 0 or p.birth >= 0):
                self.fail("a closed row with a birth or death at or after the epoch")
        elif not self.backfill_only:
            hh = g.households.get(p.household)
            if (
                hh is None
                or hh.head == 0
                or not g.persons[hh.head].alive
                or hh.head not in hh.members
            ):
                self.fail(f"living person {p.id} has no household with a living head")
            elif hh.residence < 1:
                self.fail(f"living person {p.id} has no settlement")
        if p.generation >= 0 and p.birth >= 0:
            self.fail("a back-filled birth at or after the epoch")
        if p.generation >= 0 and (p.birth % DAY or (p.death is not None and p.death % DAY)):
            self.fail("a back-filled tick that is not a day start")
        if p.spouse:
            s = g.persons[p.spouse]
            if s.spouse != p.id or s.sex == p.sex or not (s.alive and p.alive):
                self.fail(f"spouse columns of {p.id} are not symmetric between two living persons")

    def parents(self, p: Person) -> None:
        """Parents of the right sex, in the window (B6: the conception; P10: the birth), alive."""
        if not p.mother:
            return
        g, y = self.g, self.y
        mo, fa = g.persons[p.mother], g.persons[p.father]
        if mo.sex != SEX_FEMALE or fa.sex != SEX_MALE:
            self.fail("parents' sexes")
        if p.conception is None:
            if not MIN_AGE_F * y <= p.birth - mo.birth < FERTILE_END * y:
                self.fail(f"mother of {p.id} outside the fertility window at the birth")
            if p.birth - fa.birth < MIN_AGE_M * y:
                self.fail(f"father of {p.id} under {MIN_AGE_M} at the birth")
        else:
            conceived = g.cal.month_start(p.conception)
            if band_of(conceived - mo.birth, y) is None:
                self.fail(f"mother of {p.id} outside the fertility window at the conception pulse")
            if conceived - fa.birth < MIN_AGE_M * y:
                self.fail(f"father of {p.id} under {MIN_AGE_M} at the conception pulse")
            if fa.death is not None and fa.death <= conceived:
                self.fail(f"child {p.id} conceived after its father's death")
        if mo.death is not None and mo.death < p.birth:
            self.fail(f"child {p.id} born after its mother's death")

    def siblings(self, mo: Person) -> None:
        """One mother's children are conceived a closed period apart, born a gap apart."""
        ps = self.g.persons
        pulses = sorted(c for c in (ps[k].conception for k in mo.children) if c is not None)
        if any(b - a <= CLOSED for a, b in itertools.pairwise(pulses)):
            self.fail(f"two conceptions of {mo.id} inside the closed period")
        births = sorted(ps[c].birth for c in mo.children)
        # P10's gap for a synthesised mother; M2's least gap on the calendar for a back-filled one
        gap = least_birth_gap(self.g.cal) if mo.generation >= 0 else SIBLING_GAP
        if any(b - a < gap for a, b in itertools.pairwise(births)):
            self.fail(f"two children of {mo.id} born less than the sibling gap apart")
        if mo.children and (mo.parity != len(mo.children) or mo.last_birth != births[-1]):
            self.fail(f"parity or last_birth_tick of {mo.id}")

    def population(self) -> None:
        """The living count at tick 0 is the manifest's population."""
        living = sum(1 for p in self.g.persons.values() if p.alive)
        if living + self.g.m.manifest_founders != self.g.m.population:
            self.fail(f"living count {living} plus the manifest founders is not the population")

    def titles(self) -> None:
        """B9, B10: contiguous tenures held by living house members, ending open at genesis."""
        g = self.g
        for t in g.titles:
            rows = t.tenures
            house = next(h for h in g.houses if h.title == t.id)
            last_ok = rows and rows[-1].end == I64_MAX and rows[-1].holder == t.holder
            if not last_ok or not g.persons[t.holder].alive:
                self.fail(
                    f"title {t.k}: the log does not end in an open tenure of the living holder"
                )
            if any(a.end != b.start for a, b in itertools.pairwise(rows)):
                self.fail(f"title {t.k}: tenures not contiguous")
            for r in rows:
                self.tenure(t, r, house)
            if rows and t.held_since != rows[-1].start:
                self.fail(f"title {t.k}: held_since")
            if (t.law in COURT_LAWS) != (len(rows) == 1 and rows[0].start == 0):
                self.fail(f"title {t.k}: a court law writes exactly the genesis row")
            if any(s.line and s.chosen != s.line[0] for s in t.successions):
                self.fail(f"title {t.k}: a succession did not seat the first of the line")

    def tenure(self, t: Title, r: Tenure, house: House) -> None:
        """One tenure: non-empty, a house member, alive throughout."""
        h = self.g.persons[r.holder]
        end = r.end if r.end != I64_MAX else 0
        if r.start > end or r.holder not in house.members or r.start % DAY:
            self.fail(f"title {t.k}: a tenure is empty, off a day start or held by a non-member")
        if h.birth > r.start or (h.death is not None and h.death < end):
            self.fail(f"title {t.k}: holder {r.holder} not alive through the tenure")

    def houses(self) -> None:
        """A house's members descend agnatically from its root; the row bound holds."""
        g = self.g
        for h in g.houses:
            if h.rows > MAX_ROWS:
                self.fail(f"house {h.k}: too many rows")
            for pid in h.members:
                p = g.persons[pid]
                if pid != h.root and (not p.father or g.persons[p.father].house != h.id):
                    self.fail("a house member's father is not of the house")


def least_birth_gap(cal: Calendar) -> int:
    """Return M2's least birth-to-birth gap on ``cal`` in minutes (P10; REV-SPR-019 R10).

    Two conceptions more than the closed period apart (pulses m and m + 21 at the closest) give a
    first birth no later than the last day start of month m + 9 and a second no earlier than the
    first day start of month m + 30: t_(m+30) + 1 day - (t_(m+10) - 1 day), minimised over m.
    """
    return min(
        cal.month_start(m + CLOSED + GESTATION + 1) - cal.month_start(m + GESTATION + 1) + 2 * DAY
        for m in range(MONTHS)
    )


def check_widths(g: Genesis) -> list[str]:
    """Check 4: the widths the spec states, and E(a) against the births reference's exact chain."""
    f = []
    for days in (DAYS_360, 365):
        if least_birth_gap(Calendar(days)) > SIBLING_GAP:
            f.append(f"the sibling gap is below M2's least birth-to-birth gap on {days} days")
    if (MAX_POPULATION * IPF_SCALE) * MAX_POPULATION * IPF_SCALE >= 1 << 128:
        f.append("an IPF product does not fit u128")
    oldest = -(6 * (FATHER_AGE[1] + 1) + YOUNGEST_YEARS + 1) * max(MONTHS_365) * MONTHS * DAY
    if oldest < I64_MIN:
        f.append("a root's birth tick does not fit i64")
    if MAX_POPULATION >= 1 << 24 or 2 * g.schedule.end_years * (2 * ONE) * 105 >= 1 << 48:
        f.append("the age-sex weights exceed the stated width")
    if any(u > Q64_ONE or u <= 0 for u in g.fert.unmarried):
        f.append("the unmarried chain leaves Q0.64")
    exact = m2f1.married_share_by_month(STANDARDS)
    for a in range(MIN_AGE_F, g.schedule.end_years):
        month = min((a - MIN_AGE_F) * MONTHS + MONTHS // 2, CHAIN_MONTHS - 1)
        if g.fert.ever_married(a) != round(exact[month] * ONE):
            f.append(f"E({a}) differs from the exact chain rounded once to Q0.32")
    return f


def check_spec(spec: dict[str, Any], values: dict[str, Any]) -> list[str]:
    """Check 4: the spec's constants equal this reference's and its check values hold."""
    f = []
    synthesis, backfill = spec_constants()
    for table, want in (("population_synthesis", synthesis), ("dynastic_backfill", backfill)):
        got = spec["algorithm"][table].get("constants", {})
        for key in sorted(set(want) | set(got)):
            if got.get(key) != want.get(key):
                f.append(
                    f"SPEC MISMATCH: [algorithm.{table}] constants.{key} = {got.get(key)!r}, "
                    f"the reference has {want.get(key)!r}"
                )
        for key, expected in spec["algorithm"][table].get("check_values", {}).items():
            if values.get(key) != expected:
                f.append(
                    f"SPEC MISMATCH: [algorithm.{table}] check_values.{key} = {expected!r}, "
                    f"the reference computes {values.get(key)!r}"
                )
    purposes = [p["purpose"] for p in spec.get("rng_purposes", [])]
    if purposes != [PURPOSE_BIRTHDAY, PURPOSE_RESIDENCE, *NEW_PURPOSES]:
        f.append("SPEC MISMATCH: [[rng_purposes]] are not the reference's purposes in order")
    return f


# ----------------------------------------------------------------------------- rendering
def sha(lines: Iterable[str]) -> str:
    """Return the SHA-256 of lines joined by newlines."""
    return hashlib.sha256("\n".join(lines).encode("ascii")).hexdigest()


def person_row(g: Genesis, p: Person) -> str:
    """Return one person's canonical row."""
    death = p.death if p.death is not None and p.death < 0 else I64_MAX
    hh = g.households.get(p.household)
    residence = hh.residence if p.alive and hh is not None else 0
    return (
        f"{p.id} {p.sex} {p.birth} {death} {p.mother} {p.father} {p.spouse if p.alive else 0} "
        f"{p.household if p.alive else 0} {residence} {p.generation} "
        f"{p.parity if p.alive else 0} {p.last_birth if p.alive else I64_MIN}"
    )


def render_house(g: Genesis, h: House) -> list[str]:
    """Return the golden lines of one house, its title and tenure log."""
    t = next(t for t in g.titles if t.id == h.title)
    living = sum(1 for pid in h.members if g.persons[pid].alive)
    return [
        f"house {h.k} {h.id} generations {h.generations} rows {h.rows} members {len(h.members)} "
        f"living {living} root {h.root}",
        f"title {t.k} {t.id} tier {t.tier} law {LAWS.index(t.law)} liege {t.liege} "
        f"polity {t.polity} holder {t.holder} held_since {t.held_since} "
        f"predecessor_exit {t.predecessor_exit} successions {len(t.successions)} "
        f"fallbacks {t.fallbacks} minors {t.minor_fallbacks}",
        *[f"tenure {t.k} {r.holder} {r.start} {r.end}" for r in t.tenures],
    ]


def render_case(g: Genesis) -> list[str]:
    """Return the golden lines of one case."""
    r = g.report
    assert r.agesex is not None and r.men_by_band is not None
    persons = [person_row(g, p) for p in sorted(g.persons.values(), key=lambda q: q.id)]
    marriages = sorted(f"{m.husband} {m.wife} {m.start} {m.end} {m.source}" for m in g.marriages)
    households = [
        f"{h.id} {h.head} {h.formed} {h.dissolved} {h.residence}"
        for h in sorted(g.households.values(), key=lambda h: h.id)
    ]
    members = sorted(f"{h.id} {pid}" for h in g.houses for pid in h.members)

    def ints(values: list[int]) -> str:
        return " ".join(map(str, values))

    lines = [
        f"case {g.m.id} {g.seed}",
        f"population {g.m.population} manifest_founders {g.m.manifest_founders} "
        f"living_dynasts {r.living_dynasts} synthesised {r.synthesised} persons {len(g.persons)}",
        f"schedule e0 {q16(g.m.e0)} beta {q16(g.m.brass_beta)} K {g.schedule.k} "
        f"end_years {g.schedule.end_years} tfr {q16(g.m.tfr)} level {g.fert.level_bits} "
        f"elite {g.fert.elite_bits}",
        f"agesex males {sum(r.agesex.cells[0::2])} females {sum(r.agesex.cells[1::2])} "
        f"sha256 {sha([ints(r.agesex.cells)])}",
        f"married {r.married} women {r.married_women} men {r.married_men} "
        f"women_by_band {ints(r.women_by_band)} men_by_band {ints(r.men_by_band.cells)}",
        *[f"ipf {i} {ints(row)}" for i, row in enumerate(r.ipf)],
        *[f"children {c} {ints(row)}" for c, row in enumerate(r.children_alloc)],
        f"leftover {r.leftover} orphans {r.orphans}",
        f"households {len(households)} marriages {len(marriages)} house_members {len(members)}",
    ]
    for h in g.houses:
        lines += render_house(g, h)
    return [
        *lines,
        f"persons_sha256 {sha(persons)}",
        f"marriages_sha256 {sha(marriages)}",
        f"households_sha256 {sha(households)}",
        f"house_members_sha256 {sha(members)}",
    ]


HEADER = [
    "# population synthesis v0 golden outputs (SPR-019): GENERATED by",
    "# python -I specs/worldgen/selftest_population_synthesis.py --write; do not edit.",
    "# Each case: the manifest and seed; counts; the schedule's compiled constants; the age-sex",
    "# cells' digest; the married marginals; the couple table (rows: wife band, columns: husband",
    "# band, 15-19 to 80+); the children allocation (rows: child band 0-4, 5-9, 10-14; columns:",
    "# mother band); each house and title with its tenure log (holder, start, end; i64::MAX while",
    "# held); digests of the person (id, sex, birth, death, mother, father, spouse, household,",
    "# residence, generation, parity, last_birth_tick), marriage, household and house_member rows",
    "# (sorted). Then the law sweep, per configuration (the manor; the manor at six generations on",
    "# the 365-day calendar) and law: the seed-1 houses and tenure logs, then the counts over the",
    "# sweep seeds (successions, stand-in fallbacks, minors seated, adult-branch fallbacks, women",
    "# holders, the longest log) and a digest of every seed's tenure logs and succession heads.",
    "constants population-synthesis-v0 backfill-v0",
]


def render(control: str | None) -> tuple[str, list[str], dict[str, Any]]:
    """Build every case; return the golden text, the failures of checks 2 and 4, check values."""
    lines = HEADER[:]
    failures: list[str] = []
    values: dict[str, Any] = {}
    for m in CASES:
        g = build(m, control)
        failures += Checker(g).run() + check_widths(g)
        lines += render_case(g)
        case_values(g, values)
    return "\n".join(lines) + "\n", failures, values


def case_values(g: Genesis, values: dict[str, Any]) -> None:
    """Record the quantities the spec's check_values name for one golden case."""
    r = g.report
    values.update(
        tfr1_bits=g.fert.tfr1_bits,
        sex_male_bits=g.fert.sex_male_bits,
        level_bits_at_default=compile_fertility(q16(TFR_DEFAULT), q16(MULT_DEFAULT)).level_bits,
        ever_married_q32_at_19=g.fert.ever_married(19),
        ever_married_q32_at_49=g.fert.ever_married(49),
        schedule_end_years_at_default=Schedule(E0_DEFAULT, BETA_DEFAULT, g.cal).end_years,
        least_birth_gap_minutes_360=least_birth_gap(Calendar(DAYS_360)),
        least_birth_gap_minutes_365=least_birth_gap(Calendar(365)),
    )
    key = f"{g.m.id}_{g.seed}"
    values[f"{key}_synthesised"] = r.synthesised
    values[f"{key}_married"] = r.married
    values[f"{key}_living_dynasts"] = r.living_dynasts
    values[f"{key}_orphans"] = r.orphans
    for h in g.houses:
        t = next(t for t in g.titles if t.id == h.title)
        living = sum(1 for pid in h.members if g.persons[pid].alive)
        values[f"{key}_house_{h.k}"] = {
            "generations": h.generations,
            "rows": h.rows,
            "members": len(h.members),
            "living": living,
            "tenures": len(t.tenures),
        }


def log_lines(seed: int, g: Genesis) -> list[str]:
    """One configuration's tenure logs and successions at a seed, for the sweep's per-law digest."""
    out: list[str] = []
    for t in g.titles:
        out += [f"{seed} {t.k} tenure {r.holder} {r.start} {r.end}" for r in t.tenures]
        out += [
            f"{seed} {t.k} succession {s.tick} head {s.line[0] if s.line else 0} chosen {s.chosen}"
            for s in t.successions
        ]
    return out


@dataclass
class SweepCounts:
    """One law's counts over a configuration's sweep seeds (REV-SPR-019 R9, R12)."""

    successions: int = 0
    fallbacks: int = 0
    minors: int = 0  # stand-in successions that seated a minor
    adult_fallbacks: int = 0  # stand-in successions decided by the adult branch
    women: int = 0  # tenure rows held by women
    longest: int = 0  # the longest tenure log
    log: list[str] = field(default_factory=list)

    def add(self, seed: int, g: Genesis) -> None:
        """Count one built configuration."""
        for t in g.titles:
            self.successions += len(t.successions)
            self.fallbacks += t.fallbacks
            self.minors += t.minor_fallbacks
            self.adult_fallbacks += t.adult_fallbacks
            self.women += sum(1 for r in t.tenures if g.persons[r.holder].sex == SEX_FEMALE)
            self.longest = max(self.longest, len(t.tenures))
        self.log += log_lines(seed, g)

    def values(self) -> dict[str, int]:
        """The counts the spec pins."""
        return {
            "successions": self.successions,
            "fallbacks": self.fallbacks,
            "minors": self.minors,
            "adult_fallbacks": self.adult_fallbacks,
        }


def law_sweep(control: str | None, seeds: int) -> tuple[list[str], list[str], dict[str, Any]]:
    """Check 3: the back-fill under each law over the sweep seeds and configurations; golden lines.

    For each configuration and law: the manor's seed-1 houses and tenure logs, then one line with
    the counts and a digest of every seed's tenure logs and successions (each line's head and the
    seated holder), so a changed window, a changed stand-in or a changed law moves a golden line.
    """
    lines: list[str] = []
    failures: list[str] = []
    values: dict[str, Any] = {}
    adult_total = 0
    for m in SWEEP_CONFIGS:
        for law in LAWS:
            counts = SweepCounts()
            for seed in range(1, seeds + 1):
                g = build(with_law(m, law, seed), control, synthesise=False)
                failures += [f"{m.id} law {law}: {x}" for x in Checker(g, backfill_only=True).run()]
                if seed == 1:
                    lines.append(f"sweep {m.id} {law} seed {seed}")
                    for h in g.houses:
                        lines += render_house(g, h)
                counts.add(seed, g)
            lines.append(
                f"sweep {m.id} {law} seeds {seeds} successions {counts.successions} "
                f"fallbacks {counts.fallbacks} minors {counts.minors} "
                f"adult_fallbacks {counts.adult_fallbacks} women_holders {counts.women} "
                f"longest_log {counts.longest} logs_sha256 {sha(counts.log)}"
            )
            values[f"sweep_{m.id}_{law}"] = counts.values()
            adult_total += counts.adult_fallbacks
    values["sweep_adult_fallbacks"] = adult_total
    if adult_total == 0:
        failures.append("the stand-in's adult branch never fires in the sweep (R9 pins one case)")
    return lines, failures, values


def first_difference(current: str, text: str) -> int:
    """Return the 1-based line of the first difference between two texts."""
    a, b = current.splitlines(), text.splitlines()
    pairs = enumerate(zip(a, b, strict=False))
    return next((i for i, (x, z) in pairs if x != z), min(len(a), len(b))) + 1


# ----------------------------------------------------------------------------- differential gate
def effect_world(g: Genesis, title: Title, s: Succession) -> dict[str, Any]:
    """The back-filled log at a succession, as an effect-language fixture world (golden header)."""
    t = s.tick
    existing = [p for p in g.persons.values() if p.birth <= t]
    sex = {SEX_MALE: "male", SEX_FEMALE: "female"}
    return {
        "now": t,
        "days_per_year": g.m.days_per_year,
        "params": {"M7.dynasty": {"majority_age": g.m.majority_age}},
        "life_course": [
            {
                "person_id": p.id,
                "mother": p.mother,
                "father": p.father,
                "birth_tick": p.birth,
                "sex": sex[p.sex],
                "death_cause": "none" if p.alive_at(t) else "natural",
            }
            for p in existing
        ],
        "person": [
            {
                "id": p.id,
                "alive": True,
                "sex": sex[p.sex],
                "birth_tick": p.birth,
                "death_tick": I64_MAX,
                "death_cause": "none",
                "mother": p.mother,
                "father": p.father,
            }
            for p in existing
            if p.alive_at(t)
        ],
        "title": [
            {
                "id": x.id,
                "tier": TIER_NAMES[x.tier],
                "liege": x.liege,
                "polity": x.polity,
                "law": x.law,
                "holder": s.holder if x.id == title.id else 0,
                "state": "held",
                "held_since": title.tenures[s.rows - 1].start if x.id == title.id else 0,
                "regent": 0,
            }
            for x in g.titles
        ],
        "tenure": [
            {"id": 600 + i, "title": title.id, "holder": r.holder, "start": r.start, "end": r.end}
            for i, r in enumerate(title.tenures[: s.rows])
        ],
    }


def effect_differential(ref_dir: Path, seeds: int) -> list[str]:
    """Run every genealogical-law succession of both sweep configurations through effect_ref.py.

    Then (REV-SPR-030 R27) the control on the same genesis and the same successions: with
    effect_ref.py's ``succession_line`` reversed, the differential must report differences (the
    lines of two or more candidates), else the control itself failed.
    """
    ref = _load("effect_ref", ref_dir / "effect_ref.py")
    golden = ref.load_golden(ref_dir / "effect_language_golden_v0.toml")
    cases: list[tuple[str, int, int, int, list[int], Any, dict[str, Any]]] = []
    for m, law in itertools.product(SWEEP_CONFIGS, GENEALOGICAL_LAWS):
        for seed in range(1, seeds + 1):
            g = build(with_law(m, law, seed), synthesise=False)
            for title in g.titles:
                for s in title.successions:
                    case = {
                        "kind": "succession",
                        "system": "M7.dynasty",
                        "law": law,
                        "world": "backfill",
                        "bind": {"holder": f"person:{s.holder}", "title": f"title:{title.id}"},
                    }
                    cases.append(
                        (law, seed, title.k, s.tick, s.line, effect_world(g, title, s), case)
                    )
    patched: Any = ref  # the module's attribute is swapped for the control, then restored
    original = ref.succession_line

    def compare(label: str) -> list[str]:
        out = []
        for law, seed, k, tick, expected, world, case in cases:
            golden.worlds["backfill"] = world
            programs = golden.reg.programs["succession_laws"][law]
            line = ref.succession_line(ref.prepare(golden, case), programs)
            if line != expected:
                out.append(
                    f"{label} law {law} seed {seed} title {k} at {tick}: the reference's line "
                    f"{expected} differs from effect_ref.py's {line}"
                )
        return out

    failures = compare("")
    print(
        f"EFFECT DIFFERENTIAL: {len(cases)} successions of {len(GENEALOGICAL_LAWS)} laws over "
        f"{len(SWEEP_CONFIGS)} configurations compared"
    )
    patched.succession_line = lambda run, programs: list(reversed(original(run, programs)))
    reversed_failures = compare("reversed")
    patched.succession_line = original
    if not reversed_failures:
        failures.append("REVERSED LINE CONTROL FAILED: a reversed succession_line differs nowhere")
    else:
        print(
            f"effect differential reversed-line control: {len(reversed_failures)} of "
            f"{len(cases)} successions differ (as required)"
        )
    return failures


def render_control(control: str) -> list[str]:
    """A planted violation: build the cases until one fails its checks (no sweep, no spec).

    REV-SPR-030 R10: a control needs the first case that exhibits it, not the whole reference run.
    """
    for m in CASES:
        g = build(m, control)
        failures = Checker(g).run() + check_widths(g)
        if failures:
            return failures
    return []


def parse(argv: list[str] | None) -> argparse.Namespace:
    """The command line."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--write", action="store_true", help="regenerate the golden file")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the golden file to check")
    parser.add_argument("--spec", type=Path, default=SPEC, help="the spec file to check against")
    parser.add_argument(
        "--control",
        choices=("spacing", "rounding-cells", "rounding-table"),
        help="plant a violation",
    )
    parser.add_argument("--effect-ref", type=Path, help="run the differential gate on this dir")
    parser.add_argument("--sweep-seeds", type=int, default=SWEEP_SEEDS, help="law sweep seeds")
    parser.add_argument("--altered-golden", type=Path, help="an altered golden that must be STALE")
    parser.add_argument("--stale-spec", type=Path, help="a spec copy that must SPEC MISMATCH")
    return parser.parse_args(argv)


def controls_in_run(args: argparse.Namespace, text: str, values: dict[str, Any]) -> list[str]:
    """The altered-golden and stale-spec controls, on the one reference run (REV-SPR-030 R10)."""
    f = []
    if args.altered_golden is not None:
        altered = args.altered_golden.read_text(encoding="utf-8")
        if altered == text:
            f.append(f"ALTERED GOLDEN CONTROL FAILED: {args.altered_golden} equals the reference")
        else:
            line = first_difference(altered, text)
            print(f"altered golden control: STALE at line {line} (as required)")
    if args.stale_spec is not None:
        with args.stale_spec.open("rb") as fh:
            mismatches = check_spec(tomllib.load(fh), values)
        if not any("SPEC MISMATCH" in x for x in mismatches):
            f.append(f"STALE SPEC CONTROL FAILED: {args.stale_spec} passes the spec check")
        else:
            print(f"stale spec control: {len(mismatches)} SPEC MISMATCH line(s) (as required)")
    return f


def main(argv: list[str] | None = None) -> int:
    """Check or regenerate the golden file; run the reference's own checks and the controls."""
    args = parse(argv)
    if args.control:
        failures = render_control(args.control)
        if not failures:
            print(f"CONTROL INERT: {args.control} failed no check")
            return EXIT_OK
        for x in failures[:20]:
            print(f"REFERENCE FAILED: {x}")
        print(f"POPULATION SYNTHESIS: FAIL (the reference's own checks: {len(failures)} failures)")
        return EXIT_REFERENCE
    try:
        text, failures, values = render(None)
        sweep_lines, sweep_failures, sweep_values = law_sweep(None, args.sweep_seeds)
        text += "\n".join(sweep_lines) + "\n"
        failures += sweep_failures
        values = {**values, **sweep_values}
        with args.spec.open("rb") as f:
            failures += check_spec(tomllib.load(f), values)
        if args.effect_ref is not None:
            failures += effect_differential(args.effect_ref, args.sweep_seeds)
    except AssertionError as e:
        failures, text, values = [f"construction error: {e}"], "", {}
    if failures:
        for x in failures[:20]:
            print(f"REFERENCE FAILED: {x}")
        print(f"POPULATION SYNTHESIS: FAIL (the reference's own checks: {len(failures)} failures)")
        return EXIT_REFERENCE
    if args.write:
        args.golden.parent.mkdir(parents=True, exist_ok=True)
        args.golden.write_text(text, encoding="utf-8")
        print(f"wrote {args.golden}")
        return EXIT_OK
    return compare_and_control(args, text, values)


def compare_and_control(args: argparse.Namespace, text: str, values: dict[str, Any]) -> int:
    """The golden comparison, then the in-run controls; the verdict line."""
    current = args.golden.read_text(encoding="utf-8") if args.golden.exists() else ""
    if current != text:
        line = first_difference(current, text)
        print(f"STALE: {args.golden} differs from the reference at line {line}")
        print("POPULATION SYNTHESIS: FAIL")
        return EXIT_FAIL
    control_failures = controls_in_run(args, text, values)
    if control_failures:
        for x in control_failures:
            print(x)
        print("POPULATION SYNTHESIS: FAIL")
        return EXIT_FAIL
    print(f"POPULATION SYNTHESIS: PASS ({len(CASES)} cases, {text.count(chr(10))} lines)")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
