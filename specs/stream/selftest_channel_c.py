"""Self-test of the World Stream v0 channel-C segment schema (SPR-011 ruling 14; SPR-013 T-11).

It loads ``specs/stream/channel_c_v0.sql`` into an in-memory DuckDB and checks:

  1. no float column anywhere (R2), and the check itself refuses a planted DOUBLE column;
  2. the record table starts with the header (timeline_id, seq, tick, kind) with the R3 types, and
     its kind CHECK matches the record_kind dictionary;
  3. the ``life_ends`` matches of ``selftest_channel_a.py``'s synthetic run (three founders, three
     deaths, one extinction over two sim-years) are written as sealed channel-C segments in the R7
     layout next to the channel-A segments, one Parquet file per table per sim-year, and read back:
     no float column in any written file, seq contiguous from 0 across segments, the R4 order
     holds (the order_violation view is empty), the segment rows chain, every match's canonical
     line (canonical_v0.py) is rebuilt from the stored rows alone (pack, semantics, bindings and
     events included) and the segments' canonical digests match it (R8), and the channel-A digest
     chain of every segment recomputes from the stored digests (R7);
  4. a SQL twin of ``life_ends`` over the channel-A tables gives the same matches: closing event,
     binding and content-addressed match id (V-SIFT-01 in miniature; R5);
  5. the order check refuses a planted swap of two records;
  6. a sim-year with no match seals an empty segment whose digest is that of no lines, and the
     segment CHECK refuses an empty segment with a last_seq;
  7. sift_match_v has the columns of specs/chronicle/schema.sql's sift_match, in order and type,
     except ``score`` (R2).

Run: python -I specs/stream/selftest_channel_c.py [--ddl PATH]
     (``--ddl`` loads another channel-C DDL: the positive control plants a float column)
"""

from __future__ import annotations

import argparse
import importlib.util
import itertools
import sys
import tempfile
from pathlib import Path
from types import ModuleType

import duckdb

HERE = Path(__file__).resolve().parent
DDL = HERE / "channel_c_v0.sql"
CHRONICLE = HERE.parent / "chronicle" / "schema.sql"
HEADER = [
    ("timeline_id", "UINTEGER"),
    ("seq", "UBIGINT"),
    ("tick", "BIGINT"),
    ("kind", "USMALLINT"),
]
MATCH_KIND = 1
RECORD_TABLE = "sift_match"
CHILD_TABLES = ("sift_binding", "sift_match_event")
DICTIONARIES = ("record_kind", "pattern")
FILES_PER_SEGMENT = 6
FLOAT_TYPES = ("FLOAT", "REAL", "DOUBLE")
FORMAT = "rehoboam/stream/channel-c/v0"
PACK, PATTERN, VERSION = "solitary_life", "life_ends", 1
LATEST, CLEAR_BINDING, COMPLETE = 0, 0, 1
PERSON = 1  # rh-ids EntityKind
EMPTY_DIGEST = "99aa06d3014798d86001c324468d497f"
D_TEST = b"\x00" * 16
D_CHAIN = b"rehoboam/stream/channel-c/v0/a-chain"


def _load(path: Path) -> ModuleType:
    """Import a sibling module by path (``python -I`` adds no directory to ``sys.path``)."""
    spec = importlib.util.spec_from_file_location(path.stem, path)
    if spec is None or spec.loader is None:
        raise ImportError(str(path))
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module  # dataclasses resolve their module through sys.modules
    spec.loader.exec_module(module)
    return module


CANON = _load(HERE / "canonical_v0.py")
CHANNEL_A = _load(HERE / "selftest_channel_a.py")
YEAR = int(CHANNEL_A.YEAR)
DEATH = int(CHANNEL_A.DEATH)
TARGET = int(CHANNEL_A.TARGET)

# One life_ends match: (seq, tick, match_id, closing event id, its channel-A seq, the deceased).
Match = tuple[int, int, int, int, int, int]


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


