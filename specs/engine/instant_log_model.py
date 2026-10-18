"""Executable model of the instant log and the core genealogy index (EVO-002).

The contract is ``CODEX/60_EVOLUTION/EVO-002_InstantLog.md`` (the P1a shared rulings X1 and X2;
SPR-038 revision 2 for parts (a) and (b)): one engine-owned record per instant in ``rh-core``,
read-only to steps, filled only at the serial merges of the instant, cleared at the instant's end
and outside every hash, keyframe and journal. It holds

- (a) ``inserted()``: the PersonIds inserted at the merges so far this instant, ascending (a
  sorted insert at each merge's row insert);
- (b) ``exited()``: the persons whose ``person.alive`` went from 1 to 0 at a merge so far this
  instant, ascending across the instant (a sorted insert), found by a scan of ``person.alive``
  only at a merge whose job wrote it; a newborn who dies in its birth instant is in both, and
  ``Bookkeeping``'s compaction removes its row, not its entries;
- (c) the merged events of every kind a registered system observes (``[[observes]]``), with
  their EventIds and casts (this model keeps the actor and target roles): by (kind, target)
  (``instant_event``) and by kind in merge order (``events_of``); and the merge-time resolution of
  a cause given as "the same-instant event of kind K about target X" (the sugar), refused by name
  when it is absent, when only a job of the step's own level emits it, when it is the step's own
  draft, or when it matches several events; a kind the system does not observe is refused at the
  step.

An insert of a PersonId that (b) lists is refused (no re-entry within an instant, EVO-002 §2.3).

The core genealogy index (children by parent over the life-course log) is kept at each merge that
appended life-course rows.

This file models the engine around them, not any system's rules: the phases in ``rh_core::Phase``
order; in each phase a timer pass and a binding pass, each a list of levels derived from the
``same_instant`` edges (``05.1`` §2 rule 4; ``schedule.rs`` on SPR-014's branch), a level split
where one system's second binding appears; the jobs of a level run against one view and merge
serially in canonical order (``run_level`` and ``merge`` in SPR-014's ``engine.rs``). The systems
are scripted stand-ins named after the consumers (M2, M7, T1, S3) and stubs (X1 to X3, B1). The
transcript shows what each step sees in (a), (b), (c) and the index, how each cause resolves,
every merge and every instant's state hash. The checks use oracles computed from the merges' own
records (each job's buffers), never from the log:

1. CLEARED: the log is empty at every instant's start;
2. VISIBILITY: every step sees exactly what earlier merges of its instant recorded, never its own
   level's (as sets), and every resolution names an id that an earlier merge assigned;
3. ORDER: (a) and (b) ascending, (c) in id (merge) order;
4. SCAN: (b)'s scan of ``person.alive`` runs exactly at the merges whose job wrote it;
5. GENEALOGY: every ``children(p)`` a step reads equals a rebuild over the rows merged before it;
6. HASH: every instant's state hash equals the plane-A digest, which reads no log and no index;
7. READERS: removing the pure readers changes no hash;
8. RESUME: a run resumed from a keyframe cut at an instant's end equals the continuous run;
9. THREAD INVARIANCE: every permutation of the job order within each level gives the same
   transcript (log, views, resolutions, refusals, hashes);
10. EXPECT: each scenario completes, or is refused where and why its script says, and its
    transcript shows what its script says it must (the Year pass sees the pre-death holder, K12);
11. NAMED: where a scenario names its expected named set (M7's world-start pass, X4), the run ends
    with it: a member of the manifest's cast who meets no other criterion stays named.

The run also writes vectors, ``specs/engine/golden/instant_log_vectors_v0.json``: per scenario
its systems (bindings, same-instant edges, observed kinds), its world at creation, every job with
the view it saw, the actions it buffered (its scripted actions) and what its merge recorded, every
resolution and refusal, and each instant's log and state hash at its end; an engine test replays the
actions through stub systems and compares the views and merges (V-DET-22).

Usage:  python -I specs/engine/instant_log_model.py            (self-test only; the golden files
                                                              are neither compared nor written)
        python -I specs/engine/instant_log_model.py --check    (self-test; exit 1 if the transcript
                                                              or the vectors differ)
        python -I specs/engine/instant_log_model.py --check --golden PATH --vectors PATH
                                                             (check other files)
        python -I specs/engine/instant_log_model.py --write    (self-test, then rewrite both files,
                                                              the only way either is written)
        python -I specs/engine/instant_log_model.py --emit-vectors  (self-test, then rewrite the
                                                              vectors only, as --write would)
        python -I specs/engine/instant_log_model.py --control <name>  (a planted fault that must
                                                              FAIL with its own message; the golden
                                                              is never written)
Controls, each failing its own check: peer-visible (jobs merge as they finish, so a later job sees
its peers: THREAD INVARIANCE), unsorted-inserted (an append for (a)'s sorted insert: ORDER),
unsorted-exited (an append for (b)'s sorted insert: ORDER, R9), log-not-cleared (CLEARED),
hash-includes-log (HASH), genealogy-late (the index kept only at the
instant's end: GENEALOGY), late-watermark (the merge-time form resolved against the log as it stands
at the job's own merge, so an earlier job of its level in canonical order is visible to it:
VISIBILITY, and the same-level refusal is lost: EXPECT), cast-demoted (M7's naming criteria without
the manifest's cast, so tick 0's pass demotes a cast member: NAMED).
Exit:   0 ok, 1 stale golden file, 2 the self-test failed or a control went undetected.
Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import bisect
import hashlib
import itertools
import json
import sys
from collections.abc import Callable
from dataclasses import dataclass, field, replace
from pathlib import Path

HERE = Path(__file__).resolve().parent
GOLDEN = HERE / "golden" / "instant_log_v0.txt"
VECTORS = HERE / "golden" / "instant_log_vectors_v0.json"
FORMAT = "rehoboam/engine/instant-log-model/v0"
VECTOR_FORMAT = "rehoboam/engine/instant-log-vectors/v0"
EXIT_OK = 0
EXIT_STALE = 1
EXIT_SELFTEST = 2

PHASES = (
    "Inputs",
    "Environment",
    "Production",
    "Exchange",
    "Vital",
    "Activity",
    "Contagion",
    "Polity",
    "Military",
    "Agenda",
    "Culture",
    "Bookkeeping",
)
VITAL = "Vital"
POLITY = "Polity"
AGENDA = "Agenda"
BOOKKEEPING = "Bookkeeping"
DAY = "Day"
YEAR = "Year"
PULSE_RANK = {"Year": 0, "Season": 1, "Month": 2, "Day": 3, "Hour": 4}
DAYS_PER_MONTH = 30
DAYS_PER_YEAR = 360
MINUTES_PER_DAY = 1440
# Entity kinds in rh_ids tag order (a title is F-4's proposed tag 6; a house is a model label).
KIND_RANK = {"P": 1, "T": 6, "H": 7}
NONE = -1
HELD, INTERREGNUM = 0, 2
SOVEREIGN, MAJOR, MINOR = 0, 1, 2
FAR = 1 << 62

TIMERS = "timers"
BINDINGS = "bindings"
PEER_VISIBLE = "peer-visible"
UNSORTED_INSERTED = "unsorted-inserted"
UNSORTED_EXITED = "unsorted-exited"
LOG_NOT_CLEARED = "log-not-cleared"
HASH_INCLUDES_LOG = "hash-includes-log"
GENEALOGY_LATE = "genealogy-late"
LATE_WATERMARK = "late-watermark"
CAST_DEMOTED = "cast-demoted"
CONTROLS = (
    PEER_VISIBLE,
    UNSORTED_INSERTED,
    UNSORTED_EXITED,
    LOG_NOT_CLEARED,
    HASH_INCLUDES_LOG,
    GENEALOGY_LATE,
    LATE_WATERMARK,
    CAST_DEMOTED,
)

M2 = "M2.demography"
M7 = "M7.dynasty"
T1 = "T1.minds"
S3 = "S3.migration"
B1 = "B1.bookkeeper"
X1 = "X1.emitter"
X2 = "X2.observer"
X3 = "X3.prober"
DEATH = "DEATH"
BIRTH = "BIRTH"
MARRIAGE = "MARRIAGE"
X_MARK = "X_MARK"

Ref = tuple[str, int]
# (c): kind, target, actor, EventId, merge index. The model's cast is the target and the actor roles
# (envelope_v0.md §1); the engine keeps the event's role rows.
Entry = tuple[str, Ref | None, Ref | None, int, int]
Cast = tuple[int, Ref | None, Ref | None]  # events_of: EventId, actor, target


def ref_text(ref: Ref | None) -> str:
    """An entity as the transcript prints it: ``P5``, ``T707``; ``-`` for none."""
    return "-" if ref is None else f"{ref[0]}{ref[1]}"


def person(pid: int) -> Ref:
    """The person ``pid`` as an entity reference."""
    return ("P", pid)


# ----------------------------------------------------------------------------- slots and buffers


@dataclass(frozen=True)
class Cause:
    """An EventRef slot as a step fills it (envelope_v0.md §2; EVO-002 §4).

    ``draft`` names an earlier draft of the same step (the merge resolves it), ``known`` an id
    assigned at an earlier merge, and ``kind`` with ``target`` the same-instant sugar.
    """

    draft: int = NONE
    known: int = NONE
    kind: str = ""
    target: Ref | None = None

    def text(self) -> str:
        """The slot as the transcript prints it."""
        if self.kind:
            return f"same_instant({self.kind}, {ref_text(self.target)})"
        if self.draft != NONE:
            return f"draft({self.draft})"
        return f"#{self.known}"


def own_draft(index: int) -> Cause:
    """A cause naming the step's own draft ``index`` (a ``DraftRef``)."""
    return Cause(draft=index)


def known(eid: int) -> Cause:
    """A cause naming an id assigned at an earlier merge (``EventRef::Known``)."""
    return Cause(known=eid)


def same_instant(kind: str, target: Ref) -> Cause:
    """The sugar: the same-instant event of ``kind`` about ``target``, resolved at the merge."""
    return Cause(kind=kind, target=target)


@dataclass(frozen=True)
class Draft:
    """An event draft: its kind, its target and actor roles, its causes and a payload."""

    kind: str
    target: Ref | None
    actor: Ref | None
    causes: tuple[Cause, ...] = ()
    payload: int = NONE


@dataclass(frozen=True)
class TimerDraft:
    """A timer a step schedules: when, about whom, its kind and argument, and its cause slot."""

    day: int
    phase: str
    entity: Ref
    kind: str
    payload: int = 0
    cause: Cause | None = None


