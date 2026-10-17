"""
Self-test for the chronicle schema and the SQL-twin method (ADR-0008, docs/05_architecture/05.4 §2.3-2.4).

It does five things:
  1. Loads schema.sql into an in-memory DuckDB.
  2. Generates two synthetic chronicles:
       * uniform   — random pairs among 300 actors (the easy case);
       * clustered — 1,000 named actors with ~20 partners each (symmetric ties), Zipf-weighted partner choice,
                     per-dyad trust that persists and drifts, and a realistic density of ~10 events per actor
                     per simulated year (05.2 §8: 20-40M named-actor events per 1,000 years at 100k persons).
  3. Runs pattern `trust_then_betrayal` under three declared semantics, each both as a SQL twin view and as
     a reference *incremental* matcher that consumes events in emission order:
       all     selection = "all",    on_complete = "keep"
       latest  selection = "latest", on_complete = "clear_binding"        (the DSL default)
       dedup   latest + dedupe = { key = ["A", "B"], bucket_days = 365 }
     It asserts IDENTICAL match sets for every (fixture, semantics) pair.
  4. Reports firing rates, to show why "all combinations" is not an acceptable default on clustered data.
  5. Exercises the causal_ancestors and title_holder_at macros.

Run:  python -I specs/chronicle/selftest.py
"""
from __future__ import annotations

import csv
import random
import sys
import tempfile
import time
from collections import defaultdict
from pathlib import Path

import duckdb

HARM, AID, PROMISE, BETRAY, TRADE, CONCEAL = 1 << 0, 1 << 1, 1 << 2, 1 << 3, 1 << 4, 1 << 5
E1_MASK, E2_MASK, UNLESS_MASK = AID | PROMISE, HARM | BETRAY | CONCEAL, AID
TRUST_ATTR, TRUST_MIN_Q88 = 1, 154               # 0.6 in q8_8
PERSON = 1                                        # rh-ids EntityKind
ACTOR, TARGET = 0, 1                              # Registry [event_roles] indexes (envelope_v0.md §1)
MIN_PER_DAY = 1440
WINDOW_TICKS = 3650 * MIN_PER_DAY                 # 3650 days in minutes
DEDUPE_BUCKET = 365 * MIN_PER_DAY
ACTS = [AID, PROMISE, HARM, BETRAY, CONCEAL, TRADE]
ACT_WEIGHTS = [0.25, 0.10, 0.15, 0.05, 0.05, 0.40]


def _emit(rnd, state, a, b, act, trust_value, out, max_step):
    """Append one event and an anchored trust(B, A) snapshot. The id is the timeline's event counter, as in
    rh-events: one counter per timeline, assigned in emission order, so id order is emission order. The id
    carries no date; the day is tick // MIN_PER_DAY."""
    events, roles, snaps, edges, last_ids = out
    state["tick"] += rnd.randint(0, max_step)
    eid = state["next_id"]
    state["next_id"] += 1
    events.append((0, eid, state["tick"], 1, act, None, 0, 100, 0, 0, None, None))
    roles.append((0, eid, 0, PERSON, a, ACTOR))        # event_role: ordinal, entity_kind, entity_id, role (SPR-020)
    roles.append((0, eid, 1, PERSON, b, TARGET))
    if act & E1_MASK:
        snaps.append((0, eid, TRUST_ATTR, PERSON, b, PERSON, a, trust_value))   # trust(B, A)@e: entity B, partner A
    if last_ids and rnd.random() < 0.3:
        kind = rnd.choice([0, 3, 4])
        motive = rnd.choice([0, 6]) if kind == 3 else None   # a motive only on motivated_by (envelope_v0.md §5)
        edges.append((0, rnd.choice(last_ids[-500:]), eid, kind, motive, 0, 256, True))
    last_ids.append(eid)


