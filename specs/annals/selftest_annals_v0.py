r"""Family annals v0: the exact reference renderer and its self-test (SPR-029; annals_v0.md).

The renderer reads a channel-A chronicle in the canonical text form (``canonical_v0.py`` parses
and checks it), the Registry (the verb and attribute dictionaries the chronicle header names), a
world file (the calendar), a names file (what the chronicle does not carry: the manifest's cast,
dynasty and title names) and, for a house annal, a keyframe extract (the rows of the genesis
keyframe that open a house whose past is not in channel A: REV-SPR-019 R1). It renders annals
from ``templates_v0.toml`` with a provenance sidecar (one row per sentence: the event ids it
shows, the persons it designates, the records it read, the template id and version), a header
naming every input by digest, and a digest of text and sidecar; the groups of one world are
listed with their digests in an annals manifest.

Checks (``--check``):

  1. provenance (V-NAR-05; REV-SPR-029 R2, R19): the group is re-rendered from its inputs and
     the text and the sidecar are compared line by line and field by field, in list order, naming
     the first field that differs; a ``P`` row comes only before the first year heading, cites no
     event and cites keyframe rows that exist (D50 as ruled). Controls: ``no-event-id``,
     ``row-drops-own``, ``body-as-prologue``, ``missing-kf-source``, ``unrelated-event``,
     ``rows-swapped``, ``prologue-altered``, ``foreign-root``, ``forged-source``,
     ``text-truncated``;
  2. template slots (annals_v0.md §3, ``lint_templates.py``); controls ``undeclared-slot`` and
     ``state-read``;
  3. no model (V-NAR-07): after rendering, no network or process module is loaded; the imports
     of this file are the standard library and the sibling references; control ``network-import``;
  4. the calendar (05.1 §4.1): known answers for 360- and 365-day years, before and after the
     epoch; ordinal and cardinal words to 999,999; the q8_8 rendering;
  5. the fixtures: every ``EVENT`` agrees with the dictionary the chronicle header names, and
     the chronicle is a possible history under M2's and M7's rules (annals_v0.md §12); controls
     ``fixture-deposed-day-one`` and ``fixture-claim-untriggered``;
  6. determinism: a render in this process equals renders in two other processes under different
     ``PYTHONHASHSEED`` values, and every committed sidecar's digest recomputes from the files;
  7. coverage: every in-scope kind is exercised by the fixtures (unexercised variants are reported);
  8. golden annals: the committed text, sidecar and manifest of every fixture equal the render,
     Edric Ashcombe's line included beyond the manifest's groups, and no file in ``golden/`` is
     without a rendered group (``STALE:`` names each file that differs or is an orphan;
     ``--write`` regenerates them);
  9. designation is injective: no two persons of one annal share a designation, and a planted
     shared name is disambiguated with the compact label; control ``shared-name``;
 10. the extract digest is canonical: a shuffled extract gives the same digest;
 11. the chronicle fold (annals_v0.md §9.1; AN-0): a chronicle file of several ``#segment``
     blocks folds to known answers derived outside this file and re-derived here from the XXH3
     reference; one block gives the plain ``canonical_digest``, no block the one-segment form; a
     changed byte changes the fold; free comments anywhere leave it unchanged; every refusal of
     §9.1 is planted and named, every file ``canonical_v0.py`` refuses is refused with its words,
     and the load path refuses a block holding a record of another sim-year; a comment hiding a
     record behind a form feed renders nothing; ``fold`` and ``segment`` refuse an unreadable file
     cleanly, and ``segment`` cuts by the world's calendar; each fixture cut by sim-year
     (``solitary`` into two segments) folds as first principles say and renders the same annals,
     only the header's ``chronicle_digest`` changed (``bin/check_all.sh`` adds the control: a cut
     with one ``#segment`` digest altered must FAIL ``fold``).

Run: python -I specs/annals/selftest_annals_v0.py --check [--golden-dir DIR] [--control NAME]
     python -I specs/annals/selftest_annals_v0.py --write
     python -I specs/annals/selftest_annals_v0.py render --world DIR \
         --group house:<id>|year|line:<id> [--sidecar] [--out FILE]
     python -I specs/annals/selftest_annals_v0.py verify --text FILE --sidecar FILE
     python -I specs/annals/selftest_annals_v0.py fold FILE      (exit 1 and FOLD REFUSED: on a
         refusal; else one line per segment or one-segment line, then chronicle_digest=<32 hex>)
     python -I specs/annals/selftest_annals_v0.py segment --world DIR [--out FILE]   (exit 1 and
         SEGMENT REFUSED: on a refusal)
"""

from __future__ import annotations

import argparse
import contextlib
import dataclasses
import importlib
import importlib.util
import io
import os
import re
import sys
import tempfile
import tomllib
from collections import Counter
from dataclasses import dataclass, field
from decimal import Decimal
from pathlib import Path
from types import ModuleType
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
TEMPLATES = HERE / "templates_v0.toml"
FIXTURES = HERE / "fixtures"
GOLDEN = HERE / "golden"
CANONICAL = ROOT / "specs" / "stream" / "canonical_v0.py"
HASH_REF = ROOT / "bin" / "gen_hash_golden.py"
LINT = HERE / "lint_templates.py"
REGISTRY = ROOT / "specs" / "manifest" / "registry_v0.toml"
REGISTRY_DELTA = ROOT / "specs" / "systems" / "next" / "registry_delta.toml"
FORMAT = "rehoboam/annals/v0"
MANIFEST_FORMAT = "rehoboam/annals/manifest/v0"
EXTRACT_FORMAT = "rehoboam/annals/keyframe-extract/v0"
RENDERER = "annals_v0/0.3.0"
D_ANNALS = b"rehoboam/annals/v0"
D_EXTRACT = b"rehoboam/annals/keyframe-extract/v0"
EXIT_OK, EXIT_STALE, EXIT_SELFTEST = 0, 1, 2

