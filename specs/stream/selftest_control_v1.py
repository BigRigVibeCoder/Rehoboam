"""Reference of the World Stream control channel v1 (IF-0 ``control.sock``; SPR-027 T-01).

The contract is ``specs/stream/control_v1.md``: the pace command over ``control.sock``, carried by
``wire_v1.md``'s frames and canonical JSON-RPC. This file is the independent reference that
``rh-protocol``'s control dispatcher and ``rh-golden``'s pace governor (SPR-027 T-03) must agree
with, as ``selftest_wire_v1.py`` is for the stream sockets:

- :data:`METHODS` is the method table (exactly five), :data:`RESERVED` the P2 names refused by
  name (IF-3's spellings), :data:`ERRORS` the error model with this socket's codes;
- :class:`Engine` is a simulated engine: a deterministic record generator that knows nothing of
  the pace; :class:`Host` holds the governor's cell and a queue of validated commands, applies the
  queue at the engine's one decision point (before an instant), re-checks a pending step there,
  publishes the snapshot ``control.status`` reads, sleeps by control_v1.md §3.3's rule in integers
  over a :class:`FakeClock`, writes the run log and answers every command on its own reply slot;
- :func:`simulate` drives a host through a scripted command sequence over the fake clock, each
  instant taking wall time (its records appended, and the tip moved, at its end), so that commands
  land while an instant computes; the self-test asserts **pace invariance** (control_v1.md §0
  fact 2): every paced run produces the headless run's channel-A bytes, digests, keyframes, ladder
  and journal, while the governor really slept, a pause really held the tip, a step stopped after
  Bookkeeping of its target, a flood of no-change commands never starved the run, a command landing
  in a step's reaching instant won, and every answer reports the state the engine applied;
- the golden file ``specs/stream/golden/control_v1.txt`` holds one frame of every message, the
  host's answers to planted requests, each scenario's clock, answer and run-log lines, and the
  invariant digest every scenario reproduced.

Usage:  python -I specs/stream/selftest_control_v1.py          (self-test, then rewrite the golden)
        python -I specs/stream/selftest_control_v1.py --check    (self-test; exit 1 if the golden
                                                              differs)
        python -I specs/stream/selftest_control_v1.py --check --golden PATH  (check another file;
                                                              the positive control uses a stale one)
        python -I specs/stream/selftest_control_v1.py --control <name>   (a planted fault that must
                                                              FAIL for its own reason; the golden
                                                              file is never written)
Exit:   0 ok, 1 stale golden file, 2 the self-test failed.
Standard library only (GOV-003 §3); selftest_wire_v1.py is loaded by path for the frame codec,
the canonical JSON, the request and error builders and the stream's method table.
"""

from __future__ import annotations

import argparse
import dataclasses
import hashlib
import importlib.util
import itertools
import sys
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve().parent
GOLDEN = HERE / "golden" / "control_v1.txt"
WIRE_REF = HERE / "selftest_wire_v1.py"
EXIT_OK = 0
EXIT_STALE = 1
EXIT_SELFTEST = 2


def _load(path: Path) -> ModuleType:
    """Import a module by path (``python -I`` adds no directory to ``sys.path``)."""
    spec = importlib.util.spec_from_file_location(path.stem, path)
    if spec is None or spec.loader is None:
        raise ImportError(str(path))
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module
    spec.loader.exec_module(module)
    return module


W = _load(WIRE_REF)  # the carrier: frames, canonical JSON, builders, the stream's method table
Json = dict[str, object]
WireError: type[ValueError] = W.WireError

# ---------------------------------------------------------------------------------- the method set
CONTROL_FORMAT = "rehoboam/stream/control/v1"
CONTROL_VERSION = 1
METHODS = frozenset(
    {"control.status", "control.pace", "control.pause", "control.resume", "control.step"}
)
COMMANDS = METHODS - {"control.status"}
STREAM_METHODS = frozenset({"stream.status", "stream.subscribe", "stream.unsubscribe"})
CONTROL_MESSAGE = (
    "this socket carries the control channel; v1 accepts the pace command only (control.status, "
    "control.pace, control.pause, control.resume, control.step) and no input"
)
# control_v1.md §5.1: the P2 operations under IF-3's names (05.9 IF-3), refused by name with the
# detail a v1 host answers them with.
RESERVED: dict[str, str] = {
    "timeline.run": (
        "v1 spells timeline.run(band) as control.pace (W30); the IF-3 name arrives with rh-server "
        "in P2"
    ),
    "timeline.pause": (
        "v1 spells timeline.pause() as control.pause (W30); the IF-3 name arrives with rh-server "
        "in P2"
    ),
    "timeline.run_until": (
        "reserved for P2: run-until on a predicate needs the sifter (05.1 section 4.4); advancing "
        "to a tick is control.step"
    ),
    "timeline.seek": (
        "reserved for P2: seek is a keyframe plus replay, read-only (05.9 IF-3); in P1a rh-cli "
        "materialize rebuilds a past tick (SPR-027 T-02) and control.step advances the tip"
    ),
    "timeline.fork": (
        "reserved for P2.7 (D47): a fork is a new timeline with its own directory and sockets "
        "(ADR-0025 section 5); v1 has one timeline"
    ),
    "player.act": (
        "reserved for P2.7 (D47): a material input is journaled through IF-2 with its acceptance "
        "cursor (BCK-002 A-36) before it is applied; v1 accepts no input"
    ),
    "control.schedule": (
        "reserved for P2.7 (D47), a provisional name: a scheduled input is journaled when issued "
        "and applied at its tick (05.10 section 2); IF-3 has no name for it; v1 accepts no input"
    ),
    "sift.set_autopause": (
        "reserved for P2 (D-5): the Watch and Play auto-pause is the sifter's and the viewer's "
        "(05.4 section 2.3), not v1's"
    ),
}
RESERVED_NAMESPACES = frozenset(
    {
        "player",
        "oracle",
        "timeline",
        "console",
        "sift",
        "world",
        "chronicle",
        "entity",
        "foresight",
        "bubble",
        "map",
        "dialogue",
        "compile",
        "narrative",
    }
)
IF3_DETAIL = "reserved for IF-3 (05.9), not carried by control v1"
# control_v1.md §5: the wire's codes keep their meanings; this socket's own take -32120..-32129.
ERRORS: dict[str, tuple[int, str]] = {
    **W.ERRORS,
    "method_not_found": (-32601, CONTROL_MESSAGE),
    "too_many_controllers": (-32120, "the host's controller cap is reached"),
    "run_finished": (-32121, "the run has ended; the pace can no longer change"),
    "past_tick": (-32122, "the step's tick is at or below the tip"),
}
CONTROL_CODES = {-32120, -32121, -32122}
CONTROL_CODE_HI, CONTROL_CODE_LO = -32120, -32129  # this socket's range (control_v1.md §5)
# The wire's decoder classifies an error response by (code, reason); teach it this socket's codes.
W.CODE_REASON.update({code: reason for reason, (code, _) in ERRORS.items()})
CONTROLLER_CAP = 8
BANDS = ("live", "1h", "1d", "1w", "1mo", "1y", "10y", "max")
STATES = frozenset({"running", "paused", "finished"})
END_REASONS = frozenset({"finished", "aborted"})
NS_PER_S = 1_000_000_000
MINUTES_PER_HOUR = 60
MINUTES_PER_DAY = 1_440
MINUTES_PER_WEEK = 10_080
MONTHS_PER_YEAR = 12
DECADE = 10

# ------------------------------------------------------------------------------ the simulated world
MINUTES_PER_YEAR = 518_400  # a 360-day year (05.1 §4.1)
INSTANTS_PER_YEAR = 12  # one instant per month: the Month pulse
INSTANT_MINUTES = MINUTES_PER_YEAR // INSTANTS_PER_YEAR
FIRST_TICK = 0  # the first instant is at the epoch, tick 0 (rh-core engine.rs; C9)
HORIZON_YEARS = 4
KEYFRAME_YEARS = 2  # the simulation's cadence (the engine's is ten years, 05.10 §4)
LINES_PER_INSTANT = 3
INSTANT_WALL_NS = 10_000_000  # the simulated engine's compute per instant: 10 ms
TIMELINE = 0
HISTORY: Json = {
    "engine_build": "reference",
    "manifest_hash": "00" * 16,
    "registry_version": "0.0.0",
    "world_seed": 0,
}


def rate(band: str, minutes_per_year: int = MINUTES_PER_YEAR) -> int | None:
    """The band's rate in sim-minutes per wall-second (control_v1.md §3.1); None for ``max``."""
    table = {
        "live": 1,
        "1h": MINUTES_PER_HOUR,
        "1d": MINUTES_PER_DAY,
        "1w": MINUTES_PER_WEEK,
        "1mo": minutes_per_year // MONTHS_PER_YEAR,
        "1y": minutes_per_year,
        "10y": DECADE * minutes_per_year,
    }
    if band == "max":
        return None
    return table[band]


