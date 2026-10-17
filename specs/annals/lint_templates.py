"""Lint of the family-annals template set (SPR-029; specs/annals/annals_v0.md §3, §7).

Every slot of every template resolves to something the event it renders carries, as the emitting
system declares it in its ``[[emits]]`` entry (``specs/systems/*.odd.toml``, a ``next/`` version
superseding its frozen one, as ``lint_odd.py --next`` does): a role the kind fills, a snapshot it
declares (attribute and role), a payload slot whose layout quotes the entry's whole ``Payload:
[...]`` clause verbatim with one slot per item, a motive only where the kind declares motives, a
cause edge, the tick, or ``father`` on ``BIRTH`` (the child's ``LIFE_ENTRY``). A slot outside that
vocabulary, a conversion or a format spec is refused by name, so a template can never read world
state or invent a fact. Every kind in scope has a template whose last variant alone is
unconditional, every out-of-scope kind an emitter declares has a ``[[label]]`` (so a cause slot
always renders), each enum's ids equal the emitter's column codes or the Registry list, and a
prologue template uses only the keyframe-extract vocabulary.

Run: python -I specs/annals/lint_templates.py [--templates FILE] [--specs DIR] [--next DIR]
     [--registry FILE]

Exit 0 with ``PASS: ...``; exit 1 with ``ERROR ...`` lines. The renderer
(``selftest_annals_v0.py``) imports :func:`check` and the slot grammar, so a planted template fails
the self-test with the same diagnostic the gate prints.
"""

from __future__ import annotations

import argparse
import re
import string
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
TEMPLATES = HERE / "templates_v0.toml"
SPECS = ROOT / "specs" / "systems"
NEXT = SPECS / "next"
REGISTRY = ROOT / "specs" / "manifest" / "registry_v0.toml"
FORMAT = "rehoboam/annals/templates/v0"

TICK_SLOTS = frozenset({"date", "year"})
CAUSE_KINDS = ("triggered", "contingent", "motivated_by")
EDGE_KINDS = frozenset(CAUSE_KINDS)
WHEN_KEYS = frozenset({"has", "lacks", "payload", "edge", "motive"})
MOTIVE_CONDITIONS = frozenset({"named", "unnamed"})
# Each prologue template has its own vocabulary of keyframe-extract slots (REV-SPR-029 R27).
PROLOGUE_SLOTS: dict[str, frozenset[str]] = {
    "prologue.house": frozenset({"house", "living"}),
    "prologue.ancestor": frozenset({"person", "birth_date", "death_date"}),
    "prologue.tenure": frozenset({"holder", "title", "start_date", "end_date"}),
    "prologue.tenure.open": frozenset({"holder", "title", "start_date"}),
}
PROLOGUE_IDS = tuple(PROLOGUE_SLOTS)
CONTROL_CHARS = re.compile(r"[\x00-\x1f\x7f]")
SLOT_RE = re.compile(r"[a-z_]+(?:\.[a-zA-Z_]+)*")
PAYLOAD_CLAUSE = re.compile(r"Payload: \[[^\]]*\]")
WITNESS = "witness"
FATHER = "father"
FATHER_KINDS = frozenset(
    {"BIRTH"}
)  # the kinds whose target's LIFE_ENTRY is written in the same instant
ROLE_SLOTS = frozenset({"actor", "target", "instrument", "witnesses", FATHER})
OUTSIDE = (
    "is outside the envelope: "
    "a template binds only to roles, snapshots, payload, edges, the tick and a birth's father"
)


@dataclass(frozen=True)
class Slot:
    """A parsed slot: its family and its arguments (annals_v0.md §3).

    Families: ``tick`` (date, year), ``role`` (actor, target, instrument, witnesses, father),
    ``payload`` (one declared payload slot), ``snapshot`` (attribute, role), ``motive``, ``cause``
    (edge kind).
    """

    family: str
    args: tuple[str, ...] = ()


@dataclass(frozen=True)
class Emit:
    """One ``[[emits]]`` entry of a system spec, as the lint reads it."""

    system: str
    kind: str
    when: str
    roles: tuple[str, ...]
    motives: tuple[str, ...]
    snapshots: tuple[tuple[str, str], ...]  # (attribute, of)


