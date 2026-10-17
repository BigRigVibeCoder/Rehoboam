"""Family annals v0: the exact reference renderer and its self-test (SPR-029; annals_v0.md).

The renderer reads a channel-A chronicle in the canonical text form (``canonical_v0.py`` parses
and checks it), a world file (the calendar), a names file (what the chronicle does not
carry: the manifest's cast, dynasty and title names) and, for a house annal, a keyframe extract
(the rows of the genesis keyframe that open a house whose past is not in channel A: REV-SPR-019
R1). It renders annals from ``templates_v0.toml`` with a provenance sidecar: one row per sentence
with the event ids it shows, the persons it designates, the records it read, the template id and
version, and a digest of text and sidecar.

Checks (``--check``):

  1. provenance (V-NAR-05): every sentence has one sidecar row; a chronicle sentence cites at least
     one event, every cited event is in the chronicle and of an in-scope kind; a prologue sentence
     cites keyframe rows; the control ``no-event-id`` plants a sentence with no row;
  2. template slots (annals_v0.md §3, ``lint_templates.py``): every slot resolves to a declared
     role, snapshot, payload slot, motive, edge or the tick; the controls ``undeclared-slot`` and
     ``state-read`` plant a slot outside DEATH's roles and a slot that reads state;
  3. no model (V-NAR-07): the renderer imports the standard library and the two sibling references
     only; nothing it calls can reach a model or the network;
  4. the calendar (05.1 §4.1): known answers for 360- and 365-day years, before and after the epoch;
  5. the fixtures agree with the Registry: every ``EVENT``'s verb id, act classes and tellability,
     and the ``stress`` attribute index;
  6. determinism: two renders are byte-equal and the digest recomputes;
  7. coverage: every in-scope kind is exercised by the fixtures (unexercised variants are reported);
  8. golden annals: the committed text and sidecar of every fixture group equal the render
     (``STALE:`` names the first file that differs; ``--write`` regenerates them).

Run: python -I specs/annals/selftest_annals_v0.py --check [--golden-dir DIR]
     [--control no-event-id | undeclared-slot | state-read]
     python -I specs/annals/selftest_annals_v0.py --write
     python -I specs/annals/selftest_annals_v0.py render --world DIR \
         --group house:<id>|year|line:<id>
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
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
LINT = HERE / "lint_templates.py"
REGISTRY = ROOT / "specs" / "manifest" / "registry_v0.toml"
REGISTRY_DELTA = ROOT / "specs" / "systems" / "next" / "registry_delta.toml"
FORMAT = "rehoboam/annals/v0"
D_ANNALS = b"rehoboam/annals/v0"
EXIT_OK, EXIT_STALE, EXIT_SELFTEST = 0, 1, 2

MINUTES_PER_DAY = 1440
MONTHS = {360: (30,) * 12, 365: (31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)}
ORDINAL_WORDS = (
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "sixth",
    "seventh",
    "eighth",
    "ninth",
    "tenth",
    "eleventh",
    "twelfth",
)
TEENS = {11, 12, 13}
ACTOR, TARGET, WITNESS, INSTRUMENT = 0, 1, 2, 3
ROLE_CODES = {"actor": ACTOR, "target": TARGET, "instrument": INSTRUMENT}
PERSON = 1
EDGE_CODES = {"contingent": 0, "triggered": 1, "motivated_by": 3}
ONE = 256
Q8_8_DIGITS = 8
# The groups the fixtures render; the golden file of each is golden/<world>_<group>.txt.
FIXTURE_GROUPS = {
    "ashcombe": ("house:9001", "house:9002", "year", "line:1003"),
    "solitary": ("year",),
}
# Modules the renderer may import (V-NAR-07): the standard library and the two sibling references.
ALLOWED_IMPORTS = frozenset(
    {
        "argparse",
        "importlib",
        "sys",
        "tomllib",
        "collections",
        "dataclasses",
        "decimal",
        "pathlib",
        "types",
        "string",
        "re",
        "random",
        "itertools",
        "struct",
        "hashlib",
        "typing",
        "__future__",
    }
)
SIBLINGS = frozenset({"canonical_v0", "lint_templates", "gen_hash_golden"})


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


# --- the calendar --------------------------------------------------------------------------------


@dataclass(frozen=True)
class Calendar:
    """The world calendar (05.1 §4.1): 360 or 365 days, 12 months, no leap years."""

    days_per_year: int
    epoch_label: str

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


def ordinal(n: int) -> str:
    """``first`` to ``twelfth`` as words, then ``13th``, ``21st``, ... (n >= 1)."""
    if 1 <= n <= len(ORDINAL_WORDS):
        return ORDINAL_WORDS[n - 1]
    suffix = "th" if n % 100 in TEENS else {1: "st", 2: "nd", 3: "rd"}.get(n % 10, "th")
    return f"{n}{suffix}"


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
        """The earliest told cause of edge kind ``kind``: ``(cause_id, motive)``."""
        for cause, code, motive, _, _, told in self.causes:
            if code == kind and told:
                return cause, motive
        return None

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
    sex: int
    mother: int
    father: int
    birth_tick: int
    exit_tick: int | None
    source: str  # ``life/<pid>`` (channel A) or ``kf:life/<pid>`` (the keyframe extract)


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

    events: dict[int, Event] = field(default_factory=dict)
    life: dict[int, LifeRow] = field(default_factory=dict)
    births: dict[int, Event] = field(default_factory=dict)  # child -> BIRTH event
    records: list[Any] = field(default_factory=list)  # the canonical Records, in file order


@dataclass
class World:
    """The world file, the names file and the keyframe extract of a fixture directory."""

    id: str
    calendar: Calendar
    persons: dict[int, str] = field(default_factory=dict)
    houses: dict[int, str] = field(default_factory=dict)
    titles: dict[int, str] = field(default_factory=dict)
    kf_life: dict[int, LifeRow] = field(default_factory=dict)
    members: dict[int, int] = field(default_factory=dict)  # person -> house
    tenures: list[Tenure] = field(default_factory=list)


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


def load_chronicle(path: Path) -> Chronicle:
    """Parse a canonical channel-A file (checked whole by the canonical reference first)."""
    _canon.digest_file(path)
    chron = Chronicle()
    for raw in path.read_text(encoding="ascii").splitlines()[1:]:
        if raw.startswith("#"):
            continue
        rec = _canon.parse(raw)
        chron.records.append(rec)
        if rec.kind == "EVENT":
            ev = _event_of(rec)
            chron.events[ev.id] = ev
            if ev.kind == "BIRTH" and ev.person(TARGET) is not None:
                chron.births[ev.person(TARGET) or 0] = ev
        elif rec.kind == "LIFE_ENTRY":
            pid = int(rec["person_id"])
            chron.life[pid] = LifeRow(
                pid,
                int(rec["sex"]),
                int(rec["mother_id"]),
                int(rec["father_id"]),
                int(rec["birth_tick"]),
                None,
                f"life/{pid}",
            )
    return chron


def _int_map(table: object) -> dict[int, str]:
    return {int(k): str(v) for k, v in table.items()} if isinstance(table, dict) else {}


def _keyframe(w: World, path: Path) -> None:
    kf = tomllib.loads(path.read_text(encoding="utf-8"))
    for row in kf.get("life_course", []):
        pid = int(row["person_id"])
        exit_tick = row.get("exit_tick")
        w.kf_life[pid] = LifeRow(
            pid,
            int(row["sex"]),
            int(row["mother_id"]),
            int(row["father_id"]),
            int(row["birth_tick"]),
            None if exit_tick is None else int(exit_tick),
            f"kf:life/{pid}",
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


def load_world(directory: Path) -> World:
    """The fixture directory: ``world.toml``, ``names.toml`` and, if present, ``keyframe.toml``."""
    meta = tomllib.loads((directory / "world.toml").read_text(encoding="utf-8"))["world"]
    w = World(str(meta["id"]), Calendar(int(meta["days_per_year"]), str(meta["epoch_label"])))
    names_path = directory / "names.toml"
    if names_path.exists():
        names = tomllib.loads(names_path.read_text(encoding="utf-8"))
        w.persons, w.houses, w.titles = (
            _int_map(names.get("persons")),
            _int_map(names.get("houses")),
            _int_map(names.get("titles")),
        )
    kf_path = directory / "keyframe.toml"
    if kf_path.exists():
        _keyframe(w, kf_path)
    return w


def attribute_index(registry: dict[str, object]) -> dict[int, tuple[str, str, list[str]]]:
    """``attr_id`` -> (name, type, enum values) from the Registry, sorted by name (R6)."""
    attrs = registry.get("attributes", {})
    out: dict[int, tuple[str, str, list[str]]] = {}
    if isinstance(attrs, dict):
        for i, name in enumerate(sorted(attrs)):
            entry = attrs[name] if isinstance(attrs[name], dict) else {}
            out[i] = (name, str(entry.get("type", "")), [str(v) for v in entry.get("values", [])])
    return out


# --- rendering -----------------------------------------------------------------------------------


@dataclass
class Sentence:
    """One rendered line with its provenance (one sidecar row)."""

    text: str
    template: str
    version: int
    events: set[int] = field(default_factory=set)
    persons: set[int] = field(default_factory=set)
    sources: set[str] = field(default_factory=set)
    prologue: bool = False

    def row(self, line: int) -> str:
        """The sidecar row: ``S|P line template version [events] [persons] [sources]``."""
        tag = "P" if self.prologue else "S"
        ev = ",".join(str(e) for e in sorted(self.events))
        ps = ",".join(str(p) for p in sorted(self.persons))
        src = ",".join(sorted(self.sources))
        return f"{tag} {line} {self.template} {self.version} [{ev}] [{ps}] [{src}]"


class Renderer:
    """Renders one group of one chronicle: a pure function of its inputs (annals_v0.md §8)."""

    def __init__(
        self,
        templates: Any,  # noqa: ANN401  (a lint_templates.Templates, loaded by path)
        world: World,
        chron: Chronicle,
        attrs: dict[int, tuple[str, str, list[str]]],
    ) -> None:
        """Bind the template set, the world (names, calendar), the chronicle, the attributes."""
        self.t = templates
        self.w = world
        self.c = chron
        self.attrs = attrs
        self.attr_ids = {name: i for i, (name, _, _) in attrs.items()}
        self.used: Counter[str] = Counter()

    # -- designation (annals_v0.md §4)

    def life_row(self, pid: int) -> LifeRow | None:
        """The person's life-course row: channel A's ``LIFE_ENTRY`` first, else the keyframe's."""
        return self.c.life.get(pid) or self.w.kf_life.get(pid)

    def designate(self, pid: int, s: Sentence) -> str:
        """A name, else the mother's Nth child, else a child of a named parent, else the id."""
        s.persons.add(pid)
        name = self.w.persons.get(pid)
        if name is not None:
            s.sources.add(f"name/{pid}")
            return name
        birth = self.c.births.get(pid)
        if birth is not None and (mother := birth.person(ACTOR)) in self.w.persons:
            s.sources.update({f"birth/{birth.id}", f"name/{mother}"})
            s.persons.add(mother or 0)
            return f"the {ordinal(birth.payload[1])} child of {self.w.persons[mother or 0]}"
        row = self.life_row(pid)
        if row is not None:
            for parent in (row.father, row.mother):
                if parent in self.w.persons:
                    s.sources.update({row.source, f"name/{parent}"})
                    s.persons.add(parent)
                    return f"a {'daughter' if row.sex else 'son'} of {self.w.persons[parent]}"
        house = self.house_of(pid, s.sources)
        if house in self.w.houses:
            s.sources.add(f"house/{house}")
            return f"person #{pid} of {self.w.houses[house or 0]}"
        return f"person #{pid}"

    def house_of(self, pid: int, sources: set[str]) -> int | None:
        """The person's house: the keyframe's row, else the father's, else the mother's (M7)."""
        if pid in self.w.members:
            sources.add(f"kf:member/{pid}")
            return self.w.members[pid]
        row = self.c.life.get(pid)
        if row is None:
            return None
        sources.add(row.source)
        for parent in (row.father, row.mother):
            house = self.house_of(parent, sources) if parent else None
            if house is not None:
                return house
        return None

    # -- slots (annals_v0.md §3)

    def _role_slot(self, role: str, ev: Event, s: Sentence) -> str:
        if role == "witnesses":
            return (
                ", ".join(
                    self.designate(p, s) for k, p, c in ev.roles if c == WITNESS and k == PERSON
                )
                or "no one"
            )
        pid = ev.person(ROLE_CODES[role])
        return self.designate(pid, s) if pid is not None else "no one"

    def _payload_slot(self, name: str, ev: Event) -> str:
        layout = self.t.payloads[ev.kind]["slots"]
        for i, slot in enumerate(layout):
            if slot["name"] != name:
                continue
            value = ev.payload[i]
            if "enum" in slot:
                enum = self.t.enums[slot["enum"]]
                return str(enum["words"][value])
            return ordinal(value) if slot.get("ordinal") else str(value)
        raise KeyError(name)

    def _snapshot_slot(self, attr: str, role: str, ev: Event) -> str:
        pid = ev.person(ROLE_CODES[role])
        value = None if pid is None else ev.snapshot(self.attr_ids[attr], pid)
        if value is None:
            return "not recorded"
        _, kind, values = self.attrs[self.attr_ids[attr]]
        if kind == "q8_8":
            return q8_8(value)
        if kind == "enum":
            return values[value]
        return str(value)

    def _cause_slot(self, kind: str, ev: Event, s: Sentence) -> str:
        found = ev.cause(EDGE_CODES[kind])
        if found is None:
            return "an event the chronicle does not tell"
        cause_id, _ = found
        cause = self.c.events.get(cause_id)
        if cause is None:
            s.sources.add(f"untold/{cause_id}")
            return "an event the chronicle does not tell"
        s.events.add(cause_id)
        variant = self.variant(cause)
        return self.fill(str(variant["label"]), cause, s) if variant else f"event {cause_id}"

    def _motive_slot(self, ev: Event) -> str:
        found = ev.cause(EDGE_CODES["motivated_by"])
        motive = None if found is None else found[1]
        words = self.t.enums["motive"]["words"]
        return "for a motive the chronicle does not name" if motive is None else str(words[motive])

    def slot(self, name: str, ev: Event, s: Sentence) -> str:
        """The text of one slot of a chronicle sentence (the lint has resolved it already)."""
        slot = _lint.parse_slot(name)
        if slot is None:
            raise ValueError(f"slot {name!r} {_lint.OUTSIDE}")
        match slot.family:
            case "tick":
                return (
                    day_phrase(ev.tick, self.w.calendar)
                    if slot.args[0] == "date"
                    else year_phrase(self.w.calendar.date(ev.tick)[0], self.w.calendar)
                )
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

    def fill(self, text: str, ev: Event, s: Sentence) -> str:
        """Substitute every slot of ``text`` for event ``ev``, recording provenance on ``s``."""
        return "".join(
            literal + (self.slot(name, ev, s) if name is not None else "")
            for literal, name, _, _ in _lint.string.Formatter().parse(text)
        )

    # -- variants (annals_v0.md §2)

    def holds(self, when: dict[str, Any], ev: Event) -> bool:
        """Whether a variant's ``when`` holds: roles present or absent, a payload enum, an edge."""
        has = all(ev.person(ROLE_CODES[r]) is not None for r in when.get("has", []))
        lacks = all(ev.person(ROLE_CODES[r]) is None for r in when.get("lacks", []))
        edge = "edge" not in when or ev.cause(EDGE_CODES[str(when["edge"])]) is not None
        return has and lacks and edge and self._payload_holds(when, ev)

    def _payload_holds(self, when: dict[str, Any], ev: Event) -> bool:
        payload = when.get("payload", {})
        if not isinstance(payload, dict) or not payload:
            return True
        slots = self.t.payloads[ev.kind]["slots"]
        layout = {str(s["name"]): (i, s) for i, s in enumerate(slots)}
        for name, want in payload.items():
            i, slot = layout[name]
            ids = self.t.enums[slot["enum"]]["ids"]
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

    def house_members(self, house: int) -> set[int]:
        """The keyframe's members of ``house`` and every in-run birth to a member parent."""
        members = {p for p, h in self.w.members.items() if h == house}
        for ev in sorted(self.c.events.values(), key=lambda e: e.id):
            if ev.kind != "BIRTH":
                continue
            child = ev.person(TARGET)
            row = None if child is None else self.c.life.get(child)
            if (
                child is not None
                and row is not None
                and (row.father in members or (row.father == 0 and row.mother in members))
            ):
                members.add(child)
        return members

    def line_members(self, root: int) -> set[int]:
        """``root`` and every descendant through the life-course rows (channel A, then keyframe)."""
        rows = {**self.w.kf_life, **self.c.life}
        members = {root}
        changed = True
        while changed:
            changed = False
            for row in rows.values():
                if row.person not in members and (row.mother in members or row.father in members):
                    members.add(row.person)
                    changed = True
        return members

    def _prologue_fill(self, pid: str, values: dict[str, str], s: Sentence) -> None:
        entry = self.t.prologue[pid]
        s.template, s.version, s.prologue = pid, int(entry["version"]), True
        text = str(entry["text"]).format(**values)
        s.text = text[:1].upper() + text[1:]

    def prologue(self, house: int, members: set[int]) -> list[Sentence]:
        """The house's opening from the keyframe extract: living count, ancestors, tenures."""
        cal = self.w.calendar
        out: list[Sentence] = []
        living = sorted(
            p for p in members if p in self.w.kf_life and self.w.kf_life[p].exit_tick is None
        )
        s = Sentence(
            "",
            "",
            0,
            sources={f"house/{house}", *(f"kf:life/{p}" for p in living)},
            persons=set(living),
        )
        self._prologue_fill(
            "prologue.house",
            {"house": self.w.houses.get(house, f"house #{house}"), "living": str(len(living))},
            s,
        )
        out.append(s)
        for pid in sorted(members):
            row = self.w.kf_life.get(pid)
            if row is None or row.exit_tick is None:
                continue
            s = Sentence("", "", 0, sources={row.source})
            who = self.designate(pid, s)
            self._prologue_fill(
                "prologue.ancestor",
                {
                    "person": who,
                    "birth_date": full_date(row.birth_tick, cal),
                    "death_date": full_date(row.exit_tick, cal),
                },
                s,
            )
            out.append(s)
        for ten in self.tenures_of(members):
            s = Sentence(
                "", "", 0, sources={f"kf:tenure/{ten.title}/{ten.start}", f"title/{ten.title}"}
            )
            values = {
                "holder": self.designate(ten.holder, s),
                "title": self.w.titles.get(ten.title, f"title #{ten.title}"),
                "start_date": full_date(ten.start, cal),
            }
            if ten.end is None:
                self._prologue_fill("prologue.tenure.open", values, s)
            else:
                self._prologue_fill(
                    "prologue.tenure", {**values, "end_date": full_date(ten.end, cal)}, s
                )
            out.append(s)
        return out

    def tenures_of(self, members: set[int]) -> list[Tenure]:
        """The keyframe's tenure rows held by a member, by title then start."""
        return [t for t in self.w.tenures if t.holder in members]


