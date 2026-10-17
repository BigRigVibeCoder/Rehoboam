"""Reference of the World Stream control channel v1 (IF-0 ``control.sock``; SPR-027 T-01).

The contract is ``specs/stream/control_v1.md``: the pace command over ``control.sock``, carried by
``wire_v1.md``'s frames and canonical JSON-RPC. This file is the independent reference that
``rh-protocol``'s control dispatcher and ``rh-golden``'s pace governor (SPR-027 T-03) must agree
with, as ``selftest_wire_v1.py`` is for the stream sockets:

- :data:`METHODS` is the method table (exactly five), :data:`RESERVED` the P2 names refused by
  name (IF-3's spellings, REV round 1 C12), :data:`ERRORS` the error model with this socket's codes;
- :class:`Engine` is a simulated engine: a deterministic record generator that knows nothing of
  the pace; :class:`Host` holds the governor's cell, applies commands at the engine's decision
  points (before and after an instant; C1), sleeps by control_v1.md §3.3's rule in integers over a
  :class:`FakeClock`, writes the run log and answers the socket;
- :func:`simulate` drives a host through a scripted command sequence over the fake clock, each
  instant taking wall time so that commands can land mid-instant, and the self-test asserts
  **pace invariance** (control_v1.md §0 fact 2): every paced run produces the headless run's
  channel-A bytes, digests, keyframes, ladder and journal, while the governor really slept, a
  pause really held the tip, a step stopped after Bookkeeping of its target, a flood of no-change
  commands never starved the run (C2), and the answers report the state the engine applied (C1);
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
# control_v1.md §5.1: the P2 operations under IF-3's names (05.9 IF-3; C12), refused by name with
# the detail a v1 host answers them with.
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

    ``generation`` counts the changes the engine applied (C1, C2): a command that changes nothing
    leaves it alone. A reached step is written only when no command landed since the engine's last
    read (the compare-and-swap on the generation it read; here, an empty queue).
    """

    band: str = "max"
    state: str = "running"
    step_target: int | None = None
    anchor_tick: int = FIRST_TICK
    anchor_wall_ns: int = 0
    generation: int = 0


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
    """A validated command waiting for the engine's next decision point (C1)."""

    conn: Connection
    rid: int | str
    method: str
    until: int | str | None = None  # control.step's request, resolved at application
    band: str | None = None
    reply: Json | None = None  # set when the engine applies or refuses it


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

    Serving threads validate a request and queue it (:meth:`submit`); the engine's loop applies
    the queue at its decision points, before an instant (:meth:`before_instant`) and after one
    (:meth:`after_instant`), and the answer reports the state the engine applied (C1). A command
    that changes nothing is answered at once and touches neither the anchor nor the generation
    (C2). ``control`` names a planted fault (control_v1.md §9); None is the honest host.
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
    mid_instant: bool = False

    def __post_init__(self) -> None:
        """Log the host's start and its start pace (control_v1.md §8.1)."""
        if self.start_paused:
            self.cell.state = "paused"
        self._log_host("start")
        self._log_pace("host.start", None)

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

    # -- the cell ---------------------------------------------------------------------------
    def _anchor(self) -> None:
        tip = self.engine.tip
        self.cell.anchor_tick = FIRST_TICK if tip is None else tip
        self.cell.anchor_wall_ns = self.clock.now_ns

    def _bump(self) -> None:
        self.cell.generation += 1

    def pace_object(self) -> Json:
        """The pace object every command returns (control_v1.md §4): the applied state."""
        c = self.cell
        step = None if c.step_target is None else {"until_tick": c.step_target}
        return {
            "band": c.band,
            "next_seq": self.engine.next_seq,
            "state": c.state,
            "step": step,
            "tick": self.engine.tip,
        }

    def _desired(self) -> tuple[str, str, int | str | None]:
        """The cell with the queued commands folded in: what the next command is judged against."""
        band, state = self.cell.band, self.cell.state
        step: int | str | None = self.cell.step_target
        for p in self.pending:
            if p.method == "control.pace":
                band = str(p.band)
            elif p.method == "control.pause":
                state, step = "paused", None
            elif p.method == "control.resume":
                state, step = "running", None
            elif p.method == "control.step":
                state, step = "running", p.until
        return band, state, step

    def _is_change(self, method: str, band: str | None, until: int | str | None) -> bool:
        d_band, d_state, d_step = self._desired()
        if method == "control.pace":
            return band != d_band
        if method == "control.pause":
            return d_state != "paused"
        if method == "control.resume":
            return d_state != "running" or d_step is not None
        return d_state != "running" or d_step != until

    # -- commands: the serving thread's side ------------------------------------------------
    def submit(
        self,
        conn: Connection,
        rid: int | str,
        method: str,
        band: str | None = None,
        until: int | str | None = None,
    ) -> Pending:
        """Queue a validated command for the engine, or answer it at once (C1, C2)."""
        p = Pending(conn, rid, method, until, band)
        if self.cell.state == "finished":
            p.reply = error(rid, "run_finished", end=self.end)
        elif not self._is_change(method, band, until):
            p.reply = W.response(rid, {**self.pace_object(), "changed": False})
        else:
            self.pending.append(p)  # the engine bumps the generation when it applies the change
        return p

    # -- commands: the engine's side --------------------------------------------------------
    def _apply_pending(self) -> None:
        """Apply the queued commands in order; each answer carries the tip it took effect at."""
        for p in self.pending:
            try:
                result = self._apply(p)
            except Refused as r:
                p.reply = error(p.rid, r.reason, **r.data)
            else:
                p.reply = W.response(p.rid, result)
        self.pending.clear()

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
            self._anchor()  # the band changed (C2)
            self._bump()
            if self.control == "pace-in-journal":
                self.engine.journal.append(f"pace {band}")  # STR-5 broken: the pace as an input
            self._log_pace("control.pace", p.conn)
        return {**self.pace_object(), "changed": changed}

    def _pause(self, p: Pending) -> Json:
        changed = self.cell.state == "running"
        if changed:
            self.cell.state, self.cell.step_target = "paused", None
            self._anchor()  # the run state changed (C2)
            self._bump()
            if self.control == "pause-writes-record":
                self.engine.append("PAUSE", self.engine.tip or FIRST_TICK)  # W35 broken
            self._log_pace("control.pause", p.conn)
        return {**self.pace_object(), "changed": changed}

    def _resume(self, p: Pending) -> Json:
        was_paused, had_step = self.cell.state == "paused", self.cell.step_target is not None
        changed = was_paused or had_step
        if changed:
            self.cell.state, self.cell.step_target = "running", None
            self._bump()
            if was_paused and self.control != "paused-time-owed":
                self._anchor()  # the run state changed; paused time is not owed
            self._log_pace("control.resume", p.conn)
        return {**self.pace_object(), "changed": changed}

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
            if self.cell.state != "running":
                self._anchor()  # the run state changed
            self.cell.state, self.cell.step_target = "running", target
            self._bump()
            self._log_pace(f"control.step:{target}", p.conn)
            if nxt > target:
                self._reach(p.conn)  # no instant lies at or below the target: paused at once
        return {**self.pace_object(), "changed": changed}

    def _reach(self, conn: Connection | None = None) -> None:
        self.cell.state, self.cell.step_target = "paused", None
        self._anchor()
        self._bump()
        self._log_pace("step.reached", conn)

    def _step_reached(self) -> bool:
        target, nxt = self.cell.step_target, self.engine.next_tick
        if target is None:
            return False
        if self.control == "step-overshoot":
            return self.engine.tip is not None and self.engine.tip > target  # W37 broken
        return nxt is None or nxt > target  # the last instant at or below the target ran

    # -- the engine's loop ------------------------------------------------------------------
    def before_instant(self) -> None:
        """Decision point 1 (control_v1.md §3.3 items 1 and 2): apply, re-check a pending step."""
        self._apply_pending()
        if self._step_reached() and self.cell.state == "running":
            self._reach()

    def deadline(self) -> int | None:
        """The wall clock at which the next instant may run; None when paused or finished."""
        c, tick = self.cell, self.engine.next_tick
        if c.state != "running" or tick is None:
            return None
        r = None if c.step_target is not None else rate(c.band)  # a step runs at max
        return deadline_ns(c.anchor_wall_ns, c.anchor_tick, tick, r)

    def run_instant(self) -> int:
        """Item 4: the engine runs one instant; the clock advances by its compute time."""
        self.mid_instant = True
        hint = self.cell.band if self.control == "band-in-hash" else None
        tick = self.engine.run_instant(hint)
        self.released.append((self.clock.now_ns, tick, self.engine.instants_run))
        self.clock.now_ns += INSTANT_WALL_NS
        self.mid_instant = False
        return tick

    def after_instant(self) -> None:
        """Decision point 2 (item 5): a reached step pauses by CAS, else the commands win."""
        if self.engine.finished:
            self.finish()
            return
        if not self.pending:  # nothing landed since item 1: the compare-and-swap succeeds
            if self._step_reached():
                self._reach()
        else:
            self._apply_pending()  # a command landed mid-instant: it wins, no step.reached here

    def finish(self, reason: str = "finished") -> None:
        """The run ended (C14): never held; a pending step is dropped; queued commands refused."""
        self.cell.state, self.cell.step_target, self.end = "finished", None, reason
        self._log_host("finish" if reason == "finished" else "abort")
        for p in self.pending:
            p.reply = error(p.rid, "run_finished", end=reason)
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
        """The ``control.status`` result (control_v1.md §4.1): the published snapshot, no wait."""
        return {
            "calendar": {"minutes_per_year": MINUTES_PER_YEAR},
            "control": {"format": CONTROL_FORMAT, "version": CONTROL_VERSION},
            "controllers": len(self.connections),
            "end": None if self.end is None else {"reason": self.end},
            "history": dict(HISTORY),
            "pace": self.pace_object(),
            "run_nonce": self.run_nonce,
            "timeline_id": TIMELINE,
            "tip": {"next_seq": self.engine.next_seq, "tick": self.engine.tip},
        }

    def dispatch(self, conn: Connection, data: bytes) -> list[bytes]:
        """Answer one frame from a controller on an idle host (between instants).

        The header is judged first; a command is queued and, the engine being at a decision
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
            if method == "control.status":
                no_params(params)
                return W.response(rid, self.status())  # type: ignore[no-any-return]
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


@dataclass(frozen=True)
class Scenario:
    """A command script over the simulated run."""

    name: str
    commands: tuple[Command, ...] = ()
    start_paused: bool = False


@dataclass
class Run:
    """What a simulated run produced: the host, and each command's answer in issue order."""

    host: Host
    issued: list[tuple[Command, Pending | Json]] = field(default_factory=list)

    @property
    def replies(self) -> list[tuple[Command, Json]]:
        """Each command and its answer (every answer exists once the run is over)."""
        out = []
        for cmd, outcome in self.issued:
            reply = outcome.reply if isinstance(outcome, Pending) else outcome
            assert reply is not None, cmd
            out.append((cmd, reply))
        return out


