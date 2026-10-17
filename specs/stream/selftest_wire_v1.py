"""Reference of the World Stream wire format v1 (IF-0 live edge; SPR-026, SPR-027).

The contract is ``specs/stream/wire_v1.md``: a 12-byte frame header, CONTROL frames carrying one
JSON-RPC 2.0 object in canonical JSON, RECORDS frames carrying canonical lines
(``specs/stream/canonical_v0.md``; the wire reads only their ``<KIND> <tl> <seq> <tick> ``
prefix), the three ``stream.*`` methods, the notifications, the error codes, and the live-edge
rule by which a viewer that falls behind the host's in-memory window reads sealed segments and
comes back with no gap and no duplicate, by ``seq``. This file is the independent reference
``rh-protocol`` (SPR-026) must agree with, as ``canonical_v0.py`` is for ``rh-stream``:

- :func:`encode_frame` and :func:`decode_frame` are the framing; :func:`encode_records` and
  :func:`decode_records` the RECORDS payload; :func:`cjson` the canonical JSON; the
  :func:`request`, :func:`response`, :func:`error` and :func:`notification` builders,
  :func:`decode_control` and the host's :meth:`Host.dispatch` the control plane, with the
  refusals of wire_v1.md §2, §3 and §4;
- :class:`Catchup` is the viewer's pure state machine of wire_v1.md §6.3 (failed, retry, jump,
  the reconnect check);
- :class:`Writer` and :class:`Host` simulate a channel's writer (appends, asynchronous seals, the
  window policy, the seal log, a restart) and its socket server (one snapshot per answer);
  :class:`Viewer` runs fast, slow, crawling, stalled, reconnecting, jumping and late cursors over
  them, and the self-test asserts gap-free, duplicate-free delivery in ``seq`` order with
  per-year digests equal to the sealed ones, under several seeds, both window policies and
  empty channel-C years; a separate check interleaves a reader's loads with a seal's steps;
- the golden file ``specs/stream/golden/wire_v1.txt`` holds one frame of every message, the
  frames that must be refused, the host's answers to planted requests, and a ``Catchup``
  transcript; ``rh-protocol``'s conformance test reproduces every line.

The record lines come from ``specs/stream/golden/canonical_v0.txt``, never hand-typed, so the
canonical format may change under this file (SPR-020); whichever change merges second regenerates
``golden/wire_v1.txt`` with this tool (REV-SPR-026 R6).

Usage:  python -I specs/stream/selftest_wire_v1.py            (self-test, then rewrite the golden)
        python -I specs/stream/selftest_wire_v1.py --check    (self-test; exit 1 if the golden
                                                           differs)
        python -I specs/stream/selftest_wire_v1.py --check --golden PATH   (check another file;
                                                           the positive control uses a stale copy)
        python -I specs/stream/selftest_wire_v1.py --control window-gap   (a host that moves the
                                                           window before the year it drops is on
                                                           disk: the live-edge check must FAIL; the
                                                           golden file is never written)
Exit:   0 ok, 1 stale golden file, 2 the self-test failed.
Standard library only (GOV-003 §3); canonical_v0.py is loaded by path for its kind table, digest
and string codec.
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
CANONICAL_GOLDEN = HERE / "golden" / "canonical_v0.txt"
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
MAX_PAYLOAD = 1024 * 1024  # every frame (wire_v1.md §2)
VIEWER_MAX_PAYLOAD = 4096  # a frame from a viewer (§2.1; REV-SPR-026 R9)
MAX_DEPTH = 4  # JSON nesting in a control frame
SUBHEADER = struct.Struct("<IQIB3s")  # timeline_id, first_seq, count, channel, reserved
SUBHEADER_LEN = SUBHEADER.size
RESERVED = b"\0\0\0"
CHANNEL_BYTE = {"A": 0x41, "C": 0x43}
BYTE_CHANNEL = {v: k for k, v in CHANNEL_BYTE.items()}
LF = b"\n"
PREFIX_FIELDS = 4  # <KIND> <tl> <seq> <tick>
INT_MAX_CHARS = 20
U32_MAX = (1 << 32) - 1
U64_MAX = (1 << 64) - 1
I64_MIN = -(1 << 63)
I64_MAX = (1 << 63) - 1
ASCII_PRINTABLE = range(0x20, 0x7F)
JSONRPC = "2.0"
METHODS = frozenset({"stream.status", "stream.subscribe", "stream.unsubscribe"})
NOTIFICATIONS = frozenset({"stream.sealed", "stream.behind", "stream.end"})
BANDS = frozenset({"live", "1h", "1d", "1w", "1mo", "1y", "10y", "max"})
STATES = frozenset({"running", "paused", "finished"})
END_REASONS = frozenset({"finished", "aborted"})
REQUEST_KEYS = frozenset({"jsonrpc", "id", "method", "params"})
NOTIFICATION_KEYS = frozenset({"jsonrpc", "method", "params"})
RESPONSE_KEYS = frozenset({"jsonrpc", "id", "result"})
ERROR_RESPONSE_KEYS = frozenset({"jsonrpc", "id", "error"})
ERROR_KEYS = frozenset({"code", "message", "data"})
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
RETRIABLE = frozenset({"future_seq", "too_many_viewers"})
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
KIND_CHANNEL: dict[str, str] = dict(_CANON.CHANNEL)  # the canonical format's kind table


def digest(lines: Iterable[str]) -> str:
    """``canonical_digest`` of lines, as 32 lowercase hex digits (canonical_v0.md §4)."""
    return str(_CANON.digest(list(lines)).hex())


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


def decode_header(buf: bytes, limit: int = MAX_PAYLOAD) -> tuple[int, int]:
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
    if length > limit:
        raise WireError(f"length {length} exceeds {limit}")
    return ftype, length


def decode_frame(buf: bytes, limit: int = MAX_PAYLOAD) -> tuple[Frame, int]:
    """The frame at the start of ``buf`` and the bytes it used, or :class:`IncompleteFrameError`."""
    ftype, length = decode_header(buf, limit)
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


def _int_token(tok: str, lo: int, hi: int, what: str) -> int:
    if len(tok) > INT_MAX_CHARS or not _CANON.INT.fullmatch(tok):
        raise WireError(f"{what} {tok!r} is not a canonical integer")
    value = int(tok)
    if not lo <= value <= hi:
        raise WireError(f"{what} {value} is outside [{lo}, {hi}]")
    return value


def check_prefix(line: str, channel: str, timeline_id: int, seq: int) -> str:
    """Check a line's frozen prefix ``<KIND> <tl> <seq> <tick> `` (wire_v1.md §2.2); return KIND.

    The rest of the line is the canonical format's business (validated by the writer at append,
    canonical_v0.md), never the wire's.
    """
    if not line or any(ord(ch) not in ASCII_PRINTABLE for ch in line):
        raise WireError(f"seq {seq}: a record line is empty or not printable ASCII")
    tokens = line.split(" ", PREFIX_FIELDS)
    if len(tokens) < PREFIX_FIELDS:
        raise WireError(f"seq {seq}: a record line needs the prefix <KIND> <tl> <seq> <tick>")
    kind, tl, sq, tick = tokens[:PREFIX_FIELDS]
    if KIND_CHANNEL.get(kind) != channel:
        raise WireError(f"seq {seq}: kind {kind!r} is not a channel-{channel} kind")
    if _int_token(tl, 0, U32_MAX, "timeline") != timeline_id:
        raise WireError(f"seq {seq}: the line's timeline {tl} is not {timeline_id}")
    if _int_token(sq, 0, U64_MAX, "seq") != seq:
        raise WireError(f"seq {seq}: the line's seq is {sq}")
    _int_token(tick, I64_MIN, I64_MAX, "tick")
    return kind


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
        check_prefix(line, records.channel, records.timeline_id, records.first_seq + i)
    body = b"".join(line.encode("ascii") + LF for line in records.lines)
    return records_payload(
        records.timeline_id, records.channel, records.first_seq, len(records.lines), body
    )


def records_payload(
    timeline_id: int, channel: str, first_seq: int, count: int, body: bytes
) -> bytes:
    """A RECORDS payload from a ready body: what a serving thread does (bytes in, bytes out)."""
    head = SUBHEADER.pack(timeline_id, first_seq, count, CHANNEL_BYTE[channel], RESERVED)
    if len(head) + len(body) > MAX_PAYLOAD:
        raise WireError(f"a RECORDS payload of {len(head) + len(body)} bytes exceeds {MAX_PAYLOAD}")
    return head + body


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
    try:
        lines = tuple(part.decode("ascii", errors="strict") for part in raw)
    except UnicodeDecodeError as e:
        raise WireError(f"a record line is not ASCII: {e}") from e
    records = Records(timeline_id, BYTE_CHANNEL[channel_byte], first_seq, lines)
    encode_records(records)  # the same prefix checks, line by line
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


def _check_json(value: object, where: str, depth: int = 1) -> None:
    """Refuse a float, a non-ASCII string or a nesting deeper than MAX_DEPTH, anywhere."""
    if _check_scalar(value, where):
        return
    if depth > MAX_DEPTH:
        raise WireError(f"{where}: nesting depth exceeds {MAX_DEPTH}")
    if isinstance(value, dict):
        for k, v in value.items():
            if not isinstance(k, str):
                raise WireError(f"{where}: a non-string key")
            _check_json(v, f"{where}.{k}", depth + 1)
    elif isinstance(value, (list, tuple)):
        for i, v in enumerate(value):
            _check_json(v, f"{where}[{i}]", depth + 1)
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
    """Parse a control payload: JSON with no float, no repeated key, ASCII strings, depth <= 4."""
    try:
        value: object = json.loads(
            payload.decode("utf-8"), parse_float=_no_float, object_pairs_hook=_no_repeat
        )
    except (UnicodeDecodeError, json.JSONDecodeError, RecursionError) as e:
        raise WireError(f"parse_error: {type(e).__name__}") from e
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


def _only_keys(obj: Json, allowed: frozenset[str], what: str) -> None:
    extra = set(obj) - allowed
    if extra:
        raise WireError(f"invalid_request: unknown member {sorted(extra)[0]!r} in a {what}")


def _classify_call(obj: Json) -> str:
    if not isinstance(obj["method"], str):
        raise WireError("invalid_request: method is not a string")
    if "params" in obj and not isinstance(obj["params"], dict):
        raise WireError("invalid_params: params is not an object")
    if "id" not in obj:
        _only_keys(obj, NOTIFICATION_KEYS, "notification")
        return "notification"
    _only_keys(obj, REQUEST_KEYS, "request")
    if obj["id"] is None or not _is_id(obj["id"]):
        raise WireError("invalid_request: a request id is an integer or a string")
    return "request"


def _classify_reply(obj: Json) -> str:
    if "id" not in obj or not _is_id(obj["id"]):
        raise WireError("invalid_request: a response carries an id")
    if ("result" in obj) == ("error" in obj):
        raise WireError("invalid_request: a response has exactly one of result and error")
    if "error" not in obj:
        _only_keys(obj, RESPONSE_KEYS, "response")
        return "response"
    _only_keys(obj, ERROR_RESPONSE_KEYS, "error response")
    err = obj["error"]
    if not isinstance(err, dict) or not isinstance(err.get("code"), int):
        raise WireError("invalid_request: error without an integer code")
    _only_keys(err, ERROR_KEYS, "error object")
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


def no_params(params: object) -> None:
    """``stream.status`` and ``stream.unsubscribe`` take no parameter: absent or ``{}``."""
    if params is not None and params != {}:
        raise WireError("invalid_params: this method takes no parameter")


def _opt_u64(value: object, where: str) -> int | None:
    return None if value is None else _u64(value, where)


def check_status(result: object) -> Json:
    """Check a ``stream.status`` result's shape and invariants (wire_v1.md §4.1)."""
    if not isinstance(result, dict):
        raise WireError("status: not an object")
    keys = ("wire", "channel", "timeline_id", "tip", "sealed", "live", "history", "calendar")
    for key in (*keys, "run_nonce"):
        if key not in result:
            raise WireError(f"status: missing {key}")
    wire, live, sealed, tip = result["wire"], result["live"], result["sealed"], result["tip"]
    if wire != {"format": WIRE_FORMAT, "version": VERSION}:
        raise WireError("status: wire is not this format and version")
    if not (isinstance(live, dict) and isinstance(sealed, dict) and isinstance(tip, dict)):
        raise WireError("status: live, sealed and tip are objects")
    _u64(result["run_nonce"], "run_nonce")
    w0, w1 = _u64(live["first_seq"], "live.first_seq"), _u64(live["next_seq"], "live.next_seq")
    if w0 > w1 or _u64(tip["next_seq"], "tip.next_seq") != w1:
        raise WireError("status: live.first_seq <= live.next_seq = tip.next_seq")
    s = _opt_u64(sealed["last_seq"], "sealed.last_seq")
    if s is None and w0 != 0:
        raise WireError("status: live.first_seq is 0 before the first non-empty seal (I1)")
    if s is not None and s + 1 < w0:
        raise WireError("status: sealed.last_seq + 1 >= live.first_seq (I1)")
    return result