@dataclass
class Specs:
    """What the lint reads from the system specs: emits by kind, and enum column codes."""

    emits: dict[str, Emit] = field(default_factory=dict)
    columns: dict[str, list[str]] = field(default_factory=dict)  # "system:entity.column" -> values
    problems: list[str] = field(default_factory=list)


@dataclass
class Templates:
    """The template file, loaded (``load_templates``)."""

    path: Path
    raw: dict[str, Any]
    version: str
    scope: tuple[str, ...]
    enums: dict[str, dict[str, Any]]
    payloads: dict[str, dict[str, Any]]
    by_kind: dict[str, list[dict[str, Any]]] = field(default_factory=dict)
    labels: dict[str, dict[str, Any]] = field(default_factory=dict)
    prologue: dict[str, dict[str, Any]] = field(default_factory=dict)
    problems: list[str] = field(default_factory=list)  # found while loading (duplicate labels)


def slot_fields(text: str) -> list[tuple[str, str | None, str | None]]:
    """``(name, conversion, format_spec)`` of each slot, through the stdlib formatter."""
    return [(n, c, f) for _, n, f, c in string.Formatter().parse(text) if n is not None]


def parse_slot(name: str) -> Slot | None:
    """Parse a slot name into a :class:`Slot`, or ``None`` when it is outside the grammar."""
    if not SLOT_RE.fullmatch(name):
        return None
    head, *rest = name.split(".")
    if not rest:
        if head in TICK_SLOTS:
            return Slot("tick", (head,))
        if head == "motive":
            return Slot("motive")
        return Slot("role", (head,)) if head in ROLE_SLOTS or head == WITNESS else None
    arity = {"payload": 1, "snapshot": 2, "cause": 1}.get(head)
    if arity != len(rest) or (head == "cause" and rest[0] not in EDGE_KINDS):
        return None
    return Slot(head, tuple(rest))


def _str_list(value: object) -> tuple[str, ...]:
    if not isinstance(value, list) or not all(isinstance(v, str) for v in value):
        return ()
    return tuple(value)


def _read_spec(path: Path, specs: Specs, by_system: dict[str, list[Emit]]) -> None:
    spec = tomllib.loads(path.read_text(encoding="utf-8"))
    system = spec.get("system", {})
    sid = system.get("id") if isinstance(system, dict) else None
    if not isinstance(sid, str):
        return
    emits: list[Emit] = []
    for entry in spec.get("emits", []):
        if not isinstance(entry, dict) or not isinstance(entry.get("kind"), str):
            continue
        snaps = tuple(
            (str(s.get("attr", "")), str(s.get("of", "")))
            for s in entry.get("snapshots", [])
            if isinstance(s, dict)
        )
        emits.append(
            Emit(
                sid,
                str(entry["kind"]),
                str(entry.get("when", "")),
                _str_list(entry.get("roles")),
                _str_list(entry.get("motives")),
                snaps,
            )
        )
    by_system[sid] = emits
    for own in spec.get("owns", []):
        if (
            isinstance(own, dict)
            and own.get("type") == "enum"
            and isinstance(own.get("column"), str)
        ):
            specs.columns[f"{sid}:{own['column']}"] = [str(v) for v in own.get("values", [])]


def load_specs(specs_dir: Path = SPECS, next_dir: Path | None = NEXT) -> Specs:
    """The ``[[emits]]`` entries by kind (``next/`` supersedes frozen) and the enum column codes."""
    specs = Specs()
    by_system: dict[str, list[Emit]] = {}
    files = sorted(p for p in specs_dir.glob("*.odd.toml") if not p.name.startswith("_"))
    if next_dir is not None and next_dir.is_dir():
        files += sorted(p for p in next_dir.glob("*.odd.toml") if not p.name.startswith("_"))
    for path in files:
        _read_spec(path, specs, by_system)
    for sid in sorted(by_system):
        for emit in by_system[sid]:
            other = specs.emits.get(emit.kind)
            if other is not None and other.system != sid:
                specs.problems.append(
                    f"kind {emit.kind} is declared by both {other.system} and {sid}"
                )
            specs.emits[emit.kind] = emit
    return specs