@dataclass
class Annal:
    """A rendered annal: its text lines and its sidecar rows, before the digest."""

    title: str
    group: str
    lines: list[str] = field(default_factory=list)
    rows: list[str] = field(default_factory=list)
    omitted: Counter[str] = field(default_factory=Counter)


def _group(r: Renderer, group: str) -> tuple[str, set[int] | None, list[Sentence]]:
    kind, _, arg = group.partition(":")
    if kind == "year":
        return "The annals of the years", None, []
    if kind == "line":
        root = int(arg)
        s = Sentence("", "", 0)
        return f"The annals of the line of {r.designate(root, s)}", r.line_members(root), []
    house = int(arg)
    members = r.house_members(house)
    return (
        f"The annals of {r.w.houses.get(house, f'house #{house}')}",
        members,
        r.prologue(house, members),
    )


def render(r: Renderer, group: str) -> Annal:
    """Render one group: a house (``house:<id>``), a line (``line:<pid>``) or all (``year``)."""
    title, members, prologue = _group(r, group)
    annal = Annal(title, group)
    scope = set(r.t.scope)
    lines, rows = annal.lines, annal.rows
    lines.append(f"# {title}")
    lines.append(f"# From the chronicle of {r.w.id}; templates {r.t.version}")

    def emit(s: Sentence) -> None:
        lines.append(s.text)
        rows.append(s.row(len(lines)))

    if prologue:
        lines.extend(("", "## Before the chronicle began"))
        for s in prologue:
            emit(s)
    year: int | None = None
    for ev in sorted(r.c.events.values(), key=lambda e: e.id):
        if members is not None and not set(ev.persons()) & members:
            continue
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