@dataclass(frozen=True)
class Timer:
    """A pending timer in the agenda (plane A), its cause resolved to an id or ``NONE``."""

    day: int
    phase: str
    entity: Ref
    seq: int
    owner: str
    kind: str
    payload: int
    cause: int

    def key(self) -> tuple[int, int, int, int, int]:
        """The agenda key: (time, phase, entity kind, entity id, seq)."""
        return (
            self.day,
            PHASES.index(self.phase),
            KIND_RANK[self.entity[0]],
            self.entity[1],
            self.seq,
        )


@dataclass(frozen=True)
class Command:
    """A queued lifecycle command (plane A): ``ScheduleDeath`` in this model."""

    owner: str
    kind: str
    victim: int
    culprit: int
    seq: int = NONE


@dataclass(frozen=True)
class Event:
    """A merged event (plane A, ADR-0024): its id, instant, phase, emitter, roles and causes."""

    eid: int
    day: int
    phase: str
    emitter: str
    draft: Draft
    causes: tuple[int, ...]


@dataclass(frozen=True)
class Title:
    """M7's model state for a title, with the scripted outcomes its stand-in step reads."""

    tier: int
    liege: int
    holder: int
    heir: int = 0
    resolve_to: int = 0
    state: int = HELD
    predecessor: int = 0


@dataclass
class Outbox:
    """What one job buffered, merged by the engine at the end of its level (05.1 §2 rule 4)."""

    events: list[Draft] = field(default_factory=list)
    timers: list[TimerDraft] = field(default_factory=list)
    commands: list[Command] = field(default_factory=list)
    taken: list[Command] = field(default_factory=list)
    kills: list[int] = field(default_factory=list)
    inserts: list[tuple[int, int, int]] = field(default_factory=list)
    titles: dict[int, Title] = field(default_factory=dict)
    houses: list[int] = field(default_factory=list)
    named: dict[int, int] = field(default_factory=dict)


# ----------------------------------------------------------------------------- plane-A state


@dataclass
class World:
    """The model's plane-A state: what a keyframe holds and the state hash covers.

    No instant log and no genealogy index: both are derived and outside every hash (EVO-002).
    """

    ledger: dict[int, int]
    life_course: list[tuple[int, int, int]]
    titles: dict[int, Title] = field(default_factory=dict)
    houses: dict[int, tuple[tuple[int, ...], bool]] = field(default_factory=dict)
    events: list[Event] = field(default_factory=list)
    agenda: list[Timer] = field(default_factory=list)
    queue: list[Command] = field(default_factory=list)
    next_event: int = 0
    next_seq: int = 0
    named: dict[int, int] = field(default_factory=dict)  # T1's person.named, 1 or 0

    def copy(self) -> World:
        """A keyframe: an independent copy (every element is immutable)."""
        return World(
            dict(self.ledger),
            list(self.life_course),
            dict(self.titles),
            dict(self.houses),
            list(self.events),
            list(self.agenda),
            list(self.queue),
            self.next_event,
            self.next_seq,
            dict(self.named),
        )

    def seed(self, owner: str, draft: TimerDraft) -> World:
        """Add a timer pending at world creation (no cause); return the world."""
        self.agenda.append(
            Timer(
                draft.day,
                draft.phase,
                draft.entity,
                self.next_seq,
                owner,
                draft.kind,
                draft.payload,
                NONE,
            )
        )
        self.next_seq += 1
        return self


def event_text(e: Event) -> str:
    """One event as plane A's canonical text writes it."""
    d = e.draft
    causes = ",".join(map(str, e.causes))
    return (
        f"{e.eid}/{e.day}/{e.phase}/{e.emitter}/{d.kind}/{ref_text(d.target)}"
        f"/{ref_text(d.actor)}/{d.payload}/{causes}"
    )


def plane_a_text(world: World) -> str:
    """Plane A's canonical text.

    Ledger, life-course log, titles, houses, events, agenda, queue, counters, and T1's named column
    when a scenario has one (a line only then, so the other scenarios' hashes do not move). It reads
    no instant log and no genealogy index.
    """
    titles = " ".join(
        f"T{t}:{x.tier},{x.liege},{x.holder},{x.state},{x.predecessor}"
        for t, x in sorted(world.titles.items())
    )
    agenda = " ".join(
        f"{x.day}/{x.phase}/{ref_text(x.entity)}/{x.seq}/{x.owner}/{x.kind}/{x.payload}/{x.cause}"
        for x in sorted(world.agenda, key=Timer.key)
    )
    named = (
        ["named " + " ".join(f"{p}:{v}" for p, v in sorted(world.named.items()))]
        if world.named
        else []
    )
    return "\n".join(
        [
            *named,
            "ledger " + " ".join(f"{p}:{a}" for p, a in sorted(world.ledger.items())),
            "life_course " + " ".join(f"{p}/{m}/{f}" for p, m, f in world.life_course),
            "titles " + titles,
            "houses " + " ".join(f"H{h}:{int(x)}" for h, (_, x) in sorted(world.houses.items())),
            "events " + " ".join(event_text(e) for e in world.events),
            "agenda " + agenda,
            "queue " + " ".join(f"{c.kind}/{c.victim}/{c.culprit}/{c.seq}" for c in world.queue),
            f"next {world.next_event} {world.next_seq}",
        ]
    )


def digest(text: str) -> str:
    """The first 16 hex digits of the text's SHA-256."""
    return hashlib.sha256(text.encode()).hexdigest()[:16]


# ----------------------------------------------------------------------------- the record


@dataclass(frozen=True)
class LogView:
    """What one job sees of the log: the entries recorded by merges before its level's.

    Every entry keeps the merge index that recorded it, so a check can tell a peer's entry (the
    viewer's own merge index) from a stale one (no merge of this instant recorded it).
    """

    inserted: tuple[tuple[int, int], ...]
    exited: tuple[tuple[int, int], ...]
    events: tuple[Entry, ...]

    def persons(self, part: str) -> tuple[int, ...]:
        """The PersonIds of part (a) or (b), in the log's order."""
        return tuple(p for p, _ in (self.inserted if part == "a" else self.exited))

    def ids(self, kind: str, target: Ref | None) -> tuple[int, ...]:
        """(c) by (kind, target): the ids, in id order."""
        return tuple(e for k, t, _, e, _ in self.events if k == kind and t == target)

    def of_kind(self, kind: str) -> tuple[Cast, ...]:
        """(c) by kind (``events_of``): each event's id and cast, in merge (id) order."""
        return tuple((e, a, t) for k, t, a, e, _ in self.events if k == kind)


@dataclass
class InstantLog:
    """The instant log (EVO-002 §2): (a), (b) and (c), each entry with its merge index."""

    inserted: list[tuple[int, int]] = field(default_factory=list)
    exited: list[tuple[int, int]] = field(default_factory=list)
    events: list[Entry] = field(default_factory=list)
    scans: list[int] = field(default_factory=list)  # the merges at which (b)'s scan ran

    def size(self) -> int:
        """How many entries the log holds."""
        return len(self.inserted) + len(self.exited) + len(self.events)

    def text(self) -> str:
        """The log's content, for a transcript line (and, in one control, for the hash)."""
        a = [p for p, _ in self.inserted]
        b = [p for p, _ in self.exited]
        c = [f"{k}:{ref_text(t)}#{e}" for k, t, _, e, _ in self.events]
        return f"(a)={a} (b)={b} (c)=[{' '.join(c)}]"

    def clear(self) -> None:
        """Empty every part (at the instant's end)."""
        self.inserted.clear()
        self.exited.clear()
        self.events.clear()

    def view(self, watermark: int) -> LogView:
        """The entries of merges with an index below ``watermark``."""
        return LogView(
            tuple(x for x in self.inserted if x[1] < watermark),
            tuple(x for x in self.exited if x[1] < watermark),
            tuple(x for x in self.events if x[4] < watermark),
        )

    def add_inserted(self, pid: int, merge: int, *, sort: bool) -> None:
        """(a): a sorted insert, so the list stays ascending (ledger order)."""
        if sort:
            bisect.insort(self.inserted, (pid, merge))
        else:
            self.inserted.append((pid, merge))

    def scan_exits(self, ledger: dict[int, int], merge: int, *, sort: bool) -> list[int]:
        """(b): the ledger rows at ``person.alive = 0`` not yet listed, each by a sorted insert.

        Rows enter alive and no row is at 0 when an instant starts (the previous Bookkeeping
        compacted them), so every row found went from 1 to 0 at this merge.
        """
        self.scans.append(merge)
        listed = {p for p, _ in self.exited}
        new = [p for p, alive in sorted(ledger.items()) if alive == 0 and p not in listed]
        for p in new:
            if sort:
                bisect.insort(self.exited, (p, merge))
            else:
                self.exited.append((p, merge))
        return new


class Genealogy:
    """The core genealogy index (EVO-002 §6): children by parent, ascending, alive or dead."""

    def __init__(self) -> None:
        """An empty index; ``extend`` builds it from the life-course rows."""
        self.kids: dict[int, list[int]] = {}
        self.rows = 0

    def extend(self, rows: list[tuple[int, int, int]]) -> None:
        """Index the rows appended since the last call: O(appended) sorted inserts."""
        for child, mother, father in rows[self.rows :]:
            for parent in sorted({mother, father} - {0}):
                bisect.insort(self.kids.setdefault(parent, []), child)
        self.rows = len(rows)

    def children(self, parent: int) -> tuple[int, ...]:
        """Every person whose mother or father is ``parent``, ascending."""
        return tuple(self.kids.get(parent, []))


def rebuild_children(rows: list[tuple[int, int, int]], parent: int) -> tuple[int, ...]:
    """The oracle: ``children(parent)`` recomputed from the rows, with no index."""
    return tuple(sorted(c for c, m, f in rows if parent in (m, f)))


# ----------------------------------------------------------------------------- registration


@dataclass(frozen=True)
class System:
    """A registered system.

    Its bindings, the systems it reads at ``same_instant`` (each precedes it in a phase both bind:
    rule 4's edge) and the kinds it observes.
    """

    sid: str
    bindings: tuple[tuple[str, str], ...]
    after: tuple[str, ...] = ()
    observes: tuple[str, ...] = ()


