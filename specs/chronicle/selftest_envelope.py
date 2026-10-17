"""Self-test of the event envelope v0 (SPR-020; specs/chronicle/envelope_v0.md).

It loads ``specs/chronicle/schema.sql`` into an in-memory DuckDB, generates a synthetic 100-year
chronicle of a small dynastic world whose events carry the envelope (roles, anchored snapshots,
typed and motive edges, ``cause_told``), and checks:

  1. anchored snapshots (V-DATA-08): every ``event_snapshot`` row equals the value the emitter
     read at emission (the generator's oracle), not the column's later value; the control plants
     the end-of-run value instead;
  2. typed edges (V-DATA-09; envelope_v0.md §5): only the nine kinds; a motive only on
     ``motivated_by``; ``evidence`` in [0, 256]; ``weight`` 0 off ``motivated_by``; one edge per
     (effect, cause); every cause earlier than its effect; ``contingent`` edges only on verbs that
     declare ``references``; the control plants a motive on a ``triggered`` edge;
  3. ``cause_told`` (05.2 §4.0): an edge's flag is exactly whether its cause has an event row;
  4. provenance of change (V-SAGA-21; envelope_v0.md §7): every non-default state value's cause
     resolves to an existing event at or before the state's start; the control plants a later one;
  5. the plane-B index: ``last_event_at_place``, ``events_at_place``, ``last_event_in_cell`` and
     ``last_event_for`` answer a brute-force oracle on random queries;
  6. the byte budget (V-SAGA-15; envelope_v0.md §9, the measure fixed for SPR-030): the stored
     (Parquet, zstd) bytes of the envelope per event, motive edges counted in full, at the P1a
     design densities, are at or under 40; the fullest P1a envelope on every event is reported and
     must stay under 48; the control plants fifty witnesses on every event.

Run: python -I specs/chronicle/selftest_envelope.py
     [--control snapshot-drift | motive-on-triggered | cause-later | budget-bloat]
     (each must FAIL its check)
"""

from __future__ import annotations

import argparse
import csv
import random
import sys
import tempfile
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

import duckdb

SCHEMA = Path(__file__).resolve().with_name("schema.sql")
DAY = 1440
YEAR = 360 * DAY
YEARS = 100
EVENTS_PER_YEAR = 600
PERSONS = 400
PLACES = 40
CELLS = 16
WORLD, PERSON = 0, 1  # rh-ids EntityKind
ACTOR, TARGET, WITNESS = 0, 1, 2  # Registry [event_roles] indexes
CONTINGENT, TRIGGERED, MOTIVATED_BY = 0, 1, 3  # causal_edge kinds
EDGE_KINDS = 9
ONE = 256  # q8_8 raw bits of 1
STRESS, CELL_X, CELL_Y = (
    25,
    3,
    4,
)  # [attributes] indexes (illustrative; a segment's dictionary names them)
STRESS_MAX = 100 * ONE
DEATH, BIRTH, MARRIAGE, SUCCEED, BREAK, FEUD, REVENGE = range(7)  # synthetic verb ids
REFERENCING_VERBS = {
    REVENGE
}  # verbs with a Registry `references` list: the only contingent emitters
MOTIVES = 9  # [motive_edges] ids 0..8
T1_VERBS = {BREAK, FEUD, REVENGE}  # the acts that carry a stress snapshot and a motive edge
SIM, ORACLE = 0, 3  # event.origin codes; an OracleMove input maps to oracle (envelope_v0.md §2)
INPUT_EVERY_YEARS = 7  # the smoke world's cadence: one material input every seven years
HOPS = 4  # an input's chain: a timer fire, its timer, its command's drain, the drain's own command
VERB_WEIGHTS = {DEATH: 25, BIRTH: 30, MARRIAGE: 10, SUCCEED: 10, BREAK: 10, FEUD: 10, REVENGE: 5}
REFUSAL = (
    0.08  # share of BIRTH drafts the admission rule refuses (a P1.11 world; P1a tells every draft)
)
WITNESS_SHARE = 0.1
CELL_SHARE = 0.2
STRESS_DRIFT = 3 * ONE
BUDGET_MEAN = 40  # V-SAGA-15: mean stored overhead per event at the P1a design densities
BUDGET_FULL = 48  # the cells-and-contaminants ceiling, for the fullest P1a envelope on every event
BLOAT_WITNESSES = 50  # the budget control: this many witnesses on every event
RELIEF = (
    20 * ONE
)  # a break's stress relief: the act's own effect, applied after the pre-act snapshot
INDEX_QUERIES = 100
HALF = 0.5
Row = tuple[object, ...]