def templates_digest(path: Path = TEMPLATES) -> bytes:
    """XXH3-64 of the template file's bytes, little-endian: part of every annals digest."""
    return int(_canon.xxh3_64(path.read_bytes())).to_bytes(8, "little")


def finish(annal: Annal, r: Renderer, tdigest: bytes) -> tuple[str, str]:
    """The text and the sidecar as files: header, rows, the omission trailer and the digest."""
    text = "\n".join(annal.lines) + "\n"
    cal = r.w.calendar
    head = (
        f"#{FORMAT} world={r.w.id} group={annal.group} templates={r.t.version} "
        f"templates_digest={tdigest.hex()} calendar={cal.days_per_year}"
    )
    body = [head, *annal.rows]
    if annal.omitted:
        kinds = ", ".join(f"{k} x{n}" for k, n in sorted(annal.omitted.items()))
        n = sum(annal.omitted.values())
        body.append(f"# omitted: {n} event(s) of kinds outside the template set: {kinds}")
    side = "\n".join(body) + "\n"
    digest = _canon.xxh3_64(
        D_ANNALS + tdigest + text.encode("utf-8") + b"\n" + side.encode("utf-8")
    )
    return text, side + f"digest {int(digest):016x}\n"


def render_group(
    world_dir: Path,
    group: str,
    templates: Any,  # noqa: ANN401
    attrs: dict[int, tuple[str, str, list[str]]],
) -> tuple[str, str, Renderer]:
    """Load a fixture directory and render one group: ``(text, sidecar, renderer)``."""
    r = Renderer(
        templates, load_world(world_dir), load_chronicle(world_dir / "chronicle_A.txt"), attrs
    )
    annal = render(r, group)
    text, side = finish(annal, r, templates_digest(templates.path))
    return text, side, r