@dataclass(frozen=True)
class Script:
    """The scripted facts the stand-in steps read: not engine state, never hashed."""

    fathers: tuple[tuple[int, int], ...] = ()
    crossings: tuple[tuple[int, int], ...] = ()
    feuds: tuple[tuple[int, int], ...] = ()
    x_marks: int = 0
    probe: Cause | None = None
    reinsert: int = 0
    reinsert_at: str = ""
    newborn_kill: int = 0
    delay: int = 2
    cast: tuple[int, ...] = ()
    criteria: tuple[tuple[int, str], ...] = ()
    late_births: tuple[tuple[int, int, int], ...] = ()  # (child, mother, father) at M2's drain
    kin_probe: tuple[int, ...] = ()  # the persons whose children M7's Day/Polity reads each instant


@dataclass(frozen=True)
class Scenario:
    """A scripted run.

    Its systems, its world at creation, the instants it runs, and what the script expects
    (``complete`` or ``refused:<reason>``).
    """

    name: str
    about: str
    systems: tuple[System, ...]
    world: Callable[[], World]
    days: tuple[int, ...]
    script: Script = Script()
    expect: str = "complete"
    keyframe_after: int = NONE
    readers: tuple[str, ...] = ()
    expect_named: tuple[int, ...] | None = None
    expect_seen: tuple[str, ...] = ()


@dataclass(frozen=True)
class Job:
    """One job of a level: a system's timer pass (``pulse`` empty) or one of its bindings."""

    sid: str
    pulse: str
    phase: str
    timers: tuple[Timer, ...] = ()

    def label(self) -> str:
        """The job as the transcript prints it."""
        return f"{self.sid}:{self.pulse or 'timers'}"


@dataclass(frozen=True)
class LevelKey:
    """A level of one instant: (day, phase, pass, index in the pass)."""

    day: int
    phase: str
    pass_name: str
    level: int


def pulses(day: int) -> tuple[str, ...]:
    """The pulses that fire at the day start ``day`` (360-day calendar), coarser first."""
    out = []
    if day % DAYS_PER_YEAR == 0:
        out.append(YEAR)
    if day % DAYS_PER_MONTH == 0:
        out.append("Month")
    out.append(DAY)
    return tuple(out)


def binding_levels(systems: tuple[System, ...], phase: str) -> dict[tuple[str, str], int]:
    """Each binding's level in ``phase``: the longest same_instant edge path that ends at it."""
    nodes = [(s.sid, p) for s in systems for p, ph in s.bindings if ph == phase]
    after = {s.sid: s.after for s in systems}
    memo: dict[tuple[str, str], int] = {}

    def level(node: tuple[str, str]) -> int:
        if node not in memo:
            preds = [w for w in nodes if w[0] != node[0] and w[0] in after[node[0]]]
            memo[node] = 1 + max((level(w) for w in preds), default=-1)
        return memo[node]

    return {n: level(n) for n in nodes}


def split_levels(groups: list[list[Job]]) -> list[list[Job]]:
    """Split a level wherever one system's second binding appears (SPR-014 ``binding_pass``)."""
    out: list[list[Job]] = []
    for group in groups:
        cur: list[Job] = []
        for job in group:
            if any(j.sid == job.sid for j in cur):
                out.append(cur)
                cur = []
            cur.append(job)
        if cur:
            out.append(cur)
    return out


# ----------------------------------------------------------------------------- records


@dataclass(frozen=True)
class ViewRecord:
    """What one job saw: where it ran, its merge index, its view and its index reads."""

    day: int
    merge: int
    where: str
    job: str
    observes: tuple[str, ...]
    view: LogView
    children: tuple[tuple[int, tuple[int, ...]], ...]


@dataclass(frozen=True)
class JobMerge:
    """One job's merge, from its own buffers: the oracles' input."""

    day: int
    merge: int
    sid: str
    inserted: tuple[int, ...]
    killed: tuple[int, ...]
    events: tuple[tuple[str, Ref | None, int], ...]
    rows: tuple[tuple[int, int, int], ...]


@dataclass(frozen=True)
class Resolution:
    """A same-instant cause resolved at a merge."""

    day: int
    merge: int
    sid: str
    cause: Cause
    eid: int


@dataclass
class Run:
    """The transcript and the records of one run."""

    lines: list[str] = field(default_factory=list)
    day_lines: dict[int, list[str]] = field(default_factory=dict)
    views: list[ViewRecord] = field(default_factory=list)
    merges: list[JobMerge] = field(default_factory=list)
    resolutions: list[Resolution] = field(default_factory=list)
    event_merge: dict[tuple[int, int], int] = field(default_factory=dict)
    refusal: tuple[str, str] | None = None
    hashes: list[tuple[int, str, str]] = field(default_factory=list)
    log_at_start: list[tuple[int, int]] = field(default_factory=list)
    rows_at_start: dict[int, list[tuple[int, int, int]]] = field(default_factory=dict)
    levels: list[tuple[LevelKey, int]] = field(default_factory=list)
    keyframe: World | None = None
    named_at_end: tuple[int, ...] = ()
    scans: dict[int, list[int]] = field(default_factory=dict)
    jobs: list[dict[str, object]] = field(default_factory=list)
    instants: list[dict[str, object]] = field(default_factory=list)

    def outcome(self) -> str:
        """``complete``, or ``refused:<reason>``."""
        return "complete" if self.refusal is None else f"refused:{self.refusal[0]}"


class RefusedError(Exception):
    """A refusal by name: a ``SimError`` that stops the run (the model's fatal error)."""

    def __init__(self, reason: str, message: str) -> None:
        """Keep the reason code and the message."""
        super().__init__(message)
        self.reason = reason
        self.message = message


# ----------------------------------------------------------------------------- the step context


class Ctx:
    """The StepCtx of one job.

    Its view of the log (read-only), plane A as its level started, and its buffers.
    """

    def __init__(self, engine: Engine, job: Job, merge: int, view: LogView) -> None:
        """A context for ``job``, whose level merges at index ``merge``."""
        self.engine = engine
        self.job = job
        self.merge = merge
        self.view = view
        self.out = Outbox()
        self.children_read: list[tuple[int, tuple[int, ...]]] = []
        self.notes: list[str] = []
        self.system = next(s for s in engine.scenario.systems if s.sid == job.sid)

    @property
    def day(self) -> int:
        """The current instant (a day start)."""
        return self.engine.day

    @property
    def script(self) -> Script:
        """The scenario's script."""
        return self.engine.scenario.script

    # reads ------------------------------------------------------------------------------------
    def inserted(self) -> tuple[int, ...]:
        """(a) as this job sees it."""
        return self.view.persons("a")

    def exited(self) -> tuple[int, ...]:
        """(b) as this job sees it."""
        return self.view.persons("b")

    def instant_event(self, kind: str, target: Ref) -> tuple[int, ...]:
        """(c) by (kind, target): the ids merged before this job's level."""
        self.require_observed(kind)
        return self.view.ids(kind, target)

    def events_of(self, kind: str) -> tuple[Cast, ...]:
        """(c) by kind: the instant's merged events of ``kind`` so far, in merge order (R10)."""
        self.require_observed(kind)
        return self.view.of_kind(kind)

    def require_observed(self, kind: str) -> None:
        """Refuse a kind the system does not declare in ``[[observes]]``."""
        if kind not in self.system.observes:
            raise RefusedError(
                "undeclared",
                f"{self.job.label()} names {kind}, which is not in its [[observes]] "
                f"{list(self.system.observes)} (SimError::UndeclaredObserve)",
            )

    def children(self, parent: int) -> tuple[int, ...]:
        """The core genealogy index as this job's level started."""
        kids = self.engine.genealogy.children(parent)
        self.children_read.append((parent, kids))
        return kids

    def alive(self, pid: int) -> bool:
        """``person.alive`` as this job's level started, with this job's own writes."""
        return self.engine.world.ledger.get(pid, 0) == 1 and pid not in self.out.kills

    def father(self, pid: int) -> int:
        """``life_course.father`` of ``pid`` (0 when unknown)."""
        return next((f for c, _, f in self.engine.world.life_course if c == pid), 0)

    def title(self, tid: int) -> Title:
        """M7's own title row, with this job's own writes."""
        return self.out.titles.get(tid, self.engine.world.titles[tid])

    def title_ids(self) -> list[int]:
        """Every title id, ascending."""
        return sorted(self.engine.world.titles)

    def houses(self) -> list[tuple[int, tuple[int, ...], bool]]:
        """M7's houses: (id, members, extinct), ascending."""
        return [(h, m, x) for h, (m, x) in sorted(self.engine.world.houses.items())]

    def persons(self) -> list[int]:
        """The living ledger rows as this job's level started, ascending."""
        return sorted(p for p, alive in self.engine.world.ledger.items() if alive == 1)

    def is_named(self, pid: int) -> bool:
        """T1's ``person.named`` as M7 reads it (lagged: as the instant started)."""
        return self.engine.world.named.get(pid, 0) == 1

    def set_named(self, pid: int, value: int) -> None:
        """T1 writes its own ``person.named`` (Promote 1, Demote 0)."""
        self.out.named[pid] = value

    def take_commands(self) -> list[Command]:
        """The queued commands this binding drains (M2's Day/Vital: ``ScheduleDeath``)."""
        taken = [c for c in self.engine.world.queue if c.owner == self.job.sid]
        self.out.taken.extend(taken)
        return taken

    def note(self, text: str) -> None:
        """A line of the transcript under this job's view."""
        self.notes.append(text)

    # writes -----------------------------------------------------------------------------------
    def emit(self, draft: Draft) -> int:
        """Buffer a draft; return its ``DraftRef`` (its index in this step)."""
        for cause in draft.causes:
            self.check_cause(cause)
        self.out.events.append(draft)
        return len(self.out.events) - 1

    def schedule(self, draft: TimerDraft) -> None:
        """Buffer a timer; its cause slot is resolved at the merge."""
        if draft.cause is not None:
            self.check_cause(draft.cause)
        self.out.timers.append(draft)

    def check_cause(self, cause: Cause) -> None:
        """A draft ref names an earlier draft of this step; the sugar names an observed kind."""
        if cause.kind:
            self.require_observed(cause.kind)
        elif cause.draft != NONE and cause.draft >= len(self.out.events):
            raise RefusedError("draft", f"{self.job.label()} names a draft it has not emitted")

    def submit(self, command: Command) -> None:
        """Buffer a lifecycle command."""
        self.out.commands.append(command)

    def kill(self, pid: int) -> None:
        """Write ``person.alive = 0`` (the ledger's owner only)."""
        self.out.kills.append(pid)

    def insert(self, pid: int, mother: int, father: int) -> None:
        """Hand rh-core a ledger row and its life-course row, inserted at the merge."""
        self.out.inserts.append((pid, mother, father))

    def write_title(self, tid: int, title: Title) -> None:
        """Write M7's own title row."""
        self.out.titles[tid] = title

    def write_house(self, hid: int) -> None:
        """Mark M7's house extinct."""
        self.out.houses.append(hid)

    def record(self, where: str) -> ViewRecord:
        """What this job saw."""
        return ViewRecord(
            self.engine.day,
            self.merge,
            where,
            self.job.label(),
            self.system.observes,
            self.view,
            tuple(self.children_read),
        )


