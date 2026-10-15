"""
Self-test for the chronicle schema and the SQL-twin method (ADR-0008, docs/09 §2.4).

It does four things:
  1. Loads schema.sql into an in-memory DuckDB.
  2. Generates a synthetic chronicle (events, roles, anchored trust snapshots, causal edges, title intervals).
  3. Runs pattern `trust_then_betrayal` twice: once as the SQL twin view, and once as a reference
     *incremental* matcher that consumes events in order (Winnow-style partial matches). It asserts
     that the two produce IDENTICAL match sets.
  4. Exercises the causal_ancestors and title_holder_at macros.

Run:  python -I specs/chronicle/selftest.py
"""
from __future__ import annotations

import random
import sys
import time
from collections import defaultdict
from pathlib import Path

import csv
import tempfile

import duckdb

HARM, AID, PROMISE, BETRAY, TRADE, CONCEAL = 1 << 0, 1 << 1, 1 << 2, 1 << 3, 1 << 4, 1 << 5
TRUST_ATTR, TRUST_MIN_Q88 = 1, 154               # 0.6 in Q8.8
WINDOW_TICKS = 3650 * 1440                        # 3650 days in minutes
MIN_PER_DAY = 1440


def synth(n_entities=300, n_events=200_000, seed=7):
    rnd = random.Random(seed)
    events, roles, snaps, edges = [], [], [], []
    tick = 0
    day_seq: dict[int, int] = defaultdict(int)
    acts = [AID, PROMISE, HARM, BETRAY, CONCEAL, TRADE]
    weights = [0.25, 0.10, 0.15, 0.05, 0.05, 0.40]
    last_ids: list[int] = []
    for _ in range(n_events):
        tick += rnd.randint(0, 2 * MIN_PER_DAY)
        day = tick // MIN_PER_DAY
        seq = day_seq[day]
        day_seq[day] += 1
        eid = (day << 24) | seq
        act = rnd.choices(acts, weights)[0]
        a = rnd.randrange(n_entities)
        b = rnd.randrange(n_entities - 1)
        b = b + 1 if b >= a else b
        events.append((0, eid, tick, 1, act, None, 0, 100, 0, 0, None, None))
        roles.append((0, eid, a, 0, 0))
        roles.append((0, eid, b, 1, 0))
        if act & (AID | PROMISE):
            snaps.append((0, eid, b, TRUST_ATTR, a, rnd.randint(0, 255)))   # trust(B, A)@e
        if last_ids and rnd.random() < 0.3:
            edges.append((0, rnd.choice(last_ids[-500:]), eid, rnd.choice([0, 3, 4]), 0, None))
        last_ids.append(eid)
    return events, roles, snaps, edges


def reference_incremental_matcher(events, roles, snaps):
    """Winnow-style: consume events in emission order and keep partial matches keyed by (A, B)."""
    by_event = defaultdict(dict)
    for (_, eid, ent, crole, _ar) in roles:
        by_event[eid]["a" if crole == 0 else "b"] = ent
    trust = {(eid, ent, partner): v for (_, eid, ent, _attr, partner, v) in snaps}
    partials: dict[tuple[int, int], list[tuple[int, int]]] = defaultdict(list)  # (A,B) -> [(e1_id, e1_tick)]
    matches = set()
    for (_, eid, tick, _verb, act, *_rest) in sorted(events, key=lambda r: r[1]):
        a, b = by_event[eid]["a"], by_event[eid]["b"]
        # 1) e2: complete open partials for (a, b) within the window (expire stale ones lazily)
        if act & (HARM | BETRAY | CONCEAL):
            live = [(e1, t1) for (e1, t1) in partials[(a, b)] if tick <= t1 + WINDOW_TICKS]
            partials[(a, b)] = live
            for (e1, _t1) in live:
                matches.add((a, b, e1, eid))
        # 2) unless: B aids A kills partials for key (A=b, B=a) opened before this event
        if act & AID:
            partials.pop((b, a), None)
        # 3) e1: open a new partial if anchored trust(B,A)@e1 >= threshold
        if act & (AID | PROMISE) and trust.get((eid, b, a), -1) >= TRUST_MIN_Q88:
            partials[(a, b)].append((eid, tick))
    return matches


def main() -> int:
    here = Path(__file__).parent
    con = duckdb.connect()
    con.execute(here.joinpath("schema.sql").read_text(encoding="utf-8"))

    events, roles, snaps, edges = synth()
    with tempfile.TemporaryDirectory() as tmp:          # bulk-load via CSV (row-wise executemany is slow)
        for table, rows in (("event", events), ("event_role", roles),
                            ("event_snapshot", snaps), ("causal_edge", edges)):
            path = Path(tmp) / f"{table}.csv"
            with path.open("w", newline="") as f:
                csv.writer(f).writerows([("" if v is None else v) for v in r] for r in rows)
            con.execute(f"COPY {table} FROM '{path.as_posix()}' (FORMAT csv, HEADER false, NULLSTR '')")
    con.executemany("INSERT INTO state_interval VALUES (?,?,?,?,?,?)",
                    [(0, 11, 7, 1, 0, 500_000), (0, 42, 7, 1, 500_000, 1_200_000), (0, 13, 7, 1, 1_200_000, None)])

    t0 = time.perf_counter()
    sql = {tuple(r[1:]) for r in con.execute("SELECT * FROM twin_trust_then_betrayal").fetchall()}
    t_sql = time.perf_counter() - t0
    t0 = time.perf_counter()
    inc = reference_incremental_matcher(events, roles, snaps)
    t_inc = time.perf_counter() - t0

    print(f"events={len(events):,}  snapshots={len(snaps):,}  edges={len(edges):,}")
    print(f"SQL twin matches        : {len(sql):,}  ({t_sql*1000:.0f} ms)")
    print(f"Incremental matcher     : {len(inc):,}  ({t_inc*1000:.0f} ms, {len(events)/t_inc/1e6:.2f} M events/s, pure Python)")
    if sql != inc:
        print(f"MISMATCH: only_sql={len(sql - inc)} only_incremental={len(inc - sql)}")
        return 1
    print("TWIN EQUIVALENCE: PASS (identical match sets)")

    root = next(e[2] for e in reversed(edges) if e[3] in (0, 3))      # latest contingent/motivated effect
    anc = con.execute("SELECT count(*), max(depth) FROM causal_ancestors(0, ?, 12)", [root]).fetchone()
    assert anc[0] > 1, anc
    print(f"causal_ancestors(root={root}): {anc[0]} events up to depth {anc[1]}: PASS")
    holder = con.execute("SELECT entity_id FROM title_holder_at(0, 7, 700000)").fetchall()
    assert holder == [(42,)], holder
    print("title_holder_at(tick=700000) -> 42: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