@dataclass
class Chronicle:
    """The generated rows, in schema.sql column order, with the generator's oracles."""

    events: list[Row] = field(default_factory=list)
    roles: list[Row] = field(default_factory=list)
    snapshots: list[Row] = field(default_factory=list)
    edges: list[Row] = field(default_factory=list)
    ruins: list[tuple[int, int, int, int, int]] = field(
        default_factory=list
    )  # tl, place, value, from, cause
    admitted: set[int] = field(default_factory=set)
    hops: list[tuple[int, int]] = field(default_factory=list)  # (cause, effect) of one job
    expected: dict[int, int] = field(
        default_factory=dict
    )  # chain event -> the origin it must carry
    inputs: int = 0
    at_emission: dict[tuple[int, int], int] = field(
        default_factory=dict
    )  # (event, actor) -> stress read
    stress: dict[int, int] = field(default_factory=dict)
    next_id: int = 0


@dataclass(frozen=True)
class Control:
    """Which planted defect, if any, the run carries (each must make its check FAIL)."""

    snapshot_drift: bool = False
    motive_on_triggered: bool = False
    cause_later: bool = False
    budget_bloat: bool = False
    origin_lost: bool = False


def _drift(rnd: random.Random, chron: Chronicle, actor: int) -> int:
    """Move the actor's stress column and return its new value (the emitter's view at emit)."""
    value = chron.stress.get(actor, rnd.randint(0, STRESS_MAX))
    value = max(0, min(STRESS_MAX, value + rnd.randint(-STRESS_DRIFT, STRESS_DRIFT)))
    chron.stress[actor] = value
    return value


def _edges(rnd: random.Random, chron: Chronicle, eid: int, verb: int, control: Control) -> None:
    """The event's causes: a triggered edge on life and succession events, a motive edge on acts."""
    if eid == 0:
        return
    cause = rnd.randrange(max(0, eid - 400), eid)
    told = cause in chron.admitted
    if verb in (SUCCEED, DEATH) and rnd.random() < HALF:
        motive = 0 if control.motive_on_triggered else None
        chron.edges.append((0, cause, eid, TRIGGERED, motive, 0, ONE, told))
    elif verb in T1_VERBS:
        kind = CONTINGENT if verb in REFERENCING_VERBS and rnd.random() < HALF else MOTIVATED_BY
        motive = rnd.randrange(MOTIVES) if kind == MOTIVATED_BY else None
        weight = rnd.randint(-ONE, ONE) if kind == MOTIVATED_BY else 0
        evidence = rnd.randint(0, ONE) if kind == MOTIVATED_BY else ONE
        chron.edges.append((0, cause, eid, kind, motive, weight, evidence, told))