# ----------------------------------------------------------------------------- the engine


class Engine:
    """The engine around the log: phases, passes, levels, jobs and serial merges."""

    def __init__(
        self,
        scenario: Scenario,
        world: World,
        flags: frozenset[str],
        orders: dict[LevelKey, tuple[int, ...]],
    ) -> None:
        """An engine over ``world``; ``orders`` permutes the execution order of chosen levels."""
        self.scenario = scenario
        self.world = world
        self.flags = flags
        self.orders = orders
        self.log = InstantLog()
        self.genealogy = Genealogy()
        self.genealogy.extend(world.life_course)
        self.observed = frozenset(k for s in scenario.systems for k in s.observes)
        self.run = Run()
        self.day = 0
        self.merge = 0
        self.levels = {ph: binding_levels(scenario.systems, ph) for ph in PHASES}

    def line(self, text: str) -> None:
        """Append a transcript line for the current instant."""
        self.run.lines.append(text)
        self.run.day_lines.setdefault(self.day, []).append(text)

    # the instant ------------------------------------------------------------------------------
    def run_day(self, day: int) -> None:
        """One instant: every phase's timer pass and binding pass, then the instant's end."""
        self.day = day
        self.merge = 0
        self.run.log_at_start.append((day, self.log.size()))
        self.run.rows_at_start[day] = list(self.world.life_course)
        self.log.scans.clear()
        fired = pulses(day)
        self.line(f"instant day {day} t={day * MINUTES_PER_DAY} pulses={','.join(fired)}")
        for phase in PHASES:
            if phase == BOOKKEEPING:
                self.compact()
            for i, jobs in enumerate(self.timer_levels(phase)):
                self.run_level(LevelKey(day, phase, TIMERS, i), jobs)
            for i, jobs in enumerate(self.binding_jobs(phase, fired)):
                self.run_level(LevelKey(day, phase, BINDINGS, i), jobs)
        self.end_instant()

    def timer_levels(self, phase: str) -> list[list[Job]]:
        """The timer pass.

        The timers due at (day, phase) leave the agenda together; each owner fires its own, in key
        order, at the level of its first binding in the phase.
        """
        slot = (self.day, phase)
        due = sorted((t for t in self.world.agenda if (t.day, t.phase) == slot), key=Timer.key)
        if not due:
            return []
        self.world.agenda = [t for t in self.world.agenda if (t.day, t.phase) != slot]
        by_level: dict[int, list[Job]] = {}
        for sid in sorted({t.owner for t in due}):
            mine = [lv for (s, _), lv in self.levels[phase].items() if s == sid]
            job = Job(sid, "", phase, tuple(t for t in due if t.owner == sid))
            by_level.setdefault(min(mine), []).append(job)
        return [by_level[k] for k in sorted(by_level)]

    def binding_jobs(self, phase: str, fired: tuple[str, ...]) -> list[list[Job]]:
        """The binding pass: the firing bindings by level, coarser pulse first, then system id."""
        by_level: dict[int, list[Job]] = {}
        for (sid, pulse), lv in self.levels[phase].items():
            if pulse in fired:
                by_level.setdefault(lv, []).append(Job(sid, pulse, phase))
        groups = [
            sorted(by_level[k], key=lambda j: (PULSE_RANK[j.pulse], j.sid))
            for k in sorted(by_level)
        ]
        return split_levels(groups)

    def run_level(self, key: LevelKey, jobs: list[Job]) -> None:
        """Run a level's jobs against one view, then merge their buffers in canonical order.

        Under the peer-visible control each job merges as soon as it finishes, in execution
        order, and a later job's view is the log as it then stands: the race the rule forbids.
        """
        m = self.merge
        self.merge += 1
        where = f"{key.phase}/{key.pass_name} L{key.level}"
        if len(jobs) > 1:
            self.run.levels.append((key, len(jobs)))
        early = PEER_VISIBLE in self.flags
        ctxs: dict[int, Ctx] = {}
        errors: dict[int, RefusedError] = {}
        for i in self.orders.get(key, tuple(range(len(jobs)))):
            ctx = Ctx(self, jobs[i], m, self.log.view(FAR if early else m))
            ctxs[i] = ctx
            try:
                self.execute(ctx)
            except RefusedError as e:
                errors[i] = e
            if early and i not in errors:
                self.merge_job(ctx, where, [ctxs[j] for j in sorted(ctxs)])
            if early and errors:
                break
        for i in sorted(ctxs):
            self.show_view(ctxs[i], where)
        if errors:
            raise errors[min(errors)]
        if not early:
            for i in range(len(jobs)):
                self.merge_job(ctxs[i], where, [ctxs[j] for j in range(len(jobs))])

    def execute(self, ctx: Ctx) -> None:
        """A timer job fires its timers in key order; a binding runs its step."""
        if not ctx.job.pulse:
            for timer in ctx.job.timers:
                FIRES[(ctx.job.sid, timer.kind)](ctx, timer)
            return
        STEPS.get((ctx.job.sid, ctx.job.pulse, ctx.job.phase), no_op)(ctx)

    def show_view(self, ctx: Ctx, where: str) -> None:
        """Record and print what a job saw, and keep its view and actions for the vectors."""
        rec = ctx.record(where)
        self.run.views.append(rec)
        self.run.jobs.append(job_vector(ctx, rec))
        parts = [f"(a)={list(rec.view.persons('a'))}", f"(b)={list(rec.view.persons('b'))}"]
        for kind in rec.observes:
            seen = " ".join(f"{ref_text(t)}#{e}" for e, _, t in rec.view.of_kind(kind))
            parts.append(f"(c){kind}=[{seen}]")
        parts.extend(f"children(P{p})={list(kids)}" for p, kids in rec.children)
        self.line(f"  view m{ctx.merge} {where} {rec.job}: " + " ".join(parts))
        for note in ctx.notes:
            self.line(f"    {note}")

    def compact(self) -> None:
        """Bookkeeping's compaction: the dead rows leave the ledger; the log keeps its entries."""
        removed = sorted(p for p, alive in self.world.ledger.items() if alive == 0)
        for p in removed:
            del self.world.ledger[p]
        if removed:
            self.line(f"  compaction: rows {removed} leave the ledger; the log keeps their entries")

    def end_instant(self) -> None:
        """The state hash at Bookkeeping, then the log is cleared."""
        plane_a = plane_a_text(self.world)
        hashed = plane_a + ("\nlog " + self.log.text() if HASH_INCLUDES_LOG in self.flags else "")
        self.run.hashes.append((self.day, digest(hashed), digest(plane_a)))
        self.run.scans[self.day] = list(self.log.scans)
        self.run.instants.append(
            {
                "day": self.day,
                "log_at_end": {
                    "inserted": [p for p, _ in self.log.inserted],
                    "exited": [p for p, _ in self.log.exited],
                    "events": [
                        [k, ref_text(t), ref_text(a), e] for k, t, a, e, _ in self.log.events
                    ],
                },
                "state_hash": digest(hashed),
            }
        )
        self.line(f"  end: log {self.log.text()}")
        self.line(f"  end: state_hash {digest(hashed)}; the log is cleared")
        if LOG_NOT_CLEARED not in self.flags:
            self.log.clear()
        if GENEALOGY_LATE in self.flags:
            self.genealogy.extend(self.world.life_course)

    # the merge --------------------------------------------------------------------------------
    def merge_job(self, ctx: Ctx, where: str, level: list[Ctx]) -> None:
        """One job's merge, in rule 4's order: events, timers, commands, then its column writes.

        Then the row inserts and the exit scan; the log and the index are filled here.
        """
        out = ctx.out
        assigned: list[int] = []
        for draft in out.events:
            assigned.append(self.merge_event(ctx, draft, assigned, level))
        for td in out.timers:
            cause = NONE if td.cause is None else self.resolve(ctx, td.cause, assigned, level)
            self.world.agenda.append(
                Timer(
                    td.day,
                    td.phase,
                    td.entity,
                    self.world.next_seq,
                    ctx.job.sid,
                    td.kind,
                    td.payload,
                    cause,
                )
            )
            self.world.next_seq += 1
        self.world.queue = [c for c in self.world.queue if c not in out.taken]
        for command in out.commands:
            self.world.queue.append(replace(command, seq=self.world.next_seq))
            self.world.next_seq += 1
        self.world.titles.update(out.titles)
        self.world.named.update(out.named)
        for hid in out.houses:
            self.world.houses[hid] = (self.world.houses[hid][0], True)
        for pid in out.kills:
            self.world.ledger[pid] = 0
        for pid, mother, father in out.inserts:
            self.insert_row(ctx, pid, mother, father)
        sort = UNSORTED_EXITED not in self.flags
        exits = self.log.scan_exits(self.world.ledger, ctx.merge, sort=sort) if out.kills else []
        if out.inserts and GENEALOGY_LATE not in self.flags:
            self.genealogy.extend(self.world.life_course)
        self.record_merge(ctx, where, assigned, exits)

    def merge_event(self, ctx: Ctx, draft: Draft, assigned: list[int], level: list[Ctx]) -> int:
        """Resolve the draft's causes, assign its id, append it; record it in (c) if observed."""
        causes = tuple(self.resolve(ctx, c, assigned, level) for c in draft.causes)
        eid = self.world.next_event
        self.world.next_event += 1
        self.world.events.append(Event(eid, self.day, ctx.job.phase, ctx.job.sid, draft, causes))
        self.run.event_merge[(self.day, eid)] = ctx.merge
        if draft.kind in self.observed:
            self.log.events.append((draft.kind, draft.target, draft.actor, eid, ctx.merge))
        return eid

    def resolve(self, ctx: Ctx, cause: Cause, assigned: list[int], level: list[Ctx]) -> int:
        """An EventRef slot at the merge.

        A draft ref to this step's id, a known id as is, the sugar against (c) as the step saw it.
        """
        if cause.kind:
            return self.resolve_same_instant(ctx, cause, level)
        if cause.draft != NONE:
            return assigned[cause.draft]
        return cause.known

    def resolve_same_instant(self, ctx: Ctx, cause: Cause, level: list[Ctx]) -> int:
        """The sugar: exactly one entry of (kind, target) merged before the step's level.

        The watermark is the level's merge index, so the jobs merged earlier in this same merge (the
        level's earlier jobs in canonical order) stay invisible, as to the step's own reads. Under
        the late-watermark control the form reads the log as it stands at the job's own merge.
        """
        late = PEER_VISIBLE in self.flags or LATE_WATERMARK in self.flags
        watermark = FAR if late else ctx.merge
        found = [
            e
            for k, t, _, e, m in self.log.events
            if k == cause.kind and t == cause.target and m < watermark
        ]
        if len(found) == 1:
            self.run.resolutions.append(
                Resolution(self.day, ctx.merge, ctx.job.sid, cause, found[0])
            )
            self.line(f"  resolve m{ctx.merge} {ctx.job.label()} {cause.text()} -> #{found[0]}")
            return found[0]
        raise refusal(ctx, cause, found, level)

    def insert_row(self, ctx: Ctx, pid: int, mother: int, father: int) -> None:
        """rh-core's row insert.

        Refused for an id the ledger holds or one that exited this instant (no re-entry), else the
        rows, (a)'s sorted insert and nothing else.
        """
        if pid in self.world.ledger:
            raise RefusedError(
                "duplicate",
                f"{ctx.job.label()} inserts P{pid}, whose ledger row is still there (dead this "
                "instant, a row stays until Bookkeeping's compaction): SimError::Ledger, the "
                "duplicate-id refusal of SPR-014's insert_rows",
            )
        if any(p == pid for p, _ in self.log.exited):
            raise RefusedError(
                "reentry",
                f"{ctx.job.label()} inserts P{pid}, who exited this instant: no re-entry within "
                "an instant, SimError::Ledger (EVO-002 §2.3)",
            )
        self.world.ledger[pid] = 1
        self.world.life_course.append((pid, mother, father))
        self.log.add_inserted(pid, ctx.merge, sort=UNSORTED_INSERTED not in self.flags)

    def record_merge(self, ctx: Ctx, where: str, assigned: list[int], exits: list[int]) -> None:
        """Keep the job's merge for the oracles and the vectors, and print it."""
        out = ctx.out
        events = tuple((d.kind, d.target, e) for d, e in zip(out.events, assigned, strict=True))
        merged = {
            "event_ids": list(assigned),
            "inserted": [pid for pid, _, _ in out.inserts],
            "exits": list(exits),
        }
        for job in reversed(self.run.jobs):
            if (job["day"], job["merge"], job["job"]) == (self.day, ctx.merge, ctx.job.label()):
                job["merged"] = merged
                break
        self.run.merges.append(
            JobMerge(
                self.day,
                ctx.merge,
                ctx.job.sid,
                tuple(p for p, _, _ in out.inserts),
                tuple(sorted(set(out.kills))),
                events,
                tuple(out.inserts),
            )
        )
        parts = [
            "events "
            + " ".join(
                f"{k}:{ref_text(t)}#{e}{'' if k in self.observed else '(unobserved)'}"
                for k, t, e in events
            )
            if events
            else "",
            f"+(a) {[p for p, _, _ in out.inserts]}" if out.inserts else "",
            f"+(b) {exits}" if exits else "",
            f"timers {len(out.timers)}" if out.timers else "",
            f"commands {len(out.commands)}" if out.commands else "",
            f"taken {len(out.taken)}" if out.taken else "",
            "named " + " ".join(f"P{p}:{v}" for p, v in sorted(out.named.items()))
            if out.named
            else "",
        ]
        shown = "; ".join(p for p in parts if p) or "nothing"
        self.line(f"  merge m{ctx.merge} {where} {ctx.job.label()}: {shown}")


