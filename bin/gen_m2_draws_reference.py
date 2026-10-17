"""Generate the differential golden of M2's draws: every death instant of small fixture worlds.

The Tester found that no test pinned M2's draw wiring (the review of SPR-012): swapping k = 0 and
k = 1 in ``resample_timer`` survived every gate. This tool is an independent model of
``[algorithm.mortality_rung0]`` S8 to S12 of ``specs/systems/M2.demography.odd.toml``, built only
on the two references, never on the Rust code:

* the keyed RNG of ``bin/gen_rng_golden.py`` (ChaCha8 cascade, the H of 05.1 §6 item 2), with
  ``system_id``, ``purpose_id``, the founder ids and rh-core's ``resample_entity_id``;
* the schedule of ``specs/demography/selftest_m2_rung0.py`` (``solve_k``, ``survivors``,
  ``month_q``, ``death_day``, ``resample_day``, ``hits``).

It runs each fixture world instant by instant as 05.1 §4 orders an instant (the material inputs
in submission order, each bumping its person's epoch and re-sampling the person's pending death;
then Vital's timer pass, the deaths; then the Month pulse's draws, one Bernoulli per living person
without a pending death, in ascending ``PersonId``), and writes every death instant. The inputs are
chosen by the tool from the worlds' own histories so that they cover every re-sample case S11
names: an input on a pending death in the middle of its month, at the timer's own instant, on the
month's last day (no day start left: dropped), two inputs on one person at one instant, and an
input on a person with no pending death (an epoch bump only).

Output: ``crates/rh-sys-demography/tests/golden/m2_draws_v1.txt``: per world a ``world`` line
(name, seed, calendar days, Q16_16 e0, years), its ``person`` lines (id, birth tick), its ``input``
lines in submission order (tick, person), its ``resample`` lines (tick, person, case, outcome) and
its ``death`` lines (tick, person) in tick then id order. Read by
``crates/rh-sys-demography/tests/draws_golden.rs``, which runs the engine on the same worlds and
inputs and requires the same deaths.

Usage:  python -I bin/gen_m2_draws_reference.py           (write the file)
        python -I bin/gen_m2_draws_reference.py --check   (exit 1 if the committed file is stale)
        --output <file> checks or writes another file
Standard library only (GOV-003 §3). Refs: SPR-012 T-07 and the Architect's ruling on the Tester's
interim report; ADR-0025 item 3; 05.1 §4, §6.
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
from collections.abc import Sequence
from dataclasses import dataclass, field
from pathlib import Path
from types import ModuleType

REPO = Path(__file__).resolve().parents[1]
OUTPUT = REPO / "crates" / "rh-sys-demography" / "tests" / "golden" / "m2_draws_v1.txt"
DAY = 1440
MONTHS_365 = (31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)
MONTHS_360 = (30,) * 12
DAYS_360 = 360
PURPOSE = "death_time"
SYSTEM = "M2.demography"
MASK64 = (1 << 64) - 1
PERSON_KIND = 1  # rh_ids::EntityKind::Person
# (name, seed, calendar days, e0 literal, founders, lowest and highest founder age in hundredths of
# a year). Many founders are close to 100: the month a person reaches 100 has q = 1 (S8), so their
# deaths, and the re-samples of them (q' = 1 too once the month reaches 100), are certain; w6's
# founders all reach 100 in the first month, so their deaths land on every day of it, the last one
# included (the case S11's c = 0 needs).
WORLDS = (
    ("w1", 11, 360, "30", 24, 6000, 10000),
    ("w2", 23, 365, "25", 30, 5500, 10000),
    ("w3", 37, 360, "45", 20, 9000, 10000),
    ("w4", 41, 365, "20", 30, 5000, 10000),
    ("w5", 58, 360, "35", 24, 9500, 10000),
    ("w6", 77, 365, "30", 60, 9995, 10000),
)
# BCK-002 A-53 (VER-SPR-012-TEST GAP-8): one larger world whose founders (aged 96 to 99.9, where a
# day of hazard is large) have their pending deaths re-sampled mid-month again and again
# (MASS_ROUNDS rounds, one input per founder per round), about 500 re-samples; the small worlds
# above hold 19, which cannot resolve a one-day error in q' (the rest of the month). Power, from
# `--power` (this model with q' over one day more, or one day less, than the rest of the month):
# 5 and 10 re-sample outcomes change, so either error fails the golden. People draw independently
# (each key carries the person's own epoch), so each round's inputs are chosen from the previous
# round's history and do not move anyone else's draws.
MASS_WORLD = ("w7", 101, 365, "30", 200, 9600, 9990)
MASS_ROUNDS = 5
CASES = ("last_day", "duplicate", "mid_month", "own_instant", "no_pending")


def load(path: Path, name: str) -> ModuleType:
    """Return the module at ``path`` (the two references)."""
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


RNG = load(REPO / "bin" / "gen_rng_golden.py", "gen_rng_golden")
SCHED = load(REPO / "specs" / "demography" / "selftest_m2_rung0.py", "selftest_m2_rung0")
TAG_RESAMPLE_ENTITY = RNG.tag("rehoboam/rh-core/resample-entity/v1")


class Keys:
    """The keyed draws of M2 in one world, with the cascade's keys cached (pure functions)."""

    def __init__(self, seed: int) -> None:
        """Keys of world ``seed`` on the root timeline (branch salt 0)."""
        self.seed = seed
        self.system = RNG.system_id(SYSTEM)
        self.purpose = RNG.purpose_id(PURPOSE)
        self.k1: dict[int, bytes] = {}
        self.k2: dict[tuple[int, int], bytes] = {}

    def _k2(self, epoch: int, entity: int) -> bytes:
        if epoch not in self.k1:
            self.k1[epoch] = RNG.prf(RNG.seed_key(self.seed), 0, self.system | (epoch << 32))
        if (epoch, entity) not in self.k2:
            self.k2[(epoch, entity)] = RNG.prf(self.k1[epoch], self.purpose, entity)
        return self.k2[(epoch, entity)]

    def step(self, epoch: int, person: int, t: int, k: int) -> int:
        """A StepCtx draw: entity = the PersonId, keyed by the person's epoch."""
        return int(RNG.stream_u64(self._k2(epoch, person), t, k))

    def resample(self, epoch: int, person: int, t: int, k: int) -> int:
        """A ResampleCtx draw: entity = resample_entity_id(person), the epoch after the input."""
        entity = RNG.h64_words(TAG_RESAMPLE_ENTITY, [PERSON_KIND, person])
        return int(RNG.stream_u64(self._k2(epoch, entity), t, k))