def check_no_floats(ddl: str) -> bool:
    """Check 1: no float column, and the check catches a planted one."""
    found = float_columns(load(ddl))
    planted = ddl.replace("opened_tick        BIGINT", "opened_tick        DOUBLE", 1)
    caught = float_columns(load(planted))
    print(f"   float columns: {found or 'none'}; planted control caught {caught}")
    expected = {"sift_match.opened_tick DOUBLE", "sift_match_v.opened_tick DOUBLE"}
    return not found and set(caught) == expected


def check_headers(ddl: str) -> bool:
    """Check 2: the record table starts with the R3 header, and its kind matches record_kind."""
    con = load(ddl)
    kinds = dict(con.execute("SELECT name, kind FROM record_kind").fetchall())
    checks = con.execute(
        "SELECT table_name, expression FROM duckdb_constraints() WHERE constraint_type = 'CHECK'"
    ).fetchall()
    cols = columns(con, RECORD_TABLE)
    ok = cols[: len(HEADER)] == HEADER
    ok &= kinds == {"MATCH": MATCH_KIND}
    ok &= (RECORD_TABLE, f"(kind = {MATCH_KIND})") in checks
    print(f"   headers: {RECORD_TABLE} starts with {[c for c, _ in HEADER]}; kinds {kinds}")
    return ok


def life_ends_matches(run: object) -> list[Match]:
    """The life_ends matches of a channel-A run: one per DEATH, binding A to the target."""
    rows = getattr(run, "rows")  # noqa: B009  (the dynamically loaded Run dataclass)
    targets = {
        row[1]: row[4] for row in rows["event_role"] if row[5] == TARGET
    }  # event_id -> person
    deaths = sorted((row[1], row) for row in rows["event"] if row[6] == DEATH)  # by channel-A seq
    out = []
    for seq, (a_seq, row) in enumerate(deaths):
        event_id, tick = row[4], row[2]
        mid = int(
            CANON.match_id(PACK, PATTERN, VERSION, [event_id], [("A", PERSON, targets[event_id])])
        )
        out.append((seq, tick, mid, event_id, a_seq, targets[event_id]))
    return out


def match_record(m: Match) -> object:
    """The canonical record of a life_ends match."""
    seq, tick, mid, event_id, a_seq, person = m
    fields = (0, seq, tick, mid, PACK, PATTERN, VERSION, LATEST, CLEAR_BINDING, COMPLETE, tick)
    return CANON.Record(
        "MATCH", (*fields, event_id, a_seq, (("A", PERSON, person),), (("e1", event_id),))
    )


def insert_matches(con: duckdb.DuckDBPyConnection, matches: list[Match]) -> None:
    """Insert the matches, their child rows and the pattern dictionary."""
    for seq, tick, mid, event_id, a_seq, person in matches:
        con.execute(
            "INSERT INTO sift_match VALUES (0, ?, ?, 1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                seq,
                tick,
                mid,
                PACK,
                PATTERN,
                VERSION,
                LATEST,
                CLEAR_BINDING,
                COMPLETE,
                tick,
                event_id,
                a_seq,
            ],
        )
        con.execute("INSERT INTO sift_binding VALUES (0, ?, 'A', ?, ?)", [mid, PERSON, person])
        con.execute("INSERT INTO sift_match_event VALUES (0, ?, 'e1', 0, ?)", [mid, event_id])
    con.execute(
        "INSERT INTO pattern VALUES (?, ?, ?, ?, ?, 1, ['A'], [], 0, 333333, 1, ?)",
        [PACK, PATTERN, VERSION, LATEST, CLEAR_BINDING, D_TEST],
    )


def a_digest(year: int) -> bytes:
    """A stand-in for the channel-A segment digest of ``year`` (the self-test has no channel A)."""
    return bytes(CANON.xxh3_128(b"channel A of year " + str(year).encode()))


def a_chain(previous: bytes, digest: bytes) -> bytes:
    """The channel-A digest chain of R7: XXH3-128(D_CHAIN || chain(y - 1) || a_digest(y))."""
    return bytes(CANON.xxh3_128(D_CHAIN + previous + digest))