def refusal(ctx: Ctx, cause: Cause, found: list[int], level: list[Ctx]) -> RefusedError:
    """Why the sugar did not resolve, by name.

    Ambiguous, the step's own draft, a same-level peer's event, or absent.
    """
    what = f"{ctx.job.label()} names {cause.text()}"
    if found:
        return RefusedError(
            "ambiguous",
            f"{what}, which matches {len(found)} events {found}: name one with the direct read "
            "(SimError::InstantCause(ambiguous))",
        )
    emitters = [
        c.job.label()
        for c in level
        if any(d.kind == cause.kind and d.target == cause.target for d in c.out.events)
    ]
    if ctx.job.label() in emitters:
        return RefusedError(
            "own_draft",
            f"{what}, its own draft of this step: name it by its DraftRef "
            "(SimError::InstantCause(own_draft))",
        )
    if emitters:
        return RefusedError(
            "same_level",
            f"{what}, which {emitters[0]} emits in its own level (merge m{ctx.merge}): a "
            "same-level peer is never visible (SimError::InstantCause(same_level))",
        )
    return RefusedError(
        "absent",
        f"{what}, but no {cause.kind} about {ref_text(cause.target)} was merged before its level "
        "this instant (SimError::InstantCause(absent))",
    )


# ----------------------------------------------------------------------------- the stand-in steps


def no_op(ctx: Ctx) -> None:
    """A binding that only reads (its view is recorded)."""
    del ctx


def m2_death(ctx: Ctx, timer: Timer) -> None:
    """M2's death timer: ``apply_death`` (alive := 0) and DEATH, target the deceased."""
    pid = timer.entity[1]
    if ctx.alive(pid):
        ctx.kill(pid)
        ctx.emit(Draft(DEATH, person(pid), None))


def m2_birth(ctx: Ctx, timer: Timer) -> None:
    """M2's birth timer: the row insert and BIRTH (actor the mother, target the child)."""
    mother, child = timer.entity[1], timer.payload
    father = dict(ctx.script.fathers)[child]
    ctx.insert(child, mother, father)
    ctx.emit(Draft(BIRTH, person(child), person(mother)))


def m2_marry(ctx: Ctx, timer: Timer) -> None:
    """M2's marriage timer: MARRIAGE (actor the husband, target the wife)."""
    ctx.emit(Draft(MARRIAGE, timer.entity, person(timer.payload)))


def m2_day_vital(ctx: Ctx) -> None:
    """M2's Day/Vital binding.

    Drain ``ScheduleDeath``; the model-only newborn death, re-entry and late birth (a later merge's
    insert below an earlier one) of three scenarios.
    """
    for command in ctx.take_commands():
        if ctx.alive(command.victim):
            ctx.kill(command.victim)
            culprit = person(command.culprit) if command.culprit else None
            ctx.emit(Draft(DEATH, person(command.victim), culprit))
    if ctx.script.newborn_kill:
        ctx.kill(ctx.script.newborn_kill)
        ctx.emit(Draft(DEATH, person(ctx.script.newborn_kill), None))
    if ctx.script.reinsert_at == VITAL:
        ctx.insert(ctx.script.reinsert, 0, 0)
    for child, mother, father in ctx.script.late_births:
        ctx.insert(child, mother, father)
        ctx.emit(Draft(BIRTH, person(child), person(mother)))


def m2_day_bookkeeping(ctx: Ctx) -> None:
    """A model-only M2 binding in Bookkeeping, after compaction: the re-entry probe."""
    if ctx.script.reinsert_at == BOOKKEEPING:
        ctx.insert(ctx.script.reinsert, 0, 0)


def m7_murder_due(ctx: Ctx, timer: Timer) -> None:
    """M7's MURDER_DUE fire: submit ScheduleDeath for a living ruler, else drop the timer."""
    ruler = timer.entity[1]
    if ctx.alive(ruler):
        ctx.submit(Command(M2, "ScheduleDeath", ruler, timer.payload))
    else:
        ctx.note(f"drop_timer(dead): MURDER_DUE about P{ruler}")


def m7_day_polity(ctx: Ctx) -> None:
    """M7's Day/Polity binding (M7 0.6.0).

    On_birth reads (a); on_death reads (b) in two phases (K6): phase 1 counts the houses, phase 2
    arbitrates by tier then TitleId, and HOUSE_EXTINCT follows the step's successions and crises, in
    ascending HouseId.
    """
    for parent in ctx.script.kin_probe:
        ctx.children(parent)
    for child in ctx.inserted():
        father = ctx.father(child)
        if father:
            ctx.children(father)
    dead = ctx.exited()
    if not dead:
        return
    ended = m7_houses(ctx, dead)
    vacated = [t for t in ctx.title_ids() if ctx.title(t).holder in dead]
    for tid in sorted(vacated, key=lambda t: (ctx.title(t).tier, t)):
        m7_arbitrate(ctx, tid)
    for hid, last in ended:
        ctx.emit(
            Draft("HOUSE_EXTINCT", person(last), None, (same_instant(DEATH, person(last)),), hid)
        )


def m7_houses(ctx: Ctx, dead: tuple[int, ...]) -> list[tuple[int, int]]:
    """on_death, phase 1's house count.

    Each house whose last living member died this instant is marked extinct; returns (house, the
    member with the largest PersonId among its dead).
    """
    ended = []
    for hid, members, extinct in ctx.houses():
        gone = [p for p in members if p in dead]
        if extinct or not gone or any(ctx.alive(p) for p in members):
            continue
        ctx.write_house(hid)
        ended.append((hid, gone[-1]))
    return ended


def m7_arbitrate(ctx: Ctx, tid: int) -> None:
    """on_death, phase 2, one vacated title (scripted outcomes).

    Its heir, else a sovereign's crisis with its timer, else a vassal joins its liege's interregnum
    (K2), else reverts.
    """
    t = ctx.title(tid)
    death = (same_instant(DEATH, person(t.holder)),)
    gone = replace(t, holder=0, state=INTERREGNUM, predecessor=t.holder)
    if t.heir and ctx.alive(t.heir):
        ctx.emit(Draft("SUCCEED", person(t.holder), person(t.heir), death, tid))
        ctx.write_title(tid, replace(t, holder=t.heir, predecessor=t.holder))
    elif t.tier == SOVEREIGN:
        crisis = ctx.emit(Draft("INTERREGNUM_CRISIS", None, None, death, tid))
        resolution = TimerDraft(ctx.day + ctx.script.delay, POLITY, ("T", tid), "RESOLVE")
        ctx.schedule(replace(resolution, cause=own_draft(crisis)))
        ctx.write_title(tid, gone)
    elif ctx.title(t.liege).state == INTERREGNUM:
        ctx.emit(Draft("INTERREGNUM_CRISIS", None, None, death, tid))
        ctx.write_title(tid, gone)
    else:
        liege = ctx.title(t.liege).holder
        ctx.emit(Draft("SUCCEED", person(t.holder), person(liege), death, tid))
        ctx.write_title(tid, replace(t, holder=liege, predecessor=t.holder))