def _emit(rnd: random.Random, chron: Chronicle, tick: int, control: Control) -> None:
    """Draft one event with its envelope; a refused draft keeps its id and writes no row."""
    verb = rnd.choices(list(VERB_WEIGHTS), list(VERB_WEIGHTS.values()))[0]
    eid = chron.next_id
    chron.next_id += 1
    if verb == BIRTH and rnd.random() < REFUSAL:
        return  # refused under the admission rule: an id, no row (05.2 §4.0)
    chron.admitted.add(eid)
    place = rnd.randrange(PLACES)
    chron.events.append((0, eid, tick, verb, 1 << verb, place, 0, 128, 0, 0, None, None))
    actor, target = rnd.sample(range(PERSONS), 2)
    cast = [(ACTOR, actor), (TARGET, target)]
    if rnd.random() < WITNESS_SHARE:
        cast.append((WITNESS, rnd.randrange(PERSONS)))
    if control.budget_bloat:
        cast.extend((WITNESS, w) for w in rnd.sample(range(PERSONS), BLOAT_WITNESSES))
    cast.sort()
    for ordinal, (role, who) in enumerate(cast):
        chron.roles.append((0, eid, ordinal, PERSON, who, role))
    if verb in T1_VERBS:
        read = _drift(
            rnd, chron, actor
        )  # the pre-act value the rule read (envelope_v0.md §4 item 2)
        chron.at_emission[(eid, actor)] = read
        chron.snapshots.append((0, eid, STRESS, PERSON, actor, None, None, read))
        if verb == BREAK:  # the act's own effect lands after the snapshot
            chron.stress[actor] = max(0, read - RELIEF)
    if rnd.random() < CELL_SHARE:
        chron.snapshots.append((0, eid, CELL_X, WORLD, 0, None, None, place % CELLS))
        chron.snapshots.append((0, eid, CELL_Y, WORLD, 0, None, None, place // CELLS))
    _edges(rnd, chron, eid, verb, control)
    if verb == REVENGE:
        chron.ruins.append((0, place, rnd.randint(1, ONE), tick, eid))


def _hop(chron: Chronicle, tick: int, origin: int, cause: int | None) -> int:
    """One mechanical consequence of a job, under the job's origin (envelope_v0.md §2, R14, R24).

    A DEATH triggered by the previous hop: every timer, command and draft a job creates carries
    the job's origin, a drain's own commands and timers carry the drained command's, so the
    chain keeps the input's. The expected origin is recorded apart from the stamped one, so a
    stamp that loses it fails check 7 (R26).
    """
    eid = chron.next_id
    chron.next_id += 1
    chron.admitted.add(eid)
    chron.expected[eid] = ORACLE
    row = (0, eid, tick, DEATH, 1 << DEATH, eid % PLACES, 0, 128, 0, origin, None, None)
    chron.events.append(row)
    actor, target = eid % PERSONS, (eid + 1) % PERSONS
    for ordinal, (role, who) in enumerate(sorted([(ACTOR, actor), (TARGET, target)])):
        chron.roles.append((0, eid, ordinal, PERSON, who, role))
    if cause is not None:
        chron.edges.append((0, cause, eid, TRIGGERED, None, 0, ONE, True))
        chron.hops.append((cause, eid))
    return eid


def _plant(chron: Chronicle, control: Control) -> None:
    """Apply the planted defects of the positive controls."""
    if control.snapshot_drift:  # the end-of-run value where the emitter's value should be
        chron.snapshots = [
            (*row[:7], chron.stress[int(str(row[4]))]) if row[2] == STRESS else row
            for row in chron.snapshots
        ]
    if control.cause_later and chron.ruins:
        _, place, value, valid_from, _ = chron.ruins[0]
        chron.ruins[0] = (0, place, value, valid_from, chron.next_id - 1)


def generate(control: Control, seed: int = 2020) -> Chronicle:
    """A 100-year synthetic chronicle at the P1a design densities."""
    rnd = random.Random(seed)
    chron = Chronicle()
    for year in range(YEARS):
        ticks: list[tuple[int, int | None]] = [
            (rnd.randrange(year * YEAR, (year + 1) * YEAR), None) for _ in range(EVENTS_PER_YEAR)
        ]
        if year and year % INPUT_EVERY_YEARS == 0:  # a material input: its consequences hop on
            at = year * YEAR + 99 * DAY + 6 * 60 + 7
            ticks += [(at + i * DAY, i) for i in range(HOPS)]
            chron.inputs += 1
        last: int | None = None
        for tick, hop in sorted(ticks):
            if hop is None:
                _emit(rnd, chron, tick, control)
            else:  # the first hop is the input's job; the control drops the origin after it
                origin = SIM if hop and control.origin_lost else ORACLE
                last = _hop(chron, tick, origin, None if hop == 0 else last)
    for actor in list(chron.stress):  # the column keeps moving after the last emission
        _drift(rnd, chron, actor)
    _plant(chron, control)
    return chron


def bulk_load(con: duckdb.DuckDBPyConnection, table: str, rows: list[Row]) -> None:
    """Load rows through a CSV file (executemany is slow at this size)."""
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / f"{table}.csv"
        with path.open("w", newline="") as f:
            csv.writer(f).writerows([("" if v is None else v) for v in r] for r in rows)
        con.execute(f"COPY {table} FROM '{path.as_posix()}' (FORMAT csv, HEADER false, NULLSTR '')")


def load(chron: Chronicle) -> duckdb.DuckDBPyConnection:
    """A database holding the chronicle, the synthetic verb table and the ruin states."""
    con = duckdb.connect()
    con.execute(SCHEMA.read_text(encoding="utf-8"))
    bulk_load(con, "event", chron.events)
    bulk_load(con, "event_role", chron.roles)
    bulk_load(con, "event_snapshot", chron.snapshots)
    bulk_load(con, "causal_edge", chron.edges)
    con.execute("CREATE TABLE verb_refs (verb_id USMALLINT, references_acts BOOLEAN)")
    con.executemany(
        "INSERT INTO verb_refs VALUES (?, ?)", [(v, v in REFERENCING_VERBS) for v in VERB_WEIGHTS]
    )
    # A transformation state with its cause inside the value (SAGA-15): Ruin { cause } on a place.
    con.execute(
        "CREATE TABLE ruin (timeline_id UINTEGER, place_id UINTEGER, value SMALLINT, "
        "valid_from BIGINT, cause_event UBIGINT)"
    )
    if chron.ruins:
        con.executemany("INSERT INTO ruin VALUES (?, ?, ?, ?, ?)", chron.ruins)
    con.execute("CREATE TABLE hop (cause_id UBIGINT, effect_id UBIGINT)")
    if chron.hops:
        con.executemany("INSERT INTO hop VALUES (?, ?)", chron.hops)
    return con


def count(con: duckdb.DuckDBPyConnection, sql: str) -> int:
    """The single integer a query returns."""
    row = con.execute(sql).fetchone()
    assert row is not None
    return int(row[0])


def _first(con: duckdb.DuckDBPyConnection, sql: str) -> int | None:
    """The first column of a query's first row, or None when it returns no row."""
    got = con.execute(sql).fetchone()
    return None if got is None else int(got[0])


def check_snapshots(con: duckdb.DuckDBPyConnection, chron: Chronicle) -> bool:
    """Check 1: every stress snapshot is the emitter's value at emission (V-DATA-08)."""
    rows = con.execute(
        f"SELECT event_id, entity_id, value FROM event_snapshot WHERE attr_id = {STRESS}"
    ).fetchall()
    wrong = sum(1 for e, who, v in rows if chron.at_emission[(int(e), int(who))] != v)
    moved = sum(1 for e, who, v in rows if chron.stress[int(who)] != v)
    print(
        f"   {len(rows):,} stress snapshots: {wrong} differ from the value at emission; "
        f"{moved} differ from the column's final value (anchoring is not the final value)"
    )
    return wrong == 0 and moved > 0 and len(rows) == len(chron.at_emission)


EDGE_RULES = {
    "a kind outside 0..8": f"SELECT count(*) FROM causal_edge WHERE kind >= {EDGE_KINDS}",
    "a motive off motivated_by": (
        "SELECT count(*) FROM causal_edge WHERE kind <> 3 AND motive IS NOT NULL"
    ),
    "a motive outside [motive_edges]": (
        f"SELECT count(*) FROM causal_edge WHERE motive >= {MOTIVES}"
    ),
    "evidence outside [0, 1]": (
        f"SELECT count(*) FROM causal_edge WHERE evidence < 0 OR evidence > {ONE}"
    ),
    "a weight off motivated_by": "SELECT count(*) FROM causal_edge WHERE kind <> 3 AND weight <> 0",
    "an evidence other than 1 on a deterministic kind": (
        f"SELECT count(*) FROM causal_edge WHERE kind IN (0, 1, 6, 7, 8) AND evidence <> {ONE}"
    ),
    "a forward cause": "SELECT count(*) FROM causal_edge WHERE cause_id >= effect_id",
    "a contingent edge without a reference": (
        "SELECT count(*) FROM causal_edge c JOIN event e ON e.event_id = c.effect_id "
        "JOIN verb_refs v ON v.verb_id = e.verb_id WHERE c.kind = 0 AND NOT v.references_acts"
    ),
    "a repeated (effect, cause)": (
        "SELECT count(*) FROM (SELECT effect_id, cause_id FROM causal_edge "
        "GROUP BY ALL HAVING count(*) > 1)"
    ),
}


def check_edges(con: duckdb.DuckDBPyConnection) -> bool:
    """Check 2: the typed-edge rules of envelope_v0.md §5 (V-DATA-09)."""
    bad = {name: count(con, sql) for name, sql in EDGE_RULES.items()}
    kinds = con.execute(
        "SELECT kind, count(*) FROM causal_edge GROUP BY kind ORDER BY kind"
    ).fetchall()
    found = {k: v for k, v in bad.items() if v} or "none"
    print(f"   edges by kind {dict(kinds)}; violations: {found}")
    return not any(bad.values())


def check_cause_told(con: duckdb.DuckDBPyConnection) -> bool:
    """Check 3: cause_told is exactly whether the cause has an event row (05.2 §4.0)."""
    wrong = count(
        con,
        "SELECT count(*) FROM causal_edge c WHERE c.cause_told <> "
        "EXISTS (SELECT 1 FROM event e WHERE e.timeline_id = c.timeline_id "
        "AND e.event_id = c.cause_id)",
    )
    untold = count(con, "SELECT count(*) FROM causal_edge WHERE NOT cause_told")
    print(f"   {untold} edges to refused causes carry cause_told = false; {wrong} flags wrong")
    return wrong == 0 and untold > 0


def check_provenance(con: duckdb.DuckDBPyConnection) -> bool:
    """Check 4: every non-default ruin value's cause is an existing, earlier event (V-SAGA-21)."""
    unresolved = count(
        con,
        "SELECT count(*) FROM ruin r WHERE r.value <> 0 AND NOT EXISTS ("
        "SELECT 1 FROM event e WHERE e.timeline_id = r.timeline_id AND e.event_id = r.cause_event "
        "AND e.tick <= r.valid_from)",
    )
    total = count(con, "SELECT count(*) FROM ruin")
    print(f"   {total} ruin states, {unresolved} without an existing earlier cause")
    return unresolved == 0 and total > 0


def check_origin(con: duckdb.DuckDBPyConnection, chron: Chronicle) -> bool:
    """Check 7: an input's origin survives every hop (envelope_v0.md §2; 05.2 §4.0 item 4; R14).

    The hops are a timer fire, the timer it inserts, the command that timer submits and the
    command the drain itself submits (R24); each event's origin is compared with the origin the
    generator expects of it, not with its neighbour's alone (R26). The engine test that owns the
    property is the sprint's `origin_through_hops` in rh-core (envelope_v0.md §12).
    """
    wrong = sum(
        1
        for row in con.execute("SELECT event_id, origin FROM event").fetchall()
        if int(row[0]) in chron.expected and int(row[1]) != chron.expected[int(row[0])]
    )
    lost = count(
        con,
        "SELECT count(*) FROM hop h JOIN event c ON c.event_id = h.cause_id "
        "JOIN event e ON e.event_id = h.effect_id WHERE e.origin <> c.origin",
    )
    sim = count(
        con,
        f"SELECT count(*) FROM event e WHERE e.origin = {SIM} AND EXISTS "
        "(SELECT 1 FROM hop h WHERE h.effect_id = e.event_id OR h.cause_id = e.event_id)",
    )
    print(
        f"   {chron.inputs} inputs, {len(chron.hops)} hops ({HOPS - 1} per input); "
        f"{wrong} chain events differ from the expected origin, {lost} hops lost it, "
        f"{sim} stamped sim"
    )
    return chron.inputs > 0 and wrong == 0 and lost == 0 and sim == 0


@dataclass
class Oracle:
    """Brute-force indexes over the admitted events, for the plane-B macros."""

    tick_of: dict[int, int]
    by_place: dict[int, list[int]]
    by_person: dict[int, list[int]]
    by_cell: dict[tuple[int, int], list[int]]

    @classmethod
    def build(cls, chron: Chronicle) -> Oracle:
        """Index the chronicle once."""
        tick_of = {int(str(e[1])): int(str(e[2])) for e in chron.events}
        by_place: dict[int, list[int]] = {}
        for e in chron.events:
            by_place.setdefault(int(str(e[5])), []).append(int(str(e[1])))
        by_person: dict[int, list[int]] = {}
        for r in chron.roles:
            by_person.setdefault(int(str(r[4])), []).append(int(str(r[1])))
        xs = {int(str(s[1])): int(str(s[7])) for s in chron.snapshots if s[2] == CELL_X}
        by_cell: dict[tuple[int, int], list[int]] = {}
        for s in chron.snapshots:
            if s[2] == CELL_Y and int(str(s[1])) in xs:
                by_cell.setdefault((xs[int(str(s[1]))], int(str(s[7]))), []).append(int(str(s[1])))
        return cls(tick_of, by_place, by_person, by_cell)

    def latest(self, ids: list[int], t: int) -> int | None:
        """The largest id among ``ids`` whose event is at or before t (ties: emission order)."""
        return max((e for e in ids if self.tick_of[e] <= t), default=None)


def check_index(con: duckdb.DuckDBPyConnection, chron: Chronicle) -> bool:
    """Check 5: the plane-B "what last happened here?" macros agree with brute force."""
    rnd = random.Random(7)
    oracle = Oracle.build(chron)
    wrong = 0
    for _ in range(INDEX_QUERIES):
        place, who, t = rnd.randrange(PLACES), rnd.randrange(PERSONS), rnd.randrange(YEARS * YEAR)
        x, y = place % CELLS, place // CELLS
        got = _first(con, f"SELECT event_id FROM last_event_at_place(0, {place}, {t})")
        wrong += got != oracle.latest(oracle.by_place.get(place, []), t)
        got = _first(con, f"SELECT event_id FROM last_event_for(0, {PERSON}, {who}, {t})")
        wrong += got != oracle.latest(oracle.by_person.get(who, []), t)
        got = _first(
            con, f"SELECT event_id FROM last_event_in_cell(0, {CELL_X}, {CELL_Y}, {x}, {y}, {t})"
        )
        wrong += got != oracle.latest(oracle.by_cell.get((x, y), []), t)
        window = count(con, f"SELECT count(*) FROM events_at_place(0, {place}, {t - YEAR}, {t})")
        wrong += window != sum(
            1 for e in oracle.by_place.get(place, []) if t - YEAR <= oracle.tick_of[e] <= t
        )
    print(f"   {INDEX_QUERIES} random queries x 4 macros: {wrong} disagree with brute force")
    return wrong == 0


def _parquet_bytes(con: duckdb.DuckDBPyConnection, query: str, tmp: Path, name: str) -> int:
    """The size of a query's result written as Parquet with zstd."""
    path = tmp / f"{name}.parquet"
    con.execute(f"COPY ({query}) TO '{path.as_posix()}' (FORMAT parquet, COMPRESSION zstd)")
    return path.stat().st_size


def stored_overhead(con: duckdb.DuckDBPyConnection, tmp: Path, name: str) -> float:
    """The envelope's stored bytes per admitted event (envelope_v0.md §9: the V-SAGA-15 measure).

    (i) cast rows beyond the first two, (ii) every snapshot row, (iii) every motivated_by edge in
    full (a motive edge is new), (iv) the typed columns of every other edge (P1.0 already stored
    their cause and kind): the Parquet bytes, zstd, divided by the admitted events.
    """
    events = count(con, "SELECT count(*) FROM event")
    roles = _parquet_bytes(con, "SELECT * FROM event_role WHERE ordinal >= 2", tmp, f"{name}_roles")
    snapshots = _parquet_bytes(con, "SELECT * FROM event_snapshot", tmp, f"{name}_snapshots")
    motive_edges = _parquet_bytes(
        con, f"SELECT * FROM causal_edge WHERE kind = {MOTIVATED_BY}", tmp, f"{name}_motive"
    )
    other_full = _parquet_bytes(
        con, f"SELECT * FROM causal_edge WHERE kind <> {MOTIVATED_BY}", tmp, f"{name}_other"
    )
    other_p10 = _parquet_bytes(
        con,
        "SELECT timeline_id, cause_id, effect_id, kind FROM causal_edge "
        f"WHERE kind <> {MOTIVATED_BY}",
        tmp,
        f"{name}_other_p10",
    )
    return (roles + snapshots + motive_edges + other_full - other_p10) / events


def fullest(chron: Chronicle) -> Chronicle:
    """The events with the fullest P1a envelope: a witness, a stress snapshot and a motive edge."""
    rnd = random.Random(3)
    full = Chronicle(events=list(chron.events), admitted=set(chron.admitted), next_id=chron.next_id)
    for e in chron.events:
        eid = int(str(e[1]))
        actor, target, witness = rnd.sample(range(PERSONS), 3)
        cast = sorted([(ACTOR, actor), (TARGET, target), (WITNESS, witness)])
        full.roles.extend((0, eid, i, PERSON, who, role) for i, (role, who) in enumerate(cast))
        full.snapshots.append(
            (0, eid, STRESS, PERSON, actor, None, None, rnd.randint(0, STRESS_MAX))
        )
        if eid > 0:
            cause = rnd.randrange(max(0, eid - 400), eid)
            motive, weight, evidence = (
                rnd.randrange(MOTIVES),
                rnd.randint(-ONE, ONE),
                rnd.randint(0, ONE),
            )
            full.edges.append(
                (0, cause, eid, MOTIVATED_BY, motive, weight, evidence, cause in chron.admitted)
            )
    return full


def check_budget(con: duckdb.DuckDBPyConnection, chron: Chronicle) -> bool:
    """Check 6: stored envelope bytes per event, design densities (<= 40), the fullest (< 48)."""
    with tempfile.TemporaryDirectory() as tmp:
        design = stored_overhead(con, Path(tmp), "design")
        full = stored_overhead(load(fullest(chron)), Path(tmp), "full")
    print(
        f"   stored envelope overhead: {design:.1f} bytes/event at the P1a design densities "
        f"(budget {BUDGET_MEAN}); {full:.1f} with the fullest P1a envelope on every event "
        f"(ceiling {BUDGET_FULL})"
    )
    return design <= BUDGET_MEAN and full < BUDGET_FULL


def main(argv: list[str] | None = None) -> int:
    """Run every check and print PASS or FAIL."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument(
        "--control",
        choices=[
            "snapshot-drift",
            "motive-on-triggered",
            "cause-later",
            "budget-bloat",
            "origin-lost",
        ],
        help="plant one defect",
    )
    args = parser.parse_args(argv)
    control = Control(
        snapshot_drift=args.control == "snapshot-drift",
        motive_on_triggered=args.control == "motive-on-triggered",
        cause_later=args.control == "cause-later",
        budget_bloat=args.control == "budget-bloat",
        origin_lost=args.control == "origin-lost",
    )
    chron = generate(control)
    con = load(chron)
    print(
        f"   {len(chron.events):,} admitted events of {chron.next_id:,} drafts over {YEARS} years; "
        f"{len(chron.roles):,} roles, {len(chron.snapshots):,} snapshots, "
        f"{len(chron.edges):,} edges"
    )
    checks: list[tuple[str, Callable[[], bool]]] = [
        ("1. anchored snapshots", lambda: check_snapshots(con, chron)),
        ("2. typed edges", lambda: check_edges(con)),
        ("3. cause_told", lambda: check_cause_told(con)),
        ("4. provenance of change", lambda: check_provenance(con)),
        ("5. the plane-B index", lambda: check_index(con, chron)),
        ("6. the byte budget", lambda: check_budget(con, chron)),
        ("7. origin through hops", lambda: check_origin(con, chron)),
    ]
    ok = True
    for name, fn in checks:
        good = fn()
        ok &= good
        print(f"{name}: {'PASS' if good else 'FAIL'}")
    print("ENVELOPE v0:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
