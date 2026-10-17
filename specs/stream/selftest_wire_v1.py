"""Reference of the World Stream wire format v1 (IF-0 live edge; SPR-026, SPR-027).

The contract is ``specs/stream/wire_v1.md``: a 12-byte frame header, CONTROL frames carrying one
JSON-RPC 2.0 object in canonical JSON, RECORDS frames carrying canonical lines
(``specs/stream/canonical_v0.md``), the three ``stream.*`` methods, the notifications, the error
codes, and the live-edge rule by which a viewer that falls behind the host's in-memory window reads
sealed segments and comes back with no gap and no duplicate, by ``seq``. This file is the
independent reference ``rh-protocol`` (SPR-026) must agree with, as ``canonical_v0.py`` is for
``rh-stream``:

- :func:`encode_frame` and :func:`decode_frame` are the framing; :func:`encode_records` and
  :func:`decode_records` the RECORDS payload; :func:`cjson` the canonical JSON; the
  :func:`request`, :func:`response`, :func:`error` and :func:`notification` builders and
  :func:`decode_control` the control plane, with the refusals of wire_v1.md §2, §3 and §4;
- :class:`Catchup` is the viewer's pure state machine of wire_v1.md §6.3;
- :class:`Writer` and :class:`Host` simulate a channel's writer (appends, seals, a bounded window)
  and its socket server; :class:`Viewer` runs fast, slow, crawling, stalled, reconnecting and
  jumping cursors over them, and the self-test asserts gap-free, duplicate-free delivery in
  ``seq`` order with per-year digests equal to the sealed ones, under several seeds;
- the golden file ``specs/stream/golden/wire_v1.txt`` holds one frame of every message, the
  frames that must be refused, and a ``Catchup`` transcript; ``rh-protocol``'s conformance test
  reproduces every frame bit for bit and every refusal.

Usage:  python -I specs/stream/selftest_wire_v1.py            (self-test, then rewrite the golden)
        python -I specs/stream/selftest_wire_v1.py --check    (self-test; exit 1 if the golden
                                                           differs)
        python -I specs/stream/selftest_wire_v1.py --check --golden PATH   (check another file;
                                                           the positive control uses a stale copy)
        python -I specs/stream/selftest_wire_v1.py --control window-gap   (a host that drops a
                                                           year from memory before its seal is on
                                                           disk: the live-edge check must FAIL)
Exit:   0 ok, 1 stale golden file, 2 the self-test failed.
Standard library only (GOV-003 §3); the canonical lines come from canonical_v0.py, loaded by path.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import random
import struct
import sys
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve().parent
GOLDEN = HERE / "golden" / "wire_v1.txt"
CANONICAL_REF = HERE / "canonical_v0.py"
EXIT_OK = 0
EXIT_STALE = 1
EXIT_SELFTEST = 2

# ---------------------------------------------------------------------------------------- the wire
WIRE_FORMAT = "rehoboam/stream/wire/v1"
MAGIC = b"RHWS"
VERSION = 1
TYPE_CONTROL = 1
TYPE_RECORDS = 2
FRAME_TYPES = frozenset({TYPE_CONTROL, TYPE_RECORDS})
FLAGS_V1 = 0
HEADER = struct.Struct("<4sHBBI")  # magic, version, type, flags, length
HEADER_LEN = HEADER.size
MAX_PAYLOAD = 16 * 1024 * 1024
SUBHEADER = struct.Struct("<IQIB3s")  # timeline_id, first_seq, count, channel, reserved
SUBHEADER_LEN = SUBHEADER.size
RESERVED = b"\0\0\0"
CHANNEL_BYTE = {"A": 0x41, "C": 0x43}
BYTE_CHANNEL = {v: k for k, v in CHANNEL_BYTE.items()}
LF = b"\n"
U32_MAX = (1 << 32) - 1
U64_MAX = (1 << 64) - 1
I64_MIN = -(1 << 63)
ASCII_PRINTABLE = range(0x20, 0x7F)
JSONRPC = "2.0"
METHODS = frozenset({"stream.status", "stream.subscribe", "stream.unsubscribe"})
NOTIFICATIONS = frozenset({"stream.sealed", "stream.behind", "stream.end"})
BANDS = frozenset({"live", "1h", "1d", "1w", "1mo", "1y", "10y", "max"})
STATES = frozenset({"running", "paused", "finished"})
READ_ONLY_MESSAGE = (
    "this socket carries the stream only and accepts no input; inputs go through the input journal"
)
# wire_v1.md §4.4: reason -> (code, message).
ERRORS: dict[str, tuple[int, str]] = {
    "parse_error": (-32700, "the payload is not JSON"),
    "invalid_request": (-32600, "not a JSON-RPC 2.0 request object"),
    "method_not_found": (-32601, READ_ONLY_MESSAGE),
    "invalid_params": (-32602, "a parameter is missing, of the wrong type or unknown"),
    "unsupported_version": (-32110, "the frame's version is not one this host speaks"),
    "invalid_frame": (-32111, "a bad magic, type, flag, length, sub-header or record line"),
    "behind_live_edge": (
        -32112,
        "the requested seq is below the live window; read sealed segments",
    ),
    "future_seq": (-32113, "the requested seq is above the tip"),
    "already_subscribed": (-32114, "this connection already holds a subscription"),
    "not_subscribed": (-32115, "this connection holds no subscription"),
    "too_many_viewers": (-32116, "the host's connection cap is reached"),
}
CODE_REASON = {code: reason for reason, (code, _) in ERRORS.items()}
Json = dict[str, object]


def _load(path: Path) -> ModuleType:
    """Import a module by path (``python -I`` adds no directory to ``sys.path``)."""
    spec = importlib.util.spec_from_file_location(path.stem, path)
    if spec is None or spec.loader is None:
        raise ImportError(str(path))
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module
    spec.loader.exec_module(module)
    return module


_CANON = _load(CANONICAL_REF)


class WireError(ValueError):
    """A frame, payload or message that the wire refuses (wire_v1.md §2, §3, §4)."""


class IncompleteFrameError(WireError):
    """The buffer ends before the frame does; read more and decode again."""


# ---------------------------------------------------------------------------------------- framing
@dataclass(frozen=True)
class Frame:
    """One frame: its type and payload (the header is derived)."""

    ftype: int
    payload: bytes


def encode_frame(frame: Frame) -> bytes:
    """The bytes of ``frame``: the 12-byte header, then the payload."""
    if frame.ftype not in FRAME_TYPES:
        raise WireError(f"frame type {frame.ftype} is not CONTROL (1) or RECORDS (2)")
    if len(frame.payload) > MAX_PAYLOAD:
        raise WireError(f"payload of {len(frame.payload)} bytes exceeds {MAX_PAYLOAD}")
    return HEADER.pack(MAGIC, VERSION, frame.ftype, FLAGS_V1, len(frame.payload)) + frame.payload


def decode_header(buf: bytes) -> tuple[int, int]:
    """Return (type, length) of the header at the start of ``buf``, refusing a bad one."""
    if len(buf) < HEADER_LEN:
        raise IncompleteFrameError(f"{len(buf)} bytes, a header needs {HEADER_LEN}")
    magic, version, ftype, flags, length = HEADER.unpack_from(buf)
    if magic != MAGIC:
        raise WireError(f"magic {magic!r} is not {MAGIC!r}")
    if version != VERSION:
        raise WireError(f"unsupported_version: {version}, this reference speaks {VERSION}")
    if ftype not in FRAME_TYPES:
        raise WireError(f"frame type {ftype} is not CONTROL (1) or RECORDS (2)")
    if flags != FLAGS_V1:
        raise WireError(f"flags {flags:#x}: every flag is reserved in v1")
    if length > MAX_PAYLOAD:
        raise WireError(f"length {length} exceeds {MAX_PAYLOAD}")
    return ftype, length


def decode_frame(buf: bytes) -> tuple[Frame, int]:
    """The frame at the start of ``buf`` and the bytes it used, or :class:`IncompleteFrameError`."""
    ftype, length = decode_header(buf)
    end = HEADER_LEN + length
    if len(buf) < end:
        raise IncompleteFrameError(f"{len(buf)} bytes, the frame needs {end}")
    return Frame(ftype, bytes(buf[HEADER_LEN:end])), end


# ---------------------------------------------------------------------------------------- records
@dataclass(frozen=True)
class Records:
    """A RECORDS payload: contiguous canonical lines of one channel from ``first_seq``."""

    timeline_id: int
    channel: str
    first_seq: int
    lines: tuple[str, ...]


def _check_line(line: str, rec_channel: str, timeline_id: int, seq: int) -> None:
    """Refuse a line that is not canonical, of another channel, timeline or seq."""
    if not line.isascii():
        raise WireError(f"seq {seq}: a record line is not ASCII")
    try:
        rec = _CANON.parse(line)
    except _CANON.CanonicalError as e:
        raise WireError(f"seq {seq}: not a canonical line: {e}") from e
    if _CANON.CHANNEL[rec.kind] != rec_channel:
        raise WireError(f"seq {seq}: a {rec.kind} record on channel {rec_channel}")
    if rec["timeline_id"] != timeline_id or rec["seq"] != seq:
        raise WireError(
            f"seq {seq}: the line's header says timeline {rec['timeline_id']} seq {rec['seq']}"
        )


def encode_records(records: Records) -> bytes:
    """The payload of a RECORDS frame, checked as the receiver checks it."""
    if not records.lines:
        raise WireError("a RECORDS frame holds at least one record (count is 0)")
    if records.channel not in CHANNEL_BYTE:
        raise WireError(f"channel {records.channel!r} is not A or C")
    if not 0 <= records.timeline_id <= U32_MAX or not 0 <= records.first_seq <= U64_MAX:
        raise WireError("timeline_id is u32 and first_seq is u64")
    if records.first_seq + len(records.lines) - 1 > U64_MAX:
        raise WireError("the frame's last seq exceeds u64")
    for i, line in enumerate(records.lines):
        _check_line(line, records.channel, records.timeline_id, records.first_seq + i)
    head = SUBHEADER.pack(
        records.timeline_id,
        records.first_seq,
        len(records.lines),
        CHANNEL_BYTE[records.channel],
        RESERVED,
    )
    return head + b"".join(line.encode("ascii") + LF for line in records.lines)


def decode_records(payload: bytes) -> Records:
    """The records of a RECORDS payload, or :class:`WireError`."""
    if len(payload) < SUBHEADER_LEN:
        raise WireError(f"a RECORDS payload needs {SUBHEADER_LEN} bytes of sub-header")
    timeline_id, first_seq, count, channel_byte, reserved = SUBHEADER.unpack_from(payload)
    if reserved != RESERVED:
        raise WireError("the reserved bytes are not zero")
    if channel_byte not in BYTE_CHANNEL:
        raise WireError(f"channel byte {channel_byte:#x} is not 'A' or 'C'")
    if count == 0:
        raise WireError("count is 0")
    body = payload[SUBHEADER_LEN:]
    if not body.endswith(LF):
        raise WireError("the body does not end with LF")
    raw = body[:-1].split(LF)
    if len(raw) != count:
        raise WireError(f"count says {count} records, the body holds {len(raw)}")
    lines = tuple(part.decode("ascii", errors="strict") for part in raw)
    records = Records(timeline_id, BYTE_CHANNEL[channel_byte], first_seq, lines)
    encode_records(records)  # the same checks, line by line
    return records


# ---------------------------------------------------------------------------------------- control
def _check_scalar(value: object, where: str) -> bool:
    """Check a JSON scalar; return False for a container (checked by the caller)."""
    if isinstance(value, bool) or value is None:
        return True
    if isinstance(value, float):
        raise WireError(f"{where}: a float ({value!r}); the wire carries integers only")
    if isinstance(value, int):
        if not I64_MIN <= value <= U64_MAX:
            raise WireError(f"{where}: {value} is outside i64..u64")
        return True
    if isinstance(value, str):
        if any(ord(ch) not in ASCII_PRINTABLE for ch in value):
            raise WireError(f"{where}: a string outside printable ASCII ({value!r})")
        return True
    return False


def _check_json(value: object, where: str) -> None:
    """Refuse a float, a non-ASCII or non-printable string, anywhere in ``value``."""
    if _check_scalar(value, where):
        return
    if isinstance(value, dict):
        for k, v in value.items():
            if not isinstance(k, str):
                raise WireError(f"{where}: a non-string key")
            _check_json(v, f"{where}.{k}")
    elif isinstance(value, (list, tuple)):
        for i, v in enumerate(value):
            _check_json(v, f"{where}[{i}]")
    else:
        raise WireError(f"{where}: {type(value).__name__} is not a JSON value")


def cjson(value: object) -> bytes:
    """The canonical JSON of ``value`` (wire_v1.md §3.1), as UTF-8 bytes."""
    _check_json(value, "value")
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()


def _no_float(text: str) -> float:
    raise WireError(f"a float ({text}); the wire carries integers only")


def _no_repeat(pairs: list[tuple[str, object]]) -> dict[str, object]:
    keys = [k for k, _ in pairs]
    if len(set(keys)) != len(keys):
        raise WireError("a repeated key")
    return dict(pairs)


def parse_json(payload: bytes) -> object:
    """Parse a control payload: JSON with no float, no repeated key, ASCII strings."""
    try:
        value: object = json.loads(
            payload.decode("utf-8"), parse_float=_no_float, object_pairs_hook=_no_repeat
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as e:
        raise WireError(f"parse_error: {e}") from e
    _check_json(value, "payload")
    return value


def request(rid: int | str, method: str, params: Json | None = None) -> Json:
    """A JSON-RPC request object."""
    obj: Json = {"id": rid, "jsonrpc": JSONRPC, "method": method}
    if params is not None:
        obj["params"] = params
    return obj


def response(rid: int | str | None, result: Json) -> Json:
    """A JSON-RPC success response."""
    return {"id": rid, "jsonrpc": JSONRPC, "result": result}


def error(rid: int | str | None, reason: str, **data: object) -> Json:
    """A JSON-RPC error response for ``reason`` (wire_v1.md §4.4) with extra ``data`` fields."""
    code, message = ERRORS[reason]
    return {
        "error": {"code": code, "data": {"reason": reason, **data}, "message": message},
        "id": rid,
        "jsonrpc": JSONRPC,
    }


def notification(method: str, params: Json) -> Json:
    """A JSON-RPC notification (no id)."""
    return {"jsonrpc": JSONRPC, "method": method, "params": params}


def control_frame(obj: Json) -> Frame:
    """A CONTROL frame carrying ``obj`` in canonical JSON."""
    return Frame(TYPE_CONTROL, cjson(obj))


def records_frame(records: Records) -> Frame:
    """A RECORDS frame carrying ``records``."""
    return Frame(TYPE_RECORDS, encode_records(records))


def _is_id(value: object) -> bool:
    return value is None or (isinstance(value, (int, str)) and not isinstance(value, bool))


def _classify_call(obj: Json) -> str:
    if not isinstance(obj["method"], str):
        raise WireError("invalid_request: method is not a string")
    if "params" in obj and not isinstance(obj["params"], dict):
        raise WireError("invalid_params: params is not an object")
    if "id" not in obj:
        return "notification"
    if obj["id"] is None or not _is_id(obj["id"]):
        raise WireError("invalid_request: a request id is an integer or a string")
    return "request"


def _classify_reply(obj: Json) -> str:
    if "id" not in obj or not _is_id(obj["id"]):
        raise WireError("invalid_request: a response carries an id")
    if ("result" in obj) == ("error" in obj):
        raise WireError("invalid_request: a response has exactly one of result and error")
    if "error" not in obj:
        return "response"
    err = obj["error"]
    if not isinstance(err, dict) or not isinstance(err.get("code"), int):
        raise WireError("invalid_request: error without an integer code")
    data = err.get("data")
    if not isinstance(data, dict) or CODE_REASON.get(err["code"]) != data.get("reason"):
        raise WireError("invalid_request: error.data.reason does not match error.code")
    return "error"


def classify(obj: object) -> str:
    """Return ``request``, ``response``, ``error`` or ``notification``; else :class:`WireError`."""
    if not isinstance(obj, dict):
        raise WireError("invalid_request: not a JSON object (a batch is refused)")
    if obj.get("jsonrpc") != JSONRPC:
        raise WireError("invalid_request: jsonrpc is not '2.0'")
    return _classify_call(obj) if "method" in obj else _classify_reply(obj)


def decode_control(payload: bytes) -> tuple[str, Json]:
    """The class and object of a CONTROL payload, or :class:`WireError`."""
    obj = parse_json(payload)
    kind = classify(obj)
    assert isinstance(obj, dict)
    return kind, obj


def _u64(value: object, where: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= U64_MAX:
        raise WireError(f"invalid_params: {where} is not a u64")
    return value


def subscribe_from(params: object) -> int | None:
    """The seq a ``stream.subscribe`` asks for, or None for ``"live"``; refuses anything else."""
    if not isinstance(params, dict) or set(params) != {"from"}:
        raise WireError("invalid_params: params are exactly {from}")
    frm = params["from"]
    if frm == "live":
        return None
    if isinstance(frm, dict) and set(frm) == {"seq"}:
        return _u64(frm["seq"], "from.seq")
    raise WireError("invalid_params: from is {seq: u64} or 'live'")


def check_status(result: object) -> Json:
    """Check a ``stream.status`` result's shape and invariants (wire_v1.md §4.1)."""
    if not isinstance(result, dict):
        raise WireError("status: not an object")
    for key in ("wire", "channel", "timeline_id", "tip", "sealed", "live", "history", "calendar"):
        if key not in result:
            raise WireError(f"status: missing {key}")
    wire, live, sealed, tip = result["wire"], result["live"], result["sealed"], result["tip"]
    if wire != {"format": WIRE_FORMAT, "version": VERSION}:
        raise WireError("status: wire is not this format and version")
    if not (isinstance(live, dict) and isinstance(sealed, dict) and isinstance(tip, dict)):
        raise WireError("status: live, sealed and tip are objects")
    w0, w1 = _u64(live["first_seq"], "live.first_seq"), _u64(live["next_seq"], "live.next_seq")
    if w0 > w1 or _u64(tip["next_seq"], "tip.next_seq") != w1:
        raise WireError("status: live.first_seq <= live.next_seq = tip.next_seq")
    s = sealed["last_seq"]
    if s is None and w0 != 0:
        raise WireError("status: live.first_seq is 0 before the first seal (I1)")
    if s is not None and _u64(s, "sealed.last_seq") + 1 < w0:
        raise WireError("status: sealed.last_seq + 1 >= live.first_seq (I1)")
    return result


