"""V-DET-17 CRN pairing: the worked computation of ``specs/determinism/crn_pairing_v0.md``.

The protocol pairs two rollouts of the same world (manifest hash, seed, Registry, build and thread
count equal) whose input journals differ by one journaled material input: a scope-only input on one
person (the ``[[input]] tick, founder`` form of ``worlds/*/inputs.toml``), which bumps that
person's RNG epoch and cancels and re-samples its pending death (ADR-0025 item 3; M2 S11). The
input is placed inside the month of the target's death in the run without it, before the death
day, as ``worlds/solo/inputs.toml`` places Ada's: so the re-sample path runs, and the pair is
VALID only if the target is alive at the input in both runs, dies at different instants, and at
least one of those instants is inside the horizon (REV-SPR-030 R3). Over the 10-year horizon the
protocol then compares the death instants of everyone the input cannot reach: under the keyed RNG
H(world_seed, salt, system, epoch, purpose, entity, t, k) with a per-entity epoch, every draw of
every other entity is the same bytes in both runs, so their deaths coincide exactly. That exact
identity is the gate (R4); docs/09's correlation of at least 0.9 is reported beside it.

This file is the protocol's exact worked computation on the dynastic fixture world
``twin_vales`` (``specs/manifest/examples/twin_vales.yaml``), on an M2-only model of the engine,
every piece of which is a frozen or next-version reference already on `main`:

  * genesis by the SPR-019 reference, through ``specs/worldgen/selftest_fixture_worlds.py``;
  * M2 F1 mortality MR9-MR13 and S8-S11 by ``specs/demography/selftest_m2_f1_mortality.py``
    (the schedule of e0 30 and beta 1; the density modifier is 1 below the default capacity);
  * the keyed draws by ``bin/gen_rng_golden.py`` with the entity and epoch conventions of
    ``bin/gen_m2_draws_reference.py`` (StepCtx: entity = PersonId; ResampleCtx: entity =
    H64(resample-entity/v1, [Person, id]), keyed by the epoch after the input).

No births (a child born before the input exists identically in both runs; one born after it is
not alive at the input in both runs, so the protocol pairs it by PersonId among the partnering
events, which this model does not run), no M7 and no T1 (their deaths fall on sovereigns and
named actors, who are in the affected set by construction: companion §3.3). The controls that
must fail (R3, R4) run in the same process on the one run A, from its state at the input.

The tiers (the A-62 review's E4: D56 (e) moved the genesis and left one VALID seed of 1 to 3, so
the redundancy is restored by searching): the nightly tier tries seeds 1, 2, ... in order until
NIGHTLY_VALID of them are VALID, up to seed NIGHTLY_CAP, and fails with fewer; the PR tier takes the
first VALID seed of PR_SEEDS and fails with none. The golden pins every seed the nightly search
tries (`--write` renders it), so a VALID seed that turns skipped, or the reverse, is STALE.

Run: python -I specs/determinism/selftest_crn_pairing.py                 (the nightly tier)
     python -I specs/determinism/selftest_crn_pairing.py --tier pr       (the PR tier)
     python -I specs/determinism/selftest_crn_pairing.py --write         (regenerate the golden)
     python -I specs/determinism/selftest_crn_pairing.py --golden PATH
     python -I specs/determinism/selftest_crn_pairing.py --seeds 1,2     (exactly these seeds; at
         least one must be VALID)
     python -I specs/determinism/selftest_crn_pairing.py --tier pr --controls   (also run the five
         mutants on every VALID seed of the tier, each of which must make the protocol FAIL:
         no-op-input, swap-remove-after-input, shift-one-day, rekey-2pct, global-epoch)
Exit: 0 pass, 1 fail (the protocol, a stale golden, or a control that did not fail), 2 the model
failed its own checks. Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import sys
from bisect import bisect_right
from copy import deepcopy
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
# The tiers (REV-SPR-030 R10; the A-62 review's E4): the PR tier (check_all) takes the first VALID
# seed of PR_SEEDS; the nightly tier (golden_nightly.sh) searches seeds 1 to NIGHTLY_CAP in order
# until NIGHTLY_VALID are VALID, and fails with fewer.
PR_SEEDS = (1, 2, 3, 4)
NIGHTLY_CAP = 12
NIGHTLY_VALID = 2
DAY = 1440
MONTH_DAYS = 30  # twin_vales.yaml: days_per_year 360, months 12 (the mirror test checks)
MONTHS = 12
MONTH = MONTH_DAYS * DAY
YEAR = MONTHS * MONTH
HORIZON_YEARS = 10  # V-DET-17: paired rollouts over 10 years
T_END = HORIZON_YEARS * YEAR  # 5,184,000: deaths at or after it are censored
INPUT_OFFSET = 6 * 60 + 7  # solo's convention: the input at 06:07 of its day, never a day start
THRESHOLD = Fraction(9, 10)  # docs/09 V-DET-17's floor on the correlation, reported
MIN_UNAFFECTED = 100  # the protocol needs at least this many unaffected persons alive at the input
MIN_WINDOW_YEARS = 5  # R14: a VALID pair has at least this many sim-years after the input ...
MIN_INFORMATIVE = 100  # ... and at least this many informative (not both-censored) pairs in U
MIN_WINDOW = MIN_WINDOW_YEARS * 12 * MONTH_DAYS * DAY
E0, BETA = WORLD.e0, WORLD.brass_beta  # the manifest's M2 literals ("30", the default "1")
SYSTEM = rng.system_id("M2.demography")
PURPOSE = rng.purpose_id("death_time")
PERSON_KIND = 1  # rh_ids::EntityKind::Person
TAG_RESAMPLE_ENTITY = rng.tag("rehoboam/rh-core/resample-entity/v1")
RHO_DIGITS = 9
MIN_PAIRS = 2  # a correlation needs two pairs; the protocol's MIN_UNAFFECTED is the real bound
REKEY_EVERY = 50  # the rekey-2pct mutant re-keys every 50th unaffected person
# Each mutant and the reason the protocol must give for it (R14: a control that fails for another
# reason is a control that did not test what it claims).
CONTROLS = {
    "no-op-input": "death instants are equal",
    "swap-remove-after-input": "pairs differ",
    "shift-one-day": "pairs differ",
    "rekey-2pct": "pairs differ",
    "global-epoch": "pairs differ",
}


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

    def step(self, epoch: int, entity: int, t: int, k: int) -> int:
        """A StepCtx draw: entity = the PersonId, keyed by the person's epoch."""
        return int(rng.stream_u64(self._k2(epoch, entity), t, k))

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
    keying: str = "entity"  # "entity": H keyed by PersonId; "position": the swap-remove mutant
    deaths: dict[int, int] = field(default_factory=dict)
    epoch: dict[int, int] = field(default_factory=dict)
    pending: dict[int, int] = field(default_factory=dict)
    alive: set[int] = field(default_factory=set)
    order: list[int] = field(default_factory=list)  # the living vector of the position mutant
    t: int = 0  # the next instant to run
    input_log: str = ""

    def start(self) -> None:
        """Tick 0: everyone alive at epoch 0, the vector in ascending id."""
        self.epoch = dict.fromkeys(self.births, 0)
        self.alive = set(self.births)
        self.order = sorted(self.births)

    def input_on(self, keys: Keys, m: Mortality, t: int, p: int) -> None:
        """A scope-only input on p at t: the epoch, then S11 on a pending death (ADR-0025 3)."""
        self.epoch[p] += 1
        tau = self.pending.get(p)
        if p not in self.alive or tau is None:
            self.input_log = f"epoch {self.epoch[p] - 1}->{self.epoch[p]} pending none"
            return
        del self.pending[p]
        t_m = t // MONTH * MONTH
        q = m.resample_q(t - self.births[p], t_m + MONTH - self.births[p])
        if not mort.hits(keys.resample(self.epoch[p], p, t, 0), q):
            self.input_log = f"epoch 0->1 pending {tau} resample miss"
            return
        new = mort.resample_day(t, t_m + MONTH, keys.resample(self.epoch[p], p, t, 1))
        self.input_log = f"epoch 0->1 pending {tau} resample {'none' if new is None else new}"
        if new is not None:
            self.pending[p] = new

    def pulse(self, keys: Keys, m: Mortality, t: int) -> None:
        """S8-S10 at the Month pulse t: one Bernoulli per living person, ascending id."""
        for i, p in enumerate(self.order if self.keying == "position" else sorted(self.alive)):
            assert p not in self.pending, "a pending death survived its month"
            entity = i + 1 if self.keying == "position" else p
            q = m.month_q(t - self.births[p])
            if mort.hits(keys.step(self.epoch[p], entity, t, 0), q):
                tau = mort.death_day(t, MONTH, keys.step(self.epoch[p], entity, t, 1))
                self.pending[p] = tau

    def die(self, p: int, t: int) -> None:
        """A death timer fires: the ledger row leaves; the position mutant swap-removes it."""
        del self.pending[p]
        self.alive.discard(p)
        self.deaths[p] = t
        if self.keying == "position":
            i = self.order.index(p)
            self.order[i] = self.order[-1]
            self.order.pop()

    def run(
        self,
        keys: Keys,
        m: Mortality,
        until: int,
        inputs: dict[int, list[int]],
        snapshots: dict[int, Rollout] | None = None,
    ) -> None:
        """Pulses, timers and inputs in tick order from self.t up to (excluding) ``until``.

        With ``snapshots``, the state at the start of every Month pulse is copied into it (R27:
        run A is snapshotted, and the state at the input is the snapshot before it run forward).
        """
        pulses = {k * MONTH for k in range(HORIZON_YEARS * MONTHS)}
        instants = sorted({x for x in pulses | set(inputs) if self.t <= x < until})
        instants = sorted(set(instants) | {tau for tau in self.pending.values() if tau < until})
        while instants:
            t = instants.pop(0)
            if snapshots is not None and t in pulses:
                self.t = t
                snapshots[t] = deepcopy(self)
            for p in inputs.get(t, []):
                self.input_on(keys, m, t, p)
            for p in sorted(q for q, tau in self.pending.items() if tau == t):
                self.die(p, t)
            if t in pulses:
                self.pulse(keys, m, t)
            nxt = [tau for tau in self.pending.values() if t < tau < until and tau not in instants]
            instants = sorted(set(instants) | set(nxt))
        self.t = until

    def alive_at(self, t: int) -> set[int]:
        """Everyone alive at t: born by then and not dead before or at t."""
        return {p for p in self.births if self.deaths.get(p, T_END + 1) > t}