def m7_resolve(ctx: Ctx, timer: Timer) -> None:
    """M7's interregnum-resolution timer.

    SUCCEED triggered to the crisis the timer carries, then each vassal that joined, after its
    liege, by tier then TitleId.
    """
    tid = timer.entity[1]
    t = ctx.title(tid)
    crisis = (known(timer.cause),)
    ctx.emit(Draft("SUCCEED", person(t.predecessor), person(t.resolve_to), crisis, tid))
    ctx.write_title(tid, replace(t, holder=t.resolve_to, state=HELD))
    joined = [
        v
        for v in ctx.title_ids()
        if ctx.title(v).liege == tid and ctx.title(v).state == INTERREGNUM
    ]
    for vid in sorted(joined, key=lambda v: (ctx.title(v).tier, v)):
        v = ctx.title(vid)
        ctx.emit(Draft("SUCCEED", person(v.predecessor), person(t.resolve_to), crisis, vid))
        ctx.write_title(vid, replace(v, holder=t.resolve_to, state=HELD))


def m7_year_polity(ctx: Ctx) -> None:
    """M7's Year/Polity binding (K12): a holder dead this instant still holds for this pass."""
    for tid in ctx.title_ids():
        holder = ctx.title(tid).holder
        if holder and holder in ctx.exited():
            ctx.note(
                f"K12: T{tid} is still held by P{holder}, dead this instant, for this Year pass; "
                "on_death arbitrates it in Day/Polity, the next merge"
            )
    m7_dormancy(ctx)


def m7_dormancy(ctx: Ctx) -> None:
    """M7's dormancy step, at tick 0 the world-start naming pass (M7 0.6.0; X4).

    A person meets a naming criterion when it is of the manifest's cast (criterion (e), the
    Architect's cast ruling) or when the script gives it another; a person not named who meets one
    is Promoted, a named adult who meets none is Demoted (every scripted person is an adult). Under
    the cast-demoted control the cast is not a criterion.
    """
    if not (ctx.script.cast or ctx.script.criteria):
        return
    others = dict(ctx.script.criteria)
    for pid in ctx.persons():
        reasons = [others[pid]] if pid in others else []
        if pid in ctx.script.cast and CAST_DEMOTED not in ctx.engine.flags:
            reasons.insert(0, "of the manifest's cast, criterion (e)")
        named = ctx.is_named(pid)
        if reasons and not named:
            ctx.submit(Command(T1, "Promote", pid, 0))
            outcome = "Promote"
        elif named and not reasons:
            ctx.submit(Command(T1, "Demote", pid, 0))
            outcome = "Demote"
        else:
            outcome = "stays named" if named else "stays dormant"
        ctx.note(f"dormancy P{pid}: {'; '.join(reasons) or 'meets no criterion'}: {outcome}")


def t1_grief(ctx: Ctx) -> None:
    """T1's Day/Vital binding.

    (b) the dead; (c) each DEATH's id by the direct read; the crossing death's break timer carries
    the sugar in its cause slot.
    """
    crossings = dict(ctx.script.crossings)
    for dead in ctx.exited():
        ids = ctx.instant_event(DEATH, person(dead))
        ctx.note(f"instant_event(DEATH, P{dead}) = {list(ids)}")
        griever = crossings.get(dead)
        if griever:
            ctx.schedule(
                TimerDraft(
                    ctx.day + 1,
                    AGENDA,
                    person(griever),
                    "BREAK",
                    cause=same_instant(DEATH, person(dead)),
                )
            )


def t1_agenda(ctx: Ctx) -> None:
    """T1's Day/Agenda binding: events_of(MARRIAGE) (R10); Promote and Demote; FEUD_ENDED."""
    marriages = ctx.events_of(MARRIAGE)
    if marriages:
        shown = " ".join(f"#{e} actor={ref_text(a)} target={ref_text(t)}" for e, a, t in marriages)
        ctx.note(f"events_of(MARRIAGE) = [{shown}]")
    for command in ctx.take_commands():
        value = 1 if command.kind == "Promote" else 0
        ctx.set_named(command.victim, value)
        ctx.note(f"apply {command.kind} P{command.victim}: person.named := {value}")
    dead = ctx.exited()
    for a, b in ctx.script.feuds:
        for survivor, gone in ((a, b), (b, a)):
            if gone in dead and survivor not in dead:
                cause = (same_instant(DEATH, person(gone)),)
                ctx.emit(Draft("FEUD_ENDED", person(gone), person(survivor), cause))


def t1_break(ctx: Ctx, timer: Timer) -> None:
    """T1's break timer: MENTAL_BREAK, motivated by the DEATH id its cause slot holds."""
    ctx.emit(Draft("MENTAL_BREAK", None, timer.entity, (known(timer.cause),)))


def x1_mark(ctx: Ctx) -> None:
    """X1: X_MARK about P9, as many times as the script says."""
    for _ in range(ctx.script.x_marks):
        ctx.emit(Draft(X_MARK, person(9), None))


def x2_note(ctx: Ctx) -> None:
    """X2: the direct read of X_MARK about P9, then X_NOTE caused by the sugar."""
    ctx.note(f"instant_event(X_MARK, P9) = {list(ctx.instant_event(X_MARK, person(9)))}")
    ctx.emit(Draft("X_NOTE", person(9), None, (same_instant(X_MARK, person(9)),)))


def x3_probe(ctx: Ctx) -> None:
    """X3: an event whose cause is the script's probe."""
    probe = ctx.script.probe
    if probe is not None:
        ctx.emit(Draft("X_PROBE", probe.target, None, (probe,)))


STEPS: dict[tuple[str, str, str], Callable[[Ctx], None]] = {
    (M2, DAY, VITAL): m2_day_vital,
    (M2, DAY, BOOKKEEPING): m2_day_bookkeeping,
    (M7, DAY, POLITY): m7_day_polity,
    (M7, YEAR, POLITY): m7_year_polity,
    (T1, DAY, VITAL): t1_grief,
    (T1, DAY, AGENDA): t1_agenda,
    (X1, DAY, AGENDA): x1_mark,
    (X2, DAY, AGENDA): x2_note,
    (X3, DAY, POLITY): x3_probe,
}
FIRES: dict[tuple[str, str], Callable[[Ctx, Timer], None]] = {
    (M2, "DEATH_TIMER"): m2_death,
    (M2, "BIRTH_TIMER"): m2_birth,
    (M2, "MARRY_TIMER"): m2_marry,
    (M7, "MURDER_DUE"): m7_murder_due,
    (M7, "RESOLVE"): m7_resolve,
    (T1, "BREAK"): t1_break,
}


# ----------------------------------------------------------------------------- the scenarios


def founders(*pids: int, parents: tuple[tuple[int, int, int], ...] = ()) -> World:
    """A world of living founders; ``parents`` gives (child, mother, father) for some of them."""
    known_parents = {c: (m, f) for c, m, f in parents}
    rows = [(p, *known_parents.get(p, (0, 0))) for p in pids]
    return World({p: 1 for p in pids}, rows)


def death(day: int, pid: int) -> TimerDraft:
    """M2's death timer for ``pid`` at (day, Vital)."""
    return TimerDraft(day, VITAL, person(pid), "DEATH_TIMER")


def birth(day: int, mother: int, child: int) -> TimerDraft:
    """M2's birth timer of ``child`` to ``mother`` at (day, Vital)."""
    return TimerDraft(day, VITAL, person(mother), "BIRTH_TIMER", child)


M2_SYS = System(M2, ((DAY, VITAL),))
M7_SYS = System(M7, ((DAY, VITAL), (DAY, POLITY), (DAY, AGENDA)), (M2,), (DEATH,))
T1_SYS = System(T1, ((DAY, VITAL), (DAY, AGENDA)), (M2,), (DEATH, MARRIAGE))
S3_SYS = System(S3, ((DAY, VITAL),), (M2,))
B1_SYS = System(B1, ((DAY, BOOKKEEPING),), (M2,), (DEATH,))
X1_SYS = System(X1, ((DAY, AGENDA),))
X3_SYS = System(X3, ((DAY, POLITY),), (M2,), (DEATH,))


def succession_world() -> World:
    """The succession day.

    P5, the last of House 11, and P8, holder of the minor T303, die in M2's timer pass; P3, who
    holds the sovereign T707 and the major T505, is murdered later in the same Vital through M7's
    timer and M2's drain, so the later merge's death has the lower PersonId ((b)'s sorted insert
    across merges, R9); P40 and P30 are born (in that key order, so (a)'s sorted insert within a
    merge matters); P9 marries P7. P1 and P3 are founder parents (rows written at creation).
    """
    w = founders(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, parents=((2, 0, 1), (3, 0, 1), (4, 0, 3)))
    w.titles = {
        707: Title(SOVEREIGN, 0, 3, resolve_to=4),
        505: Title(MAJOR, 707, 3, heir=4),
        303: Title(MINOR, 707, 8),
    }
    w.houses = {11: ((5,), False), 12: ((1, 2, 3, 4), False)}
    for draft in (death(1, 5), death(1, 8), birth(1, 6, 40), birth(1, 7, 30)):
        w.seed(M2, draft)
    w.seed(M2, TimerDraft(1, VITAL, person(7), "MARRY_TIMER", 9))
    return w.seed(M7, TimerDraft(1, VITAL, person(3), "MURDER_DUE", 9))


def one_death(pid: int, *others: int) -> Callable[[], World]:
    """A world of ``pid`` and ``others``, ``pid`` dying at day 1."""
    return lambda: founders(pid, *others).seed(M2, death(1, pid))


def year_world() -> World:
    """A year start (day 360) at which the sovereign's holder P3 dies."""
    w = founders(3, 4)
    w.titles = {707: Title(SOVEREIGN, 0, 3, heir=4)}
    return w.seed(M2, death(DAYS_PER_YEAR, 3))


def cast_world() -> World:
    """Tick 0: genesis named P1 (the manifest's cast) and P3; P2 holds the sovereign T707."""
    w = founders(1, 2, 3)
    w.titles = {707: Title(SOVEREIGN, 0, 2)}
    w.named = {1: 1, 3: 1}
    return w


def newborn_world() -> World:
    """P50 is born at day 1 to P6 and P2 (M2's timer pass)."""
    return founders(2, 6).seed(M2, birth(1, 6, 50))