# ---------------------------------------------------------------------------------------- catch-up
@dataclass(frozen=True)
class Event:
    """What the viewer's loop hears: a response, a notification, a frame or a closed socket."""

    kind: str  # subscribed | behind | records | sealed | end | closed | read_sealed_done
    first_seq: int = 0
    count: int = 0
    sealed_last_seq: int = 0
    live_first_seq: int = 0


@dataclass(frozen=True)
class Action:
    """What the viewer's loop does next (wire_v1.md §6.3)."""

    kind: str  # subscribe | read_sealed | deliver | ignore | done | error
    a: int = 0
    b: int = 0
    detail: str = ""


class Catchup:
    """The pure catch-up state machine of a cursor: subscribe, go to disk when behind, resume."""

    def __init__(self, cursor: int = 0) -> None:
        """Start a loop whose next seq is ``cursor``."""
        self.cursor = cursor
        self.state = "idle"
        self.read_to = 0  # the last seq a sealed read must reach

    def start(self) -> Action:
        """The first action: subscribe from the cursor."""
        self.state = "subscribing"
        return Action("subscribe", self.cursor)

    def next(self, event: Event) -> Action:
        """Feed one event; return the action it calls for."""
        if self.state == "done":
            return Action("error", detail="an event after stream.end")
        handlers: dict[str, Callable[[Event], Action]] = {
            "closed": self._closed,
            "subscribed": self._subscribed,
            "behind": self._behind,
            "records": self._records,
            "read_sealed_done": self._read_done,
            "sealed": lambda _: Action("ignore"),
            "end": self._end,
        }
        handler = handlers.get(event.kind)
        return handler(event) if handler else Action("error", detail=f"unknown event {event.kind}")

    def _closed(self, _: Event) -> Action:
        self.state = "subscribing"
        return Action("subscribe", self.cursor)

    def _end(self, _: Event) -> Action:
        if self.state != "live":
            return Action("error", detail="stream.end while not subscribed")
        self.state = "done"
        return Action("done", self.cursor)

    def _subscribed(self, event: Event) -> Action:
        if self.state != "subscribing":
            return Action("error", detail="a subscribe response while not subscribing")
        if event.first_seq != self.cursor:
            return Action("error", detail=f"first_seq {event.first_seq} is not the cursor")
        self.state = "live"
        return Action("deliver", self.cursor, 0)

    def _behind(self, event: Event) -> Action:
        if self.state not in ("subscribing", "live"):
            return Action("error", detail="behind while not subscribed or subscribing")
        s, w0 = event.sealed_last_seq, event.live_first_seq
        if s + 1 < w0:
            return Action("error", detail=f"I1 broken: sealed_last_seq {s} + 1 < live_first {w0}")
        if s < self.cursor:
            return Action("error", detail=f"behind with sealed_last_seq {s} below cursor")
        self.state = "reading_sealed"
        self.read_to = s
        return Action("read_sealed", self.cursor, s)

    def _records(self, event: Event) -> Action:
        if self.state != "live":
            return Action("error", detail="records while not subscribed")
        if event.first_seq != self.cursor or event.count == 0:
            return Action(
                "error", detail=f"frame first_seq {event.first_seq} is not the cursor {self.cursor}"
            )
        self.cursor += event.count
        return Action("deliver", event.first_seq, event.count)

    def _read_done(self, event: Event) -> Action:
        if self.state != "reading_sealed" or event.first_seq != self.cursor:
            return Action("error", detail="a sealed read that does not start at the cursor")
        if event.first_seq + event.count - 1 != self.read_to:
            return Action("error", detail=f"a sealed read of {event.count} seqs to {self.read_to}")
        self.cursor = event.first_seq + event.count
        self.state = "subscribing"
        return Action("subscribe", self.cursor)