def month_starts(days: int, years: int) -> list[int]:
    """The Month pulse instants of ``years`` years, and the start of the year after."""
    lengths = MONTHS_360 if days == DAYS_360 else MONTHS_365
    out, t = [], 0
    for _ in range(years):
        for n in lengths:
            out.append(t)
            t += n * DAY
    out.append(t)
    return out


@dataclass
class World:
    """One fixture world and its history."""

    name: str
    seed: int
    days: int
    e0_bits: int
    years: int
    births: dict[int, int]
    inputs: list[tuple[int, int]] = field(default_factory=list)
    deaths: list[tuple[int, int]] = field(default_factory=list)
    resamples: list[tuple[int, int, str, str]] = field(default_factory=list)
    timers: list[tuple[int, int, int]] = field(default_factory=list)  # (t_m, person, tau)


@dataclass
class Run:
    """The state of one simulation: the references bound to the world, and who is where."""

    w: World
    lx: list[int]
    keys: Keys
    starts: list[int]
    epoch: dict[int, int]
    alive: set[int]
    pending: dict[int, int] = field(default_factory=dict)
    instants: list[int] = field(default_factory=list)
    q_shift: int = 0  # --power only: q' over (rest of the month + q_shift) minutes, a planted error

    def month_end(self, t: int) -> int:
        """The first Month pulse strictly after ``t``."""
        return next(s for s in self.starts if s > t)

    def schedule(self, tau: int) -> None:
        """Add ``tau`` to the instants still to run."""
        if tau not in self.instants:
            self.instants.append(tau)
            self.instants.sort()

    def apply_input(self, t: int, p: int) -> None:
        """One material input on ``p`` at ``t``: the epoch, then S11 on a pending death."""
        w = self.w
        self.epoch[p] += 1
        tau = self.pending.get(p)
        if tau is None or tau < t:
            w.resamples.append((t, p, "no_pending", "-"))
            return
        del self.pending[p]
        t_end = self.month_end(t)
        q = SCHED.month_q(self.lx, t - w.births[p], t_end - t + self.q_shift, w.days * DAY)
        if not SCHED.hits(self.keys.resample(self.epoch[p], p, t, 0), q):
            w.resamples.append((t, p, "pending", "miss"))
            return
        new = SCHED.resample_day(t, t_end, self.keys.resample(self.epoch[p], p, t, 1))
        w.resamples.append((t, p, "pending", "none" if new is None else str(new)))
        if new is not None:
            self.pending[p] = new
            self.schedule(new)

    def month_step(self, t: int) -> None:
        """S8-S10 at the Month pulse ``t``: one Bernoulli per living person, ascending id."""
        w = self.w
        t_end = self.month_end(t)
        for p in sorted(self.alive):
            if p in self.pending:
                raise AssertionError(f"{w.name}: {p} has a pending death at a Month pulse")
            q = SCHED.month_q(self.lx, t - w.births[p], t_end - t, w.days * DAY)
            if SCHED.hits(self.keys.step(self.epoch[p], p, t, 0), q):
                tau = SCHED.death_day(t, t_end - t, self.keys.step(self.epoch[p], p, t, 1))
                self.pending[p] = tau
                w.timers.append((t, p, tau))
                self.schedule(tau)


