"""V-DET-17 CRN pairing: the worked computation of ``specs/determinism/crn_pairing_v0.md``.

The protocol pairs two rollouts of the same world (manifest hash, seed, Registry, build and thread
count equal) whose input journals differ by one journaled material input: a scope-only input on one
person (the ``[[input]] tick, founder`` form of ``worlds/*/inputs.toml``), which bumps that
person's RNG epoch and re-samples a pending death (ADR-0025 item 3; M2 S11). Over the 10-year
horizon it then correlates the death instants of the unaffected persons across the pair; under the
keyed RNG H(world_seed, salt, system, epoch, purpose, entity, t, k) with a per-entity epoch,
every draw of every other entity is the same bytes in both runs, so their deaths coincide and the
correlation is 1. Docs/09 V-DET-17 asks for at least 0.9 on unaffected lineages.

This file is the protocol's exact worked computation on the dynastic fixture world
``twin_vales`` at seeds 1 and 2 (``specs/manifest/examples/twin_vales.yaml``), on an M2-only model
of the engine, every piece of which is a frozen or next-version reference already on `main`:

  * genesis by the SPR-019 reference (``specs/worldgen/selftest_population_synthesis.py``);
  * M2 F1 mortality MR9-MR13 and S8-S11 by ``specs/demography/selftest_m2_f1_mortality.py``
    (the schedule of e0 30 and beta 1; the density modifier is 1 below 10,000,000 living);
  * the keyed draws by ``bin/gen_rng_golden.py`` with the entity and epoch conventions of
    ``bin/gen_m2_draws_reference.py`` (StepCtx: entity = PersonId; ResampleCtx: entity =
    H64(resample-entity/v1, [Person, id]), keyed by the epoch after the input).

No births (a child born before the input exists identically in both runs and would only add
pairs; one born after it is not alive at the input in both runs, so the protocol excludes it), no
M7 and no T1 (their deaths are agenda or ruler-exit timers on named actors, who are in the affected
set by construction). The statistic, the affected set, the censoring and the threshold are the
protocol's; the companion §3 derives them. Golden: ``specs/determinism/golden/crn_pairing_v0.txt``.

Run: python -I specs/determinism/selftest_crn_pairing.py            (check; exit 1 on failure)
     python -I specs/determinism/selftest_crn_pairing.py --write    (regenerate the golden)
     python -I specs/determinism/selftest_crn_pairing.py --golden PATH
     python -I specs/determinism/selftest_crn_pairing.py --control global-epoch   (must FAIL: the
         input bumps every living person's epoch, the mistake a world-wide epoch counter would be)
Exit: 0 pass, 1 fail (the statistic, or a stale golden), 2 the model failed its own checks.
Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import sys
from bisect import bisect_right
from dataclasses import dataclass, field
from fractions import Fraction
from pathlib import Path
from types import ModuleType
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GOLDEN = HERE / "golden" / "crn_pairing_v0.txt"
EXIT_OK, EXIT_FAIL, EXIT_MODEL = 0, 1, 2


def _load(name: str, path: Path) -> ModuleType:
    """Load a reference by path (python -I adds no script directory to sys.path)."""
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SystemExit(f"{path} not found")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


rng = _load("gen_rng_golden", ROOT / "bin" / "gen_rng_golden.py")
mort = _load(
    "selftest_m2_f1_mortality", ROOT / "specs" / "demography" / "selftest_m2_f1_mortality.py"
)
fw = _load("selftest_fixture_worlds", ROOT / "specs" / "worldgen" / "selftest_fixture_worlds.py")
ps = fw.ps
Genesis = Any  # the SPR-019 reference's types, loaded by path

# ------------------------------------------------------------------------- the protocol's constants
WORLD = fw.TWIN_VALES  # the dynastic fixture world; the target is its primogeniture sovereign
SEEDS = (1, 2)
DAY = 1440
MONTH_DAYS = 30  # twin_vales runs the 360-day calendar: twelve months of 30 days
MONTH = MONTH_DAYS * DAY
YEAR = 12 * MONTH
HORIZON_YEARS = 10  # V-DET-17: paired rollouts over 10 years
T_END = HORIZON_YEARS * YEAR  # 5,184,000: deaths at or after it are censored
# The input's tick: inside year 1, on the 15th day of its first month, 6 h 7 min into the day: not
# a Month pulse and not a day start, so it lands between an M2 pulse and a pending timer; early
# enough that the genesis sovereign is alive at it at both seeds (the golden says so).
T_IN = YEAR + 14 * DAY + 6 * 60 + 7  # 538,927
KIN_DEGREES = 6  # k: the effect language's kin(p, n) reaches 6 and M7's dormancy_degrees <= 6
THRESHOLD = Fraction(9, 10)  # docs/09 V-DET-17: correlation >= 0.9 on the unaffected
MIN_UNAFFECTED = 100  # the protocol needs at least this many unaffected persons alive at the input
E0, BETA = WORLD.e0, WORLD.brass_beta  # the manifest's M2 literals ("30", the default "1")
SYSTEM = rng.system_id("M2.demography")
PURPOSE = rng.purpose_id("death_time")
PERSON_KIND = 1  # rh_ids::EntityKind::Person
TAG_RESAMPLE_ENTITY = rng.tag("rehoboam/rh-core/resample-entity/v1")
RHO_DIGITS = 9
MIN_PAIRS = 2  # a correlation needs two pairs; the protocol's MIN_UNAFFECTED is the real bound


# ----------------------------------------------------------------------------- the keyed draws
class Keys:
    """M2's keyed draws in one world on the root timeline, the cascade's keys cached."""

    def __init__(self, seed: int) -> None:
        """Keys of world ``seed``."""
        self.seed = seed
        self.k1: dict[int, bytes] = {}
        self.k2: dict[tuple[int, int], bytes] = {}

    def _k2(self, epoch: int, entity: int) -> bytes:
        if epoch not in self.k1:
            self.k1[epoch] = rng.prf(rng.seed_key(self.seed), 0, SYSTEM | (epoch << 32))
        if (epoch, entity) not in self.k2:
            self.k2[(epoch, entity)] = rng.prf(self.k1[epoch], PURPOSE, entity)
        return self.k2[(epoch, entity)]

    def step(self, epoch: int, person: int, t: int, k: int) -> int:
        """A StepCtx draw: entity = the PersonId, keyed by the person's epoch."""
        return int(rng.stream_u64(self._k2(epoch, person), t, k))

    def resample(self, epoch: int, person: int, t: int, k: int) -> int:
        """A ResampleCtx draw: entity = resample_entity_id(person), the epoch after the input."""
        entity = rng.h64_words(TAG_RESAMPLE_ENTITY, [PERSON_KIND, person])
        return int(rng.stream_u64(self._k2(epoch, entity), t, k))