S = NS_PER_S
MS = 1_000_000
YEAR1 = FIRST_TICK + MINUTES_PER_YEAR
STEP_TICK = FIRST_TICK + 15 * INSTANT_MINUTES  # instant 15, mid-year 2
STEP_TICK_OFF = STEP_TICK + 2_000  # between instants 15 and 16: stops after 15 (W37)
STEP_TICK_LATER = STEP_TICK + 3 * INSTANT_MINUTES  # instant 18


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
    Scenario(  # C1: commands landing mid-instant
        "mid-instant",
        (
            Command(5 * MS, "a", "control.pause"),
            Command(100 * MS, "a", "control.resume"),
            Command(105 * MS, "a", "control.step", {"until": "instant"}),
            Command(S, "a", "control.resume"),
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
)


def _issue(run: Run, conns: dict[str, Connection], cmd: Command) -> None:
    """A controller sends ``cmd``: validated now; applied at the engine's next decision point."""
    host = run.host
    conn = conns.setdefault(cmd.conn, host.connect())
    rid = len(run.issued) + 1
    data = W.encode_frame(W.control_frame(W.request(rid, cmd.method, cmd.params)))
    frame, _ = W.decode_frame(data)
    outcome = host._answer(conn, frame.payload)
    run.issued.append((cmd, outcome))


def simulate(sc: Scenario, control: str | None = None) -> Run:
    """Drive a host through ``sc`` over the fake clock; the engine runs to its horizon.

    Commands land at their wall time; one that lands mid-instant waits for the instant's end
    (C1); one that lands during a governor wait ends the wait (control_v1.md §3.3 item 3).
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
        host.run_instant()
        while queue and start < queue[0].at_ns <= host.clock.now_ns:  # landed mid-instant
            _issue(run, conns, queue.pop(0))
        host.after_instant()
    while queue:  # after the end: run_finished
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
    pace = {**host.pace_object(), "changed": True}
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
    beyond = HORIZON_YEARS * MINUTES_PER_YEAR + 1
    res = _result(_ask(host, 3, "control.step", {"until": {"tick": beyond}}))
    if res["state"] != "running" or res["step"] != {"until_tick": beyond}:
        problems.append("a step beyond the horizon runs to the end")
    gen = host.cell.generation
    _ask(host, 4, "control.step", {"until": {"tick": beyond}})
    if host.cell.generation != gen:
        problems.append("a no-change command bumped the generation (C2)")
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


def _replies(run: Run, method: str) -> list[tuple[int, Json]]:
    return [(c.at_ns, r) for c, r in run.replies if c.method == method]


def _held(run: Run) -> list[str]:
    """A pause held the tip and wrote nothing; the resume owed no time (control_v1.md §4.3)."""
    problems = []
    statuses = _replies(run, "control.status")
    pairs = zip(_replies(run, "control.pause"), _replies(run, "control.resume"), strict=False)
    for (p_at, p_reply), (r_at, r_reply) in pairs:
        if any(p_at < w < r_at for w, _, _ in run.host.released):
            problems.append("an instant ran while paused")
        seqs = set()
        for at, st in statuses:
            if p_at <= at <= r_at:
                tip = _result(st)["tip"]
                assert isinstance(tip, dict)
                seqs.add(int(str(tip["next_seq"])))
        p_res, r_res = _result(p_reply), _result(r_reply)
        seqs |= {int(str(p_res["next_seq"])), int(str(r_res["next_seq"]))}
        if len(seqs) > 1:
            problems.append("a pause wrote a record: next_seq moved while paused")
        if p_res["tick"] != r_res["tick"]:
            problems.append("the tip moved while paused")
        after = [w for w, _, _ in run.host.released if w >= r_at]
        tip_at_resume = r_res["tick"]
        assert isinstance(tip_at_resume, int)
        exact = _exact_releases(run, "1y", r_at, tip_at_resume, r_at)
        if exact:
            problems.append(f"paused time was owed: {exact[0]}")
        if not after:
            problems.append("nothing ran after the resume")
    return problems


def _stepped(run: Run) -> list[str]:
    """Each step stopped after Bookkeeping of the last instant at or below its target (W37)."""
    problems = []
    reached = [
        (int(ln.split()[1]), ln.split()[4]) for ln in run.host.run_log if " step.reached " in ln
    ]
    for at, reply in _replies(run, "control.step"):
        if _reason(reply) != "ok":
            continue  # reported once by the invariance check
        res = _result(reply)
        step, tick = res["step"], res["tick"]
        target = step["until_tick"] if isinstance(step, dict) else tick
        assert isinstance(target, int)
        want = target - (target - FIRST_TICK) % INSTANT_MINUTES
        hit = next((tip for w, tip in reached if w >= at), None)
        if hit is None or hit == "~":
            problems.append(f"the step to {target} never paused")
        elif int(hit) > want:
            problems.append(f"the step overshot its target: paused at {hit}, not {want}")
        elif int(hit) < want:
            problems.append(f"the step stopped short: paused at {hit}, not {want}")
    return problems


def _step_scenario_checks(run: Run) -> list[str]:
    """The step scenario's exact answers: the resume at 1y re-anchors at the tip (C3's mutants)."""
    problems = _stepped(run)
    resume_at, resume_reply = _replies(run, "control.resume")[-1]
    tip = _result(resume_reply)["tick"]
    assert isinstance(tip, int)
    if tip != STEP_TICK_LATER:
        problems.append(f"the last resume answered tip {tip}, expected {STEP_TICK_LATER}")
    problems += _exact_releases(run, "1y", resume_at, tip, resume_at)
    return problems


def _contest_checks(run: Run) -> list[str]:
    """C3: a pause clears a step, a resume clears a step, a later step replaces the target."""
    problems = []
    by_method = {m: _replies(run, m) for m in ("control.pause", "control.resume", "control.step")}
    pause_b = _result(by_method["control.pause"][1][1])
    if pause_b["step"] is not None or pause_b["state"] != "paused":
        problems.append("a pause left the step pending")
    steps = [_result(r) for _, r in by_method["control.step"]]
    if any(s["state"] != "running" for s in steps[:3]):
        problems.append("a step did not run")
    reached = [int(ln.split()[4]) for ln in run.host.run_log if " step.reached " in ln]
    if _tick(40) in reached:
        problems.append("the step cancelled by the pause still reached its target")
    if _tick(24) in reached or _tick(30) not in reached or _tick(20) not in reached:
        problems.append(f"the replaced step's targets are wrong: {reached}")
    return problems


def _mid_instant_checks(run: Run) -> list[str]:
    """C1: answers report the state the engine applied, at the tip it applied it."""
    problems = []
    pause = _result(_replies(run, "control.pause")[0][1])
    if pause["tick"] != FIRST_TICK or pause["state"] != "paused":
        problems.append(f"a pause landing mid-instant 0 answered {pause}")
    if run.host.released[1][0] < 100 * MS:
        problems.append("instant 1 ran while paused")
    step = _result(_replies(run, "control.step")[0][1])
    if step["tick"] != _tick(1) or step["step"] != {"until_tick": _tick(2)}:
        problems.append(f"a step landing mid-instant 1 answered {step}")
    reached = [int(ln.split()[4]) for ln in run.host.run_log if " step.reached " in ln]
    if reached != [_tick(2)]:
        problems.append(f"the mid-instant step paused at {reached}, expected {[_tick(2)]}")
    return problems


def _invariance_checks(control: str | None = None) -> list[str]:
    """Pace invariance over every scenario, plus the sleep, hold, step and answer assertions."""
    ref = simulate(SCENARIOS[0]).host.engine
    problems = []
    extra: dict[str, Callable[[Run], list[str]]] = {
        "pause-resume": _held,
        "step": _step_scenario_checks,
        "step-contest": _contest_checks,
        "mid-instant": _mid_instant_checks,
    }
    for sc in SCENARIOS:
        run = simulate(sc, control)
        problems += _same_history(run, ref, sc.name)
        problems += _slept(run, sc.name)
        if sc.name in extra:
            problems += [f"{sc.name}: {p}" for p in extra[sc.name](run)]
        if any(_reason(r) != "ok" for _, r in run.replies):
            problems.append(f"{sc.name}: a scripted command was refused")
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
        if any(" step.reached " in ln for ln in log[-2:]) and sc.name == "step":
            problems.append(f"{sc.name}: a step reached at the end (C14)")
    return problems


def self_test(control: str | None = None) -> list[str]:
    """Run every check; return the report lines (the last says PASS or FAIL)."""
    global METHODS  # noqa: PLW0603 - the planted extra-method control widens the table
    if control == "pace-on-stream":
        W.METHODS = W.METHODS | {"control.pace"}  # type: ignore[attr-defined]  # the stream table gains a command
    if control == "extra-method":
        METHODS = METHODS | {"control.seek"}  # a sixth method on the control table
    invariance: Check = (
        "pace invariance: eleven scenarios reproduce the headless run's bytes, digests, keyframes, "
        "ladder and journal; the governor slept, no flood starved it; a pause held the tip; a step "
        "stopped after Bookkeeping of its target; answers report the applied state",
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
def _answer_token(cmd: Command, reply: Json) -> str:
    if _reason(reply) != "ok":
        return _reason(reply)
    res = _result(reply)
    step = res["step"]
    until = step["until_tick"] if isinstance(step, dict) else "-"
    changed = res.get("changed", "-")
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
        "#   clock when instant k started; each instant takes 10 ms; control_v1.md §3.3), `answer",
        "#   <at_ns> <conn> <method> <band> <state> <until|-> <tick> <next_seq> <changed>` or",
        "#   `answer <at_ns> <conn> <method> <reason>` per scripted command (C1: the applied",
        "#   state),",
        "#   and `log <run-log line>` lines (control_v1.md §8.1); rh-golden's governor test",
        "#   reproduces the clock and answer lines.",
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
            f"answer {c.at_ns} {c.conn} {c.method} {_answer_token(c, r)}"
            for c, r in run.replies
            if c.method != "control.status"
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