def synth_uniform(n_entities=300, n_events=200_000, seed=7):
    rnd = random.Random(seed)
    out = ([], [], [], [], [])
    state = {"tick": 0, "next_id": 0}
    for _ in range(n_events):
        act = rnd.choices(ACTS, ACT_WEIGHTS)[0]
        a = rnd.randrange(n_entities)
        b = rnd.randrange(n_entities - 1)
        b = b + 1 if b >= a else b
        _emit(rnd, state, a, b, act, rnd.randint(0, 255), out, 2 * MIN_PER_DAY)
    return out[:4]


def synth_clustered(n_entities=1_000, partners_each=10, n_events=200_000, events_per_actor_year=10, seed=11):
    """Each actor links to ~10 others and is linked by ~10 more: ~20 symmetric partners per actor, chosen with
    Zipf weights (a few strong ties). Trust is a per-dyad value that persists and drifts."""
    rnd = random.Random(seed)
    partners: list[set[int]] = [set() for _ in range(n_entities)]
    for a in range(n_entities):
        while len(partners[a]) < partners_each:
            b = rnd.randrange(n_entities)
            if b != a:
                partners[a].add(b)
                partners[b].add(a)
    plist = [sorted(p) for p in partners]
    for p in plist:
        rnd.shuffle(p)                                  # rank order = tie strength
    zipf = [[1.0 / (r + 1) for r in range(len(p))] for p in plist]
    mean_step = 360 * MIN_PER_DAY / (events_per_actor_year * n_entities)
    trust: dict[tuple[int, int], int] = {}
    out = ([], [], [], [], [])
    state = {"tick": 0, "next_id": 0}
    for _ in range(n_events):
        act = rnd.choices(ACTS, ACT_WEIGHTS)[0]
        a = rnd.randrange(n_entities)
        b = rnd.choices(plist[a], zipf[a])[0]
        key = (b, a)                                    # trust(B, A)
        v = trust.get(key)
        if v is None:
            v = max(0, min(255, int(rnd.gauss(150, 50))))
        v = max(0, min(255, v + rnd.randint(-12, 12)))  # drift
        if act & E2_MASK:
            v = max(0, v - 25)                          # harm erodes the victim's trust in the actor
        trust[key] = v
        _emit(rnd, state, a, b, act, v, out, int(2 * mean_step))
    return out[:4]