def load_emits(specs_dir: Path = SPECS, next_dir: Path | None = NEXT) -> dict[str, Emit]:
    """The ``[[emits]]`` entries by kind (the renderer's view of :func:`load_specs`)."""
    return load_specs(specs_dir, next_dir).emits


def load_registry(path: Path = REGISTRY) -> dict[str, Any]:
    """The Registry TOML (``[event_roles]``, ``[motive_edges]``, ``[attributes]`` are read)."""
    return tomllib.loads(path.read_text(encoding="utf-8"))


def load_templates(path: Path = TEMPLATES) -> Templates:
    """Load the template file; shape errors are reported by :func:`check`, not raised here."""
    raw = tomllib.loads(path.read_text(encoding="utf-8"))
    meta = raw.get("templates", {})
    meta = meta if isinstance(meta, dict) else {}
    enums_raw = raw.get("enums", {})
    enums = (
        {str(k): v for k, v in enums_raw.items() if isinstance(v, dict)}
        if isinstance(enums_raw, dict)
        else {}
    )
    payloads_raw = raw.get("payloads", {})
    payloads = (
        {str(k): v for k, v in payloads_raw.items() if isinstance(v, dict)}
        if isinstance(payloads_raw, dict)
        else {}
    )
    t = Templates(
        path, raw, str(meta.get("version", "")), _str_list(meta.get("scope")), enums, payloads
    )
    for entry in raw.get("template", []):
        if isinstance(entry, dict):
            t.by_kind.setdefault(str(entry.get("kind", "")), []).append(entry)
    for entry in raw.get("label", []):
        if isinstance(entry, dict):
            kind = str(entry.get("kind", ""))
            if kind in t.labels:
                t.problems.append(f"[[label]] {kind}: duplicate kind")
            t.labels[kind] = entry
    for entry in raw.get("prologue", []):
        if isinstance(entry, dict):
            t.prologue[str(entry.get("id", ""))] = entry
    return t


# --- checks --------------------------------------------------------------------------------------


def _check_meta(t: Templates, errs: list[str]) -> None:
    meta = t.raw.get("templates")
    if not isinstance(meta, dict) or meta.get("format") != FORMAT:
        errs.append(f"[templates] format must be {FORMAT!r}")
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", t.version):
        errs.append(f"[templates] version {t.version!r} is not a semver string")
    if not t.scope:
        errs.append("[templates] scope is empty")
    for kind in t.scope:
        if kind not in t.by_kind:
            errs.append(f"kind {kind} is in scope but has no template")
    for kind, entries in t.by_kind.items():
        if kind not in t.scope:
            errs.append(f"template {entries[0].get('id')}: kind {kind} is not in scope")
        else:
            _check_variant_order(kind, entries, errs)


def _check_variant_order(kind: str, entries: list[dict[str, Any]], errs: list[str]) -> None:
    if "when" in entries[-1]:
        errs.append(
            f"kind {kind}: its last template ({entries[-1].get('id')}) has a `when`; "
            "the last variant must be unconditional"
        )
    for entry in entries[:-1]:
        if "when" not in entry:
            errs.append(
                f"kind {kind}: template {entry.get('id')} is unconditional but not last, "
                "so the later variants can never render"
            )


def _check_enums(t: Templates, reg: dict[str, Any], specs: Specs, errs: list[str]) -> None:
    for name, table in t.enums.items():
        ids, words = table.get("ids", []), table.get("words", [])
        if (
            not isinstance(ids, list)
            or not isinstance(words, list)
            or len(ids) != len(words)
            or not ids
        ):
            errs.append(f"[enums.{name}] ids and words must be non-empty lists of equal length")
            continue
        if "registry" in table:
            want = reg.get(str(table["registry"]), {})
            want = want.get("ids") if isinstance(want, dict) else None
            if ids != want:
                errs.append(
                    f"[enums.{name}] ids must equal the Registry [{table['registry']}] "
                    "in list order"
                )
        elif "column" in table:
            codes = specs.columns.get(str(table["column"]))
            if codes is None:
                errs.append(
                    f"[enums.{name}] column {table['column']!r} is not an enum column "
                    "an emitter owns"
                )
            elif ids != codes:
                errs.append(
                    f"[enums.{name}] ids {ids} must equal the codes of {table['column']}: {codes}"
                )
        else:
            errs.append(
                f"[enums.{name}] names neither a `column` nor a `registry` list to check "
                "its ids against"
            )