def continue_run(
    state: Rollout, keys: Keys, m: Mortality, inputs: dict[int, list[int]], *, bump: list[int]
) -> Rollout:
    """Copy a rollout at its instant, bump the epochs of ``bump`` there, and run to the horizon."""
    r = deepcopy(state)
    for p in bump:
        r.epoch[p] += 1
    r.run(keys, m, T_END, inputs)
    return r


def rerun_subset(
    done: Rollout, state: Rollout, keys: Keys, m: Mortality, bump: list[int]
) -> Rollout:
    """``done`` with the persons of ``bump`` re-run from ``state`` at one epoch more.

    Exact under entity keying: a person's draws depend on nothing but its own state, so only the
    re-keyed persons' instants can change (the model's own independence, which the global-epoch
    mutant re-runs in full).
    """
    part = deepcopy(state)
    part.births = {p: state.births[p] for p in bump}
    part.alive &= set(bump)
    part.pending = {p: tau for p, tau in part.pending.items() if p in bump}
    part.deaths = {p: t for p, t in part.deaths.items() if p in bump}
    for p in bump:
        part.epoch[p] += 1
    part.run(keys, m, T_END, {})
    r = deepcopy(done)
    for p in bump:
        r.deaths.pop(p, None)
        if p in part.deaths:
            r.deaths[p] = part.deaths[p]
    return r