# ---------------------------------------------------------------------------------------- the host
MINUTES_PER_YEAR = 518_400
TICKS_PER_YEAR = 12  # the simulation's instants per sim-year (a coarse month)
TICK_MINUTES = MINUTES_PER_YEAR // TICKS_PER_YEAR
TIMELINE = 0
MAX_APPENDS_PER_TICK = 5


@dataclass(frozen=True)
class Segment:
    """A sealed sim-year on the model's disk."""

    sim_year: int
    first_seq: int
    last_seq: int
    lines: tuple[str, ...]
    digest: str


@dataclass
class Writer:
    """One channel's writer: appends to a window, seals sim-years to disk (wire_v1.md §6.1)."""

    channel: str
    retain_sealed_years: int = 1  # wire_v1.md §6.1: the last sealed year stays to the next seal
    break_i1: bool = False  # the window-gap control: drop a year before its seal is on disk
    disk: dict[int, Segment] = field(default_factory=dict)
    window: list[tuple[int, str]] = field(default_factory=list)  # (seq, line), contiguous
    next_seq: int = 0
    first_seq: int = 0
    tip_tick: int | None = None
    sealed_last_seq: int | None = None
    sealed_last_year: int | None = None
    year_first_seq: int = 0
    current_year: int = 0
    finished: bool = False
    on_seal: list[Callable[[Segment], None]] = field(default_factory=list)

    def append(self, line: str) -> int:
        """Append one canonical line at the next seq; never waits for anything."""
        seq = self.next_seq
        self.window.append((seq, line))
        self.next_seq += 1
        self.tip_tick = int(_CANON.parse(line)["tick"])
        return seq

    def seal_year(self) -> Segment:
        """Seal the current sim-year: write it to disk, then drop retained years from memory."""
        lines = tuple(line for seq, line in self.window if seq >= self.year_first_seq)
        seg = Segment(
            self.current_year,
            self.year_first_seq,
            self.next_seq - 1,
            lines,
            str(_CANON.digest(lines).hex()),
        )
        if self.break_i1:
            # The control: memory is dropped as if sealed, but the disk misses the last record.
            short = lines[:-1]
            digest = str(_CANON.digest(short).hex())
            seg = Segment(seg.sim_year, seg.first_seq, seg.last_seq - 1, short, digest)
        self.disk[seg.sim_year] = seg
        self.sealed_last_seq, self.sealed_last_year = seg.last_seq, seg.sim_year
        self.year_first_seq = self.next_seq
        self.current_year += 1
        self._drop_retained()
        for hook in self.on_seal:
            hook(seg)
        return seg

    def _drop_retained(self) -> None:
        """Keep the unsealed year and ``retain_sealed_years`` sealed ones; drop the rest."""
        keep_from_year = self.current_year - self.retain_sealed_years
        kept = [seg.first_seq for seg in self.disk.values() if seg.sim_year >= keep_from_year]
        keep_seq = min([*kept, self.year_first_seq])
        self.window = [(seq, line) for seq, line in self.window if seq >= keep_seq]
        self.first_seq = keep_seq

    def lines(self, start: int, limit: int) -> tuple[str, ...]:
        """The window's lines from ``start``, at most ``limit``; ``start`` must be in the window."""
        offset = start - self.first_seq
        return tuple(line for _, line in self.window[offset : offset + limit])

    def read_sealed(self, lo: int, hi: int) -> tuple[str, ...]:
        """The sealed lines with ``lo <= seq <= hi``, in order, from the model's disk."""
        out: list[str] = []
        for seg in sorted(self.disk.values(), key=lambda s: s.first_seq):
            if seg.last_seq < lo or seg.first_seq > hi:
                continue
            a, b = max(lo, seg.first_seq) - seg.first_seq, min(hi, seg.last_seq) - seg.first_seq
            out.extend(seg.lines[a : b + 1])
        return tuple(out)


