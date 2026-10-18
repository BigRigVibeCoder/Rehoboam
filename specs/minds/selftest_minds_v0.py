"""Exact reference and self-test of minds v0, part A (the T1-A spec PR; T1 0.4.0, A0 to A16).

The algorithm is stated in the T1 spec (``specs/systems/next/T1.minds.odd.toml`` while it is a
draft; ``specs/systems/T1.minds.odd.toml`` once SPR-023 T-01 promotes it), section
[algorithm.minds_f1], steps A0 to A16: the mind table and its key ``mind.person``, the derived
indexes and their freshness inside a binding (A2), the trait draw (A3), world start by the
Architect's ruling X4 (A5), Promote and Demote (A6, A7), the instant's exits from the engine's
instant log (A8, ruling X1), liege and vassal rows refreshed monthly (A9), opinion modifiers and
their sum (A10, A11), thoughts and goals (A12) and the Registry tables the A-part reads (A14). This
file implements them with Python integers only (no float anywhere), on the keyed RNG of
``bin/gen_rng_golden.py`` (the independent rh-ids reference, reused, never re-implemented), so it
reproduces the engine's integers bit for bit. The I-steps (decisions) and S-steps (stress, grief,
breaks) are T1-I/S's: where an A-step meets them (a Demote during a break, the timers a Demote
cancels, the opinion reads of ``decide`` in the binding that appended a modifier), the scripted
worlds stand in for them with scripted writes, timers and probes, each named as a stand-in. It
checks:

  1. the spec's ``check_values`` and ``constants`` against this file's arithmetic and the Registry
     (codes, q8_8 values, the trait draw's SIGMA, the month's length on both calendars);
  2. the widths the steps state (the trait product below 2^47, the thought sum needing i32);
  3. the step prose states the numbers this file computes (a reviewer reads the steps, the engine
     compiles the constants: they must not drift apart);
  4. the trait draw: its draws equal ``gen_rng_golden.h`` key for key, its moments over 1,024
     persons are trait_sd's, and a clamp witness at the Registry's largest trait_sd;
  5. the Registry typing of A14 and its refusals, on planted copies of the three tables;
  6. six scripted worlds through world start, promotion, demotion, re-promotion, the deferred
     Demote, timers a Demote cancels and keeps, exits, the monthly relation refresh on both
     calendars, opinion modifiers, their sum and expiry, index freshness, thoughts and goals, with
     every invariant the A-steps state observed at every instant's Bookkeeping;
  7. the golden file ``specs/minds/golden/minds_v0.txt`` equals this reference's output.

Run: python -I specs/minds/selftest_minds_v0.py --check        (check; the default)
     python -I specs/minds/selftest_minds_v0.py --write        (regenerate the golden file)
     python -I specs/minds/selftest_minds_v0.py --golden PATH  (check another golden file)
     python -I specs/minds/selftest_minds_v0.py --spec PATH    (another spec: a stale-check control)
     python -I specs/minds/selftest_minds_v0.py --control NAME (a planted mutant with the observers
                                                                on: must FAIL for its own reason)
     python -I specs/minds/selftest_minds_v0.py --mutant NAME  (a planted mutant rendered against
                                                                the golden: must FAIL naming the
                                                                golden lines it changes)
Exit: 0 pass, 1 fail. Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import bisect
import difflib
import sys
import tomllib
from collections import Counter
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from decimal import Decimal
from fractions import Fraction
from math import isqrt
from pathlib import Path
from typing import Any, TypeGuard

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "bin"))

import gen_rng_golden as rng  # noqa: E402  (the independent rh-ids reference; SPR-002)

NEXT_SPEC = ROOT / "specs" / "systems" / "next" / "T1.minds.odd.toml"
MAIN_SPEC = ROOT / "specs" / "systems" / "T1.minds.odd.toml"
REGISTRY = ROOT / "specs" / "manifest" / "registry_v0.toml"
GOLDEN = HERE / "golden" / "minds_v0.txt"
TABLE = "minds_f1"
EXIT_OK, EXIT_FAIL = 0, 1

MASK64 = (1 << 64) - 1
NONE_EVENT = MASK64  # an absent EventRef in a u64 column: 05.1 §2 rule 2's -1
I64_MIN = -(1 << 63)
I16_MAX = (1 << 15) - 1
I32_MAX = (1 << 31) - 1
WIDTH_TRAIT_BITS = 47  # A3: |256 x B x c_k| < 2^47
Q8 = 256  # q8_8 raw units per point
Q16 = 1 << 16  # Q16_16 bits per unit
THOUGHT_GAIN = 100  # A12: trait points per overlay point
DELTA_MAX = Q8  # A12: an overlay delta is clamped to [0, 1] overlay point
HEXACO = (
    "honesty_humility",
    "emotionality",
    "extraversion",
    "agreeableness",
    "conscientiousness",
    "openness",
)
LIEGE, VASSAL = "liege", "vassal"
KIN_KINDS = ("parent", "child", "spouse", "sibling", "grandparent", "grandchild", "kin_other")
BREAK_STATES = ("calm", "pending", "refractory")
CALM = 0
CANCELLED_KINDS = frozenset({"poll", "interrupt", "break", "refractory_end"})  # A7 (a)
OWED_KINDS = frozenset({"DEATH_DUE", "MARRY_DUE"})  # A7: never cancelled by a Demote
PHASES = (
    "Inputs",
    "Environment",
    "Production",
    "Exchange",
    "Vital",
    "Activity",
    "Contagion",
    "Polity",
    "Military",
    "Agenda",
    "Culture",
    "Bookkeeping",
)
VITAL, POLITY, AGENDA = PHASES.index("Vital"), PHASES.index("Polity"), PHASES.index("Agenda")
MONTH_DAYS_365 = (31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)
CALENDARS = (360, 365)  # 05.1 §4.1
# The scripted parameter values, inside the Registry bounds (T1 has no Registry default in P1a;
# A0: a built T1 missing one is refused by name). Exact decimals, compiled to Q16_16 as
# rh_manifest::compile does (rounded half to even; SPR-011 ruling 9).
PARAMS = {"trait_sd": Decimal("20"), "thought_cap": Decimal("30"), "thought_decay": Decimal("4.5")}
# Check 4: the trait draw's moments over this many persons, within this relative band (the
# standard error of a variance over 7,168 draws is about 1.7%, so 6% is about 3.5 of them).
MOMENT_PERSONS = 1024
MOMENT_TOL = Fraction(6, 100)
MOMENT_SEED = 7
CLAMP_SD = Decimal("50")  # check 4's clamp witness: the Registry's largest trait_sd
CROSS_PERSONS = 4
# Every planted mutant (the minds plan's mutants and the T1-A controls) and the observer whose
# failure is its own reason (REV-WAVE6 X4, V2).
MUTANTS = {
    "redraw-on-repromote": "INV-T1-05 (immutable)",
    "demote-cancels-death-due": "A7 (owed outcome)",
    "stale-index": "A2 (index freshness)",
    "trait-key-wrong-t": "INV-T1-05 (key)",
    "world-start-genesis-names": "INV-T1-11 (genesis)",
    "trait-k-swap": "INV-T1-05 (key)",
    "opinion-no-expiry": "INV-T1-06",
    "relation-not-ended": "INV-T1-12 (refresh)",
}
ALIASES = {"repromote-redraw": "redraw-on-repromote"}  # the minds plan's name (SPR-023 T-03)


# ----------------------------------------------------------------------------- arithmetic (A0)
def rhe(n: int, d: int) -> int:
    """A0: n / d rounded half to even, for an integer n and d > 0."""
    if d <= 0:
        raise ValueError("rhe needs d > 0")
    q, r = divmod(n, d)
    twice = 2 * r
    if twice > d or (twice == d and q % 2 == 1):
        return q + 1
    return q


def fixed(value: Decimal | int, scale: int) -> int:
    """An exact decimal at ``scale`` units per point, rounded half to even."""
    f = Fraction(value) * scale
    return rhe(f.numerator, f.denominator)


def narrow(bits: int) -> int:
    """A0: a Q16_16 parameter to q8_8, rhe(b / 256)."""
    return rhe(bits, Q16 // Q8)


def clamp(x: int, lo: int, hi: int) -> int:
    """``x`` clamped to [lo, hi]."""
    return max(lo, min(hi, x))


def commas(n: int) -> str:
    """An integer with thousands separators, as the step prose writes it."""
    return f"{n:,}"


# ----------------------------------------------------------------------------- spec and Registry
def load_toml(path: Path) -> dict[str, Any]:
    """A TOML file with every decimal kept exact."""
    return tomllib.loads(path.read_text(encoding="utf-8"), parse_float=Decimal)


def default_spec() -> Path:
    """The next-version draft while it holds the table, else the promoted spec (the fallback)."""
    if NEXT_SPEC.exists() and TABLE in load_toml(NEXT_SPEC).get("algorithm", {}):
        return NEXT_SPEC
    return MAIN_SPEC


@dataclass(frozen=True)
class Spec:
    """The parts of the T1 spec this reference reads."""

    constants: dict[str, Any]
    check: dict[str, Any]
    steps: dict[str, str]
    version: str
    applies: dict[str, tuple[str, str]]  # command -> (key, key_field)

    @staticmethod
    def load(path: Path) -> Spec:
        """Read ``[algorithm.minds_f1]`` and the ``[[applies]]`` keys of ``path``."""
        doc = load_toml(path)
        alg = doc["algorithm"][TABLE]
        steps = {str(s).split(None, 1)[0]: str(s) for s in alg["steps"]}
        applies = {str(a["command"]): (str(a["key"]), str(a["key_field"])) for a in doc["applies"]}
        return Spec(
            dict(alg["constants"]), dict(alg["check_values"]), steps, str(alg["version"]), applies
        )


@dataclass(frozen=True)
class Consts:
    """``constants`` of the algorithm table, in the units the steps use."""

    trait_k: int
    fields: int
    field_bits: int
    trait_lo: int
    trait_hi: int
    amb_lo: int
    amb_hi: int
    amb_mean: int
    months: int
    day: int

    @staticmethod
    def of(spec: Spec) -> Consts:
        """Compile the constants (points to q8_8 raw)."""
        c = spec.constants
        lo, hi = (int(x) * Q8 for x in c["trait_scale"])
        alo, ahi = (int(x) * Q8 for x in c["ambition_scale"])
        return Consts(
            int(c["trait_k"]),
            int(c["trait_fields"]),
            int(c["trait_field_bits"]),
            lo,
            hi,
            alo,
            ahi,
            int(c["ambition_mean"]) * Q8,
            int(c["months_per_year"]),
            int(c["minutes_per_day"]),
        )

    @property
    def field_mask(self) -> int:
        """One field of a draw."""
        return (1 << self.field_bits) - 1

    @property
    def field_mean(self) -> int:
        """The mean of a field sum, an integer: fields x (2^bits - 1) / 2."""
        return self.fields * self.field_mask // 2

    def month(self, days_per_year: int) -> int:
        """A0: MONTH = Y / 12 minutes (05.1 §4.4's twelfth of a year)."""
        return self.day * days_per_year // self.months


def sigma_q16() -> tuple[int, str]:
    """A3: SIGMA, the integer nearest 2^16 sd(S), and that value to four decimals.

    sd(S)^2 = (2^32 - 1) / 3 for the sum of four uniform 16-bit fields, so (2^16 sd)^2 =
    (2^64 - 2^32) / 3, an integer; isqrt and its remainder give the nearest integer exactly.
    """
    n = ((1 << 64) - (1 << 32)) // 3
    sigma, four = nearest_root(n), nearest_root(n * 10**8)
    return sigma, f"{four // 10**4}.{four % 10**4:04d}"


def nearest_root(n: int) -> int:
    """The integer nearest sqrt(n), exactly (no tie: 4n is never an odd square)."""
    root = isqrt(n)
    return root + 1 if 4 * n > (2 * root + 1) ** 2 else root


SIGMA, SIGMA_TEXT = sigma_q16()


@dataclass(frozen=True)
class Reg:
    """The Registry tables the A-part reads, typed by A14."""

    relation_codes: tuple[str, ...]
    closeness: dict[str, int]
    opinion_codes: tuple[str, ...]
    opinion_mag: dict[str, int]
    opinion_months: dict[str, int]
    overlay_codes: tuple[str, ...]
    overlay_traits: dict[str, dict[str, int]]
    vow_codes: tuple[str, ...]
    bounds: dict[str, tuple[Decimal, Decimal]]


def _number(v: object) -> TypeGuard[int | Decimal]:
    return isinstance(v, (int, Decimal)) and not isinstance(v, bool)


def _fields(table: str, key: str, v: object, want: set[str]) -> list[str]:
    if not isinstance(v, dict):
        return [f"[{table}] {key}: not a table"]
    errs = [f"[{table}] {key}: unknown field {f}" for f in sorted(set(v) - want)]
    return errs + [f"[{table}] {key}: missing field {f}" for f in sorted(want - set(v))]


def type_relation_kinds(t: dict[str, Any]) -> tuple[dict[str, int], list[str]]:
    """A14 [relation_kinds]: {closeness in [0, 1]}; liege, vassal and the kinship ids present."""
    errs: list[str] = []
    out: dict[str, int] = {}
    for k in sorted(t):
        errs += _fields("relation_kinds", k, t[k], {"closeness"})
        c = t[k].get("closeness") if isinstance(t[k], dict) else None
        if not (_number(c) and 0 <= c <= 1):
            errs.append(f"[relation_kinds] {k}: closeness {c} outside [0, 1]")
        else:
            out[k] = fixed(c, Q8)
    errs += [f"[relation_kinds] missing {k}" for k in (LIEGE, VASSAL, *KIN_KINDS) if k not in t]
    return out, errs


def type_opinion_modifiers(t: dict[str, Any]) -> tuple[dict[str, tuple[int, int]], list[str]]:
    """A14 [opinion_modifiers]: {magnitude in [-100, 100], months an integer in [1, 65535]}."""
    errs: list[str] = []
    out: dict[str, tuple[int, int]] = {}
    for k in sorted(t):
        errs += _fields("opinion_modifiers", k, t[k], {"magnitude", "months"})
        v = t[k] if isinstance(t[k], dict) else {}
        m, months = v.get("magnitude"), v.get("months")
        if not (_number(m) and -100 <= m <= 100):  # noqa: PLR2004  (the A14 bounds)
            errs.append(f"[opinion_modifiers] {k}: magnitude {m} outside [-100, 100]")
        elif not (isinstance(months, int) and not isinstance(months, bool)):
            errs.append(f"[opinion_modifiers] {k}: months {months} is not an integer")
        elif not 1 <= months <= (1 << 16) - 1:
            errs.append(f"[opinion_modifiers] {k}: months {months} outside [1, 65535]")
        else:
            out[k] = (fixed(m, Q8), months)
    return out, errs


def type_overlay_traits(
    t: dict[str, Any], overlays: Sequence[str]
) -> tuple[dict[str, dict[str, int]], list[str]]:
    """A14 [overlay_traits]: an [overlays] id -> {HEXACO trait: a decimal in [-100, 100]}."""
    errs: list[str] = []
    out: dict[str, dict[str, int]] = {}
    for k in sorted(t):
        if k not in overlays:
            errs.append(f"[overlay_traits] {k} is not an [overlays] id")
            continue
        row = t[k] if isinstance(t[k], dict) else {}
        for trait in sorted(row):
            v = row[trait]
            if trait not in HEXACO:
                errs.append(f"[overlay_traits] {k}: {trait} is not a HEXACO trait")
            elif not (_number(v) and -100 <= v <= 100):  # noqa: PLR2004  (the A14 bounds)
                errs.append(f"[overlay_traits] {k}.{trait}: {v} outside [-100, 100]")
            else:
                out.setdefault(k, {})[trait] = fixed(v, Q8)
    return out, errs


def load_registry(doc: dict[str, Any]) -> tuple[Reg, list[str]]:
    """Type the three tables and read the codes and the T1 parameter bounds."""
    rel, e1 = type_relation_kinds(doc["relation_kinds"])
    ops, e2 = type_opinion_modifiers(doc["opinion_modifiers"])
    overlays = tuple(str(i) for i in doc["overlays"]["ids"])
    traits, e3 = type_overlay_traits(doc["overlay_traits"], overlays)
    bounds = {
        str(n): (Decimal(lo), Decimal(hi))
        for n, (lo, hi) in doc["systems"]["T1.minds"]["params"].items()
    }
    reg = Reg(
        tuple(sorted(rel)),
        rel,
        tuple(sorted(ops)),
        {k: v[0] for k, v in ops.items()},
        {k: v[1] for k, v in ops.items()},
        overlays,
        traits,
        tuple(sorted(doc["vow_kinds"])),
        bounds,
    )
    return reg, e1 + e2 + e3


# ----------------------------------------------------------------------------- the trait draw (A3)
SALT = rng.branch_salt([], 0)  # the canonical timeline: no fork steps
SYSTEM_T1 = rng.system_id("T1.minds")
PURPOSE_TRAIT = rng.purpose_id("trait_draw")
DRAW_BYTES = 8


def rng_key(seed: int, person: int, epoch: int, t: int) -> rng.RngKey:
    """StepCtx::rng(trait_draw, person, k)'s key at instant t: every field of H but k."""
    return (seed, SALT, SYSTEM_T1, PURPOSE_TRAIT, person, epoch, t)


def raw_draws(k: Consts, seed: int, person: int, epoch: int, t: int) -> list[int]:
    """d_0 .. d_6: words 2k and 2k + 1 of the stream's block 0 (gen_rng_golden.stream_u64)."""
    block = rng.chacha_block(rng.stream_key(rng_key(seed, person, epoch, t)), 0, t)
    return [
        int.from_bytes(block[DRAW_BYTES * i : DRAW_BYTES * (i + 1)], "little")
        for i in range(k.trait_k)
    ]


def field_sum(k: Consts, d: int) -> int:
    """S_k: the sum of the draw's four 16-bit fields."""
    return sum((d >> (k.field_bits * i)) & k.field_mask for i in range(k.fields))


def scaled(k: Consts, d: int, sd_bits: int) -> int:
    """v_k = rhe(256 x B x c_k / SIGMA), in q8_8 raw units, before the clamp."""
    return rhe(Q8 * sd_bits * (field_sum(k, d) - k.field_mean), SIGMA)


def draw_mind(
    k: Consts, seed: int, person: int, epoch: int, t: int
) -> tuple[tuple[int, ...], int, list[int]]:
    """A3: the six traits and ambition of a mind created at t, and the field sums."""
    draws = raw_draws(k, seed, person, epoch, t)
    sd = fixed(PARAMS["trait_sd"], Q16)
    return draw_values(k, draws, sd)


def draw_values(
    k: Consts, draws: Sequence[int], sd_bits: int
) -> tuple[tuple[int, ...], int, list[int]]:
    """The clamped values of seven draws at ``sd_bits``: one rounding, then the clamp."""
    v = [scaled(k, d, sd_bits) for d in draws]
    traits = tuple(clamp(x, k.trait_lo, k.trait_hi) for x in v[: len(HEXACO)])
    ambition = clamp(k.amb_mean + v[len(HEXACO)], k.amb_lo, k.amb_hi)
    return traits, ambition, [field_sum(k, d) for d in draws]


# ----------------------------------------------------------------------------- the calendar (A0)
@dataclass(frozen=True)
class Calendar:
    """05.1 §4.1's calendar: 360 or 365 days, 12 months."""

    days: int
    k: Consts

    @property
    def year(self) -> int:
        """Y in minutes."""
        return self.k.day * self.days

    @property
    def month(self) -> int:
        """MONTH = Y / 12 (A0): the unit of an opinion's months, nothing else."""
        return self.k.month(self.days)

    def month_starts(self) -> list[int]:
        """The days of the year on which a calendar month starts."""
        lengths = (self.days // self.k.months,) * self.k.months
        if self.days != CALENDARS[0]:
            lengths = MONTH_DAYS_365
        return [sum(lengths[:m]) for m in range(self.k.months)]

    def pulses(self, horizon: int) -> list[int]:
        """Every Month pulse instant t_m in [0, horizon]."""
        out, year = [], 0
        while year * self.year <= horizon:
            out += [year * self.year + d * self.k.day for d in self.month_starts()]
            year += 1
        return [t for t in out if t <= horizon]


# ----------------------------------------------------------------------------- the model
@dataclass
class Mind:
    """A1: one row of the mind.* columns."""

    person: int
    traits: tuple[int, ...]
    ambition: int
    stress: int = 0
    stress_in: int = 0
    stress_decayed: int = 0
    stress_relieved: int = 0
    break_state: int = CALM
    break_timer: int = 0
    refractory_until: int = 0
    interrupt_day: int = 0
    interrupts_today: int = 0


@dataclass
class Relation:
    """A9: a liege or vassal row."""

    a: int
    b: int
    kind: str
    ended_at: int = 0


@dataclass
class Opinion:
    """A10: an opinion modifier row."""

    holder: int
    about: int
    kind: str
    magnitude: int
    source_event: int
    expires: int


@dataclass
class Thought:
    """A12: a thought row, one per (holder, overlay)."""

    holder: int
    overlay: int
    magnitude: int
    source_event: int
    refreshed_at: int


@dataclass
class Goal:
    """A12: a goal row, one per (holder, vow)."""

    holder: int
    vow: int
    kind: int
    target: int
    priority: int


@dataclass
class Person:
    """A ledger row as T1 reads it: M2's alive, T1's named, the entity's epoch."""

    alive: bool = True
    named: int = 0
    epoch: int = 0


@dataclass(frozen=True)
class Command:
    """A queued lifecycle command for T1, keyed (tick, phase, PersonId, seq)."""

    tick: int
    phase: int
    person: int
    seq: int
    kind: str
    fields: tuple[tuple[str, int], ...]
    cause: int = NONE_EVENT

    def get(self, name: str) -> int:
        """A payload field."""
        return dict(self.fields)[name]

    @property
    def key(self) -> tuple[int, int, int, int]:
        """The inbox order (05.1 §2 rule 2)."""
        return (self.tick, self.phase, self.person, self.seq)


@dataclass
class Timer:
    """A pending T1 timer (a stand-in for one the I- and S-steps schedule)."""

    tid: int
    kind: str
    entity: int
    due: int
    phase: int
    payload: int


class Index[R]:
    """A2: a derived index, key -> rows in ascending row order, with its covered length."""

    def __init__(self, name: str, key: Callable[[R], tuple[int, ...]]) -> None:
        """An empty index named ``name`` over rows keyed by ``key``."""
        self.name = name
        self.key = key
        self.rows: dict[tuple[int, ...], list[int]] = {}
        self.covered = 0

    def hook(self, table: Sequence[R]) -> None:
        """rh-core's post-merge hook: add the rows [covered, len)."""
        for i in range(self.covered, len(table)):
            self.rows.setdefault(self.key(table[i]), []).append(i)
        self.covered = len(table)

    def rebuild(self, table: Sequence[R]) -> dict[tuple[int, ...], list[int]]:
        """The index rebuilt from its table (INV-T1-14)."""
        out: dict[tuple[int, ...], list[int]] = {}
        for i, row in enumerate(table):
            out.setdefault(self.key(row), []).append(i)
        return out


@dataclass(frozen=True)
class Op:
    """One scripted act of a case, on a day start: see ``Sim`` for the verbs."""

    day: int
    verb: str
    ints: tuple[int, ...] = ()
    name: str = ""


@dataclass(frozen=True)
class Case:
    """A scripted world: its persons, cast, M7's genesis titles and offices, its script."""

    name: str
    note: str
    days: int
    seed: int
    persons: tuple[int, ...]
    cast: tuple[int, ...]
    criteria0: tuple[int, ...]  # whom M7's dormancy pass at tick 0 finds meeting a criterion
    titles: tuple[tuple[int, int, int], ...]  # (TitleId, holder, liege TitleId)
    offices: tuple[tuple[int, int, int], ...]  # (OfficeId, TitleId, holder)
    script: tuple[Op, ...]
    horizon_day: int


@dataclass
class Config:
    """A run of the reference: a planted mutant, and whether the observers run."""

    mutant: str = ""
    observe: bool = True


SENDS_VITAL = {  # stub senders' commands (S6, S7 at their Day/Vital binding)
    "s7_opinion": "OpinionModifier",
    "thought": "ApplyThought",
    "influence": "ApplyInfluence",
    "goal": "AgendaGoal",
    "clear_goal": "ClearGoal",
}
SENDS_POLITY = {"promote": "Promote", "demote": "Demote", "m7_opinion": "OpinionModifier"}
APPLY_KINDS = frozenset(SENDS_VITAL.values())
PROMOTE_KINDS = frozenset({"Promote", "Demote"})


class Sim:
    """One scripted world through the A-steps, with the observers.

    Verbs: ``die`` (M2 kills a person in Vital), ``epoch`` (a material input in a person's scope
    at Inputs), ``title`` and ``office`` (M7 writes at Polity, read lagged by T1), ``promote``,
    ``demote`` and ``m7_opinion`` (M7 sends at Polity), ``dormancy0`` (M7's tick-0 pass: Promote
    each person of the case's criteria set whose lagged person.named is 0), ``s7_opinion``,
    ``thought``, ``influence``, ``goal`` and ``clear_goal`` (the stub senders, at Vital), and the
    stand-ins for T1-I/S at the end of Day/Agenda: ``break`` (a break state the S-steps would
    write), ``timer`` (a timer the I- or S-steps would schedule) and ``probe`` (an opinion read
    of ``decide``, in the binding that applied the instant's modifiers).
    """

    def __init__(self, case: Case, k: Consts, reg: Reg, spec: Spec, cfg: Config) -> None:
        """A world at genesis (not yet run)."""
        self.case, self.k, self.reg, self.spec, self.cfg = case, k, reg, spec, cfg
        self.cal = Calendar(case.days, k)
        self.ledger = {p: Person() for p in case.persons}
        self.titles = {tid: (h, liege) for tid, h, liege in case.titles}
        self.offices = {oid: (title, h) for oid, title, h in case.offices}
        self.minds: list[Mind] = []
        self.relations: list[Relation] = []
        self.opinions: list[Opinion] = []
        self.thoughts: list[Thought] = []
        self.goals: list[Goal] = []
        self.ix_person: Index[Mind] = Index("person -> mind", lambda m: (m.person,))
        self.ix_a: Index[Relation] = Index("relation.a", lambda r: (r.a,))
        self.ix_b: Index[Relation] = Index("relation.b", lambda r: (r.b,))
        self.ix_op: Index[Opinion] = Index("(holder, about)", lambda o: (o.holder, o.about))
        self.queue: list[Command] = []
        self.timers: list[Timer] = []
        self.seq = 0
        self.next_timer = 1
        self.dropped: Counter[tuple[str, str]] = Counter()
        self.cancelled: Counter[tuple[str, str]] = Counter()
        self.lines: list[str] = []
        self.failures: list[str] = []
        self.created: dict[int, tuple[int, int, tuple[int, ...], int]] = {}
        self.t = 0
        self.binding = ""
        self.step = ""
        self.cap = narrow(fixed(PARAMS["thought_cap"], Q16))
        self.decay = narrow(fixed(PARAMS["thought_decay"], Q16))
        self.pulses = self.cal.pulses(case.horizon_day * k.day)

    # ------------------------------------------------------------------ plumbing
    def mut(self, name: str) -> bool:
        """Whether ``name`` is the planted mutant."""
        return self.cfg.mutant == name

    def fail(self, msg: str) -> None:
        """Record an observer failure once."""
        line = f"{self.case.name}: {msg}"
        if line not in self.failures:
            self.failures.append(line)

    def trace(self, what: str) -> None:
        """One golden line of the instant's trace: the binding, the step, what it did."""
        day = self.t // self.k.day
        where = f"{self.binding} {self.step}".strip()
        self.lines.append(f"{self.case.name} t={self.t}/d{day} {where}: {what}")

    def at(self, binding: str, step: str) -> None:
        """Enter a binding's step (for the trace)."""
        self.binding, self.step = binding, step

    def look[R](self, ix: Index[R], table: Sequence[R], key: tuple[int, ...]) -> list[int]:
        """A2: the index's rows for ``key``, then the table's rows past the covered length."""
        rows = list(ix.rows.get(key, ()))
        if not self.mut("stale-index"):
            rows += [i for i in range(ix.covered, len(table)) if ix.key(table[i]) == key]
        if self.cfg.observe:
            full = [i for i, row in enumerate(table) if ix.key(row) == key]
            if rows != full:
                self.fail(
                    f"A2 (index freshness): {ix.name} lookup of {key} at t={self.t} in "
                    f"{self.binding} returns rows {rows}, the table holds {full} (covered to "
                    f"{ix.covered}: a row appended earlier in the binding is missed)"
                )
        return rows

    def hook(self) -> None:
        """The post-merge hook after a T1 binding's level; INV-T1-14 after it."""
        pairs: tuple[tuple[Index[Any], Sequence[Any]], ...] = (
            (self.ix_person, self.minds),
            (self.ix_a, self.relations),
            (self.ix_b, self.relations),
            (self.ix_op, self.opinions),
        )
        for ix, table in pairs:
            ix.hook(table)
            if self.cfg.observe and ix.rows != ix.rebuild(table):
                self.fail(f"INV-T1-14: {ix.name} differs from its rebuild at t={self.t}")

    def drop(self, c: Command, reason: str) -> None:
        """StepCtx::drop_command: counted under dropped{T1.minds, kind, reason}."""
        self.dropped[(c.kind, reason)] += 1
        self.trace(f"drop {c.kind} {c.person} {reason}")

    def send(self, phase: int, kind: str, fields: dict[str, int], cause: int = NONE_EVENT) -> None:
        """A command for T1, keyed by its [[applies]] key_field, its seq given at the merge."""
        person = fields[self.spec.applies[kind][1]]
        self.queue.append(
            Command(self.t, phase, person, self.seq, kind, tuple(fields.items()), cause)
        )
        self.seq += 1

    def mind_of(self, p: int) -> Mind | None:
        """The mind of ``p`` through the person-to-row index, or None."""
        rows = self.look(self.ix_person, self.minds, (p,))
        return self.minds[rows[0]] if rows else None

    # ------------------------------------------------------------------ the run
    def run(self) -> None:
        """Genesis, every instant of the script and the Month pulses, the final state."""
        self.genesis()
        days = {op.day for op in self.case.script} | {0, self.case.horizon_day}
        fixed_instants = sorted({d * self.k.day for d in days} | set(self.pulses))
        t = -1
        while True:
            end = fixed_instants[-1]
            due = [tm.due for tm in self.timers if tm.phase == VITAL and t < tm.due <= end]
            later = [x for x in fixed_instants if x > t] + due
            if not later:
                break
            t = min(later)
            self.instant(t)
        self.state_lines()

    def genesis(self) -> None:
        """A5: genesis names only the manifest's cast (ruling X4); INV-T1-11's genesis half."""
        named = set(self.case.cast)
        if self.mut("world-start-genesis-names"):
            named |= set(self.case.criteria0)  # the frozen text's second route
        for p in sorted(named):
            self.ledger[p].named = 1
        self.at("genesis", "")
        self.trace("names " + (" ".join(map(str, sorted(named))) or "none"))
        if self.cfg.observe:
            for p in sorted(named - set(self.case.cast)):
                self.fail(
                    f"INV-T1-11 (genesis): genesis named {p}, who is not in the manifest's cast "
                    "(Architect ruling X4: one naming route, M7's)"
                )

    def instant(self, t: int) -> None:
        """One day-start instant, phase by phase (05.1 §4.2)."""
        self.t = t
        lagged = (dict(self.titles), dict(self.offices))
        lagged_named = {p: x.named for p, x in self.ledger.items()}
        ops = [op for op in self.case.script if op.day * self.k.day == t]
        for op in ops:
            if op.verb == "epoch":
                self.ledger[op.ints[0]].epoch += 1
        exited = self.vital(ops)
        self.at("Day/Vital", "exits")
        if exited:
            self.exits(exited)
        self.hook()
        self.polity(ops, lagged_named)
        if t in self.pulses:
            self.at("Month/Agenda", "relations")
            self.relations_step(lagged)
            self.at("Month/Agenda", "expire")
            self.expire()
            self.hook()
        self.day_agenda(ops, lagged)
        self.hook()
        self.bookkeeping()

    def vital(self, ops: Sequence[Op]) -> list[int]:
        """Vital: T1's timer pass, M2's deaths, the stub senders; the instant's exits."""
        self.at("Vital", "T1's timer pass")
        before = {p for p, x in self.ledger.items() if x.alive}
        self.fire_owed()
        for op in ops:
            if op.verb == "die":
                self.ledger[op.ints[0]].alive = False
        for op in ops:
            if op.verb in SENDS_VITAL:
                self.stub_send(op)
        # the engine's instant log: persons whose alive went 1 -> 0, ascending (SPR-038 T5)
        return sorted(p for p in before if not self.ledger[p].alive)

    def fire_owed(self) -> None:
        """T1's Vital timer pass: due DEATH_DUE and MARRY_DUE (stand-ins of the I-steps')."""
        due = [tm for tm in self.timers if tm.due == self.t and tm.phase == VITAL]
        for tm in sorted(due, key=lambda x: (x.entity, x.tid)):
            self.timers.remove(tm)
            parties = (tm.entity,) if tm.kind == "DEATH_DUE" else (tm.entity, tm.payload)
            if not all(p in self.ledger and self.ledger[p].alive for p in parties):
                self.dropped[(tm.kind, "dead")] += 1
                self.trace(f"drop_timer {tm.kind} {tm.entity} dead")
                continue
            if tm.kind == "DEATH_DUE":
                self.ledger[tm.entity].alive = False  # M2's drain applies the ScheduleDeath
                what = f"ScheduleDeath victim={tm.entity} culprit={tm.payload}; M2 applies it"
            else:
                what = f"Marry a={tm.entity} b={tm.payload} submitted to M2"
            self.trace(f"fire {tm.kind}: {what}")

    def stub_send(self, op: Op) -> None:
        """A stub S6 or S7 send at Vital."""
        kind = SENDS_VITAL[op.verb]
        i = op.ints
        if kind == "OpinionModifier":
            self.send(VITAL, kind, self.opinion_fields(op))
        elif kind == "ApplyThought":
            ov = self.reg.overlay_codes.index(op.name)
            fields = {"holder": i[0], "overlay": ov, "delta": i[1], "source_event": i[2]}
            self.send(VITAL, kind, fields)
        elif kind == "ApplyInfluence":
            ov = self.reg.overlay_codes.index(op.name)
            self.send(VITAL, kind, {"bearer": i[0], "overlay": ov, "delta": i[1]}, cause=i[2])
        elif kind == "AgendaGoal":
            vk = self.reg.vow_codes.index(op.name)
            fields = {"vow": i[0], "holder": i[1], "kind": vk, "target": i[2], "priority": i[3]}
            self.send(VITAL, kind, fields)
        else:
            self.send(VITAL, kind, {"vow": i[0], "holder": i[1]})

    def opinion_fields(self, op: Op) -> dict[str, int]:
        """An OpinionModifier payload from (holder, about, magnitude, source_event) and a kind."""
        h, about, mag, src = op.ints
        code = self.reg.opinion_codes.index(op.name)
        return {"holder": h, "about": about, "kind": code, "magnitude": mag, "source_event": src}

    def polity(self, ops: Sequence[Op], lagged_named: dict[int, int]) -> None:
        """Polity: M7 writes its titles and offices and sends Promote, Demote, OpinionModifier."""
        for op in ops:
            if op.verb == "title":
                self.titles[op.ints[0]] = (op.ints[1], op.ints[2])
            elif op.verb == "office":
                self.offices[op.ints[0]] = (op.ints[1], op.ints[2])
            elif op.verb == "dormancy0":
                for p in sorted(self.case.criteria0):
                    if lagged_named.get(p, 0) == 0:
                        self.send(POLITY, "Promote", {"person": p})
            elif op.verb == "m7_opinion":
                self.send(POLITY, "OpinionModifier", self.opinion_fields(op))
            elif op.verb in SENDS_POLITY:
                self.send(POLITY, SENDS_POLITY[op.verb], {"person": op.ints[0]})

    def take(self) -> list[Command]:
        """The binding's whole inbox in key order; rh-core's dead-key drop at the take."""
        inbox = sorted(self.queue, key=lambda c: c.key)
        self.queue = []
        kept = []
        for c in inbox:
            living = self.spec.applies[c.kind][0] == "living"
            row = self.ledger.get(c.person)
            if living and (row is None or not row.alive):
                self.drop(c, "dead_key")
            else:
                kept.append(c)
        return kept

    def day_agenda(self, ops: Sequence[Op], lagged: tuple[dict[int, Any], ...]) -> None:
        """A15's Day/Agenda: world_start, apply_commands, promote; then the stand-ins."""
        self.at("Day/Agenda", "take")
        inbox = self.take()
        if self.t == 0:
            self.at("Day/Agenda", "world_start")
            self.world_start(lagged)
        self.at("Day/Agenda", "apply_commands")
        for c in inbox:
            if c.kind in APPLY_KINDS:
                self.apply_command(c)
        self.at("Day/Agenda", "promote")
        self.promote_step([c for c in inbox if c.kind in PROMOTE_KINDS], lagged)
        self.at("Day/Agenda", "stand-in (T1-I/S)")
        for op in ops:
            self.stand_in(op)

    def stand_in(self, op: Op) -> None:
        """The T1-I/S stand-ins at the end of Day/Agenda (break_act ... threshold, decide)."""
        if op.verb == "break":
            mind = self.mind_of(op.ints[0])
            if mind is not None:
                mind.break_state = BREAK_STATES.index(op.name)
                self.trace(f"break_state {op.ints[0]} := {op.name}")
        elif op.verb == "timer":
            entity, due_day, payload = op.ints
            phase = VITAL if op.name in OWED_KINDS else AGENDA
            tm = Timer(self.next_timer, op.name, entity, due_day * self.k.day, phase, payload)
            self.next_timer += 1
            self.timers.append(tm)
            self.trace(f"schedule {op.name} entity={entity} due=d{due_day}")
        elif op.verb == "probe":
            self.probe(op.ints[0], op.ints[1])

    def bookkeeping(self) -> None:
        """Bookkeeping: rh-core compacts the dead's ledger rows; then the observers."""
        for p in [p for p, x in self.ledger.items() if not x.alive]:
            del self.ledger[p]
        if self.cfg.observe:
            self.observe()

    # ------------------------------------------------------------------ A4, A5: creation
    def create(self, p: int) -> None:
        """A4: append p's mind, its traits drawn at t (0 at world start) and p's epoch."""
        epoch = self.ledger[p].epoch
        t_key = 0 if self.mut("trait-key-wrong-t") else self.t
        traits, ambition, _sums = draw_mind(self.k, self.case.seed, p, epoch, t_key)
        if self.mut("trait-k-swap"):
            traits = (traits[1], traits[0], *traits[2:])
        self.minds.append(Mind(p, traits, ambition))
        self.created[p] = (self.t, epoch, traits, ambition)
        self.trace(
            f"create {p} row={len(self.minds) - 1} key=(t={t_key}, epoch={epoch}) "
            f"traits={','.join(map(str, traits))} ambition={ambition}",
        )

    def world_start(self, lagged: tuple[dict[int, Any], ...]) -> None:
        """A5: at t = 0, a mind for each living named person without one, ascending."""
        pairs = implied_pairs(lagged)
        for p in sorted(self.ledger):
            row = self.ledger[p]
            if row.named == 1 and row.alive and self.mind_of(p) is None:
                self.create(p)
                self.refresh(p, pairs)

    # ------------------------------------------------------------------ A6, A7: Promote, Demote
    def promote_step(self, cmds: Sequence[Command], lagged: tuple[dict[int, Any], ...]) -> None:
        """The promote step: Promote and Demote in key order, the pairs computed once."""
        if not cmds:
            return
        pairs = implied_pairs(lagged)
        for c in cmds:
            if c.kind == "Promote":
                self.promote(c, pairs)
            else:
                self.demote(c)

    def promote(self, c: Command, pairs: dict[int, list[tuple[int, str]]]) -> None:
        """A6: create the mind or reuse the kept one, never redrawn; then refresh(p)."""
        p = c.person
        if self.ledger[p].named == 1:
            self.drop(c, "ended")
            return
        self.ledger[p].named = 1
        mind = self.mind_of(p)
        if mind is None:
            self.create(p)
        else:
            if self.mut("redraw-on-repromote"):
                traits, ambition, _ = draw_mind(
                    self.k, self.case.seed, p, self.ledger[p].epoch, self.t
                )
                mind.traits, mind.ambition = traits, ambition
            self.trace(f"reuse {p} row={self.minds.index(mind)} (never redrawn)")
        self.refresh(p, pairs)

    def demote(self, c: Command) -> None:
        """A7: dropped as ended or deferred, or cancel, unname, delete, end."""
        p = c.person
        mind = self.mind_of(p)
        if mind is None or self.ledger[p].named == 0:
            self.drop(c, "ended")
            return
        if mind.break_state != CALM:
            self.drop(c, "deferred")
            return
        cancelled = self.cancel_own(p)
        self.ledger[p].named = 0
        goals = self.delete_goals(p)
        thoughts = self.delete_thoughts(p)
        ended = self.end_rows(self.look(self.ix_a, self.relations, (p,)))
        self.trace(
            f"demote {p}: cancelled {cancelled or 'none'}; goals {goals}, thoughts {thoughts} "
            f"deleted; rows ended {ended or 'none'}",
        )

    def cancel_own(self, p: int) -> str:
        """A7 (a): p's own poll, interrupt, break and refractory-end timers, by kind."""
        out = []
        for tm in sorted((x for x in self.timers if x.entity == p), key=lambda x: (x.due, x.tid)):
            if tm.kind in CANCELLED_KINDS or self.mut("demote-cancels-death-due"):
                self.timers.remove(tm)
                self.cancelled[(tm.kind, "ended")] += 1
                out.append(tm.kind)
                if tm.kind in OWED_KINDS and self.cfg.observe:
                    self.fail(
                        f"A7 (owed outcome): a Demote of {p} at t={self.t} cancelled the owed "
                        f"{tm.kind} due t={tm.due}: a Demote cancels poll, interrupt, break and "
                        "refractory-end timers only (ADR-0025 item 1)"
                    )
        return ",".join(out)

    # ------------------------------------------------------------------ A8: exits
    def exits(self, exited: Sequence[int]) -> None:
        """A8: end the exited persons' open rows, as a or b; delete their thought rows."""
        for d in exited:
            rows = sorted(
                set(self.look(self.ix_a, self.relations, (d,)))
                | set(self.look(self.ix_b, self.relations, (d,)))
            )
            ended = self.end_rows(rows)
            thoughts = self.delete_thoughts(d)
            self.trace(f"{d}: rows ended {ended or 'none'}, thoughts {thoughts} deleted")

    def end_rows(self, rows: Sequence[int]) -> str:
        """ended_at := now on each open row of ``rows``, in row order."""
        out = []
        for r in rows:
            if self.relations[r].ended_at == 0:
                self.relations[r].ended_at = self.t
                out.append(str(r))
        return ",".join(out)

    # ------------------------------------------------------------------ A9: relations
    def kept(self, a: int, b: int) -> bool:
        """A9: a named living a and a living b, both with a ledger row."""
        ra, rb = self.ledger.get(a), self.ledger.get(b)
        return bool(ra and rb and ra.named == 1 and ra.alive and rb.alive)

    def refresh(self, a: int, pairs: dict[int, list[tuple[int, str]]]) -> None:
        """A9's refresh of ``a``: end the open rows no longer implied, append the new pairs."""
        open_rows = [
            r for r in self.look(self.ix_a, self.relations, (a,)) if self.relations[r].ended_at == 0
        ]
        want = [(b, kind) for b, kind in pairs.get(a, []) if self.kept(a, b)]
        have = {(self.relations[r].b, self.relations[r].kind) for r in open_rows}
        for r in open_rows:
            rel = self.relations[r]
            if (rel.b, rel.kind) not in want and not self.mut("relation-not-ended"):
                rel.ended_at = self.t
                self.trace(f"refresh {a}: end row={r} ({a}, {rel.b}, {rel.kind})")
        for b, kind in want:
            if (b, kind) not in have:
                self.relations.append(Relation(a, b, kind))
                self.trace(f"refresh {a}: append row={len(self.relations) - 1} ({a}, {b}, {kind})")
        if self.cfg.observe:
            now = sorted(
                (self.relations[r].b, self.relations[r].kind)
                for r in self.look(self.ix_a, self.relations, (a,))
                if self.relations[r].ended_at == 0
            )
            if now != sorted(want):
                self.fail(
                    f"INV-T1-12 (refresh): right after refresh({a}) at t={self.t} its open rows "
                    f"are {now}, its implied pairs {sorted(want)}"
                )

    def relations_step(self, lagged: tuple[dict[int, Any], ...]) -> None:
        """A9's Month/Agenda step: refresh every named living actor, ascending PersonId."""
        pairs = implied_pairs(lagged)
        for key in sorted(self.ix_person.rows):
            row = self.ledger.get(key[0])
            if row is not None and row.named == 1 and row.alive:
                self.refresh(key[0], pairs)

    # ------------------------------------------------------------------ A10, A11: opinions
    def apply_command(self, c: Command) -> None:
        """apply_commands: A10's OpinionModifier and A12's thought and goal commands."""
        if c.kind == "OpinionModifier":
            self.apply_opinion(c)
        elif c.kind in ("ApplyThought", "ApplyInfluence"):
            self.apply_thought(c)
        elif c.kind == "AgendaGoal":
            self.upsert_goal(c)
        else:
            self.clear_goal(c)

    def apply_opinion(self, c: Command) -> None:
        """A10: one row; the Registry magnitude, else the payload's clamped; expires by MONTH."""
        kind = self.reg.opinion_codes[c.get("kind")]
        reg_mag = self.reg.opinion_mag[kind]
        mag = reg_mag if reg_mag != 0 else clamp(c.get("magnitude"), -100 * Q8, 100 * Q8)
        expires = self.t + self.reg.opinion_months[kind] * self.cal.month
        row = Opinion(c.get("holder"), c.get("about"), kind, mag, c.get("source_event"), expires)
        self.opinions.append(row)
        self.trace(
            f"opinion row={len(self.opinions) - 1} ({row.holder} about {row.about}) {kind} "
            f"magnitude={mag} expires={expires} source={row.source_event}",
        )

    def opinion(self, a: int, b: int) -> tuple[int, int]:
        """A11: clamp(sum of the unexpired modifiers), and how many rows counted."""
        rows = self.look(self.ix_op, self.opinions, (a, b))
        live = [
            self.opinions[r]
            for r in rows
            if self.opinions[r].expires > self.t or self.mut("opinion-no-expiry")
        ]
        return clamp(sum(o.magnitude for o in live), -100 * Q8, 100 * Q8), len(live)

    def probe(self, a: int, b: int) -> None:
        """A stand-in for decide's read of opinion(a, b), in this binding (A2's freshness)."""
        value, n = self.opinion(a, b)
        self.trace(f"probe opinion({a}, {b}) = {value} over {n} rows")
        if self.cfg.observe and value != self.opinion_full(a, b):
            self.fail(
                f"INV-T1-06: opinion({a}, {b}) read at t={self.t} in the binding is {value}, the "
                f"whole table gives {self.opinion_full(a, b)}"
            )

    def opinion_full(self, a: int, b: int) -> int:
        """INV-T1-06's observer: the sum recomputed from the whole table."""
        total = sum(
            o.magnitude
            for o in self.opinions
            if o.holder == a and o.about == b and o.expires > self.t
        )
        return clamp(total, -100 * Q8, 100 * Q8)

    # ------------------------------------------------------------------ A12: thoughts and goals
    def apply_thought(self, c: Command) -> None:
        """A12: find or insert the (holder, overlay) row; add 100 x d, capped; refreshed now."""
        holder = c.person
        overlay = c.get("overlay")
        d = clamp(c.get("delta"), 0, DELTA_MAX)
        source = c.get("source_event") if c.kind == "ApplyThought" else c.cause
        key = (holder, overlay)
        i = bisect.bisect_left(self.thoughts, key, key=lambda x: (x.holder, x.overlay))
        if i == len(self.thoughts) or (self.thoughts[i].holder, self.thoughts[i].overlay) != key:
            self.thoughts.insert(i, Thought(holder, overlay, 0, source, self.t))
        row = self.thoughts[i]
        row.magnitude = min(self.cap, row.magnitude + THOUGHT_GAIN * d)
        row.source_event, row.refreshed_at = source, self.t
        name = self.reg.overlay_codes[overlay]
        self.trace(f"thought ({holder}, {name}) magnitude={row.magnitude}")

    def upsert_goal(self, c: Command) -> None:
        """A12: AgendaGoal inserts or overwrites the (holder, vow) row."""
        key = (c.person, c.get("vow"))
        i = bisect.bisect_left(self.goals, key, key=lambda x: (x.holder, x.vow))
        g = Goal(c.person, c.get("vow"), c.get("kind"), c.get("target"), c.get("priority"))
        if i < len(self.goals) and (self.goals[i].holder, self.goals[i].vow) == key:
            self.goals[i] = g
            verb = "overwrite"
        else:
            self.goals.insert(i, g)
            verb = "insert"
        self.trace(f"goal {verb} ({g.holder}, vow {g.vow}) priority={g.priority}")

    def clear_goal(self, c: Command) -> None:
        """A12: ClearGoal removes the (holder, vow) row, else it is dropped as ended."""
        key = (c.person, c.get("vow"))
        i = bisect.bisect_left(self.goals, key, key=lambda x: (x.holder, x.vow))
        if i < len(self.goals) and (self.goals[i].holder, self.goals[i].vow) == key:
            del self.goals[i]
            self.trace(f"goal removed ({key[0]}, vow {key[1]})")
        else:
            self.drop(c, "ended")

    def delete_goals(self, p: int) -> int:
        """Delete p's goal rows; how many."""
        n = len(self.goals)
        self.goals = [g for g in self.goals if g.holder != p]
        return n - len(self.goals)

    def delete_thoughts(self, p: int) -> int:
        """Delete p's thought rows; how many."""
        n = len(self.thoughts)
        self.thoughts = [x for x in self.thoughts if x.holder != p]
        return n - len(self.thoughts)

    def expire(self) -> None:
        """A12's thought decay at t_m: rows with no command since the previous pulse."""
        earlier = [t for t in self.pulses if t < self.t]
        t_prev = earlier[-1] if earlier else I64_MIN
        for row in self.thoughts:
            if row.refreshed_at < t_prev and row.magnitude > 0:
                row.magnitude = max(0, row.magnitude - self.decay)
                name = self.reg.overlay_codes[row.overlay]
                self.trace(f"thought ({row.holder}, {name}) decays to {row.magnitude}")

    # ------------------------------------------------------------------ observers
    def observe(self) -> None:
        """Every invariant the A-steps state, over every row (the test observers' form)."""
        self.observe_traits()
        self.observe_named()
        self.observe_relations()
        self.observe_tables()
        for key in sorted(self.ix_op.rows):
            a, b = key
            if self.opinion(a, b)[0] != self.opinion_full(a, b):
                self.fail(f"INV-T1-06: opinion({a}, {b}) at t={self.t} differs from its recompute")

    def observe_traits(self) -> None:
        """INV-T1-05: traits never change, and are the draw keyed at the creation instant."""
        for m in self.minds:
            t0, epoch, traits, ambition = self.created[m.person]
            if (m.traits, m.ambition) != (traits, ambition):
                self.fail(
                    f"INV-T1-05 (immutable): the traits of the mind of {m.person} changed after "
                    f"its creation at t={t0} (a re-promotion reuses the kept row: REV-P1a MAJ-9)"
                )
            want = draw_mind(self.k, self.case.seed, m.person, epoch, t0)[:2]
            if (traits, ambition) != want:
                self.fail(
                    f"INV-T1-05 (key): the mind of {m.person} created at t={t0} does not hold "
                    f"trait_draw keyed at (person {m.person}, epoch {epoch}, t={t0}) (A3)"
                )

    def observe_named(self) -> None:
        """INV-T1-11: every named person has exactly one mind; mind.person is unique."""
        counts = Counter(m.person for m in self.minds)
        for p, row in sorted(self.ledger.items()):
            if row.named == 1 and counts[p] != 1:
                self.fail(f"INV-T1-11: {p} is named with {counts[p]} mind rows at t={self.t}")
        for p, n in sorted(counts.items()):
            if n > 1:
                self.fail(f"INV-T1-11: mind.person {p} is not unique ({n} rows)")

    def observe_relations(self) -> None:
        """INV-T1-12: every open row is liege or vassal, named living a, living b, unique."""
        seen = set()
        for r, rel in enumerate(self.relations):
            if rel.ended_at != 0:
                continue
            key = (rel.a, rel.b, rel.kind)
            ok = rel.kind in (LIEGE, VASSAL) and rel.a != rel.b and self.kept(rel.a, rel.b)
            if not ok or key in seen:
                self.fail(f"INV-T1-12: open row {r} {key} at t={self.t}")
            seen.add(key)

    def observe_tables(self) -> None:
        """INV-T1-13 and INV-T1-05's thought bound."""
        keys = [(x.holder, x.overlay) for x in self.thoughts]
        if keys != sorted(set(keys)):
            self.fail(f"INV-T1-13: the thought table is not sorted and unique at t={self.t}")
        gkeys = [(g.holder, g.vow) for g in self.goals]
        if gkeys != sorted(set(gkeys)):
            self.fail(f"INV-T1-13: the goal table is not sorted and unique at t={self.t}")
        for x in self.thoughts:
            if not 0 <= x.magnitude <= self.cap:
                self.fail(f"INV-T1-05: thought ({x.holder}, {x.overlay}) = {x.magnitude}")

    # ------------------------------------------------------------------ the final state
    def state_lines(self) -> None:
        """The case's tables and counters at its horizon."""
        n = self.case.name
        for r, m in enumerate(self.minds):
            row = self.ledger.get(m.person)
            named = "-" if row is None else str(row.named)
            self.lines.append(
                f"{n} mind row={r} person={m.person} traits={','.join(map(str, m.traits))} "
                f"ambition={m.ambition} named={named} break={BREAK_STATES[m.break_state]}"
            )
        for r, rel in enumerate(self.relations):
            self.lines.append(
                f"{n} relation row={r} ({rel.a}, {rel.b}, {rel.kind}) ended_at={rel.ended_at}"
            )
        for r, o in enumerate(self.opinions):
            self.lines.append(
                f"{n} opinion row={r} ({o.holder} about {o.about}) {o.kind} "
                f"magnitude={o.magnitude} expires={o.expires} source={o.source_event}"
            )
        for x in self.thoughts:
            self.lines.append(
                f"{n} thought ({x.holder}, {self.reg.overlay_codes[x.overlay]}) "
                f"magnitude={x.magnitude} refreshed_at={x.refreshed_at} source={x.source_event}"
            )
        for g in self.goals:
            self.lines.append(
                f"{n} goal ({g.holder}, vow {g.vow}) {self.reg.vow_codes[g.kind]} "
                f"target={g.target} priority={g.priority}"
            )
        self.lines += [f"{n} timer {tm.kind} entity={tm.entity} due={tm.due}" for tm in self.timers]
        self.lines += [f"{n} dropped {k} {r} {c}" for (k, r), c in sorted(self.dropped.items())]
        self.lines += [f"{n} cancelled {k} {r} {c}" for (k, r), c in sorted(self.cancelled.items())]


def implied_pairs(lagged: tuple[dict[int, Any], ...]) -> dict[int, list[tuple[int, str]]]:
    """A9: the liege and vassal pairs M7's lagged titles and offices imply, a -> sorted (b, kind).

    A title's vassal titles' holders and its offices' holders serve its holder (the court of
    effect_language_v0.md §5); a pair with no holder or a holder serving itself is none.
    """
    titles, offices = lagged
    pairs: set[tuple[int, int, str]] = set()

    def add(h: int, g: int) -> None:
        if g not in (0, h):
            pairs.update({(h, g, LIEGE), (g, h, VASSAL)})

    for tid in sorted(titles):
        h, liege = titles[tid]
        if h != 0 and liege != 0:
            add(h, titles.get(liege, (0, 0))[0])
    for oid in sorted(offices):
        title, h = offices[oid]
        if h != 0:
            add(h, titles.get(title, (0, 0))[0])
    out: dict[int, list[tuple[int, str]]] = {}
    for a, b, kind in sorted(pairs, key=lambda x: (x[0], x[1], x[2])):
        out.setdefault(a, []).append((b, kind))
    return out


# ----------------------------------------------------------------------------- the cases (check 6)
def founders(seed: int, n: int) -> tuple[int, ...]:
    """Founder PersonIds k = 0 .. n - 1 (rh-ids' founder layout; SPR-011 ruling 10's order)."""
    return tuple(rng.h64_words(rng.TAG_FOUNDER, [seed, k]) for k in range(n))


F = founders(7, 8)
CASES: tuple[Case, ...] = (
    Case(
        "world_start",
        "X4: genesis names the cast (F0-F2); M7's tick-0 pass promotes F3-F6; M7 demotes F2",
        360,
        7,
        F,
        F[:3],
        F[3:7],
        ((100, F[3], 0), (200, F[0], 100), (300, F[4], 100)),
        ((1, 100, F[1]), (2, 200, F[5])),
        (Op(0, "dormancy0"), Op(0, "demote", (F[2],))),
        30,
    ),
    Case(
        "promote_cycle",
        "MAJ-9 re-promotion, epochs, ended and deferred drops, the timers a Demote keeps",
        360,
        11,
        tuple(range(1001, 1010)),
        (),
        (1003, 1007),
        ((10, 1003, 0), (20, 1007, 10)),
        ((5, 10, 1006), (6, 10, 1008)),
        (
            Op(0, "dormancy0"),
            Op(2, "epoch", (1005,)),
            Op(5, "promote", (1005,)),
            Op(5, "promote", (1008,)),
            Op(5, "promote", (1003,)),
            Op(6, "timer", (1007, 400, 0), "poll"),
            Op(6, "timer", (1007, 300, 0), "interrupt"),
            Op(6, "timer", (1007, 45, 1005), "MARRY_DUE"),
            Op(6, "timer", (1008, 70, 1003), "DEATH_DUE"),
            Op(6, "goal", (501, 1007, 1008, 40 * Q8), "AVENGE_KIN"),
            Op(6, "thought", (1007, 64, 600), "paranoia"),
            Op(40, "demote", (1007,)),
            Op(40, "demote", (1008,)),
            Op(40, "demote", (1009,)),
            Op(46, "break", (1005,), "pending"),
            Op(50, "demote", (1005,)),
            Op(55, "break", (1005,), "calm"),
            Op(100, "promote", (1006,)),
            Op(100, "demote", (1006,)),
            Op(360, "demote", (1005,)),
            Op(360, "promote", (1007,)),
        ),
        390,
    ),
    Case(
        "relations_monthly",
        "365-day months: a regrant, a death and a resignation reach the rows at the next pulse",
        365,
        13,
        tuple(range(2001, 2011)),
        (),
        (2001, 2002, 2003, 2004),
        ((1, 2001, 0), (2, 2002, 1), (3, 2003, 1)),
        ((1, 1, 2004), (2, 2, 2005)),
        (
            Op(0, "dormancy0"),
            Op(10, "title", (3, 2006, 1)),
            Op(10, "promote", (2006,)),
            Op(40, "die", (2002,)),
            Op(40, "title", (2, 2007, 1)),
            Op(40, "promote", (2007,)),
            Op(70, "office", (1, 1, 0)),
            Op(100, "die", (2005,)),
            Op(100, "die", (2008,)),
            Op(365, "demote", (2003,)),
        ),
        365,
    ),
    Case(
        "opinions",
        "365 days: Registry and payload magnitudes, the clamp, expiry, freshness, dead keys",
        365,
        17,
        tuple(range(3001, 3007)),
        (),
        (3001, 3002),
        (),
        (),
        (
            Op(0, "dormancy0"),
            Op(1, "m7_opinion", (3003, 3001, 0, 900), "GRANTED_BY"),
            Op(1, "promote", (3003,)),
            Op(1, "probe", (3003, 3001)),
            *(Op(2, "s7_opinion", (3001, 3002, 0, 901 + i), "KIN_HARMED_BY") for i in range(5)),
            Op(2, "probe", (3001, 3002)),
            Op(3, "s7_opinion", (3002, 3001, -3200, 906), "VOW_BROKEN_BY"),
            Op(3, "s7_opinion", (3002, 3005, -30720, 907), "VOW_BROKEN_BY"),
            Op(3, "s7_opinion", (3002, 3001, 7 * Q8, 908), "CLAIM_PRESSED_BY"),
            Op(3, "probe", (3002, 3001)),
            Op(3, "probe", (3002, 3005)),
            Op(5, "s7_opinion", (3001, 3004, 0, 909), "FEUD_WITH"),
            Op(10, "die", (3004,)),
            Op(10, "s7_opinion", (3004, 3001, 0, 910), "MARRIED_TO"),
            Op(20, "probe", (3001, 3004)),
            Op(30, "s7_opinion", (3004, 3002, 0, 911), "MARRIED_TO"),
            Op(3652, "probe", (3002, 3001)),
            Op(3653, "probe", (3002, 3001)),
        ),
        3653,
    ),
    Case(
        "opinions_360",
        "360 days: CLAIM_PRESSED_BY's 120 months are 5,184,000 minutes",
        360,
        19,
        (3101, 3102),
        (),
        (3101, 3102),
        (),
        (),
        (
            Op(0, "dormancy0"),
            Op(1, "s7_opinion", (3101, 3102, 0, 950), "CLAIM_PRESSED_BY"),
            Op(1, "probe", (3101, 3102)),
            Op(3600, "probe", (3101, 3102)),
            Op(3601, "probe", (3101, 3102)),
        ),
        3601,
    ),
    Case(
        "thoughts_goals",
        "thoughts capped and decayed monthly, goals upserted and cleared, exits and a Demote",
        360,
        23,
        tuple(range(4001, 4005)),
        (),
        (4001, 4002),
        (),
        (),
        (
            Op(0, "dormancy0"),
            Op(1, "influence", (4001, 64, 700), "paranoia"),
            Op(1, "thought", (4001, 26, 701), "hubris"),
            Op(1, "goal", (9001, 4001, 4003, 40 * Q8), "AVENGE_KIN"),
            Op(2, "influence", (4001, 128, 702), "paranoia"),
            Op(2, "influence", (4001, 384, 703), "corruption"),
            Op(3, "goal", (9001, 4001, 4003, 60 * Q8), "AVENGE_KIN"),
            Op(3, "goal", (9002, 4002, 4001, 20 * Q8), "PROTECT"),
            Op(3, "thought", (4002, 128, 704), "despair"),
            Op(3, "thought", (4003, 64, 705), "filial_piety"),
            Op(3, "thought", (4001, 13, 706), "filial_piety"),
            Op(100, "influence", (4001, 0, 707), "paranoia"),
            Op(130, "die", (4002,)),
            Op(140, "clear_goal", (9002, 4002)),
            Op(140, "clear_goal", (9999, 4001)),
            Op(150, "demote", (4001,)),
            Op(160, "influence", (4001, 64, 708), "despair"),
        ),
        180,
    ),
)


def run_cases(k: Consts, reg: Reg, spec: Spec, cfg: Config) -> tuple[list[str], list[str]]:
    """Check 6: every case's golden lines and its observers' failures."""
    lines, failures = [], []
    for case in CASES:
        lines.append(
            f"case {case.name} days_per_year={case.days} month={k.month(case.days)} "
            f"seed={case.seed}: {case.note}"
        )
        sim = Sim(case, k, reg, spec, cfg)
        sim.run()
        lines += sim.lines
        failures += sim.failures
    return lines, failures


# ----------------------------------------------------------------------------- checks 1 to 5
def check_values(spec: Spec, reg: Reg, k: Consts) -> list[str]:
    """Check 1: check_values against this file's arithmetic and the Registry."""
    want: dict[str, Any] = {
        "sigma_q16": SIGMA,
        "trait_field_mean": k.field_mean,
        "trait_field_max": k.fields * k.field_mask,
        "ambition_mean_raw": k.amb_mean,
        "trait_bounds_raw": [k.trait_lo, k.trait_hi],
        "month_minutes": [k.month(d) for d in CALENDARS],
        "relation_codes": list(reg.relation_codes),
        "closeness_raw": [reg.closeness[c] for c in reg.relation_codes],
        "opinion_codes": list(reg.opinion_codes),
        "opinion_magnitude_raw": [reg.opinion_mag[c] for c in reg.opinion_codes],
        "opinion_months": [reg.opinion_months[c] for c in reg.opinion_codes],
        "overlay_codes": list(reg.overlay_codes),
        "overlay_traits_raw": reg.overlay_traits,
    }
    errs = [
        f"check_values.{name} = {spec.check.get(name)!r}, the reference computes {value!r}"
        for name, value in want.items()
        if _plain(spec.check.get(name)) != value
    ]
    errs += [f"check_values.{n} is not checked" for n in sorted(set(spec.check) - set(want))]
    if k.trait_k != len(HEXACO) + 1:
        errs.append(f"constants.trait_k = {k.trait_k}: six HEXACO traits and ambition are 7")
    return errs


def _plain(v: object) -> object:
    """A TOML value with inline tables as dicts and lists as lists (for comparison)."""
    if isinstance(v, dict):
        return {str(a): _plain(b) for a, b in v.items()}
    if isinstance(v, list):
        return [_plain(x) for x in v]
    return v


def check_widths(k: Consts, reg: Reg) -> list[str]:
    """Check 2: the widths the steps state hold at the Registry extremes."""
    errs = []
    sd_max = fixed(reg.bounds["trait_sd"][1], Q16)
    product = Q8 * sd_max * (k.fields * k.field_mask - k.field_mean)
    if product >= 1 << WIDTH_TRAIT_BITS:
        errs.append(f"A3: 256 x B x c reaches {product}, not below 2^47")
    cap_max = narrow(fixed(reg.bounds["thought_cap"][1], Q16))
    worst = cap_max + THOUGHT_GAIN * DELTA_MAX
    if not I16_MAX < worst <= I32_MAX:
        errs.append(f"A12: the thought sum reaches {worst}: the step says it needs i32, not i16")
    for name in ("trait_sd", "thought_cap", "thought_decay"):
        lo, hi = reg.bounds[name]
        if not lo <= PARAMS[name] <= hi:
            errs.append(f"the scripted {name} {PARAMS[name]} is outside the Registry [{lo}, {hi}]")
    raws = [*reg.closeness.values(), *reg.opinion_mag.values()]
    raws += [v for row in reg.overlay_traits.values() for v in row.values()]
    errs += [f"a Registry value {v} does not fit q8_8" for v in raws if abs(v) > I16_MAX]
    return errs


def prose_needles(k: Consts, reg: Reg) -> dict[str, list[str]]:
    """Check 3: what each step's text must state, computed here."""
    claim = reg.opinion_months["CLAIM_PRESSED_BY"]
    kin = reg.opinion_mag["KIN_HARMED_BY"]
    five = 5 * kin
    cap_max = narrow(fixed(reg.bounds["thought_cap"][1], Q16))
    m360, m365 = (k.month(d) for d in CALENDARS)
    return {
        "A0": [f"{commas(m360)} at 360", f"{commas(m365)} at 365"],
        "A3": [str(SIGMA), SIGMA_TEXT, "< 2^47"],
        "A9": [f"codes {reg.relation_codes.index(LIEGE)} and {reg.relation_codes.index(VASSAL)}"],
        "A10": [
            f"({claim} months)",
            f"t + {commas(claim * m365)} at 365",
            f"t + {commas(claim * m360)} at 360",
        ],
        "A11": [f"({kin // Q8} each)", f"sum to {five}", f"read {-100 * Q8}"],
        "A12": [
            f"{commas(cap_max)} + {commas(THOUGHT_GAIN * DELTA_MAX)} = "
            f"{commas(cap_max + THOUGHT_GAIN * DELTA_MAX)}"
        ],
    }


def check_prose(spec: Spec, k: Consts, reg: Reg) -> list[str]:
    """Check 3: the step prose states the numbers this file computes."""
    errs = []
    for step, needles in prose_needles(k, reg).items():
        text = spec.steps.get(step, "")
        errs += [f"{step} does not state {n!r}" for n in needles if n not in text]
    return errs


def trait_table(k: Consts) -> tuple[list[str], list[str]]:
    """Check 4: the draws against gen_rng_golden.h, sample lines, moments, a clamp witness."""
    errs, lines = [], []
    for p in range(1, CROSS_PERSONS + 1):
        for epoch, t in ((0, 0), (1, 0), (0, 7 * k.day)):
            got = raw_draws(k, MOMENT_SEED, p, epoch, t)
            want = [rng.h(rng_key(MOMENT_SEED, p, epoch, t), i) for i in range(k.trait_k)]
            if got != want:
                errs.append(f"A3: the draws of person {p} differ from gen_rng_golden.h")
    combos = (
        (7, 1001, 0, 0, Decimal("20")),
        (7, 1001, 0, k.day, Decimal("20")),
        (7, 1001, 1, 0, Decimal("20")),
        (8, 1001, 0, 0, Decimal("20")),
        (7, 1001, 0, 0, Decimal("15")),
        (7, 1001, 0, 0, Decimal("50")),
        (7, F[0], 0, 0, Decimal("20")),
    )
    for seed, p, epoch, t, sd in combos:
        traits, amb, sums = draw_values(k, raw_draws(k, seed, p, epoch, t), fixed(sd, Q16))
        lines.append(
            f"traits seed={seed} person={p} epoch={epoch} t={t} trait_sd={sd}: "
            f"S={','.join(map(str, sums))} -> {','.join(map(str, traits))} ambition={amb}"
        )
    lines.append(clamp_witness(k))
    return lines, errs + moments(k)


def clamp_witness(k: Consts) -> str:
    """The first person (seed 7, t = 0) whose draw clamps at trait_sd 50."""
    sd = fixed(CLAMP_SD, Q16)
    p = 1
    while True:
        draws = raw_draws(k, MOMENT_SEED, p, 0, 0)
        v = [scaled(k, d, sd) for d in draws]
        traits, amb, _ = draw_values(k, draws, sd)
        if list(traits) != v[: len(HEXACO)] or amb != k.amb_mean + v[len(HEXACO)]:
            return (
                f"traits clamp witness seed={MOMENT_SEED} person={p} trait_sd={CLAMP_SD}: "
                f"unclamped {','.join(map(str, v))} -> {','.join(map(str, traits))} ambition={amb}"
            )
        p += 1


def moments(k: Consts) -> list[str]:
    """The unclamped values' mean and variance over 1,024 persons are 0 and trait_sd^2."""
    sd = fixed(PARAMS["trait_sd"], Q16)
    values = [
        scaled(k, d, sd)
        for p in range(1, MOMENT_PERSONS + 1)
        for d in raw_draws(k, MOMENT_SEED, p, 0, 0)
    ]
    n = len(values)
    mean = Fraction(sum(values), n)
    var = Fraction(sum(v * v for v in values), n) - mean * mean
    target = (Fraction(PARAMS["trait_sd"]) * Q8) ** 2
    errs = []
    if abs(mean) > MOMENT_TOL * Fraction(PARAMS["trait_sd"]) * Q8:
        errs.append(f"A3: the mean of {n} draws is {float(mean):.1f} raw, not near 0")
    if abs(var / target - 1) > MOMENT_TOL:
        errs.append(f"A3: the variance of {n} draws is {float(var / target):.4f} of trait_sd^2")
    return errs


def check_typing(doc: dict[str, Any]) -> list[str]:
    """Check 5: A14's refusals on planted copies of the three tables, each by its message."""
    errs = []
    rel, ops, ovt = doc["relation_kinds"], doc["opinion_modifiers"], doc["overlay_traits"]
    overlays = [str(i) for i in doc["overlays"]["ids"]]
    planted: list[tuple[Callable[[], list[str]], str]] = [
        (
            lambda: type_relation_kinds({**rel, "sibling": {"closeness": Decimal("1.5")}})[1],
            "closeness 1.5 outside [0, 1]",
        ),
        (
            lambda: type_relation_kinds({k: v for k, v in rel.items() if k != LIEGE})[1],
            "missing liege",
        ),
        (
            lambda: type_relation_kinds({**rel, "spouse": {"closeness": 1, "weight": 2}})[1],
            "unknown field weight",
        ),
        (
            lambda: type_opinion_modifiers({**ops, "FEUD_WITH": {"magnitude": -40, "months": 0}})[
                1
            ],
            "months 0 outside [1, 65535]",
        ),
        (
            lambda: type_opinion_modifiers(
                {**ops, "FEUD_WITH": {"magnitude": -40, "months": Decimal("12.5")}}
            )[1],
            "is not an integer",
        ),
        (
            lambda: type_opinion_modifiers({**ops, "FEUD_WITH": {"magnitude": 150, "months": 12}})[
                1
            ],
            "magnitude 150 outside [-100, 100]",
        ),
        (
            lambda: type_opinion_modifiers({**ops, "FEUD_WITH": {"magnitude": -40}})[1],
            "missing field months",
        ),
        (
            lambda: type_overlay_traits({**ovt, "rage": {"emotionality": 10}}, overlays)[1],
            "rage is not an [overlays] id",
        ),
        (
            lambda: type_overlay_traits({**ovt, "hubris": {"ambition": 10}}, overlays)[1],
            "ambition is not a HEXACO trait",
        ),
        (
            lambda: type_overlay_traits({**ovt, "despair": {"openness": 120}}, overlays)[1],
            "120 outside [-100, 100]",
        ),
    ]
    for probe, needle in planted:
        got = probe()
        if not any(needle in e for e in got):
            errs.append(f"A14: a planted table was not refused with {needle!r} (got {got})")
    return errs


# ----------------------------------------------------------------------------- render and main
def render(spec: Spec, reg: Reg, k: Consts, cfg: Config) -> tuple[str, dict[str, list[str]]]:
    """The golden text and every check's failures."""
    failures: dict[str, list[str]] = {
        "1. check values": check_values(spec, reg, k),
        "2. widths": check_widths(k, reg),
        "3. step prose": check_prose(spec, k, reg),
    }
    trait_lines, failures["4. trait draw"] = trait_table(k)
    failures["5. Registry typing"] = check_typing(load_toml(REGISTRY))
    case_lines, failures["6. scripted worlds"] = run_cases(k, reg, spec, cfg)
    params = " ".join(f"{n}={v}" for n, v in PARAMS.items())
    header = [
        "# minds v0, part A (T1-A; T1 0.4.0 [algorithm.minds_f1] A0-A16): GENERATED by",
        "# python -I specs/minds/selftest_minds_v0.py --write; do not edit. Traits, ambition and",
        "# magnitudes are q8_8 raw (1/256 point); t is SimTime in minutes, d its day.",
        f"algorithm {spec.version} sigma_q16={SIGMA} params {params}",
    ]
    return "\n".join([*header, *trait_lines, *case_lines]) + "\n", failures


def first_change(golden: str, text: str) -> tuple[int, str, str, int]:
    """The first changed golden line (1-based), its text, its replacement, and the count."""
    a, b = golden.splitlines(), text.splitlines()
    ops = [o for o in difflib.SequenceMatcher(None, a, b, autojunk=False).get_opcodes()]
    changed = [o for o in ops if o[0] != "equal"]
    if not changed:
        return 0, "", "", 0
    _tag, i1, i2, j1, j2 = changed[0]
    count = sum(max(o[2] - o[1], o[4] - o[3]) for o in changed)
    old = a[i1] if i1 < i2 else "(no line)"
    new = b[j1] if j1 < j2 else "(no line)"
    return i1 + 1, old, new, count


def report(failures: dict[str, list[str]]) -> bool:
    """Print each check's verdict; True when all pass."""
    ok = True
    for name, errs in failures.items():
        print(f"{name}: {'PASS' if not errs else 'FAIL'}")
        for e in errs:
            print(f"  {e}")
        ok = ok and not errs
    return ok


def run_control(name: str, failures: dict[str, list[str]]) -> int:
    """--control: the planted mutant must fail its own observer."""
    own = MUTANTS[name]
    found = [e for errs in failures.values() for e in errs]
    report(failures)
    if any(own in e for e in found):
        print(f"CONTROL {name}: FAIL for its own reason ({own})")
        print("MINDS V0 (A-STEPS): FAIL")
        return EXIT_FAIL
    if found:
        print(f"CONTROL {name}: FAIL, but not for its own reason ({own})")
        print("MINDS V0 (A-STEPS): FAIL")
        return EXIT_FAIL
    print(f"CONTROL {name}: NOT DETECTED (no observer failed)")
    print("MINDS V0 (A-STEPS): PASS")
    return EXIT_OK


def run_mutant(name: str, text: str, golden: Path) -> int:
    """--mutant: the planted mutant must change a named golden line."""
    current = golden.read_text(encoding="utf-8") if golden.exists() else ""
    line, old, new, count = first_change(current, text)
    if count == 0:
        print(f"MUTANT {name} SURVIVED: no golden line changed")
        return EXIT_OK
    print(f"STALE: {golden.name} line {line} ({count} golden lines changed)")
    print(f"  golden: {old}")
    print(f"  mutant: {new}")
    print(f"MUTANT {name}: FAIL, it changes the named golden line above")
    return EXIT_FAIL


def check_golden(text: str, golden: Path) -> int:
    """The default: the golden file must equal the reference's output."""
    current = golden.read_text(encoding="utf-8") if golden.exists() else ""
    if current != text:
        line, old, new, count = first_change(current, text)
        print(f"STALE: {golden} differs from the reference at line {line} ({count} lines): {old}")
        print(f"  the reference renders: {new}")
        print("MINDS V0 (A-STEPS): FAIL")
        return EXIT_FAIL
    print(f"MINDS V0 (A-STEPS): PASS ({len(CASES)} cases, {text.count(chr(10))} golden lines)")
    return EXIT_OK


def parse(argv: Sequence[str] | None) -> argparse.Namespace:
    """The command line."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n", 1)[0])
    parser.add_argument("--check", action="store_true", help="check the golden file (default)")
    parser.add_argument("--write", action="store_true", help="regenerate the golden file")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the golden file")
    parser.add_argument("--spec", type=Path, default=None, help="the T1 spec to read")
    names = sorted({*MUTANTS, *ALIASES})
    parser.add_argument("--control", choices=names, help="a mutant that must fail its observer")
    parser.add_argument("--mutant", choices=names, help="a mutant that must change the golden")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    """Run the checks; then check, write, or run a control or a mutant."""
    args = parse(argv)
    spec = Spec.load(args.spec or default_spec())
    reg, typing_errs = load_registry(load_toml(REGISTRY))
    if typing_errs:
        print("\n".join(f"REGISTRY REFUSED: {e}" for e in typing_errs))
        return EXIT_FAIL
    k = Consts.of(spec)
    name = ALIASES.get(args.control or args.mutant or "", args.control or args.mutant or "")
    cfg = Config(name, observe=not args.mutant)
    text, failures = render(spec, reg, k, cfg)
    if args.control:
        return run_control(name, failures)
    if not report(failures):
        print("MINDS V0 (A-STEPS): FAIL (the reference's own checks)")
        return EXIT_FAIL
    if args.mutant:
        return run_mutant(name, text, args.golden)
    if args.write:
        args.golden.parent.mkdir(parents=True, exist_ok=True)
        args.golden.write_text(text, encoding="utf-8")
        print(f"wrote {args.golden} ({text.count(chr(10))} lines)")
        return EXIT_OK
    return check_golden(text, args.golden)


if __name__ == "__main__":
    sys.exit(main())