# ----------------------------------------------------------------------------- the affected set
def affected(g: Genesis, target: int, title: Any) -> dict[str, set[int]]:  # noqa: ANN401
    """The protocol's affected set S around the target, by component (companion §3.3)."""
    gen = ps.Genealogy(g.persons, 0)
    line = set(ps.Law(gen, target, title.tenures, g.majority).line_for(title.law))
    house_id = g.persons[target].house
    house = {pid for h in g.houses if h.id == house_id for pid in h.members}
    realm = realm_houses(g, title)
    named = {pid for h in realm for pid in fw.house_report(g, gen, h, fw.DORMANCY_DEGREES).full}
    realm_members = {pid for h in realm for pid in h.members}  # R16: every member of every house
    core = {target} | line | house | named | realm_members
    close: set[int] = set()
    for pid in core:
        p = g.persons[pid]
        if p.spouse:
            close.add(p.spouse)
        if p.household in g.households:
            close |= {q.id for q in g.persons.values() if q.household == p.household and q.alive}
    return {
        "line": line,
        "house": house,
        "named_realm": named,
        "realm_members": realm_members,
        "households": close - core,
    }


def realm_houses(g: Genesis, title: Any) -> list[Any]:  # noqa: ANN401
    """The houses of the title's polity: the manifest lists dynasties polity by polity, in order."""
    first = 0
    for polity in WORLD.polities:
        n = len(polity.dynasties)
        houses = [h for h in g.houses if first <= h.k < first + n]
        if any(h.title == title.id for h in houses):
            return houses
        first += n
    raise AssertionError("a title outside every polity")


