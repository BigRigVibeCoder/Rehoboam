"""Self-test of the World Stream v0 channel-A segment schema (SPR-011 ruling 13).

It loads ``specs/stream/channel_a_v0.sql`` into an in-memory DuckDB and checks:

  1. no float column anywhere (R2), and the check itself refuses a planted DOUBLE column;
  2. every record table starts with the header (timeline_id, seq, tick, kind) with the R3 types, and
     its kind CHECK matches the record_kind dictionary;
  3. a synthetic run of three founders over two sim-years (genesis, monthly digests, one input,
     three deaths and the extinction, a keyframe marker) is written as sealed segments in the R5
     layout, one Parquet file per table per sim-year, and read back: no float column in any
     written file, seq contiguous from 0 across segments, the R4 in-instant order holds (the
     order_violation view is empty), the segment rows chain, and life_course_v rebuilds
     schema.sql's life_course;
  4. the order check refuses a planted swap of two records.

Run: python -I specs/stream/selftest_channel_a.py
"""

from __future__ import annotations

import itertools
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

import duckdb

DDL = Path(__file__).resolve().with_name("channel_a_v0.sql")
HEADER = [
    ("timeline_id", "UINTEGER"),
    ("seq", "UBIGINT"),
    ("tick", "BIGINT"),
    ("kind", "USMALLINT"),
]
RECORD_TABLES = {
    "input": "INPUT",
    "event": "EVENT",
    "life_course_entry": "LIFE_ENTRY",
    "life_course_exit": "LIFE_EXIT",
    "step_digest": "STEP_DIGEST",
    "keyframe_marker": "KEYFRAME",
}
CHILD_TABLES = ("event_role", "causal_edge")
DICTIONARIES = ("verb", "act_class", "record_kind")
FLOAT_TYPES = ("FLOAT", "REAL", "DOUBLE")
YEAR = 360 * 1440  # minutes per year, 360-day calendar
MONTH = 30 * 1440
MONTHS = 24
DEATH, EXTINCTION = 0, 1  # verb ids of the synthetic dictionary (sorted by name)
TARGET = 1  # [event_roles] index of target
TRIGGERED = 1  # causal_edge kind
FOUNDERS = (0x1111, 0x2222, 0x3333)
DEATH_TICKS = {0x2222: 5 * MONTH + 777, 0x1111: 14 * MONTH + 3, 0x3333: 20 * MONTH + 41}
INPUT_TICK = 3 * MONTH + 100
KEYFRAME_TICK = 12 * MONTH
FORMAT = "rehoboam/stream/channel-a/v0"


def float_columns(con: duckdb.DuckDBPyConnection) -> list[str]:
    """Return every table or view column whose type is or contains a float (R2)."""
    rows = con.execute(
        "SELECT table_name, column_name, data_type FROM information_schema.columns"
    ).fetchall()
    return [f"{t}.{c} {d}" for t, c, d in rows if any(f in d.upper() for f in FLOAT_TYPES)]


def load(ddl: str) -> duckdb.DuckDBPyConnection:
    """Return an in-memory database with ``ddl`` applied."""
    con = duckdb.connect()
    con.execute(ddl)
    return con


def check_no_floats() -> bool:
    """Check 1: no float column, and the check catches a planted one."""
    ddl = DDL.read_text(encoding="utf-8")
    found = float_columns(load(ddl))
    planted = ddl.replace("magnitude    INTEGER", "magnitude    DOUBLE", 1)
    caught = float_columns(load(planted))
    print(f"   float columns: {found or 'none'}; planted control caught {caught}")
    return not found and caught == ["event.magnitude DOUBLE"]


def check_headers() -> bool:
    """Check 2: every record table starts with the R3 header, and its kind matches record_kind."""
    con = load(DDL.read_text(encoding="utf-8"))
    kinds = dict(con.execute("SELECT name, kind FROM record_kind").fetchall())
    checks = con.execute(
        "SELECT table_name, expression FROM duckdb_constraints() WHERE constraint_type = 'CHECK'"
    ).fetchall()
    ok = sorted(kinds) == sorted(RECORD_TABLES.values())
    for table, name in RECORD_TABLES.items():
        cols = con.execute(
            "SELECT column_name, data_type FROM information_schema.columns "
            "WHERE table_name = ? ORDER BY ordinal_position",
            [table],
        ).fetchall()
        ok &= cols[: len(HEADER)] == HEADER
        ok &= (table, f"(kind = {kinds[name]})") in checks
    print(f"   headers: {len(RECORD_TABLES)} record tables start with {[c for c, _ in HEADER]}")
    return ok