def simulate(w: World, q_shift: int = 0) -> None:
    """Run ``w`` with its inputs: fill deaths, re-samples and every scheduled timer.

    ``q_shift`` plants an error in q' (``--power`` only; the golden file is always made with 0).
    """
    starts = month_starts(w.days, w.years)
    run = Run(
        w,
        SCHED.survivors(SCHED.solve_k(w.e0_bits)),
        Keys(w.seed),
        starts,
        dict.fromkeys(w.births, 0),
        set(w.births),
        q_shift=q_shift,
    )
    w.deaths, w.resamples, w.timers = [], [], []
    by_tick: dict[int, list[int]] = {}
    for t_in, p in w.inputs:
        by_tick.setdefault(t_in, []).append(p)
    months = set(starts[:-1])
    run.instants = sorted(months | set(by_tick))
    while run.instants and run.instants[0] < starts[-1]:
        t = run.instants.pop(0)
        for p in by_tick.get(t, []):  # Inputs, in submission order
            run.apply_input(t, p)
        for p in sorted(q for q, tau in run.pending.items() if tau == t):  # Vital's timer pass
            del run.pending[p]
            run.alive.discard(p)
            w.deaths.append((t, p))
        if t in months:
            run.month_step(t)


def world(spec: tuple[str, int, int, str, int, int, int]) -> World:
    """A fixture world: founder ids founder(seed, k), birth ticks from a SplitMix64 of the seed."""
    name, seed, days, e0, n, lo, hi = spec
    year = days * DAY
    state = seed
    births = {}
    for k in range(n):
        person = RNG.h64_words(RNG.TAG_FOUNDER, [seed, k])
        state = (state + RNG.GAMMA) & MASK64
        age = lo * year // 100 + RNG.mix(state) % ((hi - lo) * year // 100)
        births[person] = -age
    years = 100 - lo // 100 + 2  # every founder reaches 100 within the run
    return World(name, seed, days, SCHED.q16(e0), years, births)


def last_day(w: World, tau: int) -> bool:
    """Whether ``tau`` is the last day start of its month."""
    starts = month_starts(w.days, w.years)
    return next(s for s in starts if s > tau) - tau == DAY


def certain(w: World, person: int, tau: int) -> bool:
    """Whether a re-sample of ``person``'s death at ``tau`` hits for sure: the month reaches 100."""
    starts = month_starts(w.days, w.years)
    t_end = next(s for s in starts if s > tau)
    return t_end - w.births[person] >= 100 * w.days * DAY