class Mortality:
    """MR9-MR12 on the compiled F1 schedule, with a bisection in place of the reference's scan.

    ``ell`` is the reference's formula on the same knots; ``check`` compares the two on a sample.
    """

    def __init__(self) -> None:
        """Compile the schedule of the manifest's e0 and beta."""
        self.sched = mort.compile_schedule(E0, BETA)
        self.ages: list[int] = self.sched.ages(YEAR)
        self.knot_l: list[int] = self.sched.knot_l

    def ell(self, x: int) -> int:
        """MR9's l(x) for an age of x minutes: linear between knots, 0 from 120."""
        assert x >= 0
        if x >= self.ages[-1]:
            return 0
        i = bisect_right(self.ages, x) - 1
        lo, hi = self.knot_l[i], self.knot_l[i + 1]
        return int(mort.rhe((hi - lo) * (x - self.ages[i]), self.ages[i + 1] - self.ages[i])) + lo

    def month_q(self, x: int) -> int:
        """MR10's Prob32 of dying within the month from age x (the density modifier is 1)."""
        now = self.ell(x)
        return int(mort.ONE if now == 0 else mort.rhe(mort.ONE * (now - self.ell(x + MONTH)), now))

    def resample_q(self, x0: int, x1: int) -> int:
        """MR12's S11 conditional for an adult, from the ages at the input and the month's end."""
        l0, l1 = self.ell(x0), self.ell(x1)
        return int(mort.ONE if l0 == 0 else mort.rhe(mort.ONE * (l0 - l1), l0))

    def check(self, sample: list[int]) -> list[str]:
        """The bisection equals the reference's ell and month_q on every sampled age."""
        f = []
        for x in sample:
            if self.ell(x) != mort.ell(self.sched, x, YEAR):
                f.append(f"ell differs from the reference at age {x}")
            if self.month_q(x) != mort.month_q(self.sched, x, MONTH, YEAR):
                f.append(f"month_q differs from the reference at age {x}")
            x0 = x + 14 * DAY
            ref = mort.resample_q(self.sched, (x, x0, x + MONTH), YEAR, newborn=False)
            if self.resample_q(x0, x + MONTH) != ref:
                f.append(f"resample_q differs from the reference at age {x}")
        return f


