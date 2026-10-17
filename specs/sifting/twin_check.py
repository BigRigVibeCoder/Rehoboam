"""Check rh-sifter's channel C against the patterns' SQL twins (SPR-013 T-11; V-SIFT-01; SIFT-3).

For a run directory that ``rh-cli sift`` has sifted, this tool, independently of ``rh-sifter``:

  1. reads every sealed channel-A segment with DuckDB (views ``event``, ``event_role``,
     ``verb``) and runs the twin of each pack the channel-C segments name
     (``specs/sifting/twins/<pack>.sql``);
  2. turns each twin row into its match: the MatchId of ``specs/stream/canonical_v0.py``
     (channel_c_v0.sql R5), the bindings and the event ids;
  3. reads channel C (``sift_match``, ``sift_binding``, ``sift_match_event``) and requires the same
     matches in the same order, every column: match ids, closing seqs, event ids, bindings, the
     closing and opening ticks, the pack, pattern and version, and the declared selection and
     on_complete (from the pack file) and status 1 (V-SIFT-01; REV-SPR-013 MIN-2);
  4. checks that channel C has one segment per channel-A segment, each recording its channel-A
     segment's ``canonical_digest`` and the chain of every channel-A digest up to it (R7),
     recomputed here with the standard-library XXH3.

``--control drop-last`` drops the twin's last match before comparing, and ``--control
other-status`` changes the status the twin expects (a column the match id does not cover): each
must FAIL with ``TWIN DIFFERS`` (the positive controls).

  5. (BCK-002 A-58, VER-SPR-013-TEST GAP-10) checks every other column of channel C against what
     the twin and channel A determine: each match record's ``timeline_id``, ``seq`` (contiguous
     from 0 in R4 order) and ``kind`` (1, MATCH), the child rows' ``timeline_id``, every row of
     every segment's ``pattern`` table (from the pack files: selection, on_complete, clauses,
     roles, dedupe, the exact budget, the pack's XXH3-128), and every column of every ``segment``
     row (its seq range, ticks, the sifter build and DSL version, the packs, channel A's history
     tuple and last seq, and its canonical digest, recomputed from the twin's matches with
     ``canonical_v0.py``); ``--control other-seq``, ``other-pattern`` and ``other-segment`` must
     FAIL with ``RECORDS DIFFER``, ``PATTERN DIFFERS`` and ``SEGMENT DIFFERS``.

Usage: python -I specs/sifting/twin_check.py RUN_DIR
           [--control drop-last|other-status|other-seq|other-pattern|other-segment]
Exit:  0 equal, 1 a difference, 2 the run directory could not be read.
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
import tomllib
from fractions import Fraction
from pathlib import Path
from types import ModuleType

import duckdb

HERE = Path(__file__).resolve().parent
TWINS = HERE / "twins"
PACKS = HERE  # patterns.<pack>.toml
SELECTIONS = ("latest", "earliest", "all")
ON_COMPLETE = ("clear_binding", "consume", "keep")
COMPLETE = 1
CANONICAL = HERE.parent / "stream" / "canonical_v0.py"
D_CHAIN = b"rehoboam/stream/channel-c/v0/a-chain"
EXIT_OK, EXIT_DIFFER, EXIT_UNREADABLE = 0, 1, 2


def _load(path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(path.stem, path)
    if spec is None or spec.loader is None:
        raise ImportError(str(path))
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module
    spec.loader.exec_module(module)
    return module


CANON = _load(CANONICAL)

Match = tuple[object, ...]


def declared(pack: str) -> dict[tuple[str, int], tuple[int, int]]:
    """Each pattern's (id, version) -> its declared (selection, on_complete) codes."""
    data = tomllib.loads((PACKS / f"patterns.{pack}.toml").read_text(encoding="utf-8"))
    return {
        (p["id"], p["version"]): (
            SELECTIONS.index(p.get("selection", "latest")),
            ON_COMPLETE.index(p.get("on_complete", "clear_binding")),
        )
        for p in data["pattern"]
    }


def channel_a(con: duckdb.DuckDBPyConnection, run_dir: Path) -> None:
    """Views over every sealed channel-A segment of the run."""
    for table in ("event", "event_role", "verb", "segment"):
        files = f"{run_dir}/stream/t*/A/y[0-9]*/{table}.parquet"
        con.execute(f"CREATE VIEW a_{table} AS SELECT * FROM read_parquet('{files}')")
        if table != "segment":
            con.execute(f"CREATE VIEW {table} AS SELECT * FROM a_{table}")