def _check_payloads(t: Templates, emits: dict[str, Emit], errs: list[str]) -> None:
    for kind, layout in t.payloads.items():
        emit = emits.get(kind)
        where = f"[payloads.{kind}]"
        if emit is None:
            errs.append(f"{where}: no system declares {kind} in [[emits]]")
            continue
        if layout.get("system") != emit.system:
            errs.append(
                f"{where}: system {layout.get('system')!r} is not the emitter {emit.system}"
            )
        cited = layout.get("cited")
        clause = PAYLOAD_CLAUSE.search(emit.when)
        if clause is None:
            errs.append(
                f"{where}: {emit.system}'s [[emits]] {kind} when states no `Payload: [...]` clause"
            )
            continue
        if cited != clause.group(0):
            errs.append(
                f"{where}: cited text {cited!r} is not the whole clause {clause.group(0)!r} of "
                f"{emit.system}'s [[emits]] {kind} when"
            )
        slots = layout.get("slots")
        items = [s for s in _split_clause(clause.group(0)) if s]
        if not isinstance(slots, list) or len(slots) != len(items):
            errs.append(
                f"{where}: {len(items)} payload item(s) in the clause, but {slots!r} slot(s)"
            )
            continue
        for s in slots:
            if not isinstance(s, dict) or not isinstance(s.get("name"), str):
                errs.append(f"{where}: a slot needs a name")
            elif "enum" in s and s["enum"] not in t.enums:
                errs.append(f"{where}: slot {s['name']} names an unknown enum {s['enum']!r}")


def _split_clause(clause: str) -> list[str]:
    """The comma-separated items of ``Payload: [a, b (c, d)]``, commas inside parentheses kept."""
    inner = clause[len("Payload: [") : -1]
    items, depth, cur = [], 0, ""
    for ch in inner:
        depth += (ch == "(") - (ch == ")")
        if ch == "," and depth == 0:
            items.append(cur.strip())
            cur = ""
        else:
            cur += ch
    items.append(cur.strip())
    return items


def _payload_slots(t: Templates, kind: str) -> dict[str, dict[str, Any]]:
    slots = t.payloads.get(kind, {}).get("slots", [])
    return (
        {str(s["name"]): s for s in slots if isinstance(s, dict) and "name" in s}
        if isinstance(slots, list)
        else {}
    )


def _role_problem(role: str, kind: str, emit: Emit) -> str | None:
    if role == "witnesses":
        return None if WITNESS in emit.roles else f"slot 'witnesses': {kind} does not fill witness"
    if role == WITNESS:
        return "slot 'witness' is undefined (a witness is many: use 'witnesses')"
    if role == FATHER:
        return (
            None
            if kind in FATHER_KINDS
            else f"slot 'father' is defined on {sorted(FATHER_KINDS)} only"
        )
    if role in emit.roles:
        return None
    return f"slot {role!r} is not a role {kind} declares in [[emits]] (roles = {list(emit.roles)})"


def _slot_problem(slot: Slot, kind: str, emit: Emit, t: Templates, *, label: bool) -> str | None:
    problem: str | None = None
    match slot.family:
        case "role":
            problem = _role_problem(slot.args[0], kind, emit)
        case "payload" if slot.args[0] not in _payload_slots(t, kind):
            problem = f"slot 'payload.{slot.args[0]}': {kind} declares no such payload slot"
        case "snapshot" if slot.args[1] in {"event", WITNESS}:
            problem = (
                f"slot 'snapshot.{slot.args[0]}.{slot.args[1]}' is undefined in v0 "
                "(annals_v0.md §3)"
            )
        case "snapshot" if tuple(slot.args) not in emit.snapshots:
            problem = (
                f"slot 'snapshot.{slot.args[0]}.{slot.args[1]}' is not a snapshot {kind} declares "
                f"in [[emits]] (snapshots = {list(emit.snapshots)})"
            )
        case "motive" if not emit.motives:
            problem = f"slot 'motive': {kind} declares no motives"
        case "cause" if label:
            problem = f"slot 'cause.{slot.args[0]}' is not allowed in a label (no recursion)"
    return problem