PEER_SYS = System(X2, ((DAY, AGENDA),), (), (X_MARK,))
NEXT_SYS = System(X2, ((DAY, AGENDA),), (X1,), (X_MARK,))
SCENARIOS = (
    Scenario(
        "succession_day",
        "deaths by timer and by drain at two merges, the drain's death below the timer pass's "
        "(R9), two births, a marriage, M7's successions and crises, T1's grief and feud, a "
        "founder parent's children read every day, a keyframe after day 1, the crisis resolved "
        "at day 3",
        (M2_SYS, M7_SYS, S3_SYS, T1_SYS, B1_SYS),
        succession_world,
        (1, 2, 3),
        Script(fathers=((40, 2), (30, 10)), crossings=((5, 6),), feuds=((9, 8),), kin_probe=(1, 3)),
        keyframe_after=1,
        readers=(S3, B1),
    ),
    Scenario(
        "same_level_peer",
        "X2 and X1 share a level (no edge): X2's direct read sees nothing, and its cause naming "
        "X1's event is refused by name",
        (X1_SYS, PEER_SYS),
        lambda: founders(9),
        (1,),
        Script(x_marks=1),
        "refused:same_level",
    ),
    Scenario(
        "next_level",
        "the same pair with X2 reading X1 at same_instant: X2 runs a level later and resolves",
        (X1_SYS, NEXT_SYS),
        lambda: founders(9),
        (1,),
        Script(x_marks=1),
    ),
    Scenario(
        "absent_target",
        "X3 names the same-instant DEATH of P77, who did not die: refused by name",
        (M2_SYS, X3_SYS),
        one_death(5, 77),
        (1,),
        Script(probe=same_instant(DEATH, person(77))),
        "refused:absent",
    ),
    Scenario(
        "ambiguous",
        "X1 emits two X_MARK about P9; X2, a level later, reads both and its sugar is refused",
        (X1_SYS, NEXT_SYS),
        lambda: founders(9),
        (1,),
        Script(x_marks=2),
        "refused:ambiguous",
    ),
    Scenario(
        "undeclared_kind",
        "X3 names a same-instant BIRTH, a kind it does not observe: refused at the step",
        (M2_SYS, X3_SYS),
        newborn_world,
        (1,),
        Script(fathers=((50, 2),), probe=same_instant(BIRTH, person(50))),
        "refused:undeclared",
    ),
    Scenario(
        "death_then_reentry",
        "P4 dies in M2's timer pass; M2's Day/Vital binding inserts P4 again: the dead row is "
        "still in the ledger, so the insert is a duplicate",
        (M2_SYS,),
        one_death(4),
        (1,),
        Script(reinsert=4, reinsert_at=VITAL),
        "refused:duplicate",
    ),
    Scenario(
        "reentry_after_compaction",
        "model only (no P1a system inserts at Bookkeeping): after compaction P4's row is gone, "
        "and the insert is refused because P4 exited this instant",
        (System(M2, ((DAY, VITAL), (DAY, BOOKKEEPING))),),
        one_death(4),
        (1,),
        Script(reinsert=4, reinsert_at=BOOKKEEPING),
        "refused:reentry",
    ),
    Scenario(
        "newborn_dies",
        "model only (no P1a sender kills a person born in the same instant, and every P1a "
        "birth is a timer fire): P50 is born at the timer pass and dies at M2's binding, in both "
        "lists; P45 is born at M2's binding, a later merge, below P50 ((a)'s sorted insert "
        "across merges); compaction removes P50's row, not its entries",
        (M2_SYS, S3_SYS, B1_SYS),
        newborn_world,
        (1,),
        Script(fathers=((50, 2),), newborn_kill=50, late_births=((45, 6, 2),)),
    ),
    Scenario(
        "year_start",
        "a year start: M7's Year/Polity and Day/Polity share a level and split, Year first; "
        "the holder dead this instant still holds for the Year pass (K12)",
        (M2_SYS, System(M7, ((DAY, VITAL), (YEAR, POLITY), (DAY, POLITY)), (M2,), (DEATH,))),
        year_world,
        (DAYS_PER_YEAR,),
        expect_seen=("K12: T707 is still held by P3, dead this instant, for this Year pass",),
    ),
    Scenario(
        "world_start_cast",
        "tick 0, the world's first instant: M7's Year/Polity dormancy step is the world-start "
        "naming pass (X4); P1, of the manifest's cast and meeting no other criterion, stays named; "
        "P2, a holder genesis did not name, is Promoted; P3, a named adult meeting none, is "
        "Demoted; T1 applies both at Day/Agenda of the same instant",
        (System(M7, ((YEAR, POLITY),)), System(T1, ((DAY, AGENDA),), (), (DEATH, MARRIAGE))),
        cast_world,
        (0,),
        Script(cast=(1,), criteria=((2, "holds T707, criterion (a)"),)),
        expect_named=(1, 2),
    ),
)


# ----------------------------------------------------------------------------- vectors


def opt_ref(ref: Ref | None) -> str | None:
    """An entity for the vectors: ``"P5"``, ``"T707"``, or null."""
    return None if ref is None else ref_text(ref)


def job_vector(ctx: Ctx, rec: ViewRecord) -> dict[str, object]:
    """One job for the vectors: where it ran, what it saw, and what it buffered (its actions)."""
    out = ctx.out
    view = rec.view
    return {
        "day": rec.day,
        "merge": rec.merge,
        "where": rec.where,
        "job": rec.job,
        "view": {
            "inserted": list(view.persons("a")),
            "exited": list(view.persons("b")),
            "events": {
                kind: [[opt_ref(t), opt_ref(a), e] for e, a, t in view.of_kind(kind)]
                for kind in rec.observes
            },
            "children": [[p, list(kids)] for p, kids in rec.children],
        },
        "actions": {
            "events": [
                {
                    "kind": d.kind,
                    "target": opt_ref(d.target),
                    "actor": opt_ref(d.actor),
                    "payload": d.payload,
                    "causes": [c.text() for c in d.causes],
                }
                for d in out.events
            ],
            "timers": [
                [
                    t.day,
                    t.phase,
                    ref_text(t.entity),
                    t.kind,
                    t.payload,
                    None if t.cause is None else t.cause.text(),
                ]
                for t in out.timers
            ],
            "commands": [[c.owner, c.kind, c.victim, c.culprit] for c in out.commands],
            "taken": [[c.owner, c.kind, c.victim, c.culprit] for c in out.taken],
            "kills": list(out.kills),
            "inserts": [list(r) for r in out.inserts],
            "named": [[pid, v] for pid, v in sorted(out.named.items())],
        },
    }


def world_vector(world: World) -> dict[str, object]:
    """A world at creation, for the vectors."""
    return {
        "ledger": [[pid, alive] for pid, alive in sorted(world.ledger.items())],
        "life_course": [list(r) for r in world.life_course],
        "titles": [
            [tid, x.tier, x.liege, x.holder, x.heir, x.resolve_to, x.state]
            for tid, x in sorted(world.titles.items())
        ],
        "houses": [[hid, list(m), x] for hid, (m, x) in sorted(world.houses.items())],
        "named": [[pid, v] for pid, v in sorted(world.named.items())],
        "agenda": [
            [t.day, t.phase, ref_text(t.entity), t.seq, t.owner, t.kind, t.payload]
            for t in sorted(world.agenda, key=Timer.key)
        ],
    }


def scenario_vector(sc: Scenario, run: Run) -> dict[str, object]:
    """One scenario's vectors.

    Its registration, its world, each job's view, actions and merge, each resolution and refusal,
    and each instant's log and state hash at its end.
    """
    return {
        "name": sc.name,
        "days": list(sc.days),
        "systems": [
            {
                "id": s.sid,
                "bindings": [[pu, ph] for pu, ph in s.bindings],
                "same_instant_after": list(s.after),
                "observes": list(s.observes),
            }
            for s in sc.systems
        ],
        "world": world_vector(sc.world()),
        "jobs": run.jobs,
        "resolutions": [[r.day, r.merge, r.sid, r.cause.text(), r.eid] for r in run.resolutions],
        "instants": run.instants,
        "outcome": run.outcome(),
        "refusal": None if run.refusal is None else run.refusal[1],
    }


# ----------------------------------------------------------------------------- runs and checks


def run_scenario(
    sc: Scenario,
    flags: frozenset[str],
    orders: dict[LevelKey, tuple[int, ...]] | None = None,
    start: World | None = None,
    days: tuple[int, ...] | None = None,
) -> Run:
    """Run a scenario (or its tail from a keyframe) until its last instant or a refusal."""
    engine = Engine(sc, start.copy() if start else sc.world(), flags, orders or {})
    for day in days if days is not None else sc.days:
        try:
            engine.run_day(day)
        except RefusedError as e:
            engine.run.refusal = (e.reason, e.message)
            engine.line(f"  REFUSED ({e.reason}): {e.message}")
            break
        if day == sc.keyframe_after:
            engine.run.keyframe = engine.world.copy()
    engine.run.named_at_end = tuple(sorted(p for p, v in engine.world.named.items() if v))
    return engine.run


def check_cleared(run: Run) -> list[str]:
    """1. The log is empty at every instant's start."""
    return [
        f"CLEARED: the instant log held {n} entries at the start of day {d} "
        "(it is cleared at every instant's end)"
        for d, n in run.log_at_start
        if n
    ]


def classify(
    v: ViewRecord, got: list[tuple[object, int]], want: set[object], merges: list[JobMerge]
) -> list[str]:
    """Each wrong entry of one part of a view, named: a peer's, a stale one, or a missed one."""
    errs = []
    own_level = {x for jm in merges if jm.merge == v.merge for x in entry_keys(jm)}
    for key, m in got:
        if key in want:
            continue
        kind = (
            "an entry its own level merged (a same-level peer)"
            if m == v.merge and key in own_level
            else "an entry no earlier merge of this instant recorded (a stale entry)"
        )
        errs.append(f"VISIBILITY: {v.job} at {v.where} (day {v.day}) saw {kind}: {key}")
    missed = want - {key for key, _ in got}
    errs.extend(
        f"VISIBILITY: {v.job} at {v.where} missed {key}" for key in sorted(map(str, missed))
    )
    return errs


def entry_keys(jm: JobMerge) -> list[object]:
    """Every log key a job's merge can give: its inserts, its kills, its events."""
    return [*(("a", p) for p in jm.inserted), *(("b", p) for p in jm.killed), *jm.events]


def view_errors(v: ViewRecord, merges: list[JobMerge]) -> list[str]:
    """2. One view against the merges of its instant with a lower index."""
    before = [jm for jm in merges if jm.merge < v.merge]
    want: set[object] = {("a", p) for jm in before for p in jm.inserted}
    want |= {("b", p) for jm in before for p in jm.killed}
    want |= {e for jm in before for e in jm.events if e[0] in v.observes}
    got: list[tuple[object, int]] = [(("a", p), m) for p, m in v.view.inserted]
    got += [(("b", p), m) for p, m in v.view.exited]
    got += [((k, t, e), m) for k, t, _, e, m in v.view.events if k in v.observes]
    return classify(v, got, want, merges)


