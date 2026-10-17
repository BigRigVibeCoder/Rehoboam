"""Check a run's sealed channel-A segments with DuckDB (SPR-013 T-09; V-DATA-10 subset; V-STR-01).

Reads every sealed table of ``<run-dir>/stream/t*/A/y*/`` with DuckDB, independently of
``rh-stream``'s reader, and checks:

  1. every segment holds exactly the format's sixteen files (the SPR-020 envelope's shape), and each
     file's columns, read by DuckDB, have exactly the names and types of its table in
     ``specs/stream/channel_a_v0.sql``; a column that ``specs/chronicle/schema.sql`` also has
     (``event``, ``event_role``, ``event_snapshot``, ``causal_edge``) has schema.sql's type; no
     column is a float (R2);
  2. every row loads into the DDL's tables (its PRIMARY KEY, UNIQUE and CHECK constraints), and
     ``life_course_v`` rebuilds schema.sql's ``life_course`` with its exact columns and types;
  3. ``seq`` is contiguous from 0 across the segments, each segment row agrees with its records, and
     the R4 order holds (``order_violation`` is empty); role ordinals are contiguous from 0;
  4. each segment's ``canonical_digest`` equals the digest that ``specs/stream/canonical_v0.py``,
     the standard-library reference, computes from the rows (the canonical line of every record,
     rebuilt from its table, its child rows (roles, snapshots, typed causes) and the segment's
     verb dictionary).

``--plant float-column`` and ``--plant digest`` copy the run directory, plant a DOUBLE column in
one segment's ``step_digest.parquet`` or alter one ``canonical_digest``, and check the copy: each
must FAIL, with ``TYPE:`` and ``DIGEST:`` respectively (the positive controls).

Usage: python -I specs/stream/check_segments.py RUN_DIR [--plant float-column|digest]
Exit:  0 every check passed, 1 a check failed, 2 the run directory could not be read.
"""

from __future__ import annotations

import argparse
import importlib.util
import itertools
import shutil
import sys
import tempfile
from pathlib import Path
from types import ModuleType

import duckdb

HERE = Path(__file__).resolve().parent
DDL_A = HERE / "channel_a_v0.sql"
SCHEMA = HERE.parents[1] / "specs" / "chronicle" / "schema.sql"
CANONICAL = HERE / "canonical_v0.py"
TABLES = (
    "act_class",
    "attribute",
    "causal_edge",
    "event",
    "event_role",
    "event_role_kind",
    "event_snapshot",
    "input",
    "keyframe_marker",
    "life_course_entry",
    "life_course_exit",
    "motive_edge_kind",
    "record_kind",
    "segment",
    "step_digest",
    "verb",
)
FLOAT_TYPES = ("FLOAT", "REAL", "DOUBLE")
DICTIONARIES = ("event_role_kind", "motive_edge_kind", "attribute")
EXIT_OK, EXIT_FAIL, EXIT_UNREADABLE = 0, 1, 2