# ----------------------------------------------------------------------------- one rollout
@dataclass
class Rollout:
    """One run of the M2-only model: who dies when, and what the input did."""

    births: dict[int, int]  # PersonId -> birth tick of everyone alive at tick 0
    inputs: dict[int, list[int]]  # tick -> persons whose epoch the input bumps, in order
    resampled: set[int] = field(default_factory=set)  # the persons S11 re-samples (the target)
    deaths: dict[int, int] = field(default_factory=dict)
    epoch: dict[int, int] = field(default_factory=dict)
    pending: dict[int, int] = field(default_factory=dict)
    alive: set[int] = field(default_factory=set)
    input_log: list[str] = field(default_factory=list)

    def input_on(self, keys: Keys, m: Mortality, t: int, p: int) -> None:
        """A scope-only input on p at t: the epoch, then S11 on a pending death (ADR-0025 3)."""
        self.epoch[p] += 1
        tau = self.pending.get(p)
        if p not in self.resampled:
            return  # the control's epoch bumps: no re-sample of these persons' timers
        if p not in self.alive or tau is None:
            self.input_log.append(f"epoch {self.epoch[p] - 1}->{self.epoch[p]} pending none")
            return
        del self.pending[p]
        t_m = t // MONTH * MONTH
        q = m.resample_q(t - self.births[p], t_m + MONTH - self.births[p])
        if not mort.hits(keys.resample(self.epoch[p], p, t, 0), q):
            self.input_log.append(f"epoch 0->1 pending {tau} resample miss")
            return
        new = mort.resample_day(t, t_m + MONTH, keys.resample(self.epoch[p], p, t, 1))
        self.input_log.append(f"epoch 0->1 pending {tau} resample {'none' if new is None else new}")
        if new is not None:
            self.pending[p] = new

    def pulse(self, keys: Keys, m: Mortality, t: int) -> None:
        """S8-S10 at the Month pulse t: one Bernoulli per living person, ascending id."""
        for p in sorted(self.alive):
            assert p not in self.pending, "a pending death survived its month"
            q = m.month_q(t - self.births[p])
            if mort.hits(keys.step(self.epoch[p], p, t, 0), q):
                self.pending[p] = mort.death_day(t, MONTH, keys.step(self.epoch[p], p, t, 1))

    def run(self, seed: int, m: Mortality) -> None:
        """Pulses, timers and inputs in tick order up to the horizon's end."""
        keys = Keys(seed)
        self.epoch = dict.fromkeys(self.births, 0)
        self.alive = set(self.births)
        pulses = {k * MONTH for k in range(HORIZON_YEARS * 12)}
        instants = sorted(pulses | set(self.inputs))
        while instants:
            t = instants.pop(0)
            for p in self.inputs.get(t, []):
                self.input_on(keys, m, t, p)
            for p in sorted(q for q, tau in self.pending.items() if tau == t):
                del self.pending[p]
                self.alive.discard(p)
                self.deaths[p] = t
            if t in pulses:
                self.pulse(keys, m, t)
            nxt = [tau for tau in self.pending.values() if tau > t and tau not in instants]
            instants = sorted(set(instants) | set(nxt))
        assert not self.pending or min(self.pending.values()) >= T_END

    def alive_at(self, t: int) -> set[int]:
        """Everyone alive at t: born by then and not dead before or at t."""
        return {p for p in self.births if self.deaths.get(p, T_END + 1) > t}


# ----------------------------------------------------------------------------- the affected set
def affected(g: Genesis, target: int) -> dict[str, set[int]]:
    """The protocol's affected set S around the target, by component (companion §3.3)."""
    gen = ps.Genealogy(g.persons, 0)
    kin = fw.kin_within(gen, target, KIN_DEGREES) | {target}
    house_id = g.persons[target].house
    house = {pid for h in g.houses if h.id == house_id for pid in h.members}
    realm = [h for h in g.houses if h.k < len(WORLD.polities[0].dynasties)]  # the first polity's
    named = {pid for h in realm for pid in fw.house_report(g, gen, h).named}
    core = kin | house | named
    close: set[int] = set()
    for pid in core:
        p = g.persons[pid]
        if p.spouse:
            close.add(p.spouse)
        if p.household in g.households:
            close |= {q.id for q in g.persons.values() if q.household == p.household and q.alive}
    return {"kin": kin, "house": house, "named_realm": named, "households": close - core}