# ----------------------------------------------------------------------------- the statistic
@dataclass
class Verdict:
    """The protocol's outcome on one pair: validity, the exact-CRN check, the correlation."""

    valid: bool
    reason: str  # why the pair is INVALID, or ""
    unaffected: int
    pairs: int
    identical: int
    rho2: Fraction | None  # None when a run's instants have no variance
    positive: bool

    def exact(self) -> bool:
        """Every unaffected pair has identical death instants (the gate, REV-SPR-030 R4)."""
        return self.identical == self.pairs

    def rho_ok(self) -> bool:
        """Pass iff rho >= 0.9: a positive covariance and rho^2 >= 0.81 (docs/09's floor)."""
        return self.rho2 is not None and self.positive and self.rho2 >= THRESHOLD * THRESHOLD

    def rho2_text(self) -> str:
        """rho^2 rounded down to RHO_DIGITS decimals, or 'undefined'."""
        if self.rho2 is None:
            return "undefined"
        scaled = (self.rho2 * 10**RHO_DIGITS).numerator // (self.rho2 * 10**RHO_DIGITS).denominator
        return f"{scaled // 10**RHO_DIGITS}.{scaled % 10**RHO_DIGITS:0{RHO_DIGITS}d}"

    def failures(self, label: str) -> list[str]:
        """The protocol's failures on this pair, prefixed by ``label``."""
        f = []
        if not self.valid:
            f.append(f"{label}: INVALID pair: {self.reason}")
        if self.unaffected < MIN_UNAFFECTED:
            f.append(f"{label}: {self.unaffected} unaffected persons, under {MIN_UNAFFECTED}")
        if not self.exact():
            f.append(f"{label}: {self.pairs - self.identical} of {self.pairs} pairs differ")
        if not self.rho_ok():
            f.append(f"{label}: rho^2 {self.rho2_text()} is under 0.81 or not positive")
        return f


def correlate(xs: list[int], ys: list[int]) -> tuple[Fraction | None, bool]:
    """Pearson's rho^2 of paired instants, exact in rationals, and the covariance's sign."""
    n = len(xs)
    if n < MIN_PAIRS:
        return None, False
    sx, sy = sum(xs), sum(ys)
    cov = n * sum(x * y for x, y in zip(xs, ys, strict=True)) - sx * sy
    vx = n * sum(x * x for x in xs) - sx * sx
    vy = n * sum(y * y for y in ys) - sy * sy
    if vx == 0 or vy == 0:
        return None, False
    return Fraction(cov * cov, vx * vy), cov > 0


def evaluate(a: Rollout, b: Rollout, s_all: set[int], target: int, t_in: int) -> Verdict:
    """Validity (R3), then the pairs over U: censor at T_END, drop both-censored, compare."""
    ta, tb = a.deaths.get(target), b.deaths.get(target)
    if target not in a.alive_at(t_in) or target not in b.alive_at(t_in):
        valid, reason = False, "the target is not alive at the input in both runs"
    elif ta == tb:
        valid, reason = False, f"the target's death instants are equal ({ta})"
    elif (ta is None or ta >= T_END) and (tb is None or tb >= T_END):
        valid, reason = False, "neither death instant of the target is inside the horizon"
    elif T_END - t_in < MIN_WINDOW:
        valid, reason = False, f"the window after the input is under {MIN_WINDOW_YEARS} years"
    else:
        valid, reason = True, ""
    unaffected = (a.alive_at(t_in) & b.alive_at(t_in)) - s_all
    xs, ys = [], []
    for p in sorted(unaffected):
        x, y = min(a.deaths.get(p, T_END), T_END), min(b.deaths.get(p, T_END), T_END)
        if x == T_END and y == T_END:
            continue
        xs.append(x)
        ys.append(y)
    identical = sum(1 for x, y in zip(xs, ys, strict=True) if x == y)
    if valid and len(xs) < MIN_INFORMATIVE:
        valid, reason = False, f"{len(xs)} informative pairs, under {MIN_INFORMATIVE}"
    rho2, positive = correlate(xs, ys)
    return Verdict(valid, reason, len(unaffected), len(xs), identical, rho2, positive)