def _check_text(text: object, kind: str, emit: Emit, t: Templates, *, label: bool) -> list[str]:
    if not isinstance(text, str) or not text:
        return ["text must be a non-empty string"]
    out: list[str] = _control_chars(text)
    try:
        fields = slot_fields(text)
    except ValueError as e:
        return [f"unbalanced braces: {e}"]
    for name, conversion, spec in fields:
        if conversion is not None or spec:
            out.append(
                f"slot {name!r} carries a conversion or format spec, which the grammar refuses"
            )
            continue
        slot = parse_slot(name)
        if slot is None:
            out.append(f"slot {name!r} {OUTSIDE}")
            continue
        problem = _slot_problem(slot, kind, emit, t, label=label)
        if problem:
            out.append(problem)
    return out


def _when_roles(key: str, roles: tuple[str, ...], kind: str, emit: Emit) -> list[str]:
    out: list[str] = []
    for role in roles:
        if role == WITNESS:
            out.append(f"`when.{key}` names 'witness', which is undefined (a witness is many)")
        elif role == FATHER and kind not in FATHER_KINDS:
            out.append(f"`when.{key}` names 'father', defined on {sorted(FATHER_KINDS)} only")
        elif role != FATHER and role not in emit.roles:
            out.append(f"`when.{key}` names {role!r}, not a role {kind} declares")
    return out


def _check_when(when: object, kind: str, emit: Emit, t: Templates) -> list[str]:
    if not isinstance(when, dict):
        return ["`when` must be a table"]
    out = [
        f"`when` key {k!r} is not one of {sorted(WHEN_KEYS)}" for k in when if k not in WHEN_KEYS
    ]
    for key in ("has", "lacks"):
        out += _when_roles(key, _str_list(when.get(key, [])), kind, emit)
    payload = when.get("payload", {})
    if isinstance(payload, dict):
        slots = _payload_slots(t, kind)
        for name, value in payload.items():
            enum = t.enums.get(str(slots.get(name, {}).get("enum", "")), {})
            if name not in slots or value not in enum.get("ids", []):
                out.append(f"`when.payload.{name}` = {value!r} is not an id of the slot's enum")
    if "edge" in when and when["edge"] not in EDGE_KINDS:
        out.append(f"`when.edge` {when['edge']!r} is not one of {CAUSE_KINDS}")
    if "motive" in when and (when["motive"] not in MOTIVE_CONDITIONS or not emit.motives):
        out.append(
            f"`when.motive` {when['motive']!r} must be 'named' or 'unnamed', on a kind with motives"
        )
    return out


def _control_chars(text: str) -> list[str]:
    """A control character in a text or label is refused (R27: annals are plain lines)."""
    m = CONTROL_CHARS.search(text)
    return [] if m is None else [f"text carries a control character U+{ord(m.group()):04X}"]


def _check_templates(t: Templates, emits: dict[str, Emit], errs: list[str]) -> None:
    seen: set[str] = set()
    for kind, entries in t.by_kind.items():
        emit = emits.get(kind)
        if emit is None:
            errs.append(f"kind {kind}: no system declares it in [[emits]]")
            continue
        for entry in entries:
            tid = str(entry.get("id", "?"))
            if tid in seen:
                errs.append(f"template {tid}: duplicate id")
            seen.add(tid)
            if not isinstance(entry.get("version"), int):
                errs.append(f"template {tid}: version must be an integer")
            problems = _check_text(entry.get("text"), kind, emit, t, label=False)
            problems += _check_text(entry.get("label"), kind, emit, t, label=True)
            if "when" in entry:
                problems += _check_when(entry["when"], kind, emit, t)
            errs.extend(f"template {tid}: {p}" for p in problems)