@dataclass
class Connection:
    """One viewer's connection as the host sees it: a cursor and what is owed to it."""

    cursor: int | None = None  # None: no subscription
    sealed_pending: list[Segment] = field(default_factory=list)
    closed: bool = False


@dataclass
class Host:
    """A channel's socket server over its writer: subscribe, status, serve (wire_v1.md §6.2)."""

    writer: Writer
    batch: int = 64
    connections: list[Connection] = field(default_factory=list)

    def __post_init__(self) -> None:
        """Register the seal hook that owes ``stream.sealed`` to every subscription."""
        self.writer.on_seal.append(self._sealed)

    def connect(self) -> Connection:
        """Accept a connection."""
        conn = Connection()
        self.connections.append(conn)
        return conn

    def close(self, conn: Connection) -> None:
        """The viewer closed: forget the subscription; nothing else changes."""
        conn.closed, conn.cursor = True, None
        self.connections.remove(conn)

    def status(self) -> Json:
        """The ``stream.status`` result (wire_v1.md §4.1)."""
        w = self.writer
        is_a = w.channel == "A"
        return {
            "calendar": {"minutes_per_year": MINUTES_PER_YEAR},
            "canonical_format": _CANON.FORMAT,
            "channel": w.channel,
            "history": {
                "engine_build": "reference",
                "manifest_hash": "00" * 16,
                "registry_version": "0.0.0",
                "world_seed": 0,
            },
            "live": {"first_seq": w.first_seq, "next_seq": w.next_seq},
            "pace": {"band": "max", "state": "finished" if w.finished else "running"}
            if is_a
            else None,
            "record_format": f"rehoboam/stream/channel-{w.channel.lower()}/v0",
            "sealed": {"last_seq": w.sealed_last_seq, "last_year": w.sealed_last_year},
            "sifted_through": None
            if is_a
            else {"a_seq": w.sealed_last_seq, "a_year": w.sealed_last_year},
            "timeline_id": TIMELINE,
            "tip": {"next_seq": w.next_seq, "tick": w.tip_tick},
            "wire": {"format": WIRE_FORMAT, "version": VERSION},
        }

    def subscribe(self, conn: Connection, rid: int, frm: int | None) -> Json:
        """Answer ``stream.subscribe`` (wire_v1.md §4.2, §6.2)."""
        w = self.writer
        if conn.cursor is not None:
            return error(rid, "already_subscribed")
        n = w.next_seq if frm is None else frm
        if n < w.first_seq:
            return error(rid, "behind_live_edge", **self._behind_data())
        if n > w.next_seq:
            return error(rid, "future_seq", next_seq=w.next_seq)
        conn.cursor = n
        return response(rid, {"first_seq": n})

    def unsubscribe(self, conn: Connection, rid: int) -> Json:
        """Answer ``stream.unsubscribe``."""
        if conn.cursor is None:
            return error(rid, "not_subscribed")
        n, conn.cursor, conn.sealed_pending = conn.cursor, None, []
        return response(rid, {"next_seq": n})

    def _behind_data(self) -> Json:
        w = self.writer
        return {
            "live_first_seq": w.first_seq,
            "sealed_last_seq": w.sealed_last_seq,
            "sealed_last_year": w.sealed_last_year,
        }

    def _sealed(self, seg: Segment) -> None:
        for conn in self.connections:
            if conn.cursor is not None:
                conn.sealed_pending.append(seg)

    def _notify(self, method: str, params: Json) -> Frame:
        base: Json = {"channel": self.writer.channel, "timeline_id": TIMELINE}
        return control_frame(notification(method, {**base, **params}))

    def serve(self, conn: Connection, budget: int) -> list[Frame]:
        """What the serving thread writes next for ``conn``, when the viewer reads ``budget``."""
        if conn.cursor is None:
            return []
        w, c = self.writer, conn.cursor
        if c < w.first_seq:
            conn.cursor, conn.sealed_pending = None, []
            return [self._notify("stream.behind", {"next_seq": c, **self._behind_data()})]
        passed = [seg for seg in conn.sealed_pending if seg.last_seq < c]
        conn.sealed_pending = [seg for seg in conn.sealed_pending if seg.last_seq >= c]
        frames = [self._notify("stream.sealed", self._sealed_params(seg)) for seg in passed]
        lines = w.lines(c, min(budget, self.batch))
        if lines:
            frames.append(records_frame(Records(TIMELINE, w.channel, c, lines)))
            conn.cursor = c + len(lines)
        elif w.finished:
            conn.cursor = None
            frames.append(self._notify("stream.end", {"next_seq": c}))
        return frames

    @staticmethod
    def _sealed_params(seg: Segment) -> Json:
        return {
            "canonical_digest": seg.digest,
            "first_seq": seg.first_seq,
            "last_seq": seg.last_seq,
            "sim_year": seg.sim_year,
        }


