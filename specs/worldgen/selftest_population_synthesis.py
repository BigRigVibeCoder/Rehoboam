"""Exact reference and self-test of population synthesis v0 and the dynastic back-fill (SPR-019).

The algorithm is stated in ``specs/worldgen/population_synthesis.toml``:
[algorithm.population_synthesis] steps P0 to P12 (the anonymous rest of a people's ``population``:
a stationary age-sex marginal, couples by iterative proportional fitting, dependent children,
households, residence) and [algorithm.dynastic_backfill] steps B0 to B10 (each manifest dynasty's
pre-epoch genealogy, marriages, deaths and tenure log back to its ``generations``). This file
implements those steps with Python integers and exact fractions only (no float anywhere), on the
keyed RNG of ``bin/gen_rng_golden.py`` (the independent rh-ids reference), the schedule of
``specs/demography/selftest_m2_rung0.py`` and the worldgen-lite reference beside it, so it
reproduces the engine's integers bit for bit. It checks:

  1. the committed golden outputs (``specs/worldgen/golden/population_synthesis_v0.txt``) equal
     this reference's, for two manifests (hamlet, manor) at two seeds each; ``--write``
     regenerates the file, never by hand;
  2. the invariants of the spec on every case: the allocated tables meet their marginals exactly
     and each cell is within one of its exact value (P3, P5, P7, P9); every id is unique and no id
     needs the rehash; every parent is inside the fertility window at each child's birth (mother
     15 to 49, father 18 or more) and alive at it (the father at the conception pulse); the
     children of one mother are at least the closed period apart; every back-filled birth and
     every closed row's death precede the epoch; the tenure rows of each title are contiguous,
     non-overlapping, held by a living male of the house, and end at genesis with the title's
     holder; the living count at tick 0 equals the manifest's population; every living person has
     a household with a living head and a settlement;
  3. the law sweep: the back-fill of the manor's houses under each of the seven succession laws
     keeps those invariants;
  4. the widths the spec states (the IPF products fit u128, every tick fits i64) and the
     constants (TFR1, the level, the conception table) equal the births spec's check values.

Run: python -I specs/worldgen/selftest_population_synthesis.py           (check; exit 1 on failure)
     python -I specs/worldgen/selftest_population_synthesis.py --write   (regenerate the golden)
     python -I specs/worldgen/selftest_population_synthesis.py --golden PATH  (check another file)
     python -I specs/worldgen/selftest_population_synthesis.py --control NAME (a planted violation;
         NAME is ``spacing`` (B6's closed period skipped) or ``rounding`` (P3 and P7 replaced by
         floors): each must FAIL the invariant that guards it)
Exit: 0 pass, 1 fail, 2 the reference failed its own checks. Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import itertools
import sys
from collections.abc import Callable
from dataclasses import dataclass, field
from fractions import Fraction
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GOLDEN = HERE / "golden" / "population_synthesis_v0.txt"
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
rung0 = _load("selftest_m2_rung0", ROOT / "specs" / "demography" / "selftest_m2_rung0.py")
rng = wl.rng  # the rh-ids reference (bin/gen_rng_golden.py), through the worldgen reference

# ----------------------------------------------------------------------------- constants
ONE = 1 << 32  # Prob32's 1; l(x) carries 32 fraction bits
MASK64 = (1 << 64) - 1
I64_MAX = (1 << 63) - 1
DAY = 1440
MONTHS = 12
DAYS_360 = 360
MONTHS_365 = (31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)  # rh-core time.rs
Q16 = 1 << 16
THOUSAND = 1000
MAX_POPULATION = 10_000_000  # the manifest's bound on a people's population
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
PURPOSE_SORTITION = "backfill_sortition"
NEW_PURPOSES = (
    PURPOSE_OFFSET,
    PURPOSE_SPINE,
    PURPOSE_MARRIAGE,
    PURPOSE_CONCEPTION,
    PURPOSE_SEX,
    PURPOSE_DEATH,
    PURPOSE_SORTITION,
)
# M2 F1 constants: read from specs/systems/next/M2.demography.odd.toml ([algorithm.fertility_f1] and
# [algorithm.partnering_f1] `constants`) through the births reference: a changed constant moves the
# goldens here (SPR-015 merged in PR #50).
m2f1 = _load("selftest_m2_f1", ROOT / "specs" / "demography" / "selftest_m2_f1.py")
M2_SPEC = m2f1.load_spec(m2f1.NEXT_SPEC)
STANDARDS = m2f1.Standards(M2_SPEC)
NATURAL = tuple(int(n) for n in STANDARDS.natural)  # Coale-Trussell n(a), thousandths, bands 15-19..45-49
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
TFR_DEFAULT = "4.9"
MULT_DEFAULT = "1.0"
E0_DEFAULT = "30"
# The births spec's check values (its [algorithm.fertility_f1] check_values).
CHECK_TFR1_BITS = 615614
CHECK_LEVEL_BITS = 34186
CHECK_CONCEPTION = (119397409, 143118858, 128689234, 112318286, 83490541, 36474834, 4576314)
CHECK_SEX_MALE_BITS = 2199861298
# [algorithm.population_synthesis] constants (design values; the companion says why).
AGE_MAX = 100  # ages 0..99: the rung-0 schedule reaches l = 0 at 100
CHILD_MAX = 15  # a dependant: aged 0..14 at the epoch; everyone older heads a household or marries
WIDE_BANDS = 14  # five-year bands 15-19 .. 80+ (the last is open), for the couple table
HUSBAND_SHIFT = 3  # a man's marriage pattern is a woman's three years younger (the F1 chain's gap)
MARRIAGE_REF_AGE = 19  # the widowhood factor starts at the women's mean age at first marriage
GAP_SEED = {0: 8, 1: 8, 2: 8, -1: 4, 3: 4}  # IPF seed by husband band minus wife band; else 1
GAP_SEED_OTHER = 1
IPF_SWEEPS = 32
IPF_SCALE = 1 << 32  # cells carry 32 fraction bits
CHILD_BANDS = 3  # 0-4, 5-9, 10-14
SIBLING_GAP_YEARS = 2  # two genesis children of one mother differ by at least two years of age
BAND_REPRESENTATIVE = 2  # P9: a band's representative age is its lower bound plus two years
# [algorithm.dynastic_backfill] constants (design values).
YOUNGEST_YEARS = 20  # generation G of a dynasty is born in the 20 years before the epoch
FATHER_AGE = (19, 45)  # a spine father's age in whole years at the heir's birth
MOTHER_AGE = (16, 35)  # a spine mother's age at the heir's birth
GROOM_AGE = (18, 30)  # a younger son's age at his marriage
BRIDE_AGE = (15, 25)  # a bride's age in whole years at her marriage (spine wives and brides alike)
ADULT_YEARS = 16  # the back-fill's majority: the first tenure starts, and seniority counts, from it
MAX_ROWS = 4096  # life-course rows per dynasty; the lineage relation's bound (effect language §5.2)
LAWS = (
    "primogeniture",
    "ultimogeniture",
    "seniority",
    "partition",
    "elective",
    "tanistry",
    "sortition",
)
ELDEST_FIRST = ("seniority", "tanistry", "elective")
SEX_MALE, SEX_FEMALE = 0, 1
ORIGIN_FOUNDER = 1
CAUSE_NATURAL = 1
EXIT_DEATH = 1
SOURCE_GENESIS = 0
EXIT_NONE, EXIT_NATURAL = 0, 1
TIER_SOVEREIGN, TIER_MAJOR = 0, 1
# Offset slots of the allocations of P3, P5 and P9 (purpose synth_offset, entity = the slot, k = 0).
SLOT_AGESEX = 0
SLOT_WOMEN = 1
SLOT_MEN = 2
SLOT_CHILDREN = 3  # + the child band
# Draw indices of backfill_marriage (B8), entity = the groom.
DRAW_GROOM_AGE, DRAW_GROOM_DAY, DRAW_BRIDE_AGE, DRAW_BRIDE_DAY = 0, 1, 2, 3

Cells = dict[tuple[int, int], list["Person"]]


def rhe(n: int, d: int) -> int:
    """Return n / d rounded to the nearest integer, ties to even (S0); d > 0."""
    return int(rung0.rhe(n, d))


def pick(u: int, n: int) -> int:
    """Return floor(u n / 2^64) for a u64 draw: an index in [0, n), n >= 1 (W0)."""
    return int(wl.pick(u, n))


def hits(draw: int, p: int) -> bool:
    """Return the Prob32 mapping: hits iff p = 2^32 or draw < p 2^32."""
    return p == ONE or draw < p << 32


def ceil_day(t: int) -> int:
    """Return the day start at or after t."""
    return -(-t // DAY) * DAY


def ceil_frac(x: Fraction) -> int:
    """Return the least integer at or above x."""
    return int(-(-x // 1))


def q16(literal: str) -> int:
    """Return the Q16_16 bits of a decimal literal, rounded half to even (SPR-011 ruling 9)."""
    return int(rung0.q16(literal))


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
class Schedule:
    """The compiled rung-0 mortality schedule at e0 (S1-S7) and the inverse draw of a death age."""

    def __init__(self, e0_bits: int, cal: Calendar) -> None:
        """Compile K and the grid survivors for ``e0_bits`` (Q16_16) on ``cal``."""
        self.cal = cal
        self.k = int(rung0.solve_k(e0_bits))
        self.grid = [int(v) for v in rung0.survivors(self.k)]
        self.ages: list[int] = [int(h) * cal.year // 2 for h in rung0.H]  # grid ages (S7)

    def survivors(self, x: int) -> int:
        """Return S7's l(x) for an age of x minutes."""
        return int(rung0.ell(self.grid, x, self.cal.year))

    def death_age(self, u: int, y: int) -> int:
        """Return B3's death age (minutes) of a person alive at age y, from the draw u.

        T = floor(u l(y) / 2^64) is the survivorship level at death, in [0, l(y)); the death age is
        the least whole minute x at which the linear interpolation of the grid falls to T or below:
        x = A_(i-1) + ceil((L_(i-1) - T)(A_i - A_(i-1)) / (L_(i-1) - L_i)) for the first grid index
        i with L_i <= T. P(x >= a) = l(a) / l(y) up to the grid's interpolation; x > y always.
        """
        ly = self.survivors(y)
        assert ly > 0, "a death cannot be drawn for an age the schedule has already ended"
        target = (u * ly) >> 64
        i = next(i for i in range(1, len(self.grid)) if self.grid[i] <= target)
        a0, a1 = self.ages[i - 1], self.ages[i]
        l0, l1 = self.grid[i - 1], self.grid[i]
        x = a0 + -(-((l0 - target) * (a1 - a0)) // (l0 - l1))
        assert x > y
        return x


class Fertility:
    """F1's compiled fertility and nuptiality constants (F2-F4, N1), from the births reference."""

    def __init__(self, tfr_bits: int, mult_bits: int) -> None:
        """Compile the level M, the elite level and the fourteen conception probabilities."""
        st = STANDARDS
        self.nuptiality_bits = [int(b) for b in m2f1.nuptiality_bits(st)]
        self.married_share = [Fraction(x) for x in m2f1.married_share_by_month(st)]  # S(a), F3
        self.tfr1_bits = int(m2f1.tfr1_bits(st))
        self.level_bits = int(m2f1.level_bits(st, tfr_bits))
        self.elite_bits = int(m2f1.elite_level_bits(self.level_bits, mult_bits))
        self.conception = [int(m2f1.conception_bits(st, self.level_bits, i)) for i in range(BANDS)]
        self.conception_elite = [
            int(m2f1.conception_bits(st, self.elite_bits, i)) for i in range(BANDS)
        ]
        self.sex_male_bits = int(m2f1.sex_male_bits(st))

    def ever_married(self, age_years: int) -> Fraction:
        """Return P5's E(a): the share ever married at the middle month of whole-year age a."""
        if age_years < MIN_AGE_F:
            return Fraction(0)
        if age_years >= FERTILE_END:
            return self.married_share[-1]
        return self.married_share[(age_years - MIN_AGE_F) * MONTHS + MONTHS // 2]


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
    children: list[int] = field(default_factory=list)

    @property
    def alive(self) -> bool:
        """Alive at tick 0: no drawn death, or one at or after the epoch."""
        return self.death is None or self.death >= 0

    def alive_at(self, t: int) -> bool:
        """Born and not yet dead at tick t (a back-filled person)."""
        assert self.death is not None
        return self.birth <= t < self.death


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


@dataclass
class House:
    """A house (M7 house.*) with its dynasty's genealogy."""

    id: int
    k: int
    generations: int
    title: int
    members: list[int] = field(default_factory=list)  # house_member rows (the agnatic tree)
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
    tfr: str
    elite_fertility_mult: str
    polities: tuple[Polity, ...]


@dataclass
class Report:
    """What the synthesis computed on the way, for the golden lines and the invariants."""

    synthesised: int = 0
    living_dynasts: int = 0
    agesex: list[int] = field(default_factory=list)
    agesex_exact: list[Fraction] = field(default_factory=list)
    married: int = 0
    women_by_age: list[int] = field(default_factory=list)
    women_by_band: list[int] = field(default_factory=list)
    men_by_band: list[int] = field(default_factory=list)
    ipf: list[list[int]] = field(default_factory=list)
    ipf_exact: list[list[Fraction]] = field(default_factory=list)
    children_alloc: list[list[int]] = field(default_factory=list)
    leftover: int = 0
    orphans: int = 0


@dataclass
class Spine:
    """B4's anchors of one dynasty: births of S_0..S_G, births of W_0..W_(G-1), marriage ticks."""

    births: list[int]
    wives: list[int]
    marriages: list[int]


class Genesis:
    """The state the two algorithms build for one manifest and seed."""

    def __init__(self, m: Manifest, control: str | None = None) -> None:
        """Compile the schedules; nothing is generated yet."""
        self.m = m
        self.control = control
        self.seed = m.seed
        self.cal = Calendar(m.days_per_year)
        self.schedule = Schedule(q16(m.e0), self.cal)
        self.fert = Fertility(q16(m.tfr), q16(m.elite_fertility_mult))
        self.persons: dict[int, Person] = {}
        self.order: list[int] = []  # ids in genesis order (k)
        self.next_k = m.manifest_founders
        self.marriages: list[Marriage] = []
        self.households: dict[int, Household] = {}
        self.houses: list[House] = []
        self.titles: list[Title] = []
        self.report = Report()

    # -- keys and ids ------------------------------------------------------------------------
    def draw(self, purpose: str, entity: int, k: int) -> int:
        """Return H(world_seed, canonical salt, rh-worldgen, epoch 0, purpose, entity, t = 0, k)."""
        return int(wl.draw(self.seed, purpose, entity, k))

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
        after the epoch, or whose mother dies before the birth, bears no child.
        """
        assert h.death is not None and w.death is not None
        m0 = self.cal.month_of(tick) + 1
        c_h = heir.conception if heir is not None else None
        if heir is not None and heir.household == 0:
            self.join(heir.id, w.household)
        last: int | None = None
        child_index = len(w.children)
        end = min(0, w.death, h.death, w.birth + FERTILE_END * self.cal.year)
        for m in itertools.count(m0):
            t_m = self.cal.month_start(m)
            if t_m >= end:
                return
            if m == c_h:
                last = m  # the forced heir, conceived now (B4); no draw
            band = band_of(t_m - w.birth, self.cal.year)
            if band is None or not self.susceptible(m, last, c_h):
                continue
            table = self.fert.conception_elite if w.house else self.fert.conception
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
    def living_males(self, house: House, t: int) -> list[Person]:
        """Return the house's males alive at tick t (birth <= t < death), ascending id."""
        ps = (self.persons[pid] for pid in sorted(house.members))
        return [p for p in ps if p.sex == SEX_MALE and p.alive_at(t)]

    def preorder(self, house: House, youngest_first: bool) -> list[int]:
        """Return the house's agnatic tree in pre-order, children by birth tick (ties by id)."""
        out: list[int] = []

        def visit(pid: int) -> None:
            out.append(pid)
            sons = [
                self.persons[c]
                for c in house.members
                if self.persons[c].father == pid and self.persons[c].sex == SEX_MALE
            ]
            sons.sort(key=lambda s: (-s.birth if youngest_first else s.birth, s.id))
            for s in sons:
                visit(s.id)

        visit(house.root)
        return out

    def heir(self, house: House, title: Title, t: int, index: int) -> int:
        """Return B9's next holder by law at a holder's death at tick t (the dead is not alive)."""
        living = self.living_males(house, t)
        assert living, "a spine member is always alive at a pre-epoch holder death (B4)"
        adults = [p for p in living if t - p.birth >= ADULT_YEARS * self.cal.year]
        if title.law in ELDEST_FIRST and adults:
            return min(adults, key=lambda p: (p.birth, p.id)).id
        if title.law == "sortition" and adults:
            return adults[pick(self.draw(PURPOSE_SORTITION, title.id, index), len(adults))].id
        alive = {p.id for p in living}
        order = self.preorder(house, title.law == "ultimogeniture")
        return next(pid for pid in order if pid in alive)

    def tenures(self, house: House, title: Title) -> None:
        """B9, B10: the tenure log from the root's majority to genesis, and the title at tick 0."""
        holder = self.persons[house.root]
        start = ceil_day(holder.birth + ADULT_YEARS * self.cal.year)
        rows: list[Tenure] = []
        while holder.death is not None and holder.death < 0:
            rows.append(Tenure(title.id, holder.id, start, holder.death))
            start = holder.death
            holder = self.persons[self.heir(house, title, start, len(rows))]
        rows.append(Tenure(title.id, holder.id, start, I64_MAX))
        title.tenures = rows
        title.holder = holder.id
        title.held_since = start
        title.predecessor_exit = EXIT_NATURAL if len(rows) > 1 else EXIT_NONE

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
        """Return P2's L_a = l(a Y) + l((a + 1) Y) for a = 0..99 (twice the person-years at a)."""
        y = self.cal.year
        return [
            self.schedule.survivors(a * y) + self.schedule.survivors((a + 1) * y)
            for a in range(AGE_MAX)
        ]

    def agesex_persons(self, n: int) -> Cells:
        """P2 to P4: the stationary age-sex marginal, its allocation and the persons by cell."""
        y = self.cal.year
        person_years = self.person_years()
        total_l = sum(person_years)
        weights = (SEX_RATIO[0], SEX_RATIO[1] - SEX_RATIO[0])
        targets = [
            Fraction(n * person_years[a] * weights[s], total_l * SEX_RATIO[1])
            for a in range(AGE_MAX)
            for s in (SEX_MALE, SEX_FEMALE)
        ]
        cells = self.allocate(targets, n, SLOT_AGESEX)
        self.report.agesex, self.report.agesex_exact = cells, targets
        out: Cells = {}
        for a in range(AGE_MAX):
            for s in (SEX_MALE, SEX_FEMALE):
                for _ in range(cells[2 * a + s]):
                    p = self.new_person(s, 0)
                    p.birth = -(a * y) - pick(self.draw(PURPOSE_BIRTHDAY, p.id, 0), y)
                    out.setdefault((a, s), []).append(p)
        return out

    def married_share(self, age_years: int) -> Fraction:
        """Return P5's C(a): ever married, times the husband's survival from the reference age."""
        e = self.fert.ever_married(age_years)
        if age_years <= MARRIAGE_REF_AGE:
            return e
        y = self.cal.year
        l_ref = self.schedule.survivors((MARRIAGE_REF_AGE + HUSBAND_SHIFT) * y)
        return e * Fraction(self.schedule.survivors((age_years + HUSBAND_SHIFT) * y), l_ref)

    def marital_marginals(self, by: Cells) -> tuple[list[int], list[int]]:
        """P5: married women per whole-year age and married men per wide band, equal totals."""
        women_caps = [len(by.get((a, SEX_FEMALE), [])) for a in range(AGE_MAX)]
        women_targets = [Fraction(women_caps[a]) * self.married_share(a) for a in range(AGE_MAX)]
        m_f = round(sum(women_targets, Fraction(0)))
        men_weights = [Fraction(0)] * WIDE_BANDS
        caps = [0] * WIDE_BANDS
        for a in range(MIN_AGE_M, AGE_MAX):
            men_here = len(by.get((a, SEX_MALE), []))
            caps[wide_band(a)] += men_here
            men_weights[wide_band(a)] += men_here * self.married_share(a - HUSBAND_SHIFT)
        m = min(m_f, sum(caps)) if sum(men_weights, Fraction(0)) > 0 else 0
        women = self.allocate(self.water_fill(women_targets, women_caps, m), m, SLOT_WOMEN)
        men = self.allocate(self.water_fill(men_weights, caps, m), m, SLOT_MEN)
        self.report.married, self.report.women_by_age, self.report.men_by_band = m, women, men
        rows = [0] * WIDE_BANDS
        for a in range(MIN_AGE_F, AGE_MAX):
            rows[wide_band(a)] += women[a]
        self.report.women_by_band = rows
        return women, men

    def couples(self, by: Cells, women: list[int], men: list[int]) -> list[tuple[int, int]]:
        """P6 to P8: the couple table by IPF and its rounding; pairing; marriages and households."""
        rows = self.report.women_by_band
        seed = [
            [GAP_SEED.get(j - i, GAP_SEED_OTHER) for j in range(WIDE_BANDS)]
            for i in range(WIDE_BANDS)
        ]
        fitted = self.ipf(seed, rows, men)
        table = self.round_table(fitted, rows, men)
        self.report.ipf, self.report.ipf_exact = table, fitted
        married_women: list[list[int]] = [[] for _ in range(WIDE_BANDS)]
        for a in range(MIN_AGE_F, AGE_MAX):
            ids = sorted(p.id for p in by.get((a, SEX_FEMALE), []))
            married_women[wide_band(a)] += ids[: women[a]]
        married_men: list[list[int]] = [[] for _ in range(WIDE_BANDS)]
        for a in range(MIN_AGE_M, AGE_MAX):
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
            weights = [Fraction(len(mothers[i]) * seed[i][c]) for i in range(WIDE_BANDS)]
            if sum(weights, Fraction(0)) == 0 or not by_band[c]:
                leftover += by_band[c]
                continue
            allocation[c] = self.allocate(weights, len(by_band[c]), SLOT_CHILDREN + c)
        self.report.children_alloc = allocation
        assign = self.assigner(age_of)
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

    def eligible(self, mother: int, child: int, age_of: dict[int, int]) -> bool:
        """P10: the mother in the fertility window at the birth, the father 18, siblings apart."""
        mo, ch = self.persons[mother], self.persons[child]
        fa = self.persons[mo.spouse]
        y = self.cal.year
        if not MIN_AGE_F * y <= ch.birth - mo.birth < FERTILE_END * y:
            return False
        if ch.birth - fa.birth < MIN_AGE_M * y:
            return False
        return all(abs(age_of[c] - age_of[child]) >= SIBLING_GAP_YEARS for c in mo.children)

    def assigner(self, age_of: dict[int, int]) -> Callable[[list[int], int], bool]:
        """Return P10's assignment: the eligible mother with the fewest children (ties: id)."""
        count: dict[int, int] = {}

        def assign(mothers: list[int], child: int) -> bool:
            options = [w for w in mothers if self.eligible(w, child, age_of)]
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

    def headship(self) -> None:
        """P12: parity; H4 at genesis: a dead head's household goes to the spouse or the oldest."""
        for p in self.persons.values():
            if p.sex == SEX_FEMALE:
                p.parity = len(p.children)
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

    # -- integerisation (P3, P5, P6, P7) --------------------------------------------------------
    def allocate(self, targets: list[Fraction], total: int, slot: int) -> list[int]:
        """P3: systematic allocation of ``total`` units over cells in index order, keyed offset.

        The targets are rescaled to sum to the total exactly; with C_k their running sums and
        phi = u / 2^64 the offset (purpose synth_offset, entity = slot, k = 0), cell k receives the
        points phi, phi + 1, ... that fall in [C_(k-1), C_k): ceil(C_k - phi) - ceil(C_(k-1) - phi)
        units, its floor or its ceiling. A target at an integer value gets exactly it, so a cell
        whose target was clipped to a cap never exceeds the cap (P5's water-filling).
        """
        assert all(t >= 0 for t in targets)
        if total == 0:
            return [0] * len(targets)
        total_t = sum(targets, Fraction(0))
        assert total_t > 0, "nothing to allocate over"
        scaled = [t * total / total_t for t in targets]
        if self.control == "rounding":
            return [int(t) for t in scaled]
        phi = Fraction(self.draw(PURPOSE_OFFSET, slot, 0), 1 << 64)
        bounds = [ceil_frac(c - phi) for c in itertools.accumulate(scaled, initial=Fraction(0))]
        out = [b - a for a, b in itertools.pairwise(bounds)]
        assert sum(out) == total
        return out

    @staticmethod
    def water_fill(weights: list[Fraction], caps: list[int], total: int) -> list[Fraction]:
        """P5: targets proportional to the weights, each clipped to its cap, summing to the total.

        A cell whose proportional share exceeds its cap is fixed at the cap and the rest is shared
        again among the others, until none exceeds; the caps must sum to at least the total.
        """
        assert sum(caps) >= total
        fixed: dict[int, Fraction] = {}
        while len(fixed) < len(weights):
            free = [i for i in range(len(weights)) if i not in fixed]
            rest = total - sum(fixed.values(), Fraction(0))
            w = sum((weights[i] for i in free), Fraction(0))
            over = [i for i in free if w and weights[i] * rest / w > caps[i]]
            if over:
                fixed.update({i: Fraction(caps[i]) for i in over})
                continue
            fixed.update({i: weights[i] * rest / w if w else Fraction(0) for i in free})
        return [fixed[i] for i in range(len(weights))]

    @staticmethod
    def ipf(seed: list[list[int]], rows: list[int], cols: list[int]) -> list[list[Fraction]]:
        """P6: iterative proportional fitting in fixed point, 32 sweeps (a row step, a column step).

        Cells carry 32 fraction bits; a step rescales a line to its marginal with one rounding
        (half to even) per cell; a line whose sum is 0 stays 0. The result is returned as exact
        fractions of 2^32 for P7.
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
        return [[Fraction(v, IPF_SCALE) for v in row] for row in x]

    def round_table(
        self, x: list[list[Fraction]], rows: list[int], cols: list[int]
    ) -> list[list[int]]:
        """P7: the two-marginal rounding of a fitted table to integers.

        Every cell is floored; the units each row and column still lack (d_i, e_j; their sums
        agree) are placed one per cell as a bipartite flow from rows to columns, found by shortest
        augmenting paths in canonical order (rows ascending, columns ascending, a used cell
        traversed backwards), first over the cells with a positive fractional part, then over any
        cell. Both marginals are met exactly and every cell is within one of its fitted value.
        """
        n_r, n_c = len(rows), len(cols)
        out = [[int(v) for v in row] for row in x]
        if self.control == "rounding":
            return out
        d = [rows[i] - sum(out[i]) for i in range(n_r)]
        e = [cols[j] - sum(out[i][j] for i in range(n_r)) for j in range(n_c)]
        assert all(v >= 0 for v in d) and all(v >= 0 for v in e), (
            "the fit's residual exceeds a unit"
        )
        assert sum(d) == sum(e), "the two marginals differ"
        flow = Flow(n_r, n_c, d, e)
        positive = {(i, j) for i in range(n_r) for j in range(n_c) if x[i][j] != int(x[i][j])}
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
# manor: 400 persons, no cast, one polity with two dynasties.
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
    TFR_DEFAULT,
    MULT_DEFAULT,
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
    "4.9",
    "1.2",
    (Polity("manor_of_ash", "primogeniture", (Dynasty("ashcombe", 3), Dynasty("thorne", 2))),),
)
CASES: tuple[Manifest, ...] = (
    HAMLET,
    Manifest(**{**HAMLET.__dict__, "seed": 7}),
    MANOR,
    Manifest(**{**MANOR.__dict__, "seed": 2}),
)


def build(m: Manifest, control: str | None = None) -> Genesis:
    """B then P: the dynasties first (their living members count against the population)."""
    g = Genesis(m, control)
    g.backfill()
    g.synthesise()
    return g


def with_law(m: Manifest, law: str) -> Manifest:
    """Return the manifest with every polity's succession law replaced."""
    pols = tuple(Polity(p.id, law, p.dynasties) for p in m.polities)
    return Manifest(**{**m.__dict__, "polities": pols})


# ----------------------------------------------------------------------------- invariants
class Checker:
    """Check 2: the spec's invariants on one built world."""

    def __init__(self, g: Genesis) -> None:
        """Collect failures for ``g``."""
        self.g = g
        self.f: list[str] = []
        self.y = g.cal.year

    def fail(self, msg: str) -> None:
        """Record a failure."""
        self.f.append(f"{self.g.m.id} seed {self.g.seed}: {msg}")

    def run(self) -> list[str]:
        """Run every check and return the failures."""
        self.ids()
        self.marginals()
        for p in self.g.persons.values():
            self.person(p)
            self.parents(p)
            self.siblings(p)
        self.population()
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
        if sum(r.agesex) != r.synthesised:
            self.fail("the age-sex cells do not sum to the synthesised count")
        if any(abs(Fraction(c) - t) >= 1 for c, t in zip(r.agesex, r.agesex_exact, strict=True)):
            self.fail("an age-sex cell is a unit or more from its exact value")
        if [sum(row) for row in r.ipf] != r.women_by_band:
            self.fail("the couple table's row sums are not the married women per band")
        if [
            sum(r.ipf[i][j] for i in range(WIDE_BANDS)) for j in range(WIDE_BANDS)
        ] != r.men_by_band:
            self.fail("the couple table's column sums are not the married men per band")
        cells = (
            (r.ipf[i][j], r.ipf_exact[i][j]) for i in range(WIDE_BANDS) for j in range(WIDE_BANDS)
        )
        if any(abs(Fraction(a) - b) > 1 for a, b in cells):
            self.fail("a couple-table cell is more than a unit from its fitted value")
        if sum(r.women_by_age) != r.married or sum(r.men_by_band) != r.married:
            self.fail("the married totals differ")

    def person(self, p: Person) -> None:
        """A living person has a household with a living head and a settlement; the dead, before."""
        g = self.g
        if p.alive:
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
        elif p.death is not None and (p.death >= 0 or p.birth >= 0):
            self.fail("a closed row with a birth or death at or after the epoch")
        if p.generation >= 0 and p.birth >= 0:
            self.fail("a back-filled birth at or after the epoch")
        if p.spouse:
            s = g.persons[p.spouse]
            if s.spouse != p.id or s.sex == p.sex or not (s.alive and p.alive):
                self.fail(f"spouse columns of {p.id} are not symmetric between two living persons")

    def parents(self, p: Person) -> None:
        """Parents are of the right sex, inside the fertility window and alive at the birth."""
        if not p.mother:
            return
        g, y = self.g, self.y
        mo, fa = g.persons[p.mother], g.persons[p.father]
        if mo.sex != SEX_FEMALE or fa.sex != SEX_MALE:
            self.fail("parents' sexes")
        if not MIN_AGE_F * y <= p.birth - mo.birth < FERTILE_END * y:
            self.fail(f"mother of {p.id} outside the fertility window")
        if p.birth - fa.birth < MIN_AGE_M * y:
            self.fail(f"father of {p.id} under {MIN_AGE_M}")
        if mo.death is not None and mo.death < p.birth:
            self.fail(f"child {p.id} born after its mother's death")
        conceived = g.cal.month_start(p.conception) if p.conception is not None else None
        if conceived is not None and fa.death is not None and fa.death <= conceived:
            self.fail(f"child {p.id} conceived after its father's death")

    def siblings(self, mo: Person) -> None:
        """One mother's children are conceived a closed period apart, born on distinct ticks."""
        ps = self.g.persons
        pulses = sorted(c for c in (ps[k].conception for k in mo.children) if c is not None)
        if any(b - a <= CLOSED for a, b in itertools.pairwise(pulses)):
            self.fail(f"two conceptions of {mo.id} inside the closed period")
        births = [ps[c].birth for c in mo.children]
        if len(set(births)) != len(births):
            self.fail(f"two children of {mo.id} share a birth tick")

    def population(self) -> None:
        """The living count at tick 0 is the manifest's population."""
        living = sum(1 for p in self.g.persons.values() if p.alive)
        if living + self.g.m.manifest_founders != self.g.m.population:
            self.fail(f"living count {living} plus the manifest founders is not the population")

    def titles(self) -> None:
        """B9, B10: contiguous tenures held by living house males, ending open at genesis."""
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

    def tenure(self, t: Title, r: Tenure, house: House) -> None:
        """One tenure: non-empty, a house male, alive throughout."""
        h = self.g.persons[r.holder]
        end = r.end if r.end != I64_MAX else 0
        if r.start >= end or h.sex != SEX_MALE or r.holder not in house.members:
            self.fail(f"title {t.k}: a tenure is empty or held by a non-member")
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


def check_constants(g: Genesis) -> list[str]:
    """Check 4: the births spec's check values and the spec's widths."""
    f = []
    default = Fertility(q16(TFR_DEFAULT), q16(MULT_DEFAULT))
    if g.fert.tfr1_bits != CHECK_TFR1_BITS:
        f.append(f"TFR1 bits {g.fert.tfr1_bits} != {CHECK_TFR1_BITS}")
    if default.level_bits != CHECK_LEVEL_BITS:
        f.append("level at the default tfr differs from the births spec's check value")
    if tuple(default.conception) != CHECK_CONCEPTION:
        f.append("conception table at the default differs from the births spec's check values")
    if g.fert.sex_male_bits != CHECK_SEX_MALE_BITS:
        f.append("SEX_M bits")
    if (MAX_POPULATION * IPF_SCALE) * MAX_POPULATION * IPF_SCALE >= 1 << 128:
        f.append("an IPF product does not fit u128")
    oldest = -(6 * (FATHER_AGE[1] + 1) + YOUNGEST_YEARS + 1) * max(MONTHS_365) * MONTHS * DAY
    if oldest < -(1 << 63):
        f.append("a root's birth tick does not fit i64")
    return f


# ----------------------------------------------------------------------------- rendering
def sha(lines: list[str]) -> str:
    """Return the SHA-256 of lines joined by newlines."""
    return hashlib.sha256("\n".join(lines).encode("ascii")).hexdigest()


def person_row(g: Genesis, p: Person) -> str:
    """Return one person's canonical row."""
    death = p.death if p.death is not None and p.death < 0 else I64_MAX
    hh = g.households.get(p.household)
    residence = hh.residence if p.alive and hh is not None else 0
    return (
        f"{p.id} {p.sex} {p.birth} {death} {p.mother} {p.father} {p.spouse if p.alive else 0} "
        f"{p.household if p.alive else 0} {residence} {p.house} {p.generation}"
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
        f"predecessor_exit {t.predecessor_exit}",
        *[f"tenure {t.k} {r.holder} {r.start} {r.end}" for r in t.tenures],
    ]


def render_case(g: Genesis) -> list[str]:
    """Return the golden lines of one case."""
    r = g.report
    persons = [person_row(g, p) for p in sorted(g.persons.values(), key=lambda q: q.id)]
    marriages = sorted(f"{m.husband} {m.wife} {m.start} {m.end} {m.source}" for m in g.marriages)
    households = [
        f"{h.id} {h.head} {h.formed} {h.dissolved} {h.residence}"
        for h in sorted(g.households.values(), key=lambda h: h.id)
    ]

    def ints(values: list[int]) -> str:
        return " ".join(map(str, values))

    lines = [
        f"case {g.m.id} {g.seed}",
        f"population {g.m.population} manifest_founders {g.m.manifest_founders} "
        f"living_dynasts {r.living_dynasts} synthesised {r.synthesised} persons {len(g.persons)}",
        f"schedule e0 {q16(g.m.e0)} K {g.schedule.k} tfr {q16(g.m.tfr)} level {g.fert.level_bits} "
        f"elite {g.fert.elite_bits}",
        f"agesex males {sum(r.agesex[0::2])} females {sum(r.agesex[1::2])} "
        f"sha256 {sha([ints(r.agesex)])}",
        f"married {r.married} women_by_band {ints(r.women_by_band)} "
        f"men_by_band {ints(r.men_by_band)}",
        *[f"ipf {i} {ints(row)}" for i, row in enumerate(r.ipf)],
        *[f"children {c} {ints(row)}" for c, row in enumerate(r.children_alloc)],
        f"leftover {r.leftover} orphans {r.orphans}",
        f"households {len(households)} marriages {len(marriages)}",
    ]
    for h in g.houses:
        lines += render_house(g, h)
    return [
        *lines,
        f"persons_sha256 {sha(persons)}",
        f"marriages_sha256 {sha(marriages)}",
        f"households_sha256 {sha(households)}",
    ]


HEADER = [
    "# population synthesis v0 golden outputs (SPR-019): GENERATED by",
    "# python -I specs/worldgen/selftest_population_synthesis.py --write; do not edit.",
    "# Each case: the manifest and seed; counts; the schedule's compiled constants; the age-sex",
    "# cells' digest; the married marginals; the couple table (rows: wife band, columns: husband",
    "# band, 15-19 to 80+); the children allocation (rows: child band 0-4, 5-9, 10-14; columns:",
    "# mother band); each house and title with its tenure log (holder, start, end; i64::MAX while",
    "# held); digests of the person, marriage and household rows (sorted by id).",
    "constants population-synthesis-v0",
]


def render(control: str | None) -> tuple[str, list[str]]:
    """Build every case; return the golden text and the failures of checks 2 and 4."""
    lines = HEADER[:]
    failures: list[str] = []
    for m in CASES:
        g = build(m, control)
        failures += Checker(g).run() + check_constants(g)
        lines += render_case(g)
    return "\n".join(lines) + "\n", failures


def check_laws(control: str | None) -> list[str]:
    """Check 3: the manor's back-fill under each law keeps the invariants."""
    failures = []
    for law in LAWS:
        g = build(with_law(MANOR, law), control)
        failures += [f"law {law}: {x}" for x in Checker(g).run()]
    return failures


def first_difference(current: str, text: str) -> int:
    """Return the 1-based line of the first difference between two texts."""
    a, b = current.splitlines(), text.splitlines()
    pairs = enumerate(zip(a, b, strict=False))
    return next((i for i, (x, z) in pairs if x != z), min(len(a), len(b))) + 1


def main(argv: list[str] | None = None) -> int:
    """Check or regenerate the golden file; run the reference's own checks."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--write", action="store_true", help="regenerate the golden file")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the golden file to check")
    parser.add_argument("--control", choices=("spacing", "rounding"), help="plant a violation")
    args = parser.parse_args(argv)
    try:
        text, failures = render(args.control)
        failures += check_laws(args.control)
    except AssertionError as e:
        failures, text = [f"construction error: {e}"], ""
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
    current = args.golden.read_text(encoding="utf-8") if args.golden.exists() else ""
    if current != text:
        line = first_difference(current, text)
        print(f"STALE: {args.golden} differs from the reference at line {line}")
        print("POPULATION SYNTHESIS: FAIL")
        return EXIT_FAIL
    print(f"POPULATION SYNTHESIS: PASS ({len(CASES)} cases, {text.count(chr(10))} lines)")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