def twin_matches(con: duckdb.DuckDBPyConnection, packs: list[str]) -> list[Match]:
    """The twins' matches, as (match_id, closed_by_seq, event ids, bindings), in R4 order."""
    out: list[tuple[tuple[object, ...], Match]] = []
    for pack in packs:
        sql = (TWINS / f"{pack}.sql").read_text(encoding="utf-8")
        semantics = declared(pack)
        for p, pattern, version, seq, tick, event_id, role, kind, entity in con.execute(
            sql
        ).fetchall():
            bindings = ((role, kind, entity),)
            mid = int(CANON.match_id(p, pattern, version, [event_id], list(bindings)))
            key = (seq, p.encode(), pattern.encode(), version, (event_id,), bindings)
            selection, on_complete = semantics[(pattern, version)]
            # One clause: the match opens and closes at its event (opened_tick = tick).
            row = (p, pattern, version, tick, tick, selection, on_complete, COMPLETE, event_id)
            out.append((key, (mid, seq, (event_id,), bindings, row)))
    return [m for _, m in sorted(out, key=lambda x: x[0])]


def sifted_matches(con: duckdb.DuckDBPyConnection, run_dir: Path) -> list[Match]:
    """rh-sifter's matches from channel C, in seq order."""
    c = f"{run_dir}/stream/t*/C/y[0-9]*"
    rows = con.execute(
        "SELECT match_id, closed_by_seq, pack, pattern_id, pattern_version, tick, opened_tick, "
        "selection, on_complete, status, closed_by_event_id "
        f"FROM read_parquet('{c}/sift_match.parquet') ORDER BY seq"
    ).fetchall()
    events: dict[int, list[int]] = {}
    for mid, eid in con.execute(
        f"SELECT match_id, event_id FROM read_parquet('{c}/sift_match_event.parquet') "
        "ORDER BY match_id, ordinal"
    ).fetchall():
        events.setdefault(mid, []).append(eid)
    binds: dict[int, list[tuple[str, int, int]]] = {}
    for mid, role, kind, entity in con.execute(
        "SELECT match_id, role, entity_kind, entity_id "
        f"FROM read_parquet('{c}/sift_binding.parquet')"
    ).fetchall():
        binds.setdefault(mid, []).append((role, kind, entity))
    return [
        (
            mid,
            seq,
            tuple(events.get(mid, [])),
            tuple(sorted(binds.get(mid, []), key=lambda b: b[0].encode())),
            tuple(rest),
        )
        for mid, seq, *rest in rows
    ]


SIFTER_BUILD = (
    "rh-sifter "
    + tomllib.loads((HERE.parent.parent / "Cargo.toml").read_text(encoding="utf-8"))["workspace"][
        "package"
    ]["version"]
)
FORMAT_C = "rehoboam/stream/channel-c/v0"
MATCH_KIND = 1
DSL_VERSION = 0


def _rational(value: object) -> tuple[int, int]:
    """A budget as the exact rational num/den (integers stay over 1)."""
    f = Fraction(str(value))
    return f.numerator, f.denominator


def expected_patterns(packs: list[str]) -> list[tuple[object, ...]]:
    """The `pattern` rows the packs determine, in pack then file order (R7, R8)."""
    rows: list[tuple[object, ...]] = []
    for pack in packs:
        path = PACKS / f"patterns.{pack}.toml"
        raw = path.read_bytes()
        for p in tomllib.loads(raw.decode("utf-8"))["pattern"]:
            dedupe = p.get("dedupe", {})
            rows.append(
                (
                    pack,
                    p["id"],
                    p["version"],
                    SELECTIONS.index(p.get("selection", "latest")),
                    ON_COMPLETE.index(p.get("on_complete", "clear_binding")),
                    len(p.get("clause", [])),
                    sorted(p.get("roles", {}), key=str.encode),
                    list(dedupe.get("key", [])),
                    int(dedupe.get("bucket_days", 0)),
                    *_rational(p["firing_budget"]["per_century_per_100k"]),
                    bytes(CANON.xxh3_128(raw)),
                )
            )
    return rows


