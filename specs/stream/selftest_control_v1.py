"""Reference of the World Stream control channel v1 (IF-0 ``control.sock``; SPR-027 T-01).

The contract is ``specs/stream/control_v1.md``: the pace command over ``control.sock``, carried by
``wire_v1.md``'s frames and canonical JSON-RPC. This file is the independent reference that
``rh-protocol``'s control dispatcher and ``rh-golden``'s pace governor (SPR-027 T-03) must agree
with, as ``selftest_wire_v1.py`` is for the stream sockets:

- :data:`METHODS` is the method table (exactly five), :data:`RESERVED` the P2.7 names refused by
  name, :data:`ERRORS` the error model with this socket's codes;
- :class:`Engine` is a simulated engine: a deterministic record generator that knows nothing of
  the pace; :class:`Governor` and :class:`FakeClock` apply control_v1.md §3.3's rule in integers;
  :class:`Host` is the socket server: the cell, the run log, the dispatcher;
- :func:`simulate` drives a host through a scripted command sequence over the fake clock, and
  the self-test asserts **pace invariance** (control_v1.md §0 fact 2): every paced run produces
  the headless run's channel-A bytes, digests, keyframes, ladder and journal, while the governor
  really slept, a pause really held the tip and a step stopped after Bookkeeping of its target;
- the golden file ``specs/stream/golden/control_v1.txt`` holds one frame of every message, the
  host's answers to planted requests, each scenario's clock and run-log lines, and the invariant
  digest every scenario reproduced.

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
INPUT_DETAIL = (
    "reserved for P2.7 (D47): a material input is journaled through IF-2 with its acceptance "
    "cursor (BCK-002 A-36) before it is applied; v1 accepts no input"
)
# control_v1.md §5.1: reserved names and the detail a v1 host answers them with.
RESERVED: dict[str, str] = {
    "control.input": INPUT_DETAIL,
    "control.schedule": INPUT_DETAIL,
    "control.fork": (
        "reserved for P2.7 (D47): a fork is a new timeline with its own directory and sockets; "
        "v1 has one timeline"
    ),
    "control.autopause": (
        "reserved for P2 (D-5): the auto-pause is the sifter's and the viewer's, not v1's"
    ),
    "control.run_until": (
        "reserved for P2: a predicate over aggregates needs the sifter; advancing to a tick is "
        "control.step"
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
CONTROL_CODE_LO, CONTROL_CODE_HI = -32120, -32129  # this socket's range (control_v1.md §5)
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
HORIZON_YEARS = 4
KEYFRAME_YEARS = 2  # the simulation's cadence (the engine's is ten years, 05.10 §4)
LINES_PER_INSTANT = 3
TIMELINE = 0
GENESIS_TICK = 0
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

    The rule in integers: ``(wall - anchor_wall) * r >= (tick - anchor_tick) * 1e9``.
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

    Instants fall at ``k * INSTANT_MINUTES`` for ``k = 1..``; each emits lines derived from the
    tick alone, a ``STEP_DIGEST`` whose ``state_hash`` is a hash of the state so far, a
    ``KEYFRAME`` every ``KEYFRAME_YEARS``, and a ladder entry per year. The journal has one row,
    written before the run, as P1a's inputs are.
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
        """The tick of the next instant, or None when the run is over."""
        return None if self.finished else (self.instants_run + 1) * INSTANT_MINUTES

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
            lo, hi = (year - 1) * MINUTES_PER_YEAR, year * MINUTES_PER_YEAR
            lines = [ln for ln in self.lines if lo < int(ln.split()[3]) <= hi]
            out.append(str(W.digest(lines)))
        return out


# ------------------------------------------------------------------------------ the governor's cell
@dataclass
class FakeClock:
    """A wall clock the governor sleeps on; ``sleep_until`` advances it."""

    now_ns: int = 0

    def sleep_until(self, t_ns: int) -> None:
        """Advance to ``t_ns`` if it is later than now."""
        self.now_ns = max(self.now_ns, t_ns)


@dataclass
class Cell:
    """The governor's cell (control_v1.md §3.3): written by commands, read between instants."""

    band: str = "max"
    state: str = "running"
    step_target: int | None = None
    anchor_tick: int = GENESIS_TICK
    anchor_wall_ns: int = 0