# ----------------------------------------------------------------------------- one seed
@dataclass
class Pair:
    """Run A, its state at the input, run B, and the protocol's sets for one seed."""

    g: Genesis
    target: int
    title: Any
    t_in: int
    a: Rollout
    at_input: Rollout  # run A's state at t_in, the controls' common starting point
    b: Rollout
    parts: dict[str, set[int]]
    verdict: Verdict

    @property
    def s_all(self) -> set[int]:
        """The affected set S."""
        return set().union(*self.parts.values())


def digest(deaths: dict[int, int]) -> str:
    """SHA-256 over 'id tick' lines in ascending id."""
    text = "\n".join(f"{p} {t}" for p, t in sorted(deaths.items())) + "\n"
    return hashlib.sha256(text.encode("ascii")).hexdigest()


def choose_target(g: Genesis, a: Rollout) -> tuple[Any, list[str]]:
    """R14, R25: the first sovereign whose run-A death leaves the window and the pairs floor.

    Primogeniture first, then tanistry; the pairs floor is read from run A's unaffected deaths
    after the input, a lower bound of the pair's informative pairs.

    Returns the title, or None when no sovereign qualifies (the seed is then SKIPPED and the
    protocol moves to the next seed), and the log of every candidate considered. A run-A death
    that is not M2's (a T1 DEATH_DUE in the engine) disqualifies a candidate the same way: the
    input re-samples M2's own timer only (crn_pairing_v0.md §3.2); in this M2-only model every
    death is M2's.
    """
    log = []
    chosen = None
    for title in sorted((t for t in g.titles if t.tier == 0), key=lambda t: t.k):
        tau = a.deaths.get(title.holder)
        if tau is None:
            log.append(
                f"candidate title {title.k} {title.law} founder {g.persons[title.holder].k} "
                f"runA_death_year none rejected"
            )
            continue
        t_in = tau - DAY + INPUT_OFFSET
        window = T_END - t_in
        s_all = set().union(*affected(g, title.holder, title).values())
        informative = sum(
            1 for p in a.alive_at(t_in) - s_all if t_in < a.deaths.get(p, T_END + 1) < T_END
        )
        ok = window >= MIN_WINDOW and informative >= MIN_INFORMATIVE
        log.append(
            f"candidate title {title.k} {title.law} founder {g.persons[title.holder].k} "
            f"runA_death_year {tau / YEAR:.2f} runA_informative {informative} "
            f"{'chosen' if ok and chosen is None else 'rejected'}"
        )
        if ok and chosen is None:
            chosen = title
    return chosen, log