def incremental_matcher(events, roles, snaps, semantics: str):
    """Reference matcher: consume events in emission order; per binding (A, B) keep open partials [(e1, tick)].
    Order per event: completion, unless, opening (see schema.sql for the semantics contract)."""
    by_event = defaultdict(dict)
    for (_, eid, _ordinal, _kind, ent, role) in roles:
        by_event[eid]["a" if role == ACTOR else "b"] = ent
    trust = {(eid, ent, partner): v for (_, eid, _attr, _k, ent, _pk, partner, v) in snaps}
    partials: dict[tuple[int, int], list[tuple[int, int]]] = defaultdict(list)
    matches: list[tuple[int, int, int, int]] = []
    deduped: set[tuple[int, int, int]] = set()
    for (_, eid, tick, _verb, act, *_rest) in sorted(events, key=lambda r: r[1]):
        a, b = by_event[eid]["a"], by_event[eid]["b"]
        # 1) completion
        if act & E2_MASK and partials[(a, b)]:
            if semantics == "all":                      # selection=all, on_complete=keep
                live = [(e1, t1) for (e1, t1) in partials[(a, b)] if tick <= t1 + WINDOW_TICKS]
                partials[(a, b)] = live
                matches.extend((a, b, e1, eid) for (e1, _t1) in live)
            else:                                       # selection=latest, on_complete=clear_binding
                e1, t1 = partials[(a, b)][-1]
                if tick <= t1 + WINDOW_TICKS:
                    partials[(a, b)].clear()
                    bucket = (a, b, tick // DEDUPE_BUCKET)
                    if semantics != "dedup" or bucket not in deduped:   # dedupe filters output only
                        deduped.add(bucket)
                        matches.append((a, b, e1, eid))
        # 2) unless: B aids A cancels binding (A=b, B=a)
        if act & UNLESS_MASK:
            partials.pop((b, a), None)
        # 3) opening
        if act & E1_MASK and trust.get((eid, b, a), -1) >= TRUST_MIN_Q88:
            partials[(a, b)].append((eid, tick))
    return set(matches)


def load(con, events, roles, snaps, edges):
    for t in ("event", "event_role", "event_snapshot", "causal_edge", "state_interval"):
        con.execute(f"DELETE FROM {t}")
    with tempfile.TemporaryDirectory() as tmp:          # bulk-load via CSV (row-wise executemany is slow)
        for table, rows in (("event", events), ("event_role", roles),
                            ("event_snapshot", snaps), ("causal_edge", edges)):
            path = Path(tmp) / f"{table}.csv"
            with path.open("w", newline="") as f:
                csv.writer(f).writerows([("" if v is None else v) for v in r] for r in rows)
            con.execute(f"COPY {table} FROM '{path.as_posix()}' (FORMAT csv, HEADER false, NULLSTR '')")


VIEWS = {"all": "twin_trust_then_betrayal_all", "latest": "twin_trust_then_betrayal", "dedup": "twin_trust_then_betrayal_dedup"}


def check_fixture(con, name, data) -> bool:
    events, roles, snaps, edges = data
    load(con, events, roles, snaps, edges)
    n = len(events)
    print(f"\n[{name}] events={n:,}  snapshots={len(snaps):,}  edges={len(edges):,}")
    print(f"  {'semantics':<8} {'SQL':>9} {'incremental':>12} {'per 1k ev':>10} {'events completing':>18}  result")
    ok = True
    for sem, view in VIEWS.items():
        t0 = time.perf_counter()
        sql = {tuple(r[1:]) for r in con.execute(f"SELECT * FROM {view}").fetchall()}
        t_sql = time.perf_counter() - t0
        t0 = time.perf_counter()
        inc = incremental_matcher(events, roles, snaps, sem)
        t_inc = time.perf_counter() - t0
        completing = len({m[3] for m in inc}) / n
        same = sql == inc
        ok &= same
        print(f"  {sem:<8} {len(sql):>9,} {len(inc):>12,} {1000 * len(inc) / n:>10.1f} {completing:>17.1%}  "
              f"{'PASS' if same else f'MISMATCH (only_sql={len(sql - inc)}, only_inc={len(inc - sql)})'}"
              f"   [sql {t_sql*1000:.0f} ms, inc {n / t_inc / 1e6:.2f} M ev/s]")
    return ok


def main() -> int:
    here = Path(__file__).parent
    con = duckdb.connect()
    con.execute(here.joinpath("schema.sql").read_text(encoding="utf-8"))

    ok = check_fixture(con, "uniform", synth_uniform())
    clustered = synth_clustered()
    ok &= check_fixture(con, "clustered", clustered)
    if not ok:
        print("\nTWIN EQUIVALENCE: FAIL")
        return 1
    print("\nTWIN EQUIVALENCE: PASS (identical match sets for every fixture and semantics)")

    edges = clustered[3]
    con.executemany("INSERT INTO state_interval VALUES (?,?,?,?,?,?)",
                    [(0, 11, 7, 1, 0, 500_000), (0, 42, 7, 1, 500_000, 1_200_000), (0, 13, 7, 1, 1_200_000, None)])
    root = next(e[2] for e in reversed(edges) if e[3] in (0, 3))      # latest contingent/motivated effect
    bad = con.execute("SELECT count(*) FROM causal_edge WHERE kind <> 3 AND motive IS NOT NULL").fetchone()
    assert bad == (0,), bad
    anc = con.execute("SELECT count(*), max(depth) FROM causal_ancestors(0, ?, 12)", [root]).fetchone()
    assert anc[0] > 1, anc
    print(f"causal_ancestors(root={root}): {anc[0]} events up to depth {anc[1]}: PASS")
    holder = con.execute("SELECT entity_id FROM title_holder_at(0, 7, 700000)").fetchall()
    assert holder == [(42,)], holder
    print("title_holder_at(tick=700000) -> 42: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