class Refused(Exception):  # noqa: N818 - a refusal, not an error of the reference
    """A refused request: the reason and its extra ``data`` fields; nothing changes."""

    def __init__(self, reason: str, **data: object) -> None:
        """Record the reason and data."""
        super().__init__(reason)
        self.reason, self.data = reason, data


@dataclass
class Connection:
    """One controller's connection as the host sees it."""

    closed: bool = False


def error(rid: int | str | None, reason: str, **data: object) -> Json:
    """A JSON-RPC error response for ``reason`` (control_v1.md §5) with extra ``data`` fields."""
    code, message = ERRORS[reason]
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

    Serving threads write the cell and the run log through the command methods; the engine's
    loop (:meth:`run_instant`) reads the cell and never a socket. ``control`` names a planted
    fault (control_v1.md §9); None is the honest host.
    """

    engine: Engine
    clock: FakeClock
    control: str | None = None
    cap: int = CONTROLLER_CAP
    run_nonce: int = 1
    cell: Cell = field(default_factory=Cell)
    run_log: list[str] = field(default_factory=list)
    connections: list[Connection] = field(default_factory=list)
    released: list[tuple[int, int, int]] = field(default_factory=list)  # (wall_ns, tick, k)
    end: str | None = None

    def __post_init__(self) -> None:
        """Log the host's start and its start pace (control_v1.md §8.1)."""
        self._log_host("start")
        self._log_pace("host.start")

    # -- the run log ------------------------------------------------------------------------
    def _tip_token(self) -> str:
        return "~" if self.engine.tip is None else str(self.engine.tip)

    def _log_pace(self, method: str) -> None:
        c = self.cell
        line = f"pace {self.clock.now_ns} {c.band} {c.state} {self._tip_token()} {method}"
        self.run_log.append(f"{line} {self.run_nonce}")

    def _log_host(self, event: str) -> None:
        self.run_log.append(
            f"host {self.clock.now_ns} {event} {self._tip_token()} {self.run_nonce}"
        )

    # -- the cell ---------------------------------------------------------------------------
    def _anchor(self) -> None:
        tip = self.engine.tip
        self.cell.anchor_tick = GENESIS_TICK if tip is None else tip
        self.cell.anchor_wall_ns = self.clock.now_ns

    def pace_object(self) -> Json:
        """The pace object every command returns (control_v1.md §4)."""
        c = self.cell
        step = None if c.step_target is None else {"until_tick": c.step_target}
        return {"band": c.band, "state": c.state, "step": step, "tick": self.engine.tip}

    def _require_running_or_paused(self) -> None:
        if self.cell.state == "finished":
            raise Refused("run_finished", end=self.end)

    def pace(self, band: str) -> Json:
        """Apply ``control.pace`` (control_v1.md §4.2)."""
        self._require_running_or_paused()
        changed = band != self.cell.band
        self.cell.band = band
        self._anchor()
        if self.control == "pace-in-journal":
            self.engine.journal.append(f"pace {band}")  # STR-5 broken: the pace as an input
        self._log_pace("control.pace")
        return {**self.pace_object(), "changed": changed}

    def pause(self) -> Json:
        """Apply ``control.pause`` (control_v1.md §4.3)."""
        self._require_running_or_paused()
        changed = self.cell.state == "running"
        self.cell.state, self.cell.step_target = "paused", None
        self._anchor()
        if self.control == "pause-writes-record" and changed:
            self.engine.append("PAUSE", self.engine.tip or GENESIS_TICK)  # W35 broken
        self._log_pace("control.pause")
        return {**self.pace_object(), "changed": changed}

    def resume(self) -> Json:
        """Apply ``control.resume`` (control_v1.md §4.4)."""
        self._require_running_or_paused()
        changed = self.cell.state == "paused" or self.cell.step_target is not None
        self.cell.state, self.cell.step_target = "running", None
        if self.control != "paused-time-owed":
            self._anchor()  # paused time is not owed
        self._log_pace("control.resume")
        return {**self.pace_object(), "changed": changed}

    def step(self, until: int | str) -> Json:
        """Apply ``control.step`` (control_v1.md §4.5)."""
        self._require_running_or_paused()
        tip = GENESIS_TICK if self.engine.tip is None else self.engine.tip
        nxt = self.engine.next_tick
        target = until if isinstance(until, int) else (tip if nxt is None else nxt)
        if target <= tip:
            raise Refused("past_tick", tip_tick=self.engine.tip)
        changed = self.cell.step_target != target or self.cell.state != "running"
        self.cell.state, self.cell.step_target = "running", target
        self._anchor()
        self._log_pace(f"control.step:{target}")
        if nxt is None or nxt > target:
            self._reach()  # no instant lies at or below the target: paused at once
        return {**self.pace_object(), "changed": changed}

    def _reach(self) -> None:
        self.cell.state, self.cell.step_target = "paused", None
        self._anchor()
        self._log_pace("step.reached")

    # -- the engine's loop ------------------------------------------------------------------
    def deadline(self) -> int | None:
        """The wall clock at which the next instant may run; None when paused or finished."""
        c, tick = self.cell, self.engine.next_tick
        if c.state != "running" or tick is None:
            return None
        return deadline_ns(c.anchor_wall_ns, c.anchor_tick, tick, rate(c.band))

    def run_instant(self) -> None:
        """Run one instant now (the loop has slept to the deadline), then the step check."""
        hint = self.cell.band if self.control == "band-in-hash" else None
        tick = self.engine.run_instant(hint)
        self.released.append((self.clock.now_ns, tick, self.engine.instants_run))
        target = self.cell.step_target
        if target is None:
            return
        nxt = self.engine.next_tick
        # Honest: the last instant at or below the target ran; step-overshoot (W37 broken): one more
        overshoot = self.control == "step-overshoot"
        reached = tick > target if overshoot else (nxt is None or nxt > target)
        if reached:
            self._reach()

    def finish(self, reason: str = "finished") -> None:
        """The run ended (or aborted): the cell says so; commands are refused from here."""
        self.cell.state, self.cell.step_target, self.end = "finished", None, reason
        self._log_host("finish" if reason == "finished" else "abort")

    # -- the socket -------------------------------------------------------------------------
    def connect(self) -> Connection:
        """Accept a connection, or refuse it at the cap."""
        conn = Connection(closed=len(self.connections) >= self.cap)
        if not conn.closed:
            self.connections.append(conn)
        return conn

    def close(self, conn: Connection) -> None:
        """The controller closed; nothing else changes."""
        conn.closed = True
        if conn in self.connections:
            self.connections.remove(conn)

    def status(self) -> Json:
        """The ``control.status`` result (control_v1.md §4.1)."""
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
        """Answer one frame from a controller: the header first, then the payload."""
        try:
            ftype, _ = W.decode_header(data, W.VIEWER_MAX_PAYLOAD)
            if ftype == W.TYPE_RECORDS:
                raise WireError("a RECORDS frame on the control socket")
            frame, _ = W.decode_frame(data, W.VIEWER_MAX_PAYLOAD)
        except W.IncompleteFrameError:
            return []
        except WireError as e:
            conn.closed = True
            reason = "unsupported_version" if "unsupported_version" in str(e) else "invalid_frame"
            extra: Json = (
                {"supported": [W.VERSION]}
                if reason == "unsupported_version"
                else {"detail": str(e)}
            )
            return [bytes(W.encode_frame(W.control_frame(error(None, reason, **extra))))]
        return [bytes(W.encode_frame(W.control_frame(self._answer(frame.payload))))]

    def _answer(self, payload: bytes) -> Json:
        try:
            kind, obj = W.decode_control(payload)
        except WireError as e:
            text = str(e)
            reason = "parse_error" if text.startswith("parse_error") else "invalid_request"
            if text.startswith("invalid_params") or "a float" in text:
                reason = "invalid_params"
            return error(None, reason, detail=text)
        if kind != "request":
            return error(None, "invalid_request", detail=f"a {kind} from a controller")
        rid, method, params = obj["id"], str(obj["method"]), obj.get("params")
        assert isinstance(rid, (int, str))
        if method in METHODS:
            return self._route(rid, method, params)
        return self._refuse_method(rid, method, params)

    def _refuse_method(self, rid: int | str, method: str, params: object) -> Json:
        if self.control == "input-accepted" and method == "control.input":
            self.engine.journal.append(f"input {params!r}")  # §5.1 broken: an input applied
            accepted: Json = W.response(rid, {"accepted": True})
            return accepted
        if method in RESERVED:
            return error(rid, "method_not_found", detail=RESERVED[method])
        if method.split(".", 1)[0] in RESERVED_NAMESPACES:
            return error(rid, "method_not_found", detail=IF3_DETAIL)
        return error(rid, "method_not_found")

    def _route(self, rid: int | str, method: str, params: object) -> Json:
        try:
            result = self._apply(method, params)
        except WireError as e:
            return error(rid, "invalid_params", detail=str(e))
        except Refused as r:
            return error(rid, r.reason, **r.data)
        return W.response(rid, result)  # type: ignore[no-any-return]

    def _apply(self, method: str, params: object) -> Json:
        if method == "control.pace":
            return self.pace(band_param(params))
        if method == "control.step":
            return self.step(step_param(params))
        no_params(params)
        if method == "control.status":
            return self.status()
        return self.pause() if method == "control.pause" else self.resume()


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