# ---------------------------------------------------------------------------------------- viewers
def _behind_event(data: Json, cursor: int) -> Event:
    return Event(
        "behind",
        first_seq=cursor,
        sealed_last_seq=int(str(data["sealed_last_seq"])),
        live_first_seq=int(str(data["live_first_seq"])),
    )


def _reason(reply: Json) -> str:
    """The error reason of a reply, or ``ok``."""
    err = reply.get("error")
    if isinstance(err, dict):
        data = err["data"]
        assert isinstance(data, dict)
        return str(data["reason"])
    return "ok"


def _first_seq(reply: Json) -> int:
    result = reply["result"]
    assert isinstance(result, dict)
    return int(str(result["first_seq"]))


@dataclass
class Viewer:
    """A cursor over a host: polls at its own rate, runs :class:`Catchup`, records what it got."""

    name: str
    host: Host
    period: int = 1  # polls every ``period`` steps
    budget: int = 64  # records it reads per poll
    disconnect_every: int = 0  # reconnecting viewer: drop the connection every n polls
    jump_at: int = -1  # jumping viewer: unsubscribe and subscribe "live" at this step
    conn: Connection | None = None
    loop: Catchup = field(default_factory=Catchup)
    received: list[tuple[int, str]] = field(default_factory=list)
    live_bodies: dict[int, bytearray] = field(default_factory=dict)  # year -> bytes seen live
    live_counts: dict[int, int] = field(default_factory=dict)
    sealed_seen: dict[int, str] = field(default_factory=dict)  # year -> announced digest
    jumped_from: int | None = None
    behinds: int = 0
    polls: int = 0
    rid: int = 0
    done: bool = False
    errors: list[str] = field(default_factory=list)

    def _apply(self, action: Action) -> None:
        """Carry out one action of the loop."""
        if action.kind == "subscribe":
            self._subscribe(action.a)
        elif action.kind == "read_sealed":
            lines = self.host.writer.read_sealed(action.a, action.b)
            self._deliver(action.a, lines, live=False)
            self._apply(self.loop.next(Event("read_sealed_done", action.a, len(lines))))
        elif action.kind == "done":
            self.done = True
        elif action.kind == "error":
            self.errors.append(action.detail)

    def _subscribe(self, seq: int) -> None:
        if self.conn is None or self.conn.closed:
            self.conn = self.host.connect()
        self.rid += 1
        _, reply = decode_control(cjson(self.host.subscribe(self.conn, self.rid, seq)))
        reason = _reason(reply)
        if reason == "ok":
            self._apply(self.loop.next(Event("subscribed", _first_seq(reply))))
        elif reason == "behind_live_edge":
            self.behinds += 1
            err = reply["error"]
            assert isinstance(err, dict) and isinstance(err["data"], dict)
            self._apply(self.loop.next(_behind_event(err["data"], seq)))
        else:
            self.errors.append(f"subscribe refused: {reason}")

    def _deliver(self, first_seq: int, lines: Sequence[str], *, live: bool) -> None:
        for i, line in enumerate(lines):
            self.received.append((first_seq + i, line))
            if live:
                year = int(_CANON.parse(line)["tick"]) // MINUTES_PER_YEAR
                self.live_bodies.setdefault(year, bytearray()).extend(line.encode() + LF)
                self.live_counts[year] = self.live_counts.get(year, 0) + 1

    def _handle(self, frame: Frame) -> int:
        """One frame from the socket, decoded through the wire; returns the records it held."""
        if frame.ftype == TYPE_RECORDS:
            rec = decode_records(frame.payload)
            action = self.loop.next(Event("records", rec.first_seq, len(rec.lines)))
            if action.kind == "deliver":
                self._deliver(rec.first_seq, rec.lines, live=True)
            self._apply(action)
            return len(rec.lines)
        _, obj = decode_control(frame.payload)
        params = obj.get("params")
        assert isinstance(params, dict)
        method = obj["method"]
        if method == "stream.sealed":
            self.sealed_seen[int(str(params["sim_year"]))] = str(params["canonical_digest"])
        elif method == "stream.behind":
            self.behinds += 1
            self._apply(self.loop.next(_behind_event(params, self.loop.cursor)))
        elif method == "stream.end":
            self._apply(self.loop.next(Event("end")))
        return 0

    def step(self, t: int, *, drain: bool = False) -> None:
        """One step of wall time: poll if due (always when draining), jump or drop if scripted."""
        if self.done or (not drain and t % self.period):
            return
        self.polls += 1
        if self.conn is None:
            self._apply(self.loop.start())
        if t == self.jump_at:
            self._jump()
        if self.disconnect_every and self.polls % self.disconnect_every == 0 and self.conn:
            self.host.close(self.conn)
            self._apply(self.loop.next(Event("closed")))
            return
        budget = self.budget
        while budget > 0 and self.conn is not None and not self.conn.closed and not self.done:
            frames = self.host.serve(self.conn, budget)
            if not frames:
                return
            budget -= sum(self._handle(frame) for frame in frames)

    def _jump(self) -> None:
        """Jump to live: unsubscribe, subscribe ``live``, start a fresh loop at the tip."""
        if self.conn is None or self.loop.state != "live":
            return
        self.rid += 1
        self.host.unsubscribe(self.conn, self.rid)
        self.jumped_from = self.loop.cursor
        self.rid += 1
        first = _first_seq(self.host.subscribe(self.conn, self.rid, None))
        self.loop = Catchup(first)
        self.loop.state = "live"
        if first != self.host.writer.next_seq:
            self.errors.append("jump to live did not land on the tip")


def _make_line(rng: random.Random, channel: str, seq: int, tick: int) -> str:
    """A canonical line of a random kind of ``channel`` with the given header."""
    kinds = [k for k, ch in _CANON.CHANNEL.items() if ch == channel]
    rec = _CANON.generate(rng, rng.choice(kinds))
    rec = rec.replace(timeline_id=TIMELINE, seq=seq, tick=tick)
    if rec.kind == "MATCH" and int(rec["opened_tick"]) > tick:
        rec = rec.replace(opened_tick=tick)
    return str(_CANON.render(rec))