def _load(path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(path.stem, path)
    if spec is None or spec.loader is None:
        raise ImportError(str(path))
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module
    spec.loader.exec_module(module)
    return module


CANON = _load(CANONICAL)


def columns(con: duckdb.DuckDBPyConnection, table: str) -> list[tuple[str, str]]:
    """(name, type) of every column of ``table``, in order."""
    return con.execute(
        "SELECT column_name, data_type FROM information_schema.columns "
        "WHERE table_name = ? ORDER BY ordinal_position",
        [table],
    ).fetchall()


def file_columns(path: Path) -> list[tuple[str, str]]:
    """(name, type) of every column of a Parquet file as DuckDB reads it."""
    rows = duckdb.sql(f"DESCRIBE SELECT * FROM read_parquet('{path}')").fetchall()
    return [(r[0], r[1]) for r in rows]


def check_types(segments: list[Path], ddl: duckdb.DuckDBPyConnection) -> list[str]:
    """Check 1: the file set and every file's columns against both DDLs."""
    problems = []
    chronicle = duckdb.connect()
    chronicle.execute(SCHEMA.read_text(encoding="utf-8"))
    for seg in segments:
        names = sorted(p.name for p in seg.iterdir())
        if names != sorted(f"{t}.parquet" for t in TABLES):
            problems.append(f"FILES: {seg.name} holds {names}")
            continue
        for table in TABLES:
            got = file_columns(seg / f"{table}.parquet")
            want = columns(ddl, table)
            if got != want:
                problems.append(f"TYPE: {seg.name}/{table}: {got} is not the DDL's {want}")
            floats = [f"{c} {t}" for c, t in got if any(f in t.upper() for f in FLOAT_TYPES)]
            if floats:
                problems.append(f"TYPE: {seg.name}/{table}: float columns {floats}")
            shared = dict(columns(chronicle, table))
            for c, t in got:
                if c in shared and shared[c] != t:
                    problems.append(
                        f"TYPE: {seg.name}/{table}.{c} is {t}, schema.sql says {shared[c]}"
                    )
    return problems


def load_rows(root: Path, con: duckdb.DuckDBPyConnection) -> None:
    """Check 2: load every sealed row into the DDL's tables (constraints apply)."""
    for table in TABLES:
        if table in ("record_kind", "verb", "act_class"):
            continue
        files = f"{root}/stream/t*/A/y*/{table}.parquet"
        # Every segment carries the envelope's dictionaries: loaded once each, so two segments
        # that disagree on a code break its PRIMARY KEY.
        distinct = "DISTINCT " if table in DICTIONARIES else ""
        con.execute(f"INSERT INTO {table} SELECT {distinct}* FROM read_parquet('{files}')")


def check_order(con: duckdb.DuckDBPyConnection) -> list[str]:
    """Check 3: contiguous seq, segment rows, R4 order, role ordinals, the life_course view."""
    problems = []
    segs = con.execute(
        "SELECT sim_year, first_seq, last_seq, record_count FROM segment ORDER BY sim_year"
    ).fetchall()
    if not segs or segs[0][1] != 0:
        problems.append("SEQ: the first segment does not start at seq 0")
    for a, b in itertools.pairwise(segs):
        if a[2] + 1 != b[1]:
            problems.append(f"SEQ: segment {b[0]} starts at {b[1]}, not {a[2] + 1}")
    for year, first, last, count in segs:
        n = con.execute(
            "SELECT count(*), min(seq), max(seq) FROM record_header r WHERE r.seq BETWEEN ? AND ?",
            [first, last],
        ).fetchone()
        if n != (count, first, last):
            problems.append(f"SEQ: segment {year}'s row disagrees with its records ({n})")
    violations = con.execute("SELECT count(*) FROM order_violation").fetchone()
    if violations != (0,):
        problems.append(f"ORDER: {violations[0] if violations else '?'} records out of R4 order")
    gaps = con.execute(
        "SELECT count(*) FROM (SELECT event_id, min(ordinal) lo, max(ordinal) hi, count(*) n "
        "FROM event_role GROUP BY event_id) WHERE lo <> 0 OR hi <> n - 1"
    ).fetchone()
    if gaps != (0,):
        problems.append("ORDER: an event's role ordinals are not 0..n-1")
    chronicle = duckdb.connect()
    chronicle.execute(SCHEMA.read_text(encoding="utf-8"))
    if columns(con, "life_course_v") != columns(chronicle, "life_course"):
        problems.append("VIEW: life_course_v does not have schema.sql's life_course columns")
    entries = con.execute("SELECT count(*) FROM life_course_entry").fetchone()
    view = con.execute("SELECT count(*) FROM life_course_v").fetchone()
    if entries != view:
        problems.append(f"VIEW: {view} life_course_v rows for {entries} entries")
    return problems


def _lines(seg: Path) -> list[str]:
    """The canonical lines of one segment, rebuilt from its tables by the reference."""
    con = duckdb.connect()
    t = {name: f"read_parquet('{seg}/{name}.parquet')" for name in TABLES}
    verbs = dict(con.execute(f"SELECT verb_id, name FROM {t['verb']}").fetchall())
    roles: dict[int, list[tuple[int, int, int]]] = {}
    for e, _o, k, i, r in con.execute(
        f"SELECT event_id, ordinal, entity_kind, entity_id, role FROM {t['event_role']} "
        "ORDER BY event_id, ordinal"
    ).fetchall():
        roles.setdefault(e, []).append((k, i, r))
    snapshots: dict[int, list[tuple[object, ...]]] = {}
    for e, *snap in con.execute(
        "SELECT event_id, attr_id, entity_kind, entity_id, partner_kind, partner_id, value "
        f"FROM {t['event_snapshot']} ORDER BY event_id, attr_id, entity_kind, entity_id, "
        "partner_kind NULLS FIRST, partner_id NULLS FIRST"
    ).fetchall():
        snapshots.setdefault(e, []).append(tuple(snap))
    causes: dict[int, list[tuple[object, ...]]] = {}
    for c, e, k, motive, weight, evidence, told in con.execute(
        "SELECT cause_id, effect_id, kind, motive, weight, evidence, cause_told "
        f"FROM {t['causal_edge']} ORDER BY effect_id, cause_id"
    ).fetchall():
        causes.setdefault(e, []).append((c, k, motive, weight, evidence, int(told)))
    records: list[tuple[int, object]] = []
    head = "timeline_id, seq, tick"
    for row in con.execute(
        f"SELECT {head}, journal_seq, class, input_kind, scope, scope_id, producer, payload_hash, "
        f"row_digest FROM {t['input']}"
    ).fetchall():
        records.append((row[1], CANON.Record("INPUT", (*row[:9], bytes(row[9]), bytes(row[10])))))
    for row in con.execute(
        f"SELECT {head}, event_id, phase, verb_id, act_classes, place_id, magnitude, tellability, "
        f"secrecy, origin, payload FROM {t['event']}"
    ).fetchall():
        tl, seq, tick, eid, phase, vid, acts, place, mag, tell, sec, orig, payload = row
        fields = (
            tl,
            seq,
            tick,
            eid,
            phase,
            vid,
            verbs[vid],
            acts,
            place,
            mag,
            tell,
            sec,
            orig,
            tuple(payload),
            tuple(roles.get(eid, [])),
            tuple(snapshots.get(eid, [])),
            tuple(causes.get(eid, [])),
        )
        records.append((seq, CANON.Record("EVENT", fields)))
    simple = (
        (
            "LIFE_ENTRY",
            "life_course_entry",
            "person_id, origin, sex, mother_id, father_id, birth_tick",
        ),
        ("LIFE_EXIT", "life_course_exit", "person_id, exit_kind, death_cause, event_id"),
        (
            "STEP_DIGEST",
            "step_digest",
            "living, births, deaths, state_hash, observation_hash, narrative_hash",
        ),
        (
            "KEYFRAME",
            "keyframe_marker",
            "keyframe_format, keyframe_path, state_hash, journal_position",
        ),
    )
    for kind, table, cols in simple:
        for row in con.execute(f"SELECT {head}, {cols} FROM {t[table]}").fetchall():
            records.append((row[1], CANON.Record(kind, tuple(row))))
    return [CANON.render(r) for _, r in sorted(records, key=lambda x: x[0])]


def check_digests(segments: list[Path]) -> list[str]:
    """Check 4: every segment's canonical_digest is the reference's over the rebuilt lines."""
    problems = []
    for seg in segments:
        stored = duckdb.sql(
            f"SELECT canonical_digest FROM read_parquet('{seg}/segment.parquet')"
        ).fetchone()
        want = CANON.digest(_lines(seg))
        if stored is None or bytes(stored[0]) != want:
            problems.append(
                f"DIGEST: {seg.name}: stored {stored}, the reference computes {want.hex()}"
            )
    return problems


def check(run_dir: Path) -> int:
    """Run every check on a run directory; print the verdict."""
    segments = sorted(run_dir.glob("stream/t*/A/y[0-9][0-9][0-9][0-9][0-9][0-9]"))
    if not segments:
        print(f"check_segments: no sealed segment under {run_dir}/stream")
        return EXIT_UNREADABLE
    ddl = duckdb.connect()
    ddl.execute(DDL_A.read_text(encoding="utf-8"))
    problems = check_types(segments, ddl)
    if not problems:
        try:
            load_rows(run_dir, ddl)
        except duckdb.Error as e:
            problems.append(f"LOAD: a sealed row breaks the DDL: {e}")
    if not problems:
        problems += check_order(ddl)
        problems += check_digests(segments)
    for p in problems:
        print(p)
    if problems:
        print(f"check_segments: FAIL, {len(problems)} problems in {len(segments)} segments")
        return EXIT_FAIL
    records = ddl.execute("SELECT count(*) FROM record_header").fetchone()
    print(
        f"check_segments: {len(segments)} segments, {records[0] if records else 0} records: "
        "every table has the DDL's types (and schema.sql's where shared), no float column, the "
        "rows keep the DDL's constraints, seq contiguous from 0, R4 order, and every "
        "canonical_digest equals the reference's"
    )
    return EXIT_OK


def plant(run_dir: Path, what: str, tmp: Path) -> Path:
    """A copy of ``run_dir`` with one planted fault (the positive controls)."""
    copy = tmp / "run"
    shutil.copytree(run_dir / "stream", copy / "stream")
    seg = sorted(copy.glob("stream/t*/A/y*"))[0]
    con = duckdb.connect()
    if what == "float-column":
        target = seg / "step_digest.parquet"
        con.execute(
            f"COPY (SELECT * REPLACE (CAST(living AS DOUBLE) AS living) "
            f"FROM read_parquet('{target}')) TO '{tmp}/x.parquet' (FORMAT PARQUET)"
        )
    else:
        target = seg / "segment.parquet"
        con.execute(
            f"COPY (SELECT * REPLACE ('\\x00'::BLOB || canonical_digest[2:] AS canonical_digest) "
            f"FROM read_parquet('{target}')) TO '{tmp}/x.parquet' (FORMAT PARQUET)"
        )
    shutil.move(tmp / "x.parquet", target)
    return copy


def main(argv: list[str] | None = None) -> int:
    """Parse the arguments and check."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("--plant", choices=("float-column", "digest"))
    args = parser.parse_args(argv)
    if args.plant is None:
        return check(args.run_dir)
    with tempfile.TemporaryDirectory() as tmp:
        return check(plant(args.run_dir, args.plant, Path(tmp)))


if __name__ == "__main__":
    sys.exit(main())
