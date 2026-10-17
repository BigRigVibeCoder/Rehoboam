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

Usage: python -I specs/sifting/twin_check.py RUN_DIR [--control drop-last|other-status]
Exit:  0 equal, 1 a difference, 2 the run directory could not be read.
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
import tomllib
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
    parser.add_argument("--control", choices=("drop-last", "other-status"))
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
        "every channel-A digest and the chain (R7)"
    )
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