@dataclass(frozen=True)
class Scenario:
    """One simulated run: its seed, length and channel."""

    seed: int
    years: int
    channel: str = "A"
    retain: int = 1
    break_i1: bool = False


def _viewers(host: Host, years: int) -> list[Viewer]:
    """The viewer mix: fast, slow, crawling, stalled, reconnecting, jumping."""
    steps = years * TICKS_PER_YEAR
    return [
        Viewer("fast", host, period=1, budget=1_000),
        Viewer("slow", host, period=3, budget=5),
        Viewer("crawl", host, period=TICKS_PER_YEAR * 3, budget=7),
        Viewer("stalled", host, period=steps + 1, budget=100_000),
        Viewer("reconnecting", host, period=2, budget=9, disconnect_every=5),
        Viewer("jumper", host, period=1, budget=50, jump_at=steps // 2),
    ]


DRAIN_LIMIT = 100_000


def simulate(sc: Scenario) -> tuple[Writer, list[Viewer]]:
    """Run a writer for ``sc.years`` sim-years with the viewer mix attached; drain at the end."""
    rng = random.Random(sc.seed)
    writer = Writer(sc.channel, sc.retain, sc.break_i1)
    host = Host(writer, batch=rng.choice([1, 8, 64, 1_000]))
    viewers = _viewers(host, sc.years)
    steps = sc.years * TICKS_PER_YEAR
    for t in range(steps):
        last_of_year = t % TICKS_PER_YEAR == TICKS_PER_YEAR - 1
        for _ in range(rng.randint(1 if last_of_year else 0, MAX_APPENDS_PER_TICK)):
            writer.append(_make_line(rng, sc.channel, writer.next_seq, t * TICK_MINUTES))
        if last_of_year:
            writer.seal_year()
        for v in viewers:
            v.step(t)
    writer.finished = True
    t = steps
    while any(not v.done for v in viewers) and t < steps + DRAIN_LIMIT:
        for v in viewers:
            v.step(t, drain=True)
        t += 1
    return writer, viewers


def _check_seqs(v: Viewer, next_seq: int) -> list[str]:
    """The received seqs are exactly 0..next_seq-1, or, for a jumper, contiguous on each side."""
    seqs = [seq for seq, _ in v.received]
    if v.jumped_from is None:
        if seqs != list(range(next_seq)):
            return [f"{v.name}: seqs are not exactly 0..{next_seq - 1} ({len(seqs)} received)"]
        return []
    before = [s for s in seqs if s < v.jumped_from]
    after = [s for s in seqs if s >= v.jumped_from]
    tail = list(range(after[0], next_seq)) if after else []
    if before != list(range(v.jumped_from)) or after != tail:
        return [f"{v.name}: seqs around the jump are not contiguous"]
    return []


def check_viewer(v: Viewer, writer: Writer) -> list[str]:
    """The delivery properties one viewer must satisfy (wire_v1.md §6.3; V-STR-05)."""
    problems = list(v.errors)
    if not v.done:
        problems.append(f"{v.name}: did not reach stream.end")
    problems.extend(_check_seqs(v, writer.next_seq))
    lines = dict(v.received)
    for seg in writer.disk.values():
        wanted = range(seg.first_seq, seg.last_seq + 1)
        if all(s in lines for s in wanted):
            got = tuple(lines[s] for s in wanted)
            if _CANON.digest(got).hex() != seg.digest:
                problems.append(f"{v.name}: year {seg.sim_year} differs from the sealed digest")
        whole = v.live_counts.get(seg.sim_year) == len(wanted)
        announced = v.sealed_seen.get(seg.sim_year)
        if whole and announced is not None:
            body = bytes(v.live_bodies[seg.sim_year])
            if _CANON.xxh3_128(body).hex() != announced:
                problems.append(f"{v.name}: live body of year {seg.sim_year} != stream.sealed")
    if v.name == "fast" and writer.retain_sealed_years and len(v.sealed_seen) != len(writer.disk):
        problems.append(f"fast: saw {len(v.sealed_seen)} stream.sealed of {len(writer.disk)}")
    return problems


SCENARIOS = (
    Scenario(1, 6),
    Scenario(2, 10, retain=0),
    Scenario(3, 4, channel="C"),
    Scenario(4, 12),
)


def _live_edge_checks(scenarios: Sequence[Scenario] = SCENARIOS) -> list[str]:
    """Every viewer of every scenario gets 0..next_seq-1 exactly, with the sealed digests."""
    problems: list[str] = []
    for sc in scenarios:
        writer, viewers = simulate(sc)
        for v in viewers:
            problems.extend(f"seed {sc.seed}: {p}" for p in check_viewer(v, writer))
        if not any(v.behinds for v in viewers):
            problems.append(f"seed {sc.seed}: no viewer exercised the sealed path")
        if writer.retain_sealed_years and any(v.behinds for v in viewers if v.name == "fast"):
            problems.append(f"seed {sc.seed}: the fast viewer fell off the edge")
    return problems


# ---------------------------------------------------------------------------------------- self-test
Check = tuple[str, Callable[[], list[str]]]


def _records_a() -> Records:
    """Three hand-written channel-A records (canonical_v0.md §3's examples, seqs 10 to 12)."""
    return Records(
        TIMELINE,
        "A",
        10,
        (
            "EVENT 0 10 0 0 4 0 DEATH 32768 ~ 0 128 0 0 [1] [1/4369/1] []",
            "LIFE_EXIT 0 11 216000 13107 1 0 ~",
            "STEP_DIGEST 0 12 216000 0 0 1 0000000000000001 0000000000000002 0000000000000003",
        ),
    )


def _records_c() -> Records:
    return Records(
        TIMELINE,
        "C",
        0,
        ("MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 0 0 1 [A/1/4369] [e1/0]",),
    )


def _frame_round_trips() -> list[str]:
    """Every frame type encodes, decodes to itself, and short buffers say Incomplete."""
    problems: list[str] = []
    for frame in [control_frame(request(1, "stream.status")), records_frame(_records_a())]:
        data = encode_frame(frame)
        back, used = decode_frame(data + b"tail")
        if back != frame or used != len(data):
            problems.append(f"frame type {frame.ftype} does not round trip")
        for cut in (0, HEADER_LEN - 1, HEADER_LEN, len(data) - 1):
            try:
                decode_frame(data[:cut])
                problems.append(f"a {cut}-byte prefix decoded")
            except IncompleteFrameError:
                pass
    return problems


def _messages() -> list[tuple[str, Frame]]:
    """One frame of every message of wire_v1.md §4, in golden order."""
    a_status = Host(Writer("A")).status()
    c_status = Host(Writer("C")).status()
    behind: Json = {"live_first_seq": 1200, "sealed_last_seq": 1199, "sealed_last_year": 3}
    sealed: Json = {
        "canonical_digest": "99aa06d3014798d86001c324468d497f",
        "channel": "A",
        "first_seq": 900,
        "last_seq": 1199,
        "sim_year": 3,
        "timeline_id": 0,
    }
    tl: Json = {"channel": "A", "timeline_id": 0}
    return [
        ("status_request", control_frame(request(1, "stream.status"))),
        ("status_result_a", control_frame(response(1, a_status))),
        ("status_result_c", control_frame(response(1, c_status))),
        ("subscribe_seq", control_frame(request(2, "stream.subscribe", {"from": {"seq": 1200}}))),
        ("subscribe_live", control_frame(request(3, "stream.subscribe", {"from": "live"}))),
        ("subscribe_result", control_frame(response(2, {"first_seq": 1200}))),
        ("unsubscribe", control_frame(request(4, "stream.unsubscribe"))),
        ("unsubscribe_result", control_frame(response(4, {"next_seq": 1234}))),
        ("error_parse", control_frame(error(None, "parse_error"))),
        ("error_invalid_request", control_frame(error(None, "invalid_request"))),
        ("error_method_not_found", control_frame(error(5, "method_not_found"))),
        ("error_invalid_params", control_frame(error(6, "invalid_params", detail="filter"))),
        (
            "error_unsupported_version",
            control_frame(error(None, "unsupported_version", supported=[1])),
        ),
        ("error_invalid_frame", control_frame(error(None, "invalid_frame", detail="flags 0x1"))),
        ("error_behind", control_frame(error(2, "behind_live_edge", **behind))),
        ("error_future", control_frame(error(2, "future_seq", next_seq=1234))),
        ("error_already_subscribed", control_frame(error(2, "already_subscribed"))),
        ("error_not_subscribed", control_frame(error(4, "not_subscribed"))),
        ("error_too_many_viewers", control_frame(error(None, "too_many_viewers", limit=64))),
        ("notify_sealed", control_frame(notification("stream.sealed", sealed))),
        (
            "notify_behind",
            control_frame(notification("stream.behind", {**tl, "next_seq": 850, **behind})),
        ),
        ("notify_end", control_frame(notification("stream.end", {**tl, "next_seq": 1234}))),
        ("records_a", records_frame(_records_a())),
        ("records_c", records_frame(_records_c())),
    ]


def _expected_kind(name: str) -> str:
    """The JSON-RPC class a golden message of this name must classify as."""
    if name.startswith("error"):
        return "error"
    if name.startswith("notify"):
        return "notification"
    return "response" if "result" in name else "request"


def _decoded(frame: Frame) -> list[str]:
    """The golden file's decoded description of a frame."""
    if frame.ftype == TYPE_CONTROL:
        return [f"control {frame.payload.decode('ascii')}"]
    rec = decode_records(frame.payload)
    head = f"records {rec.timeline_id} {rec.channel} {rec.first_seq} {len(rec.lines)}"
    return [head, *(f"line {line}" for line in rec.lines)]


def _message_checks() -> list[str]:
    """Every message encodes to canonical JSON, decodes, classifies and re-encodes identically."""
    problems: list[str] = []
    statuses: list[Json] = []
    for name, frame in _messages():
        if frame.ftype == TYPE_RECORDS:
            if encode_records(decode_records(frame.payload)) != frame.payload:
                problems.append(f"{name}: records do not round trip")
            continue
        kind, obj = decode_control(frame.payload)
        if cjson(obj) != frame.payload:
            problems.append(f"{name}: not canonical after a round trip")
        if kind != _expected_kind(name):
            problems.append(f"{name}: classified as {kind}")
        if name.startswith("status_result"):
            result = obj["result"]
            assert isinstance(result, dict)
            statuses.append(result)
    for status in statuses:
        try:
            check_status(status)
        except WireError as e:
            problems.append(f"status: {e}")
    return problems


def _reframe(records: Records, channel: str | None = None, *, bad_line: bool = False) -> bytes:
    """A RECORDS frame built without the sender's checks, for a planted refusal."""
    lines = list(records.lines)
    if bad_line:
        lines[1] = lines[1] + " "
    head = SUBHEADER.pack(
        records.timeline_id,
        records.first_seq,
        len(lines),
        CHANNEL_BYTE[channel or records.channel],
        RESERVED,
    )
    return encode_frame(Frame(TYPE_RECORDS, head + b"".join(ln.encode() + LF for ln in lines)))


def _refusals() -> list[tuple[str, bytes, str]]:
    """Frames the receiver must refuse: (name, bytes, the reason it names)."""
    good = encode_frame(control_frame(request(1, "stream.status")))
    rec = encode_frame(records_frame(_records_a()))
    sub = HEADER_LEN
    bad_json: list[tuple[str, bytes, str]] = [
        ("control_batch", b"[]", "invalid_request"),
        ("control_float", b'{"id":1.5,"jsonrpc":"2.0","method":"stream.status"}', "float"),
        (
            "control_non_ascii",
            b'{"id":"\xc3\xa9","jsonrpc":"2.0","method":"stream.status"}',
            "ASCII",
        ),
        ("control_repeated_key", b'{"id":1,"id":2,"jsonrpc":"2.0","method":"x"}', "repeated"),
        ("control_not_json", b"{", "parse_error"),
        ("control_wrong_jsonrpc", b'{"id":1,"jsonrpc":"1.0","method":"stream.status"}', "2.0"),
        ("control_bool_id", b'{"id":true,"jsonrpc":"2.0","method":"stream.status"}', "id"),
        ("control_params_array", b'{"id":1,"jsonrpc":"2.0","method":"m","params":[]}', "params"),
        ("control_result_and_error", b'{"error":{},"id":1,"jsonrpc":"2.0","result":{}}', "one"),
    ]
    return [
        ("bad_magic", b"RHWX" + good[4:], "magic"),
        ("version_2", good[:4] + b"\x02\x00" + good[6:], "unsupported_version"),
        ("type_3", good[:6] + b"\x03" + good[7:], "frame type"),
        ("flag_1", good[:7] + b"\x01" + good[8:], "flags"),
        ("length_over_max", good[:8] + struct.pack("<I", MAX_PAYLOAD + 1), "exceeds"),
        *[(n, encode_frame(Frame(TYPE_CONTROL, p)), r) for n, p, r in bad_json],
        ("records_reserved", rec[: sub + 17] + b"\x01\0\0" + rec[sub + 20 :], "reserved"),
        ("records_channel_b", rec[: sub + 16] + b"B" + rec[sub + 17 :], "channel byte"),
        ("records_count_short", rec[: sub + 12] + struct.pack("<I", 2) + rec[sub + 16 :], "count"),
        ("records_count_zero", rec[: sub + 12] + struct.pack("<I", 0) + rec[sub + 16 :], "count"),
        ("records_seq_skip", rec[: sub + 4] + struct.pack("<Q", 11) + rec[sub + 12 :], "seq"),
        ("records_no_final_lf", encode_frame(Frame(TYPE_RECORDS, rec[sub:-1])), "LF"),
        ("records_wrong_channel", _reframe(_records_a(), channel="C"), "channel"),
        ("records_not_canonical", _reframe(_records_a(), bad_line=True), "canonical"),
    ]


def _decode_any(data: bytes) -> None:
    frame, _ = decode_frame(data)
    if frame.ftype == TYPE_RECORDS:
        decode_records(frame.payload)
    else:
        decode_control(frame.payload)


def _refusal_checks() -> list[str]:
    """Every planted frame is refused with a message naming the reason."""
    problems: list[str] = []
    for name, data, reason in _refusals():
        try:
            _decode_any(data)
            problems.append(f"{name}: accepted")
        except WireError as e:
            if reason not in str(e):
                problems.append(f"{name}: refused for {e!r}, expected {reason!r}")
    return problems


SUBSCRIBE_PARAMS: tuple[tuple[Json, bool], ...] = (
    ({"from": {"seq": 3}}, True),
    ({"from": "live"}, True),
    ({"from": {"seq": -1}}, False),
    ({"from": {"seq": 1}, "filter": {}}, False),
    ({"from": {"seq": True}}, False),
    ({"from": {"seq": 1, "x": 2}}, False),
    ({}, False),
)


def _method_checks() -> list[str]:
    """The host answers only the three methods; anything else is the read-only refusal."""
    problems: list[str] = []
    for method in ("player.act", "timeline.fork", "oracle.whisper", "stream.pace", "stream.x"):
        if method in METHODS:
            problems.append(f"{method} is a stream method")
    err = error(7, "method_not_found")["error"]
    assert isinstance(err, dict)
    if err["message"] != READ_ONLY_MESSAGE:
        problems.append("method_not_found does not carry the read-only message")
    for params, ok in SUBSCRIBE_PARAMS:
        try:
            subscribe_from(params)
            if not ok:
                problems.append(f"subscribe params {params} accepted")
        except WireError:
            if ok:
                problems.append(f"subscribe params {params} refused")
    return problems


def _host_checks() -> list[str]:
    """Subscribe and unsubscribe answers of a small host: future, behind, already, not."""
    problems: list[str] = []
    writer = Writer("A", retain_sealed_years=0)
    host = Host(writer)
    year_records = 5
    for seq in range(year_records):
        writer.append(_make_line(random.Random(seq), "A", seq, 0))
    writer.seal_year()
    writer.append(_make_line(random.Random(9), "A", year_records, MINUTES_PER_YEAR))
    conn = host.connect()
    cases = [
        (host.subscribe(conn, 1, 9), "future_seq"),
        (host.subscribe(conn, 2, 2), "behind_live_edge"),
        (host.unsubscribe(conn, 3), "not_subscribed"),
        (host.subscribe(conn, 4, 5), "ok"),
        (host.subscribe(conn, 5, 6), "already_subscribed"),
        (host.unsubscribe(conn, 6), "ok"),
    ]
    for reply, want in cases:
        if _reason(reply) != want:
            problems.append(f"expected {want}, got {_reason(reply)}")
    status = check_status(host.status())
    want_live: Json = {"first_seq": year_records, "next_seq": year_records + 1}
    if status["live"] != want_live:
        problems.append(f"window after the seal is {status['live']}")
    return problems


CATCHUP_SCENARIO: tuple[Event, ...] = (
    Event("behind", sealed_last_seq=99, live_first_seq=100),
    Event("read_sealed_done", 0, 100),
    Event("subscribed", 100),
    Event("records", 100, 50),
    Event("sealed"),
    Event("closed"),
    Event("subscribed", 150),
    Event("records", 150, 1),
    Event("behind", sealed_last_seq=400, live_first_seq=401),
    Event("read_sealed_done", 151, 250),
    Event("subscribed", 401),
    Event("records", 402, 1),
    Event("records", 401, 3),
    Event("end"),
    Event("records", 404, 1),
)
CATCHUP_ERRORS = 2  # the out-of-order frame (402 before 401) and the record after stream.end
CATCHUP_END = "action done 404 0 %"


def _fmt_event(ev: Event) -> str:
    return f"{ev.kind} {ev.first_seq} {ev.count} {ev.sealed_last_seq} {ev.live_first_seq}"


def _fmt_action(act: Action) -> str:
    detail = _CANON.encode_str(act.detail) if act.detail else "%"
    return f"{act.kind} {act.a} {act.b} {detail}"


def catchup_transcript(events: Iterable[Event] = CATCHUP_SCENARIO) -> list[str]:
    """The actions :class:`Catchup` takes on a fixed event list, as golden lines."""
    loop = Catchup()
    out = [f"action {_fmt_action(loop.start())}"]
    for ev in events:
        out.append(f"event {_fmt_event(ev)}")
        out.append(f"action {_fmt_action(loop.next(ev))}")
    return out


def _catchup_checks() -> list[str]:
    """The fixed scenario's transcript is what the state machine must produce."""
    lines = catchup_transcript()
    errors = [ln for ln in lines if ln.startswith("action error")]
    if len(errors) != CATCHUP_ERRORS:
        return [f"{len(errors)} errors in the transcript, expected {CATCHUP_ERRORS}: {errors}"]
    if CATCHUP_END not in lines:
        return ["the loop did not finish at 404"]
    return []


def self_test(control: str | None = None) -> list[str]:
    """Run every check; return the report lines (the last says PASS or FAIL)."""
    live_edge: Check = (
        "live-edge rule: fast, slow, crawling, stalled, reconnecting, jumping viewers",
        _live_edge_checks,
    )
    if control == "window-gap":
        live_edge = (
            "live-edge rule (control: a year dropped before its seal, I1 broken)",
            lambda: _live_edge_checks(
                [Scenario(1, 6, break_i1=True), Scenario(2, 6, retain=0, break_i1=True)]
            ),
        )
    checks: list[Check] = [
        ("frame round trips and Incomplete", _frame_round_trips),
        ("every message: canonical, decoded, classified", _message_checks),
        ("refusals: bad frames, payloads and lines", _refusal_checks),
        ("read-only method table and subscribe params", _method_checks),
        ("host answers: future, behind, already, not subscribed, window", _host_checks),
        live_edge,
        ("Catchup state machine transcript", _catchup_checks),
    ]
    report, failed = [], False
    for i, (name, check) in enumerate(checks, 1):
        problems = check()
        failed |= bool(problems)
        report.append(f"{i}. {name}: {'FAIL' if problems else 'PASS'}")
        report.extend(f"   - {p}" for p in problems[:8])
    report.append("WIRE v1: FAIL" if failed else "WIRE v1: PASS")
    return report


# ---------------------------------------------------------------------------------------- golden
def render_golden() -> str:
    """The golden file's full text."""
    header = [
        "# World Stream wire format v1: golden frames, refusals and a Catchup transcript.",
        "# GENERATED by specs/stream/selftest_wire_v1.py; do not edit. Contract: wire_v1.md.",
        "# `frame <name> <hex>`: one frame, header and payload; then its decoded form:",
        "#   `control <canonical json>` or `records <timeline> <channel> <first_seq> <count>`",
        "#   followed by `line <canonical line>` per record. rh-protocol encodes the decoded form",
        "#   and must produce the hex; it decodes the hex and must produce the decoded form.",
        "# `refuse <name> <hex> <reason>`: a frame the receiver refuses, naming <reason>.",
        "# `scenario catchup` then `action ...` / `event ...` pairs: Catchup (wire_v1.md §6.3)",
        "#   driven by events `<kind> <first_seq> <count> <sealed_last_seq> <live_first_seq>`,",
        "#   answering `<kind> <a> <b> <detail>` (detail as a canonical string token).",
    ]
    body: list[str] = []
    for name, frame in _messages():
        body.append(f"frame {name} {encode_frame(frame).hex()}")
        body.extend(_decoded(frame))
    for name, data, reason in _refusals():
        body.append(f"refuse {name} {data.hex()} {_CANON.encode_str(reason)}")
    body.append("scenario catchup")
    body.extend(catchup_transcript())
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


def main(argv: list[str] | None = None) -> int:
    """Run the self-test, then write or check the golden file."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the file to write or check")
    parser.add_argument("--control", choices=["window-gap"], help="a planted fault that must FAIL")
    args = parser.parse_args(argv)
    report = self_test(args.control)
    print("\n".join(report))
    if report[-1].endswith("FAIL"):
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