@dataclass
class Run:
    """A synthetic run's records, built in R4 order, with seq assigned as they are appended."""

    rows: dict[str, list[tuple[object, ...]]] = field(default_factory=dict)
    seq: int = 0
    event_id: int = 0

    def record(self, table: str, tick: int, *payload: object) -> None:
        """Append one record with the next seq."""
        kind = list(RECORD_TABLES).index(table) + 1
        self.rows.setdefault(table, []).append((0, self.seq, tick, kind, *payload))
        self.seq += 1

    def child(self, table: str, *row: object) -> None:
        """Append one child row (no seq)."""
        self.rows.setdefault(table, []).append((0, *row))


def instant(run: Run, tick: int, living: set[int], deaths: list[int]) -> None:
    """Write one instant's records in R4 order: input, events, life records, digest, keyframe."""
    if tick == INPUT_TICK:
        run.record(
            "input", tick, 0, 0, "resample", "person", 0x1111, "script", b"\0" * 16, b"\1" * 16
        )
    exits = []
    for p in sorted(q for q, t in DEATH_TICKS.items() if t == tick):
        living.discard(p)
        deaths.append(p)
        run.record("event", tick, run.event_id, 4, DEATH, 1 << 15, None, 0, 128, 0, 0, [1])
        run.child("event_role", run.event_id, 0, 1, p, TARGET)
        exits.append((p, run.event_id))
        run.event_id += 1
        if not living:
            run.record("event", tick, run.event_id, 4, EXTINCTION, 1 << 15, None, 0, 255, 0, 0, [])
            run.child("causal_edge", run.event_id - 1, run.event_id, TRIGGERED)
            run.event_id += 1
    if tick == 0:
        for p in FOUNDERS:
            run.record("life_course_entry", 0, p, 1, p % 2, 0, 0, -(20 + p % 7) * YEAR)
    for p, eid in exits:
        run.record("life_course_exit", tick, p, 0, 1, eid)
    if tick % MONTH == 0:
        run.record("step_digest", tick, len(living), 0, len(deaths), tick, 0, 0)
        deaths.clear()
    if tick == KEYFRAME_TICK:
        run.record("keyframe_marker", tick, "rehoboam/rh-snapshot/keyframe/v1", "kf/y1", tick, 1)


def synthetic_run() -> Run:
    """Return the records of three founders over two sim-years."""
    run, living, deaths = Run(), set(FOUNDERS), list[int]()
    ticks = sorted({m * MONTH for m in range(MONTHS)} | set(DEATH_TICKS.values()) | {INPUT_TICK})
    for tick in ticks:
        instant(run, tick, living, deaths)
    return run


def write_segments(con: duckdb.DuckDBPyConnection, run: Run, root: Path) -> list[Path]:
    """Seal one directory per sim-year (R5) and return the directories."""
    for table, rows in run.rows.items():
        con.executemany(f"INSERT INTO {table} VALUES ({', '.join('?' * len(rows[0]))})", rows)
    con.execute("INSERT INTO verb VALUES (0, 'DEATH', 32768, 128, 'M2.demography', false)")
    con.execute("INSERT INTO verb VALUES (1, 'EXTINCTION', 32768, 255, 'M2.demography', false)")
    con.execute("INSERT INTO act_class VALUES (15, 'death')")
    dirs = []
    for (year,) in con.execute(
        f"SELECT DISTINCT tick // {YEAR} FROM record_header ORDER BY 1"
    ).fetchall():
        d = root / "stream" / f"t{0:010d}" / "A" / f"y{year:06d}"
        d.mkdir(parents=True)
        span = f"tick >= {year * YEAR} AND tick < {(year + 1) * YEAR}"
        for table in RECORD_TABLES:
            con.execute(
                f"COPY (SELECT * FROM {table} WHERE {span} ORDER BY seq) TO '{d / table}.parquet'"
            )
        ids = f"SELECT event_id FROM event WHERE {span}"
        con.execute(
            f"COPY (SELECT * FROM event_role WHERE event_id IN ({ids})) TO '{d}/event_role.parquet'"
        )
        con.execute(
            f"COPY (SELECT * FROM causal_edge WHERE effect_id IN ({ids})) "
            f"TO '{d}/causal_edge.parquet'"
        )
        for table in DICTIONARIES:
            con.execute(f"COPY {table} TO '{d / table}.parquet'")
        con.execute(
            f"INSERT INTO segment SELECT 0, 'A', {year}, '{FORMAT}', min(seq), max(seq), count(*), "
            "min(tick), max(tick), 'test', '0.2.0', '\\x00'::BLOB, 1, ['M2.demography'], "
            f"['M4.economy'], '\\x00'::BLOB FROM record_header WHERE {span}"
        )
        con.execute(
            f"COPY (SELECT * FROM segment WHERE sim_year = {year}) TO '{d}/segment.parquet'"
        )
        dirs.append(d)
    return dirs