def seal_year(con: duckdb.DuckDBPyConnection, d: Path, year: int, lines: list[str]) -> None:
    """Seal one sim-year's segment: every table, empty or not, then the segment row (R7).

    The chain starts at ``a_chain(-1)`` and folds in every earlier year's stand-in digest.
    """
    d.mkdir(parents=True)
    chain = bytes(CANON.xxh3_128(D_CHAIN + (0).to_bytes(4, "little")))
    for y in range(year + 1):
        chain = a_chain(chain, a_digest(y))
    span = f"tick >= {year * YEAR} AND tick < {(year + 1) * YEAR}"
    con.execute(
        f"COPY (SELECT * FROM sift_match WHERE {span} ORDER BY seq) TO '{d}/sift_match.parquet'"
    )
    ids = f"SELECT match_id FROM sift_match WHERE {span}"
    for table in CHILD_TABLES:
        con.execute(
            f"COPY (SELECT * FROM {table} WHERE match_id IN ({ids})) TO '{d}/{table}.parquet'"
        )
    for table in DICTIONARIES:
        con.execute(f"COPY {table} TO '{d}/{table}.parquet'")
    digest = bytes(CANON.digest(lines))
    con.execute(
        f"INSERT INTO segment SELECT 0, 'C', {year}, '{FORMAT}', "
        "coalesce(min(seq), (SELECT count(*) FROM sift_match)), max(seq), count(*), "
        "min(tick), max(tick), 'test-sifter', 0, [?], 'test', '0.2.0', ?, 1, "
        "(SELECT max(closed_by_seq) FROM sift_match), ?, ?, ? "
        f"FROM sift_match WHERE {span}",
        [PACK, D_TEST, a_digest(year), chain, digest],
    )
    con.execute(f"COPY (SELECT * FROM segment WHERE sim_year = {year}) TO '{d}/segment.parquet'")