MINUTES_PER_DAY = 1440
MONTHS = {360: (30,) * 12, 365: (31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)}
GESTATION_MONTHS = 9  # M2 0.3.0 F9: a birth in the ninth month after its conception
WORDS_LIMIT = 1_000_000  # the words table's range (REV-SPR-029 R25)
UNITS = (
    "",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
)
TENS = ("", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety")
ORDINAL_UNITS = {
    "one": "first",
    "two": "second",
    "three": "third",
    "five": "fifth",
    "eight": "eighth",
    "nine": "ninth",
    "twelve": "twelfth",
}
ACTOR, TARGET, WITNESS, INSTRUMENT = 0, 1, 2, 3
ROLE_CODES = {"actor": ACTOR, "target": TARGET, "instrument": INSTRUMENT}
PERSON = 1
BORN, FOUNDER = 0, 1  # life-course origins
EDGE_CODES = {"contingent": 0, "triggered": 1, "motivated_by": 3}
TRIGGERED, MOTIVATED_BY = 1, 3
ONE = 256
Q8_8_DIGITS = 8
M7_KINDS = frozenset(
    {
        "SUCCEED",
        "INTERREGNUM_CRISIS",
        "DEPOSED",
        "REGENCY_BEGUN",
        "REGENCY_ENDED",
        "SEAT_GRANTED",
        "HOUSE_EXTINCT",
        "CLAIM_CREATED",
        "CLAIM_PRESSED",
        "CLAIM_RENOUNCED",
    }
)
CLAIM_CAUSES = frozenset({"SUCCEED", "DEPOSED"})  # M7: a passed-over heir, or a deposition
DEPOSITION_FIRST_DAY = 2  # M7: a deposition timer fires on days 2 to the last of its month
FIXTURE_WORLDS = ("ashcombe", "solitary", "untold")
# Groups check 8 renders beyond the manifest's (REV-SPR-029 R39): Edric Ashcombe's line keeps the
# line grouping covered now that §9's `line:<least named id>` is Orm's. The self-test lists them;
# the annals manifest does not.
EXTRA_GROUPS: dict[str, tuple[str, ...]] = {"ashcombe": ("line:6740622430173101898",)}
PROLOGUE_HEADING = "## Before the chronicle began"
MANIFEST_HASH = re.compile(r"sha256:[0-9a-f]{64}")
# Modules the renderer may import (V-NAR-07): the standard library and the sibling references.
ALLOWED_IMPORTS = frozenset(
    {
        "argparse",
        "contextlib",
        "dataclasses",
        "importlib",
        "io",
        "os",
        "re",
        "sys",
        "tempfile",
        "tomllib",
        "collections",
        "decimal",
        "pathlib",
        "types",
        "typing",
        "string",
        "itertools",
        "struct",
        "hashlib",
    }
)
SIBLINGS = frozenset({"canonical_v0", "lint_templates", "gen_hash_golden"})
FORBIDDEN_MODULES = ("socket", "ssl", "http.client", "urllib.request", "subprocess")
CONTROLS = (
    "no-event-id",
    "row-drops-own",
    "body-as-prologue",
    "missing-kf-source",
    "unrelated-event",
    "rows-swapped",
    "prologue-altered",
    "foreign-root",
    "forged-source",
    "text-truncated",
    "undeclared-slot",
    "state-read",
    "network-import",
    "shared-name",
    "fixture-deposed-day-one",
    "fixture-claim-untriggered",
)
PLANTED_LINE = "Thus the chronicle ended, and the land was quiet."
FORGED_ROW = 424242  # a life row no fixture holds, for the forged-source control (R37)
HEX_PREFIX_MIN = 4  # the shortest id prefix a bare label may show (REV-SPR-029 R17)
HASH_SEEDS = ("1", "12345")  # the other processes of check 6 (REV-SPR-029 R29)
IMPORT_LINE = re.compile(r"^(?:import|from)\s+([A-Za-z_][\w.]*)")
MIN_ROWS_FOR_MUTANTS = 2
MIN_CUT_SEGMENTS = 2  # check 11: solitary's cut must fold, not take the one-segment rule


class AnnalsError(ValueError):
    """An input the renderer refuses by name (annals_v0.md §1)."""


def _load(path: Path) -> ModuleType:
    """Import a module by path (``python -I`` adds no directory to ``sys.path``)."""
    spec = importlib.util.spec_from_file_location(path.stem, path)
    if spec is None or spec.loader is None:
        raise ImportError(str(path))
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module
    spec.loader.exec_module(module)
    return module


_canon = _load(CANONICAL)
_lint = _load(LINT)
_xxh = _load(HASH_REF)  # check 11 derives the fold's expected values from the XXH3 reference alone


# --- the calendar and the words -----------------------------------------------------------------


@dataclass(frozen=True)
class Calendar:
    """The world calendar (05.1 §4.1): 360 or 365 days, 12 months, no leap years."""

    days_per_year: int
    epoch_label: str

    def __post_init__(self) -> None:
        """Refuse a calendar the engine does not have (REV-SPR-029 F18)."""
        if self.days_per_year not in MONTHS:
            raise AnnalsError(f"days_per_year {self.days_per_year} is not 360 or 365 (05.1 §4.1)")

    def date(self, tick: int) -> tuple[int, int, int]:
        """``(year, month, day)`` of a tick: year 0 starts at the epoch; months and days from 1."""
        day = tick // MINUTES_PER_DAY
        year, day_of_year = divmod(day, self.days_per_year)
        month = 1
        for length in MONTHS[self.days_per_year]:
            if day_of_year < length:
                break
            day_of_year -= length
            month += 1
        return year, month, day_of_year + 1

    def month_index(self, tick: int) -> int:
        """Months since the epoch's first month (negative before it), for gestation arithmetic."""
        year, month, _ = self.date(tick)
        return year * len(MONTHS[self.days_per_year]) + month - 1


def _below_thousand(n: int) -> str:
    hundreds, rest = divmod(n, 100)
    tens, units = divmod(rest, 10)
    small = UNITS[rest] if rest < len(UNITS) else TENS[tens] + (f"-{UNITS[units]}" if units else "")
    if hundreds == 0:
        return small
    return f"{UNITS[hundreds]} hundred" + (f" and {small}" if small else "")


def cardinal(n: int) -> str:
    """``zero`` to ``nine hundred and ninety-nine thousand nine hundred and ninety-nine``."""
    if not 0 <= n < WORDS_LIMIT:
        raise AnnalsError(f"no words for {n}: the words run to {WORDS_LIMIT - 1}")
    thousands, rest = divmod(n, 1000)
    if thousands == 0:
        return _below_thousand(rest) or "zero"
    words = f"{_below_thousand(thousands)} thousand"
    if rest == 0:
        return words
    return words + (" and " if rest < 100 else " ") + _below_thousand(rest)  # noqa: PLR2004


def ordinal(n: int) -> str:
    """``first``, ``twenty-first``, ``one thousand and first``, ``two thousandth`` (n >= 1)."""
    words = cardinal(n)
    sep = "-" if "-" in words.split()[-1] else " "
    head, _, last = words.rpartition(sep)
    if last in ORDINAL_UNITS:
        tail = ORDINAL_UNITS[last]
    elif last.endswith("y"):
        tail = last[:-1] + "ieth"
    else:
        tail = last + "th"
    return head + (sep if head else "") + tail


def year_phrase(year: int, cal: Calendar) -> str:
    """``the twelfth year (Year of Ash)`` or ``the third year before the epoch (Year of Ash)``."""
    if year >= 0:
        return f"the {ordinal(year + 1)} year ({cal.epoch_label})"
    return f"the {ordinal(-year)} year before the epoch ({cal.epoch_label})"


def day_phrase(tick: int, cal: Calendar) -> str:
    """``day 14 of month 3``: the date inside its year (the year is the heading)."""
    _, month, day = cal.date(tick)
    return f"day {day} of month {month}"


def full_date(tick: int, cal: Calendar) -> str:
    """``day 14 of month 3 of the twelfth year (Year of Ash)``."""
    return f"{day_phrase(tick, cal)} of {year_phrase(cal.date(tick)[0], cal)}"


def q8_8(value: int) -> str:
    """A q8_8 raw value as an exact decimal with at least one fraction digit (``63.0``)."""
    text = format(Decimal(value) / ONE, f".{Q8_8_DIGITS}f").rstrip("0")
    return text + "0" if text.endswith(".") else text


# --- the inputs ----------------------------------------------------------------------------------


@dataclass(frozen=True)
class Event:
    """An admitted ``EVENT`` record, the fields a template may bind to."""

    id: int
    tick: int
    kind: str
    payload: tuple[int, ...]
    roles: tuple[tuple[int, int, int], ...]  # (entity_kind, entity_id, role), ordinal order
    snapshots: tuple[tuple[int, int, int, int | None, int | None, int], ...]
    causes: tuple[
        tuple[int, int, int | None, int, int, int], ...
    ]  # (cause, kind, motive, w, ev, told)

    def person(self, role: int) -> int | None:
        """The person filling ``role`` (at most one, except witness)."""
        for kind, entity, code in self.roles:
            if code == role and kind == PERSON:
                return entity
        return None

    def persons(self) -> tuple[int, ...]:
        """Every person in the cast, in ordinal order."""
        return tuple(e for k, e, _ in self.roles if k == PERSON)

    def cause(self, kind: int) -> tuple[int, int | None] | None:
        """The chosen told cause of edge kind ``kind``: ``(cause_id, motive)`` (annals_v0.md §3)."""
        edges = [(c, m, w) for c, code, m, w, _, told in self.causes if code == kind and told]
        if not edges:
            return None
        if kind == MOTIVATED_BY:  # the largest positive weight, ties to the lower cause id (R20,
            positive = [e for e in edges if e[2] > 0]  # R31): an edge of weight <= 0 never moved
            if not positive:  # the actor, so none is chosen and the plain variant renders
                return None
            c, m, _ = max(positive, key=lambda e: (e[2], -e[0]))
            return c, m
        return edges[0][0], edges[0][1]

    def untold(self, kind: int) -> int | None:
        """The earliest untold cause of edge kind ``kind``, when there is one (F15)."""
        ids = [c for c, code, _, _, _, told in self.causes if code == kind and not told]
        return min(ids) if ids else None

    def triggered_by(self, ids: set[int]) -> bool:
        """Whether a told ``triggered`` edge leads to one of ``ids``."""
        return any(
            code == TRIGGERED and told and c in ids for c, code, _, _, _, told in self.causes
        )

    def snapshot(self, attr: int, entity: int) -> int | None:
        """The anchored value of attribute ``attr`` of ``entity``, without a partner."""
        for a, k, e, pk, _, value in self.snapshots:
            if (a, k, e, pk) == (attr, PERSON, entity, None):
                return value
        return None


@dataclass(frozen=True)
class LifeRow:
    """A life-course row: a ``LIFE_ENTRY`` record, or a keyframe row for a house prologue."""

    person: int
    origin: int  # 0 born, 1 founder, 2 immigrant
    sex: int
    mother: int
    father: int
    birth_tick: int
    exit_tick: int | None
    source: str  # ``life/<pid>`` (channel A) or ``kf:life/<pid>`` (the keyframe extract)
    founder: int | None = None  # the founder index k of founder(seed, k), from the extract


@dataclass(frozen=True)
class Tenure:
    """A row of M7's tenure log in the keyframe extract."""

    title: int
    holder: int
    start: int
    end: int | None


@dataclass
class Chronicle:
    """One timeline's channel A, as the annals read it."""

    timeline: int = 0
    registry_version: str = ""
    digest: str = ""
    events: dict[int, Event] = field(default_factory=dict)
    life: dict[int, LifeRow] = field(default_factory=dict)
    exits: dict[int, int] = field(default_factory=dict)  # person -> exit tick
    records: list[Any] = field(default_factory=list)  # the canonical Records, in file order


@dataclass
class World:
    """The world file, the names file and the keyframe extract of a fixture directory."""

    id: str
    calendar: Calendar
    manifest_hash: str = "-"
    persons: dict[int, str] = field(default_factory=dict)
    houses: dict[int, str] = field(default_factory=dict)
    titles: dict[int, str] = field(default_factory=dict)
    keyframe: str = "-"
    state_hash: str = "-"
    house_ids: list[int] = field(default_factory=list)
    title_ids: list[int] = field(default_factory=list)
    kf_life: dict[int, LifeRow] = field(default_factory=dict)
    members: dict[int, int] = field(default_factory=dict)  # person -> house
    tenures: list[Tenure] = field(default_factory=list)

    @property
    def extract_digest(self) -> str:
        """XXH3-64 of the extract's canonical form, 16 hex digits; ``-`` without an extract."""
        return extract_digest(self) if self.keyframe != "-" else "-"


@dataclass(frozen=True)
class Dictionary:
    """The Registry's dictionaries the chronicle header names (annals_v0.md §1)."""

    version: str
    verbs: dict[str, dict[str, Any]]
    verb_names: list[str]
    act_classes: list[str]
    attrs: dict[int, tuple[str, str, list[str]]]  # attr_id -> (name, type, enum values)


def _event_of(rec: Any) -> Event:  # noqa: ANN401  (a canonical_v0.Record, loaded by path)
    get = rec.__getitem__
    return Event(
        int(get("event_id")),
        int(get("tick")),
        str(get("verb")),
        tuple(int(v) for v in get("payload")),
        tuple((int(a), int(b), int(c)) for a, b, c in get("roles")),
        tuple(
            (
                int(a),
                int(b),
                int(c),
                None if d is None else int(d),
                None if e is None else int(e),
                int(f),
            )
            for a, b, c, d, e, f in get("snapshots")
        ),
        tuple(
            (int(a), int(b), None if c is None else int(c), int(d), int(e), int(f))
            for a, b, c, d, e, f in get("causes")
        ),
    )


# --- a chronicle of several segments: the #segment blocks and the fold (annals_v0.md §9.1) ------

# Only this exact prefix is reserved (REV-AN-0 R8): a line that begins so is a segment line or is
# refused; other spellings (``#Segment``, ``# segment``) are free comments.
SEGMENT_PREFIX = "#segment"
SEQ = r"0|[1-9][0-9]{0,19}"  # a canonical int of 1 to 20 digits; u64 checked after (REV-AN-0 R9)
SEGMENT_LINE = re.compile(
    rf"#segment y(?P<year>[0-9]{{6}}) first_seq=(?P<first_seq>{SEQ}) "
    rf"last_seq=(?P<last_seq>{SEQ}) canonical_digest=(?P<digest>[0-9a-f]{{32}})"
)
YEARS_LIMIT = 1_000_000  # six digits: y000000 to y999999 (channel_a_v0.sql R5)
ONE_SEGMENT_LINE = "one segment: no line begins #segment (annals_v0.md §9.1 rule 2)"
Numbered = list[tuple[int, Any]]  # (1-based file line, canonical_v0.Record)


@dataclass(frozen=True)
class Segment:
    """One ``#segment`` block: its sim-year, its seq range and its recomputed digest (§9.1)."""

    year: int
    first_seq: int
    last_seq: int
    digest: bytes  # canonical_digest of the block's record lines (canonical_v0.md §4), 16 bytes
    line: int  # the file line of its #segment line


@dataclass(frozen=True)
class Fold:
    """A checked chronicle file: its segments, its ``chronicle_digest`` and its records (§9.1)."""

    segments: tuple[Segment, ...]  # empty: no line begins #segment (one segment)
    chronicle_digest: bytes
    # (file line, canonical_v0.Record) of every record, in file order, from the file split on LF
    # alone: what a render reads, so a comment never hides a record (REV-AN-0 R1)
    records: tuple[tuple[int, Any], ...]


@dataclass
class _Block:
    """A segment line as written, and the records that follow it up to the next one."""

    line: int
    year: int
    first_seq: int
    last_seq: int
    digest: str
    records: Numbered = field(default_factory=list)


def fold_digests(digests: list[bytes]) -> bytes:
    """§9: one segment's ``canonical_digest`` is the chronicle's; several fold by XXH3-128."""
    return digests[0] if len(digests) == 1 else bytes(_canon.xxh3_128(b"".join(digests)))


def _canonical_records(data: bytes, where: str) -> tuple[list[str], Numbered]:
    """The file checked whole as a canonical file, exactly as ``canonical_v0.py --digest`` does.

    Rule 1 of §9.1, then the rule a chronicle adds: it holds at least one record (REV-AN-0 R5).
    """
    lines = _canon._file_lines(data, where)
    header = _canon.HEADER_LINE.fullmatch(lines[0])
    if header is None or not _canon._header_values_ok(header):
        raise _canon.CanonicalError(f"{where}:1: not a canonical header line: {lines[0]!r}")
    records: Numbered = []
    for n, raw in enumerate(lines[1:], 2):
        if raw.startswith("#"):
            continue
        try:
            records.append((n, _canon.parse(raw)))
        except _canon.CanonicalError as e:
            raise _canon.CanonicalError(f"{where}:{n}: {e}") from None
    _canon._check_records(records, where, int(header["timeline"]), header["channel"])
    if not records:
        raise AnnalsError(f"{where}: a chronicle holds at least one record (annals_v0.md §9.1)")
    return lines, records


def _segment_line(raw: str, n: int, where: str, seen: dict[int, int]) -> _Block:
    """Parse one segment line; its year must be new and above every earlier one (§9.1 rule 3)."""
    m = SEGMENT_LINE.fullmatch(raw)
    if m is None:
        raise AnnalsError(f"{where}:{n}: not a segment line (annals_v0.md §9.1): {raw!r}")
    for key in ("first_seq", "last_seq"):
        if int(m[key]) > _canon.MASK64:
            raise AnnalsError(
                f"{where}:{n}: not a segment line (annals_v0.md §9.1): {key}={m[key]} is past u64"
            )
    year = int(m["year"])
    if year in seen:
        raise AnnalsError(f"{where}:{n}: segment y{year:06d} repeats (line {seen[year]})")
    if seen and year < max(seen):
        raise AnnalsError(
            f"{where}:{n}: segment y{year:06d} follows y{max(seen):06d}: the years must increase"
        )
    seen[year] = n
    return _Block(n, year, int(m["first_seq"]), int(m["last_seq"]), m["digest"])


def _blocks(lines: list[str], records: Numbered, where: str) -> list[_Block]:
    """The segment blocks, line by line in file order (§9.1 rule 3)."""
    by_line = dict(records)
    seen: dict[int, int] = {}
    blocks: list[_Block] = []
    for n, raw in enumerate(lines[1:], 2):
        if raw.startswith(SEGMENT_PREFIX):
            blocks.append(_segment_line(raw, n, where, seen))
        elif n in by_line:
            if not blocks:
                raise AnnalsError(
                    f"{where}:{n}: a record before the first #segment line: every record of a "
                    "chronicle with segment lines belongs to a segment"
                )
            blocks[-1].records.append((n, by_line[n]))
    return blocks


def _check_block(b: _Block, lines: list[str], where: str) -> Segment:
    """A block against its segment line: a record, the seq range, the digest (§9.1 rule 4)."""
    name = f"segment y{b.year:06d}"
    if not b.records:
        raise AnnalsError(
            f"{where}:{b.line}: {name} holds no record (a channel-A segment holds at least one: "
            "channel_a_v0.sql segment.record_count > 0)"
        )
    first, last = int(b.records[0][1]["seq"]), int(b.records[-1][1]["seq"])
    for key, said, has in (("first", b.first_seq, first), ("last", b.last_seq, last)):
        if said != has:
            raise AnnalsError(
                f"{where}:{b.line}: {name}: {key}_seq={said} on its #segment line, but its {key} "
                f"record has seq {has}"
            )
    digest = bytes(_canon.digest(lines[n - 1] for n, _ in b.records))
    if digest.hex() != b.digest:
        raise AnnalsError(
            f"{where}:{b.line}: {name} (records on lines {b.records[0][0]}-{b.records[-1][0]}): "
            f"its #segment line says canonical_digest={b.digest}; its records recompute to "
            f"{digest.hex()}"
        )
    return Segment(b.year, first, last, digest, b.line)


def fold(data: bytes, where: str) -> Fold:
    """Check a chronicle file and compute its ``chronicle_digest`` (annals_v0.md §9.1).

    The file is checked whole as a canonical file first (its own refusals, two swapped blocks
    among them) and must hold a record; when no line begins ``#segment`` it is one segment and its
    digest is its ``canonical_digest``; otherwise every block is checked against its segment line
    and the digests are folded. The fold reads no calendar (REV-AN-0 R7: ``load_chronicle`` checks
    each record's sim-year against its block's).
    """
    lines, records = _canonical_records(data, where)
    if not any(raw.startswith(SEGMENT_PREFIX) for raw in lines[1:]):
        plain = bytes(_canon.digest(lines[n - 1] for n, _ in records))
        return Fold((), plain, tuple(records))
    segments = tuple(_check_block(b, lines, where) for b in _blocks(lines, records, where))
    return Fold(segments, fold_digests([s.digest for s in segments]), tuple(records))


def check_segment_years(f: Fold, calendar: Calendar, where: str) -> None:
    """Every record of a block is of the block's sim-year on the world's calendar (REV-AN-0 R7).

    The exporter cuts by sim-year (``tick div minutes per year``, channel_a_v0.sql R5), so a block
    holding a record of another year was not cut so; ``fold`` cannot tell without the calendar.
    """
    if not f.segments:
        return
    per_year = calendar.days_per_year * MINUTES_PER_DAY
    base = int(f.records[0][1]["seq"])  # seq is contiguous (§9.1 rule 1), so seq - base indexes
    for s in f.segments:
        for n, rec in f.records[s.first_seq - base : s.last_seq - base + 1]:
            tick = int(rec["tick"])
            if tick // per_year != s.year:
                raise AnnalsError(
                    f"{where}:{n}: segment y{s.year:06d} (line {s.line}) holds a record of "
                    f"sim-year {tick // per_year} (tick {tick} on the {calendar.days_per_year}-day "
                    "calendar): a block holds the records of its own sim-year (annals_v0.md §9.1)"
                )


def segment_chronicle(data: bytes, days_per_year: int, where: str) -> str:
    """A one-segment chronicle cut into the form of §9.1: one block per sim-year with a record.

    What ``rh-cli annals-inputs`` writes for a run (SPR-029 T-01), here for a fixture: a record's
    sim-year is ``tick div minutes per year`` (channel_a_v0.sql R5). Comment lines are not carried
    (no digest reads them); a file that already holds a segment line is refused.
    """
    lines, records = _canonical_records(data, where)
    if any(raw.startswith(SEGMENT_PREFIX) for raw in lines[1:]):
        raise AnnalsError(f"{where}: the file already holds #segment lines")
    Calendar(days_per_year, "")  # refuses a calendar the engine does not have, by name
    per_year = days_per_year * MINUTES_PER_DAY
    years: dict[int, Numbered] = {}
    for n, rec in records:
        year = int(rec["tick"]) // per_year
        if not 0 <= year < YEARS_LIMIT:
            raise AnnalsError(f"{where}:{n}: tick {rec['tick']} is outside y000000 to y999999")
        years.setdefault(year, []).append((n, rec))
    out = [lines[0]]
    for year, block in sorted(years.items()):
        body = [lines[n - 1] for n, _ in block]
        out.append(
            f"#segment y{year:06d} first_seq={block[0][1]['seq']} last_seq={block[-1][1]['seq']} "
            f"canonical_digest={bytes(_canon.digest(body)).hex()}"
        )
        out += body
    return "\n".join(out) + "\n"


def load_chronicle(
    path: Path, dictionary: Dictionary, calendar: Calendar | None = None
) -> Chronicle:
    """Parse a canonical channel-A file of one segment or several (§9.1), checked whole first.

    The records are the ones ``fold`` parsed from the file split on LF alone (REV-AN-0 R1), and
    each block's records are checked against its sim-year on the world's calendar (R7).
    """
    data = path.read_bytes()
    checked = fold(data, str(path))
    if calendar is not None:
        check_segment_years(checked, calendar, str(path))
    chron = Chronicle(digest=checked.chronicle_digest.hex())
    header = _canon.HEADER_LINE.fullmatch(data.split(b"\n", 1)[0].decode("ascii"))
    chron.timeline = int(header["timeline"])
    chron.registry_version = str(_canon.decode_str(header["registry"]))
    if chron.registry_version != dictionary.version:
        raise AnnalsError(
            f"{path}: the chronicle's registry_version {chron.registry_version!r} is not the "
            f"dictionary this renderer read, {dictionary.version!r}"
        )
    for _, rec in checked.records:
        chron.records.append(rec)
        if rec.kind == "EVENT":
            ev = _event_of(rec)
            chron.events[ev.id] = ev
        elif rec.kind == "LIFE_ENTRY":
            pid = int(rec["person_id"])
            chron.life[pid] = LifeRow(
                pid,
                int(rec["origin"]),
                int(rec["sex"]),
                int(rec["mother_id"]),
                int(rec["father_id"]),
                int(rec["birth_tick"]),
                None,
                f"life/{pid}",
            )
        elif rec.kind == "LIFE_EXIT":
            chron.exits[int(rec["person_id"])] = int(rec["tick"])
    return chron


def _int_map(table: object) -> dict[int, str]:
    return {int(k): str(v) for k, v in table.items()} if isinstance(table, dict) else {}


def _keyframe(w: World, kf: dict[str, Any], where: str) -> None:
    extract = kf.get("extract", {})
    if extract.get("format") != EXTRACT_FORMAT:
        raise AnnalsError(f"{where}: [extract] format must be {EXTRACT_FORMAT!r}")
    w.keyframe = str(extract.get("keyframe", "-"))
    w.state_hash = str(extract.get("state_hash", "-"))
    w.house_ids = sorted(int(h["id"]) for h in kf.get("house", []))
    w.title_ids = sorted(int(t["id"]) for t in kf.get("title", []))
    for row in kf.get("life_course", []):
        pid = int(row["person_id"])
        exit_tick = row.get("exit_tick")
        founder = row.get("founder")
        w.kf_life[pid] = LifeRow(
            pid,
            FOUNDER,
            int(row["sex"]),
            int(row["mother_id"]),
            int(row["father_id"]),
            int(row["birth_tick"]),
            None if exit_tick is None else int(exit_tick),
            f"kf:life/{pid}",
            None if founder is None else int(founder),
        )
    for row in kf.get("house_member", []):
        w.members[int(row["person"])] = int(row["house"])
    for row in kf.get("tenure", []):
        end = row.get("end")
        w.tenures.append(
            Tenure(
                int(row["title"]),
                int(row["holder"]),
                int(row["start"]),
                None if end is None else int(end),
            )
        )
    w.tenures.sort(key=lambda t: (t.title, t.start))


def _opt(v: int | None) -> str:
    """``~`` for an absent value, else its decimal form."""
    return "~" if v is None else str(v)


def extract_digest(w: World) -> str:
    """XXH3-64 of the extract's canonical form: fixed table order, rows sorted by key (R23)."""
    lines = [f"extract {w.keyframe} {w.state_hash}"]
    lines += [f"house {h}" for h in w.house_ids]
    lines += [f"title {t}" for t in w.title_ids]
    for r in sorted(w.kf_life.values(), key=lambda r: r.person):
        lines.append(
            f"life {r.person} {r.sex} {r.mother} {r.father} {r.birth_tick}"
            f" {_opt(r.exit_tick)} {_opt(r.founder)}"
        )
    lines += [f"member {p} {h}" for p, h in sorted(w.members.items())]
    lines += [
        f"tenure {t.title} {t.holder} {t.start} {_opt(t.end)}"
        for t in sorted(w.tenures, key=lambda t: (t.title, t.start))
    ]
    data = D_EXTRACT + b"".join(line.encode("ascii") + b"\n" for line in lines)
    return f"{int(_canon.xxh3_64(data)):016x}"


def load_world(directory: Path) -> World:
    """The fixture directory: ``world.toml``, ``names.toml`` and, if present, ``keyframe.toml``."""
    meta = tomllib.loads((directory / "world.toml").read_text(encoding="utf-8"))["world"]
    w = World(str(meta["id"]), Calendar(int(meta["days_per_year"]), str(meta["epoch_label"])))
    names_path = directory / "names.toml"
    if names_path.exists():
        names = tomllib.loads(names_path.read_text(encoding="utf-8"))
        w.manifest_hash = str(names.get("world", {}).get("manifest_hash", "-"))
        if not MANIFEST_HASH.fullmatch(w.manifest_hash):
            raise AnnalsError(f"{names_path}: manifest_hash must be sha256:<64 hex digits>")
        w.persons = _int_map(names.get("persons"))
        w.houses = _int_map(names.get("houses"))
        w.titles = _int_map(names.get("titles"))
    kf_path = directory / "keyframe.toml"
    if kf_path.exists():
        _keyframe(w, tomllib.loads(kf_path.read_text(encoding="utf-8")), str(kf_path))
    return w


def load_dictionary(registry: Path = REGISTRY, delta: Path | None = REGISTRY_DELTA) -> Dictionary:
    """The Registry's verbs, act classes and attributes; a next delta adds its verbs, ``+next``."""
    reg = tomllib.loads(registry.read_text(encoding="utf-8"))
    verbs: dict[str, dict[str, Any]] = dict(reg["verbs"])
    version = str(reg["registry_version"])
    if delta is not None and delta.exists():
        verbs.update(tomllib.loads(delta.read_text(encoding="utf-8")).get("verbs", {}))
        version += "+next"
    attrs: dict[int, tuple[str, str, list[str]]] = {}
    for i, name in enumerate(sorted(reg["attributes"])):
        entry = reg["attributes"][name]
        attrs[i] = (name, str(entry.get("type", "")), [str(v) for v in entry.get("values", [])])
    return Dictionary(version, verbs, sorted(verbs), list(reg["act_classes"]["ids"]), attrs)


# --- designation and houses (annals_v0.md §4; one function for both: R4, R6, R18) -----------------


@dataclass
class Provenance:
    """What a designation or a sentence read: persons and source records."""

    persons: set[int] = field(default_factory=set)
    sources: set[str] = field(default_factory=set)


class Kinship:
    """The life-course rows (channel A first, then the keyframe) and the names, read-only."""

    def __init__(self, world: World, chron: Chronicle, *, disambiguate: bool = True) -> None:
        """Bind the rows a designation may read; the label pool is the run's persons (R28)."""
        self.w = world
        self.c = chron
        self.rows: dict[int, LifeRow] = {**world.kf_life, **chron.life}
        self.pool: set[int] = (
            set(world.kf_life) | set(world.members) | set(chron.life) | set(chron.exits)
        )
        for ev in chron.events.values():
            self.pool.update(ev.persons())
        for row in chron.life.values():  # a LIFE_ENTRY's parents (R38): a birth's father is in
            self.pool.update(p for p in (row.mother, row.father) if p)  # the pool
        named = Counter(world.persons[p] for p in self.pool if p in world.persons)
        self.shared = {n for n, k in named.items() if k > 1} if disambiguate else set()

    def row(self, pid: int) -> LifeRow | None:
        """The person's life-course row: channel A's ``LIFE_ENTRY`` first, else the keyframe's."""
        return self.c.life.get(pid) or self.w.kf_life.get(pid)

    def house_of(self, pid: int, prov: Provenance) -> int | None:
        """§4 rule 4, M7 0.5.0's agnatic rule: the keyframe row; else the father's house; else none.

        Only a birth during the run walks to its father, by this same rule, through any number of
        in-run rows; any other origin without a row has none, and the mother's house never passes.
        """
        if pid in self.w.members:
            prov.sources.add(f"kf:member/{pid}")
            return self.w.members[pid]
        row = self.c.life.get(pid)
        if row is None or row.origin != BORN:
            return None
        prov.sources.add(row.source)
        return self.house_of(row.father, prov) if row.father else None

    def label_of(self, pid: int, prov: Provenance) -> str:
        """``Founder #k`` (1-based) from the extract, else ``p-`` and a unique hex prefix."""
        row = self.w.kf_life.get(pid)
        if row is not None and row.founder is not None:
            prov.sources.add(row.source)
            return f"Founder #{row.founder + 1}"
        own = f"{pid:016x}"
        others = [f"{p:016x}" for p in self.pool if p != pid]
        n = HEX_PREFIX_MIN
        while any(o.startswith(own[:n]) for o in others) and n < len(own):
            n += 1
        return f"p-{own[:n]}"

    def rank(self, pid: int, parent: int, sex: int, prov: Provenance) -> int | None:
        """The person's birth order among the parent's children of the same sex, by birth tick."""
        siblings = sorted(
            (r for r in self.rows.values() if parent in (r.father, r.mother) and r.sex == sex),
            key=lambda r: (r.birth_tick, r.person),
        )
        me = next(r for r in siblings if r.person == pid)
        if sum(r.birth_tick == me.birth_tick for r in siblings) > 1:
            return None  # twins: the rows cannot order them
        prov.sources.update(r.source for r in siblings)
        return siblings.index(me) + 1

    def designate(self, pid: int, prov: Provenance) -> str:
        """A name; else the Nth son or daughter of a named father, then mother; else the label."""
        prov.persons.add(pid)
        name = self.w.persons.get(pid)
        if name is not None:
            prov.sources.add(f"name/{pid}")
            return f"{name} ({self.label_of(pid, prov)})" if name in self.shared else name
        row = self.row(pid)
        if row is not None:
            for parent in (row.father, row.mother):
                if parent not in self.w.persons:
                    continue
                rank = self.rank(pid, parent, row.sex, prov)
                if rank is not None:
                    prov.sources.update({row.source, f"name/{parent}"})
                    prov.persons.add(parent)
                    child = "daughter" if row.sex else "son"
                    phrase = f"the {ordinal(rank)} {child} of {self.w.persons[parent]}"
                    if self.w.persons[parent] in self.shared:  # the parent stays bare; the
                        phrase += f" ({self.label_of(pid, prov)})"  # person's own label (R33)
                    return phrase
        return self.label_of(pid, prov)


# --- rendering -----------------------------------------------------------------------------------


@dataclass
class Sentence:
    """One rendered line with its provenance (one sidecar row)."""

    text: str
    template: str
    version: int
    events: set[int] = field(default_factory=set)
    prov: Provenance = field(default_factory=Provenance)
    prologue: bool = False

    def row(self, line: int) -> str:
        """The sidecar row: ``S|P line template version [events] [persons] [sources]``."""
        tag = "P" if self.prologue else "S"
        ev = ",".join(str(e) for e in sorted(self.events))
        ps = ",".join(str(p) for p in sorted(self.prov.persons))
        src = ",".join(sorted(self.prov.sources))
        return f"{tag} {line} {self.template} {self.version} [{ev}] [{ps}] [{src}]"


class Renderer:
    """Renders one group of one chronicle: a pure function of its inputs (annals_v0.md §9)."""

    def __init__(
        self,
        templates: Any,  # noqa: ANN401  (a lint_templates.Templates, loaded by path)
        world: World,
        chron: Chronicle,
        dictionary: Dictionary,
        *,
        disambiguate: bool = True,
    ) -> None:
        """Bind the templates, the world (names, calendar, extract), chronicle and dictionary."""
        self.t = templates
        self.w = world
        self.c = chron
        self.d = dictionary
        self.kin = Kinship(world, chron, disambiguate=disambiguate)
        self.attr_ids = {name: i for i, (name, _, _) in dictionary.attrs.items()}
        self.used: Counter[str] = Counter()

    # -- slots (annals_v0.md §3)

    def _father_row(self, ev: Event) -> LifeRow | None:
        child = ev.person(TARGET)
        row = None if child is None else self.c.life.get(child)
        return row if row is not None and row.father else None

    def _role_slot(self, role: str, ev: Event, s: Sentence) -> str:
        if role == "witnesses":
            cast = [p for k, p, c in ev.roles if c == WITNESS and k == PERSON]
            return ", ".join(self.kin.designate(p, s.prov) for p in cast) or "no one"
        if role == "father":
            row = self._father_row(ev)
            if row is None:
                return "no one"
            s.prov.sources.add(row.source)
            return self.kin.designate(row.father, s.prov)
        pid = ev.person(ROLE_CODES[role])
        return self.kin.designate(pid, s.prov) if pid is not None else "no one"

    def _enum_word(self, enum: str, code: int, ev: Event) -> str:
        words = self.t.enums[enum]["words"]
        if not 0 <= code < len(words):
            raise AnnalsError(f"event {ev.id}: code {code} is outside the enum {enum}")
        return str(words[code])

    def _payload_slot(self, name: str, ev: Event) -> str:
        for i, slot in enumerate(self.t.payloads[ev.kind]["slots"]):
            if slot["name"] != name:
                continue
            value = ev.payload[i]
            if "enum" in slot:
                return self._enum_word(str(slot["enum"]), value, ev)
            return ordinal(value) if slot.get("ordinal") else str(value)
        raise KeyError(name)

    def _snapshot_slot(self, attr: str, role: str, ev: Event) -> str:
        pid = ev.person(ROLE_CODES[role])
        value = None if pid is None else ev.snapshot(self.attr_ids[attr], pid)
        if value is None:
            return "not recorded"
        _, kind, values = self.d.attrs[self.attr_ids[attr]]
        if kind == "q8_8":
            return q8_8(value)
        if kind == "enum":
            if not 0 <= value < len(values):
                raise AnnalsError(
                    f"event {ev.id}: snapshot code {value} is outside the attribute {attr}"
                )
            return values[value]
        return str(value)

    def label(self, ev: Event, s: Sentence) -> str:
        """The noun phrase naming ``ev`` as a cause: its variant's label, or its ``[[label]]``."""
        variant = self.variant(ev)
        if variant is not None:
            return self.fill(str(variant["label"]), ev, s)
        entry = self.t.labels.get(ev.kind)
        if entry is None:
            raise AnnalsError(f"event {ev.id}: no template or [[label]] names a {ev.kind} (R27)")
        return self.fill(str(entry["label"]), ev, s)

    def _cause_slot(self, kind: str, ev: Event, s: Sentence) -> str:
        found = ev.cause(EDGE_CODES[kind])
        cause = None if found is None else self.c.events.get(found[0])
        if cause is None:
            untold = found[0] if found is not None else ev.untold(EDGE_CODES[kind])
            if untold is not None:
                s.prov.sources.add(f"untold/{untold}")
            return "an event the chronicle does not tell"
        s.events.add(cause.id)
        return self.label(cause, s)

    def _motive_slot(self, ev: Event) -> str:
        found = ev.cause(MOTIVATED_BY)
        motive = None if found is None else found[1]
        if motive is None:
            return "for a motive the chronicle does not name"
        return self._enum_word("motive", motive, ev)

    def _tick_slot(self, which: str, ev: Event) -> str:
        if which == "date":
            return day_phrase(ev.tick, self.w.calendar)
        return year_phrase(self.w.calendar.date(ev.tick)[0], self.w.calendar)

    def slot(self, name: str, ev: Event, s: Sentence) -> str:
        """The text of one slot of a chronicle sentence (the lint has resolved it already)."""
        slot = _lint.parse_slot(name)
        if slot is None:
            raise AnnalsError(f"slot {name!r} {_lint.OUTSIDE}")
        match slot.family:
            case "tick":
                return self._tick_slot(slot.args[0], ev)
            case "role":
                return self._role_slot(slot.args[0], ev, s)
            case "payload":
                return self._payload_slot(slot.args[0], ev)
            case "snapshot":
                return self._snapshot_slot(slot.args[0], slot.args[1], ev)
            case "motive":
                return self._motive_slot(ev)
            case _:
                return self._cause_slot(slot.args[0], ev, s)

    def fill(
        self, text: str, ev: Event | None, s: Sentence, values: dict[str, str] | None = None
    ) -> str:
        """Substitute every slot of ``text``: from the event, or from ``values`` for a prologue."""
        out = []
        for literal, name, spec, conversion in _lint.string.Formatter().parse(text):
            out.append(literal)
            if name is None:
                continue
            if spec or conversion is not None:
                raise AnnalsError(f"slot {name!r} carries a conversion or format spec")
            if values is not None:
                out.append(values[name])
            elif ev is not None:
                out.append(self.slot(name, ev, s))
        return "".join(out)

    # -- variants (annals_v0.md §2)

    def holds(self, when: dict[str, Any], ev: Event) -> bool:
        """Whether a variant's ``when`` holds: roles, a payload enum, an edge, a named motive."""
        has = all(self._filled(r, ev) for r in when.get("has", []))
        lacks = all(not self._filled(r, ev) for r in when.get("lacks", []))
        edge = "edge" not in when or ev.cause(EDGE_CODES[str(when["edge"])]) is not None
        return (
            has
            and lacks
            and edge
            and self._payload_holds(when, ev)
            and self._motive_holds(when, ev)
        )

    def _filled(self, role: str, ev: Event) -> bool:
        if role == "father":
            return self._father_row(ev) is not None
        return ev.person(ROLE_CODES[role]) is not None

    def _motive_holds(self, when: dict[str, Any], ev: Event) -> bool:
        if "motive" not in when:
            return True
        found = ev.cause(MOTIVATED_BY)
        named = found is not None and found[1] is not None
        return bool(named == (when["motive"] == "named"))

    def _payload_holds(self, when: dict[str, Any], ev: Event) -> bool:
        payload = when.get("payload", {})
        if not isinstance(payload, dict) or not payload:
            return True
        layout = {str(s["name"]): (i, s) for i, s in enumerate(self.t.payloads[ev.kind]["slots"])}
        for name, want in payload.items():
            i, slot = layout[name]
            ids = self.t.enums[slot["enum"]]["ids"]
            if not 0 <= ev.payload[i] < len(ids):
                raise AnnalsError(
                    f"event {ev.id}: code {ev.payload[i]} is outside the enum {slot['enum']}"
                )
            if ids[ev.payload[i]] != want:
                return False
        return True

    def variant(self, ev: Event) -> dict[str, Any] | None:
        """The first template of the event's kind whose ``when`` holds (``None`` out of scope)."""
        entries: list[dict[str, Any]] = self.t.by_kind.get(ev.kind, [])
        for entry in entries:
            if "when" not in entry or self.holds(entry["when"], ev):
                return entry
        return None

    def sentence(self, ev: Event) -> Sentence:
        """Render one chronicle event through its variant, capitalised, with its sidecar row."""
        entry = self.variant(ev)
        assert entry is not None
        s = Sentence("", str(entry["id"]), int(entry["version"]), {ev.id})
        text = self.fill(str(entry["text"]), ev, s)
        s.text = text[:1].upper() + text[1:]
        self.used[s.template] += 1
        return s

    # -- groups (annals_v0.md §5) and prologues (§6)

    def group_events(self, members: set[int] | None) -> list[Event]:
        """The group's events in id order: a member's, or castless with a told edge in (R22)."""
        events = sorted(self.c.events.values(), key=lambda e: e.id)
        if members is None:
            return events
        chosen: set[int] = set()
        for ev in events:  # ascending ids: a joined castless event counts for later ones
            if set(ev.persons()) & members or (not ev.persons() and ev.triggered_by(chosen)):
                chosen.add(ev.id)
        return [ev for ev in events if ev.id in chosen]

    def house_members(self, house: int) -> set[int]:
        """The keyframe's members of ``house`` and every person whose house walk ends there."""
        members = {p for p, h in self.w.members.items() if h == house}
        members |= {p for p in self.c.life if self.kin.house_of(p, Provenance()) == house}
        return members

    def line_members(self, root: int) -> set[int]:
        """``root`` and every descendant through the life-course rows (channel A, then keyframe)."""
        members = {root}
        changed = True
        while changed:
            changed = False
            for row in self.kin.rows.values():
                if row.person not in members and (row.mother in members or row.father in members):
                    members.add(row.person)
                    changed = True
        return members

    def _prologue_sentence(self, pid: str, values: dict[str, str], s: Sentence) -> Sentence:
        entry = self.t.prologue[pid]
        s.template, s.version, s.prologue = pid, int(entry["version"]), True
        text = self.fill(str(entry["text"]), None, s, values)
        s.text = text[:1].upper() + text[1:]
        self.used[pid] += 1
        return s

    def _named(self, table: dict[int, str], key: int, what: str) -> str:
        if key not in table:
            raise AnnalsError(
                f"{what} {key} has no name in the names file (R27: no raw id in the text)"
            )
        return table[key]

    def prologue(self, house: int, members: set[int]) -> list[Sentence]:
        """The house's opening from the keyframe extract: living count, ancestors and tenures."""
        cal = self.w.calendar
        living = sorted(
            p for p in members if p in self.w.kf_life and self.w.kf_life[p].exit_tick is None
        )
        prov = Provenance(set(living), {f"house/{house}", *(f"kf:life/{p}" for p in living)})
        count = f"{cardinal(len(living))} living member" + ("" if len(living) == 1 else "s")
        values = {"house": self._named(self.w.houses, house, "house"), "living": count}
        out = [self._prologue_sentence("prologue.house", values, Sentence("", "", 0, prov=prov))]
        for pid in sorted(members):
            row = self.w.kf_life.get(pid)
            if row is None or row.exit_tick is None:
                continue
            s = Sentence("", "", 0, prov=Provenance(sources={row.source}))
            values = {
                "person": self.kin.designate(pid, s.prov),
                "birth_date": full_date(row.birth_tick, cal),
                "death_date": full_date(row.exit_tick, cal),
            }
            out.append(self._prologue_sentence("prologue.ancestor", values, s))
        for ten in (t for t in self.w.tenures if t.holder in members):
            s = Sentence(
                "",
                "",
                0,
                prov=Provenance(
                    sources={f"kf:tenure/{ten.title}/{ten.start}", f"title/{ten.title}"}
                ),
            )
            values = {
                "holder": self.kin.designate(ten.holder, s.prov),
                "title": self._named(self.w.titles, ten.title, "title"),
                "start_date": full_date(ten.start, cal),
            }
            if ten.end is None:
                out.append(self._prologue_sentence("prologue.tenure.open", values, s))
            else:
                values["end_date"] = full_date(ten.end, cal)
                out.append(self._prologue_sentence("prologue.tenure", values, s))
        return out


@dataclass
class Annal:
    """A rendered annal: its text lines and its sidecar rows, before the digest."""

    title: str
    group: str
    lines: list[str] = field(default_factory=list)
    rows: list[str] = field(default_factory=list)
    omitted: Counter[str] = field(default_factory=Counter)
    castless: Counter[str] = field(default_factory=Counter)


def group_of(r: Renderer, group: str) -> tuple[str, set[int] | None, list[Sentence]]:
    """A group's title, members (``None`` for every year) and prologue sentences."""
    kind, _, arg = group.partition(":")
    if kind == "year":
        return "The annals of the years", None, []
    if kind == "line":
        root = int(arg)
        return (
            f"The annals of the line of {r.kin.designate(root, Provenance())}",
            r.line_members(root),
            [],
        )
    if kind != "house":
        raise AnnalsError(f"group {group!r} is not house:<id>, line:<id> or year")
    house = int(arg)
    members = r.house_members(house)
    return (
        f"The annals of {r._named(r.w.houses, house, 'house')}",
        members,
        r.prologue(house, members),
    )


def render(r: Renderer, group: str) -> Annal:
    """Render one group: a house (``house:<id>``), a line (``line:<pid>``) or all (``year``)."""
    title, members, prologue = group_of(r, group)
    annal = Annal(title, group)
    scope = set(r.t.scope)
    lines, rows = annal.lines, annal.rows
    lines.append(f"# {title}")
    lines.append(f"# From the chronicle of {r.w.id}; templates {r.t.version}")

    def emit(s: Sentence) -> None:
        lines.append(s.text)
        rows.append(s.row(len(lines)))

    if prologue:
        lines.extend(("", PROLOGUE_HEADING))
        for s in prologue:
            emit(s)
    chosen = r.group_events(members)
    if members is not None:
        for ev in r.c.events.values():
            if not ev.persons() and ev not in chosen and ev.kind in scope:
                annal.castless[ev.kind] += 1
    year: int | None = None
    for ev in chosen:
        if ev.kind not in scope:
            annal.omitted[ev.kind] += 1
            continue
        y = r.w.calendar.date(ev.tick)[0]
        if y != year:
            year = y
            phrase = year_phrase(y, r.w.calendar)
            lines.extend(("", f"## {phrase[:1].upper()}{phrase[1:]}"))
        emit(r.sentence(ev))
    return annal


def templates_digest(path: Path = TEMPLATES) -> int:
    """XXH3-64 of the template file's bytes: part of every annals digest (a 16-hex-digit token)."""
    return int(_canon.xxh3_64(path.read_bytes()))


def annals_digest(text: str, side_body: str, tdigest: int) -> str:
    """``XXH3-64("rehoboam/annals/v0" || templates_digest LE || text || LF || sidecar)``, 16 hex."""
    data = (
        D_ANNALS
        + tdigest.to_bytes(8, "little")
        + text.encode("utf-8")
        + b"\n"
        + side_body.encode("utf-8")
    )
    return f"{int(_canon.xxh3_64(data)):016x}"


def finish(annal: Annal, r: Renderer, tdigest: int) -> tuple[str, str]:
    """The text and the sidecar as files: the header, rows, trailers and the digest line."""
    text = "\n".join(annal.lines) + "\n"
    head = (
        f"#{FORMAT} world={r.w.id} group={annal.group} timeline={r.c.timeline} "
        f"chronicle_digest={r.c.digest} registry_version={r.c.registry_version} "
        f"keyframe_extract={r.w.extract_digest} state_hash={r.w.state_hash} "
        f"manifest_hash={r.w.manifest_hash} templates={r.t.version} "
        f"templates_digest={tdigest:016x} renderer={RENDERER} calendar={r.w.calendar.days_per_year}"
    )
    body = [head, *annal.rows]
    trailers = (
        ("omitted", annal.omitted, "of kinds outside the template set"),
        ("castless", annal.castless, "with no cast and no told triggered edge into this group"),
    )
    for label, counter, what in trailers:
        if counter:
            kinds = ", ".join(f"{k} x{n}" for k, n in sorted(counter.items()))
            body.append(f"# {label}: {sum(counter.values())} event(s) {what}: {kinds}")
    side = "\n".join(body) + "\n"
    return text, side + f"digest {annals_digest(text, side, tdigest)}\n"


def render_group(
    world_dir: Path,
    group: str,
    templates: Any,  # noqa: ANN401
    dictionary: Dictionary,
    *,
    disambiguate: bool = True,
) -> tuple[str, str, Renderer]:
    """Load a fixture directory and render one group: ``(text, sidecar, renderer)``."""
    world = load_world(world_dir)
    chron = load_chronicle(world_dir / "chronicle_A.txt", dictionary, world.calendar)
    r = Renderer(templates, world, chron, dictionary, disambiguate=disambiguate)
    annal = render(r, group)
    text, side = finish(annal, r, templates_digest(templates.path))
    return text, side, r


def manifest(world: str, groups: dict[str, str], r: Renderer, tdigest: int) -> str:
    """The annals manifest of one world: its groups in (kind, numeric id) order, with digests."""
    head = (
        f"#{MANIFEST_FORMAT} world={world} chronicle_digest={r.c.digest} "
        f"templates={r.t.version} templates_digest={tdigest:016x}"
    )
    order = sorted(groups, key=lambda g: (g.partition(":")[0], int(g.partition(":")[2] or 0)))
    lines = [head, *(f"group {g} {groups[g]}" for g in order)]
    digest = _canon.xxh3_64(b"".join(line.encode("utf-8") + b"\n" for line in lines))
    return "\n".join([*lines, f"digest {int(digest):016x}"]) + "\n"


def fixture_groups(world: str) -> tuple[str, ...]:
    """The groups a fixture renders: each house of the extract, the least named id's line, years."""
    w = load_world(FIXTURES / world)
    groups = tuple(f"house:{h}" for h in sorted(set(w.members.values())))
    if w.members:
        groups += (f"line:{min(w.persons)}",)
    return (*groups, "year")


def verify(text_path: Path, side_path: Path) -> list[str]:
    """Recompute a committed sidecar's digest from its text and sidecar files (REV-SPR-029 F26)."""
    side = side_path.read_text(encoding="utf-8")
    body, _, tail = side.rpartition("digest ")
    header = re.search(r"templates_digest=([0-9a-f]{16})", side.splitlines()[0])
    if header is None or not tail.strip():
        return [f"{side_path}: no header with templates_digest, or no digest line"]
    want = annals_digest(text_path.read_text(encoding="utf-8"), body, int(header[1], 16))
    return (
        []
        if want == tail.strip()
        else [f"{side_path}: digest {tail.strip()} is not the recomputed {want}"]
    )


# --- the self-test -------------------------------------------------------------------------------


@dataclass(frozen=True)
class Control:
    """The positive control in force (each must FAIL its check by name)."""

    name: str = ""


def _plant_templates(t: Any, control: Control) -> None:  # noqa: ANN401
    natural = next(e for e in t.by_kind["DEATH"] if e["id"] == "death.natural")
    if control.name == "undeclared-slot":
        natural["text"] = str(natural["text"]) + " The instrument was {instrument}."
    if control.name == "state-read":
        natural["text"] = str(natural["text"]) + " The title passed to {state.title.holder}."


def _plant_chronicle(chron: Chronicle, control: Control) -> None:
    """The fixture controls of check 5 (R26): a deposition on day 1; a claim with no cause."""
    moved: set[int] = set()
    for ev in list(chron.events.values()):
        triggered = {c for c, code, _, _, _, _ in ev.causes if code == TRIGGERED}
        if control.name == "fixture-deposed-day-one" and (
            ev.kind == "DEPOSED" or (ev.kind == "CLAIM_CREATED" and triggered & moved)
        ):  # each deposition moves with the claims it creates (R37)
            chron.events[ev.id] = dataclasses.replace(ev, tick=ev.tick - MINUTES_PER_DAY)
            moved.add(ev.id)
        if control.name == "fixture-claim-untriggered" and ev.kind == "CLAIM_CREATED":
            chron.events[ev.id] = dataclasses.replace(ev, causes=())


def _row_fields(line: str) -> list[str]:
    return line.split(" ")


def _plant_text(text: str, side: str, control: Control) -> tuple[str, str]:
    """The text mutants: no-event-id, prologue-altered (P12) and text-truncated (P23).

    A sentence appended with no row; a prologue sentence's text altered; the last sentence and
    its row dropped, so the text is shorter than the render (R34).
    """
    lines = text.splitlines()
    if control.name == "no-event-id":
        lines.append(PLANTED_LINE)
    elif control.name == "prologue-altered":
        n = next((i for i, line in enumerate(lines) if line.startswith("## ")), None)
        if n is None or lines[n] != PROLOGUE_HEADING:
            return text, side
        lines[n + 1] = lines[n + 1].removesuffix(".") + ", so the record says."
    elif control.name == "text-truncated":
        n = max(i for i, line in enumerate(lines) if line and not line.startswith("#"))
        lines = lines[:n]
        side = (
            "\n".join(
                line
                for line in side.splitlines()
                if line[:2] not in {"S ", "P "} or int(_row_fields(line)[1]) != n + 1
            )
            + "\n"
        )
    else:
        return text, side
    return "\n".join(lines) + "\n", side


def _other_root(r: Renderer, group: str) -> int | None:
    """Another house's root (its earliest-born keyframe member), for the P13 control."""
    others = {h for h in r.w.members.values() if f"house:{h}" != group}
    rows = [r.w.kf_life[p] for p, h in r.w.members.items() if h in others and p in r.w.kf_life]
    return min(rows, key=lambda row: (row.birth_tick, row.person)).person if rows else None


def _plant_sidecar(side: str, control: Control, other_root: int | None) -> str:
    """The sidecar mutants of REV-SPR-029 F2, G2 and R19: P1, P3, P4, P5, P6, P7b, P13."""
    lines = side.splitlines()
    srows = [i for i, line in enumerate(lines) if line.startswith("S ")]
    prows = [i for i, line in enumerate(lines) if line.startswith("P ")]
    if len(srows) < MIN_ROWS_FOR_MUTANTS:
        return side
    first, second = srows[0], srows[1]
    f = _row_fields(lines[first])
    match control.name:
        case "row-drops-own":  # P1: a row cites its cause but not its own event
            i = next((i for i in srows if "," in _row_fields(lines[i])[4]), None)
            if i is None:  # a group with no cited cause has no row to plant on
                return side
            f = _row_fields(lines[i])
            ids = [int(e) for e in f[4][1:-1].split(",")]
            f[4] = "[" + ",".join(str(e) for e in ids if e != max(ids)) + "]"
            lines[i] = " ".join(f)
        case "body-as-prologue":  # P3: a body sentence retagged as a prologue row
            f[0], f[6] = "P", "[kf:life/1]"
            lines[first] = " ".join(f)
        case "missing-kf-source":  # P4: a prologue row citing a keyframe row that does not exist
            i = prows[0] if prows else first
            lines[i] = lines[i].rsplit(" ", 1)[0] + " [kf:life/99999]"
        case "unrelated-event":  # P5: an existing, unrelated event cited
            own = int(f[4][1:-1].split(",")[-1])
            f[4] = f[4][:-1] + f",{0 if own != 0 else 1}]"
            lines[first] = " ".join(f)
        case "rows-swapped":  # P6: two rows with their contents exchanged
            a, b = _row_fields(lines[first]), _row_fields(lines[second])
            lines[first], lines[second] = " ".join([*a[:2], *b[2:]]), " ".join([*b[:2], *a[2:]])
        case "foreign-root" if prows and other_root:  # P13: another house's root cited
            p = _row_fields(lines[prows[0]])
            p[5] = f"[{other_root}]"
            lines[prows[0]] = " ".join(p)
        case "forged-source":  # P7b: a forged source row (a life row the chronicle has not)
            forged = f"life/{FORGED_ROW}"
            f[6] = f[6][:-1] + (f",{forged}]" if f[6] != "[]" else f"{forged}]")
            lines[first] = " ".join(f)
        case _:
            return side
    return "\n".join(lines) + "\n"


def _compare_rows(got: list[str], want: list[str], what: str) -> list[str]:
    """Field by field, in list order: the first field that differs, by name (R19)."""
    names = ("tag", "line", "template", "version", "events", "persons", "sources")
    out: list[str] = []
    for g, w in zip(got, want, strict=False):
        gf, wf = _row_fields(g), _row_fields(w)
        if gf == wf:
            continue
        for name, a, b in zip(names, gf, wf, strict=False):
            if a != b:
                out.append(f"{what} line {gf[1]}: {name} differs: {a} where the render gives {b}")
                break
        else:
            out.append(f"{what} line {gf[1]}: the row differs: {g!r} where the render gives {w!r}")
    if len(got) != len(want):
        out.append(f"{what}: {len(got)} rows where the render gives {len(want)}")
    return out


def _check_prologue_rules(text: str, side: str, w: World) -> list[str]:
    """D50 as ruled: ``P`` rows before the first year heading, no events, existing keyframe rows."""
    lines = text.splitlines()
    years = [
        n for n, line in enumerate(lines, 1) if line.startswith("## ") and line != PROLOGUE_HEADING
    ]
    first_year = years[0] if years else len(lines) + 1
    out: list[str] = []
    for line in side.splitlines():
        if not line.startswith("P "):
            continue
        _, n, _, _, events, _, sources = _row_fields(line)
        if int(n) >= first_year:
            out.append(f"line {n}: a prologue row after the first year heading")
        if events != "[]":
            out.append(f"line {n}: a prologue row cites events {events}")
        kf = [s for s in sources[1:-1].split(",") if s.startswith("kf:")]
        if not kf:
            out.append(f"line {n}: a prologue sentence cites no keyframe row")
        out += [
            f"line {n}: source {s} is not a row of the keyframe extract"
            for s in kf
            if not _kf_source_exists(s, w)
        ]
    return out


def _kf_source_exists(source: str, w: World) -> bool:
    kind, _, rest = source.partition("/")
    if kind == "kf:life":
        return int(rest) in w.kf_life
    if kind == "kf:member":
        return int(rest) in w.members
    if kind == "kf:tenure":
        title, _, start = rest.partition("/")
        return any(t.title == int(title) and t.start == int(start) for t in w.tenures)
    return False


def check_provenance(text: str, side: str, r: Renderer, group: str) -> list[str]:
    """V-NAR-05 (R19): the group re-rendered from its inputs; text and sidecar compared in order."""
    want_text, want_side = finish(render(r, group), r, templates_digest(r.t.path))
    out: list[str] = []
    got_lines, want_lines = text.splitlines(), want_text.splitlines()
    rows = {int(_row_fields(line)[1]) for line in side.splitlines() if line[:2] in {"S ", "P "}}
    for n, (g, w) in enumerate(zip(got_lines, want_lines, strict=False), 1):
        if g != w:
            out.append(
                f"line {n}: the text differs from the render: {g!r} where the render gives {w!r}"
            )
            break
    for n in range(len(got_lines) + 1, len(want_lines) + 1):  # a text shorter than the render
        out.append(f"line {n}: missing: {want_lines[n - 1]!r}")  # (R34)
    for n in range(len(want_lines) + 1, len(got_lines) + 1):
        line = got_lines[n - 1]
        out.append(
            f"line {n} has no sidecar row: {line!r}"
            if n not in rows
            else f"line {n}: an extra line: {line!r}"
        )
    got_side, want_side_lines = side.splitlines(), want_side.splitlines()
    if got_side[:1] != want_side_lines[:1]:  # the first differing token, by name (R37)
        got_toks = got_side[0].split(" ") if got_side else []
        want_toks = want_side_lines[0].split(" ")
        n = next(  # the first position whose token differs, is extra or is missing (R41)
            i
            for i in range(max(len(got_toks), len(want_toks)))
            if i >= len(got_toks) or i >= len(want_toks) or got_toks[i] != want_toks[i]
        )
        got_tok = got_toks[n] if n < len(got_toks) else "nothing"
        want_tok = want_toks[n] if n < len(want_toks) else "nothing"
        out.append(f"header: token {n} differs: {got_tok} where the render gives {want_tok}")
    rows_got = [line for line in got_side if line[:2] in {"S ", "P "}]
    rows_want = [line for line in want_side_lines if line[:2] in {"S ", "P "}]
    out += _compare_rows(rows_got, rows_want, "sidecar")
    for what in ("# omitted:", "# castless:", "digest "):
        gl = next((line for line in got_side if line.startswith(what)), None)
        wl = next((line for line in want_side_lines if line.startswith(what)), None)
        if gl != wl:
            out.append(
                f"sidecar: {what.strip(' :#')} line differs: {gl!r} where the render gives {wl!r}"
            )
    return out + _check_prologue_rules(text, side, r.w)


def check_calendar() -> list[str]:
    """Known answers of the two calendars, before and after the epoch; the words; q8_8."""
    c360, c365 = Calendar(360, "Y"), Calendar(365, "Y")
    day = MINUTES_PER_DAY
    cases = [
        (c360, 0, (0, 1, 1)),
        (c360, day * 73, (0, 3, 14)),
        (c360, day * 360 * 12 - day, (11, 12, 30)),
        (c360, -day, (-1, 12, 30)),
        (c360, -day * 360, (-1, 1, 1)),
        (c365, day * 59, (0, 3, 1)),
        (c365, day * 364, (0, 12, 31)),
        (c365, day * 365 + day - 1, (1, 1, 1)),
        (c365, -1, (-1, 12, 31)),
    ]
    out = [
        f"{c.days_per_year}-day: tick {t} gave {c.date(t)}, not {w}"
        for c, t, w in cases
        if c.date(t) != w
    ]
    words = [
        (1, "first"),
        (2, "second"),
        (12, "twelfth"),
        (13, "thirteenth"),
        (20, "twentieth"),
        (21, "twenty-first"),
        (32, "thirty-second"),
        (100, "one hundredth"),
        (111, "one hundred and eleventh"),
        (1000, "one thousandth"),
        (1001, "one thousand and first"),
        (1100, "one thousand one hundredth"),
        (2024, "two thousand and twenty-fourth"),
        (999_999, "nine hundred and ninety-nine thousand nine hundred and ninety-ninth"),
    ]
    out += [
        f"ordinal({n}) = {ordinal(n)!r}, not {want!r}" for n, want in words if ordinal(n) != want
    ]
    cardinals = (
        (0, "zero"),
        (1, "one"),
        (21, "twenty-one"),
        (1000, "one thousand"),
        (1001, "one thousand and one"),
        (1100, "one thousand one hundred"),
        (2024, "two thousand and twenty-four"),
        (999_999, "nine hundred and ninety-nine thousand nine hundred and ninety-nine"),
    )
    out += [f"cardinal({n}) = {cardinal(n)!r}" for n, want in cardinals if cardinal(n) != want]
    if q8_8(16128) != "63.0" or q8_8(64) != "0.25" or q8_8(-384) != "-1.5":
        out.append("q8_8 rendering")
    for bad in (lambda: Calendar(364, "Y"), lambda: cardinal(WORDS_LIMIT)):
        try:
            bad()
            out.append("a 364-day calendar or a number past the words' range was accepted")
        except AnnalsError:
            pass
    return out


def check_dictionary(chron: Chronicle, d: Dictionary) -> list[str]:
    """Every fixture EVENT agrees with the dictionary the chronicle header names."""
    out: list[str] = []
    for rec in chron.records:
        if rec.kind != "EVENT":
            continue
        verb = str(rec["verb"])
        if verb not in d.verbs:
            out.append(f"event {rec['event_id']}: verb {verb} is not in the dictionary")
            continue
        bits = sum(1 << d.act_classes.index(str(a)) for a in d.verbs[verb]["act_classes"])
        want = (d.verb_names.index(verb), bits, d.verbs[verb]["tellability"])
        got = (rec["verb_id"], rec["act_classes"], rec["tellability"])
        if got != want:
            out.append(
                f"event {rec['event_id']} ({verb}): (verb_id, act_classes, tellability)"
                f" {got} is not {want}"
            )
        out += [
            f"event {rec['event_id']}: snapshot attribute {a} is unknown"
            for a, *_ in rec["snapshots"]
            if a not in d.attrs
        ]
    return out


class History:
    """A possible history under M2's and M7's rules, as the fixtures must be (annals_v0.md §12)."""

    def __init__(self, chron: Chronicle, world: World) -> None:
        """Track holders, claims and the dead while walking the events in id order."""
        self.c, self.w, self.cal = chron, world, world.calendar
        self.holders: dict[int, int | None] = {
            t.title: t.holder for t in world.tenures if t.end is None
        }
        self.claims: dict[int, set[int]] = {}  # claimant -> titles with an unrenounced claim
        self.problems: list[str] = []

    def same_tick(self, ev: Event, kind: str) -> list[Event]:
        """The events of ``kind`` at the event's tick."""
        return [e for e in self.c.events.values() if e.tick == ev.tick and e.kind == kind]

    def birth(self, ev: Event) -> None:
        """A birth nine months after a conception at or after the epoch, the father then alive."""
        if self.cal.month_index(ev.tick) < GESTATION_MONTHS:
            self.problems.append(
                f"event {ev.id}: a birth from a pregnancy pending at genesis (SPR-019 D-08)"
            )
        child = ev.person(TARGET)
        row = None if child is None else self.c.life.get(child)
        if row is None or row.father not in self.c.exits:
            return
        if (
            self.cal.month_index(self.c.exits[row.father])
            < self.cal.month_index(ev.tick) - GESTATION_MONTHS
        ):
            self.problems.append(
                f"event {ev.id}: a birth more than nine months after the father's death"
            )

    def death(self, ev: Event) -> None:
        """A dead holder's title passes or falls vacant in the same instant (SAGA-13)."""
        dead = ev.person(TARGET)
        for title, holder in self.holders.items():
            if holder != dead:
                continue
            passed = [
                e
                for e in self.same_tick(ev, "SUCCEED")
                if e.person(TARGET) == dead and e.payload[:1] == (title,)
            ]
            vacant = [
                e for e in self.same_tick(ev, "INTERREGNUM_CRISIS") if e.payload[:1] == (title,)
            ]
            if not passed and not vacant:
                self.problems.append(
                    f"event {ev.id}: the holder of title {title} dies with no SUCCEED or "
                    "INTERREGNUM_CRISIS in the instant"
                )
            self.holders[title] = None

    def m7(self, ev: Event) -> None:
        """M7's rules: payloads name the title, claims arise at successions and depositions."""
        if not ev.payload:
            self.problems.append(
                f"event {ev.id} ({ev.kind}): an M7 payload names no title (envelope_v0.md §1)"
            )
            return
        title, actor, target = ev.payload[0], ev.person(ACTOR), ev.person(TARGET)
        if ev.kind == "SUCCEED":
            if self.holders.get(title) not in (target, None):
                self.problems.append(f"event {ev.id}: SUCCEED's target is not the title's holder")
            self.holders[title] = actor
        elif ev.kind == "DEPOSED":
            self.deposed(ev, title, actor, target)
        elif ev.kind == "CLAIM_CREATED":
            self.claim(ev, title, actor)
        elif ev.kind == "CLAIM_RENOUNCED" and actor is not None:
            self.claims.get(actor, set()).discard(title)
        elif ev.kind == "INTERREGNUM_CRISIS":
            self.holders[title] = None

    def deposed(self, ev: Event, title: int, actor: int | None, target: int | None) -> None:
        """A deposition on day 2 or later; the deposer holds a claim; the deposed gains one."""
        if self.cal.date(ev.tick)[2] < DEPOSITION_FIRST_DAY:
            self.problems.append(
                f"event {ev.id}: a deposition on day 1 (M7's timer fires on days 2 to the last)"
            )
        if actor is not None and title not in self.claims.get(actor, set()):
            self.problems.append(
                f"event {ev.id}: the deposer holds no unrenounced claim to title {title}"
            )
        if not any(e.person(ACTOR) == target for e in self.same_tick(ev, "CLAIM_CREATED")):
            self.problems.append(f"event {ev.id}: the deposed gains no claim in the instant")
        self.holders[title] = None

    def claim(self, ev: Event, title: int, actor: int | None) -> None:
        """A claim arises only from a succession that passed the heir over or from a deposition."""
        causes = {
            self.c.events[c].kind
            for c, code, _, _, _, told in ev.causes
            if told and code == TRIGGERED and c in self.c.events
        }
        if not causes & CLAIM_CAUSES:
            self.problems.append(
                f"event {ev.id}: a claim created with no triggered edge to a succession or a "
                "deposition (M7's when)"
            )
        if actor is not None:
            self.claims.setdefault(actor, set()).add(title)

    def murder_attempt(self, ev: Event) -> None:
        """T1's edges: motivated_by, or triggered to a MENTAL_BREAK (envelope_v0.md §3)."""
        for c, code, _, _, _, _ in ev.causes:
            cause = self.c.events.get(c)
            if code == TRIGGERED and cause is not None and cause.kind != "MENTAL_BREAK":
                self.problems.append(f"event {ev.id}: a MURDER_ATTEMPT triggered by a {cause.kind}")

    def run(self) -> list[str]:
        """Walk the chronicle; every problem found."""
        for ev in sorted(self.c.events.values(), key=lambda e: e.id):
            if ev.kind == "BIRTH":
                self.birth(ev)
            elif ev.kind == "DEATH":
                self.death(ev)
            elif ev.kind in M7_KINDS:
                self.m7(ev)
            elif ev.kind == "MURDER_ATTEMPT":
                self.murder_attempt(ev)
        return self.problems


def _golden_paths(golden: Path, world: str, group: str) -> tuple[Path, Path]:
    stem = f"{world}_{group.replace(':', '-')}"
    return golden / f"{stem}.txt", golden / f"{stem}.sidecar.txt"


@dataclass
class Run:
    """What one ``--check`` run accumulates."""

    results: list[tuple[str, list[str]]] = field(default_factory=list)
    stale: list[str] = field(default_factory=list)
    orphans: list[str] = field(default_factory=list)  # golden files no group owns (R39)
    used: Counter[str] = field(default_factory=Counter)
    kinds_used: set[str] = field(default_factory=set)
    provenance: list[str] = field(default_factory=list)
    determinism: list[str] = field(default_factory=list)
    injective: list[str] = field(default_factory=list)

    def add(self, name: str, problems: list[str]) -> None:
        """Record a check's outcome."""
        self.results.append((name, problems))


@dataclass(frozen=True)
class Setup:
    """The templates, the dictionary, the control and the golden directory of one run."""

    t: Any
    d: Dictionary
    control: Control
    golden: Path


def annal_persons(r: Renderer, members: set[int] | None) -> set[int]:
    """Every person an annal designates: members, its events' and their causes' cast, fathers."""
    pool = set(members or ())
    for ev in r.group_events(members):
        pool.update(ev.persons())
        for c, _, _, _, _, told in ev.causes:
            if told and c in r.c.events:
                pool.update(r.c.events[c].persons())
        row = r._father_row(ev) if ev.kind == "BIRTH" else None
        if row is not None:
            pool.add(row.father)
    return pool


def _check_injective(r: Renderer, group: str) -> list[str]:
    """No two persons of one annal share a designation (R6, R24)."""
    seen: dict[str, int] = {}
    out: list[str] = []
    for pid in sorted(annal_persons(r, group_of(r, group)[1])):
        text = r.kin.designate(pid, Provenance())
        if text in seen:
            out.append(f"{group}: persons {seen[text]} and {pid} share the designation {text!r}")
        seen[text] = pid
    return out


def _check_shared_name(d: Dictionary) -> list[str]:
    """A planted shared name is disambiguated with each person's compact label (R24)."""
    world = load_world(FIXTURES / "ashcombe")
    chron = load_chronicle(FIXTURES / "ashcombe" / "chronicle_A.txt", d, world.calendar)
    named = sorted(world.persons)
    world.persons[named[1]] = world.persons[named[0]]
    kin = Kinship(world, chron)
    name = world.persons[named[0]]
    got = [kin.designate(p, Provenance()) for p in named[:2]]
    want = [f"{name} ({kin.label_of(p, Provenance())})" for p in named[:2]]
    if got != want or got[0] == got[1]:
        return [f"a shared name {name!r} rendered as {got[0]!r} and {got[1]!r}, not {want}"]
    return []


def _other_process(world: str, group: str, seed: str) -> tuple[str, str]:
    """Render ``group`` in another interpreter under ``PYTHONHASHSEED=seed`` (R29)."""
    with tempfile.TemporaryDirectory() as tmp:
        text, side = Path(tmp) / "text", Path(tmp) / "side"
        argv = [
            sys.executable,
            "-B",
            "-s",
            str(Path(__file__)),
            "render",
            "--world",
            str(FIXTURES / world),
            "--group",
            group,
            "--out",
            str(text),
            "--sidecar-out",
            str(side),
        ]
        env = {"PYTHONHASHSEED": seed, "PATH": os.environ.get("PATH", "")}
        rc = os.spawnve(os.P_WAIT, sys.executable, argv, env)
        if rc != 0:
            raise AnnalsError(f"the render in another process exited {rc}")
        return text.read_text(encoding="utf-8"), side.read_text(encoding="utf-8")


def _render_one(run: Run, setup: Setup, world: str, group: str) -> tuple[str, Renderer]:
    """Render a group, plant the control, run the per-group checks; return the sidecar's digest."""
    if setup.control.name == "network-import":
        importlib.import_module("urllib.request")
    text, side, r = render_group(FIXTURES / world, group, setup.t, setup.d)
    for seed in HASH_SEEDS:
        if _other_process(world, group, seed) != (text, side):
            run.determinism.append(
                f"{world} {group}: the render under PYTHONHASHSEED={seed} differs"
            )
    text, side = _plant_text(text, side, setup.control)
    side = _plant_sidecar(side, setup.control, _other_root(r, group))
    run.provenance += [f"{world} {group}: {p}" for p in check_provenance(text, side, r, group)]
    run.injective += _check_injective(r, group)
    run.used.update(r.used)
    run.kinds_used.update(ev.kind for ev in r.group_events(group_of(r, group)[1]) if r.variant(ev))
    text_path, side_path = _golden_paths(setup.golden, world, group)
    stale = [
        p
        for p, want in ((text_path, text), (side_path, side))
        if not p.exists() or p.read_bytes() != want.encode("utf-8")
    ]
    run.stale += [str(p) for p in stale]
    if not stale:  # the committed digest recomputes from the committed files (F26)
        run.determinism += verify(text_path, side_path)
    return side.rstrip("\n").rsplit(" ", 1)[1], r


def checked_groups(world: str) -> tuple[str, ...]:
    """The manifest's groups and the extra groups check 8 renders (R39)."""
    return (*fixture_groups(world), *EXTRA_GROUPS.get(world, ()))


def golden_files(golden: Path) -> set[Path]:
    """Every file a rendered group or a manifest owns in ``golden``."""
    owned: set[Path] = set()
    for world in FIXTURE_WORLDS:
        owned.add(golden / f"{world}.manifest.txt")
        for group in checked_groups(world):
            owned.update(_golden_paths(golden, world, group))
    return owned


def _render_world(run: Run, setup: Setup, world: str) -> None:
    digests: dict[str, str] = {}
    r: Renderer | None = None
    for group in checked_groups(world):
        digests[group], r = _render_one(run, setup, world, group)
    assert r is not None
    listed = {g: digests[g] for g in fixture_groups(world)}  # the manifest lists §9's groups only
    path = setup.golden / f"{world}.manifest.txt"
    want = manifest(world, listed, r, templates_digest(setup.t.path))
    if not path.exists() or path.read_text(encoding="utf-8") != want:
        run.stale.append(str(path))


def check_imports() -> list[str]:
    """V-NAR-07: the imports of this file are the standard library and the sibling references."""
    out: list[str] = []
    for line in Path(__file__).read_text(encoding="utf-8").splitlines():
        found = IMPORT_LINE.match(line)
        if found is None:
            continue
        module = found[1].split(".")[0]
        if module not in ALLOWED_IMPORTS and module not in SIBLINGS and module != "__future__":
            out.append(f"import of {module!r}")
    return out


def check_modules() -> list[str]:
    """V-NAR-07 (F16): after rendering, no network or process module is loaded."""
    return [
        f"module {m} was imported during the render" for m in FORBIDDEN_MODULES if m in sys.modules
    ]


def _check_fixtures(d: Dictionary, control: Control) -> list[str]:
    problems: list[str] = []
    for world in FIXTURE_WORLDS:
        w = load_world(FIXTURES / world)
        chron = load_chronicle(FIXTURES / world / "chronicle_A.txt", d, w.calendar)
        _plant_chronicle(chron, control)
        problems += [f"{world}: {p}" for p in check_dictionary(chron, d) + History(chron, w).run()]
    return problems


def check_extract_canonical() -> list[str]:
    """R23: the extract digest does not depend on the order of the TOML's rows."""
    path = FIXTURES / "ashcombe" / "keyframe.toml"
    kf = tomllib.loads(path.read_text(encoding="utf-8"))
    straight = World("x", Calendar(360, "Y"))
    _keyframe(straight, kf, str(path))
    shuffled = World("x", Calendar(360, "Y"))
    _keyframe(
        shuffled, {k: (v[::-1] if isinstance(v, list) else v) for k, v in kf.items()}, str(path)
    )
    if straight.extract_digest != shuffled.extract_digest:
        return ["the extract digest changed when the rows were reordered"]
    return []


# Check 11's known answers: a two-segment chronicle written out here (the `solitary` fixture's four
# records at AN-0, cut at its sim-years 0 and 3), so that a fixture change cannot move them. The
# digests were derived outside this file with the C xxHash (python-xxhash 4.0.1, XXH3-128, seed 0,
# canonical form) and are re-derived below from bin/gen_hash_golden.py, never through `fold`.
KAT_HEADER = (
    "#rehoboam/stream/canonical/v0 timeline=0 channel=A registry_version=0.2.0+next "
    "format=rehoboam/stream/channel-a/v0"
)
KAT_LIFE = "LIFE_ENTRY 0 0 0 1328186490420642308 1 1 0 0 -12875040"
KAT_YEAR3 = (
    "EVENT 0 1 1634400 0 4 15 DEATH 32768 ~ 0 128 0 0 [1] [1/1328186490420642308/1] [] []",
    "EVENT 0 2 1634400 1 4 18 EXTINCTION 32768 ~ 0 255 0 0 [] [] [] [0/1/~/0/256/1]",
    "LIFE_EXIT 0 3 1634400 1328186490420642308 0 1 0",
)
KAT_D0 = "f32459f3cf28ee72e4bd92621a43dec6"  # canonical_digest of y000000 (the LIFE_ENTRY)
KAT_D3 = "497ac1184aadd39d525247eb1f164cc7"  # canonical_digest of y000003 (three records)
KAT_FOLD = "5ff75d89291d7870fce169d9019ff3b2"  # XXH3-128(D0 || D3)
KAT_ONE = "dc0d277e20275f23f7503a382de3984b"  # the four records as one segment
KAT_SEG0 = f"#segment y000000 first_seq=0 last_seq=0 canonical_digest={KAT_D0}"
KAT_SEG3 = f"#segment y000003 first_seq=1 last_seq=3 canonical_digest={KAT_D3}"
KAT_BLOCK0 = f"{KAT_SEG0}\n{KAT_LIFE}\n"
KAT_BLOCK3 = "\n".join((KAT_SEG3, *KAT_YEAR3)) + "\n"
KAT_TEXT = f"{KAT_HEADER}\n{KAT_BLOCK0}{KAT_BLOCK3}"
KAT_REC4 = "LIFE_ENTRY 0 4 1634400 1328186490420642309 1 1 0 0 -12875040"
KAT_RECORDS_COUNT = 4
EMPTY_DIGEST = "99aa06d3014798d86001c324468d497f"  # XXH3-128 of no bytes (canonical_v0.md §4)


def _xxh128(data: bytes) -> bytes:
    """XXH3-128 straight from bin/gen_hash_golden.py, in canonical form (canonical_v0.md §4)."""
    return int(_xxh.xxh3_128(data)).to_bytes(16, "big")


def _lines_digest(lines: tuple[str, ...] | list[str]) -> bytes:
    """canonical_digest of record lines (canonical_v0.md §4), from the XXH3 reference alone."""
    return _xxh128(b"".join(line.encode("ascii") + b"\n" for line in lines))


# Every refusal of §9.1, planted in the known-answer file: (case, text replaced, its replacement,
# the words the refusal must hold).
FOLD_REFUSALS = (
    (
        "a record before the first segment line",
        KAT_BLOCK0,
        f"{KAT_LIFE}\n{KAT_SEG0}\n",
        ":2: a record before the first #segment line",
    ),
    (
        "an empty segment",
        KAT_SEG3,
        f"#segment y000001 first_seq=1 last_seq=1 canonical_digest={EMPTY_DIGEST}\n{KAT_SEG3}",
        ":4: segment y000001 holds no record",
    ),
    (
        "a file with no records",
        KAT_BLOCK0 + KAT_BLOCK3,
        "",
        "a chronicle holds at least one record (annals_v0.md §9.1)",
    ),
    (
        "a repeated year",
        KAT_SEG3,
        KAT_SEG3.replace("y000003", "y000000"),
        ":4: segment y000000 repeats (line 2)",
    ),
    (
        "a year out of order",
        KAT_SEG0,
        KAT_SEG0.replace("y000000", "y000005"),
        ":4: segment y000003 follows y000005: the years must increase",
    ),
    (
        "a year one lower than the last (y000002 after y000003)",
        KAT_YEAR3[-1],
        (
            f"{KAT_YEAR3[-1]}\n#segment y000002 first_seq=4 last_seq=4 "
            f"canonical_digest={_lines_digest([KAT_REC4]).hex()}\n{KAT_REC4}"
        ),
        "segment y000002 follows y000003: the years must increase",
    ),
    (
        "the segment lines swapped, the records in place",
        KAT_BLOCK0 + KAT_SEG3,
        f"{KAT_SEG3}\n{KAT_LIFE}\n{KAT_SEG0}",
        ":4: segment y000000 follows y000003: the years must increase",
    ),
    (
        "two blocks swapped (canonical_v0.md §1: contiguous seq)",
        KAT_BLOCK0 + KAT_BLOCK3,
        KAT_BLOCK3 + KAT_BLOCK0,
        "seq 0 does not follow 3",
    ),
    (
        "a digest that is not the records'",
        KAT_SEG3,
        KAT_SEG3[:-1] + "0",
        f":4: segment y000003 (records on lines 5-7): its #segment line says canonical_digest="
        f"{KAT_D3[:-1]}0; its records recompute to {KAT_D3}",
    ),
    (
        "a first_seq that is not the block's",
        KAT_SEG3,
        KAT_SEG3.replace("first_seq=1", "first_seq=2"),
        ":4: segment y000003: first_seq=2 on its #segment line, but its first record has seq 1",
    ),
    (
        "a last_seq that is not the block's",
        KAT_SEG3,
        KAT_SEG3.replace("last_seq=3", "last_seq=4"),
        ":4: segment y000003: last_seq=4 on its #segment line, but its last record has seq 3",
    ),
    (
        "a seq past u64",
        KAT_SEG3,
        KAT_SEG3.replace("first_seq=1", "first_seq=18446744073709551616"),
        ":4: not a segment line (annals_v0.md §9.1): first_seq=18446744073709551616 is past u64",
    ),
    *(
        (f"not a segment line: {case}", KAT_SEG3, planted, ":4: not a segment line")
        for case, planted in (
            ("a five-digit year", KAT_SEG3.replace("y000003", "y00003")),
            ("a seven-digit year", KAT_SEG3.replace("y000003", "y0000003")),
            (
                "a seq of 21 digits",
                KAT_SEG3.replace("first_seq=1", "first_seq=100000000000000000000"),
            ),
            ("a double space", KAT_SEG3.replace(" first_seq", "  first_seq")),
            ("a leading zero", KAT_SEG3.replace("first_seq=1", "first_seq=01")),
            ("an uppercase hex digit", KAT_SEG3.replace("497ac", "497AC")),
            ("a missing field", KAT_SEG3.replace(" last_seq=3", "")),
            ("a trailing token", f"{KAT_SEG3} y000003"),
            ("a longer keyword", KAT_SEG3.replace("#segment ", "#segments ")),
        )
    ),
)


def _expected_fold(text: str, days_per_year: int) -> tuple[int, bytes]:
    """First principles: the records grouped by sim-year, each group hashed, the digests folded."""
    groups: dict[int, list[str]] = {}
    for line in text.splitlines()[1:]:
        if not line.startswith("#"):
            year = int(line.split(" ")[3]) // (days_per_year * MINUTES_PER_DAY)
            groups.setdefault(year, []).append(line)
    digests = [_lines_digest(groups[y]) for y in sorted(groups)]
    return len(digests), digests[0] if len(digests) == 1 else _xxh128(b"".join(digests))


def _fold_text(text: str, where: str) -> str:
    """The fold of a text, or the refusal's words."""
    try:
        return fold(text.encode("ascii"), where).chronicle_digest.hex()
    except (AnnalsError, _canon.CanonicalError) as e:
        return f"REFUSED: {e}"


def _fold_known_answers() -> list[str]:
    """The known answers, re-derived from the XXH3 reference; one segment; none; a changed byte."""
    out: list[str] = []
    derived = (_lines_digest([KAT_LIFE]).hex(), _lines_digest(KAT_YEAR3).hex())
    folded = _xxh128(bytes.fromhex(KAT_D0) + bytes.fromhex(KAT_D3)).hex()
    if derived != (KAT_D0, KAT_D3) or folded != KAT_FOLD:
        out.append(f"the XXH3 reference gives {derived} and {folded}, not the known answers")
    got = fold(KAT_TEXT.encode("ascii"), "kat")
    if got.chronicle_digest.hex() != KAT_FOLD or [s.digest.hex() for s in got.segments] != [
        KAT_D0,
        KAT_D3,
    ]:
        out.append(f"the two-segment known answer folds to {got.chronicle_digest.hex()}")
    cases = (
        ("no segment line", f"{KAT_HEADER}\n{KAT_LIFE}\n" + "\n".join(KAT_YEAR3) + "\n", KAT_ONE),
        ("one segment line: the plain canonical_digest", f"{KAT_HEADER}\n{KAT_BLOCK3}", KAT_D3),
    )
    for name, text, want in cases:
        if (have := _fold_text(text, name)) != want:
            out.append(f"{name}: {have}, not {want}")
    changed = KAT_LIFE.replace("-12875040", "-12875041")
    seg = KAT_SEG0.replace(KAT_D0, _lines_digest([changed]).hex())
    text = KAT_TEXT.replace(KAT_BLOCK0, f"{seg}\n{changed}\n")
    want = _xxh128(_lines_digest([changed]) + bytes.fromhex(KAT_D3)).hex()
    if (have := _fold_text(text, "changed byte")) != want or have == KAT_FOLD:
        out.append(f"a one-byte change with its segment line recomputed folds to {have}")
    return out


def _fold_comments_and_seq() -> list[str]:
    """R11 (start at seq 5) and R2/R8 (free comments anywhere leave fold unchanged)."""
    out: list[str] = []
    # R11: the first block may start at any seq; the exporter starts at 0
    rec5 = KAT_LIFE.replace("LIFE_ENTRY 0 0 0", "LIFE_ENTRY 0 5 0")
    want_seq5 = _lines_digest([rec5]).hex()
    seq5 = (
        f"{KAT_HEADER}\n"
        f"#segment y000000 first_seq=5 last_seq=5 canonical_digest={want_seq5}\n"
        f"{rec5}\n"
    )
    if (have := _fold_text(seq5, "seq5")) != want_seq5:
        out.append(f"block starting at seq 5: {have}, not {want_seq5}")

    # R2 / R8: free comments inside a block, right after a segment line, at EOF
    commented = (
        f"{KAT_HEADER}\n"
        f"{KAT_SEG0}\n"
        "# free comment right after segment line\n"
        "#Segment capital S free comment (R8)\n"
        f"{KAT_LIFE}\n"
        "# free comment inside block 0\n"
        "# segment spaced free comment (R8)\n"
        f"{KAT_SEG3}\n"
        f"{KAT_YEAR3[0]}\n"
        "# STATUS sim_tick=1634400\n"
        f"{KAT_YEAR3[1]}\n"
        "# free comment before last record\n"
        f"{KAT_YEAR3[2]}\n"
        "# STATUS final\n"
        "#^ stream footer\n"
    )
    if (have := _fold_text(commented, "commented")) != KAT_FOLD:
        out.append(f"free comments changed the fold: {have}, not {KAT_FOLD}")
    return out


def _fold_calendar_check() -> list[str]:
    """R2 plant: one-record world at tick 520,000 cut on 360- and 365-day calendars."""
    out: list[str] = []
    tick_mid = 520000  # between 360-day (518400) and 365-day (525600) year boundaries
    one_rec = (
        f"{KAT_HEADER}\nLIFE_ENTRY 0 0 {tick_mid} 1328186490420642308 1 1 0 0 -12875040\n"
    ).encode("ascii")
    cut360 = segment_chronicle(one_rec, 360, "mid360")
    cut365 = segment_chronicle(one_rec, 365, "mid365")
    if "#segment y000001 " not in cut360:
        out.append("segment_chronicle on 360-day calendar did not cut tick 520000 into y000001")
    if "#segment y000000 " not in cut365:
        out.append("segment_chronicle on 365-day calendar did not cut tick 520000 into y000000")
    try:
        check_segment_years(fold(cut360.encode("ascii"), "c360"), Calendar(365, ""), "c360")
        out.append("check_segment_years accepted 360-day cut on 365-day calendar")
    except AnnalsError as e:
        if "sim-year 0" not in str(e):
            out.append(f"check_segment_years wrong error on 360-day cut: {e}")
    try:
        check_segment_years(fold(cut365.encode("ascii"), "c365"), Calendar(360, ""), "c365")
        out.append("check_segment_years accepted 365-day cut on 360-day calendar")
    except AnnalsError as e:
        if "sim-year 1" not in str(e):
            out.append(f"check_segment_years wrong error on 360-day cut: {e}")
    return out


def _fold_plants_and_differentials() -> list[str]:
    """R1 (form feed), R2 (calendar boundaries), R3 (canonical differential), R10 (CLI)."""
    out: list[str] = _fold_calendar_check()
    # R1 plant: form feed inside a comment must never expose a hidden record to load_chronicle
    hidden_comment = (
        f"{KAT_HEADER}\n"
        f"{KAT_SEG0}\n"
        "# note\x0cEVENT 0 9 0 0 4 15 DEATH 32768 ~ 0 128 0 0 [1] [1/1/1] [] []\n"
        f"{KAT_LIFE}\n"
        f"{KAT_BLOCK3}"
    )
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "hidden.txt"
        p.write_bytes(hidden_comment.encode("ascii"))
        chron = load_chronicle(p, Dictionary("0.2.0+next", {}, [], [], {}), Calendar(365, ""))
        if len(chron.records) != KAT_RECORDS_COUNT:
            out.append(
                f"load_chronicle rendered a record hidden behind form feed (R1): "
                f"{len(chron.records)} != {KAT_RECORDS_COUNT}"
            )

    # R3: canonical_v0.REFUSED_FILES differential
    for data, why in _canon.REFUSED_FILES:
        try:
            fold(data, f"canonical refusal: {why}")
            out.append(f"fold accepted {why} from canonical_v0.REFUSED_FILES")
        except (_canon.CanonicalError, AnnalsError):
            pass

    # R10: CLI error handling returns non-zero without traceback
    with contextlib.redirect_stderr(io.StringIO()):
        if _fold_command(argparse.Namespace(file=Path("/nonexistent/file.txt"))) == EXIT_OK:
            out.append("_fold_command on nonexistent file returned EXIT_OK")
        world_ns = argparse.Namespace(world=Path("/nonexistent/world"), out=None)
        if _segment_command(world_ns) == EXIT_OK:
            out.append("_segment_command on nonexistent world returned EXIT_OK")

    return out


def _fold_refusals() -> list[str]:
    """Every refusal of §9.1, each with its words."""
    out: list[str] = []
    for case, old, new, words in FOLD_REFUSALS:
        if KAT_TEXT.count(old) != 1:
            out.append(f"{case}: the plant does not apply")
            continue
        got = _fold_text(KAT_TEXT.replace(old, new), "kat")
        if not got.startswith("REFUSED: ") or words not in got:
            out.append(f"{case}: {got!r}, not a refusal holding {words!r}")
    return out


def _fold_fixture(setup: Setup, world: str) -> list[str]:
    """A fixture cut by sim-year folds as first principles say, and renders the same annals."""
    path = FIXTURES / world / "chronicle_A.txt"
    days = load_world(FIXTURES / world).calendar.days_per_year
    cut = segment_chronicle(path.read_bytes(), days, str(path))
    count, want = _expected_fold(path.read_text(encoding="ascii"), days)
    got = fold(cut.encode("ascii"), f"{world} cut")
    out: list[str] = []
    if len(got.segments) != count or got.chronicle_digest != want:
        out.append(f"{world}: the cut folds to {got.chronicle_digest.hex()}, not {want.hex()}")
    if world == "solitary" and count < MIN_CUT_SEGMENTS:
        out.append("solitary: the cut has fewer than two segments, so it tests no fold")
    plain = fold(path.read_bytes(), str(path)).chronicle_digest.hex()
    if plain != _canon.digest_file(path).hex():
        out.append(f"{world}: the file with no segment line is not its canonical_digest")
    folded = f"chronicle_digest={got.chronicle_digest.hex()} "
    with tempfile.TemporaryDirectory() as tmp:
        cut_dir = Path(tmp)
        for src in (FIXTURES / world).iterdir():
            (cut_dir / src.name).write_bytes(src.read_bytes())
        (cut_dir / "chronicle_A.txt").write_text(cut, encoding="ascii")
        for group in checked_groups(world):
            text, side, _ = render_group(FIXTURES / world, group, setup.t, setup.d)
            cut_text, cut_side, _ = render_group(cut_dir, group, setup.t, setup.d)
            head, *rows = side.splitlines()[:-1]
            want_side = [head.replace(f"chronicle_digest={plain} ", folded), *rows]
            if cut_text != text or cut_side.splitlines()[:-1] != want_side:
                out.append(f"{world} {group}: the cut renders other annals than the file")
    return out


def check_fold(setup: Setup) -> list[str]:
    """Check 11 (annals_v0.md §9.1; AN-0): the chronicle digest of a file of several segments."""
    try:
        out = (
            _fold_known_answers()
            + _fold_comments_and_seq()
            + _fold_plants_and_differentials()
            + _fold_refusals()
        )
        for world in FIXTURE_WORLDS:
            out += _fold_fixture(setup, world)
    except (AnnalsError, _canon.CanonicalError) as e:
        return [f"the fold refused a file it must read: {e}"]
    return out


def self_test(control: Control, golden: Path) -> Run:
    """Run every check; the control plants its defect first."""
    run = Run()
    t = _lint.load_templates(TEMPLATES)
    _plant_templates(t, control)
    lint_problems = _lint.check(t, _lint.load_specs(), _lint.load_registry(REGISTRY))
    run.add("2. template slots (annals_v0.md §3)", lint_problems)
    run.add("4. the calendar (05.1 §4.1)", check_calendar())
    if lint_problems:
        run.add("3. no model (V-NAR-07)", check_imports())
        return run
    setup = Setup(t, load_dictionary(), control, golden)
    for world in FIXTURE_WORLDS:
        _render_world(run, setup, world)
    owned = golden_files(golden)  # a file no rendered group owns is stale (R39)
    run.orphans += [str(p) for p in sorted(golden.iterdir()) if p.is_file() and p not in owned]
    run.results.insert(0, ("1. provenance (V-NAR-05)", run.provenance))
    run.results.insert(2, ("3. no model (V-NAR-07)", check_imports() + check_modules()))
    run.add(
        "5. the fixtures: the dictionary, a possible history", _check_fixtures(setup.d, control)
    )
    run.add("6. determinism (two other processes, the digest recomputes)", run.determinism)
    unexercised = [
        f"kind {k} is not exercised by any fixture" for k in t.scope if k not in run.kinds_used
    ]
    run.add("7. coverage: every in-scope kind rendered", unexercised)
    shared = (
        _check_shared_name(setup.d)
        if control.name != "shared-name"
        else _shared_name_control(setup.d)
    )
    run.add("9. designation is injective", run.injective + shared)
    run.add("10. the extract digest is canonical", check_extract_canonical())
    run.add("11. the chronicle fold over #segment blocks (§9.1; AN-0)", check_fold(setup))
    return run


def _shared_name_control(d: Dictionary) -> list[str]:
    """The ``shared-name`` control: two persons named Maud with disambiguation off (R24)."""
    world = load_world(FIXTURES / "ashcombe")
    named = sorted(world.persons)
    world.persons[named[1]] = world.persons[named[0]]
    r = Renderer(
        _lint.load_templates(TEMPLATES),
        world,
        load_chronicle(FIXTURES / "ashcombe" / "chronicle_A.txt", d, world.calendar),
        d,
        disambiguate=False,
    )
    return _check_injective(r, "year")


def write_golden(golden: Path) -> None:
    """Regenerate every golden annal, sidecar and manifest from the fixtures (never by hand)."""
    t = _lint.load_templates(TEMPLATES)
    d = load_dictionary()
    tdigest = templates_digest(t.path)
    golden.mkdir(parents=True, exist_ok=True)
    for world in FIXTURE_WORLDS:
        digests: dict[str, str] = {}
        for group in checked_groups(world):
            text, side, r = render_group(FIXTURES / world, group, t, d)
            digests[group] = side.rstrip("\n").rsplit(" ", 1)[1]
            for path, data in zip(_golden_paths(golden, world, group), (text, side), strict=True):
                path.write_bytes(data.encode("utf-8"))
                print(f"wrote {path}")
        listed = {g: digests[g] for g in fixture_groups(world)}
        path = golden / f"{world}.manifest.txt"
        path.write_text(manifest(world, listed, r, tdigest), encoding="utf-8")
        print(f"wrote {path}")
    for path in sorted(golden.iterdir()):
        if path.is_file() and path not in golden_files(golden):
            print(f"orphan {path}: no rendered group owns it (delete it)")


def _report(run: Run, t: Any, control: Control) -> int:  # noqa: ANN401
    failed = False
    for name, problems in run.results:
        print(f"{name}: {'FAIL' if problems else 'PASS'}")
        for p in problems:
            print(f"  {p}")
        failed = failed or bool(problems)
    unused = [
        str(e["id"]) for entries in t.by_kind.values() for e in entries if e["id"] not in run.used
    ]
    if unused:
        print(f"  (variants no fixture exercises: {', '.join(unused)})")
    if failed:
        print("ANNALS v0: FAIL")
        return EXIT_SELFTEST
    if control.name:
        print("ANNALS v0: the control did not fail its check")
        return EXIT_SELFTEST
    for path in run.stale:
        print(f"STALE: {path} differs from the render (regenerate with --write)")
    for path in run.orphans:
        print(f"STALE: orphan {path}: no rendered group owns it (delete it)")
    if run.stale or run.orphans:
        return EXIT_STALE
    groups = sum(len(checked_groups(w)) for w in FIXTURE_WORLDS)
    print(f"8. golden annals and manifests: PASS ({groups} groups)")
    print("ANNALS v0: PASS")
    return EXIT_OK


def _render_command(args: argparse.Namespace) -> int:
    text, side, _ = render_group(
        args.world, args.group, _lint.load_templates(TEMPLATES), load_dictionary()
    )
    if args.out is not None:
        args.out.write_text(text, encoding="utf-8")
    if args.sidecar_out is not None:
        args.sidecar_out.write_text(side, encoding="utf-8")
    if args.out is None and args.sidecar_out is None:
        sys.stdout.write(side if args.sidecar else text)
    return EXIT_OK


def _fold_command(args: argparse.Namespace) -> int:
    """Print each segment's digest and the ``chronicle_digest``, or the refusal (§9.1)."""
    path: Path = args.file
    if not path.is_file():
        print(f"FOLD REFUSED: {path}: file does not exist", file=sys.stderr)
        return EXIT_SELFTEST
    try:
        result = fold(path.read_bytes(), str(path))
    except (AnnalsError, _canon.CanonicalError, OSError) as e:
        print(f"FOLD REFUSED: {e}", file=sys.stderr)
        return EXIT_STALE
    for s in result.segments:
        print(
            f"segment y{s.year:06d} first_seq={s.first_seq} last_seq={s.last_seq} "
            f"canonical_digest={s.digest.hex()}"
        )
    if not result.segments:
        print(ONE_SEGMENT_LINE)
    print(f"chronicle_digest={result.chronicle_digest.hex()}")
    return EXIT_OK


def _segment_command(args: argparse.Namespace) -> int:
    """Write a fixture directory's chronicle with one ``#segment`` block per sim-year (§9.1)."""
    path = args.world / "chronicle_A.txt"
    if not path.is_file():
        print(f"SEGMENT REFUSED: {path}: file does not exist", file=sys.stderr)
        return EXIT_SELFTEST
    try:
        days = load_world(args.world).calendar.days_per_year
        text = segment_chronicle(path.read_bytes(), days, str(path))
    except (AnnalsError, _canon.CanonicalError, OSError) as e:
        print(f"SEGMENT REFUSED: {e}", file=sys.stderr)
        return EXIT_STALE
    if args.out is None:
        sys.stdout.write(text)
    else:
        args.out.write_text(text, encoding="ascii")
    return EXIT_OK


def _verify_command(args: argparse.Namespace) -> int:
    """Recompute a committed sidecar's digest from its text and sidecar files (F26)."""
    problems = verify(args.text, args.sidecar)
    print("\n".join(problems) if problems else f"OK: {args.sidecar} digest recomputes")
    return EXIT_SELFTEST if problems else EXIT_OK


def main(argv: list[str] | None = None) -> int:
    """Command line: ``--check``, ``--write``, ``render``, ``verify``, ``fold`` or ``segment``."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--golden-dir", type=Path, default=GOLDEN)
    parser.add_argument("--control", choices=CONTROLS)
    sub = parser.add_subparsers(dest="command")
    rp = sub.add_parser("render", help="render one group of a fixture directory")
    rp.add_argument("--world", type=Path, required=True)
    rp.add_argument("--group", default="year")
    rp.add_argument("--sidecar", action="store_true", help="print the sidecar instead of the text")
    rp.add_argument("--out", type=Path, help="write the text here instead of stdout")
    rp.add_argument("--sidecar-out", type=Path, help="write the sidecar here")
    vp = sub.add_parser("verify", help="recompute a committed sidecar's digest")
    vp.add_argument("--text", type=Path, required=True)
    vp.add_argument("--sidecar", type=Path, required=True)
    fp = sub.add_parser("fold", help="check a chronicle's #segment blocks; print chronicle_digest")
    fp.add_argument("file", type=Path)
    sp = sub.add_parser("segment", help="write a fixture's chronicle one block per sim-year")
    sp.add_argument("--world", type=Path, required=True)
    sp.add_argument("--out", type=Path, help="write the file here instead of stdout")
    args = parser.parse_args(argv)
    commands = {
        "render": _render_command,
        "verify": _verify_command,
        "fold": _fold_command,
        "segment": _segment_command,
    }
    if args.command in commands:
        return commands[args.command](args)
    if args.write:
        write_golden(args.golden_dir)
        return EXIT_OK
    control = Control(args.control or "")
    run = self_test(control, args.golden_dir)
    return _report(run, _lint.load_templates(TEMPLATES), control)


if __name__ == "__main__":
    sys.exit(main())