def read_back(root: Path) -> duckdb.DuckDBPyConnection:
    """Return a fresh database holding every sealed segment under ``root``."""
    con = load(DDL.read_text(encoding="utf-8"))
    for table in [*RECORD_TABLES, *CHILD_TABLES, "segment"]:
        con.execute(
            f"INSERT INTO {table} SELECT * "
            f"FROM read_parquet('{root}/stream/t*/A/y*/{table}.parquet')"
        )
    return con


def check_segments() -> bool:
    """Check 3: write a synthetic run's segments, read them back, check R2, R4, R5 and the view."""
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        dirs = write_segments(load(DDL.read_text(encoding="utf-8")), synthetic_run(), root)
        files = sorted(root.glob("stream/t*/A/y*/*.parquet"))
        con = read_back(root)
        floats = [
            f"{f.relative_to(root)}: {name} {ty}"
            for f in files
            for name, ty, *_ in duckdb.sql(f"DESCRIBE SELECT * FROM read_parquet('{f}')").fetchall()
            if any(x in ty.upper() for x in FLOAT_TYPES)
        ]
        violations = con.execute("SELECT count(*) FROM order_violation").fetchone()
        segs = con.execute(
            "SELECT first_seq, last_seq, record_count FROM segment ORDER BY sim_year"
        ).fetchall()
        chained = all(a[1] + 1 == b[0] for a, b in itertools.pairwise(segs)) and segs[0][0] == 0
        lc = con.execute(
            "SELECT person_id, death_tick, death_cause, exit_kind FROM life_course_v ORDER BY 1"
        ).fetchall()
        want = [(p, DEATH_TICKS[p], 1, 0) for p in sorted(FOUNDERS)]
        per_dir = {len(list(d.glob("*.parquet"))) for d in dirs}
        print(
            f"   segments: {len(dirs)} sim-years, {len(files)} files ({per_dir} per segment), "
            f"seq ranges {[(a, b) for a, b, _ in segs]}, "
            f"float columns in files: {floats or 'none'}, "
            f"order violations: {violations}"
        )
    return not floats and violations == (0,) and chained and lc == want and per_dir == {12}


def check_order_control() -> bool:
    """Check 4: swapping the seq of two records of one instant is an order violation."""
    run = synthetic_run()
    con = load(DDL.read_text(encoding="utf-8"))
    for table, rows in run.rows.items():
        con.executemany(f"INSERT INTO {table} VALUES ({', '.join('?' * len(rows[0]))})", rows)
    clean = con.execute("SELECT count(*) FROM order_violation").fetchone()
    first_exit = con.execute("SELECT seq FROM life_course_exit ORDER BY seq LIMIT 1").fetchone()
    assert first_exit is not None
    death = first_exit[0] - 1  # the DEATH event just before it, same instant
    con.execute(f"UPDATE life_course_exit SET seq = {death} WHERE seq = {first_exit[0]}")
    con.execute(f"UPDATE event SET seq = {first_exit[0]} WHERE seq = {death}")
    swapped = con.execute("SELECT count(*) FROM order_violation").fetchone()
    print(f"   order control: clean {clean}, after swapping an exit before its DEATH {swapped}")
    return clean == (0,) and swapped is not None and swapped[0] > 0


def main() -> int:
    """Run every check and print PASS or FAIL."""
    checks = [
        ("1. no float columns", check_no_floats),
        ("2. record headers", check_headers),
        ("3. sealed segments round-trip", check_segments),
        ("4. order positive control", check_order_control),
    ]
    ok = True
    for name, fn in checks:
        good = fn()
        ok &= good
        print(f"{name}: {'PASS' if good else 'FAIL'}")
    print("CHANNEL A v0:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
