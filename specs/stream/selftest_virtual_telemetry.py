"""Virtual telemetry self-test (ADR-0031; `05.10` §9; V-STR-06/07 prototype).

Claim under test: with keyed counter-based randomness, a person's daily activity is a PURE function of
(seed, person, day, life-course state at the start of the day, regional conditions). So:
  1. random access: any person-day in history is computed on demand, without replaying anything else;
  2. it equals what a sequential engine loop computed for the same person-day (viewer == engine);
  3. encounters are a second phase over fixed positions, so they are random-access per (place, block) too;
  4. storage is zero: only the life-course log (residence/occupation intervals) is kept.

The model here is a toy (12 occupations x 15 blocks, a 3-state Markov day), not S5 itself.
Run: python -I specs/stream/selftest_virtual_telemetry.py
"""
import sys
import time

MASK = (1 << 64) - 1
BLOCKS = 15                      # activity blocks per day
ACTIVITIES = ("home", "work", "market")


def mix(*xs: int) -> int:
    """splitmix64-style keyed hash: stands in for H(seed, system, purpose, entity, t, k) (ADR-0006)."""
    h = 0x9E3779B97F4A7C15
    for x in xs:
        h = (h ^ (x & MASK)) * 0xBF58476D1CE4E5B9 & MASK
        h ^= h >> 31
        h = h * 0x94D049BB133111EB & MASK
        h ^= h >> 29
    return h


class LifeCourse:
    """Synthetic life-course log: residence and workplace intervals per person (DATA-8 analogue)."""

    def __init__(self, seed: int, n_persons: int, n_cells: int, years: int):
        self.seed, self.n, self.cells = seed, n_persons, n_cells
        self.moves = {}  # person -> sorted list of (start_day, home_cell, work_cell, occupation)
        for p in range(n_persons):
            iv, day = [], 0
            while day < years * 360:
                h = mix(seed, 1, p, day)
                iv.append((day, h % n_cells, (h >> 20) % n_cells, (h >> 40) % 12))
                day += 360 * (1 + (h >> 50) % 15)          # a move every 1-15 years
            self.moves[p] = iv

    def state(self, person: int, day: int):
        iv = self.moves[person]
        lo, hi = 0, len(iv) - 1
        while lo < hi:                                      # last interval starting on or before `day`
            mid = (lo + hi + 1) // 2
            if iv[mid][0] <= day:
                lo = mid
            else:
                hi = mid - 1
        return iv[lo][1:]


def regional_flag(seed: int, cell: int, day: int) -> int:
    """Regional condition from the step digest (e.g. curfew/plague): 1 = stay home. Here: rare, keyed."""
    return 1 if mix(seed, 2, cell // 16, day // 30) % 50 == 0 else 0


def activity_core(seed: int, person: int, day: int, home: int, work: int, occ: int):
    """S5 activity(person, day) -> [(block, cell, activity)]: depends only on start-of-day state + flags + keyed RNG."""
    curfew = regional_flag(seed, home, day)
    out, act = [], 0
    for b in range(BLOCKS):
        r = mix(seed, 3, person, day, b) % 100
        if curfew:
            act = 0
        elif 4 <= b <= 10:
            act = 1 if r < 70 + occ else (2 if r < 90 else 0)
        else:
            act = 0 if r < 80 else 2
        cell = home if act == 0 else (work if act == 1 else (home + work) // 2)
        out.append((b, cell, ACTIVITIES[act]))
    return out


def activity(seed: int, lc: LifeCourse, person: int, day: int):
    """Viewer side: look the start-of-day state up in the life-course log (random access, no replay)."""
    return activity_core(seed, person, day, *lc.state(person, day))


def encounters(seed, lc, cell, day, block, residents):
    """Phase 2: who is in `cell` at (day, block). Uses only phase-1 positions, so it is random-access too."""
    return sorted(p for p in residents if activity(seed, lc, p, day)[block][1] == cell)


def main() -> int:
    seed, n, cells, years = 0x5EED, 1500, 200, 60
    lc = LifeCourse(seed, n, cells, years)
    fails = []

    # 1-3. A STATEFUL engine loop advances day by day from day 0, applying moves as they fall due (it never consults
    # the log by lookup) and records a sample of person-days plus one full occupancy map. The viewer then answers the
    # same questions by random access into the life-course log. They must agree exactly.
    cur = {p: lc.moves[p][0][1:] for p in range(n)}
    agenda = {}                                             # scheduled move outcomes, keyed by due day (ADR-0025)
    for p in range(n):
        for start, *st in lc.moves[p][1:]:
            agenda.setdefault(start, []).append((p, tuple(st)))
    sample, occupancy = {}, {}
    probe_day, probe_block = 3_030, 7
    t0 = time.perf_counter()
    for day in range(0, 3_060):
        for p, st in agenda.get(day, []):                   # apply due move events (engine-side state change)
            cur[p] = st
        if day < 3_000:
            continue
        for p in range(n):
            a = activity_core(seed, p, day, *cur[p])
            if (p * 31 + day) % 97 == 0:
                sample[(p, day)] = a
            if day == probe_day:
                occupancy.setdefault(a[probe_block][1], []).append(p)
    engine_s = time.perf_counter() - t0
    mismatch = sum(1 for (p, d), a in sample.items() if activity(seed, lc, p, d) != a)
    print(f"  {'ok  ' if mismatch == 0 else 'FAIL'} random access equals the stateful engine loop on {len(sample)} sampled person-days")
    if mismatch:
        fails.append("random-access mismatch")

    # Random access deep in history, with no replay: time per person-day.
    qs = [(mix(seed, 9, i) % n, mix(seed, 10, i) % (years * 360)) for i in range(20_000)]
    t0 = time.perf_counter()
    for p, d in qs:
        activity(seed, lc, p, d)
    per_us = (time.perf_counter() - t0) / len(qs) * 1e6
    print(f"  ok   random access anywhere in {years} years: {per_us:.1f} µs per person-day (pure Python; the Rust target of < 1 ms has ample room)")

    # Encounters: the viewer's on-demand answer for one (cell, day, block) equals the engine's occupancy map.
    cell = activity(seed, lc, 42, probe_day)[probe_block][1]
    e_view = encounters(seed, lc, cell, probe_day, probe_block, range(n))
    e_engine = sorted(occupancy.get(cell, []))
    ok = e_view == e_engine and 42 in e_view
    print(f"  {'ok  ' if ok else 'FAIL'} who was at person 42's place on day {probe_day}, block {probe_block}: viewer == engine ({len(e_view)} present)")
    if not ok:
        fails.append("encounters")

    # 4. Storage: life-course intervals only.
    n_iv = sum(len(v) for v in lc.moves.values())
    stored = n_iv * 16
    virtual = n * years * 360 * BLOCKS * 2
    print(f"  ok   stored: {n_iv} life-course intervals ≈ {stored/1e3:.0f} kB; virtual telemetry it reproduces ≈ {virtual/1e9:.1f} GB at 2 B per block")
    print(f"  info stateful engine loop: {engine_s:.1f} s for {n} persons x 3,060 days (pure Python)")

    print("VIRTUAL TELEMETRY:", "PASS" if not fails else f"FAIL {fails}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