def check_visibility(run: Run) -> list[str]:
    """2.

    Every view sees exactly the earlier merges of its instant; every resolution names an id an
    earlier merge assigned.
    """
    errs = []
    for v in run.views:
        errs += view_errors(v, [jm for jm in run.merges if jm.day == v.day])
    for r in run.resolutions:
        m = run.event_merge.get((r.day, r.eid), FAR)
        if m >= r.merge:
            errs.append(
                f"VISIBILITY: {r.sid} resolved the merge-time form {r.cause.text()} at m{r.merge} "
                f"to #{r.eid}, an event merged at m{m}: an entry its own level merged (the form "
                "reads the log as the level started, never a same-level peer)"
            )
    return errs


def check_scan(run: Run) -> list[str]:
    """4. (b)'s scan of ``person.alive`` ran exactly at the merges whose job wrote it."""
    errs = []
    for day, scans in run.scans.items():
        want = sorted({jm.merge for jm in run.merges if jm.day == day and jm.killed})
        if sorted(set(scans)) != want:
            errs.append(
                f"SCAN: on day {day} (b)'s scan ran at merges {sorted(set(scans))}; only the "
                f"merges whose job wrote person.alive, {want}, may run it (SPR-038's written flag)"
            )
    return errs


def check_order(run: Run) -> list[str]:
    """3. (a) and (b) ascending without repeats; (c) in id order."""
    errs = []
    for v in run.views:
        for part, name in (("a", "inserted()"), ("b", "exited()")):
            ps = list(v.view.persons(part))
            if ps != sorted(set(ps)):
                errs.append(
                    f"ORDER: {name} is not in ascending row order at {v.where} {v.job}: {ps}"
                )
        ids = [e for _, _, _, e, _ in v.view.events]
        if ids != sorted(ids):
            errs.append(f"ORDER: (c) is not in id order at {v.where} {v.job}: {ids}")
    return errs


def check_genealogy(run: Run) -> list[str]:
    """5. Every children(p) a step read equals a rebuild over the rows merged before it."""
    errs = []
    for v in run.views:
        rows = run.rows_at_start[v.day] + [
            r for jm in run.merges if jm.day == v.day and jm.merge < v.merge for r in jm.rows
        ]
        for parent, kids in v.children:
            want = rebuild_children(rows, parent)
            if kids != want:
                errs.append(
                    f"GENEALOGY: children(P{parent}) at {v.where} {v.job} is {list(kids)} where "
                    f"a rebuild of the rows merged before it gives {list(want)}"
                )
    return errs


def check_hash(run: Run) -> list[str]:
    """6. The state hash is plane A's digest: no log, no index."""
    return [
        f"HASH: the state hash of day {d} covers the instant log ({h} where plane A alone "
        f"gives {a})"
        for d, h, a in run.hashes
        if h != a
    ]


@dataclass(frozen=True)
class Verdict:
    """One scenario's checks, and the counts the golden file records."""

    errors: list[str]
    orders: int
    notes: tuple[str, ...]


def check_readers(sc: Scenario, run: Run, flags: frozenset[str]) -> tuple[list[str], str]:
    """7. Removing the pure readers changes no hash."""
    if not sc.readers:
        return [], ""
    bare = replace(sc, systems=tuple(s for s in sc.systems if s.sid not in sc.readers))
    other = run_scenario(bare, flags)
    same = [(d, h) for d, h, _ in other.hashes] == [(d, h) for d, h, _ in run.hashes]
    note = f"readers {list(sc.readers)} removed: the {len(run.hashes)} state hashes are equal"
    return ([] if same else [f"READERS: removing {list(sc.readers)} changed a state hash"]), note


def check_resume(sc: Scenario, run: Run, flags: frozenset[str]) -> tuple[list[str], str]:
    """8. A run resumed from the keyframe cut after day k equals the continuous run."""
    if run.keyframe is None:
        return [], ""
    tail = tuple(d for d in sc.days if d > sc.keyframe_after)
    resumed = run_scenario(sc, flags, start=run.keyframe, days=tail)
    errs = [
        f"RESUME: day {d} of the run resumed from the keyframe of day {sc.keyframe_after} "
        "differs from the continuous run"
        for d in tail
        if resumed.day_lines.get(d) != run.day_lines.get(d)
    ]
    return errs, f"resumed from the keyframe of day {sc.keyframe_after}: days {list(tail)} equal"


def check_threads(sc: Scenario, run: Run, flags: frozenset[str]) -> tuple[list[str], int]:
    """9.

    Every permutation of every multi-job level's execution order gives the same transcript.
    """
    errs, count = [], 0
    for key, n in run.levels:
        for perm in itertools.permutations(range(n)):
            if list(perm) == sorted(perm):
                continue
            count += 1
            other = run_scenario(sc, flags, orders={key: perm})
            if other.lines != run.lines:
                errs.append(
                    f"THREAD INVARIANCE: job order {list(perm)} at day {key.day} "
                    f"{key.phase}/{key.pass_name} L{key.level} changed the transcript"
                )
    return errs, count


def verdict(sc: Scenario, run: Run, flags: frozenset[str]) -> Verdict:
    """Every check of one scenario."""
    errs = check_cleared(run) + check_visibility(run) + check_order(run) + check_scan(run)
    errs += check_genealogy(run) + check_hash(run)
    readers, readers_note = check_readers(sc, run, flags)
    resume, resume_note = check_resume(sc, run, flags)
    threads, count = check_threads(sc, run, flags)
    errs += readers + resume + threads
    if run.outcome() != sc.expect:
        errs.append(f"EXPECT: {sc.name} ended {run.outcome()}; its script expects {sc.expect}")
    errs += [
        f"EXPECT: {sc.name}'s transcript lacks {seen!r}"
        for seen in sc.expect_seen
        if not any(seen in line for line in run.lines)
    ]
    if sc.expect_named is not None and run.named_at_end != sc.expect_named:
        errs.append(
            f"NAMED: {sc.name} ends with {list(run.named_at_end)} named where its script expects "
            f"{list(sc.expect_named)} (a cast member is named from world start and never demoted)"
        )
    notes = tuple(n for n in (readers_note, resume_note) if n)
    return Verdict(errs, count, notes)


def self_test(control: str | None) -> tuple[list[str], str, str]:
    """Run every scenario and its checks; return the report, the golden text and the vectors."""
    flags = frozenset({control} if control else set())
    report: list[str] = []
    golden = [f"# {FORMAT}", "# Generated by specs/engine/instant_log_model.py; never typed."]
    vectors: list[dict[str, object]] = []
    failed = False
    for sc in SCENARIOS:
        run = run_scenario(sc, flags)
        vectors.append(scenario_vector(sc, run))
        v = verdict(sc, run, flags)
        failed |= bool(v.errors)
        status = "FAIL" if v.errors else "PASS"
        report.append(
            f"scenario {sc.name}: {run.outcome()}; {len(run.views)} views, "
            f"{len(run.resolutions)} resolutions, {v.orders} permuted orders: {status}"
        )
        report.extend(f"  {e}" for e in v.errors)
        golden += scenario_block(sc, run, v)
    report.append(f"INSTANT LOG MODEL v0: {'FAIL' if failed else 'PASS'}")
    doc = {
        "format": VECTOR_FORMAT,
        "generated_by": "specs/engine/instant_log_model.py --write or --emit-vectors; never typed",
        "scenarios": vectors,
    }
    return report, "\n".join(golden) + "\n", json.dumps(doc, indent=1) + "\n"


def scenario_block(sc: Scenario, run: Run, v: Verdict) -> list[str]:
    """One scenario's part of the golden file: its registration, transcript and checks."""
    out = ["", f"scenario {sc.name}: {sc.about}"]
    for s in sc.systems:
        binds = ",".join(f"{p}/{ph}" for p, ph in s.bindings)
        out.append(
            f"  system {s.sid} binds {binds} after {list(s.after)} observes {list(s.observes)}"
        )
    out += run.lines
    out.append(f"  outcome {run.outcome()} (expected {sc.expect})")
    out.append(f"  thread invariance: {v.orders} permuted orders, every transcript equal")
    if sc.expect_named is not None:
        out.append(
            f"  named at the end: {list(run.named_at_end)} (expected {list(sc.expect_named)})"
        )
    out += [f"  {n}" for n in v.notes]
    return out


def check_golden(golden: Path, text: str) -> int:
    """Compare a rendered file (the transcript or the vectors) with the committed one."""
    try:
        current = golden.read_text(encoding="utf-8")
    except OSError as e:
        print(f"STALE: cannot read {golden}: {e}", file=sys.stderr)
        return EXIT_STALE
    if current != text:
        print(f"STALE: {golden} differs from the reference", file=sys.stderr)
        return EXIT_STALE
    print(f"OK: {golden} matches the reference ({text.count(chr(10))} lines)")
    return EXIT_OK


def write(path: Path, text: str) -> None:
    """Write one generated file (never typed)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    print(f"wrote {path} ({text.count(chr(10))} lines)")


def main(argv: list[str] | None = None) -> int:
    """Run the self-test, then write or check the golden file (never with --control)."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    mode.add_argument("--write", action="store_true", help="rewrite both files (never typed)")
    mode.add_argument("--emit-vectors", action="store_true", help="rewrite the vectors file only")
    parser.add_argument(
        "--golden", type=Path, default=GOLDEN, help="the transcript to write or check"
    )
    parser.add_argument(
        "--vectors", type=Path, default=VECTORS, help="the vectors to write or check"
    )
    parser.add_argument("--control", choices=CONTROLS, help="a planted fault that must FAIL")
    args = parser.parse_args(argv)
    if args.control and (args.write or args.emit_vectors):
        parser.error("--control never writes the golden file or the vectors")
    report, text, vectors = self_test(args.control)
    print("\n".join(report))
    if report[-1].endswith("FAIL"):
        return EXIT_SELFTEST
    if args.control:
        print("CONTROL PASSED: the planted fault was not detected", file=sys.stderr)
        return EXIT_SELFTEST
    if args.check:
        return max(check_golden(args.golden, text), check_golden(args.vectors, vectors))
    if args.emit_vectors:
        write(args.vectors, vectors)
        return EXIT_OK
    if not args.write:
        print(
            "self-test only: the golden files are neither compared (--check) nor written (--write)"
        )
        return EXIT_OK
    write(args.golden, text)
    write(args.vectors, vectors)
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