@dataclass
class Run:
    """What a simulated run produced."""

    host: Host
    replies: list[tuple[Command, Json]] = field(default_factory=list)
    status_before_pause: dict[int, Json] = field(default_factory=dict)


S = NS_PER_S
YEAR1 = MINUTES_PER_YEAR
STEP_TICK = 15 * INSTANT_MINUTES  # instant 15, mid-year 2
STEP_TICK_OFF = STEP_TICK + 2_000  # between instants 15 and 16: stops after 15 (W37)
STEP_TICK_LATER = STEP_TICK + 3 * INSTANT_MINUTES  # instant 18
SCENARIOS: tuple[Scenario, ...] = (
    Scenario("headless-max"),
    Scenario("band-1y", (Command(0, "a", "control.pace", {"band": "1y"}),)),
    Scenario("band-10y", (Command(0, "a", "control.pace", {"band": "10y"}),)),
    Scenario(
        "pause-resume",
        (
            Command(0, "a", "control.pace", {"band": "1y"}),
            Command(S + 50_000_000, "a", "control.pause"),
            Command(S + 60_000_000, "a", "control.status"),
            Command(3 * S + 50_000_000, "a", "control.status"),
            Command(3 * S + 50_000_000, "a", "control.resume"),
        ),
    ),
    Scenario(
        "step",
        (
            Command(0, "a", "control.pause"),
            Command(100_000_000, "a", "control.step", {"until": {"tick": STEP_TICK_OFF}}),
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
            Command(S // 2 + 200_000_000, "a", "control.pause"),
            Command(S // 2 + 400_000_000, "b", "control.resume"),
            Command(S, "b", "control.step", {"until": "instant"}),
            Command(2 * S, "a", "control.pause"),
            Command(2 * S + 100_000_000, "a", "control.resume"),
            Command(2 * S + 100_000_000, "b", "control.pace", {"band": "max"}),
        ),
    ),
)


def _issue(run: Run, conns: dict[str, Connection], cmd: Command) -> None:
    conn = conns.setdefault(cmd.conn, run.host.connect())
    data = W.encode_frame(W.control_frame(W.request(len(run.replies) + 1, cmd.method, cmd.params)))
    replies = run.host.dispatch(conn, data)
    frame, _ = W.decode_frame(b"".join(replies))
    _, obj = W.decode_control(frame.payload)
    run.replies.append((cmd, obj))


def simulate(sc: Scenario, control: str | None = None) -> Run:
    """Drive a host through ``sc`` over the fake clock; the engine runs to its horizon."""
    host = Host(Engine(), FakeClock(), control)
    run, conns = Run(host), dict[str, Connection]()
    queue = sorted(sc.commands, key=lambda c: c.at_ns)
    while True:
        while queue and queue[0].at_ns <= host.clock.now_ns:
            _issue(run, conns, queue.pop(0))
        if host.engine.finished:
            break
        deadline = host.deadline()
        if deadline is None:  # paused: the host waits for the next command
            if not queue:
                raise RuntimeError(f"{sc.name}: paused with no command left")
            host.clock.sleep_until(queue[0].at_ns)
            continue
        if queue and queue[0].at_ns < deadline:  # a command lands during the sleep
            host.clock.sleep_until(queue[0].at_ns)
            continue
        host.clock.sleep_until(deadline)
        host.run_instant()
    host.finish()
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


def _fresh_host(years: int = 1, finished: bool = False, control: str | None = None) -> Host:
    """A host whose engine ran ``years`` sim-years at ``max`` on a clock at 0."""
    host = Host(Engine(), FakeClock(), control)
    for _ in range(years * INSTANTS_PER_YEAR):
        host.run_instant()
    if finished:
        for _ in range((HORIZON_YEARS - years) * INSTANTS_PER_YEAR):
            host.run_instant()
        host.finish()
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
        extra = sorted(stream - STREAM_METHODS)
        problems.append(f"the stream method table carries {', '.join(extra) or 'a missing read'}")
    if METHODS & stream:
        problems.append("the two method tables are not disjoint")
    if RESERVED.keys() & METHODS:
        problems.append("a reserved name is in the method table")
    return problems


def _method_table_checks(control: str | None = None) -> list[str]:
    """The method table is exactly §4's, disjoint from the stream's; reserved names refused."""
    problems = _table_checks()
    host = _fresh_host(control=control)
    for name, detail in RESERVED.items():
        reply = _ask(host, 1, name, {"x": 1})
        if _reason(reply) != "method_not_found":
            problems.append(f"an input was accepted on control.sock: {name} -> {_reason(reply)}")
        elif _detail(reply) != detail:
            problems.append(f"{name}: detail {_detail(reply)!r}")
    for name in ("player.act", "timeline.fork", "oracle.whisper", "stream.subscribe", "x.y"):
        reply = _ask(host, 2, name)
        if _reason(reply) != "method_not_found":
            problems.append(f"{name}: {_reason(reply)}")
    reply = _ask(host, 3, "player.act", {"verb_id": 3})
    if _detail(reply) != IF3_DETAIL:
        problems.append("player.act: the IF-3 detail is missing")
    if host.engine.journal != Engine().journal:
        problems.append("an input was accepted on control.sock: the journal changed")
    return problems


def _messages() -> list[tuple[str, Json]]:
    """One object of every request, result and error (the golden ``frame`` lines)."""
    host = _fresh_host()
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
        ("error_reserved", error(8, "method_not_found", detail=RESERVED["control.input"])),
        (
            "error_invalid_params",
            error(
                9,
                "invalid_params",
                detail="invalid_params: band is one of live, 1h, 1d, 1w, 1mo, 1y, 10y, max",
            ),
        ),
        ("error_too_many_controllers", error(None, "too_many_controllers", limit=CONTROLLER_CAP)),
        ("error_run_finished", error(10, "run_finished", end="finished")),
        ("error_past_tick", error(11, "past_tick", tip_tick=YEAR1)),
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
        if not CONTROL_CODE_HI <= code <= CONTROL_CODE_LO or code not in CONTROL_CODES:
            problems.append(f"{reason}: code {code} outside {CONTROL_CODE_HI}..{CONTROL_CODE_LO}")
    if ERRORS["method_not_found"][1] != CONTROL_MESSAGE:
        problems.append("method_not_found does not carry the control socket's message")
    return problems


def _dispatch_cases() -> list[tuple[str, bytes, bool]]:
    """Planted frames from a controller: (name, bytes, on a finished host?)."""
    status = W.encode_frame(W.control_frame(W.request(1, "control.status")))

    def req(rid: int, method: str, params: Json | None = None) -> bytes:
        return bytes(W.encode_frame(W.control_frame(W.request(rid, method, params))))

    return [
        ("dispatch_status", status, False),
        ("dispatch_pace", req(2, "control.pace", {"band": "1y"}), False),
        ("dispatch_pace_same", req(3, "control.pace", {"band": "max"}), False),
        ("dispatch_pace_unknown_band", req(4, "control.pace", {"band": "1yr"}), False),
        (
            "dispatch_pace_float",
            bytes(
                W.encode_frame(
                    W.Frame(
                        W.TYPE_CONTROL,
                        b'{"id":5,"jsonrpc":"2.0","method":"control.pace","params":{"band":1.5}}',
                    )
                )
            ),
            False,
        ),
        ("dispatch_pace_extra_member", req(6, "control.pace", {"band": "1y", "x": 1}), False),
        ("dispatch_pause", req(7, "control.pause"), False),
        ("dispatch_resume", req(8, "control.resume"), False),
        ("dispatch_step_tick", req(9, "control.step", {"until": {"tick": STEP_TICK}}), False),
        ("dispatch_step_instant", req(10, "control.step", {"until": "instant"}), False),
        ("dispatch_step_past", req(11, "control.step", {"until": {"tick": YEAR1}}), False),
        ("dispatch_step_bad", req(12, "control.step", {"until": {"year": 2}}), False),
        ("dispatch_status_with_param", req(13, "control.status", {"x": 1}), False),
        ("dispatch_control_input", req(14, "control.input", {"verb_id": 3}), False),
        ("dispatch_control_fork", req(15, "control.fork", {"tick": 0}), False),
        ("dispatch_player_act", req(16, "player.act", {"verb_id": 3}), False),
        ("dispatch_timeline_fork", req(17, "timeline.fork", {"tick": 0}), False),
        ("dispatch_stream_subscribe", req(18, "stream.subscribe", {"from": "live"}), False),
        ("dispatch_pace_finished", req(19, "control.pace", {"band": "1y"}), True),
        ("dispatch_step_finished", req(20, "control.step", {"until": "instant"}), True),
        ("dispatch_status_finished", req(21, "control.status"), True),
        (
            "dispatch_notification",
            bytes(W.encode_frame(W.control_frame(W.notification("control.pace", {})))),
            False,
        ),
        ("dispatch_batch", bytes(W.encode_frame(W.Frame(W.TYPE_CONTROL, b"[]"))), False),
        ("dispatch_version_2", status[:4] + b"\x02\x00" + status[6:], False),
        (
            "dispatch_records_frame",
            bytes(W.encode_frame(W.records_frame(W.golden_records("A", 1)))),
            False,
        ),
        (
            "dispatch_oversize",
            status[:8]
            + (W.VIEWER_MAX_PAYLOAD + 1).to_bytes(4, "little")
            + b"\0" * (W.VIEWER_MAX_PAYLOAD + 1),
            False,
        ),
    ]


EXPECTED_REASONS = {
    "dispatch_status": "ok",
    "dispatch_pace": "ok",
    "dispatch_pace_same": "ok",
    "dispatch_pace_unknown_band": "invalid_params",
    "dispatch_pace_float": "invalid_params",
    "dispatch_pace_extra_member": "invalid_params",
    "dispatch_pause": "ok",
    "dispatch_resume": "ok",
    "dispatch_step_tick": "ok",
    "dispatch_step_instant": "ok",
    "dispatch_step_past": "past_tick",
    "dispatch_step_bad": "invalid_params",
    "dispatch_status_with_param": "invalid_params",
    "dispatch_control_input": "method_not_found",
    "dispatch_control_fork": "method_not_found",
    "dispatch_player_act": "method_not_found",
    "dispatch_timeline_fork": "method_not_found",
    "dispatch_stream_subscribe": "method_not_found",
    "dispatch_pace_finished": "run_finished",
    "dispatch_step_finished": "run_finished",
    "dispatch_status_finished": "ok",
    "dispatch_notification": "invalid_request",
    "dispatch_batch": "invalid_request",
    "dispatch_version_2": "unsupported_version",
    "dispatch_records_frame": "invalid_frame",
    "dispatch_oversize": "invalid_frame",
}
LOGGED = {
    "dispatch_pace",
    "dispatch_pace_same",
    "dispatch_pause",
    "dispatch_resume",
    "dispatch_step_tick",
    "dispatch_step_instant",
}


def dispatch_transcript() -> list[tuple[str, bytes, bytes, Host, Cell]]:
    """Each planted request, the host's reply bytes, the host after, and the cell before."""
    out = []
    for name, data, finished in _dispatch_cases():
        host = _fresh_host(finished=finished)
        before = dataclasses.replace(host.cell)
        replies = host.dispatch(host.connect(), data)
        out.append((name, data, b"".join(replies), host, before))
    return out


def _log_lines_for_commands(host: Host) -> int:
    """The run-log lines beyond the host's own (start, the start pace, finish or abort)."""
    return len(host.run_log) - 2 - (1 if host.end else 0)


def _dispatch_checks() -> list[str]:
    """Every planted request gets its reason; a refusal changes nothing; a command logs once."""
    problems = []
    for name, _, reply, host, before in dispatch_transcript():
        frame, used = W.decode_frame(reply)
        _, obj = W.decode_control(frame.payload)
        if used != len(reply):
            problems.append(f"{name}: more than one reply frame")
        if _reason(obj) != EXPECTED_REASONS[name]:
            problems.append(f"{name}: {_reason(obj)}, expected {EXPECTED_REASONS[name]}")
        expected_lines = 1 if name in LOGGED else 0
        if _log_lines_for_commands(host) != expected_lines:
            problems.append(f"{name}: {_log_lines_for_commands(host)} run-log lines")
        if _reason(obj) != "ok" and host.cell != before:
            problems.append(f"{name}: a refusal changed the cell")
        if host.engine.journal != Engine().journal:
            problems.append(f"{name}: the journal changed under a pace command")
    if _reason(_ask(_fresh_host(), 1, "x.y")) != "method_not_found":
        problems.append("an unknown method was not refused")
    return problems


def _result(reply: Json) -> Json:
    res = reply.get("result")
    if not isinstance(res, dict):
        raise AssertionError(f"not a result: {reply!r}")
    return res


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


def _slept(run: Run, name: str) -> list[str]:
    """The governor really slept at a band, and never at max."""
    problems = []
    rel = run.host.released
    if name == "headless-max" and any(w != 0 for w, _, _ in rel):
        problems.append("the clock moved at max")
    if name == "band-1y":
        years = [w for w, _, k in rel if k % INSTANTS_PER_YEAR == 0]
        if any(b - a < S for a, b in itertools.pairwise(years)) or years[0] < S:
            problems.append("a sim-year at 1y took less than a wall-second")
    if name == "band-10y" and rel[-1][0] < HORIZON_YEARS * S // DECADE:
        problems.append("the run at 10y was faster than the band")
    return problems


def _replies(run: Run, method: str) -> list[tuple[int, Json]]:
    return [(c.at_ns, r) for c, r in run.replies if c.method == method]


def _held(run: Run) -> list[str]:
    """A pause held the tip and wrote nothing; the resume owed no time (control_v1.md §4.3)."""
    problems = []
    statuses = _replies(run, "control.status")
    one_y = rate("1y")
    assert one_y is not None
    pairs = zip(_replies(run, "control.pause"), _replies(run, "control.resume"), strict=False)
    for (p_at, _), (r_at, _) in pairs:
        if any(p_at < w < r_at for w, _, _ in run.host.released):
            problems.append("an instant ran while paused")
        seqs = set()
        for at, st in statuses:
            if p_at <= at <= r_at:
                tip = _result(st)["tip"]
                assert isinstance(tip, dict)
                seqs.add(int(str(tip["next_seq"])))
        if len(seqs) > 1:
            problems.append("a pause wrote a record: next_seq moved while paused")
        after = [w for w, _, _ in run.host.released if w >= r_at]
        if after and after[0] < r_at + ceil_div(INSTANT_MINUTES * S, one_y):
            problems.append("paused time was owed: an instant ran at once after the resume")
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
        want = target - target % INSTANT_MINUTES
        hit = next((tip for w, tip in reached if w >= at), None)
        if hit is None or hit == "~":
            problems.append(f"the step to {target} never paused")
        elif int(hit) > want:
            problems.append(f"the step overshot its target: paused at {hit}, not {want}")
        elif int(hit) < want:
            problems.append(f"the step stopped short: paused at {hit}, not {want}")
    return problems


def _invariance_checks(control: str | None = None) -> list[str]:
    """Pace invariance over every scenario, plus the sleep, hold and step assertions."""
    ref = simulate(SCENARIOS[0]).host.engine
    problems = []
    for sc in SCENARIOS:
        run = simulate(sc, control)
        problems += _same_history(run, ref, sc.name)
        problems += _slept(run, sc.name)
        if sc.name == "pause-resume":
            problems += _held(run)
        if sc.name == "step":
            problems += _stepped(run)
        if any(_reason(r) != "ok" for _, r in run.replies):
            problems.append(f"{sc.name}: a scripted command was refused")
    return problems


def _is_command_line(line: str) -> bool:
    return line.startswith("pace ") and " host.start " not in line and " step.reached " not in line


def _run_log_checks() -> list[str]:
    """One pace line per accepted command, none per refusal or read; the host lines."""
    problems = []
    for sc in SCENARIOS:
        run = simulate(sc)
        log = run.host.run_log
        commands = [c for c, r in run.replies if c.method in COMMANDS and _reason(r) == "ok"]
        pace_lines = [ln for ln in log if _is_command_line(ln)]
        if len(pace_lines) != len(commands):
            problems.append(f"{sc.name}: {len(pace_lines)} pace lines for {len(commands)} commands")
        for ln, c in zip(pace_lines, commands, strict=False):
            if ln.split()[5].split(":")[0] != c.method:
                problems.append(f"{sc.name}: {ln!r} is not {c.method}")
        heads = (log[0].split()[2], log[1].split()[5], log[-1].split()[2])
        if heads != ("start", "host.start", "finish"):
            problems.append(f"{sc.name}: the host lines are not start, pace, ..., finish")
        if any(len(ln.split()) not in {5, 7} for ln in log):
            problems.append(f"{sc.name}: a run-log line has the wrong field count")
    return problems


def self_test(control: str | None = None) -> list[str]:
    """Run every check; return the report lines (the last says PASS or FAIL)."""
    if control == "pace-on-stream":
        W.METHODS = W.METHODS | {"control.pace"}  # type: ignore[attr-defined]  # the stream table gains a command
    invariance: Check = (
        "pace invariance: six scenarios reproduce the headless run's bytes, digests, keyframes, "
        "ladder and journal; the governor slept; a pause held the tip; a step stopped after "
        "Bookkeeping of its target",
        lambda: _invariance_checks(control=control),
    )
    checks: list[Check] = [
        (
            "the method table: five methods, disjoint from the stream's; reserved names refused",
            lambda: _method_table_checks(control),
        ),
        ("every message: canonical, decoded, classified; the codes in range", _message_checks),
        (
            "the host's dispatcher: answers, refusals that change nothing, one log line a command",
            _dispatch_checks,
        ),
        ("host: the cap, status invariants, idempotence", _host_checks),
        ("the governor's rule: rates and deadlines in integers", _governor_checks),
        invariance,
        ("the run log: one pace line per accepted command, the host lines", _run_log_checks),
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
def render_golden() -> str:
    """The golden file's full text."""
    header = [
        "# World Stream control channel v1: golden frames, dispatcher answers, paced scenarios.",
        "# GENERATED by specs/stream/selftest_control_v1.py; do not edit. Contract: control_v1.md.",
        "# `frame <name> <hex>`: one frame, header and payload, then `control <canonical json>`;",
        "#   rh-protocol encodes the decoded form and must produce the hex, and back.",
        "# `dispatch <name> <request hex> <reply hex>`: a frame from a controller and the host's",
        "#   answer on a fresh connection to a host whose engine ran one sim-year at max on a",
        "#   clock at 0 (`_finished` cases: the whole horizon, then finished; control_v1.md §4-5).",
        "# `scenario <name>` then `clock <wall_ns> <tick> <k>` per instant released (the fake",
        "#   wall clock when instant k ran; control_v1.md §3.3) and `log <run-log line>` lines",
        "#   (control_v1.md §8.1); rh-golden's governor test reproduces the clock lines.",
        "# `invariance <sha256 of the channel-A lines> <keyframe ticks> <journal head> <ladder>`:",
        "#   what every scenario produced, equal to the headless run's (control_v1.md §0 fact 2).",
    ]
    body: list[str] = []
    for name, obj in _messages():
        body.append(f"frame {name} {W.encode_frame(W.control_frame(obj)).hex()}")
        body.append(f"control {W.cjson(obj).decode()}")
    for name, data, reply, _, _ in dispatch_transcript():
        body.append(f"dispatch {name} {data.hex()} {reply.hex()}")
    ref: Engine | None = None
    for sc in SCENARIOS:
        run = simulate(sc)
        body.append(f"scenario {sc.name}")
        body.extend(f"clock {w} {t} {k}" for w, t, k in run.host.released)
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