# --- the self-test -------------------------------------------------------------------------------


@dataclass(frozen=True)
class Control:
    """The positive controls (each must FAIL its check)."""

    no_event_id: bool = False
    undeclared_slot: bool = False
    state_read: bool = False


def _plant_templates(t: Any, control: Control) -> None:  # noqa: ANN401
    entries = t.by_kind["DEATH"]
    natural = next(e for e in entries if e["id"] == "death.natural")
    if control.undeclared_slot:
        natural["text"] = str(natural["text"]) + " The instrument was {instrument}."
    if control.state_read:
        natural["text"] = str(natural["text"]) + " The title passed to {state.title.holder}."


def check_provenance(text: str, side: str, chron: Chronicle, scope: set[str]) -> list[str]:
    """V-NAR-05: one row per sentence; chronicle rows cite events, prologue rows keyframe rows."""
    rows = {int(line.split(" ")[1]): line for line in side.splitlines() if line[:2] in {"S ", "P "}}
    problems: list[str] = []
    for n, line in enumerate(text.splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        row = rows.pop(n, None)
        if row is None:
            problems.append(f"line {n} has no sidecar row: {line!r}")
            continue
        tag, _, _, _, events, _, sources = row.split(" ")
        ids = [int(e) for e in events[1:-1].split(",") if e]
        if tag == "S" and not ids:
            problems.append(f"line {n}: a chronicle sentence cites no event")
        if tag == "P" and "kf:" not in sources:
            problems.append(f"line {n}: a prologue sentence cites no keyframe row")
        for eid in ids:
            if eid not in chron.events:
                problems.append(f"line {n}: event {eid} is not in the chronicle")
            elif chron.events[eid].kind not in scope:
                kind = chron.events[eid].kind
                problems.append(f"line {n}: event {eid} ({kind}) is outside the template scope")
    problems.extend(f"sidecar row for line {n} has no sentence" for n in sorted(rows))
    return problems


def check_calendar() -> list[str]:
    """Known answers of the two calendars, before and after the epoch."""
    c360, c365 = Calendar(360, "Y"), Calendar(365, "Y")
    cases = [
        (c360, 0, (0, 1, 1)),
        (c360, 1440 * 73, (0, 3, 14)),
        (c360, 1440 * 360 * 12 - 1440, (11, 12, 30)),
        (c360, -1440, (-1, 12, 30)),
        (c360, -1440 * 360, (-1, 1, 1)),
        (c365, 1440 * 59, (0, 3, 1)),
        (c365, 1440 * 364, (0, 12, 31)),
        (c365, 1440 * 365 + 1439, (1, 1, 1)),
        (c365, -1, (-1, 12, 31)),
    ]
    out = [
        f"{cal.days_per_year}-day: tick {tick} gave {cal.date(tick)}, not {want}"
        for cal, tick, want in cases
        if cal.date(tick) != want
    ]
    words = [
        (1, "first"),
        (12, "twelfth"),
        (13, "13th"),
        (21, "21st"),
        (22, "22nd"),
        (23, "23rd"),
        (111, "111th"),
        (112, "112th"),
    ]
    out += [
        f"ordinal({n}) = {ordinal(n)!r}, not {want!r}" for n, want in words if ordinal(n) != want
    ]
    if q8_8(16128) != "63.0" or q8_8(64) != "0.25" or q8_8(-384) != "-1.5":
        out.append("q8_8 rendering")
    return out


def check_fixture_registry(
    chron: Chronicle, registry: dict[str, object], attrs: dict[int, tuple[str, str, list[str]]]
) -> list[str]:
    """Every fixture EVENT agrees with the Registry's verb table (with the next M2 delta)."""
    verbs: dict[str, Any] = dict(registry["verbs"])  # type: ignore[call-overload]
    if REGISTRY_DELTA.exists():
        verbs.update(tomllib.loads(REGISTRY_DELTA.read_text(encoding="utf-8")).get("verbs", {}))
    names = sorted(verbs)
    acts: list[str] = registry["act_classes"]["ids"]  # type: ignore[index]
    out: list[str] = []
    for rec in chron.records:
        if rec.kind != "EVENT":
            continue
        verb = str(rec["verb"])
        if verb not in verbs:
            out.append(f"event {rec['event_id']}: verb {verb} is not in the Registry")
            continue
        bits = sum(1 << acts.index(a) for a in verbs[verb]["act_classes"])
        want = (names.index(verb), bits, verbs[verb]["tellability"])
        got = (rec["verb_id"], rec["act_classes"], rec["tellability"])
        if got != want:
            out.append(
                f"event {rec['event_id']} ({verb}): (verb_id, act_classes, tellability) {got} "
                f"differs from the Registry's {want}"
            )
        for attr_id, *_ in rec["snapshots"]:
            if attr_id not in attrs:
                out.append(f"event {rec['event_id']}: snapshot attribute {attr_id} is unknown")
    return out


def _golden_paths(golden: Path, world: str, group: str) -> tuple[Path, Path]:
    stem = f"{world}_{group.replace(':', '-')}"
    return golden / f"{stem}.txt", golden / f"{stem}.sidecar.txt"


@dataclass
class Run:
    """What one ``--check`` run accumulates."""

    results: list[tuple[str, list[str]]] = field(default_factory=list)
    stale: list[str] = field(default_factory=list)
    rendered: dict[tuple[str, str], tuple[str, str]] = field(default_factory=dict)
    used: Counter[str] = field(default_factory=Counter)

    def add(self, name: str, problems: list[str]) -> None:
        """Record a check's outcome."""
        self.results.append((name, problems))


def _render_all(
    run: Run,
    t: Any,  # noqa: ANN401
    attrs: dict[int, tuple[str, str, list[str]]],
    control: Control,
    golden: Path,
) -> tuple[list[str], list[str]]:
    provenance: list[str] = []
    determinism: list[str] = []
    for world, groups in FIXTURE_GROUPS.items():
        for group in groups:
            text, side, r = render_group(FIXTURES / world, group, t, attrs)
            if control.no_event_id and group == "year":
                head, _, tail = text.partition("\n## ")
                text = head + "\nThus the chronicle began, and the land was quiet.\n## " + tail
            provenance += [
                f"{world} {group}: {p}" for p in check_provenance(text, side, r.c, set(t.scope))
            ]
            again = render_group(FIXTURES / world, group, t, attrs)
            if (again[0], again[1]) != (text, side):
                determinism.append(f"{world} {group}: two renders differ")
            run.used.update(r.used)
            run.rendered[(world, group)] = (text, side)
            for path, want in zip(_golden_paths(golden, world, group), (text, side), strict=True):
                if not path.exists() or path.read_bytes() != want.encode("utf-8"):
                    run.stale.append(str(path))
    return provenance, determinism


def check_imports() -> list[str]:
    """V-NAR-07: this file imports only the standard library and the sibling references."""
    out: list[str] = []
    for line in Path(__file__).read_text(encoding="utf-8").splitlines():
        words = line.split()
        if len(words) > 1 and words[0] in {"import", "from"} and not line.startswith(" "):
            module = words[1].split(".")[0]
            if module not in ALLOWED_IMPORTS and module not in SIBLINGS and module != "__future__":
                out.append(f"import of {module!r}")
    loaded = {m for m in sys.modules if m in SIBLINGS}
    return out + [
        f"sibling {m} not loaded" for m in ("canonical_v0", "lint_templates") if m not in loaded
    ]


def self_test(control: Control, golden: Path) -> Run:
    """Run every check; the controls plant their defects first."""
    run = Run()
    t = _lint.load_templates(TEMPLATES)
    _plant_templates(t, control)
    registry = _lint.load_registry(REGISTRY)
    attrs = attribute_index(registry)
    lint_problems = _lint.check(t, _lint.load_emits(), registry)
    run.add("2. template slots (annals_v0.md §3)", lint_problems)
    run.add("3. no model (V-NAR-07)", check_imports())
    run.add("4. the calendar (05.1 §4.1)", check_calendar())
    if lint_problems:
        return run
    provenance, determinism = _render_all(run, t, attrs, control, golden)
    run.results.insert(0, ("1. provenance (V-NAR-05)", provenance))
    fixture_problems = [
        p
        for world in FIXTURE_GROUPS
        for p in check_fixture_registry(
            load_chronicle(FIXTURES / world / "chronicle_A.txt"), registry, attrs
        )
    ]
    run.add("5. fixtures agree with the Registry", fixture_problems)
    run.add("6. determinism (byte-equal renders)", determinism)
    kinds_used = {
        str(e["kind"]) for entries in t.by_kind.values() for e in entries if e["id"] in run.used
    }
    run.add(
        "7. coverage: every in-scope kind rendered",
        [f"kind {k} is not exercised by any fixture" for k in t.scope if k not in kinds_used],
    )
    return run


def write_golden(golden: Path) -> None:
    """Regenerate every golden annal and sidecar from the fixtures (never by hand)."""
    t = _lint.load_templates(TEMPLATES)
    attrs = attribute_index(_lint.load_registry(REGISTRY))
    golden.mkdir(parents=True, exist_ok=True)
    for world, groups in FIXTURE_GROUPS.items():
        for group in groups:
            text, side, _ = render_group(FIXTURES / world, group, t, attrs)
            for path, data in zip(_golden_paths(golden, world, group), (text, side), strict=True):
                path.write_bytes(data.encode("utf-8"))
                print(f"wrote {path}")


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
    if control != Control():
        print("ANNALS v0: the control did not fail its check")
        return EXIT_SELFTEST
    for path in run.stale:
        print(f"STALE: {path} differs from the render (regenerate with --write)")
    if run.stale:
        return EXIT_STALE
    print(f"8. golden annals: PASS ({sum(len(g) for g in FIXTURE_GROUPS.values())} groups)")
    print("ANNALS v0: PASS")
    return EXIT_OK


def main(argv: list[str] | None = None) -> int:
    """Command line: ``--check``, ``--write`` or ``render``."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--golden-dir", type=Path, default=GOLDEN)
    parser.add_argument("--control", choices=["no-event-id", "undeclared-slot", "state-read"])
    sub = parser.add_subparsers(dest="command")
    rp = sub.add_parser("render", help="render one group of a fixture directory to stdout")
    rp.add_argument("--world", type=Path, required=True)
    rp.add_argument("--group", default="year")
    rp.add_argument("--sidecar", action="store_true", help="print the sidecar instead of the text")
    args = parser.parse_args(argv)
    if args.command == "render":
        t = _lint.load_templates(TEMPLATES)
        text, side, _ = render_group(
            args.world, args.group, t, attribute_index(_lint.load_registry(REGISTRY))
        )
        sys.stdout.write(side if args.sidecar else text)
        return EXIT_OK
    if args.write:
        write_golden(args.golden_dir)
        return EXIT_OK
    control = Control(
        args.control == "no-event-id",
        args.control == "undeclared-slot",
        args.control == "state-read",
    )
    run = self_test(control, args.golden_dir)
    t = _lint.load_templates(TEMPLATES)
    return _report(run, t, control)


if __name__ == "__main__":
    sys.exit(main())