def check_patterns(
    con: duckdb.DuckDBPyConnection, run_dir: Path, packs: list[str], control: str | None
) -> list[str]:
    """Every segment's `pattern` table is exactly the packs' rows."""
    want = expected_patterns(packs)
    if control == "other-pattern":
        want = [(*r[:9], int(str(r[9])) + 1, *r[10:]) for r in want]
    by_file: dict[str, list[tuple[object, ...]]] = {}
    cols = (
        "pack, pattern_id, pattern_version, selection, on_complete, clauses, roles, dedupe_key, "
        "dedupe_days, budget_num, budget_den, pack_digest"
    )
    for *row, name in con.execute(
        f"SELECT {cols}, filename FROM "
        f"read_parquet('{run_dir}/stream/t*/C/y[0-9]*/pattern.parquet', filename=true)"
    ).fetchall():
        by_file.setdefault(str(name), []).append(
            tuple(bytes(v) if isinstance(v, (bytes, bytearray)) else v for v in row)
        )
    return [
        f"PATTERN DIFFERS: {Path(name).parent.name}: {rows} where the packs give {want}"
        for name, rows in sorted(by_file.items())
        if rows != want
    ][:3]


def _line(tl: int, seq: int, m: Match) -> str:
    """The canonical MATCH line of a twin match given its seq."""
    mid, cseq, events, binds, row = m
    assert isinstance(row, tuple) and isinstance(events, tuple) and isinstance(binds, tuple)
    pack, pattern, version, tick, opened, sel, onc, status, cev = row
    fields = (tl, seq, tick, mid, pack, pattern, version, sel, onc, status, opened, cev, cseq)
    clause_events = tuple(("e1", e) for e in events)
    return str(CANON.render(CANON.Record("MATCH", (*fields, binds, clause_events))))


def check_records(
    con: duckdb.DuckDBPyConnection, run_dir: Path, twin: list[Match], control: str | None
) -> list[str]:
    """Each MATCH record's timeline, seq and kind, and the child rows' timeline (GAP-10)."""
    c = f"{run_dir}/stream/t*/C/y[0-9]*"
    tl = int(con.execute("SELECT DISTINCT timeline_id FROM a_segment").fetchall()[0][0])
    got = con.execute(
        f"SELECT timeline_id, seq, kind FROM read_parquet('{c}/sift_match.parquet') ORDER BY seq"
    ).fetchall()
    start = 1 if control == "other-seq" else 0
    want = [(tl, i + start, MATCH_KIND) for i in range(len(twin))]
    problems = []
    if [tuple(r) for r in got] != want:
        problems.append(
            f"RECORDS DIFFER: (timeline_id, seq, kind) {got[:3]} where the twin gives {want[:3]}"
        )
    for table in ("sift_binding", "sift_match_event"):
        tls = {
            r[0]
            for r in con.execute(
                f"SELECT timeline_id FROM read_parquet('{c}/{table}.parquet')"
            ).fetchall()
        }
        if tls - {tl}:
            problems.append(f"RECORDS DIFFER: {table} rows of timeline {sorted(tls)}, not {tl}")
    return problems


def check_segment_rows(
    con: duckdb.DuckDBPyConnection,
    run_dir: Path,
    packs: list[str],
    twin: list[Match],
    control: str | None,
) -> list[str]:
    """Every column of every channel-C `segment` row (GAP-10)."""
    a = con.execute(
        "SELECT sim_year, timeline_id, first_seq, last_seq, engine_build, registry_version, "
        "manifest_hash, world_seed FROM a_segment ORDER BY sim_year"
    ).fetchall()
    got = con.execute(
        "SELECT timeline_id, channel, sim_year, format, first_seq, last_seq, record_count, "
        "first_tick, last_tick, sifter_build, dsl_version, packs, engine_build, registry_version, "
        "manifest_hash, world_seed, a_last_seq, canonical_digest FROM "
        f"read_parquet('{run_dir}/stream/t*/C/y[0-9]*/segment.parquet') ORDER BY sim_year"
    ).fetchall()
    next_seq, want = 0, []
    for year, tl, a_first, a_last, build, reg, mhash, seed in a:
        ms = [m for m in twin if a_first <= int(str(m[1])) <= a_last]
        lines = [_line(tl, next_seq + i, m) for i, m in enumerate(ms)]
        ticks = [m[4][3] for m in ms if isinstance(m[4], tuple)]
        n = len(ms)
        want.append(
            (
                tl,
                "C",
                year,
                FORMAT_C,
                next_seq,
                next_seq + n - 1 if n else None,
                n,
                ticks[0] if n else None,
                ticks[-1] if n else None,
                SIFTER_BUILD,
                DSL_VERSION,
                packs,
                build,
                reg,
                bytes(mhash),
                seed,
                a_last + (1 if control == "other-segment" else 0),
                bytes(CANON.digest(lines)),
            )
        )
        next_seq += n
    got_rows = [tuple(bytes(v) if isinstance(v, (bytes, bytearray)) else v for v in r) for r in got]
    return [
        f"SEGMENT DIFFERS: y{w[2]:06d}: {g} where the twin and channel A give {w}"
        for g, w in zip(got_rows, want, strict=False)
        if g != w
    ][:3]