# ----------------------------------------------------------------------------- the statistic
@dataclass
class Statistic:
    """Pearson's correlation of paired, right-censored death instants, exact in rationals."""

    pairs: int
    identical: int
    rho2: Fraction | None  # None when a run's instants have no variance
    positive: bool

    def passes(self) -> bool:
        """Pass iff rho >= 0.9: a positive covariance and rho^2 >= 0.81."""
        return self.rho2 is not None and self.positive and self.rho2 >= THRESHOLD * THRESHOLD

    def rho2_text(self) -> str:
        """rho^2 rounded down to RHO_DIGITS decimals, or 'undefined'."""
        if self.rho2 is None:
            return "undefined"
        scaled = (self.rho2 * 10**RHO_DIGITS).numerator // (self.rho2 * 10**RHO_DIGITS).denominator
        return f"{scaled // 10**RHO_DIGITS}.{scaled % 10**RHO_DIGITS:0{RHO_DIGITS}d}"


def correlate(a: Rollout, b: Rollout, unaffected: set[int]) -> Statistic:
    """Pair each unaffected person's death instants, censor at T_END, drop both-censored pairs."""
    xs, ys = [], []
    for p in sorted(unaffected):
        x, y = min(a.deaths.get(p, T_END), T_END), min(b.deaths.get(p, T_END), T_END)
        if x == T_END and y == T_END:
            continue
        xs.append(x)
        ys.append(y)
    n = len(xs)
    identical = sum(1 for x, y in zip(xs, ys, strict=True) if x == y)
    if n < MIN_PAIRS:
        return Statistic(n, identical, None, False)
    sx, sy = sum(xs), sum(ys)
    cov = n * sum(x * y for x, y in zip(xs, ys, strict=True)) - sx * sy
    vx = n * sum(x * x for x in xs) - sx * sx
    vy = n * sum(y * y for y in ys) - sy * sy
    if vx == 0 or vy == 0:
        return Statistic(n, identical, None, False)
    return Statistic(n, identical, Fraction(cov * cov, vx * vy), cov > 0)


# ----------------------------------------------------------------------------- one seed
def digest(deaths: dict[int, int]) -> str:
    """SHA-256 over 'id tick' lines in ascending id."""
    text = "\n".join(f"{p} {t}" for p, t in sorted(deaths.items())) + "\n"
    return hashlib.sha256(text.encode("ascii")).hexdigest()


def render_seed(seed: int, m: Mortality, control: str | None) -> tuple[list[str], list[str]]:
    """Build the world, run the pair, correlate; return the golden lines and the failures."""
    g = ps.build(ps.Manifest(**{**WORLD.__dict__, "seed": seed}))
    births = {p.id: p.birth for p in g.persons.values() if p.alive}
    title = next(t for t in g.titles if t.k == 0)
    target = title.holder
    assert title.tier == 0 and g.persons[target].house == g.houses[0].id, "the sovereign of house 0"
    bumped = sorted(births) if control == "global-epoch" else [target]
    a = Rollout(births, {})
    b = Rollout(births, {T_IN: bumped}, resampled={target})
    a.run(seed, m)
    b.run(seed, m)
    parts = affected(g, target)
    s_all = set().union(*parts.values())
    both = a.alive_at(T_IN) & b.alive_at(T_IN)
    unaffected = both - s_all
    stat = correlate(a, b, unaffected)
    failures = []
    if len(unaffected) < MIN_UNAFFECTED:
        failures.append(
            f"seed {seed}: {len(unaffected)} unaffected persons, under {MIN_UNAFFECTED}"
        )
    if not stat.passes():
        failures.append(f"seed {seed}: rho^2 {stat.rho2_text()} is under 0.81 or not positive")
    tgt = g.persons[target]

    def tick(r: Rollout) -> str:
        return str(r.deaths[target]) if target in r.deaths else "censored"

    lines = [
        f"world {WORLD.id} {seed} horizon_years {HORIZON_YEARS} t_end {T_END}",
        f"target founder {tgt.k} id {tgt.id} age_years {-tgt.birth // YEAR} title {title.k} "
        f"law {title.law}",
        f"input tick {T_IN} kind scope_only founder {tgt.k} {b.input_log[0]}",
        f"affected kin{KIN_DEGREES} {len(parts['kin'])} house {len(parts['house'])} "
        f"named_realm {len(parts['named_realm'])} households {len(parts['households'])} "
        f"total {len(s_all)}",
        f"living_at_input A {len(a.alive_at(T_IN))} B {len(b.alive_at(T_IN))} "
        f"unaffected {len(unaffected)}",
        f"deaths A {len(a.deaths)} B {len(b.deaths)} pairs {stat.pairs} identical {stat.identical} "
        f"rho2 {stat.rho2_text()} rho_at_least_0.9 {'yes' if stat.passes() else 'no'}",
        f"target_death A {tick(a)} B {tick(b)}",
        f"deaths_sha256 A {digest(a.deaths)} B {digest(b.deaths)}",
    ]
    return lines, failures