def choose_inputs(w: World) -> None:
    """Add inputs one case at a time, each against the history the earlier inputs leave."""
    after = -1  # before the first Month pulse, so its timers are candidates
    for case in CASES:
        simulate(w)
        if case == "no_pending":
            dead_by = {p: t for t, p in w.deaths}
            pend = {p for t_m, p, tau in w.timers if t_m <= after < tau}
            choice = next(
                (
                    p
                    for p in sorted(w.births)
                    if dead_by.get(p, 1 << 62) > after + DAY and p not in pend
                ),
                None,
            )
            if choice is not None:
                w.inputs.append((after + 377, choice))
                after += 377
            continue
        for t_m, p, tau in w.timers:
            if t_m <= after:
                continue
            if case == "mid_month" and tau - t_m >= 3 * DAY and not last_day(w, tau):
                w.inputs.append((t_m + (tau - t_m) // 2 + 7, p))
            elif (case == "own_instant" and not last_day(w, tau)) or (
                case == "last_day" and last_day(w, tau) and certain(w, p, tau)
            ):
                w.inputs.append((tau, p))
            elif case == "duplicate" and tau - t_m >= 2 * DAY:
                w.inputs += [(tau - DAY + 11, p), (tau - DAY + 11, p)]
            else:
                continue
            after = w.inputs[-1][0]
            break
    simulate(w)


def choose_mass_inputs(w: World) -> None:
    """MASS_WORLD: per round, one input inside each founder's next pending death, in tick order."""
    for _ in range(MASS_ROUNDS):
        simulate(w)
        last: dict[int, int] = {}
        for t, p in w.inputs:
            last[p] = max(last.get(p, -1), t)
        new: list[tuple[int, int]] = []
        chosen: set[int] = set()
        for t_m, p, tau in w.timers:
            after = last.get(p, -1)
            if p in chosen or t_m < after or tau <= after or tau - t_m < 2 * DAY:
                continue
            chosen.add(p)
            new.append((t_m + (tau - t_m) // 2 + 7, p))
        if not new:
            break
        w.inputs = sorted(w.inputs + new)
    simulate(w)


def power() -> list[tuple[int, int]]:
    """For a q' one day too long and one day too short: the re-sample outcomes that change."""
    w = world(MASS_WORLD)
    choose_mass_inputs(w)
    base = list(w.resamples)
    out = []
    for shift in (DAY, -DAY):
        simulate(w, shift)
        out.append((shift, sum(a != b for a, b in zip(base, w.resamples, strict=False))))
    return out


def render() -> tuple[str, dict[str, int]]:
    """Return the golden file's text and the count of each re-sample case covered."""
    lines = [
        "# M2's death instants on fixture worlds. GENERATED by bin/gen_m2_draws_reference.py;"
        " never edit.",
        "# An independent model of [algorithm.mortality_rung0] S8-S12 on the keyed-RNG reference.",
        "# world name seed days e0_bits years | person id birth | input tick person |",
        "# resample tick person case outcome | death tick person",
    ]
    covered = dict.fromkeys(
        ("mid_month", "own_instant", "last_day", "duplicate", "no_pending", "hit", "miss"), 0
    )
    for spec in (*WORLDS, MASS_WORLD):
        w = world(spec)
        if spec is MASS_WORLD:
            choose_mass_inputs(w)
        else:
            choose_inputs(w)
        lines.append(f"world {w.name} {w.seed} {w.days} {w.e0_bits} {w.years}")
        lines += [f"person {p} {b}" for p, b in sorted(w.births.items())]
        lines += [f"input {t} {p}" for t, p in w.inputs]
        for t, p, kind, outcome in w.resamples:
            lines.append(f"resample {t} {p} {kind} {outcome}")
        lines += [f"death {t} {p}" for t, p in sorted(w.deaths)]
        classify(w, covered)
    return "\n".join(lines) + "\n", covered


def classify(w: World, covered: dict[str, int]) -> None:
    """Count the S11 cases this world's re-samples exercise."""
    ticks = [t for t, _ in w.inputs]
    for i, (t, p, kind, outcome) in enumerate(w.resamples):
        if kind == "no_pending":
            covered["no_pending"] += 1
            continue
        if ticks.count(t) > 1 and i > 0 and w.resamples[i - 1][:2] == (t, p):
            covered["duplicate"] += 1
        if outcome == "miss":
            covered["miss"] += 1
        elif outcome != "none":
            covered["hit"] += 1
        if outcome == "none":
            covered["last_day"] += 1
        elif any(tau == t for _, q, tau in w.timers if q == p):
            covered["own_instant"] += 1
        else:
            covered["mid_month"] += 1


def main(argv: Sequence[str] | None = None) -> int:
    """Write the golden file, or with ``--check`` verify that it is current."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    parser.add_argument("--output", type=Path, default=OUTPUT, help="the golden file")
    parser.add_argument(
        "--power", action="store_true", help="print the outcomes a one-day error in q' changes"
    )
    args = parser.parse_args(argv)
    if args.power:
        changes = power()
        for shift, changed in changes:
            print(f"gen_m2_draws_reference: q' {shift:+d} minutes changes {changed} outcomes")
        return 0 if all(changed > 0 for _, changed in changes) else 1
    output: Path = args.output
    text, covered = render()
    missing = [case for case, n in covered.items() if n == 0]
    summary = ", ".join(f"{case} {n}" for case, n in covered.items())
    if missing:
        print(f"gen_m2_draws_reference: the fixtures miss re-sample cases {missing} ({summary})")
        return 2
    deaths = sum(1 for line in text.splitlines() if line.startswith("death "))
    if args.check:
        current = output.read_text(encoding="utf-8") if output.exists() else ""
        if current != text:
            print(f"STALE: {output} differs; run python -I bin/gen_m2_draws_reference.py")
            return 1
        print(f"gen_m2_draws_reference: current ({deaths} deaths; re-samples: {summary})")
        return 0
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(text, encoding="utf-8")
    print(f"gen_m2_draws_reference: wrote {output} ({deaths} deaths; re-samples: {summary})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