def write_segments(ddl: str, matches: list[Match], root: Path) -> list[Path]:
    """Seal one channel-C directory per sim-year of the matches (R7) and return the directories."""
    con = load(ddl)
    insert_matches(con, matches)
    dirs = []
    for year in sorted({tick // YEAR for _, tick, *_ in matches}):
        d = root / "stream" / f"t{0:010d}" / "C" / f"y{year:06d}"
        lines = [str(CANON.render(match_record(m))) for m in matches if m[1] // YEAR == year]
        seal_year(con, d, year, lines)
        dirs.append(d)
    return dirs


def read_back(ddl: str, root: Path) -> duckdb.DuckDBPyConnection:
    """Return a fresh database holding every sealed channel-C segment under ``root``."""
    con = load(ddl)
    for table in [RECORD_TABLE, *CHILD_TABLES, "pattern", "segment"]:
        con.execute(  # the pattern dictionary repeats in every segment (R7): ignore the duplicates
            f"INSERT {'OR IGNORE ' if table == 'pattern' else ''}INTO {table} SELECT * "
            f"FROM read_parquet('{root}/stream/t*/C/y*/{table}.parquet')"
        )
    return con


STORED = """
SELECT m.timeline_id, m.seq, m.tick, m.match_id, m.pack, m.pattern_id, m.pattern_version,
       m.selection, m.on_complete, m.status, m.opened_tick, m.closed_by_event_id, m.closed_by_seq,
       (SELECT list(struct_pack(role := role, entity_kind := entity_kind, entity_id := entity_id)
                    ORDER BY encode(role))
          FROM sift_binding b WHERE b.timeline_id = m.timeline_id AND b.match_id = m.match_id),
       (SELECT list(struct_pack(clause := clause, event_id := event_id) ORDER BY ordinal)
          FROM sift_match_event e WHERE e.timeline_id = m.timeline_id AND e.match_id = m.match_id)
FROM sift_match m ORDER BY m.seq
"""


def stored_lines(con: duckdb.DuckDBPyConnection) -> dict[int, list[str]]:
    """The canonical MATCH lines rebuilt from the stored rows alone, per sim-year, in seq order."""
    out: dict[int, list[str]] = {}
    for *scalars, bindings, events in con.execute(STORED).fetchall():
        fields = (
            *scalars,
            tuple((b["role"], b["entity_kind"], b["entity_id"]) for b in bindings or []),
            tuple((e["clause"], e["event_id"]) for e in events or []),
        )
        out.setdefault(scalars[2] // YEAR, []).append(
            str(CANON.render(CANON.Record("MATCH", fields)))
        )
    return out


def check_segments(ddl: str) -> bool:
    """Check 3: write the synthetic run's matches as segments, read them back: R2, R4, R7, R8."""
    matches = life_ends_matches(CHANNEL_A.synthetic_run())
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        dirs = write_segments(ddl, matches, root)
        files = sorted(root.glob("stream/t*/C/y*/*.parquet"))
        con = read_back(ddl, root)
        floats = [
            f"{f.relative_to(root)}: {name} {ty}"
            for f in files
            for name, ty, *_ in duckdb.sql(f"DESCRIBE SELECT * FROM read_parquet('{f}')").fetchall()
            if any(x in ty.upper() for x in FLOAT_TYPES)
        ]
        violations = con.execute("SELECT count(*) FROM order_violation").fetchone()
        segs = con.execute(
            "SELECT sim_year, first_seq, last_seq, record_count, canonical_digest, "
            "a_canonical_digest, a_chain FROM segment ORDER BY sim_year"
        ).fetchall()
        chained = segs[0][1] == 0 and all(a[1] + a[3] == b[1] for a, b in itertools.pairwise(segs))
        rebuilt = stored_lines(con)
        digests_ok = all(bytes(CANON.digest(rebuilt.get(s[0], []))) == s[4] for s in segs)
        chain = bytes(CANON.xxh3_128(D_CHAIN + (0).to_bytes(4, "little")))
        a_chains_ok = True
        for s in segs:
            chain = a_chain(chain, s[5])
            a_chains_ok &= chain == s[6]
        per_dir = {len(list(d.glob("*.parquet"))) for d in dirs}
    print(
        f"   segments: {len(dirs)} sim-years, {len(files)} files ({per_dir} per segment), "
        f"{len(matches)} matches, seq ranges {[(s[1], s[2]) for s in segs]}, "
        f"float columns in files: {floats or 'none'}, order violations: {violations}, "
        f"digests rebuilt from rows: {'equal' if digests_ok else 'DIFFER'}, "
        f"channel-A chain: {'equal' if a_chains_ok else 'DIFFER'}"
    )
    return (
        not floats
        and violations == (0,)
        and chained
        and digests_ok
        and a_chains_ok
        and per_dir == {FILES_PER_SEGMENT}
        and len(matches) == len(CHANNEL_A.FOUNDERS)
    )


TWIN = """
SELECT e.seq, e.event_id, e.tick, r.entity_id
FROM event e JOIN event_role r ON r.timeline_id = e.timeline_id AND r.event_id = e.event_id
WHERE e.verb_id = ? AND r.role = ?
ORDER BY e.seq
"""


def check_twin(ddl: str) -> bool:
    """Check 4: the SQL twin of life_ends over channel A gives the same matches and match ids."""
    run = CHANNEL_A.synthetic_run()
    con_a = load(CHANNEL_A.DDL.read_text(encoding="utf-8"))
    for table, rows in run.rows.items():
        con_a.executemany(f"INSERT INTO {table} VALUES ({', '.join('?' * len(rows[0]))})", rows)
    twin = con_a.execute(TWIN, [DEATH, TARGET]).fetchall()
    twin_ids = [
        int(CANON.match_id(PACK, PATTERN, VERSION, [e], [("A", PERSON, p)])) for _, e, _, p in twin
    ]
    con_c = load(ddl)
    insert_matches(con_c, life_ends_matches(run))
    sifted = con_c.execute(
        "SELECT m.closed_by_seq, m.closed_by_event_id, m.tick, b.entity_id, m.match_id "
        "FROM sift_match m JOIN sift_binding b USING (timeline_id, match_id) ORDER BY m.seq"
    ).fetchall()
    same = [tuple(s[:4]) for s in sifted] == twin and [s[4] for s in sifted] == twin_ids
    print(
        f"   twin: {len(twin)} matches from SQL, {len(sifted)} sifted; "
        f"{'equal' if same else 'DIFFER'}"
    )
    return same and len(twin) == len(CHANNEL_A.FOUNDERS)


def check_order_control(ddl: str) -> bool:
    """Check 5: swapping the seq of two records is an order violation."""
    con = load(ddl)
    insert_matches(con, life_ends_matches(CHANNEL_A.synthetic_run()))
    clean = con.execute("SELECT count(*) FROM order_violation").fetchone()
    con.execute("UPDATE sift_match SET seq = CASE seq WHEN 0 THEN 1 WHEN 1 THEN 0 ELSE seq END")
    swapped = con.execute("SELECT count(*) FROM order_violation").fetchone()
    print(f"   order control: clean {clean}, after swapping the first two matches {swapped}")
    return clean == (0,) and swapped is not None and swapped[0] > 0


def check_empty_segment(ddl: str) -> bool:
    """Check 6: an empty segment is accepted with the empty digest; a malformed one is refused."""
    con = load(ddl)
    empty = bytes(CANON.digest([]))
    con.execute(
        "INSERT INTO segment VALUES (1, 'C', 7, ?, 5, NULL, 0, NULL, NULL, 'test-sifter', 0, [?], "
        "'test', '0.2.0', ?, 1, 40, ?, ?, ?)",
        [FORMAT, PACK, D_TEST, D_TEST, D_TEST, empty],
    )
    accepted = con.execute("SELECT count(*) FROM segment").fetchone() == (1,)
    try:
        con.execute(
            "INSERT INTO segment VALUES (1, 'C', 8, ?, 5, 5, 0, NULL, NULL, 'test-sifter', 0, [?], "
            "'test', '0.2.0', ?, 1, 40, ?, ?, ?)",
            [FORMAT, PACK, D_TEST, D_TEST, D_TEST, empty],
        )
        refused = False
    except duckdb.ConstraintException:
        refused = True
    print(
        f"   empty segment: accepted {accepted}, digest {empty.hex()}, malformed refused {refused}"
    )
    return accepted and refused and empty.hex() == EMPTY_DIGEST


def columns(con: duckdb.DuckDBPyConnection, table: str) -> list[tuple[str, str]]:
    """Return (name, type) of every column of ``table``, in order."""
    return con.execute(
        "SELECT column_name, data_type FROM information_schema.columns "
        "WHERE table_name = ? ORDER BY ordinal_position",
        [table],
    ).fetchall()


def check_chronicle_view(ddl: str) -> bool:
    """Check 7: sift_match_v has schema.sql's sift_match columns, in order and type, minus score."""
    view = columns(load(ddl), "sift_match_v")
    table = [
        c
        for c in columns(load(CHRONICLE.read_text(encoding="utf-8")), "sift_match")
        if c[0] != "score"
    ]
    print(f"   sift_match_v: {[c for c, _ in view]}")
    if view != table:
        print(f"   schema.sql sift_match (minus score): {table}")
    return view == table


def main(argv: list[str] | None = None) -> int:
    """Run every check and print PASS or FAIL."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--ddl", type=Path, default=DDL, help="the channel-C DDL to test")
    args = parser.parse_args(argv)
    ddl = args.ddl.read_text(encoding="utf-8")
    checks = [
        ("1. no float columns", check_no_floats),
        ("2. record header", check_headers),
        ("3. sealed segments round-trip and digests", check_segments),
        ("4. life_ends SQL twin equivalence", check_twin),
        ("5. order positive control", check_order_control),
        ("6. empty segment", check_empty_segment),
        ("7. sift_match_v = schema.sql sift_match minus score", check_chronicle_view),
    ]
    ok = True
    for name, fn in checks:
        try:
            good = fn(ddl)
        except (duckdb.Error, ValueError, TypeError) as e:  # a check that cannot run has failed
            print(f"   error: {type(e).__name__}: {e}")
            good = False
        ok &= good
        print(f"{name}: {'PASS' if good else 'FAIL'}")
    print("CHANNEL C v0:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