# ---------------------------------------------------------------------------------------- catch-up
@dataclass(frozen=True)
class Event:
    """What the viewer's loop hears (wire_v1.md §6.3): a reply, a frame or an input."""

    kind: str  # see Catchup
    first_seq: int = 0
    count: int = 0
    sealed_last_seq: int = 0
    live_first_seq: int = 0
    reason: str = ""
    same_run: bool = True


@dataclass(frozen=True)
class Action:
    """What the viewer's loop does next."""

    kind: str  # subscribe | subscribe_live | read_sealed | deliver | ignore | wait | close ...
    a: int = 0
    b: int = 0
    detail: str = ""


BACKOFF_FIRST = 1
BACKOFF_CAP = 64


class Catchup:
    """The pure catch-up state machine of a cursor (wire_v1.md §6.3).

    States: ``idle``, ``connecting`` (a reconnect, waiting for ``status``), ``subscribing``,
    ``live``, ``reading_sealed``, ``retry`` (backing off), ``jumping``, ``failed`` (a protocol
    error: only ``closed`` is accepted), ``done``, ``fatal`` (another run on the socket).
    Events: ``subscribed``, ``behind``, ``records``, ``sealed``, ``end``, ``closed``,
    ``read_sealed_done``, ``refused`` (an error reply with ``reason``), ``retry_elapsed``,
    ``status`` (``same_run``), ``jump`` (an input), ``unsubscribed``.
    """

    def __init__(self, cursor: int = 0) -> None:
        """Start a loop whose next seq is ``cursor``."""
        self.cursor = cursor
        self.state = "idle"
        self.read_to = 0
        self.backoff = BACKOFF_FIRST
        self.skipped: tuple[int, int] | None = None  # (from, to) of a jump

    def start(self) -> Action:
        """The first action: subscribe from the cursor."""
        self.state = "subscribing"
        return Action("subscribe", self.cursor)

    def next(self, event: Event) -> Action:
        """Feed one event; return the action it calls for."""
        if self.state == "failed":
            return self._closed(event) if event.kind == "closed" else Action("ignore")
        if self.state in ("done", "fatal"):
            return Action("ignore", detail=self.state)
        handlers: dict[str, Callable[[Event], Action]] = {
            "closed": self._closed,
            "status": self._status,
            "subscribed": self._subscribed,
            "behind": self._behind,
            "records": self._records,
            "read_sealed_done": self._read_done,
            "refused": self._refused,
            "retry_elapsed": self._retry_elapsed,
            "sealed": lambda _: Action("ignore"),
            "end": self._end,
            "jump": self._jump,
            "unsubscribed": self._unsubscribed,
        }
        handler = handlers.get(event.kind)
        return handler(event) if handler else self._fail(f"unknown event {event.kind}")

    def _fail(self, detail: str) -> Action:
        self.state = "failed"
        return Action("close", detail=detail)

    def _closed(self, _: Event) -> Action:
        self.state = "connecting"
        return Action("status")

    def _status(self, event: Event) -> Action:
        if self.state != "connecting":
            return Action("ignore")  # an informational status while subscribed
        if not event.same_run:
            self.state = "fatal"
            return Action("stop", detail="the socket serves another run (history differs)")
        self.state = "subscribing"
        return Action("subscribe", self.cursor)

    def _end(self, event: Event) -> Action:
        if self.state != "live":
            return self._fail("stream.end while not subscribed")
        self.state = "done"
        return Action("done", self.cursor, detail=event.reason)

    def _subscribed(self, event: Event) -> Action:
        if self.state == "jumping":
            self.skipped = (self.cursor, event.first_seq)
            self.cursor = event.first_seq
            self.state = "live"
            return Action("deliver", self.cursor, 0)
        if self.state != "subscribing":
            return self._fail("a subscribe response while not subscribing")
        if event.first_seq != self.cursor:
            return self._fail(f"first_seq {event.first_seq} is not the cursor")
        self.state = "live"
        self.backoff = BACKOFF_FIRST
        return Action("deliver", self.cursor, 0)

    def _behind(self, event: Event) -> Action:
        if self.state not in ("subscribing", "live"):
            return self._fail("behind while not subscribed or subscribing")
        if self.state == "live" and event.first_seq != self.cursor:
            return self._fail(f"stream.behind.next_seq {event.first_seq} is not the cursor")
        s, w0 = event.sealed_last_seq, event.live_first_seq
        if s + 1 < w0:
            return self._fail(f"I1 broken: sealed_last_seq {s} + 1 < live_first {w0}")
        if s < self.cursor:
            return self._fail(f"behind with sealed_last_seq {s} below cursor")
        self.state = "reading_sealed"
        self.read_to = s
        return Action("read_sealed", self.cursor, s)

    def _records(self, event: Event) -> Action:
        if self.state != "live":
            return self._fail("records while not subscribed")
        if event.first_seq != self.cursor or event.count == 0:
            return self._fail(f"frame first_seq {event.first_seq} is not the cursor {self.cursor}")
        self.cursor += event.count
        return Action("deliver", event.first_seq, event.count)

    def _read_done(self, event: Event) -> Action:
        if self.state != "reading_sealed" or event.first_seq != self.cursor:
            return self._fail("a sealed read that does not start at the cursor")
        if event.first_seq + event.count - 1 != self.read_to:
            return self._fail(f"a sealed read of {event.count} seqs to {self.read_to}")
        self.cursor = event.first_seq + event.count
        self.state = "subscribing"
        return Action("subscribe", self.cursor)

    def _refused(self, event: Event) -> Action:
        if event.reason not in RETRIABLE:
            return self._fail(f"subscribe refused: {event.reason}")
        self.state = "retry"
        delay, self.backoff = self.backoff, min(self.backoff * 2, BACKOFF_CAP)
        return Action("wait", delay, detail=event.reason)

    def _retry_elapsed(self, _: Event) -> Action:
        if self.state != "retry":
            return self._fail("retry_elapsed while not retrying")
        self.state = "subscribing"
        return Action("subscribe", self.cursor)

    def _jump(self, _: Event) -> Action:
        if self.state != "live":
            return Action("ignore", detail="jump while not live")
        self.state = "jumping"
        return Action("unsubscribe")

    def _unsubscribed(self, event: Event) -> Action:
        if self.state != "jumping":
            return self._fail("unsubscribed while not jumping")
        if event.first_seq != self.cursor:
            return self._fail("unsubscribe.next_seq is not the cursor")
        return Action("subscribe_live")