def check_chain(con: duckdb.DuckDBPyConnection, run_dir: Path) -> list[str]:
    """R7: one channel-C segment per channel-A segment, with its digest and the chain."""
    a = con.execute(
        "SELECT sim_year, timeline_id, canonical_digest FROM a_segment ORDER BY sim_year"
    ).fetchall()
    c = con.execute(
        f"SELECT sim_year, a_canonical_digest, a_chain, packs FROM "
        f"read_parquet('{run_dir}/stream/t*/C/y[0-9]*/segment.parquet') ORDER BY sim_year"
    ).fetchall()
    if [r[0] for r in a] != [r[0] for r in c]:
        return [f"CHAIN: channel C's years {len(c)} are not channel A's {len(a)}"]
    problems = []
    chain = bytes(CANON.xxh3_128(D_CHAIN + int(a[0][1]).to_bytes(4, "little")))
    for (year, _, digest), (_, a_digest, a_chain, _) in zip(a, c, strict=True):
        chain = bytes(CANON.xxh3_128(D_CHAIN + chain + bytes(digest)))
        if bytes(a_digest) != bytes(digest) or bytes(a_chain) != chain:
            problems.append(f"CHAIN: y{year:06d} does not record channel A's digest and chain")
    return problems


def main(argv: list[str] | None = None) -> int:
    """Compare the twins with channel C."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("run_dir", type=Path)
    parser.add_argument(
        "--control",
        choices=("drop-last", "other-status", "other-seq", "other-pattern", "other-segment"),
    )
    args = parser.parse_args(argv)
    run_dir: Path = args.run_dir
    if not list(run_dir.glob("stream/t*/C/y[0-9]*/segment.parquet")):
        print(f"twin_check: no channel C under {run_dir}/stream (run rh-cli sift first)")
        return EXIT_UNREADABLE
    con = duckdb.connect()
    channel_a(con, run_dir)
    packs_row = con.execute(
        f"SELECT DISTINCT packs FROM read_parquet('{run_dir}/stream/t*/C/y[0-9]*/segment.parquet')"
    ).fetchall()
    if len(packs_row) != 1:
        print("twin_check: channel C's segments name different pack lists")
        return EXIT_DIFFER
    packs = list(packs_row[0][0])
    twin = twin_matches(con, packs)
    if args.control == "drop-last":
        twin = twin[:-1]
    elif args.control == "other-status":
        twin = [
            (*m[:4], (*row[:7], 0, *row[8:])) if isinstance(row := m[4], tuple) else m for m in twin
        ]
    sifted = sifted_matches(con, run_dir)
    problems = check_chain(con, run_dir)
    problems += check_records(con, run_dir, twin, args.control)
    problems += check_patterns(con, run_dir, packs, args.control)
    problems += check_segment_rows(con, run_dir, packs, twin, args.control)
    if twin != sifted:
        first = next(
            (i for i, (a, b) in enumerate(zip(twin, sifted, strict=False)) if a != b),
            min(len(twin), len(sifted)),
        )
        problems.append(
            f"TWIN DIFFERS: {len(twin)} twin matches, {len(sifted)} sifted; first difference at "
            f"match {first}: twin {twin[first] if first < len(twin) else None}, "
            f"sifted {sifted[first] if first < len(sifted) else None}"
        )
    for p in problems:
        print(p)
    if problems:
        return EXIT_DIFFER
    print(
        f"twin_check: V-SIFT-01: {len(twin)} matches of {','.join(packs)}: the SQL twin and "
        "rh-sifter agree on every match id, binding, event id, closing seq, tick, opened_tick, "
        "pack, pattern, version, selection, on_complete and status; channel C records "
        "every channel-A digest and the chain (R7); every record's timeline, seq and kind, "
        "every pattern row and every segment column are the twin's and channel A's (GAP-10)"
    )
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
