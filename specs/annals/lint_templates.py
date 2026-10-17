"""Lint of the family-annals template set (SPR-029; specs/annals/annals_v0.md §3, §7).

Every slot of every template resolves to something the event it renders carries, as the emitting
system declares it in its ``[[emits]]`` entry (``specs/systems/*.odd.toml``, a ``next/`` version
superseding its frozen one, as ``lint_odd.py --next`` does): a role the kind fills, a snapshot it
declares (attribute and role), a payload slot whose layout quotes the entry's ``when`` text
verbatim, a motive only where the kind declares motives, a cause edge, or the tick. A slot outside
that vocabulary is refused by name, so a template can never read world state or invent a fact.
Every kind in scope has a template and a final variant without a condition, so every admitted event
of an in-scope kind renders exactly one sentence. The enums a slot renders through agree with the
Registry (``[motive_edges]``), and a prologue template uses only the keyframe-extract vocabulary.

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
WHEN_KEYS = frozenset({"has", "lacks", "payload", "edge"})
PROLOGUE_SLOTS = frozenset(
    {
        "house",
        "living",
        "person",
        "birth_date",
        "death_date",
        "holder",
        "title",
        "start_date",
        "end_date",
    }
)
PROLOGUE_IDS = ("prologue.house", "prologue.ancestor", "prologue.tenure", "prologue.tenure.open")
SLOT_RE = re.compile(r"[a-z_]+(?:\.[a-zA-Z_]+)*")
WITNESS = "witness"


@dataclass(frozen=True)
class Slot:
    """A parsed slot: its family and its arguments (annals_v0.md §3).

    Families: ``tick`` (date, year), ``role`` (actor, target, instrument, witnesses), ``payload``
    (one declared payload slot), ``snapshot`` (attribute, role), ``motive``, ``cause`` (edge kind).
    """

    family: str
    args: tuple[str, ...] = ()

    @property
    def text(self) -> str:
        """The slot as written between braces."""
        return (
            ".".join((self.family, *self.args))
            if self.family in {"payload", "snapshot", "cause"}
            else (self.args[0] if self.args else self.family)
        )


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
class Templates:
    """The template file, loaded (``load_templates``)."""

    path: Path
    raw: dict[str, object]
    version: str
    scope: tuple[str, ...]
    enums: dict[str, dict[str, list[str]]]
    payloads: dict[str, dict[str, object]]
    by_kind: dict[str, list[dict[str, object]]] = field(default_factory=dict)
    prologue: dict[str, dict[str, object]] = field(default_factory=dict)


def slot_fields(text: str) -> list[str]:
    """The slot names of a template text, in order, through the stdlib formatter."""
    return [name for _, name, _, _ in string.Formatter().parse(text) if name is not None]


def parse_slot(name: str) -> Slot | None:
    """Parse a slot name into a :class:`Slot`, or ``None`` when it is outside the grammar."""
    if not SLOT_RE.fullmatch(name):
        return None
    head, *rest = name.split(".")
    if not rest:
        family = "tick" if head in TICK_SLOTS else "motive" if head == "motive" else "role"
        return Slot(family, () if family == "motive" else (head,))
    arity = {"payload": 1, "snapshot": 2, "cause": 1}.get(head)
    if arity != len(rest) or (head == "cause" and rest[0] not in EDGE_KINDS):
        return None
    return Slot(head, tuple(rest))


def _str_list(value: object) -> tuple[str, ...]:
    if not isinstance(value, list) or not all(isinstance(v, str) for v in value):
        return ()
    return tuple(value)


def _emits_of(path: Path) -> list[Emit]:
    spec = tomllib.loads(path.read_text(encoding="utf-8"))
    system = spec.get("system", {})
    sid = system.get("id") if isinstance(system, dict) else None
    if not isinstance(sid, str):
        return []
    out: list[Emit] = []
    for entry in spec.get("emits", []):
        if not isinstance(entry, dict) or not isinstance(entry.get("kind"), str):
            continue
        snaps = tuple(
            (str(s.get("attr", "")), str(s.get("of", "")))
            for s in entry.get("snapshots", [])
            if isinstance(s, dict)
        )
        out.append(
            Emit(
                sid,
                str(entry["kind"]),
                str(entry.get("when", "")),
                _str_list(entry.get("roles")),
                _str_list(entry.get("motives")),
                snaps,
            )
        )
    return out


def load_emits(specs: Path = SPECS, next_dir: Path | None = NEXT) -> dict[str, Emit]:
    """Every ``[[emits]]`` entry by kind; a ``next/`` spec supersedes the frozen spec of its id."""
    by_system: dict[str, list[Emit]] = {}
    for path in sorted(p for p in specs.glob("*.odd.toml") if not p.name.startswith("_")):
        for emit in _emits_of(path):
            by_system.setdefault(emit.system, []).append(emit)
    if next_dir is not None and next_dir.is_dir():
        for path in sorted(next_dir.glob("*.odd.toml")):
            emits = _emits_of(path)
            if emits:
                by_system[emits[0].system] = emits
    return {emit.kind: emit for emits in by_system.values() for emit in emits}


def load_registry(path: Path = REGISTRY) -> dict[str, object]:
    """The Registry TOML (``[event_roles]``, ``[motive_edges]``, ``[attributes]`` are read)."""
    return tomllib.loads(path.read_text(encoding="utf-8"))


def load_templates(path: Path = TEMPLATES) -> Templates:
    """Load the template file; shape errors are reported by :func:`check`, not raised here."""
    raw = tomllib.loads(path.read_text(encoding="utf-8"))
    meta = raw.get("templates", {})
    meta = meta if isinstance(meta, dict) else {}
    enums_raw = raw.get("enums", {})
    enums: dict[str, dict[str, list[str]]] = {}
    if isinstance(enums_raw, dict):
        for name, table in enums_raw.items():
            if isinstance(table, dict):
                enums[str(name)] = {k: list(v) for k, v in table.items() if isinstance(v, list)}
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
    for entry in raw.get("prologue", []):
        if isinstance(entry, dict):
            t.prologue[str(entry.get("id", ""))] = entry
    return t


# --- checks --------------------------------------------------------------------------------------

OUTSIDE = (
    "is outside the envelope: "
    "a template binds only to roles, snapshots, payload, edges and the tick"
)


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
        elif "when" in entries[-1]:
            errs.append(
                f"kind {kind}: its last template ({entries[-1].get('id')}) has a `when`; "
                "the last variant must be unconditional"
            )


def _check_enums(t: Templates, reg: dict[str, object], errs: list[str]) -> None:
    for name, table in t.enums.items():
        ids, words = table.get("ids", []), table.get("words", [])
        if len(ids) != len(words) or not ids:
            errs.append(f"[enums.{name}] ids and words must be non-empty lists of equal length")
    motives = reg.get("motive_edges", {})
    want = motives.get("ids", []) if isinstance(motives, dict) else []
    if t.enums.get("motive", {}).get("ids") != want:
        errs.append("[enums.motive] ids must equal the Registry [motive_edges] in list order")


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
        if not isinstance(cited, str) or cited not in emit.when:
            errs.append(
                f"{where}: cited text {cited!r} does not occur verbatim in {emit.system}'s "
                f"[[emits]] {kind} when"
            )
        slots = layout.get("slots")
        if not isinstance(slots, list) or not slots:
            errs.append(f"{where}: slots must be a non-empty list")
            continue
        for s in slots:
            if not isinstance(s, dict) or not isinstance(s.get("name"), str):
                errs.append(f"{where}: a slot needs a name")
            elif "enum" in s and s["enum"] not in t.enums:
                errs.append(f"{where}: slot {s['name']} names an unknown enum {s['enum']!r}")


def _payload_slots(t: Templates, kind: str) -> dict[str, dict[str, object]]:
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
    out: list[str] = []
    try:
        names = slot_fields(text)
    except ValueError as e:
        return [f"unbalanced braces: {e}"]
    for name in names:
        slot = parse_slot(name)
        if slot is None:
            out.append(f"slot {name!r} {OUTSIDE}")
            continue
        problem = _slot_problem(slot, kind, emit, t, label=label)
        if problem:
            out.append(problem)
    return out


def _check_when(when: object, kind: str, emit: Emit, t: Templates) -> list[str]:
    if not isinstance(when, dict):
        return ["`when` must be a table"]
    out = [
        f"`when` key {k!r} is not one of {sorted(WHEN_KEYS)}" for k in when if k not in WHEN_KEYS
    ]
    for key in ("has", "lacks"):
        for role in _str_list(when.get(key, [])):
            if role not in emit.roles:
                out.append(f"`when.{key}` names {role!r}, not a role {kind} declares")
    payload = when.get("payload", {})
    if isinstance(payload, dict):
        slots = _payload_slots(t, kind)
        for name, value in payload.items():
            enum = t.enums.get(str(slots.get(name, {}).get("enum", "")), {})
            if name not in slots or value not in enum.get("ids", []):
                out.append(f"`when.payload.{name}` = {value!r} is not an id of the slot's enum")
    if "edge" in when and when["edge"] not in EDGE_KINDS:
        out.append(f"`when.edge` {when['edge']!r} is not one of {CAUSE_KINDS}")
    return out


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


def _check_prologue(t: Templates, errs: list[str]) -> None:
    for pid in PROLOGUE_IDS:
        if pid not in t.prologue:
            errs.append(f"prologue template {pid} is missing")
    for pid, entry in t.prologue.items():
        text = entry.get("text")
        names = slot_fields(text) if isinstance(text, str) else []
        for name in names:
            if name not in PROLOGUE_SLOTS:
                errs.append(
                    f"prologue template {pid}: slot {name!r} is not in the keyframe-extract "
                    f"vocabulary {sorted(PROLOGUE_SLOTS)}"
                )


def check(t: Templates, emits: dict[str, Emit], reg: dict[str, object]) -> list[str]:
    """Every problem of the template set, in a stable order (empty when it passes)."""
    errs: list[str] = []
    _check_meta(t, errs)
    _check_enums(t, reg, errs)
    _check_payloads(t, emits, errs)
    _check_templates(t, emits, errs)
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
    errs = check(t, load_emits(args.specs, args.next_dir), load_registry(args.registry))
    for e in errs:
        print(f"ERROR {args.templates.name}: {e}")
    if errs:
        print(f"FAIL: {len(errs)} problem(s) in {args.templates}")
        return 1
    n = sum(len(v) for v in t.by_kind.values())
    print(
        f"PASS: {n} templates linted, {len(t.scope)} kinds in scope, "
        f"{len(t.prologue)} prologue templates"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