def run_pair(seed: int, m: Mortality) -> Pair | None:
    """Genesis, run A to the horizon, the target and input from run A, run B; None if skipped."""
    g = ps.build(ps.Manifest(**{**WORLD.__dict__, "seed": seed}))
    births = {p.id: p.birth for p in g.persons.values() if p.alive}
    keys = Keys(seed)
    a = Rollout(births)
    a.start()
    snapshots: dict[int, Rollout] = {}
    a.run(keys, m, T_END, {}, snapshots=snapshots)
    title, log = choose_target(g, a)
    SKIPPED[seed] = log
    if title is None:
        SKIPPED[seed].append(f"deaths_sha256 A {digest(a.deaths)}")  # R25: run A is pinned
        return None
    target = title.holder
    tau = a.deaths[target]
    # The input: the day before the target's run-A death, 06:07 (inside that death's month, after
    # its Month pulse since death days start at the pulse's day + 1).
    t_in = tau - DAY + INPUT_OFFSET
    # The state at the input: run A's snapshot at that month's pulse, run forward (R27).
    at_input = deepcopy(snapshots[t_in // MONTH * MONTH])
    at_input.run(keys, m, t_in, {})
    before = {p: t for p, t in a.deaths.items() if t < t_in}
    assert at_input.deaths == before, "run A to the input equals run A's deaths before it"
    b = continue_run(at_input, keys, m, {t_in: [target]}, bump=[])
    parts = affected(g, target, title)
    s_all = set().union(*parts.values())
    return Pair(g, target, title, t_in, a, at_input, b, parts, evaluate(a, b, s_all, target, t_in))


SKIPPED: dict[int, list[str]] = {}  # per seed, the candidate log of choose_target (rendered)


def render_seed(seed: int, pair: Pair) -> list[str]:
    """The golden lines of one seed."""
    g, a, b, v = pair.g, pair.a, pair.b, pair.verdict
    tgt = g.persons[pair.target]
    parts = pair.parts
    s_all = pair.s_all
    alive_in_s = len(s_all & a.alive_at(pair.t_in))

    def tick(r: Rollout) -> str:
        return str(r.deaths[pair.target]) if pair.target in r.deaths else "censored"

    return [
        f"world {WORLD.id} {seed} horizon_years {HORIZON_YEARS} t_end {T_END}",
        *SKIPPED[seed],
        f"target founder {tgt.k} id {tgt.id} age_years {-tgt.birth // YEAR} title {pair.title.k} "
        f"law {pair.title.law}",
        f"input tick {pair.t_in} kind scope_only founder {tgt.k} {b.input_log} "
        f"window_years {(T_END - pair.t_in) / YEAR:.2f}",
        f"affected line {len(parts['line'])} house {len(parts['house'])} "
        f"named_realm {len(parts['named_realm'])} realm_members {len(parts['realm_members'])} "
        f"households {len(parts['households'])} total {len(s_all)} alive_at_input {alive_in_s}",
        f"living_at_input A {len(a.alive_at(pair.t_in))} B {len(b.alive_at(pair.t_in))} "
        f"unaffected {v.unaffected}",
        f"pair {'VALID' if v.valid else 'INVALID'} deaths A {len(a.deaths)} B {len(b.deaths)} "
        f"pairs {v.pairs} identical {v.identical} rho2 {v.rho2_text()} "
        f"rho_at_least_0.9 {'yes' if v.rho_ok() else 'no'}",
        f"target_death A {tick(a)} B {tick(b)}",
        f"deaths_sha256 A {digest(a.deaths)} B {digest(b.deaths)}",
    ]


# ----------------------------------------------------------------------------- the controls
def mutant(name: str, pair: Pair, keys: Keys, m: Mortality) -> tuple[Rollout, Rollout]:
    """The pair (A', B') of one mutant engine, from run A's state at the input."""
    t_in, target, s_all = pair.t_in, pair.target, pair.s_all
    unaffected = sorted((pair.a.alive_at(t_in) & pair.b.alive_at(t_in)) - s_all)
    inputs = {t_in: [target]}
    if name == "no-op-input":  # the input changes nothing: B is A
        return pair.a, pair.a
    if name == "swap-remove-after-input":  # draws keyed by the living vector's position
        base = deepcopy(pair.at_input)
        base.keying = "position"
        base.order = sorted(base.alive)  # R14: the living vector at the input, nobody dead in it
        return continue_run(base, keys, m, {}, bump=[]), continue_run(
            base, keys, m, inputs, bump=[]
        )
    if name == "shift-one-day":  # every unaffected death in B one day later
        b = deepcopy(pair.b)
        for p in unaffected:
            if p in b.deaths:
                b.deaths[p] += DAY
        return pair.a, b
    if name == "rekey-2pct":  # the input re-keys every 50th unaffected person too
        return pair.a, rerun_subset(pair.b, pair.at_input, keys, m, unaffected[::REKEY_EVERY])
    if name == "global-epoch":  # a world-wide epoch counter: everyone re-keyed at the input
        bump = sorted(p for p in pair.at_input.alive if p != target)
        return pair.a, continue_run(pair.at_input, keys, m, inputs, bump=bump)
    raise AssertionError(name)


def run_controls(seed: int, pair: Pair, m: Mortality) -> tuple[list[str], int]:
    """Every mutant must make the protocol fail; return the report lines and the count that did."""
    keys = Keys(seed)
    lines, failed = [], 0
    for name, reason in CONTROLS.items():
        a, b = mutant(name, pair, keys, m)
        v = evaluate(a, b, pair.s_all, pair.target, pair.t_in)
        f = v.failures(f"PROTOCOL FAILED: seed {seed} [{name}]")
        needs_valid = reason == "pairs differ"  # R25: a differing pair is only evidence if VALID
        if not f:
            lines.append(f"CONTROL PASSED: seed {seed} [{name}]: the protocol did not fail")
        elif not any(reason in x for x in f) or (needs_valid and not v.valid):
            lines += f
            lines.append(
                f"CONTROL FAILED FOR ANOTHER REASON: seed {seed} [{name}]: wanted {reason!r}"
                + ("" if v.valid else " on a VALID pair")
            )
        else:
            failed += 1
            lines += f
    return lines, failed


# ----------------------------------------------------------------------------- the golden
HEADER = [
    "# CRN pairing golden outputs (SPR-030; V-DET-17): GENERATED by",
    "# python -I specs/determinism/selftest_crn_pairing.py --write; do not edit.",
    "# Per seed of twin_vales: the sovereign candidates and the chosen target (R14: the",
    "# primogeniture sovereign, else the tanistry one, else the seed is SKIPPED), the one",
    "# scope-only input inside the month of the target's run-A death and what S11 did with the",
    "# pending death; the affected set by component, with how many of it are alive at the input;",
    "# the living at the input and the unaffected; the pair's validity, the deaths, the pairs,",
    "# the identical pairs and rho^2 (exact rationals, rounded down); the target's death in each",
    "# run; the digests of both runs' death instants. The seeds are those the nightly tier",
    "# tries: 1, 2, ... until enough are VALID (the PR tier takes the first VALID of its seeds).",
    f"constants crn-pairing-v0 horizon_years {HORIZON_YEARS} input_offset {INPUT_OFFSET}"
    f" threshold {THRESHOLD} min_unaffected {MIN_UNAFFECTED} e0 {E0} beta {BETA}"
    f" pr_seeds {','.join(map(str, PR_SEEDS))} nightly_valid {NIGHTLY_VALID}"
    f" nightly_cap {NIGHTLY_CAP}",
]


def split_blocks(text: str) -> tuple[str, dict[str, str]]:
    """Split a golden text into its header and its 'world id seed ...' blocks."""
    header: list[str] = []
    blocks: dict[str, str] = {}
    key = ""
    for line in text.splitlines():
        if line.startswith("world "):
            key = line
            blocks[key] = ""
        if key:
            blocks[key] += line + "\n"
        else:
            header.append(line)
    return "\n".join(header) + ("\n" if header else ""), blocks


def first_difference(current: str, text: str) -> int:
    """Return the 1-based line of the first difference between two texts."""
    a, b = current.splitlines(), text.splitlines()
    pairs = enumerate(zip(a, b, strict=False))
    return next((i for i, (x, z) in pairs if x != z), min(len(a), len(b))) + 1


def compare(golden_text: str, blocks: dict[str, str]) -> str | None:
    """Return a STALE line for the first rendered block that differs from the golden, or None."""
    header, golden_blocks = split_blocks(golden_text)
    expected_header = "\n".join(HEADER) + "\n"
    if header != expected_header:
        return f"STALE: header differs at line {first_difference(header, expected_header)}"
    for key, text in blocks.items():
        current = golden_blocks.get(key, "")
        if current != text:
            offset = golden_text.find(key)
            base = golden_text[:offset].count("\n") if offset >= 0 else 0
            return f"STALE: {key!r} differs at line {base + first_difference(current, text)}"
    return None


def parse(argv: list[str] | None) -> argparse.Namespace:
    """The command line."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--write", action="store_true", help="regenerate the golden file")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the golden file to check")
    parser.add_argument(
        "--tier", choices=("pr", "nightly"), default="nightly", help="the seed search (E4)"
    )
    parser.add_argument("--seeds", help="exactly these seeds, comma-separated (overrides --tier)")
    parser.add_argument("--controls", action="store_true", help="also run the five mutants")
    return parser.parse_args(argv)


def plan(args: argparse.Namespace) -> tuple[str, tuple[int, ...], int, bool]:
    """(the label, the seeds to try in order, the VALID seeds required, stop once that many).

    ``--write`` renders the nightly search, which covers the PR tier's seeds up to its first VALID
    one, so the golden pins every seed either tier tries.
    """
    if args.seeds and not args.write:
        seeds = tuple(int(s) for s in args.seeds.split(","))
        return f"seeds {','.join(map(str, seeds))}", seeds, 1, False
    if args.tier == "pr" and not args.write:
        return "tier pr", PR_SEEDS, 1, True
    return "tier nightly", tuple(range(1, NIGHTLY_CAP + 1)), NIGHTLY_VALID, True


@dataclass
class Search:
    """What a tier's seed search found: the golden blocks, failures and the controls' outcome."""

    seeds: list[int] = field(default_factory=list)  # the seeds tried, in order
    valid: int = 0
    blocks: dict[str, str] = field(default_factory=dict)
    failures: list[str] = field(default_factory=list)
    control_lines: list[str] = field(default_factory=list)
    controls_failed: int = 0


def search(args: argparse.Namespace, m: Mortality) -> tuple[str, Search]:
    """Try the tier's seeds in order (E4), stopping once it has its VALID seeds."""
    label, candidates, required, stop_early = plan(args)
    s = Search()
    for seed in candidates:
        if stop_early and s.valid >= required:
            break  # the search has its VALID seeds; later seeds are not tried
        s.seeds.append(seed)
        pair = run_pair(seed, m)
        if pair is None:  # R14: no sovereign of this seed meets the power floor: pinned, skipped
            key = f"world {WORLD.id} {seed} horizon_years {HORIZON_YEARS} t_end {T_END}"
            s.blocks[key] = (
                "\n".join([key, *SKIPPED[seed], "pair SKIPPED no_sovereign_qualifies"]) + "\n"
            )
            continue
        s.valid += 1
        lines = render_seed(seed, pair)
        s.blocks[lines[0]] = "\n".join(lines) + "\n"
        s.failures += pair.verdict.failures(f"PROTOCOL FAILED: seed {seed}")
        if args.controls:
            seed_lines, failed = run_controls(seed, pair, m)
            s.control_lines += seed_lines
            s.controls_failed += failed
    if s.valid < required:
        s.failures.append(
            f"PROTOCOL FAILED: {s.valid} VALID seed(s) among {','.join(map(str, s.seeds))}; "
            f"{label} needs {required}"
        )
    return label, s


def main(argv: list[str] | None = None) -> int:
    """Check or regenerate the golden file; run the controls that must fail."""
    args = parse(argv)
    m = Mortality()
    model = m.check(
        [0, DAY, MONTH - DAY, YEAR, 16 * YEAR + 3 * DAY, 61 * YEAR + 7 * DAY, 119 * YEAR]
    )
    if model:
        for f in model:
            print(f"MODEL FAILED: {f}")
        print("CRN PAIRING: FAIL (the model's own checks)")
        return EXIT_MODEL
    label, s = search(args, m)
    blocks, failures, control_lines = s.blocks, s.failures, s.control_lines
    controls_failed, valid_seeds, seeds = s.controls_failed, s.valid, s.seeds
    text = "\n".join(HEADER) + "\n" + "".join(blocks.values())
    print(text, end="")
    for line in control_lines:
        print(line)
    if failures:
        for f in failures:
            print(f)
        print("CRN PAIRING: FAIL")
        return EXIT_FAIL
    if args.write:
        args.golden.parent.mkdir(parents=True, exist_ok=True)
        args.golden.write_text(text, encoding="utf-8")
        print(f"wrote {args.golden}")
        return EXIT_OK
    current = args.golden.read_text(encoding="utf-8") if args.golden.exists() else ""
    stale = compare(current, blocks)
    if stale:
        print(stale)
        print("CRN PAIRING: FAIL")
        return EXIT_FAIL
    wanted = len(CONTROLS) * valid_seeds
    if args.controls and controls_failed != wanted:
        print(f"CRN PAIRING: FAIL (controls: {controls_failed} of {wanted} made the protocol fail)")
        return EXIT_FAIL
    note = f", controls {controls_failed} of {wanted} FAIL as required" if args.controls else ""
    skipped = len(seeds) - valid_seeds
    print(
        f"CRN PAIRING: PASS ({label}, seeds tried {','.join(map(str, seeds))}: {valid_seeds} "
        f"VALID and exact pair(s), {skipped} seed(s) skipped for power{note})"
    )
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