# ---------------------------------------------------------------------------------------- the host
MINUTES_PER_YEAR = 518_400
TICKS_PER_YEAR = 12  # the simulation's instants per sim-year (a coarse month)
TICK_MINUTES = MINUTES_PER_YEAR // TICKS_PER_YEAR
TIMELINE = 0
MAX_RECORDS_PER_YEAR = 40
EMPTY_EVERY = 3  # every third channel-C year has no match
EMPTY_DIGEST = digest([])


@dataclass(frozen=True)
class Segment:
    """A sealed sim-year on the model's disk (``last_seq`` None: no record)."""

    sim_year: int
    first_seq: int
    last_seq: int | None
    lines: tuple[str, ...]
    digest: str


@dataclass(frozen=True)
class Snapshot:
    """What a serving thread reads atomically (wire_v1.md §6.2): the window and the sealed set."""

    w0: int
    next_seq: int
    s: int | None  # sealed.last_seq: the last non-empty sealed segment
    last_year: int | None


@dataclass(frozen=True)
class Planned:
    """One record of the writer's deterministic plan."""

    seq: int
    year: int
    step: int
    line: str


@dataclass
class Writer:
    """One channel's writer: a plan of records, appends, asynchronous seals, the window policy.

    ``policy`` is ``year_end`` (wire_v1.md §6.1 option b: the previous year leaves memory when
    the current one completes, once its seal is on disk) or ``sync`` (option a: the seal is
    synchronous and the sealed year leaves memory at once). ``break_i4`` is the control: the
    window moves before the dropped year is on disk.
    """

    channel: str
    plan: list[Planned]
    policy: str = "year_end"
    seal_delay: int = 0
    break_i4: bool = False
    disk: dict[int, Segment] = field(default_factory=dict)
    seal_log: list[Segment] = field(default_factory=list)
    window: list[tuple[int, bytes]] = field(default_factory=list)  # (seq, line + LF)
    w0: int = 0
    next_seq: int = 0
    s: int | None = None
    last_year: int | None = None
    pending: list[tuple[int, int, int, tuple[str, ...]]] = field(default_factory=list)
    year_first: dict[int, int] = field(default_factory=dict)
    current_year: int = 0
    cursor: int = 0  # into the plan
    tip_tick: int | None = None
    run_nonce: int = 1
    finished: bool = False
    end_reason: str = "finished"
    peak_years: int = 0

    def __post_init__(self) -> None:
        """Year 0 starts at seq 0."""
        self.year_first[0] = 0

    def snapshot(self) -> Snapshot:
        """One atomic read of the published state."""
        return Snapshot(self.w0, self.next_seq, self.s, self.last_year)

    def step(self, t: int) -> None:
        """Append the plan's records due at step ``t``; complete the year at its last tick."""
        while self.cursor < len(self.plan) and self.plan[self.cursor].step <= t:
            self._append(self.plan[self.cursor])
            self.cursor += 1
        self._complete_seals(t)
        if t % TICKS_PER_YEAR == TICKS_PER_YEAR - 1:
            self._year_completes(t)
        years = {seq_year(seq, self.year_first) for seq, _ in self.window}
        self.peak_years = max(self.peak_years, len(years))

    def _append(self, rec: Planned) -> None:
        """Append one planned line: a memory write and a published next_seq; nothing waits."""
        assert rec.seq == self.next_seq
        self.window.append((rec.seq, rec.line.encode("ascii") + LF))
        self.next_seq += 1
        self.tip_tick = rec.year * MINUTES_PER_YEAR + (rec.step % TICKS_PER_YEAR) * TICK_MINUTES

    def _year_completes(self, t: int) -> None:
        y = self.current_year
        lines = tuple(
            line[:-1].decode("ascii") for seq, line in self.window if seq >= self.year_first[y]
        )
        self.current_year = y + 1
        self.year_first[y + 1] = self.next_seq
        self.pending.append((t + self.seal_delay, y, self.year_first[y], lines))
        if self.break_i4:
            self._drop_through(y)  # the control: y leaves memory before its seal lands
        elif self.policy == "sync":
            self._complete_seals(t + self.seal_delay)
            self._drop_through(y)
        else:
            self._drop_through(y - 1)

    def _complete_seals(self, t: int) -> None:
        """Seals whose disk write is due: publish the disk, then ``s`` (I4), then the log."""
        due = [p for p in self.pending if p[0] <= t]
        self.pending = [p for p in self.pending if p[0] > t]
        for _, y, first, lines in due:
            last = first + len(lines) - 1 if lines else None
            seg = Segment(y, first, last, lines, digest(lines))
            self.disk[y] = seg
            if last is not None:
                self.s = last
            self.last_year = y
            self.seal_log.append(seg)

    def _drop_through(self, year: int) -> None:
        """Drop every year <= ``year`` once it is on disk (I4); the control does not wait."""
        drop_to = max((y for y in self.disk if y <= year), default=-1)
        if self.break_i4 and year >= 0:
            drop_to = year  # the control: the window moves whether or not the year is on disk
        if drop_to < 0:
            return
        keep_seq = self.year_first[drop_to + 1]
        self.window = [(seq, line) for seq, line in self.window if seq >= keep_seq]
        self.w0 = keep_seq

    def restart(self) -> None:
        """A `kill -9` and resume: the unsealed year is lost and re-emitted from its keyframe."""
        self.run_nonce += 1
        y = self.current_year
        self.window = [(seq, line) for seq, line in self.window if seq < self.year_first[y]]
        self.next_seq = self.year_first[y]
        self.cursor = next(i for i, rec in enumerate(self.plan) if rec.seq == self.next_seq)

    def read(self, snap: Snapshot, start: int, limit: int) -> tuple[bytes, int]:
        """The window bytes from ``start`` under ``snap``: up to ``limit`` lines, and the count."""
        offset = start - self.w0
        chunk = self.window[max(offset, 0) : max(offset, 0) + limit]
        chunk = [(seq, line) for seq, line in chunk if start <= seq < snap.next_seq]
        return b"".join(line for _, line in chunk), len(chunk)

    def read_sealed(self, lo: int, hi: int) -> tuple[str, ...]:
        """The sealed lines with ``lo <= seq <= hi``, in order, from the model's disk."""
        out: list[str] = []
        for seg in sorted(self.disk.values(), key=lambda s: s.first_seq):
            if seg.last_seq is None or seg.last_seq < lo or seg.first_seq > hi:
                continue
            a, b = max(lo, seg.first_seq) - seg.first_seq, min(hi, seg.last_seq) - seg.first_seq
            out.extend(seg.lines[a : b + 1])
        return tuple(out)