def ceil_div(a: int, b: int) -> int:
    """Ceiling of a / b for non-negative a and positive b."""
    return -(-a // b)


def deadline_ns(anchor_wall_ns: int, anchor_tick: int, tick: int, r: int | None) -> int:
    """The wall clock at or after which the instant at ``tick`` may run (control_v1.md §3.3).

    The rule in integers: ``(wall - anchor_wall) * r >= (tick - anchor_tick) * 1e9``. Python's
    integers stand for the host's i128 nanoseconds (C10).
    """
    if r is None:
        return anchor_wall_ns
    return anchor_wall_ns + ceil_div((tick - anchor_tick) * NS_PER_S, r)


def _h(*parts: object) -> str:
    """A 16-hex-digit hash of the parts (the simulated ``state_hash``)."""
    return hashlib.sha256(":".join(str(p) for p in parts).encode()).hexdigest()[:16]


@dataclass
class Engine:
    """A deterministic record generator that knows nothing of the pace.

    Instants fall at ``FIRST_TICK + k * INSTANT_MINUTES`` for ``k = 0..``; each emits lines
    derived from the tick alone, a ``STEP_DIGEST`` whose ``state_hash`` is a hash of the state so
    far at each year's end, a ``KEYFRAME`` every ``KEYFRAME_YEARS``, and a ladder entry per year.
    The journal has one row, written before the run, as P1a's inputs are.
    """

    horizon_years: int = HORIZON_YEARS
    lines: list[str] = field(default_factory=list)
    journal: list[str] = field(default_factory=lambda: ["resample person 4369 at 0"])
    keyframes: list[int] = field(default_factory=list)
    ladder: list[str] = field(default_factory=list)
    tip: int | None = None
    instants_run: int = 0
    state: str = _h("genesis")

    @property
    def finished(self) -> bool:
        """Whether every instant of the horizon has run."""
        return self.instants_run >= self.horizon_years * INSTANTS_PER_YEAR

    @property
    def next_tick(self) -> int | None:
        """The tick of the next instant (the agenda's next entry), or None when the run is over."""
        return None if self.finished else FIRST_TICK + self.instants_run * INSTANT_MINUTES

    @property
    def next_seq(self) -> int:
        """The seq the next record gets."""
        return len(self.lines)

    def journal_head(self) -> str:
        """The journal's chain head."""
        head = _h("journal")
        for row in self.journal:
            head = _h(head, row)
        return head

    def append(self, kind: str, tick: int, *fields: object) -> None:
        """Append one channel-A line in the canonical shape ``<KIND> <tl> <seq> <tick> …``."""
        rest = " ".join(str(f) for f in fields)
        self.lines.append(f"{kind} {TIMELINE} {self.next_seq} {tick} {rest}".rstrip())

    def run_instant(self, hint: str | None = None) -> int:
        """Run the next instant to the end of its Bookkeeping; return its tick.

        ``hint`` is None in every honest run: the engine has no argument for the pace. The
        planted control ``band-in-hash`` passes the band here and mixes it into the state.
        """
        tick = self.next_tick
        if tick is None:
            raise RuntimeError("the run is over")
        for i in range(LINES_PER_INSTANT):
            self.append("EVENT", tick, i, _h("event", tick, i)[:8])
            self.state = _h(self.state, tick, i)
        if hint is not None:
            self.state = _h(self.state, hint)  # band-in-hash: history would read the pace
        self.instants_run += 1
        self.tip = tick
        if self.instants_run % INSTANTS_PER_YEAR == 0:
            self._bookkeeping_year_end(tick)
        return tick

    def _bookkeeping_year_end(self, tick: int) -> None:
        self.append("STEP_DIGEST", tick, 1, 0, 0, self.state, "0" * 16, "0" * 16)
        year = self.instants_run // INSTANTS_PER_YEAR
        self.ladder.append(f"year {year} {self.state}")
        if year % KEYFRAME_YEARS == 0:
            self.keyframes.append(tick)
            self.append("KEYFRAME", tick, "kf/v1", f"kf/y{year:06d}.arrow", self.state, 1)

    def year_digests(self) -> list[str]:
        """``canonical_digest`` per sim-year of the lines (by tick)."""
        out = []
        for year in range(1, self.horizon_years + 1):
            lo, hi = (
                FIRST_TICK + (year - 1) * MINUTES_PER_YEAR,
                FIRST_TICK + year * MINUTES_PER_YEAR,
            )
            lines = [ln for ln in self.lines if lo <= int(ln.split()[3]) < hi]
            out.append(str(W.digest(lines)))
        return out


# ------------------------------------------------------------------------------ the governor's cell
@dataclass
class FakeClock:
    """A monotonic wall clock the governor sleeps on; ``sleep_until`` advances it (C10)."""

    now_ns: int = 0

    def sleep_until(self, t_ns: int) -> None:
        """Advance to ``t_ns`` if it is later than now."""
        self.now_ns = max(self.now_ns, t_ns)


@dataclass
class Cell:
    """The governor's cell as the engine applied it (control_v1.md §3.3).

    Only the engine writes it, at its decision point. The anchor is reset whenever the effective
    rate changes (K4): a band change, a run-state change, a step's start, end or clearing.
    """

    band: str = "max"
    state: str = "running"
    step_target: int | None = None
    anchor_tick: int = FIRST_TICK
    anchor_wall_ns: int = 0

    def effective(self) -> tuple[str, int | None]:
        """The state and the rate the governor sleeps by: a pending step runs at ``max``."""
        return self.state, (None if self.step_target is not None else rate(self.band))


@dataclass(frozen=True)
class Snapshot:
    """What the engine publishes at its decision point; what ``control.status`` reads (K6)."""

    band: str
    state: str
    step_target: int | None
    tip: int | None
    next_seq: int
    next_tick: int | None
    end: str | None

    def pace(self) -> Json:
        """The pace object (control_v1.md §4) of this snapshot."""
        step = None if self.step_target is None else {"until_tick": self.step_target}
        return {
            "band": self.band,
            "next_seq": self.next_seq,
            "state": self.state,
            "step": step,
            "tick": self.tip,
        }


class Refused(Exception):  # noqa: N818 - a refusal, not an error of the reference
    """A refused request: the reason and its extra ``data`` fields; nothing changes."""

    def __init__(self, reason: str, **data: object) -> None:
        """Record the reason and data."""
        super().__init__(reason)
        self.reason, self.data = reason, data


@dataclass
class Connection:
    """One controller's connection as the host sees it: its number (C11) and whether it closed."""

    number: int = 0
    closed: bool = False


@dataclass
class Pending:
    """A validated request on its own reply slot, waiting for the engine's decision point (K2)."""

    conn: Connection
    rid: int | str
    method: str
    until: int | str | None = None  # control.step's request, resolved at application
    band: str | None = None
    reply: Json | None = None  # the slot: set when the engine applies or refuses it
    applied_at_ns: int | None = None
    answered_seq: int | None = None  # the host's answer counter when the slot was filled (L1)


def error(rid: int | str | None, reason: str, **data: object) -> Json:
    """A JSON-RPC error response for ``reason`` (control_v1.md §5) with extra ``data`` fields."""
    code, message = ERRORS[reason]
    if reason == "method_not_found":
        data = {"socket": "control", **data}  # C13: every -32601 from this socket says so
    return {
        "error": {"code": code, "data": {"reason": reason, **data}, "message": message},
        "id": rid,
        "jsonrpc": W.JSONRPC,
    }


def band_param(params: object) -> str:
    """The band a ``control.pace`` asks for; refuses anything else (control_v1.md §4.2)."""
    if not isinstance(params, dict) or set(params) != {"band"}:
        raise WireError("invalid_params: params are exactly {band}")
    band = params["band"]
    if not isinstance(band, str) or band not in BANDS:
        raise WireError(f"invalid_params: band is one of {', '.join(BANDS)}")
    return band


def step_param(params: object) -> int | str:
    """The tick a ``control.step`` asks for, or ``"instant"`` (control_v1.md §4.5)."""
    if not isinstance(params, dict) or set(params) != {"until"}:
        raise WireError("invalid_params: params are exactly {until}")
    until = params["until"]
    if until == "instant":
        return "instant"
    if isinstance(until, dict) and set(until) == {"tick"}:
        tick = until["tick"]
        if (
            isinstance(tick, bool)
            or not isinstance(tick, int)
            or not W.I64_MIN <= tick <= W.I64_MAX
        ):
            raise WireError("invalid_params: until.tick is an i64")
        return tick
    raise WireError("invalid_params: until is {tick: i64} or 'instant'")


def no_params(params: object) -> None:
    """``control.status``, ``control.pause`` and ``control.resume`` take no parameter."""
    if params is not None and params != {}:
        raise WireError("invalid_params: this method takes no parameter")


@dataclass
class Host:
    """The control socket's server over a simulated engine and a fake clock.

    Serving threads validate a request and queue it (:meth:`submit`), each on its own reply slot;
    the engine's loop applies the queue at its one decision point, before an instant
    (:meth:`before_instant`), re-checks a pending step there and publishes the snapshot. A
    request that changes nothing against the published state while the queue is empty is answered
    at once (K3); every other request is answered by the engine when it applies it, whether it
    changed something, changed nothing or was refused. A connection's requests are served one at
    a time, in request order. ``control`` names a planted fault (control_v1.md §9); None is the
    honest host.
    """

    engine: Engine
    clock: FakeClock
    control: str | None = None
    cap: int = CONTROLLER_CAP
    run_nonce: int = 1
    start_paused: bool = False  # --start paused (C8)
    cell: Cell = field(default_factory=Cell)
    pending: list[Pending] = field(default_factory=list)
    run_log: list[str] = field(default_factory=list)
    connections: list[Connection] = field(default_factory=list)
    connections_opened: int = 0
    released: list[tuple[int, int, int]] = field(default_factory=list)  # (wall_ns, tick, k)
    end: str | None = None
    answers_given: int = 0
    published: Snapshot = field(init=False)

    def __post_init__(self) -> None:
        """Log the host's start and its start pace (control_v1.md §8.1); publish the snapshot."""
        if self.start_paused:
            self.cell.state = "paused"
        self._log_host("start")
        self._log_pace("host.start", None)
        self.publish()

    # -- the run log ------------------------------------------------------------------------
    def _tip_token(self) -> str:
        return "~" if self.engine.tip is None else str(self.engine.tip)

    def _log_pace(self, method: str, conn: Connection | None) -> None:
        c = self.cell
        who = f"conn={conn.number if conn else '-'} pid=-"
        line = f"pace {self.clock.now_ns} {c.band} {c.state} {self._tip_token()} {method}"
        self.run_log.append(f"{line} {self.run_nonce} {who}")

    def _log_host(self, event: str) -> None:
        self.run_log.append(
            f"host {self.clock.now_ns} {event} {self._tip_token()} {self.run_nonce}"
        )

    # -- the cell and the snapshot ----------------------------------------------------------
    def _anchor(self) -> None:
        tip = self.engine.tip
        self.cell.anchor_tick = FIRST_TICK if tip is None else tip
        self.cell.anchor_wall_ns = self.clock.now_ns

    def snapshot(self) -> Snapshot:
        """The cell and the engine's tip as they stand now."""
        c = self.cell
        return Snapshot(
            c.band,
            c.state,
            c.step_target,
            self.engine.tip,
            self.engine.next_seq,
            self.engine.next_tick,
            self.end,
        )

    def publish(self) -> None:
        """Publish the snapshot ``control.status`` reads (control_v1.md §3.3)."""
        self.published = self.snapshot()

    def _changes_published(self, method: str, band: str | None, until: int | str | None) -> bool:
        """Whether a command would change the published (applied) state (K3)."""
        p = self.published
        if method == "control.pace":
            return band != p.band
        if method == "control.pause":
            return p.state != "paused"
        if method == "control.resume":
            return p.state != "running" or p.step_target is not None
        return p.state != "running" or until == "instant" or p.step_target != until

    def _inflight(self, conn: Connection) -> bool:
        return any(p.conn is conn for p in self.pending)

    # -- requests: the serving thread's side ------------------------------------------------
    def submit(
        self,
        conn: Connection,
        rid: int | str,
        method: str,
        band: str | None = None,
        until: int | str | None = None,
    ) -> Pending:
        """Queue a validated request on its own slot, or answer it at once (K2, K3, L1, L3).

        Answered at once, from the published snapshot: a ``control.status`` with nothing of its
        own connection in flight (whatever other connections have queued; L3), a command after the
        run ended (``run_finished``), and a no-change command on a connection with nothing in
        flight while the queue is empty. Everything else waits for the engine's decision point,
        which is atomic (L1): a request counts as queued until its slot is filled.
        """
        p = Pending(conn, rid, method, until, band)
        if method == "control.status" and not self._inflight(conn):
            self._fill(p, W.response(rid, self.status()))  # waits only behind its own connection
        elif self.cell.state == "finished":
            self._fill(p, error(rid, "run_finished", end=self.end))
        elif self.pending or self._inflight(conn):
            self.pending.append(p)
        elif not self._changes_published(method, band, until):
            self._fill(p, W.response(rid, {**self.published.pace(), "changed": False}))
        else:
            self.pending.append(p)
        return p

    def _fill(self, p: Pending, reply: Json) -> None:
        """Fill a request's reply slot (the answer counter orders the answers; L1)."""
        self.answers_given += 1
        p.reply, p.answered_seq, p.applied_at_ns = reply, self.answers_given, self.clock.now_ns

    # -- requests: the engine's side --------------------------------------------------------
    def _apply_pending(self) -> list[Pending]:
        """Apply the queue in order; each command's answer is the cell right after it (L2).

        A queued ``control.status`` is returned, to be answered with the snapshot published at
        the end of the decision point.
        """
        deferred = []
        for p in self.pending:
            if p.method == "control.status":
                deferred.append(p)
                continue
            before = self.cell.effective()
            try:
                result = self._apply(p)
            except Refused as r:
                reply = error(p.rid, r.reason, **r.data)
            else:
                reply = W.response(p.rid, result)
            owed = self.control == "paused-time-owed" and p.method == "control.resume"
            if self.cell.effective() != before and not owed:
                self._anchor()  # the effective rate changed (K4); the control keeps a stale anchor
            self._fill(p, reply)
        self.pending.clear()
        return deferred

    def _apply(self, p: Pending) -> Json:
        if p.method == "control.pace":
            return self._pace(p)
        if p.method == "control.pause":
            return self._pause(p)
        if p.method == "control.resume":
            return self._resume(p)
        return self._step(p)

    def _pace(self, p: Pending) -> Json:
        band = str(p.band)
        changed = band != self.cell.band
        if changed:
            self.cell.band = band
            if self.control == "pace-in-journal":
                self.engine.journal.append(f"pace {band}")  # STR-5 broken: the pace as an input
            self._log_pace("control.pace", p.conn)
        return {**self.snapshot().pace(), "changed": changed}

    def _pause(self, p: Pending) -> Json:
        changed = self.cell.state == "running"
        if changed:
            self.cell.state, self.cell.step_target = "paused", None
            if self.control == "pause-writes-record":
                self.engine.append("PAUSE", self.engine.tip or FIRST_TICK)  # W35 broken
            self._log_pace("control.pause", p.conn)
        return {**self.snapshot().pace(), "changed": changed}

    def _resume(self, p: Pending) -> Json:
        changed = self.cell.state == "paused" or self.cell.step_target is not None
        if changed:
            self.cell.state, self.cell.step_target = "running", None
            self._log_pace("control.resume", p.conn)
        return {**self.snapshot().pace(), "changed": changed}

    def _step(self, p: Pending) -> Json:
        nxt = self.engine.next_tick
        if nxt is None:
            raise Refused("run_finished", end=self.end)
        target = nxt if p.until == "instant" else int(p.until or 0)
        tip = self.engine.tip
        if (tip is None and target < FIRST_TICK) or (tip is not None and target <= tip):
            raise Refused("past_tick", tip_tick=tip)  # C9: before the first instant, only below it
        changed = self.cell.step_target != target or self.cell.state != "running"
        if changed:
            self.cell.state, self.cell.step_target = "running", target
            self._log_pace(f"control.step:{target}", p.conn)
            if nxt > target:
                self._reach()  # already passed: reached inside its own application (L2)
        return {**self.snapshot().pace(), "changed": changed}

    def _reach(self) -> None:
        """The engine's own pause at a step's target (control_v1.md §3.3 item 1)."""
        self.cell.state, self.cell.step_target = "paused", None
        self._anchor()
        self._log_pace("step.reached", None)

    def _step_reached(self) -> bool:
        target, nxt = self.cell.step_target, self.engine.next_tick
        if target is None or self.cell.state != "running":
            return False
        if self.control == "step-overshoot":
            return self.engine.tip is not None and self.engine.tip > target  # W37 broken
        return nxt is None or nxt > target  # the last instant at or below the target ran

    # -- the engine's loop ------------------------------------------------------------------
    def before_instant(self) -> None:
        """The decision point (control_v1.md §3.3 item 1): apply, re-check a step, publish.

        Atomic (L1): nothing is submitted between taking the queue and publishing; a queued
        status is answered last, with the published snapshot (L2).
        """
        deferred = self._apply_pending()
        if self._step_reached():
            self._reach()
        self.publish()
        for p in deferred:
            self._fill(p, W.response(p.rid, self.status()))

    def deadline(self) -> int | None:
        """The wall clock at which the next instant may run; None when paused or finished."""
        state, r = self.cell.effective()
        tick = self.engine.next_tick
        if state != "running" or tick is None:
            return None
        return deadline_ns(self.cell.anchor_wall_ns, self.cell.anchor_tick, tick, r)

    def run_instant(self) -> int:
        """Item 4: the engine computes one instant; its records land, and the tip moves, at its end.

        The clock advances by the instant's compute first (commands landing meanwhile were queued
        by :func:`simulate` before this call), then the records are appended (K1).
        """
        k, tick = self.engine.instants_run + 1, self.engine.next_tick
        self.released.append((self.clock.now_ns, FIRST_TICK if tick is None else tick, k))
        self.clock.now_ns += INSTANT_WALL_NS
        hint = self.cell.band if self.control == "band-in-hash" else None
        return self.engine.run_instant(hint)

    def after_instant(self) -> None:
        """After the last instant the run finishes (C14, K5); otherwise item 1 follows."""
        if self.engine.finished:
            self.finish()

    def finish(self, reason: str = "finished") -> None:
        """The run ended, in one critical section (K5).

        The queue is drained with ``run_finished``, the cell says ``finished``, a pending step is
        dropped with no ``step.reached``, and every later request is refused at submission.
        """
        self.cell.state, self.cell.step_target, self.end = "finished", None, reason
        self._log_host("finish" if reason == "finished" else "abort")
        self.publish()
        for p in self.pending:
            if p.method == "control.status":
                self._fill(p, W.response(p.rid, self.status()))
            else:
                self._fill(p, error(p.rid, "run_finished", end=reason))
        self.pending.clear()

    # -- the socket -------------------------------------------------------------------------
    def connect(self) -> Connection:
        """Accept a connection, or refuse it at the cap."""
        self.connections_opened += 1
        conn = Connection(self.connections_opened, closed=len(self.connections) >= self.cap)
        if not conn.closed:
            self.connections.append(conn)
        return conn

    def close(self, conn: Connection) -> None:
        """The controller closed; nothing else changes (a pause outlives its connection, §7)."""
        conn.closed = True
        if conn in self.connections:
            self.connections.remove(conn)

    def status(self) -> Json:
        """The ``control.status`` result (control_v1.md §4.1) from the published snapshot (K6, L3).

        Always the published snapshot, whatever other connections have queued; a status queued
        behind its own connection's command is answered after the publish that applied it.
        """
        snap = self.published
        return {
            "calendar": {"minutes_per_year": MINUTES_PER_YEAR},
            "control": {"format": CONTROL_FORMAT, "version": CONTROL_VERSION},
            "controllers": len(self.connections),
            "end": None if snap.end is None else {"reason": snap.end},
            "history": dict(HISTORY),
            "pace": snap.pace(),
            "run_nonce": self.run_nonce,
            "timeline_id": TIMELINE,
            "tip": {"next_seq": snap.next_seq, "tick": snap.tip},
        }

    def dispatch(self, conn: Connection, data: bytes) -> list[bytes]:
        """Answer one frame from a controller on an idle host (between instants).

        The header is judged first; a request is queued and, the engine being at its decision
        point (its governor wait is interruptible), applied at once; the reply reports it.
        """
        try:
            ftype, _ = W.decode_header(data, W.VIEWER_MAX_PAYLOAD)
            if ftype == W.TYPE_RECORDS:
                raise WireError("a RECORDS frame on the control socket")
            frame, _ = W.decode_frame(data, W.VIEWER_MAX_PAYLOAD)
        except W.IncompleteFrameError:
            return []
        except WireError as e:
            conn.closed = True  # a header or frame error: id null, the connection closes (C16)
            reason = "unsupported_version" if "unsupported_version" in str(e) else "invalid_frame"
            extra: Json = (
                {"supported": [W.VERSION]}
                if reason == "unsupported_version"
                else {"detail": str(e)}
            )
            return [bytes(W.encode_frame(W.control_frame(error(None, reason, **extra))))]
        outcome = self._answer(conn, frame.payload)
        if isinstance(outcome, Pending):
            if outcome.reply is None:
                self.before_instant()  # the idle engine applies at once
            reply = outcome.reply
            assert reply is not None
            outcome = reply
        if "error" in outcome and outcome.get("id") is None:
            conn.closed = True  # an answer with id null closes the connection (C16 as corrected)
        return [bytes(W.encode_frame(W.control_frame(outcome)))]

    def _answer(self, conn: Connection, payload: bytes) -> Json | Pending:
        try:
            kind, obj = W.decode_control(payload)
        except WireError as e:
            text = str(e)  # answered with id null, then the connection closes (C16)
            reason = "parse_error" if text.startswith("parse_error") else "invalid_request"
            if text.startswith("invalid_params") or "a float" in text:
                reason = "invalid_params"
            return error(None, reason, detail=text)
        if kind != "request":
            return error(None, "invalid_request", detail=f"a {kind} from a controller")
        rid, method, params = obj["id"], str(obj["method"]), obj.get("params")
        assert isinstance(rid, (int, str))
        if method in METHODS:
            return self._route(conn, rid, method, params)
        return self._refuse_method(rid, method, params)

    def _refuse_method(self, rid: int | str, method: str, params: object) -> Json:
        if self.control == "input-accepted" and method == "player.act":
            self.engine.journal.append(f"input {params!r}")  # §5.1 broken: an input applied
            accepted: Json = W.response(rid, {"accepted": True})
            return accepted
        if method in RESERVED:
            return error(rid, "method_not_found", detail=RESERVED[method])
        if method.split(".", 1)[0] in RESERVED_NAMESPACES:
            return error(rid, "method_not_found", detail=IF3_DETAIL)
        return error(rid, "method_not_found")

    def _route(
        self, conn: Connection, rid: int | str, method: str, params: object
    ) -> Json | Pending:
        try:
            if method == "control.pace":
                return self.submit(conn, rid, method, band=band_param(params))
            if method == "control.step":
                return self.submit(conn, rid, method, until=step_param(params))
            no_params(params)
            return self.submit(conn, rid, method)
        except WireError as e:
            return error(rid, "invalid_params", detail=str(e))


# ---------------------------------------------------------------------------------------- scenarios
@dataclass(frozen=True)
class Command:
    """A scripted request: issued on ``conn`` when the fake clock reaches ``at_ns``."""

    at_ns: int
    conn: str
    method: str
    params: Json | None = None
    rid: int | None = None  # an explicit request id; else the issue index


@dataclass(frozen=True)
class Scenario:
    """A command script over the simulated run."""

    name: str
    commands: tuple[Command, ...] = ()
    start_paused: bool = False


@dataclass
class Run:
    """What a simulated run produced: the host, and each command's slot in issue order."""

    host: Host
    issued: list[tuple[Command, Pending | Json]] = field(default_factory=list)

    @property
    def replies(self) -> list[tuple[Command, Json]]:
        """Each command and its answer (every slot is filled once the run is over)."""
        out = []
        for cmd, outcome in self.issued:
            reply = outcome.reply if isinstance(outcome, Pending) else outcome
            assert reply is not None, cmd
            out.append((cmd, reply))
        return out

    def applied_at(self, cmd: Command) -> int:
        """When the engine applied ``cmd`` (fake clock), or its issue time if answered at once."""
        for c, outcome in self.issued:
            if c is cmd:
                at = outcome.applied_at_ns if isinstance(outcome, Pending) else None
                return cmd.at_ns if at is None else at
        raise KeyError(cmd)


S = NS_PER_S
MS = 1_000_000
W_NS = INSTANT_WALL_NS
YEAR1 = FIRST_TICK + MINUTES_PER_YEAR
STEP_TICK = FIRST_TICK + 15 * INSTANT_MINUTES  # instant 15, mid-year 2
STEP_TICK_OFF = STEP_TICK + 2_000  # between instants 15 and 16: stops after 15 (W37)
STEP_TICK_LATER = STEP_TICK + 3 * INSTANT_MINUTES  # instant 18
BEYOND = HORIZON_YEARS * MINUTES_PER_YEAR + 1  # a tick past the horizon


def _tick(k: int) -> int:
    return FIRST_TICK + k * INSTANT_MINUTES


def _every(
    method: str, params: Json | None, every_ns: int, until_ns: int, start: int
) -> list[Command]:
    return [Command(t, "a", method, params) for t in range(start, until_ns + 1, every_ns)]


SCENARIOS: tuple[Scenario, ...] = (
    Scenario("headless-max"),
    Scenario("band-1y", (Command(0, "a", "control.pace", {"band": "1y"}),)),
    Scenario("band-10y", (Command(0, "a", "control.pace", {"band": "10y"}),)),
    Scenario(
        "pause-resume",
        (
            Command(0, "a", "control.pace", {"band": "1y"}),
            Command(S + 50 * MS, "a", "control.pause"),
            Command(S + 60 * MS, "a", "control.status"),
            Command(3 * S + 50 * MS, "a", "control.status"),
            Command(3 * S + 50 * MS, "a", "control.resume"),
        ),
    ),
    Scenario(
        "step",
        (
            Command(0, "a", "control.pause"),
            Command(100 * MS, "a", "control.step", {"until": {"tick": STEP_TICK_OFF}}),
            Command(10 * S, "a", "control.status"),
            Command(10 * S + S // 2, "a", "control.pace", {"band": "1y"}),
            Command(11 * S, "a", "control.step", {"until": "instant"}),
            Command(12 * S, "a", "control.step", {"until": {"tick": STEP_TICK_LATER}}),
            Command(13 * S, "a", "control.resume"),
        ),
    ),
    Scenario(
        "two-controllers",
        (
            Command(0, "a", "control.pace", {"band": "1y"}),
            Command(S // 2, "b", "control.pace", {"band": "10y"}),
            Command(S // 2 + 200 * MS, "a", "control.pause"),
            Command(S // 2 + 400 * MS, "b", "control.resume"),
            Command(S, "b", "control.step", {"until": "instant"}),
            Command(2 * S, "a", "control.pause"),
            Command(2 * S + 100 * MS, "a", "control.resume"),
            Command(2 * S + 100 * MS, "b", "control.pace", {"band": "max"}),
        ),
    ),
    Scenario(  # C3: a step contested by a second controller
        "step-contest",
        (
            Command(0, "a", "control.pace", {"band": "1y"}),
            Command(0, "a", "control.pause"),
            Command(10 * MS, "a", "control.step", {"until": {"tick": _tick(40)}}),
            Command(55 * MS, "b", "control.pause"),
            Command(100 * MS, "b", "control.resume"),
            Command(200 * MS, "b", "control.step", {"until": {"tick": _tick(20)}}),
            Command(S, "b", "control.step", {"until": {"tick": _tick(24)}}),
            Command(S + 5 * MS, "b", "control.step", {"until": {"tick": _tick(30)}}),
            Command(2 * S, "a", "control.resume"),
        ),
    ),
    Scenario(  # K1: commands landing while an instant computes, three of them in a step's
        # reaching instant (a resume, a pause, a no-change command), and one at a release time
        "mid-instant",
        (
            Command(5 * MS, "a", "control.pause"),  # in instant 0: applied after it
            Command(100 * MS, "a", "control.step", {"until": "instant"}),  # instant 1 at max
            Command(105 * MS, "a", "control.resume"),  # in the reaching instant: clears the step
            Command(150 * MS, "a", "control.status"),  # between instants, running at max
            Command(200 * MS, "a", "control.pause"),  # at the decision point's time: before 11
            Command(300 * MS, "a", "control.step", {"until": "instant"}),  # instant 12
            Command(305 * MS, "a", "control.pause"),  # in the reaching instant: wins
            Command(400 * MS, "a", "control.step", {"until": "instant"}),  # instant 13
            Command(405 * MS, "a", "control.pace", {"band": "max"}),  # no change, answered at once
            Command(405 * MS, "a", "control.status"),  # the published snapshot, mid-instant
            Command(500 * MS, "a", "control.resume"),
        ),
    ),
    Scenario(  # C2: a flood of no-change commands at 1y must not starve the run
        "starve-1y",
        (
            Command(0, "a", "control.pace", {"band": "1y"}),
            *_every("control.resume", None, 50 * MS, 3 * S, 50 * MS),
            *_every("control.pace", {"band": "1y"}, 50 * MS, 3 * S, 25 * MS),
            *_every("control.status", None, 50 * MS, 3 * S, 10 * MS),
        ),
    ),
    Scenario(  # C2: a resume every 0.9 s at 1mo (one instant per second)
        "starve-1mo",
        (
            Command(0, "a", "control.pace", {"band": "1mo"}),
            *_every("control.resume", None, 900 * MS, 46 * S, 900 * MS),
        ),
    ),
    Scenario(  # C8: --start paused
        "start-paused",
        (Command(S // 2, "a", "control.resume"),),
        start_paused=True,
    ),
    Scenario(  # K4: a step started while running and cleared by a resume re-anchors the band
        "step-clear",
        (
            Command(0, "a", "control.pace", {"band": "1y"}),
            Command(200 * MS, "a", "control.step", {"until": {"tick": _tick(30)}}),
            Command(295 * MS, "a", "control.resume"),
        ),
    ),
    Scenario(  # K5: a step beyond the horizon, a pause landing in the last instant
        "finish-race",
        (
            Command(0, "a", "control.step", {"until": {"tick": BEYOND}}),
            Command(47 * W_NS + 5 * MS, "a", "control.pause"),
        ),
    ),
    Scenario(  # K3: the queue fold: P4, P5, P13, P16
        "queue-fold",
        (
            Command(5 * MS, "a", "control.pause"),  # in instant 0
            Command(6 * MS, "b", "control.pause"),  # queued behind a's: paused, changed false (P4)
            Command(100 * MS, "a", "control.step", {"until": {"tick": _tick(5)}}),
            Command(
                100 * MS, "b", "control.step", {"until": {"tick": _tick(5)}}
            ),  # same: false (P5)
            Command(300 * MS, "c", "control.resume"),
            Command(400 * MS, "c", "control.pause"),  # pipelined on one connection (P13)
            Command(400 * MS, "c", "control.pause"),  # answered after the first, changed false
            Command(500 * MS, "a", "control.step", {"until": "instant"}),
            Command(500 * MS, "b", "control.step", {"until": "instant"}),  # the target shown (P16)
            Command(600 * MS, "a", "control.resume"),
            Command(655 * MS, "a", "control.pause"),  # opposite commands in one instant (L1):
            Command(656 * MS, "b", "control.resume"),  # b is answered running, changed true
            Command(700 * MS, "c", "control.pause", rid=50),  # a pipelined (pause, status) pair,
            Command(700 * MS, "c", "control.status", rid=7),  # ids unsorted: answered in order
            Command(800 * MS, "a", "control.step", {"until": {"tick": _tick(26) + 2_000}}),
            Command(800 * MS, "a", "control.status"),  # queued in the item 1 that reached it (L2)
            Command(900 * MS, "a", "control.resume"),
            Command(915 * MS, "b", "control.pause"),  # queued while instant 28 computes
            Command(
                920 * MS, "c", "control.status"
            ),  # at the instant's end: the published tip (L3)
            Command(S, "a", "control.resume"),
        ),
    ),
)


def _issue(run: Run, conns: dict[str, Connection], cmd: Command) -> None:
    """A controller sends ``cmd``: validated now; applied at the engine's decision point."""
    host = run.host
    if cmd.conn not in conns:
        conns[cmd.conn] = host.connect()
    rid = len(run.issued) + 1 if cmd.rid is None else cmd.rid
    data = W.encode_frame(W.control_frame(W.request(rid, cmd.method, cmd.params)))
    frame, _ = W.decode_frame(data)
    outcome = host._answer(conns[cmd.conn], frame.payload)
    run.issued.append((cmd, outcome))


def simulate(sc: Scenario, control: str | None = None) -> Run:
    """Drive a host through ``sc`` over the fake clock; the engine runs to its horizon.

    The delivery rule (control_v1.md §9): at a decision point at time ``now``, every command with
    ``at_ns <= now`` is delivered and applied before the instant; a governor wait is cut by a
    command with ``at_ns < deadline``, which is then delivered at a new decision point; a command
    with ``release <= at_ns < release + INSTANT_WALL_NS`` lands while the instant computes (a
    command at exactly the release time the engine waited for lands during the instant), one with
    ``at_ns == release + INSTANT_WALL_NS`` lands after the instant's records and before the
    decision point (a status then still reads the snapshot published before the instant; L3),
    and both are applied at the decision point after the instant.
    """
    host = Host(Engine(), FakeClock(), control, start_paused=sc.start_paused)
    run, conns = Run(host), dict[str, Connection]()
    queue = sorted(sc.commands, key=lambda c: c.at_ns)
    while not host.engine.finished:
        while queue and queue[0].at_ns <= host.clock.now_ns:
            _issue(run, conns, queue.pop(0))
        host.before_instant()
        deadline = host.deadline()
        if deadline is None:  # paused: the host waits for the next command
            if not queue:
                raise RuntimeError(f"{sc.name}: paused with no command left")
            host.clock.sleep_until(queue[0].at_ns)
            continue
        if queue and queue[0].at_ns < deadline:  # a command lands during the wait: it ends it
            host.clock.sleep_until(queue[0].at_ns)
            continue
        host.clock.sleep_until(deadline)
        start = host.clock.now_ns
        while queue and queue[0].at_ns < start + INSTANT_WALL_NS:  # lands while it computes
            _issue(run, conns, queue.pop(0))
        host.run_instant()
        while queue and queue[0].at_ns == host.clock.now_ns:  # after the records, before item 1
            _issue(run, conns, queue.pop(0))
        host.after_instant()
    while queue:  # after the end: run_finished at submission
        _issue(run, conns, queue.pop(0))
    return run


# ------------------------------------------------------------------------------------------ checks
Check = tuple[str, Callable[[], list[str]]]


def _reason(reply: Json) -> str:
    err = reply.get("error")
    if isinstance(err, dict):
        data = err["data"]
        assert isinstance(data, dict)
        return str(data["reason"])
    return "ok"


def _detail(reply: Json) -> str:
    err = reply.get("error")
    if isinstance(err, dict):
        data = err["data"]
        assert isinstance(data, dict)
        return str(data.get("detail", ""))
    return ""


def _result(reply: Json) -> Json:
    res = reply.get("result")
    if not isinstance(res, dict):
        raise AssertionError(f"not a result: {reply!r}")
    return res


def _pace_of(reply: Json) -> Json:
    """The pace object of an answer: a command's result, or a status result's ``pace``."""
    res = _result(reply)
    pace = res.get("pace", res)
    assert isinstance(pace, dict)
    return pace


def _fresh_host(
    years: int = 1,
    finished: bool = False,
    control: str | None = None,
    with_step: bool = False,
) -> Host:
    """A host whose engine ran ``years`` sim-years at ``max`` from a clock at 0, idle after."""
    host = Host(Engine(), FakeClock(), control)
    for _ in range(years * INSTANTS_PER_YEAR):
        host.before_instant()
        host.run_instant()
        host.after_instant()
    if finished:
        for _ in range((HORIZON_YEARS - years) * INSTANTS_PER_YEAR):
            host.before_instant()
            host.run_instant()
            host.after_instant()
    if with_step:
        _ask(host, 100, "control.step", {"until": {"tick": _tick(HORIZON_YEARS * 12 - 1)}})
    host.before_instant()
    return host


def _ask(host: Host, rid: int, method: str, params: Json | None = None) -> Json:
    data = W.encode_frame(W.control_frame(W.request(rid, method, params)))
    frame, _ = W.decode_frame(b"".join(host.dispatch(host.connect(), data)))
    _, obj = W.decode_control(frame.payload)
    assert isinstance(obj, dict)
    return obj


def _table_checks() -> list[str]:
    problems = []
    five = {"control.status", "control.pace", "control.pause", "control.resume", "control.step"}
    if five != METHODS:
        problems.append("the control method table is not exactly the five methods of §4")
    stream: frozenset[str] = W.METHODS
    if stream != STREAM_METHODS:
        if "control.pace" in stream:
            problems.append("the stream method table carries control.pace")
        else:
            problems.append(f"the stream method table is {sorted(stream)}")
    if METHODS & stream:
        problems.append("the two method tables are not disjoint")
    if RESERVED.keys() & METHODS:
        problems.append("a reserved name is in the method table")
    return problems


# control_v1.md §5.1, typed out independently of RESERVED (C3): the check compares the two.
RESERVED_TABLE: dict[str, str] = {
    "timeline.run": "v1 spells timeline.run(band) as control.pace (W30)",
    "timeline.pause": "v1 spells timeline.pause() as control.pause (W30)",
    "timeline.run_until": "reserved for P2: run-until on a predicate needs the sifter",
    "timeline.seek": "reserved for P2: seek is a keyframe plus replay, read-only (05.9 IF-3)",
    "timeline.fork": "reserved for P2.7 (D47): a fork is a new timeline",
    "player.act": "reserved for P2.7 (D47): a material input is journaled through IF-2",
    "control.schedule": "reserved for P2.7 (D47), a provisional name: a scheduled input",
    "sift.set_autopause": "reserved for P2 (D-5): the Watch and Play auto-pause",
}


def _reserved_checks(host: Host) -> list[str]:
    """Every §5.1 name is refused with its detail; the table and RESERVED agree."""
    problems = []
    if set(RESERVED_TABLE) != set(RESERVED):
        problems.append("the reserved set differs from §5.1's table")
    for name, prefix in RESERVED_TABLE.items():
        reply = _ask(host, 1, name, {"x": 1})
        if _reason(reply) != "method_not_found":
            problems.append(f"an input was accepted on control.sock: {name} -> {_reason(reply)}")
        elif not _detail(reply).startswith(prefix):
            problems.append(f"{name}: detail {_detail(reply)!r} does not open with §5.1's")
    return problems


def _method_table_checks(control: str | None = None) -> list[str]:
    """The method table is exactly §4's, disjoint from the stream's; reserved names refused."""
    problems = _table_checks()
    host = _fresh_host(control=control)
    problems += _reserved_checks(host)
    for name in ("oracle.whisper", "console.set_mode", "world.create"):
        if _detail(_ask(host, 2, name)) != IF3_DETAIL:
            problems.append(f"{name}: the IF-3 detail is missing")
    for name in ("stream.subscribe", "x.y", "control.seek", "control.input"):
        reply = _ask(host, 3, name)
        if _reason(reply) != "method_not_found" or _detail(reply):
            problems.append(f"{name}: {_reason(reply)} with detail {_detail(reply)!r}")
    err = _ask(host, 4, "x.y")["error"]
    assert isinstance(err, dict)
    data = err["data"]
    if not isinstance(data, dict) or data.get("socket") != "control":
        problems.append("method_not_found does not carry data.socket = control (C13)")
    if host.engine.journal != Engine().journal:
        problems.append("an input was accepted on control.sock: the journal changed")
    return problems


def _messages() -> list[tuple[str, Json]]:
    """One object of every request, result and error (the golden ``frame`` lines)."""
    host = _fresh_host()
    host.connect()  # a status is answered on a connection, so controllers >= 1 (C22)
    status = host.status()
    pace = {**host.published.pace(), "changed": True}
    return [
        ("status_request", W.request(1, "control.status")),
        ("status_result", W.response(1, status)),
        ("pace_request", W.request(2, "control.pace", {"band": "1y"})),
        ("pace_result", W.response(2, {**pace, "band": "1y"})),
        ("pause_request", W.request(3, "control.pause")),
        ("pause_result", W.response(3, {**pace, "state": "paused"})),
        ("resume_request", W.request(4, "control.resume")),
        ("resume_result", W.response(4, pace)),
        ("step_request_tick", W.request(5, "control.step", {"until": {"tick": STEP_TICK}})),
        ("step_request_instant", W.request(6, "control.step", {"until": "instant"})),
        ("step_result", W.response(5, {**pace, "step": {"until_tick": STEP_TICK}})),
        ("error_method_not_found", error(7, "method_not_found")),
        ("error_reserved", error(8, "method_not_found", detail=RESERVED["player.act"])),
        (
            "error_invalid_params",
            error(9, "invalid_params", detail=f"invalid_params: band is one of {', '.join(BANDS)}"),
        ),
        ("error_too_many_controllers", error(None, "too_many_controllers", limit=CONTROLLER_CAP)),
        ("error_run_finished", error(10, "run_finished", end="finished")),
        ("error_past_tick", error(11, "past_tick", tip_tick=YEAR1 - INSTANT_MINUTES)),
        (
            "error_invalid_frame",
            error(None, "invalid_frame", detail="a RECORDS frame on the control socket"),
        ),
    ]


def _message_checks() -> list[str]:
    """Every message encodes canonically, decodes and classifies; the codes are in range."""
    problems = []
    for name, obj in _messages():
        data = W.encode_frame(W.control_frame(obj))
        frame, used = W.decode_frame(data)
        if used != len(data):
            problems.append(f"{name}: trailing bytes")
        kind, back = W.decode_control(frame.payload)
        if back != obj:
            problems.append(f"{name}: does not round-trip")
        expected = "error" if "error" in obj else ("request" if "method" in obj else "response")
        if kind != expected:
            problems.append(f"{name}: classified {kind}, expected {expected}")
    for reason in ("too_many_controllers", "run_finished", "past_tick"):
        code = ERRORS[reason][0]
        if not CONTROL_CODE_LO <= code <= CONTROL_CODE_HI or code not in CONTROL_CODES:
            problems.append(f"{reason}: code {code} outside {CONTROL_CODE_LO}..{CONTROL_CODE_HI}")
    if ERRORS["method_not_found"][1] != CONTROL_MESSAGE:
        problems.append("method_not_found does not carry the control socket's message")
    return problems


@dataclass(frozen=True)
class DispatchCase:
    """A planted frame from a controller and the host it is sent to."""

    name: str
    data: bytes
    finished: bool = False
    with_step: bool = False
    years: int = 1


def _req(rid: int, method: str, params: Json | None = None) -> bytes:
    return bytes(W.encode_frame(W.control_frame(W.request(rid, method, params))))


def _dispatch_cases() -> list[DispatchCase]:
    """Planted frames from a controller (golden ``dispatch`` lines)."""
    status = _req(1, "control.status")
    float_band = b'{"id":5,"jsonrpc":"2.0","method":"control.pace","params":{"band":1.5}}'
    records = W.records_payload(TIMELINE, "A", 0, 1, b"EVENT 0 0 0 x\n")  # a literal line (C23)
    oversize = status[:8] + (W.VIEWER_MAX_PAYLOAD + 1).to_bytes(4, "little")
    cases = [
        DispatchCase("dispatch_status", status),
        DispatchCase("dispatch_pace", _req(2, "control.pace", {"band": "1y"})),
        DispatchCase("dispatch_pace_same", _req(3, "control.pace", {"band": "max"})),
        DispatchCase("dispatch_pace_unknown_band", _req(4, "control.pace", {"band": "1yr"})),
        DispatchCase(
            "dispatch_pace_float", bytes(W.encode_frame(W.Frame(W.TYPE_CONTROL, float_band)))
        ),
        DispatchCase("dispatch_pace_extra_member", _req(6, "control.pace", {"band": "1y", "x": 1})),
        DispatchCase("dispatch_pause", _req(7, "control.pause")),
        DispatchCase("dispatch_pause_with_step", _req(7, "control.pause"), with_step=True),
        DispatchCase("dispatch_resume", _req(8, "control.resume")),
        DispatchCase("dispatch_resume_with_step", _req(8, "control.resume"), with_step=True),
        DispatchCase("dispatch_step_tick", _req(9, "control.step", {"until": {"tick": STEP_TICK}})),
        DispatchCase("dispatch_step_instant", _req(10, "control.step", {"until": "instant"})),
        DispatchCase(
            "dispatch_step_replace",
            _req(9, "control.step", {"until": {"tick": STEP_TICK}}),
            with_step=True,
        ),
        DispatchCase(
            "dispatch_step_past",
            _req(11, "control.step", {"until": {"tick": YEAR1 - INSTANT_MINUTES}}),
        ),
        DispatchCase(
            "dispatch_step_genesis",
            _req(11, "control.step", {"until": {"tick": FIRST_TICK}}),
            years=0,
        ),
        DispatchCase(
            "dispatch_step_before_first",
            _req(11, "control.step", {"until": {"tick": FIRST_TICK - 5}}),
            years=0,
        ),
        DispatchCase(
            "dispatch_step_reached_at_once",
            _req(23, "control.step", {"until": {"tick": YEAR1 - INSTANT_MINUTES + 2_000}}),
        ),
        DispatchCase("dispatch_step_bad", _req(12, "control.step", {"until": {"year": 2}})),
        DispatchCase("dispatch_status_with_param", _req(13, "control.status", {"x": 1})),
    ]
    cases += [
        DispatchCase(f"dispatch_reserved_{name.replace('.', '_')}", _req(14, name, {"x": 1}))
        for name in RESERVED
    ]
    cases += [
        DispatchCase("dispatch_oracle_whisper", _req(15, "oracle.whisper", {"text": "x"})),
        DispatchCase("dispatch_stream_subscribe", _req(16, "stream.subscribe", {"from": "live"})),
        DispatchCase("dispatch_unknown_method", _req(17, "control.seek", {"tick": 0})),
        DispatchCase(
            "dispatch_pace_finished", _req(18, "control.pace", {"band": "1y"}), finished=True
        ),
        DispatchCase("dispatch_pause_finished", _req(19, "control.pause"), finished=True),
        DispatchCase("dispatch_resume_finished", _req(20, "control.resume"), finished=True),
        DispatchCase(
            "dispatch_step_finished", _req(21, "control.step", {"until": "instant"}), finished=True
        ),
        DispatchCase("dispatch_status_finished", _req(22, "control.status"), finished=True),
        DispatchCase(
            "dispatch_notification",
            bytes(W.encode_frame(W.control_frame(W.notification("control.pace", {})))),
        ),
        DispatchCase("dispatch_batch", bytes(W.encode_frame(W.Frame(W.TYPE_CONTROL, b"[]")))),
        DispatchCase("dispatch_not_json", bytes(W.encode_frame(W.Frame(W.TYPE_CONTROL, b"{")))),
        DispatchCase("dispatch_version_2", status[:4] + b"\x02\x00" + status[6:]),
        DispatchCase(
            "dispatch_records_frame", bytes(W.encode_frame(W.Frame(W.TYPE_RECORDS, records)))
        ),
        DispatchCase("dispatch_oversize", oversize + b"\0" * (W.VIEWER_MAX_PAYLOAD + 1)),
    ]
    return cases


EXPECTED_REASONS = {
    "dispatch_status": "ok",
    "dispatch_pace": "ok",
    "dispatch_pace_same": "ok",
    "dispatch_pace_unknown_band": "invalid_params",
    "dispatch_pace_float": "invalid_params",
    "dispatch_pace_extra_member": "invalid_params",
    "dispatch_pause": "ok",
    "dispatch_pause_with_step": "ok",
    "dispatch_resume": "ok",
    "dispatch_resume_with_step": "ok",
    "dispatch_step_tick": "ok",
    "dispatch_step_instant": "ok",
    "dispatch_step_replace": "ok",
    "dispatch_step_past": "past_tick",
    "dispatch_step_genesis": "ok",
    "dispatch_step_before_first": "past_tick",
    "dispatch_step_reached_at_once": "ok",
    "dispatch_step_bad": "invalid_params",
    "dispatch_status_with_param": "invalid_params",
    "dispatch_oracle_whisper": "method_not_found",
    "dispatch_stream_subscribe": "method_not_found",
    "dispatch_unknown_method": "method_not_found",
    "dispatch_pace_finished": "run_finished",
    "dispatch_pause_finished": "run_finished",
    "dispatch_resume_finished": "run_finished",
    "dispatch_step_finished": "run_finished",
    "dispatch_status_finished": "ok",
    "dispatch_notification": "invalid_request",
    "dispatch_batch": "invalid_request",
    "dispatch_not_json": "parse_error",
    "dispatch_version_2": "unsupported_version",
    "dispatch_records_frame": "invalid_frame",
    "dispatch_oversize": "invalid_frame",
    **{f"dispatch_reserved_{name.replace('.', '_')}": "method_not_found" for name in RESERVED},
}
# Cases whose command changes the cell: exactly one run-log line each (two for a step reached at
# once); every other case writes none.
LOGGED = {
    "dispatch_pace": 1,
    "dispatch_pause": 1,
    "dispatch_pause_with_step": 1,
    "dispatch_resume_with_step": 1,
    "dispatch_step_tick": 1,
    "dispatch_step_instant": 1,
    "dispatch_step_replace": 1,
    "dispatch_step_genesis": 1,
    "dispatch_step_reached_at_once": 2,
}
# C16 as corrected: every answer with id null closes the connection; one with the request's id
# keeps it open (method_not_found, invalid_params on a request, past_tick, run_finished).
CLOSES = {
    "dispatch_version_2",
    "dispatch_records_frame",
    "dispatch_oversize",
    "dispatch_not_json",
    "dispatch_batch",
    "dispatch_notification",
    "dispatch_pace_float",
}


def dispatch_transcript() -> list[tuple[DispatchCase, bytes, Host, Cell, Connection]]:
    """Each planted request, the host's reply bytes, the host after, the cell before, the conn."""
    out = []
    for case in _dispatch_cases():
        host = _fresh_host(years=case.years, finished=case.finished, with_step=case.with_step)
        before = dataclasses.replace(host.cell)
        conn = host.connect()
        replies = host.dispatch(conn, case.data)
        out.append((case, b"".join(replies), host, before, conn))
    return out


def _log_lines_for_commands(host: Host, with_step: bool) -> int:
    """The run-log lines beyond the host's own (start, start pace, finish) and the setup step."""
    return len(host.run_log) - 2 - (1 if host.end else 0) - (1 if with_step else 0)


def _dispatch_checks() -> list[str]:
    """Every planted request gets its reason; a refusal changes nothing; a change logs once."""
    problems = []
    for case, reply, host, before, conn in dispatch_transcript():
        frame, used = W.decode_frame(reply)
        _, obj = W.decode_control(frame.payload)
        if used != len(reply):
            problems.append(f"{case.name}: more than one reply frame")
        if _reason(obj) != EXPECTED_REASONS[case.name]:
            problems.append(f"{case.name}: {_reason(obj)}, expected {EXPECTED_REASONS[case.name]}")
        lines = _log_lines_for_commands(host, case.with_step)
        if lines != LOGGED.get(case.name, 0):
            problems.append(
                f"{case.name}: {lines} run-log lines, expected {LOGGED.get(case.name, 0)}"
            )
        if _reason(obj) != "ok" and host.cell != before:
            problems.append(f"{case.name}: a refusal changed the cell")
        if conn.closed != (case.name in CLOSES):
            problems.append(f"{case.name}: connection closed = {conn.closed}")
        if host.engine.journal != Engine().journal:
            problems.append(f"{case.name}: the journal changed under a pace command")
        problems += _dispatch_result_checks(case.name, obj)
    return problems


def _dispatch_result_checks(name: str, obj: Json) -> list[str]:
    """What the answers must say: the state the engine applied (C1, C3's mutants)."""
    if _reason(obj) != "ok" or name.startswith("dispatch_status"):
        return []
    res = _result(obj)
    want: dict[str, tuple[object, object, object]] = {  # (state, step, changed)
        "dispatch_pace": ("running", None, True),
        "dispatch_pace_same": ("running", None, False),
        "dispatch_pause": ("paused", None, True),
        "dispatch_pause_with_step": ("paused", None, True),
        "dispatch_resume": ("running", None, False),
        "dispatch_resume_with_step": ("running", None, True),
        "dispatch_step_tick": ("running", {"until_tick": STEP_TICK}, True),
        "dispatch_step_instant": ("running", {"until_tick": YEAR1}, True),
        "dispatch_step_replace": ("running", {"until_tick": STEP_TICK}, True),
        "dispatch_step_genesis": ("running", {"until_tick": FIRST_TICK}, True),
        "dispatch_step_reached_at_once": ("paused", None, True),
    }
    state, step, changed = want[name]
    got = (res["state"], res["step"], res["changed"])
    if got != (state, step, changed):
        return [f"{name}: answered {got}, expected {(state, step, changed)}"]
    return []


def _host_checks() -> list[str]:
    """The cap, the status invariants, idempotence, and a step past the horizon."""
    problems = []
    host = _fresh_host()
    conns = [host.connect() for _ in range(CONTROLLER_CAP + 1)]
    if not conns[-1].closed or conns[-2].closed:
        problems.append("the controller cap is not exactly 8")
    st = host.status()
    pace, tip = st["pace"], st["tip"]
    assert isinstance(pace, dict) and isinstance(tip, dict)
    if pace["tick"] != tip["tick"] or st["controllers"] != CONTROLLER_CAP:
        problems.append("status: pace.tick = tip.tick and controllers = open connections")
    if (st["end"] is None) != (pace["state"] != "finished"):
        problems.append("status: end is null iff not finished")
    first = _result(_ask(host, 1, "control.pause"))["changed"]
    second = _result(_ask(host, 2, "control.pause"))["changed"]
    if first is not True or second is not False:
        problems.append("pause is not idempotent with changed")
    res = _result(_ask(host, 3, "control.step", {"until": {"tick": BEYOND}}))
    if res["state"] != "running" or res["step"] != {"until_tick": BEYOND}:
        problems.append("a step beyond the horizon runs to the end")
    anchor = (host.cell.anchor_tick, host.cell.anchor_wall_ns)
    _ask(host, 4, "control.step", {"until": {"tick": BEYOND}})
    if (host.cell.anchor_tick, host.cell.anchor_wall_ns) != anchor:
        problems.append("a no-change command re-anchored (C2)")
    return problems


def _governor_checks() -> list[str]:
    """The rate table and the deadline arithmetic (control_v1.md §3.1, §3.3), in integers."""
    problems = []
    expected: dict[str, int | None] = {
        "live": 1,
        "1h": 60,
        "1d": 1440,
        "1w": 10080,
        "1mo": 43200,
        "1y": 518400,
        "10y": 5184000,
        "max": None,
    }
    for band in BANDS:
        if rate(band) != expected[band]:
            problems.append(f"rate({band}) = {rate(band)}, expected {expected[band]}")
    if rate("1mo", 525_600) != 43_800:  # noqa: PLR2004 - a 365-day year's twelfth
        problems.append("1mo on a 365-day year is not a twelfth of 525600")
    one_y = rate("1y")
    if deadline_ns(5 * S, 0, MINUTES_PER_YEAR, one_y) != 6 * S:
        problems.append("a sim-year at 1y is not one wall-second")
    if deadline_ns(0, 0, INSTANT_MINUTES, one_y) != ceil_div(S, INSTANTS_PER_YEAR):
        problems.append("an instant at 1y is not a twelfth of a second, rounded up")
    if deadline_ns(7, 0, INSTANT_MINUTES, None) != 7:  # noqa: PLR2004
        problems.append("max sleeps")
    live = rate("live")
    if deadline_ns(0, YEAR1, YEAR1 + INSTANT_MINUTES, live) != INSTANT_MINUTES * S:
        problems.append("the anchor is not subtracted")
    far = deadline_ns(0, -(1 << 62), 1 << 62, live)  # i128 range: no overflow in the reference
    if far != (1 << 63) * S:
        problems.append("the deadline arithmetic overflows")
    return problems


RECORD_KINDS = frozenset({"EVENT", "STEP_DIGEST", "KEYFRAME"})
INSTANT_1 = 2  # `released` numbers instants from 1, so instant 1 (tick 43200) is k = 2


def _same_history(run: Run, ref: Engine, name: str) -> list[str]:
    e = run.host.engine
    problems = []
    if e.ladder != ref.ladder:
        problems.append(f"{name}: the state hash differs from the headless run ({e.ladder[-1]!r})")
    if e.lines != ref.lines:
        extra = [ln for ln in e.lines if ln.split()[0] not in RECORD_KINDS]
        what = f"a pause wrote a record: {extra[0]!r}" if extra else "the channel-A lines differ"
        problems.append(f"{name}: {what}")
    if e.year_digests() != ref.year_digests():
        problems.append(f"{name}: the per-year digests differ")
    if e.keyframes != ref.keyframes:
        problems.append(f"{name}: the keyframe set differs")
    if e.journal_head() != ref.journal_head() or e.journal != ref.journal:
        problems.append(f"{name}: the journal changed under a pace command")
    return problems


def _exact_releases(
    run: Run, band: str, anchor_wall: int, anchor_tick: int, since: int
) -> list[str]:
    """Every instant released at or after ``since`` ran exactly at the band's deadline."""
    r = rate(band)
    problems = []
    for w, tick, k in run.host.released:
        if w < since:
            continue
        want = deadline_ns(anchor_wall, anchor_tick, tick, r)
        if w != want:
            problems.append(f"instant {k} released at {w}, the band says {want}")
            break
    return problems


def _slept(run: Run, name: str) -> list[str]:
    """The governor really slept at a band, never at max, and no flood starved it (C2)."""
    problems = []
    rel = run.host.released
    if name == "headless-max":
        gaps = {b - a for (a, _, _), (b, _, _) in itertools.pairwise(rel)}
        if gaps != {INSTANT_WALL_NS}:
            problems.append("the clock moved between instants at max")
    if name in {"band-1y", "starve-1y"}:
        problems += [f"{name}: {p}" for p in _exact_releases(run, "1y", 0, FIRST_TICK, 0)]
        years = [w for w, _, k in rel if k % INSTANTS_PER_YEAR == 0]
        if any(b - a < S for a, b in itertools.pairwise(years)):
            problems.append("a sim-year at 1y took less than a wall-second")
    if name == "starve-1mo":
        problems += [f"{name}: {p}" for p in _exact_releases(run, "1mo", 0, FIRST_TICK, 0)]
    if name == "band-10y" and rel[-1][0] < (HORIZON_YEARS - 1) * S // DECADE:
        problems.append("the run at 10y was faster than the band")
    if name.startswith("starve") and len(rel) != HORIZON_YEARS * INSTANTS_PER_YEAR:
        problems.append(f"{name}: the flood starved the run")
    return problems


def _replies(run: Run, method: str, conn: str | None = None) -> list[tuple[Command, Json]]:
    return [(c, r) for c, r in run.replies if c.method == method and conn in (None, c.conn)]


def _resume_exact(run: Run, cmd: Command, reply: Json, band: str) -> list[str]:
    """After a resume, the next instants run exactly at the band's deadline from (tip, applied)."""
    tip = _pace_of(reply)["tick"]
    assert isinstance(tip, int)
    at = run.applied_at(cmd)
    return _exact_releases(run, band, at, tip, at)


def _held(run: Run) -> list[str]:
    """A pause held the tip and wrote nothing; the resume owed no time (control_v1.md §4.3)."""
    problems = []
    statuses = _replies(run, "control.status")
    pairs = zip(_replies(run, "control.pause"), _replies(run, "control.resume"), strict=False)
    for (p_cmd, p_reply), (r_cmd, r_reply) in pairs:
        p_at, r_at = run.applied_at(p_cmd), run.applied_at(r_cmd)
        if any(p_at < w < r_at for w, _, _ in run.host.released):
            problems.append("an instant ran while paused")
        seqs = {int(str(_pace_of(st)["next_seq"])) for c, st in statuses if p_at <= c.at_ns <= r_at}
        p_res, r_res = _result(p_reply), _result(r_reply)
        seqs |= {int(str(p_res["next_seq"])), int(str(r_res["next_seq"]))}
        if len(seqs) > 1:
            problems.append("a pause wrote a record: next_seq moved while paused")
        if p_res["tick"] != r_res["tick"]:
            problems.append("the tip moved while paused")
        exact = _resume_exact(run, r_cmd, r_reply, "1y")
        if exact:
            problems.append(f"paused time was owed: {exact[0]}")
    return problems


def _stepped(run: Run) -> list[str]:
    """Each step stopped after Bookkeeping of the last instant at or below its target (W37)."""
    problems = []
    reached = [
        (int(ln.split()[1]), ln.split()[4]) for ln in run.host.run_log if " step.reached " in ln
    ]
    for cmd, reply in _replies(run, "control.step"):
        if _reason(reply) != "ok":
            continue  # reported once by the invariance check
        res = _result(reply)
        step, tick = res["step"], res["tick"]
        target = step["until_tick"] if isinstance(step, dict) else tick
        assert isinstance(target, int)
        want = target - (target - FIRST_TICK) % INSTANT_MINUTES
        at = run.applied_at(cmd)
        hit = next((tip for w, tip in reached if w >= at), None)
        if hit is None or hit == "~":
            problems.append(f"the step to {target} never paused")
        elif int(hit) > want:
            problems.append(f"the step overshot its target: paused at {hit}, not {want}")
        elif int(hit) < want:
            problems.append(f"the step stopped short: paused at {hit}, not {want}")
    return problems


def _step_scenario_checks(run: Run) -> list[str]:
    """The step scenario's exact answers: the resume at 1y re-anchors at the tip."""
    problems = _stepped(run)
    cmd, reply = _replies(run, "control.resume")[-1]
    tip = _result(reply)["tick"]
    if tip != STEP_TICK_LATER:
        problems.append(f"the last resume answered tip {tip}, expected {STEP_TICK_LATER}")
    problems += _resume_exact(run, cmd, reply, "1y")
    return problems


def _contest_checks(run: Run) -> list[str]:
    """C3: a pause clears a step, a resume clears a step, a later step replaces the target."""
    problems = []
    pause_b = _result(_replies(run, "control.pause", "b")[0][1])
    if pause_b["step"] is not None or pause_b["state"] != "paused":
        problems.append("a pause left the step pending")
    steps = [_result(r) for _, r in _replies(run, "control.step")]
    if any(s["state"] != "running" for s in steps[:3]):
        problems.append("a step did not run")
    reached = [int(ln.split()[4]) for ln in run.host.run_log if " step.reached " in ln]
    if _tick(40) in reached:
        problems.append("the step cancelled by the pause still reached its target")
    if _tick(24) in reached or _tick(30) not in reached or _tick(20) not in reached:
        problems.append(f"the replaced step's targets are wrong: {reached}")
    return problems


def _mid_instant_checks(run: Run) -> list[str]:
    """K1: a command landing while an instant computes is applied after it and wins a step."""
    problems = []
    pauses = _replies(run, "control.pause")
    first = _result(pauses[0][1])
    if first["tick"] != FIRST_TICK or first["state"] != "paused":
        problems.append(f"a pause landing in instant 0 answered {first}")
    second = [w for w, _, k in run.host.released if k == INSTANT_1]
    if second and second[0] < 100 * MS:
        problems.append("instant 1 ran while paused")
    resume = _result(_replies(run, "control.resume")[0][1])
    if resume["state"] != "running" or resume["step"] is not None or resume["tick"] != _tick(1):
        problems.append(f"a resume landing in the reaching instant answered {resume}")
    tie = _result(pauses[1][1])
    if tie["tick"] != _tick(10) or tie["state"] != "paused":
        problems.append(f"a pause at exactly a decision point's time answered {tie}")
    win = _result(pauses[2][1])
    if win["tick"] != _tick(11) or win["state"] != "paused" or win["step"] is not None:
        problems.append(f"a pause landing in the reaching instant answered {win}")
    log = run.host.run_log
    reached = [int(ln.split()[4]) for ln in log if " step.reached " in ln]
    if reached != [_tick(12)]:
        problems.append(f"step.reached was written at {reached}, expected only {[_tick(12)]}")
    nochange = _result(_replies(run, "control.pace")[0][1])
    if nochange["changed"] is not False or nochange["tick"] != _tick(11):
        problems.append(f"a no-change command in the reaching instant answered {nochange}")
    status = _pace_of(_replies(run, "control.status")[1][1])
    if status["tick"] != _tick(11) or status["step"] != {"until_tick": _tick(12)}:
        problems.append(f"a status read mid-instant did not show the published snapshot: {status}")
    return problems


def _clear_checks(run: Run) -> list[str]:
    """K4: a resume that clears a step re-anchors the band at (tip, now)."""
    cmd, reply = _replies(run, "control.resume")[0]
    res = _result(reply)
    problems = []
    if res["step"] is not None or res["state"] != "running":
        problems.append(f"the clearing resume answered {res}")
    problems += _resume_exact(run, cmd, reply, "1y")
    at = run.applied_at(cmd)
    nxt = [w for w, _, _ in run.host.released if w > at]
    if nxt and nxt[0] - at != ceil_div(INSTANT_MINUTES * S, rate("1y") or 1):
        problems.append(f"the first instant after the clearing came {nxt[0] - at} ns later")
    return problems


def _finish_checks(run: Run) -> list[str]:
    """K5: the end is never held; a pause landing in the last instant is run_finished."""
    problems = []
    _, pause = _replies(run, "control.pause")[0]
    if _reason(pause) != "run_finished":
        problems.append(f"a pause landing in the last instant was answered {_reason(pause)}")
    log = run.host.run_log
    if any(" step.reached " in ln for ln in log):
        problems.append("the finish wrote step.reached")
    if any(" control.pause " in ln for ln in log):
        problems.append("the finish applied the queue")
    if log[-1].split()[2] != "finish":
        problems.append("the run log does not end with finish")
    return problems


def _fold_checks(run: Run) -> list[str]:
    """K3: P4, P5, P13, P16 against the applied state, in request order."""
    problems = []
    pauses = _replies(run, "control.pause")
    p4 = _result(pauses[1][1])
    if (p4["state"], p4["changed"]) != ("paused", False):
        problems.append(f"P4: b's pause behind a's answered {p4}")
    steps = _replies(run, "control.step")
    p5a, p5b = _result(steps[0][1]), _result(steps[1][1])
    if p5a["changed"] is not True or (p5b["changed"], p5b["step"]) != (
        False,
        {"until_tick": _tick(5)},
    ):
        problems.append(f"P5: identical steps answered {p5a} and {p5b}")
    c_pauses = [r for c, r in pauses if c.conn == "c"][:2]  # the pipelined pair at 400 ms
    p13 = [_result(r)["changed"] for r in c_pauses]
    if p13 != [True, False]:
        problems.append(f"P13: pipelined pauses answered changed {p13}")
    p16a, p16 = _result(steps[2][1]), _result(steps[3][1])
    if p16["step"] is None or p16["step"] != p16a["step"] or p16["changed"] is not False:
        problems.append(f"P16: b's step instant behind a's answered {p16}")
    return problems + _atomic_checks(run)


def _answer_order(run: Run, conn: str) -> list[int]:
    """The host's answer counter for each of ``conn``'s requests, in arrival order."""
    out = []
    for c, outcome in run.issued:
        if c.conn == conn:
            seq = outcome.answered_seq if isinstance(outcome, Pending) else None
            out.append(-1 if seq is None else seq)
    return out


def _atomic_checks(run: Run) -> list[str]:
    """L1 to L3: opposite commands, pipelined pairs, a step reached at once, a status at an end.

    Opposite commands landing in one instant, a pipelined pair answered in arrival order, a step
    reached inside its own application with a status behind it, and a status at an instant's end
    reading the published snapshot.
    """
    problems = []
    if any(_reason(r) != "ok" for _, r in run.replies):
        return ["a scripted command was refused, so the atomic cases were not checked"]
    order = _answer_order(run, "c")
    if -1 in order or order != sorted(order):
        problems.append(f"L1: c's answers were not given in arrival order: {order}")
    resume_b = _result(_replies(run, "control.resume", "b")[0][1])
    if (resume_b["state"], resume_b["changed"]) != ("running", True):
        problems.append(f"L1: b's resume opposite a's pause answered {resume_b}")
    if not any(w == 660 * MS for w, _, _ in run.host.released):
        problems.append("L1: the cell did not end running after the opposite commands")
    statuses = _replies(run, "control.status")
    pipelined = _pace_of(statuses[0][1])
    if pipelined["state"] != "paused":
        problems.append(f"L1: a status pipelined behind a pause answered {pipelined}")
    reached = _result(_replies(run, "control.step", "a")[-1][1])
    if (reached["state"], reached["step"], reached["changed"]) != ("paused", None, True):
        problems.append(f"L2: a step reached at once answered {reached}")
    queued_status = _pace_of(statuses[1][1])
    if (queued_status["state"], queued_status["step"]) != ("paused", None):
        problems.append(f"L2: a status queued in the reaching item 1 answered {queued_status}")
    by_release = {w: tick for w, tick, _ in run.host.released}
    end_status = _pace_of(statuses[2][1])
    if end_status["tick"] != by_release.get(900 * MS) or end_status["tick"] == by_release.get(
        910 * MS
    ):
        problems.append(f"L3: a status at an instant's end answered tip {end_status['tick']}")
    return problems


def _engine_lines_checks(run: Run, name: str) -> list[str]:
    """The engine's own lines (step.reached, host.start) carry conn=- (K10)."""
    bad = [
        ln
        for ln in run.host.run_log
        if (" step.reached " in ln or " host.start " in ln) and " conn=- pid=- " not in ln + " "
    ]
    return [f"{name}: an engine line carries a controller's identity: {bad[0]!r}"] if bad else []


def _invariance_checks(control: str | None = None) -> list[str]:
    """Pace invariance over every scenario, plus the sleep, hold, step and answer assertions."""
    ref = simulate(SCENARIOS[0]).host.engine
    problems = []
    extra: dict[str, Callable[[Run], list[str]]] = {
        "pause-resume": _held,
        "step": _step_scenario_checks,
        "step-contest": _contest_checks,
        "mid-instant": _mid_instant_checks,
        "step-clear": _clear_checks,
        "finish-race": _finish_checks,
        "queue-fold": _fold_checks,
    }
    for sc in SCENARIOS:
        run = simulate(sc, control)
        problems += _same_history(run, ref, sc.name)
        problems += _slept(run, sc.name)
        problems += _engine_lines_checks(run, sc.name)
        if sc.name in extra:
            problems += [f"{sc.name}: {p}" for p in extra[sc.name](run)]
        refused = [c for c, r in run.replies if _reason(r) != "ok"]
        if refused and sc.name != "finish-race":
            problems.append(f"{sc.name}: a scripted command was refused: {refused[0]}")
    return problems


def _is_command_line(line: str) -> bool:
    return line.startswith("pace ") and " host.start " not in line and " step.reached " not in line


def _run_log_checks() -> list[str]:
    """One pace line per command that changed the cell, none otherwise; the host lines."""
    problems = []
    for sc in SCENARIOS:
        run = simulate(sc)
        log = run.host.run_log
        changes = [
            c
            for c, r in run.replies
            if c.method in COMMANDS and _reason(r) == "ok" and _result(r)["changed"] is True
        ]
        pace_lines = [ln for ln in log if _is_command_line(ln)]
        if len(pace_lines) != len(changes):
            problems.append(f"{sc.name}: {len(pace_lines)} pace lines for {len(changes)} changes")
        for ln, c in zip(pace_lines, changes, strict=False):
            if ln.split()[5].split(":")[0] != c.method:
                problems.append(f"{sc.name}: {ln!r} is not {c.method}")
        heads = (log[0].split()[2], log[1].split()[5], log[-1].split()[2])
        if heads != ("start", "host.start", "finish"):
            problems.append(f"{sc.name}: the host lines are not start, pace, ..., finish")
        if any(len(ln.split()) not in {5, 9} for ln in log):
            problems.append(f"{sc.name}: a run-log line has the wrong field count")
    return problems


def self_test(control: str | None = None) -> list[str]:
    """Run every check; return the report lines (the last says PASS or FAIL)."""
    global METHODS  # noqa: PLW0603 - the planted extra-method control widens the table
    if control == "pace-on-stream":
        W.METHODS = W.METHODS | {"control.pace"}  # type: ignore[attr-defined]  # the stream table gains a command
    if control == "extra-method":
        METHODS = METHODS | {"control.seek"}  # a sixth method on the control table
    invariance: Check = (
        "pace invariance: fourteen scenarios reproduce the headless run's bytes, digests, "
        "keyframes, ladder and journal; the governor slept, no flood starved it; a pause held the "
        "tip; a step stopped after Bookkeeping of its target; a command landing in a reaching "
        "instant won; the end was never held; answers report the applied state in request order",
        lambda: _invariance_checks(control=control),
    )
    checks: list[Check] = [
        (
            "the method table: five methods, disjoint from the stream's; reserved names refused",
            lambda: _method_table_checks(control),
        ),
        ("every message: canonical, decoded, classified; the codes in range", _message_checks),
        (
            "the host's dispatcher: answers, refusals that change nothing, one log line a change",
            _dispatch_checks,
        ),
        ("host: the cap, status invariants, idempotence", _host_checks),
        ("the governor's rule: rates and deadlines in integers", _governor_checks),
        invariance,
        ("the run log: one pace line per change, the host lines", _run_log_checks),
    ]
    report, failed = [], False
    for i, (name, check) in enumerate(checks, 1):
        problems = check()
        failed |= bool(problems)
        report.append(f"{i}. {name}: {'FAIL' if problems else 'PASS'}")
        report.extend(f"   - {p}" for p in problems[:8])
    report.append("CONTROL v1: FAIL" if failed else "CONTROL v1: PASS")
    return report


# ------------------------------------------------------------------------------------------ golden
def _answer_token(reply: Json) -> str:
    if _reason(reply) != "ok":
        return _reason(reply)
    res = _pace_of(reply)
    step = res["step"]
    until = step["until_tick"] if isinstance(step, dict) else "-"
    changed = _result(reply).get("changed", "-")
    tick = "~" if res["tick"] is None else res["tick"]
    return f"{res['band']} {res['state']} {until} {tick} {res['next_seq']} {changed}"


def render_golden() -> str:
    """The golden file's full text."""
    header = [
        "# World Stream control channel v1: golden frames, dispatcher answers, paced scenarios.",
        "# GENERATED by specs/stream/selftest_control_v1.py; do not edit. Contract: control_v1.md.",
        "# `frame <name> <hex>`: one frame, header and payload, then `control <canonical json>`;",
        "#   rh-protocol encodes the decoded form and must produce the hex, and back.",
        "# `dispatch <name> <request hex> <reply hex>`: a frame from a controller and the host's",
        "#   answer on a fresh connection to an idle host whose engine ran one sim-year at max",
        "#   from a clock at 0 (`_finished`: the whole horizon; `_with_step`: a step to the last",
        "#   instant pending; `_genesis`, `_before_first`: no instant yet) (control_v1.md §4-5).",
        "# `scenario <name>` then `clock <wall_ns> <tick> <k>` per instant released (the fake wall",
        "#   clock when instant k started computing; an instant takes 10 ms and its records land,",
        "#   and the tip moves, at its end; control_v1.md §3.3 and §9's delivery rule), `answer",
        "#   <at_ns> <conn> <method> <band> <state> <until|-> <tick> <next_seq> <changed|->` or",
        "#   `answer <at_ns> <conn> <method> <reason>` per scripted command, status reads included",
        "#   (the applied state, or the published snapshot for a status), and `log <run-log line>`",
        "#   lines (control_v1.md §8.1); rh-golden's governor test reproduces the clock and answer",
        "#   lines.",
        "# `invariance <sha256 of the channel-A lines> <keyframe ticks> <journal head> <ladder>`:",
        "#   what every scenario produced, equal to the headless run's (control_v1.md §0 fact 2).",
    ]
    body: list[str] = []
    for name, obj in _messages():
        body.append(f"frame {name} {W.encode_frame(W.control_frame(obj)).hex()}")
        body.append(f"control {W.cjson(obj).decode()}")
    for case, reply, _, _, _ in dispatch_transcript():
        body.append(f"dispatch {case.name} {case.data.hex()} {reply.hex()}")
    ref: Engine | None = None
    for sc in SCENARIOS:
        run = simulate(sc)
        body.append(f"scenario {sc.name}")
        body.extend(f"clock {w} {t} {k}" for w, t, k in run.host.released)
        body.extend(
            f"answer {c.at_ns} {c.conn} {c.method} {_answer_token(r)}" for c, r in run.replies
        )
        body.extend(f"log {ln}" for ln in run.host.run_log)
        ref = ref or run.host.engine
    assert ref is not None
    lines_digest = hashlib.sha256("\n".join(ref.lines).encode()).hexdigest()
    kfs = ",".join(str(t) for t in ref.keyframes) or "-"
    body.append(
        f"invariance {lines_digest} {kfs} {ref.journal_head()} {ref.ladder[-1].split()[-1]}"
    )
    return "\n".join([*header, *body]) + "\n"


def _check_golden(golden: Path, text: str) -> int:
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


CONTROLS = (
    "pause-writes-record",
    "pace-in-journal",
    "band-in-hash",
    "input-accepted",
    "step-overshoot",
    "pace-on-stream",
    "paused-time-owed",
    "extra-method",
)


def main(argv: list[str] | None = None) -> int:
    """Run the self-test, then write or check the golden file (never with --control)."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the file to write or check")
    parser.add_argument("--control", choices=CONTROLS, help="a planted fault that must FAIL")
    args = parser.parse_args(argv)
    report = self_test(args.control)
    print("\n".join(report))
    if report[-1].endswith("FAIL"):
        return EXIT_SELFTEST
    if args.control:
        print("CONTROL PASSED: the planted fault was not detected", file=sys.stderr)
        return EXIT_SELFTEST
    text = render_golden()
    if args.check:
        return _check_golden(args.golden, text)
    args.golden.parent.mkdir(parents=True, exist_ok=True)
    args.golden.write_text(text, encoding="utf-8")
    print(f"wrote {args.golden} ({text.count(chr(10))} lines)")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