def _check_labels(t: Templates, emits: dict[str, Emit], errs: list[str]) -> None:
    for kind in sorted(emits):
        if kind not in t.scope and kind not in t.labels:
            errs.append(
                f"kind {kind} ({emits[kind].system}) is out of scope and has no [[label]], "
                "so a cause slot could not render it"
            )
    for kind, entry in t.labels.items():
        emit = emits.get(kind)
        if emit is None:
            errs.append(f"[[label]] {kind}: no system declares it in [[emits]]")
            continue
        if kind in t.scope and kind in t.by_kind:  # a kind with no template is reported once
            errs.append(f"[[label]] {kind}: an in-scope kind takes its label from its templates")
        if not isinstance(entry.get("version"), int):
            errs.append(f"[[label]] {kind}: version must be an integer")
        text = entry.get("label")
        errs.extend(f"[[label]] {kind}: {p}" for p in _check_text(text, kind, emit, t, label=True))
        for name, _, _ in slot_fields(text if isinstance(text, str) else ""):
            if (parse_slot(name) or Slot("")).family not in {"role", "tick"}:
                errs.append(
                    f"[[label]] {kind}: slot {name!r} is not a role "
                    "(a label uses the cause's roles only)"
                )


def _check_prologue(t: Templates, errs: list[str]) -> None:
    for pid in PROLOGUE_IDS:
        if pid not in t.prologue:
            errs.append(f"prologue template {pid} is missing")
    for pid, entry in t.prologue.items():
        if pid not in PROLOGUE_SLOTS:
            errs.append(f"prologue template {pid}: not a known prologue id {list(PROLOGUE_IDS)}")
            continue
        if not isinstance(entry.get("version"), int):
            errs.append(f"prologue template {pid}: version must be an integer")
        text = entry.get("text")
        if not isinstance(text, str) or not text:
            errs.append(f"prologue template {pid}: text must be a non-empty string")
            continue
        errs.extend(f"prologue template {pid}: {p}" for p in _control_chars(text))
        for name, conversion, spec in slot_fields(text):
            if conversion is not None or spec:
                errs.append(
                    f"prologue template {pid}: slot {name!r} carries a conversion or format spec"
                )
            elif name not in PROLOGUE_SLOTS[pid]:
                errs.append(
                    f"prologue template {pid}: slot {name!r} is not in its vocabulary "
                    f"{sorted(PROLOGUE_SLOTS[pid])}"
                )


def check(t: Templates, specs: Specs | dict[str, Emit], reg: dict[str, Any]) -> list[str]:
    """Every problem of the template set, in a stable order (empty when it passes)."""
    if isinstance(specs, dict):
        specs = Specs(specs)
    errs: list[str] = list(specs.problems) + list(t.problems)
    _check_meta(t, errs)
    _check_enums(t, reg, specs, errs)
    _check_payloads(t, specs.emits, errs)
    _check_templates(t, specs.emits, errs)
    _check_labels(t, specs.emits, errs)
    _check_prologue(t, errs)
    return errs


def main(argv: list[str] | None = None) -> int:
    """Command line: lint the template set against the system specs and the Registry."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--templates", type=Path, default=TEMPLATES)
    parser.add_argument("--specs", type=Path, default=SPECS)
    parser.add_argument("--next", dest="next_dir", type=Path, default=NEXT)
    parser.add_argument("--registry", type=Path, default=REGISTRY)
    args = parser.parse_args(argv)
    t = load_templates(args.templates)
    errs = check(t, load_specs(args.specs, args.next_dir), load_registry(args.registry))
    for e in errs:
        print(f"ERROR {args.templates.name}: {e}")
    if errs:
        print(f"FAIL: {len(errs)} problem(s) in {args.templates}")
        return 1
    n = sum(len(v) for v in t.by_kind.values())
    print(
        f"PASS: {n} templates linted, {len(t.scope)} kinds in scope, {len(t.labels)} labels, "
        f"{len(t.prologue)} prologue templates"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