def seq_year(seq: int, year_first: dict[int, int]) -> int:
    """The sim-year a seq belongs to, from the first seq of every year."""
    return max(y for y, first in year_first.items() if first <= seq)


@dataclass
class Connection:
    """One viewer's connection as the host sees it: a cursor and its place in the seal log."""

    cursor: int | None = None  # None: no subscription
    seal_index: int = 0
    closed: bool = False


@dataclass
class Host:
    """A channel's socket server over its writer (wire_v1.md §6.2).

    The host reads the writer through one :class:`Snapshot` per answer and never writes it;
    the writer never touches a connection. Sealed notifications are pulled from the seal log.
    """

    writer: Writer
    batch: int = 64
    cap: int = 64
    connections: list[Connection] = field(default_factory=list)

    def connect(self) -> Connection:
        """Accept a connection."""
        conn = Connection(closed=len(self.connections) >= self.cap)
        if not conn.closed:
            self.connections.append(conn)
        return conn

    def close(self, conn: Connection) -> None:
        """The viewer closed: forget the subscription; nothing else changes."""
        conn.closed, conn.cursor = True, None
        if conn in self.connections:
            self.connections.remove(conn)

    def status(self) -> Json:
        """The ``stream.status`` result (wire_v1.md §4.1), from one snapshot."""
        w, snap = self.writer, self.writer.snapshot()
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
            "live": {"first_seq": snap.w0, "next_seq": snap.next_seq},
            "pace": {"band": "max", "state": "finished" if w.finished else "running"}
            if is_a
            else None,
            "record_format": f"rehoboam/stream/channel-{w.channel.lower()}/v0",
            "run_nonce": w.run_nonce,
            "sealed": {"last_seq": snap.s, "last_year": snap.last_year},
            "sifted_through": None if is_a else {"a_seq": snap.s, "a_year": snap.last_year},
            "timeline_id": TIMELINE,
            "tip": {"next_seq": snap.next_seq, "tick": w.tip_tick},
            "wire": {"format": WIRE_FORMAT, "version": VERSION},
        }

    def subscribe(self, conn: Connection, rid: int | str, frm: int | None) -> Json:
        """Answer ``stream.subscribe`` (wire_v1.md §4.2, §6.2) against one snapshot."""
        if conn.cursor is not None:
            return error(rid, "already_subscribed")
        snap = self.writer.snapshot()
        n = snap.next_seq if frm is None else frm
        if n < snap.w0:
            return error(rid, "behind_live_edge", **_behind_data(snap))
        if n > snap.next_seq:
            return error(rid, "future_seq", next_seq=snap.next_seq)
        conn.cursor, conn.seal_index = n, len(self.writer.seal_log)
        return response(rid, {"first_seq": n})

    def unsubscribe(self, conn: Connection, rid: int | str) -> Json:
        """Answer ``stream.unsubscribe``."""
        if conn.cursor is None:
            return error(rid, "not_subscribed")
        n, conn.cursor = conn.cursor, None
        return response(rid, {"next_seq": n})

    def _notify(self, method: str, params: Json) -> Frame:
        base: Json = {"channel": self.writer.channel, "timeline_id": TIMELINE}
        return control_frame(notification(method, {**base, **params}))

    def _sealed_frames(self, conn: Connection, c: int) -> list[Frame]:
        """Pull the seal-log entries the cursor has passed (an empty year at once)."""
        frames = []
        log = self.writer.seal_log
        while conn.seal_index < len(log):
            seg = log[conn.seal_index]
            if seg.last_seq is not None and seg.last_seq >= c:
                break
            frames.append(self._notify("stream.sealed", _sealed_params(seg)))
            conn.seal_index += 1
        return frames

    def serve(self, conn: Connection, budget: int) -> list[Frame]:
        """What the serving thread writes next for ``conn``: one snapshot, then bytes copied."""
        if conn.cursor is None or conn.closed:
            return []
        w, c, snap = self.writer, conn.cursor, self.writer.snapshot()
        if c < snap.w0:
            conn.cursor = None
            return [self._notify("stream.behind", {"next_seq": c, **_behind_data(snap)})]
        frames = self._sealed_frames(conn, c)
        body, count = w.read(snap, c, min(budget, self.batch))
        if count:
            frames.append(Frame(TYPE_RECORDS, records_payload(TIMELINE, w.channel, c, count, body)))
            conn.cursor = c + count
        elif w.finished:
            conn.cursor = None
            frames.append(self._notify("stream.end", {"next_seq": c, "reason": w.end_reason}))
        return frames

    def dispatch(self, conn: Connection, data: bytes) -> list[bytes]:
        """Answer one frame from a viewer (wire_v1.md §4): the host's dispatcher."""
        try:
            frame, _ = decode_frame(data, VIEWER_MAX_PAYLOAD)
        except IncompleteFrameError:
            return []
        except WireError as e:
            conn.closed = True
            reason = "unsupported_version" if "unsupported_version" in str(e) else "invalid_frame"
            extra: Json = (
                {"supported": [VERSION]} if reason == "unsupported_version" else {"detail": str(e)}
            )
            return [encode_frame(control_frame(error(None, reason, **extra)))]
        if frame.ftype == TYPE_RECORDS:
            conn.closed = True
            return [
                encode_frame(
                    control_frame(
                        error(None, "invalid_frame", detail="a RECORDS frame from a viewer")
                    )
                )
            ]
        return [encode_frame(control_frame(self._answer(conn, frame.payload)))]

    def _answer(self, conn: Connection, payload: bytes) -> Json:
        try:
            kind, obj = decode_control(payload)
        except WireError as e:
            text = str(e)
            reason = "parse_error" if text.startswith("parse_error") else "invalid_request"
            if text.startswith("invalid_params"):
                reason = "invalid_params"
            return error(None, reason, detail=text)
        if kind != "request":
            return error(None, "invalid_request", detail=f"a {kind} from a viewer")
        rid = obj["id"]
        assert isinstance(rid, (int, str))
        method, params = obj["method"], obj.get("params")
        if method not in METHODS:
            return error(rid, "method_not_found")
        try:
            return self._route(conn, rid, str(method), params)
        except WireError as e:
            return error(rid, "invalid_params", detail=str(e))

    def _route(self, conn: Connection, rid: int | str, method: str, params: object) -> Json:
        if method == "stream.subscribe":
            return self.subscribe(conn, rid, subscribe_from(params))
        no_params(params)
        if method == "stream.status":
            return response(rid, self.status())
        return self.unsubscribe(conn, rid)