HEADER = [
    "# CRN pairing golden outputs (SPR-030; V-DET-17): GENERATED by",
    "# python -I specs/determinism/selftest_crn_pairing.py --write; do not edit.",
    "# Per seed of twin_vales: the target (the primogeniture sovereign at genesis), the one",
    "# scope-only input and what S11 did with the target's pending death; the affected set by",
    "# component; the living at the input and the unaffected; the paired deaths, the identical",
    "# pairs and rho^2 (exact rationals, rounded down); the target's death in each run; the",
    "# digests of both runs' death instants.",
    f"constants crn-pairing-v0 horizon_years {HORIZON_YEARS} t_in {T_IN} kin_degrees {KIN_DEGREES}"
    f" threshold {THRESHOLD} min_unaffected {MIN_UNAFFECTED} e0 {E0} beta {BETA}",
]


def render(control: str | None) -> tuple[str, list[str]]:
    """Render every seed; the model's own checks first."""
    m = Mortality()
    failures = m.check(
        [0, DAY, MONTH - DAY, YEAR, 16 * YEAR + 3 * DAY, 61 * YEAR + 7 * DAY, 119 * YEAR]
    )
    lines = HEADER[:]
    for seed in SEEDS if control is None else SEEDS[:1]:  # a control needs one seed to fail
        seed_lines, seed_failures = render_seed(seed, m, control)
        lines += seed_lines
        failures += seed_failures
    return "\n".join(lines) + "\n", failures


def first_difference(current: str, text: str) -> int:
    """Return the 1-based line of the first difference between two texts."""
    a, b = current.splitlines(), text.splitlines()
    pairs = enumerate(zip(a, b, strict=False))
    return next((i for i, (x, z) in pairs if x != z), min(len(a), len(b))) + 1


def main(argv: list[str] | None = None) -> int:
    """Check or regenerate the golden file; run a control that must fail."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--write", action="store_true", help="regenerate the golden file")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the golden file to check")
    parser.add_argument("--control", choices=["global-epoch"], help="a control that must fail")
    args = parser.parse_args(argv)
    text, failures = render(args.control)
    print(text, end="")
    model = [f for f in failures if "differs from the reference" in f]
    if model:
        for f in model:
            print(f"MODEL FAILED: {f}")
        print("CRN PAIRING: FAIL (the model's own checks)")
        return EXIT_MODEL
    if failures:
        for f in failures:
            print(f"PROTOCOL FAILED: {f}")
        print("CRN PAIRING: FAIL")
        return EXIT_FAIL
    if args.control:
        print("CRN PAIRING: PASS under a control that must fail")
        return EXIT_FAIL
    if args.write:
        args.golden.parent.mkdir(parents=True, exist_ok=True)
        args.golden.write_text(text, encoding="utf-8")
        print(f"wrote {args.golden}")
        return EXIT_OK
    current = args.golden.read_text(encoding="utf-8") if args.golden.exists() else ""
    if current != text:
        line = first_difference(current, text)
        print(f"STALE: {args.golden} differs from the reference at line {line}")
        print("CRN PAIRING: FAIL")
        return EXIT_FAIL
    print(f"CRN PAIRING: PASS ({len(SEEDS)} seeds, rho >= {THRESHOLD} on the unaffected)")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