def _behind_data(snap: Snapshot) -> Json:
    return {
        "live_first_seq": snap.w0,
        "sealed_last_seq": snap.s,
        "sealed_last_year": snap.last_year,
    }


def _sealed_params(seg: Segment) -> Json:
    return {
        "canonical_digest": seg.digest,
        "first_seq": seg.first_seq,
        "last_seq": seg.last_seq,
        "sim_year": seg.sim_year,
    }


# ---------------------------------------------------------------------------------------- viewers
def _behind_event(data: Json, cursor: int) -> Event:
    s = data["sealed_last_seq"]
    return Event(
        "behind",
        first_seq=cursor,
        sealed_last_seq=-1 if s is None else int(str(s)),
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


def _result(reply: Json, key: str) -> int:
    result = reply["result"]
    assert isinstance(result, dict)
    return int(str(result[key]))


@dataclass
class Viewer:
    """A cursor over a host: polls at its own rate, runs :class:`Catchup`, records what it got."""

    name: str
    host: Host
    period: int = 1  # polls every ``period`` steps
    budget: int = 64  # records it reads per poll
    disconnect_every: int = 0  # reconnecting viewer: drop the connection every n polls
    jump_at: int = -1  # jumping viewer: the step at which it jumps to live
    start_at: int = 0  # late viewer: the step it first connects
    conn: Connection | None = None
    loop: Catchup = field(default_factory=Catchup)
    received: list[tuple[int, str]] = field(default_factory=list)
    live_bodies: dict[int, bytearray] = field(default_factory=dict)  # year -> bytes seen live
    live_counts: dict[int, int] = field(default_factory=dict)
    sealed_seen: dict[int, str] = field(default_factory=dict)  # year -> announced digest
    history: Json | None = None
    behinds: int = 0
    retries: int = 0
    waits: int = 0
    polls: int = 0
    rid: int = 0
    done: bool = False
    end_reason: str = ""
    errors: list[str] = field(default_factory=list)

    def on_closed(self) -> None:
        """The host dropped the connection."""
        self._apply(self.loop.next(Event("closed")))

    def _apply(self, action: Action) -> None:
        """Carry out one action of the loop."""
        calls: dict[str, Callable[[Action], None]] = {
            "subscribe": lambda a: self._subscribe(a.a),
            "subscribe_live": lambda _: self._subscribe(None),
            "read_sealed": self._read_sealed,
            "status": lambda _: self._reconnect(),
            "unsubscribe": lambda _: self._unsubscribe(),
            "wait": self._wait,
            "done": self._finish,
            "close": self._stop,
            "stop": self._stop,
        }
        call = calls.get(action.kind)
        if call is not None:
            call(action)

    def _read_sealed(self, action: Action) -> None:
        lines = self.host.writer.read_sealed(action.a, action.b)
        self._deliver(action.a, lines, live=False)
        self._apply(self.loop.next(Event("read_sealed_done", action.a, len(lines))))

    def _wait(self, action: Action) -> None:
        self.waits = action.a
        self.retries += 1

    def _finish(self, action: Action) -> None:
        self.done, self.end_reason = True, action.detail
        if self.conn is not None:
            self.host.close(self.conn)

    def _stop(self, action: Action) -> None:
        self.errors.append(action.detail)
        if self.conn is not None:
            self.host.close(self.conn)

    def _call(self, method: str, params: Json | None = None) -> Json:
        """One request through the wire and the host's dispatcher, back through the wire."""
        assert self.conn is not None
        self.rid += 1
        replies = self.host.dispatch(
            self.conn, encode_frame(control_frame(request(self.rid, method, params)))
        )
        frame, _ = decode_frame(replies[0])
        _, obj = decode_control(frame.payload)
        return obj

    def _reconnect(self) -> None:
        self.conn = self.host.connect()
        if self.conn.closed:  # the cap: too_many_viewers before any request
            self._apply(self.loop.next(Event("refused", reason="too_many_viewers")))
            return
        try:
            status = _status_of(self._call("stream.status"))
        except WireError as e:
            self._apply(Action("stop", detail=f"status refused: {e}"))
            return
        same = self.history is None or status["history"] == self.history
        self.history = status["history"] if isinstance(status["history"], dict) else None
        self._apply(self.loop.next(Event("status", same_run=same)))

    def _subscribe(self, seq: int | None) -> None:
        if self.conn is None or self.conn.closed:
            self._apply(self.loop.next(Event("closed")))
            return
        params: Json = {"from": "live" if seq is None else {"seq": seq}}
        reply = self._call("stream.subscribe", params)
        reason = _reason(reply)
        if reason == "ok":
            self._apply(self.loop.next(Event("subscribed", _result(reply, "first_seq"))))
        elif reason == "behind_live_edge":
            self.behinds += 1
            err = reply["error"]
            assert isinstance(err, dict) and isinstance(err["data"], dict)
            self._apply(self.loop.next(_behind_event(err["data"], self.loop.cursor)))
        else:
            self._apply(self.loop.next(Event("refused", reason=reason)))

    def _unsubscribe(self) -> None:
        reply = self._call("stream.unsubscribe")
        self._apply(self.loop.next(Event("unsubscribed", _result(reply, "next_seq"))))

    def _deliver(self, first_seq: int, lines: Sequence[str], *, live: bool) -> None:
        for i, line in enumerate(lines):
            self.received.append((first_seq + i, line))
            if live:
                year = seq_year(first_seq + i, self.host.writer.year_first)
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
            self._apply(self.loop.next(_behind_event(params, int(str(params["next_seq"])))))
        elif method == "stream.end":
            self._apply(self.loop.next(Event("end", reason=str(params["reason"]))))
        return 0

    def step(self, t: int, *, drain: bool = False) -> None:
        """One step of wall time: poll if due (always when draining), jump or drop if scripted."""
        if self.done or t < self.start_at or (not drain and t % self.period):
            return
        self.polls += 1
        if self.waits:
            self.waits -= 1
            if self.waits == 0:
                self._apply(self.loop.next(Event("retry_elapsed")))
            return
        if self.conn is None:
            self._apply(self.loop.start())
        if t == self.jump_at:
            self._apply(self.loop.next(Event("jump")))
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


def _status_of(reply: Json) -> Json:
    return check_status(reply["result"])


# ---------------------------------------------------------------------------------------- the plan
def _make_line(rng: random.Random, channel: str, seq: int, tick: int) -> str:
    """A canonical line of a random kind of ``channel`` with the given header."""
    kinds = [k for k, ch in KIND_CHANNEL.items() if ch == channel]
    rec = _CANON.generate(rng, rng.choice(kinds))
    rec = rec.replace(timeline_id=TIMELINE, seq=seq, tick=tick)
    if rec.kind == "MATCH" and int(rec["opened_tick"]) > tick:
        rec = rec.replace(opened_tick=tick)
    return str(_CANON.render(rec))


@dataclass(frozen=True)
class Scenario:
    """One simulated run."""

    seed: int
    years: int
    channel: str = "A"
    policy: str = "year_end"
    seal_delay: int = 0
    restart_at: int = -1
    cap: int = 64
    break_i4: bool = False


def make_plan(sc: Scenario) -> list[Planned]:
    """The run's records: per year a count (0 allowed on channel C), each at a step of its year."""
    rng = random.Random(sc.seed)
    plan: list[Planned] = []
    seq = 0
    for year in range(sc.years):
        count = rng.randint(1, MAX_RECORDS_PER_YEAR)
        if sc.channel == "C" and year % EMPTY_EVERY == 1:
            count = 0  # channel C seals a sim-year with no match (channel_c_v0.sql R7)
        steps = sorted(rng.randrange(TICKS_PER_YEAR) for _ in range(count))
        for offset in steps:
            step = year * TICKS_PER_YEAR + offset
            rng_seq = random.Random(sc.seed * 1_000_003 + seq)
            line = _make_line(rng_seq, sc.channel, seq, step * TICK_MINUTES)
            plan.append(Planned(seq, year, step, line))
            seq += 1
    return plan


def _viewers(host: Host, years: int) -> list[Viewer]:
    """The viewer mix: fast, slow, crawling, stalled, reconnecting, jumping, late."""
    steps = years * TICKS_PER_YEAR
    return [
        Viewer("fast", host, period=1, budget=1_000),
        Viewer("slow", host, period=3, budget=5),
        Viewer("crawl", host, period=TICKS_PER_YEAR * 3, budget=7),
        Viewer("stalled", host, period=steps + 1, budget=100_000),
        Viewer("reconnecting", host, period=2, budget=9, disconnect_every=5),
        Viewer("jumper", host, period=1, budget=50, jump_at=steps // 2),
        Viewer("late", host, period=1, budget=20, start_at=steps // 3),
    ]


DRAIN_LIMIT = 100_000


def simulate(sc: Scenario) -> tuple[Writer, list[Viewer]]:
    """Run a writer for ``sc.years`` sim-years with the viewer mix attached; drain at the end."""
    rng = random.Random(sc.seed)
    writer = Writer(sc.channel, make_plan(sc), sc.policy, sc.seal_delay, sc.break_i4)
    host = Host(writer, batch=rng.choice([1, 8, 64, 1_000]), cap=sc.cap)
    viewers = _viewers(host, sc.years)
    steps = sc.years * TICKS_PER_YEAR
    for t in range(steps):
        if t == sc.restart_at:
            writer.restart()  # the host died and resumed: every connection dropped
            for v in viewers:
                if v.conn is not None and not v.conn.closed:
                    host.close(v.conn)
                    v.on_closed()
        writer.step(t)
        for v in viewers:
            v.step(t)
    writer.step(steps + max(sc.seal_delay, 1))  # the last seal lands
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
    skipped, start = v.loop.skipped, 0
    if skipped is None:
        if seqs != list(range(start, next_seq)):
            return [
                f"{v.name}: seqs are not exactly {start}..{next_seq - 1} ({len(seqs)} received)"
            ]
        return []
    before = [s for s in seqs if s < skipped[0]]
    after = [s for s in seqs if s >= skipped[0]]
    if before != list(range(start, skipped[0])) or after != list(range(skipped[1], next_seq)):
        return [f"{v.name}: seqs around the jump {skipped} are not contiguous"]
    return []


def check_viewer(v: Viewer, writer: Writer) -> list[str]:
    """The delivery properties one viewer must satisfy (wire_v1.md §6.3; V-STR-05)."""
    problems = list(v.errors)
    if not v.done:
        problems.append(f"{v.name}: did not reach stream.end")
    problems.extend(_check_seqs(v, writer.next_seq))
    lines = dict(v.received)
    for seg in writer.disk.values():
        wanted = range(seg.first_seq, seg.last_seq + 1) if seg.last_seq is not None else range(0)
        if all(s in lines for s in wanted) and digest(lines[s] for s in wanted) != seg.digest:
            problems.append(f"{v.name}: year {seg.sim_year} differs from the sealed digest")
        whole = v.live_counts.get(seg.sim_year, 0) == len(wanted)
        announced = v.sealed_seen.get(seg.sim_year)
        if whole and announced is not None:
            body = bytes(v.live_bodies.get(seg.sim_year, b""))
            if _CANON.xxh3_128(body).hex() != announced:
                problems.append(f"{v.name}: live body of year {seg.sim_year} != stream.sealed")
    return problems


SCENARIOS = (
    Scenario(1, 6, seal_delay=3),
    Scenario(2, 10, policy="sync"),
    Scenario(3, 5, channel="C", seal_delay=2),
    Scenario(4, 12, seal_delay=1, restart_at=50),
    Scenario(5, 6, seal_delay=5, cap=6),
)


def _scenario_checks(sc: Scenario, writer: Writer, viewers: list[Viewer]) -> list[str]:
    """What the whole run must show: the window bound, the sealed path, retries, the fast viewer."""
    problems: list[str] = []
    bound = 1 if sc.policy == "sync" else 2
    if writer.peak_years > bound:
        problems.append(f"the window held {writer.peak_years} years, the bound is {bound}")
    if not any(v.behinds for v in viewers):
        problems.append("no viewer exercised the sealed path")
    if sc.policy == "year_end" and any(v.behinds for v in viewers if v.name == "fast"):
        problems.append("the fast viewer fell off the edge")
    if sc.restart_at >= 0 and not any(v.retries for v in viewers):
        problems.append("no viewer retried after the restart")
    if sc.cap < len(viewers) and not any(v.retries for v in viewers if v.name == "late"):
        problems.append("the late viewer was never told too_many_viewers")
    if sc.channel == "C" and not any(seg.last_seq is None for seg in writer.disk.values()):
        problems.append("no empty channel-C year")
    fast = next(v for v in viewers if v.name == "fast")
    if sc.policy == "year_end" and len(fast.sealed_seen) != len(writer.disk):
        problems.append(f"fast saw {len(fast.sealed_seen)} stream.sealed of {len(writer.disk)}")
    return [f"seed {sc.seed}: {p}" for p in problems]


def _live_edge_checks(scenarios: Sequence[Scenario] = SCENARIOS) -> list[str]:
    """Every viewer of every scenario gets 0..next_seq-1 exactly, with the sealed digests."""
    problems: list[str] = []
    for sc in scenarios:
        writer, viewers = simulate(sc)
        for v in viewers:
            problems.extend(f"seed {sc.seed}: {p}" for p in check_viewer(v, writer))
        problems.extend(_scenario_checks(sc, writer, viewers))
    return problems


# --------------------------------------------------------------------------------------- seal races
def _seal_steps(policy: str) -> list[Callable[[Writer], None]]:
    """The writer's micro-steps of one seal, in the order I4 fixes: disk, s, then w0."""

    def disk(w: Writer) -> None:
        w.disk[0] = Segment(0, 0, 9, (), EMPTY_DIGEST)

    def publish_s(w: Writer) -> None:
        w.s = 9

    def move_w0(w: Writer) -> None:
        w.w0 = 10

    return [disk, publish_s, move_w0] if policy == "ordered" else [disk, move_w0, publish_s]


def _interleaved(order: str, read: str) -> bool:
    """Whether a reader's loads, interleaved with a seal's steps, can ever show I1 broken."""
    steps = _seal_steps(order)
    for k in range(len(steps) + 1):  # the reader's first load happens after k steps
        for j in range(k, len(steps) + 1):  # its second load after j steps
            w = Writer("A", [], next_seq=10)
            for s in steps[:k]:
                s(w)
            s_seen: int | None
            w0_seen: int | None
            if read == "snapshot":
                snap = w.snapshot()
                s_seen, w0_seen = snap.s, snap.w0
            else:
                first: int | None = w.s if read == "s_then_w0" else w.w0
                for s in steps[k:j]:
                    s(w)
                second: int | None = w.w0 if read == "s_then_w0" else w.s
                s_seen, w0_seen = (first, second) if read == "s_then_w0" else (second, first)
            if (-1 if s_seen is None else s_seen) + 1 < (w0_seen or 0):
                return True
    return False


def _race_checks() -> list[str]:
    """R4: one snapshot never shows I1 broken; `w0` before `s` neither; `s` then `w0` can."""
    problems = []
    if _interleaved("ordered", "snapshot"):
        problems.append("a snapshot read showed I1 broken")
    if _interleaved("ordered", "w0_then_s"):
        problems.append("reading w0 before s showed I1 broken under I4")
    if not _interleaved("ordered", "s_then_w0"):
        problems.append("the control: reading s then w0 never showed I1 broken (it must)")
    if not _interleaved("unordered", "w0_then_s"):
        problems.append("the control: a writer moving w0 before s never showed I1 broken (it must)")
    return problems


# ---------------------------------------------------------------------------------------- self-test
Check = tuple[str, Callable[[], list[str]]]


def golden_records(channel: str, count: int) -> Records:
    """The first ``count`` lines of the canonical golden file's segment of ``channel``."""
    lines = CANONICAL_GOLDEN.read_text(encoding="ascii").splitlines()
    start = next(i for i, ln in enumerate(lines) if ln.startswith(f"segment 0 {channel} 0 "))
    return Records(TIMELINE, channel, 0, tuple(lines[start + 1 : start + 1 + count]))


def _records_a() -> Records:
    return golden_records("A", 3)


def _records_c() -> Records:
    return golden_records("C", 1)


def _frame_round_trips() -> list[str]:
    """Every frame type encodes, decodes to itself, and short buffers say incomplete."""
    problems = []
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
    a_status = Host(Writer("A", [])).status()
    c_status = Host(Writer("C", [])).status()
    rec = _records_a()
    behind: Json = {"live_first_seq": 1200, "sealed_last_seq": 1199, "sealed_last_year": 3}
    sealed: Json = {
        "canonical_digest": digest(rec.lines),
        "channel": "A",
        "first_seq": 0,
        "last_seq": len(rec.lines) - 1,
        "sim_year": 0,
        "timeline_id": 0,
    }
    empty: Json = {
        **sealed,
        "canonical_digest": EMPTY_DIGEST,
        "channel": "C",
        "first_seq": 3,
        "last_seq": None,
        "sim_year": 1,
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
        ("notify_sealed_empty", control_frame(notification("stream.sealed", empty))),
        (
            "notify_behind",
            control_frame(notification("stream.behind", {**tl, "next_seq": 850, **behind})),
        ),
        (
            "notify_end",
            control_frame(
                notification("stream.end", {**tl, "next_seq": 1234, "reason": "finished"})
            ),
        ),
        (
            "notify_end_aborted",
            control_frame(
                notification("stream.end", {**tl, "next_seq": 1234, "reason": "aborted"})
            ),
        ),
        ("records_a", records_frame(rec)),
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
    problems = []
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


def _reframe(
    records: Records, channel: str | None = None, *, mutate: Callable[[str], str] | None = None
) -> bytes:
    """A RECORDS frame built without the sender's checks, for a planted refusal."""
    lines = list(records.lines)
    if mutate is not None:
        lines[1] = mutate(lines[1])
    head = SUBHEADER.pack(
        records.timeline_id,
        records.first_seq,
        len(lines),
        CHANNEL_BYTE[channel or records.channel],
        RESERVED,
    )
    return encode_frame(Frame(TYPE_RECORDS, head + b"".join(ln.encode() + LF for ln in lines)))


def _deep(depth: int) -> bytes:
    return (
        b'{"id":1,"jsonrpc":"2.0","method":"m","params":'
        + b'{"a":' * depth
        + b"1"
        + b"}" * depth
        + b"}"
    )


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
        (
            "control_unknown_member",
            b'{"id":1,"jsonrpc":"2.0","method":"stream.status","x":1}',
            "unknown member",
        ),
        ("control_too_deep", _deep(MAX_DEPTH), "depth"),
        ("control_recursion", b"[" * 100_000, "parse_error"),
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
        ("records_wrong_channel", _reframe(_records_a(), channel="C"), "kind"),
        ("records_lowercase_kind", _reframe(_records_a(), mutate=str.lower), "kind"),
        (
            "records_short_prefix",
            _reframe(_records_a(), mutate=lambda ln: ln.split(" ")[0]),
            "prefix",
        ),
        ("records_non_ascii", _reframe(_records_a(), mutate=lambda ln: ln + "é"), "ASCII"),
        (
            "records_leading_zero",
            _reframe(_records_a(), mutate=lambda ln: ln.replace(" 1 ", " 01 ", 1)),
            "integer",
        ),
    ]


def _decode_any(data: bytes) -> None:
    frame, _ = decode_frame(data)
    if frame.ftype == TYPE_RECORDS:
        decode_records(frame.payload)
    else:
        decode_control(frame.payload)


def _refusal_checks() -> list[str]:
    """Every planted frame is refused with a message naming the reason."""
    problems = []
    for name, data, reason in _refusals():
        try:
            _decode_any(data)
            problems.append(f"{name}: accepted")
        except WireError as e:
            if reason not in str(e):
                problems.append(f"{name}: refused for {e!r}, expected {reason!r}")
    return problems


def _dispatch_cases() -> list[tuple[str, bytes]]:
    """Planted frames from a viewer and what the host's dispatcher answers (golden `dispatch`)."""
    status = encode_frame(control_frame(request(1, "stream.status")))
    return [
        ("dispatch_status", status),
        (
            "dispatch_player_act",
            encode_frame(control_frame(request(2, "player.act", {"verb_id": 3}))),
        ),
        (
            "dispatch_timeline_fork",
            encode_frame(control_frame(request(3, "timeline.fork", {"tick": 0}))),
        ),
        (
            "dispatch_stream_pace",
            encode_frame(control_frame(request(4, "stream.pace", {"band": "max"}))),
        ),
        (
            "dispatch_status_with_param",
            encode_frame(control_frame(request(5, "stream.status", {"x": 1}))),
        ),
        (
            "dispatch_subscribe_filter",
            encode_frame(
                control_frame(request(6, "stream.subscribe", {"from": "live", "filter": {}}))
            ),
        ),
        (
            "dispatch_subscribe_float",
            encode_frame(
                Frame(
                    TYPE_CONTROL,
                    b'{"id":7,"jsonrpc":"2.0","method":"stream.subscribe","params":{"from":{"seq":1.5}}}',
                )
            ),
        ),
        (
            "dispatch_subscribe_future",
            encode_frame(control_frame(request(8, "stream.subscribe", {"from": {"seq": 99}}))),
        ),
        (
            "dispatch_unsubscribe_none",
            encode_frame(control_frame(request(9, "stream.unsubscribe"))),
        ),
        ("dispatch_notification", encode_frame(control_frame(notification("stream.sealed", {})))),
        ("dispatch_batch", encode_frame(Frame(TYPE_CONTROL, b"[]"))),
        ("dispatch_not_json", encode_frame(Frame(TYPE_CONTROL, b"{"))),
        ("dispatch_version_2", status[:4] + b"\x02\x00" + status[6:]),
        ("dispatch_records_from_viewer", encode_frame(records_frame(_records_a()))),
        (
            "dispatch_oversize",
            status[:8]
            + struct.pack("<I", VIEWER_MAX_PAYLOAD + 1)
            + b"\0" * (VIEWER_MAX_PAYLOAD + 1),
        ),
    ]


def dispatch_transcript() -> list[tuple[str, bytes, bytes]]:
    """Each planted request and the host's reply bytes, on a fresh connection."""
    out = []
    for name, data in _dispatch_cases():
        host = Host(Writer("A", []))
        replies = host.dispatch(host.connect(), data)
        out.append((name, data, b"".join(replies)))
    return out


EXPECTED_REASONS = {
    "dispatch_status": "ok",
    "dispatch_player_act": "method_not_found",
    "dispatch_timeline_fork": "method_not_found",
    "dispatch_stream_pace": "method_not_found",
    "dispatch_status_with_param": "invalid_params",
    "dispatch_subscribe_filter": "invalid_params",
    "dispatch_subscribe_float": "invalid_request",
    "dispatch_subscribe_future": "future_seq",
    "dispatch_unsubscribe_none": "not_subscribed",
    "dispatch_notification": "invalid_request",
    "dispatch_batch": "invalid_request",
    "dispatch_not_json": "parse_error",
    "dispatch_version_2": "unsupported_version",
    "dispatch_records_from_viewer": "invalid_frame",
    "dispatch_oversize": "invalid_frame",
}


def _dispatch_checks() -> list[str]:
    """The host answers only the three methods; everything else is the read-only refusal."""
    problems = []
    for name, _, reply in dispatch_transcript():
        frame, used = decode_frame(reply)
        _, obj = decode_control(frame.payload)
        if used != len(reply):
            problems.append(f"{name}: more than one reply frame")
        if _reason(obj) != EXPECTED_REASONS[name]:
            problems.append(f"{name}: {_reason(obj)}, expected {EXPECTED_REASONS[name]}")
    err = error(7, "method_not_found")["error"]
    assert isinstance(err, dict)
    if err["message"] != READ_ONLY_MESSAGE:
        problems.append("method_not_found does not carry the read-only message")
    return problems


def _host_checks() -> list[str]:
    """Subscribe and unsubscribe answers of a small host: future, behind, already, not."""
    problems = []
    sc = Scenario(9, 2, policy="sync")
    writer = Writer("A", make_plan(sc), "sync")
    host = Host(writer)
    for t in range(TICKS_PER_YEAR + 1):
        writer.step(t)
    conn = host.connect()
    first_of_year_1 = writer.year_first[1]
    cases = [
        (host.subscribe(conn, 1, writer.next_seq + 1), "future_seq"),
        (host.subscribe(conn, 2, 0), "behind_live_edge"),
        (host.unsubscribe(conn, 3), "not_subscribed"),
        (host.subscribe(conn, 4, first_of_year_1), "ok"),
        (host.subscribe(conn, 5, first_of_year_1), "already_subscribed"),
        (host.unsubscribe(conn, 6), "ok"),
    ]
    for reply, want in cases:
        if _reason(reply) != want:
            problems.append(f"expected {want}, got {_reason(reply)}")
    status = check_status(host.status())
    if status["live"] != {"first_seq": first_of_year_1, "next_seq": writer.next_seq}:
        problems.append(f"window after the seal is {status['live']}")
    return problems


CATCHUP_SCENARIO: tuple[Event, ...] = (
    Event("behind", sealed_last_seq=99, live_first_seq=100),
    Event("read_sealed_done", 0, 100),
    Event("subscribed", 100),
    Event("records", 100, 50),
    Event("sealed"),
    Event("closed"),
    Event("status", same_run=True),
    Event("refused", reason="too_many_viewers"),
    Event("retry_elapsed"),
    Event("subscribed", 150),
    Event("records", 150, 1),
    Event("behind", 151, sealed_last_seq=400, live_first_seq=401),
    Event("read_sealed_done", 151, 250),
    Event("refused", reason="future_seq"),
    Event("retry_elapsed"),
    Event("subscribed", 401),
    Event("records", 402, 1),
    Event("records", 403, 1),
    Event("closed"),
    Event("status", same_run=True),
    Event("subscribed", 401),
    Event("records", 401, 3),
    Event("jump"),
    Event("unsubscribed", 404),
    Event("subscribed", 900),
    Event("records", 900, 2),
    Event("end", reason="finished"),
    Event("records", 902, 1),
)
CATCHUP_CLOSES = 1  # the out-of-order frame (402 before 401)
CATCHUP_END = "action done 902 0 finished"


def _fmt_event(ev: Event) -> str:
    return (
        f"{ev.kind} {ev.first_seq} {ev.count} {ev.sealed_last_seq} {ev.live_first_seq} "
        f"{_CANON.encode_str(ev.reason) if ev.reason else '%'} {int(ev.same_run)}"
    )


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
    closes = [ln for ln in lines if ln.startswith("action close")]
    problems = []
    if len(closes) != CATCHUP_CLOSES:
        problems.append(
            f"{len(closes)} closes in the transcript, expected {CATCHUP_CLOSES}: {closes}"
        )
    if "action ignore 0 0 %" not in lines[lines.index(closes[0]) + 1 : lines.index(closes[0]) + 5]:
        problems.append("after a close, the next records frame was not ignored")
    if CATCHUP_END not in lines:
        problems.append("the loop did not finish at 902 with reason finished")
    if sum(ln.startswith("action wait") for ln in lines) != 2:  # noqa: PLR2004 (two refusals)
        problems.append("the two retriable refusals did not each produce a wait")
    return problems


def self_test(control: str | None = None) -> list[str]:
    """Run every check; return the report lines (the last says PASS or FAIL)."""
    live_edge: Check = (
        "live-edge rule: fast, slow, crawling, stalled, reconnecting, jumping, late viewers; "
        "both window policies; empty channel-C years; a restart; the connection cap",
        _live_edge_checks,
    )
    if control == "window-gap":
        live_edge = (
            "live-edge rule (control: the window moves before the dropped year is on disk)",
            lambda: _live_edge_checks(
                [
                    Scenario(1, 6, seal_delay=3, break_i4=True),
                    Scenario(2, 6, seal_delay=2, break_i4=True),
                ]
            ),
        )
    checks: list[Check] = [
        ("frame round trips and incomplete buffers", _frame_round_trips),
        ("every message: canonical, decoded, classified", _message_checks),
        ("refusals: bad frames, payloads and line prefixes", _refusal_checks),
        (
            "the host's dispatcher: read-only method table, parameter refusals, caps",
            _dispatch_checks,
        ),
        ("host answers: future, behind, already, not subscribed, window", _host_checks),
        live_edge,
        ("seal races: one snapshot, and w0 before s, never show I1 broken", _race_checks),
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
        "# World Stream wire format v1: golden frames, refusals, dispatcher answers, Catchup.",
        "# GENERATED by specs/stream/selftest_wire_v1.py; do not edit. Contract: wire_v1.md.",
        "# `frame <name> <hex>`: one frame, header and payload; then its decoded form:",
        "#   `control <canonical json>` or `records <timeline> <channel> <first_seq> <count>`",
        "#   followed by `line <canonical line>` per record. rh-protocol encodes the decoded form",
        "#   and must produce the hex; it decodes the hex and must produce the decoded form.",
        "# `refuse <name> <hex> <reason>`: a frame the receiver refuses, naming <reason>.",
        "# `dispatch <name> <request hex> <reply hex>`: a frame from a viewer, the host's answer",
        "#   on a fresh connection to a host with no record (wire_v1.md §4).",
        "# `scenario catchup` then `action ...` / `event ...` pairs: Catchup (wire_v1.md §6.3)",
        "#   driven by events `<kind> <first_seq> <count> <sealed_last_seq> <live_first_seq>",
        "#   <reason> <same_run>`, answering `<kind> <a> <b> <detail>` (strings as tokens).",
        "# The record lines are golden/canonical_v0.txt's; a canonical change regenerates this.",
    ]
    body: list[str] = []
    for name, frame in _messages():
        body.append(f"frame {name} {encode_frame(frame).hex()}")
        body.extend(_decoded(frame))
    for name, data, reason in _refusals():
        body.append(f"refuse {name} {data.hex()} {_CANON.encode_str(reason)}")
    for name, data, reply in dispatch_transcript():
        body.append(f"dispatch {name} {data.hex()} {reply.hex()}")
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
    """Run the self-test, then write or check the golden file (never with --control)."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the file to write or check")
    parser.add_argument("--control", choices=["window-gap"], help="a planted fault that must FAIL")
    args = parser.parse_args(argv)
    sys.setrecursionlimit(10_000)
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
