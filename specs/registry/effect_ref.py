"""Reference parser, type checker and evaluator of the Registry effect language v0.

This is the independent oracle for ``specs/registry/effect_language_v0.md``: it reads the golden
cases of ``specs/registry/effect_language_golden_v0.toml`` and reproduces every expected value or
rejection code. The Rust evaluator of SPR-017 (``rh-registry``) must match it bit for bit, as
``bin/gen_fixed_vectors.py`` is the oracle for ``rh-core::fixed``.

Arithmetic is exact (``fractions.Fraction``), rounded once half to even with Python's
``round(Fraction)`` and then saturated, so no rounding code of the engine is reused. The module is
standard-library only (GOV-003 §3) and shares no code with the engine.

Usage:  python -I specs/registry/effect_ref.py            (report every case)
        python -I specs/registry/effect_ref.py --check    (exit 1 on any mismatch)
        python -I specs/registry/effect_ref.py --case ID  (one case, verbose)
        python -I specs/registry/effect_ref.py --golden PATH   (another golden file)

Refs: 05.5 §2.1, §3.2; 05.1 §2, §6; ADR-0006; ADR-0010; SPR-017 (PRJ-002); V-WC-10.
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from collections import deque
from collections.abc import Callable
from dataclasses import dataclass, field
from fractions import Fraction
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parents[2]
GOLDEN = REPO / "specs" / "registry" / "effect_language_golden_v0.toml"
LANGUAGE_VERSION = "0.4.0"

# ----------------------------------------------------------------------------- numeric limits
I64_MIN = -(1 << 63)
I64_MAX = (1 << 63) - 1
FRAC_BITS = {"q8_8": 8, "q16_16": 16}
STORAGE_BITS = {"q8_8": 16, "q16_16": 32}
KIN_DEGREES_MAX = 6
DESCENT_MAX = 6
LINEAGE_HEIGHT_MAX = 3
EXPRESSION_DEPTH_MAX = 64
GENEALOGY = ("life_course.person_id", "life_course.mother", "life_course.father")
AGNATIC_GENEALOGY = ("life_course.person_id", "life_course.father")
LINEAGE_READS = (*GENEALOGY, "life_course.birth_tick", "person.alive")
LINEAGE_BOUND = 4096
TENURES_MAX = 16
MINUTES_PER_DAY = 1440  # rh_core::time::MINUTES_PER_DAY, an engine constant (minor 6)
NESTING_MAX = 2
BOUND_PRODUCT_MAX = 65_536
DRAW_BITS = 64

KEYWORDS = frozenset(
    {"and", "or", "not", "if", "then", "else", "when", "for", "in", "where", "true", "false"}
    | {"none", "div", "mod"}
)
AGGREGATES = frozenset(["count", "any", "all", "sum", "min", "max"])
NUMERIC = frozenset({"int", "q8_8", "q16_16"})
FIXED = frozenset({"q8_8", "q16_16"})
LITERALS = frozenset({"lit_int", "lit_dec", "lit_none"})
CMP_OPS = frozenset({"==", "!=", "<", "<=", ">", ">="})
ORDER_OPS = frozenset({"<", "<=", ">", ">="})
ARITH_OPS = frozenset({"+", "-", "*", "/", "div", "mod"})
DIVISION_OPS = frozenset({"/", "div", "mod"})

# Relation prelude (spec §5.3): name -> (argument kinds, yielded kind, columns read, bound).
RELATIONS: dict[str, tuple[tuple[str, ...], str, tuple[str, ...], int]] = {
    "children": (("person",), "person", GENEALOGY, 64),
    "kin": (("person", "degree"), "person", GENEALOGY, 256),
    "ancestors": (("person", "generations"), "person", GENEALOGY, 126),
    "descendants": (("person", "generations"), "person", GENEALOGY, 256),
    "lineage": (("person", "height"), "person", LINEAGE_READS, LINEAGE_BOUND),
    "line": (("person", "person"), "person", GENEALOGY, DESCENT_MAX),
    "line_agnatic": (("person", "person"), "person", AGNATIC_GENEALOGY, DESCENT_MAX),
    "titles": (("person",), "title", ("title.holder",), 16),
    "vassals": (("title",), "title", ("title.liege",), 64),
    "offices": (("person",), "office", ("office.holder",), 16),
    "court": (
        ("title",),
        "person",
        ("title.liege", "title.holder", "office.title", "office.holder"),
        128,
    ),
    "claims_on": (("title",), "claim", ("claim.title",), 64),
    "claims_of": (("person",), "claim", ("claim.claimant",), 16),
    "vows_of": (("person",), "vow", ("vow.vower",), 8),
    "holders_of": (("title", "tenures"), "person", ("tenure.title", "tenure.holder"), TENURES_MAX),
}
# Genealogical accessors: name -> (result type, columns read).
ACCESSORS: dict[str, tuple[str, tuple[str, ...]]] = {
    "up": ("int", GENEALOGY),
    "common": ("ent:person", GENEALOGY),
    "up_agnatic": ("int", AGNATIC_GENEALOGY),
    "common_agnatic": ("ent:person", AGNATIC_GENEALOGY),
    "descends": ("bool", GENEALOGY),
}
# Degree-like relation arguments: the bound each one is checked against.
DEGREE_BOUNDS = {
    "degree": KIN_DEGREES_MAX,
    "generations": DESCENT_MAX,
    "height": LINEAGE_HEIGHT_MAX,
    "tenures": TENURES_MAX,
}
# Builtins with fixed signatures: name -> (parameter types, result type).
FIXED_BUILTINS: dict[str, tuple[tuple[str, ...], str]] = {
    "widen": (("q8_8",), "q16_16"),
    "narrow": (("q16_16",), "q8_8"),
    "q8": (("int",), "q8_8"),
    "q16": (("int",), "q16_16"),
}
# Builtins whose arguments unify to one numeric type, with that result type.
POLY_BUILTINS: dict[str, int] = {"abs": 1, "min": 2, "max": 2, "clamp": 3, "safe_div": 3}
KNOWN_CALLS = (
    frozenset(FIXED_BUILTINS)
    | frozenset(POLY_BUILTINS)
    | frozenset(ACCESSORS)
    | frozenset({"ordinal", "exists", "floor", "round", "draw", "chance", "related"})
)


class InvariantError(Exception):
    """A hard invariant failure at evaluation (a truncated candidates relation, a repeated draw)."""


class CompileError(Exception):
    """A program rejected at compile time, with its code (spec §10)."""

    def __init__(self, code: str, detail: str = "") -> None:
        """Keep the code and a free-text detail (the detail is not contractual)."""
        super().__init__(f"{code}: {detail}" if detail else code)
        self.code = code


# ----------------------------------------------------------------------------- values
@dataclass(frozen=True, order=True)
class Fx:
    """A fixed-point value: raw bits of the type ``ty`` (``q8_8`` or ``q16_16``)."""

    ty: str
    bits: int


@dataclass(frozen=True, order=True)
class En:
    """An enum value: its type and declaration index."""

    enum: str
    index: int


@dataclass(frozen=True, order=True)
class Et:
    """An entity reference: its kind and id (0 is ``none``)."""

    kind: str
    ident: int


@dataclass(frozen=True, order=True)
class EventRef:
    """An opaque event value (spec §3.1): the verb's own event (``own``) or ``none``."""

    own: bool


@dataclass(frozen=True)
class Cmd:
    """An emitted lifecycle command in queue form: name, person id, encoded args.

    An event-typed argument is the text ``event`` (the verb's own event, which the merge
    rewrites) or ``0`` (``none``); every other argument is its ``i64`` encoding.
    """

    name: str
    person: int
    args: tuple[int | str, ...]


Value = bool | int | Fx | En | Et | EventRef | tuple[Any, ...] | list[Any]


def sat(value: int, lo: int, hi: int) -> int:
    """Saturate an exact integer to ``[lo, hi]``."""
    return max(lo, min(hi, value))


def q_range(ty: str) -> tuple[int, int]:
    """Return the raw-bit range of a fixed type."""
    bits = STORAGE_BITS[ty]
    return -(1 << (bits - 1)), (1 << (bits - 1)) - 1


def fx_from_fraction(ty: str, value: Fraction) -> Fx:
    """Round an exact value half to even to the type's precision, then saturate."""
    lo, hi = q_range(ty)
    return Fx(ty, sat(round(value * (1 << FRAC_BITS[ty])), lo, hi))


def fx_value(x: Fx) -> Fraction:
    """Return the exact value the bits stand for."""
    return Fraction(x.bits, 1 << FRAC_BITS[x.ty])


def literal_bits(ty: str, text: str) -> int:
    """Compile a literal to the bits of ``ty`` (05.5 §3.2); ``E_LITERAL_RANGE`` if too large."""
    exact = Fraction(text)
    if ty == "int":
        bits = int(exact)
        lo, hi = I64_MIN, I64_MAX
    else:
        bits = round(exact * (1 << FRAC_BITS[ty]))
        lo, hi = q_range(ty)
    if not lo <= bits <= hi:
        raise CompileError("E_LITERAL_RANGE", f"{text} as {ty}")
    return bits


def zero_of(ty: str, enum_defaults: dict[str, int] | None = None) -> Value:
    """Return the zero of a scalar type (a missing read, spec §4.5); enums take their default."""
    if ty == "bool":
        return False
    if ty == "int":
        return 0
    if ty in FIXED:
        return Fx(ty, 0)
    if ty.startswith("enum:"):
        return En(ty[5:], (enum_defaults or {}).get(ty[5:], 0))
    return Et(ty[4:], 0)


# ----------------------------------------------------------------------------- arithmetic
def arith_int(op: str, a: int, b: int) -> int:
    """The saturating ``int`` operators of spec §4.3."""
    if op == "+":
        return sat(a + b, I64_MIN, I64_MAX)
    if op == "-":
        return sat(a - b, I64_MIN, I64_MAX)
    if op == "*":
        return sat(a * b, I64_MIN, I64_MAX)
    if op == "/":
        return sat(round(Fraction(a, b)), I64_MIN, I64_MAX)
    remainder = a % abs(b)  # Euclidean: 0 <= remainder < |b|
    if op == "mod":
        return remainder
    return sat((a - remainder) // b, I64_MIN, I64_MAX)


def arith_fx(op: str, a: Fx, b: Fx) -> Fx:
    """The saturating fixed-point operators of spec §4.3 (one rounding, then saturation)."""
    lo, hi = q_range(a.ty)
    if op in ("+", "-"):
        return Fx(a.ty, sat(a.bits + b.bits if op == "+" else a.bits - b.bits, lo, hi))
    if op == "*":
        return fx_from_fraction(a.ty, fx_value(a) * fx_value(b))
    return fx_from_fraction(a.ty, fx_value(a) / fx_value(b))


def arith(op: str, a: Value, b: Value) -> Value:
    """Dispatch a binary arithmetic operator on same-typed operands."""
    if isinstance(a, Fx) and isinstance(b, Fx):
        return arith_fx(op, a, b)
    assert isinstance(a, int) and isinstance(b, int)
    return arith_int(op, a, b)


def negate(a: Value) -> Value:
    """Saturating negation (``-MIN`` is ``MAX``)."""
    if isinstance(a, Fx):
        lo, hi = q_range(a.ty)
        return Fx(a.ty, sat(-a.bits, lo, hi))
    assert isinstance(a, int)
    return sat(-a, I64_MIN, I64_MAX)


def is_zero(a: Value) -> bool:
    """Whether a numeric value is zero."""
    return a.bits == 0 if isinstance(a, Fx) else a == 0


def numeric_key(a: Value) -> int:
    """The integer that orders a numeric value (its bits)."""
    if isinstance(a, Fx):
        return a.bits
    assert isinstance(a, int)
    return a


# ----------------------------------------------------------------------------- environment
@dataclass
class Column:
    """A plane column as the checker sees it (spec §7.4)."""

    entity: str
    ty: str  # a language type: bool, int, q8_8, q16_16, enum:<name>, ent:<kind>
    plane: str
    owner: str
    key: str  # the entity kind a read takes: the entity itself, or `person` for the life-course log


@dataclass
class System:
    """A declaring system: its declared reads, command sends, RNG purposes and parameters."""

    reads: set[str]
    sends: dict[str, str]  # command -> declared target
    rng: set[str]
    params: dict[str, tuple[Fraction, Fraction, str]]  # name -> (lo, hi, type)
    neutral: dict[str, object]  # column -> the coupling's neutral value, raw (NEW-4)


@dataclass
class Command:
    """A command signature from ``[registry.commands]``."""

    owner: str
    params: list[tuple[str, str]]  # (name, language type)


@dataclass
class Registry:
    """The Registry slice of the golden file."""

    enums: dict[str, list[str]]
    enum_defaults: dict[str, int]
    columns: dict[str, Column]
    systems: dict[str, System]
    commands: dict[str, Command]
    registered: set[str]
    programs: dict[str, dict[str, dict[str, str]]]
    keys: dict[str, str]  # entity kind -> the column its rows are keyed by (life_course: person_id)


@dataclass
class World:
    """A fixture snapshot: entity tables, parameters, draws and the calendar."""

    now: int
    days_per_year: int
    tables: dict[str, dict[int, dict[str, Value]]]
    params: dict[str, dict[str, Value]]
    draws: dict[tuple[str, str, int, int], int]
    enum_defaults: dict[str, int] = field(default_factory=dict)
    missing_reads: int = 0
    truncated: int = 0
    dropped_actions: int = 0
    neutral_reads: int = 0
    drawn: set[tuple[str, str, int, int]] = field(default_factory=set)
    children_index: dict[int, list[int]] | None = None

    def children(self, ident: int) -> list[int]:
        """The children-by-parent index over the life-course log (the host's derived index)."""
        if self.children_index is None:
            index: dict[int, list[int]] = {}
            for pid, row in self.tables.get("life_course", {}).items():
                for column in ("mother", "father"):
                    parent = row.get(column)
                    if isinstance(parent, Et) and parent.ident:
                        index.setdefault(parent.ident, []).append(pid)
            self.children_index = {k: sorted(set(v)) for k, v in index.items()}
        return self.children_index.get(ident, [])

    def alive(self, ident: int) -> bool:
        """The ledger's ``person.alive``; a compacted (missing) row is dead. A prelude read."""
        row = self.row("person", ident)
        return row is not None and bool(row.get("alive", False))

    def row(self, kind: str, ident: int) -> dict[str, Value] | None:
        """Return the row of an entity, or ``None`` if it does not exist."""
        return self.tables.get(kind, {}).get(ident)

    def read(self, column: Column, name: str, entity: Et) -> Value:
        """Read a column; a missing entity yields the type's zero and counts a diagnostic."""
        row = self.row(column.entity, entity.ident) if entity.ident else None
        if row is None or name not in row:
            self.missing_reads += 1
            return zero_of(column.ty, self.enum_defaults)
        return row[name]

    def ids(self, kind: str) -> list[int]:
        """Every id of a kind, ascending."""
        return sorted(self.tables.get(kind, {}))


def type_of_param(lo: object, hi: object) -> str:
    """A parameter is ``int`` when both bounds are written as integers, else ``q16_16``."""
    return "int" if isinstance(lo, int) and isinstance(hi, int) else "q16_16"


def parse_type(spec: dict[str, Any]) -> str:
    """Map a golden column declaration to a language type."""
    ty = str(spec["type"])
    if ty == "enum":
        return f"enum:{spec['enum']}"
    if ty == "ref":
        return f"ent:{spec['ref']}"
    if ty not in ("bool", "int", "q8_8", "q16_16", "event"):
        raise ValueError(f"unknown golden type {ty}")
    return ty


def load_registry(raw: dict[str, Any]) -> Registry:
    """Build the Registry slice from the golden file's ``[registry]`` table."""
    systems: dict[str, System] = {}
    for name, s in raw["systems"].items():
        params = {
            p: (Fraction(str(lo)), Fraction(str(hi)), type_of_param(lo, hi))
            for p, (lo, hi) in s.get("params", {}).items()
        }
        systems[name] = System(
            reads=set(s.get("reads", [])),
            sends={c["command"]: c["target"] for c in s.get("sends", [])},
            rng=set(s.get("rng", [])),
            params=params,
            neutral=dict(s.get("neutral", {})),
        )
    columns = {
        name: Column(c["entity"], parse_type(c), c["plane"], c["owner"], c.get("key", c["entity"]))
        for name, c in raw["columns"].items()
    }
    commands = {
        name: Command(c["owner"], [(p["name"], parse_type(p)) for p in c.get("params", [])])
        for name, c in raw["commands"].items()
    }
    enums = {k: list(v["values"]) for k, v in raw["enums"].items()}
    defaults = {k: enums[k].index(str(v["default"])) for k, v in raw["enums"].items()}
    return Registry(
        enums=enums,
        enum_defaults=defaults,
        columns=columns,
        systems=systems,
        commands=commands,
        registered=set(raw["registered"]["systems"]),
        programs=raw.get("programs", {}),
        keys=dict(raw["entities"].get("keys", {})),
    )


def cell_value(reg: Registry, column: Column, raw: object) -> Value:
    """Convert a golden table cell to a value of the column's type."""
    ty = column.ty
    if ty == "bool":
        return bool(raw)
    if ty in FIXED:
        return fx_from_fraction(ty, Fraction(str(raw)))
    if ty.startswith("enum:"):
        return En(ty[5:], reg.enums[ty[5:]].index(str(raw)))
    assert isinstance(raw, int), f"{column.entity} cell {raw!r} is not an integer"
    return raw if ty == "int" else Et(ty[4:], raw)


def fan_rows(fan: dict[str, Any]) -> dict[str, list[dict[str, Any]]]:
    """The synthetic fan tree of a ``[worlds.<name>.fan]`` table (golden header).

    Person 1 is the root; ids are breadth-first, so the father of ``i`` (``i >= 2``) is
    ``(i - 2) div fanout + 1`` and mothers are unknown. Every person is male, born one
    ``generation_days`` before its children (the root at ``-depth`` generations), ascending ids
    within a generation sharing a birth tick. With ``dead_leaves``, the last generation died a
    day ago and is compacted: it has life-course rows and no ledger rows.
    """
    fanout, depth = int(fan["fanout"]), int(fan["depth"])
    generation = int(fan.get("generation_days", 10_800)) * MINUTES_PER_DAY
    dead_leaves = bool(fan.get("dead_leaves", False))
    persons: list[dict[str, Any]] = []
    log: list[dict[str, Any]] = []
    first, size = 1, 1
    for g in range(depth + 1):
        for i in range(first, first + size):
            father = 0 if i == 1 else (i - 2) // fanout + 1
            birth = -(depth - g) * generation
            leaf_dead = dead_leaves and g == depth
            log.append(
                {
                    "id": 100_000 + i,
                    "person_id": i,
                    "mother": 0,
                    "father": father,
                    "birth_tick": birth,
                    "sex": "male",
                    "death_cause": "natural" if leaf_dead else "none",
                }
            )
            if not leaf_dead:
                persons.append(
                    {
                        "id": i,
                        "alive": True,
                        "sex": "male",
                        "birth_tick": birth,
                        "death_tick": 0,
                        "death_cause": "none",
                        "mother": 0,
                        "father": father,
                    }
                )
        first, size = first + size, size * fanout
    return {"person": persons, "life_course": log}


def load_world(reg: Registry, raw: dict[str, Any]) -> World:
    """Build a fixture world from a ``[worlds.<name>]`` table."""
    tables: dict[str, dict[int, dict[str, Value]]] = {}
    generated = fan_rows(raw["fan"]) if "fan" in raw else {}
    for kind, rows in [*raw.items(), *generated.items()]:
        if not isinstance(rows, list):
            continue
        tables[kind] = {}
        key_column = reg.keys.get(kind, "id")
        for row in rows:
            cells: dict[str, Value] = {}
            for col, raw_cell in row.items():
                if col != "id":
                    cells[col] = cell_value(reg, reg.columns[f"{kind}.{col}"], raw_cell)
            key = int(row[key_column]) if key_column == "id" else scalar_int(cells[key_column])
            tables[kind][key] = cells
    params: dict[str, dict[str, Value]] = {}
    for system, values in raw.get("params", {}).items():
        params[system] = {}
        for name, raw_value in values.items():
            ty = reg.systems[system].params[name][2]
            params[system][name] = (
                int(raw_value) if ty == "int" else fx_from_fraction(ty, Fraction(str(raw_value)))
            )
    draws: dict[tuple[str, str, int, int], int] = {}
    for key, raw_draw in raw.get("draws", {}).items():
        purpose, entity, k = key.split("/")
        kind, ident = entity.split(":")
        draws[purpose, kind, int(ident), int(k)] = int(str(raw_draw))

    return World(
        int(raw["now"]),
        int(raw["days_per_year"]),
        tables,
        params,
        draws,
        enum_defaults=reg.enum_defaults,
    )


# ----------------------------------------------------------------------------- lexer
TOKEN_RE = re.compile(
    r"(?P<ws>\s+|#[^\n]*)"
    r"|(?P<num>[0-9][0-9A-Za-z_.]*|\.[0-9][0-9A-Za-z_.]*)"
    r"|(?P<ident>[A-Za-z_][A-Za-z0-9_]*)"
    r"|(?P<op>==|!=|<=|>=|[-+*/<>().,:;\[\]])"
)
INT_RE = re.compile(r"[0-9]+")
DEC_RE = re.compile(r"[0-9]+\.[0-9]+")


@dataclass(frozen=True)
class Token:
    """A lexical token: its kind (``num``, ``ident``, ``op``, ``kw``, ``eof``) and text."""

    kind: str
    text: str


def lex(text: str) -> list[Token]:
    """Tokenise a program; numeric tokens outside ``Int``/``Decimal`` are ``E_FLOAT_LITERAL``."""
    tokens: list[Token] = []
    pos = 0
    while pos < len(text):
        m = TOKEN_RE.match(text, pos)
        if m is None:
            raise CompileError("E_SYNTAX", f"unexpected character {text[pos]!r}")
        pos = m.end()
        if m.lastgroup == "ws":
            continue
        if m.lastgroup == "num":
            if not (INT_RE.fullmatch(m.group()) or DEC_RE.fullmatch(m.group())):
                raise CompileError("E_FLOAT_LITERAL", m.group())
            tokens.append(Token("num", m.group()))
        elif m.lastgroup == "ident":
            tokens.append(Token("kw" if m.group() in KEYWORDS else "ident", m.group()))
        else:
            tokens.append(Token("op", m.group()))
    tokens.append(Token("eof", ""))
    return tokens


# ----------------------------------------------------------------------------- checker context
@dataclass
class Scope:
    """The variables in scope, innermost last, with their types."""

    names: dict[str, str] = field(default_factory=dict)


@dataclass
class Ctx:
    """The compile-time context: Registry, declaring system, scopes and nesting budget."""

    reg: Registry
    system_name: str
    scope: dict[str, str]
    allow_key: bool = False
    depth: int = 0
    bound_product: int = 1
    draw_sites: set[str] = field(default_factory=set)
    loop_vars: list[str] = field(default_factory=list)
    outcome_payload: bool = (
        False  # the verb's event carries a drawn outcome: the program draws nothing
    )

    @property
    def system(self) -> System:
        """The declaring system's declarations."""
        if self.system_name not in self.reg.systems:
            raise CompileError("E_UNKNOWN_NAME", f"system {self.system_name}")
        return self.reg.systems[self.system_name]

    def bind(self, name: str, ty: str, loop: bool = False) -> None:
        """Bind a variable; shadowing is ``E_SYNTAX``. A loop variable is one a draw must key on."""
        if name in self.scope or name in KEYWORDS:
            raise CompileError("E_SYNTAX", f"variable {name} shadows a name")
        self.scope[name] = ty
        if loop:
            self.loop_vars.append(name)

    def unbind(self, name: str) -> None:
        """Remove a binding at the end of its construct."""
        del self.scope[name]
        if self.loop_vars and self.loop_vars[-1] == name:
            self.loop_vars.pop()

    def check_read(self, name: str) -> Column:
        """Accept a column read: it exists, is plane A, is owned or declared with a neutral."""
        column = self.reg.columns.get(name)
        if column is None:
            raise CompileError("E_UNKNOWN_NAME", f"column {name}")
        if column.plane != "A":
            raise CompileError("E_PLANE", name)
        if column.owner != self.base_system:
            if name not in self.system.reads:
                raise CompileError("E_UNDECLARED_READ", f"{self.system_name} reads {name}")
            if name not in self.system.neutral:  # minor 5: static, whatever the world registers
                raise CompileError("E_NO_NEUTRAL", f"{self.system_name} reads {name}: no neutral")
        return column

    @property
    def base_system(self) -> str:
        """The declaring system's Registry id (a case variant strips its suffix)."""
        return self.system_name.split("@")[0]

    def neutral_for(self, name: str, column: Column) -> Value | None:
        """The neutral value of a read whose owner is not registered (NEW-4), or ``None``."""
        if column.owner in self.reg.registered or column.owner == self.base_system:
            return None
        return cell_value(self.reg, column, self.system.neutral[name])

    def note_draw(self, site: str, keyed: list[Node]) -> None:
        """The static draw lint (§4.6): unique sites, and every loop variable in the key."""
        if site in self.draw_sites:
            raise CompileError("E_RNG_REUSE", site)
        self.draw_sites.add(site)
        for name in self.loop_vars:
            if not any(mentions(node, name) for node in keyed):
                raise CompileError("E_RNG_REUSE", f"draw inside a loop over {name} not keyed on it")

    def enter(self, bound: int) -> None:
        """Enter a bounded iteration: nesting depth and bound product are budgeted."""
        self.depth += 1
        self.bound_product *= bound
        if self.depth > NESTING_MAX or self.bound_product > BOUND_PRODUCT_MAX:
            raise CompileError("E_BOUND", f"depth {self.depth}, product {self.bound_product}")

    def leave(self, bound: int) -> None:
        """Leave a bounded iteration."""
        self.depth -= 1
        self.bound_product //= bound


def is_key(ty: str) -> bool:
    """Whether a type is a tuple or list key type (spec §3.4)."""
    return ty.startswith(("tuple(", "list("))


def unify(ta: str, tb: str) -> str:
    """Unify two types (spec §4.1): literals adopt concrete types; mismatches are ``E_TYPE``."""
    if ta == tb:
        return ta
    if ta in LITERALS and tb in LITERALS:
        if {ta, tb} == {"lit_int", "lit_dec"}:
            return "lit_dec"
        raise CompileError("E_TYPE", f"{ta} with {tb}")
    lit, con = (ta, tb) if ta in LITERALS else (tb, ta)
    if lit in LITERALS and (
        (lit == "lit_int" and con in NUMERIC)
        or (lit == "lit_dec" and con in FIXED)
        or (lit == "lit_none" and (con.startswith("ent:") or con == "event"))
    ):
        return con
    raise CompileError("E_TYPE", f"{ta} with {tb}")


def concrete(ty: str) -> str:
    """The default concrete type of a possibly polymorphic type."""
    if ty == "lit_int":
        return "int"
    if ty == "lit_dec":
        return "q16_16"
    if ty == "lit_none":
        raise CompileError("E_TYPE", "none without an entity kind")
    return ty


def mentions(node: object, name: str) -> bool:
    """Whether a variable occurs in a node (the loop-variable rule of the draw lint)."""
    if isinstance(node, Var):
        return node.name == name
    if isinstance(node, Node):
        return any(mentions(v, name) for v in vars(node).values())
    if isinstance(node, list):
        return any(mentions(item, name) for item in node)
    return False


def fold_int(node: Node) -> int | None:
    """The value of an ``int`` constant expression (literals, the day length), or ``None``."""
    if isinstance(node, NumLit) and "." not in node.text:
        return int(node.text)
    if isinstance(node, Dotted) and node.head == "calendar" and node.tail == "minutes_per_day":
        return MINUTES_PER_DAY
    if isinstance(node, Neg):
        inner = fold_int(node.e)
        return None if inner is None else sat(-inner, I64_MIN, I64_MAX)
    if isinstance(node, Bin) and node.op in ARITH_OPS:
        a, b = fold_int(node.a), fold_int(node.b)
        if a is None or b is None or (node.op in DIVISION_OPS and b == 0):
            return None
        return arith_int(node.op, a, b)
    return None


def require_numeric(ty: str) -> None:
    """``E_TYPE`` unless the type is numeric or a numeric literal."""
    if ty not in NUMERIC and ty not in ("lit_int", "lit_dec"):
        raise CompileError("E_TYPE", f"{ty} is not numeric")


def no_event(ty: str) -> str:
    """``E_TYPE`` for an ``event`` outside an event-typed payload field (spec §3.1)."""
    if ty == "event":
        raise CompileError("E_TYPE", "an event has no value outside a payload field")
    return ty


# ----------------------------------------------------------------------------- evaluator context
@dataclass
class Ev:
    """The run-time context: the world, the declaring system and the bound variables."""

    reg: Registry
    world: World
    system_name: str
    vars: dict[str, Value]


# ----------------------------------------------------------------------------- AST
class Node:
    """An expression node. ``check`` types it, ``resolve`` fixes literal types, ``eval`` runs it."""

    ty: str = ""

    def check(self, cx: Ctx) -> str:
        """Type the node and return its (possibly polymorphic) type."""
        raise NotImplementedError

    def resolve(self, ty: str) -> None:
        """Give the node (and its literal-carrying children) the concrete type ``ty``."""
        self.ty = ty

    def eval(self, ev: Ev) -> Value:
        """Evaluate the node."""
        raise NotImplementedError

    def finish(self) -> str:
        """Resolve the node to its default concrete type after checking."""
        self.resolve(concrete(self.ty))
        return self.ty


class NumLit(Node):
    """An ``Int`` or ``Decimal`` literal."""

    def __init__(self, text: str) -> None:
        """Keep the literal's text; its type comes from context."""
        self.text = text
        self.bits = 0

    def check(self, cx: Ctx) -> str:
        """``lit_int`` or ``lit_dec``."""
        self.ty = "lit_dec" if "." in self.text else "lit_int"
        return self.ty

    def resolve(self, ty: str) -> None:
        """Compile the literal into ``ty`` (``E_LITERAL_RANGE`` if it does not fit)."""
        self.ty = ty
        self.bits = literal_bits(ty, self.text)

    def eval(self, ev: Ev) -> Value:
        """The compiled value."""
        return self.bits if self.ty == "int" else Fx(self.ty, self.bits)


class BoolLit(Node):
    """``true`` or ``false``."""

    def __init__(self, value: bool) -> None:
        """Keep the value."""
        self.value = value

    def check(self, cx: Ctx) -> str:
        """``bool``."""
        self.ty = "bool"
        return self.ty

    def eval(self, ev: Ev) -> Value:
        """The value."""
        return self.value


class NoneLit(Node):
    """``none``: polymorphic over entity kinds."""

    def check(self, cx: Ctx) -> str:
        """``lit_none`` until a context gives it a kind."""
        self.ty = "lit_none"
        return self.ty

    def eval(self, ev: Ev) -> Value:
        """``none`` of the resolved kind (an event-typed ``none`` is the absent event)."""
        if self.ty == "event":
            return EventRef(own=False)
        return Et(self.ty[4:], 0)


class Var(Node):
    """A bound variable."""

    def __init__(self, name: str) -> None:
        """Keep the name."""
        self.name = name

    def check(self, cx: Ctx) -> str:
        """The variable's type from the scope."""
        if self.name not in cx.scope:
            raise CompileError("E_UNKNOWN_NAME", self.name)
        self.ty = cx.scope[self.name]
        return self.ty

    def eval(self, ev: Ev) -> Value:
        """The bound value."""
        return ev.vars[self.name]


class Dotted(Node):
    """``head.tail`` without a call: an enum literal, ``params.x`` or ``calendar.x``."""

    def __init__(self, head: str, tail: str) -> None:
        """Keep both identifiers."""
        self.head = head
        self.tail = tail
        self.index = 0

    def check(self, cx: Ctx) -> str:
        """Resolve the name (spec §4.7)."""
        if self.head == "params":
            if self.tail not in cx.system.params:
                raise CompileError("E_UNKNOWN_NAME", f"params.{self.tail}")
            self.ty = cx.system.params[self.tail][2]
        elif self.head == "calendar":
            if self.tail not in ("days_per_year", "minutes_per_day"):
                raise CompileError("E_UNKNOWN_NAME", f"calendar.{self.tail}")
            self.ty = "int"
        elif self.head in cx.reg.enums and self.tail in cx.reg.enums[self.head]:
            self.index = cx.reg.enums[self.head].index(self.tail)
            self.ty = f"enum:{self.head}"
        else:
            raise CompileError("E_UNKNOWN_NAME", f"{self.head}.{self.tail}")
        return self.ty

    def is_constant(self, cx: Ctx, ty: str) -> bool:
        """Whether this is a constant divisor whose rounded bounds exclude zero (spec §4.4)."""
        if self.head == "calendar":
            return True
        if self.head == "params":
            lo, hi, _ = cx.system.params[self.tail]
            scale = 1 if ty == "int" else 1 << FRAC_BITS[ty]
            return round(lo * scale) > 0 or round(hi * scale) < 0
        return False

    def eval(self, ev: Ev) -> Value:
        """The parameter, calendar constant or enum value."""
        if self.head == "params":
            return ev.world.params[ev.system_name][self.tail]
        if self.head == "calendar":
            return ev.world.days_per_year if self.tail == "days_per_year" else MINUTES_PER_DAY
        return En(self.head, self.index)


class Read(Node):
    """``entity.column(e)``: a declared read."""

    def __init__(self, entity: str, column: str, arg: Node) -> None:
        """Keep the column name and the entity expression."""
        self.name = f"{entity}.{column}"
        self.entity = entity
        self.arg = arg
        self.column: Column | None = None
        self.neutral: Value | None = None

    def check(self, cx: Ctx) -> str:
        """Check the read is allowed and the argument has the column's entity kind."""
        self.column = cx.check_read(self.name)
        self.neutral = cx.neutral_for(self.name, self.column)
        self.arg.resolve(unify(self.arg.check(cx), f"ent:{self.column.key}"))
        self.ty = self.column.ty
        return self.ty

    def eval(self, ev: Ev) -> Value:
        """Read the column (missing: the type's zero; an absent owner: the neutral value)."""
        assert self.column is not None
        entity = self.arg.eval(ev)
        assert isinstance(entity, Et)
        if self.neutral is not None:
            ev.world.neutral_reads += 1
            return self.neutral
        return ev.world.read(self.column, self.name.split(".")[1], entity)


class Rel(Node):
    """A prelude relation ``name(args)``; its type is ``rel:<kind>``."""

    def __init__(self, name: str, args: list[Node]) -> None:
        """Keep the name and arguments."""
        self.name = name
        self.args = args

    @property
    def bound(self) -> int:
        """The relation's declared bound."""
        return RELATIONS[self.name][3]

    def check(self, cx: Ctx) -> str:
        """Check the relation exists, its arity, its argument kinds and its reads."""
        if self.name not in RELATIONS:
            raise CompileError("E_UNKNOWN_NAME", f"relation {self.name}")
        kinds, yields, reads, _ = RELATIONS[self.name]
        if len(self.args) != len(kinds):
            raise CompileError("E_ARITY", self.name)
        for arg, kind in zip(self.args, kinds, strict=True):
            if kind in DEGREE_BOUNDS:
                check_degree(arg, DEGREE_BOUNDS[kind], cx)
            else:
                arg.resolve(unify(arg.check(cx), f"ent:{kind}"))
        for column in reads:
            cx.check_read(column)
        self.ty = f"rel:{yields}"
        return self.ty

    def eval(self, ev: Ev) -> Value:
        """The relation's elements in canonical order, at most its bound."""
        args = [a.eval(ev) for a in self.args]
        return relation(ev.world, self.name, args)


def check_degree(arg: Node, upper: int, cx: Ctx) -> None:
    """A degree: an ``Int`` literal, or an ``int`` parameter whose bounds lie in ``1..upper``."""
    if isinstance(arg, NumLit) and "." not in arg.text:
        lo = hi = Fraction(arg.text)
    elif isinstance(arg, Dotted) and arg.head == "params":
        if arg.check(cx) != "int":
            raise CompileError("E_TYPE", f"params.{arg.tail} is not an int parameter")
        lo, hi, _ = cx.system.params[arg.tail]
    else:
        raise CompileError("E_CONST", "a degree must be an Int literal or an int parameter")
    if not 1 <= lo <= hi <= upper:
        raise CompileError("E_BOUND", f"degree in [{lo}, {hi}] outside 1..{upper}")
    arg.resolve("int")


class Call(Node):
    """A builtin, an accessor, ``draw`` or ``chance``."""

    def __init__(self, name: str, args: list[Node]) -> None:
        """Keep the name and arguments."""
        self.name = name
        self.args = args

    def check(self, cx: Ctx) -> str:
        """Type the call by its builtin's signature."""
        handlers = {
            "ordinal": self.check_ordinal,
            "exists": self.check_exists,
            "floor": self.check_rounding,
            "round": self.check_rounding,
            "draw": self.check_draw,
            "chance": self.check_draw,
            "related": self.check_related,
        }
        if self.name in handlers:
            self.ty = handlers[self.name](cx)
        elif self.name in FIXED_BUILTINS:
            self.ty = self.check_fixed(cx)
        elif self.name in POLY_BUILTINS:
            self.ty = self.check_poly(cx)
        elif self.name in ACCESSORS:
            self.ty = self.check_accessor(cx)
        else:
            raise CompileError("E_UNKNOWN_NAME", f"call {self.name}")
        return self.ty

    def arity(self, n: int) -> None:
        """``E_ARITY`` unless the call has ``n`` arguments."""
        if len(self.args) != n:
            raise CompileError("E_ARITY", f"{self.name} takes {n}")

    def check_ordinal(self, cx: Ctx) -> str:
        """``ordinal(enum) -> int``."""
        self.arity(1)
        if not self.args[0].check(cx).startswith("enum:"):
            raise CompileError("E_TYPE", "ordinal of a non-enum")
        return "int"

    def check_exists(self, cx: Ctx) -> str:
        """``exists(entity) -> bool``."""
        self.arity(1)
        ty = concrete(self.args[0].check(cx))
        if not ty.startswith("ent:"):
            raise CompileError("E_TYPE", "exists of a non-entity")
        self.args[0].resolve(ty)
        return "bool"

    def check_rounding(self, cx: Ctx) -> str:
        """``floor`` and ``round``: fixed point to ``int``."""
        self.arity(1)
        ty = self.args[0].check(cx)
        ty = "q16_16" if ty == "lit_dec" else ty
        if ty not in FIXED:
            raise CompileError("E_TYPE", f"{self.name} of {ty}")
        self.args[0].resolve(ty)
        return "int"

    def check_fixed(self, cx: Ctx) -> str:
        """Builtins with fixed signatures."""
        params, result = FIXED_BUILTINS[self.name]
        self.arity(len(params))
        for arg, want in zip(self.args, params, strict=True):
            arg.resolve(unify(arg.check(cx), want))
        return result

    def check_poly(self, cx: Ctx) -> str:
        """Builtins whose numeric arguments unify to one type."""
        self.arity(POLY_BUILTINS[self.name])
        ty = self.args[0].check(cx)
        for arg in self.args[1:]:
            ty = unify(ty, arg.check(cx))
        require_numeric(ty)
        if ty not in LITERALS:
            self.resolve(ty)
        return ty

    def check_accessor(self, cx: Ctx) -> str:
        """The genealogical accessors ``up``, ``common`` and their agnatic forms."""
        result, reads = ACCESSORS[self.name]
        self.arity(2)
        for arg in self.args:
            arg.resolve(unify(arg.check(cx), "ent:person"))
        for column in reads:
            cx.check_read(column)
        return result

    def check_related(self, cx: Ctx) -> str:
        """``related(a, b, n) -> bool``: within ``n`` degrees of kinship."""
        self.arity(3)
        for arg in self.args[:2]:
            arg.resolve(unify(arg.check(cx), "ent:person"))
        check_degree(self.args[2], KIN_DEGREES_MAX, cx)
        for column in GENEALOGY:
            cx.check_read(column)
        return "bool"

    def check_draw(self, cx: Ctx) -> str:
        """``draw(purpose, e, k) -> int`` and ``chance(purpose, e, k, p) -> bool``."""
        self.arity(3 if self.name == "draw" else 4)
        purpose = self.args[0]
        if not isinstance(purpose, Var):
            raise CompileError("E_CONST", "purpose must be an identifier")
        if purpose.name not in cx.system.rng:
            raise CompileError("E_RNG_PURPOSE", purpose.name)
        if cx.outcome_payload:  # minor 2: the host drew the outcome; a draw here would re-roll it
            raise CompileError("E_RNG_REUSE", "a verb with an outcome payload draws nothing")
        self.args[1].resolve(concrete(self.args[1].check(cx)))
        if not self.args[1].ty.startswith("ent:"):
            raise CompileError("E_TYPE", "draw entity")
        index = self.args[2]
        index.resolve(concrete(index.check(cx)))
        if index.ty != "int" and not index.ty.startswith("ent:"):
            raise CompileError("E_TYPE", "draw index must be an int or an entity")
        if self.name == "chance":
            self.args[3].resolve(unify(self.args[3].check(cx), "q16_16"))
        cx.note_draw(f"{purpose.name}/{site_key(self.args[1])}/{site_key(index)}", self.args[1:3])
        return "int" if self.name == "draw" else "bool"

    def resolve(self, ty: str) -> None:
        """Polymorphic builtins pass the type down to their arguments."""
        self.ty = ty
        if self.name in POLY_BUILTINS:
            for arg in self.args:
                arg.resolve(ty)

    def eval(self, ev: Ev) -> Value:
        """Evaluate the builtin."""
        if self.name in ("draw", "chance"):
            return self.eval_draw(ev)
        if self.name == "related":
            a, b, n = (arg.eval(ev) for arg in self.args)
            assert isinstance(a, Et) and isinstance(b, Et) and isinstance(n, int)
            both = a.ident != 0 and b.ident != 0
            return both and b.ident in kin_within(ev.world, a.ident, n)
        if self.name in ACCESSORS:
            x, p = (self.args[0].eval(ev), self.args[1].eval(ev))
            assert isinstance(x, Et) and isinstance(p, Et)
            if self.name == "descends":
                return descends(ev.world, x.ident, p.ident)
            return accessor(ev.world, self.name, x, p)
        values = [a.eval(ev) for a in self.args]
        return builtin(self.name, values)

    def eval_draw(self, ev: Ev) -> Value:
        """The keyed draw: ``int`` in ``[0, 2^63)`` or the ``draw_hits`` test."""
        purpose = self.args[0]
        assert isinstance(purpose, Var)
        entity = self.args[1].eval(ev)
        assert isinstance(entity, Et)
        index = self.args[2].eval(ev)
        k = index.ident if isinstance(index, Et) else index
        assert isinstance(k, int)
        key = (purpose.name, entity.kind, entity.ident, k)
        if key in ev.world.drawn:  # minor 3: the run-time layer of the reuse rule
            raise InvariantError(f"E_RNG_REUSE: {key} drawn twice in one step")
        ev.world.drawn.add(key)
        if key not in ev.world.draws:
            raise KeyError(f"the golden world lists no draw for {key}")
        u = ev.world.draws[key]
        if self.name == "draw":
            return u >> 1
        p = self.args[3].eval(ev)
        assert isinstance(p, Fx)
        return draw_hits(u, p)


def site_key(node: Node) -> str:
    """A draw key's text for the site lint: a folded constant, or the typed canonical form."""
    folded = fold_int(node)
    return f"int:{folded}" if folded is not None else f"{node.ty}:{canon(node)}"


def canon(node: object) -> str:
    """A canonical text of a node: its class and attributes, recursively (the draw-site lint)."""
    if isinstance(node, Node):
        skip = {"ty", "bits", "column", "neutral", "index"}
        parts = [f"{k}={canon(v)}" for k, v in sorted(vars(node).items()) if k not in skip]
        return f"{type(node).__name__}({', '.join(parts)})"
    if isinstance(node, list):
        return "[" + ", ".join(canon(item) for item in node) + "]"
    return repr(node)


def draw_hits(u: int, p: Fx) -> bool:
    """``rh_core::fixed::draw_hits``: ``u / 2^64 < p``, with ``p`` a ``q16_16``."""
    if p.bits <= 0:
        return False
    if p.bits >= 1 << FRAC_BITS["q16_16"]:
        return True
    return u < p.bits << (DRAW_BITS - FRAC_BITS["q16_16"])


def builtin(name: str, values: list[Value]) -> Value:
    """Evaluate a builtin with fixed or polymorphic numeric signature."""
    if name == "ordinal":
        enum = values[0]
        assert isinstance(enum, En)
        return enum.index
    if name in CONVERSIONS:
        return CONVERSIONS[name](values[0])
    return poly_builtin(name, values)


def poly_builtin(name: str, values: list[Value]) -> Value:
    """``abs``, ``min``, ``max``, ``clamp`` and ``safe_div``."""
    if name == "abs":
        return values[0] if numeric_key(values[0]) >= 0 else negate(values[0])
    if name == "min":
        return min(values[0], values[1], key=numeric_key)
    if name == "max":
        return max(values[0], values[1], key=numeric_key)
    if name == "clamp":
        return min(max(values[0], values[1], key=numeric_key), values[2], key=numeric_key)
    return values[2] if is_zero(values[1]) else arith("/", values[0], values[1])


def to_fixed(ty: str, n: Value) -> Value:
    """``q8(n)`` and ``q16(n)``: saturating."""
    assert isinstance(n, int)
    return fx_from_fraction(ty, Fraction(n))


def floor_fx(x: Value) -> Value:
    """``floor``: toward negative infinity."""
    assert isinstance(x, Fx)
    return x.bits >> FRAC_BITS[x.ty]


def round_fx(x: Value) -> Value:
    """``round``: nearest, half to even."""
    assert isinstance(x, Fx)
    return round(fx_value(x))


def widen(x: Value) -> Value:
    """``widen``: exact."""
    assert isinstance(x, Fx)
    return Fx("q16_16", x.bits << (FRAC_BITS["q16_16"] - FRAC_BITS["q8_8"]))


def narrow(x: Value) -> Value:
    """``narrow``: half to even to 8 fractional bits, then saturate."""
    assert isinstance(x, Fx)
    return fx_from_fraction("q8_8", fx_value(x))


def q8_of(n: Value) -> Value:
    """``q8(n)``."""
    return to_fixed("q8_8", n)


def q16_of(n: Value) -> Value:
    """``q16(n)``."""
    return to_fixed("q16_16", n)


CONVERSIONS: dict[str, Callable[[Value], Value]] = {
    "widen": widen,
    "narrow": narrow,
    "q8": q8_of,
    "q16": q16_of,
    "floor": floor_fx,
    "round": round_fx,
}


class Exists(Call):
    """``exists(e)``: the entity is not ``none`` and has a row."""

    def eval(self, ev: Ev) -> Value:
        """Whether the row exists."""
        entity = self.args[0].eval(ev)
        assert isinstance(entity, Et)
        return entity.ident != 0 and ev.world.row(entity.kind, entity.ident) is not None


@dataclass(eq=False)
class Agg(Node):
    """An aggregate over a relation (spec §5.4); ``body`` and ``orelse`` are optional by kind."""

    op: str
    var: str
    rel: Rel
    where: Node | None
    body: Node | None
    orelse: Node | None

    def check(self, cx: Ctx) -> str:
        """Type the aggregate by its kind and bind the variable over its parts."""
        rel_ty = self.rel.check(cx)
        cx.enter(self.rel.bound)
        cx.bind(self.var, f"ent:{rel_ty[4:]}", loop=True)
        try:
            self.ty = self.check_parts(cx)
        finally:
            cx.unbind(self.var)
            cx.leave(self.rel.bound)
        return self.ty

    def check_parts(self, cx: Ctx) -> str:
        """The per-aggregate rules on ``where``, body and ``else``."""
        if self.where is not None:
            self.where.resolve(unify(self.where.check(cx), "bool"))
        if self.op in ("count", "any", "all"):
            return self.check_predicate(cx)
        needs_else = self.op != "sum"
        if self.body is None or needs_else != (self.orelse is not None):
            raise CompileError("E_SYNTAX", f"{self.op}: body{' and else' * needs_else} required")
        ty = self.body.check(cx)
        if self.orelse is not None:
            ty = unify(ty, self.orelse.check(cx))
        require_numeric(ty)
        if ty not in LITERALS:
            self.resolve(ty)
        return ty

    def check_predicate(self, cx: Ctx) -> str:
        """``count`` takes no body; ``any``/``all`` take ``where`` or a ``bool`` body, not both."""
        if self.orelse is not None or (self.op == "count" and self.body is not None):
            raise CompileError("E_SYNTAX", f"{self.op} takes no body or else")
        if self.body is not None:
            if self.where is not None:
                raise CompileError("E_SYNTAX", f"{self.op} with both where and body")
            self.body.resolve(unify(self.body.check(cx), "bool"))
        return "int" if self.op == "count" else "bool"

    def resolve(self, ty: str) -> None:
        """Pass a numeric type into the body and the default."""
        self.ty = ty
        if self.op in ("sum", "min", "max"):
            assert self.body is not None
            self.body.resolve(ty)
            if self.orelse is not None:
                self.orelse.resolve(ty)

    def eval(self, ev: Ev) -> Value:
        """Fold over the relation in canonical order."""
        elements = self.rel.eval(ev)
        assert isinstance(elements, list)
        picked: list[Value] = []
        for element in elements:
            ev.vars[self.var] = element
            if self.where is None or self.where.eval(ev):
                picked.append(self.body.eval(ev) if self.body is not None else True)
        ev.vars.pop(self.var, None)
        return self.fold(ev, picked)

    def fold(self, ev: Ev, picked: list[Value]) -> Value:
        """Combine the picked bodies."""
        if self.op == "count":
            return len(picked)
        if self.op == "any":
            return any(bool(v) for v in picked)
        if self.op == "all":
            return all(bool(v) for v in picked)
        if self.op == "sum":
            total: Value = zero_of(self.ty)
            for v in picked:
                total = arith("+", total, v)
            return total
        if not picked:
            assert self.orelse is not None
            return self.orelse.eval(ev)
        pick = min if self.op == "min" else max
        return pick(picked, key=numeric_key)


class Tuple(Node):
    """A key tuple (two or more elements)."""

    def __init__(self, items: list[Node]) -> None:
        """Keep the elements."""
        self.items = items

    def check(self, cx: Ctx) -> str:
        """Every element is a scalar or a key; the tuple's type fixes arity and element types."""
        if not cx.allow_key:
            raise CompileError("E_TYPE", "a tuple outside an order program")
        parts = []
        for item in self.items:
            item.check(cx)
            parts.append(no_event(item.finish()))
        self.ty = f"tuple({','.join(parts)})"
        return self.ty

    def eval(self, ev: Ev) -> Value:
        """The tuple of element values."""
        return tuple(item.eval(ev) for item in self.items)


class ListKey(Node):
    """``[ key for y in relation ]``: a bounded list key."""

    def __init__(self, body: Node, var: str, rel: Rel) -> None:
        """Keep the body, the variable and the relation."""
        self.body = body
        self.var = var
        self.rel = rel

    def check(self, cx: Ctx) -> str:
        """Bind the variable over the body; only where keys are allowed."""
        if not cx.allow_key:
            raise CompileError("E_TYPE", "a list key outside an order program")
        rel_ty = self.rel.check(cx)
        cx.enter(self.rel.bound)
        cx.bind(self.var, f"ent:{rel_ty[4:]}", loop=True)
        try:
            self.body.check(cx)
            element = no_event(self.body.finish())
        finally:
            cx.unbind(self.var)
            cx.leave(self.rel.bound)
        self.ty = f"list({element})"
        return self.ty

    def eval(self, ev: Ev) -> Value:
        """The list of body values in relation order."""
        elements = self.rel.eval(ev)
        assert isinstance(elements, list)
        out: list[Value] = []
        for element in elements:
            ev.vars[self.var] = element
            out.append(self.body.eval(ev))
        ev.vars.pop(self.var, None)
        return out


class If(Node):
    """``if c then a else b``."""

    def __init__(self, cond: Node, a: Node, b: Node) -> None:
        """Keep the three parts."""
        self.cond = cond
        self.a = a
        self.b = b

    def check(self, cx: Ctx) -> str:
        """The condition is ``bool``; the branches unify."""
        self.cond.resolve(unify(self.cond.check(cx), "bool"))
        self.ty = unify(self.a.check(cx), self.b.check(cx))
        if self.ty not in LITERALS:
            self.resolve(self.ty)
        return self.ty

    def resolve(self, ty: str) -> None:
        """Both branches take the type."""
        self.ty = ty
        self.a.resolve(ty)
        self.b.resolve(ty)

    def eval(self, ev: Ev) -> Value:
        """The chosen branch."""
        return self.a.eval(ev) if self.cond.eval(ev) else self.b.eval(ev)


class Neg(Node):
    """Unary minus."""

    def __init__(self, e: Node) -> None:
        """Keep the operand."""
        self.e = e

    def check(self, cx: Ctx) -> str:
        """Numeric."""
        self.ty = self.e.check(cx)
        require_numeric(self.ty)
        return self.ty

    def resolve(self, ty: str) -> None:
        """Pass the type down."""
        self.ty = ty
        self.e.resolve(ty)

    def eval(self, ev: Ev) -> Value:
        """Saturating negation."""
        return negate(self.e.eval(ev))


class Not(Node):
    """Logical not."""

    def __init__(self, e: Node) -> None:
        """Keep the operand."""
        self.e = e

    def check(self, cx: Ctx) -> str:
        """``bool``."""
        self.e.resolve(unify(self.e.check(cx), "bool"))
        self.ty = "bool"
        return self.ty

    def eval(self, ev: Ev) -> Value:
        """The negation."""
        return not self.e.eval(ev)


class Bin(Node):
    """A binary operator: arithmetic, comparison, ``and`` or ``or``."""

    def __init__(self, op: str, a: Node, b: Node) -> None:
        """Keep the operator and operands."""
        self.op = op
        self.a = a
        self.b = b

    def check(self, cx: Ctx) -> str:
        """Dispatch on the operator class."""
        if self.op in ("and", "or"):
            self.a.resolve(unify(self.a.check(cx), "bool"))
            self.b.resolve(unify(self.b.check(cx), "bool"))
            self.ty = "bool"
        elif self.op in CMP_OPS:
            self.ty = self.check_cmp(cx)
        else:
            self.ty = self.check_arith(cx)
        return self.ty

    def check_cmp(self, cx: Ctx) -> str:
        """Comparisons: same type; ordering only on numbers."""
        ty = concrete(unify(self.a.check(cx), self.b.check(cx)))
        if is_key(ty) or ty.startswith("rel:") or ty == "event":
            raise CompileError("E_TYPE", f"comparison of {ty}")
        if self.op in ORDER_OPS and ty not in NUMERIC:
            raise CompileError("E_TYPE", f"{self.op} on {ty}")
        self.a.resolve(ty)
        self.b.resolve(ty)
        return "bool"

    def check_arith(self, cx: Ctx) -> str:
        """Arithmetic: same numeric type; ``div``/``mod`` on ``int``; guarded division."""
        ty = unify(self.a.check(cx), self.b.check(cx))
        require_numeric(ty)
        if self.op in ("div", "mod") and ty not in ("int", "lit_int"):
            raise CompileError("E_TYPE", f"{self.op} on {ty}")
        if ty not in LITERALS:
            self.resolve(ty)
        if self.op in DIVISION_OPS:
            self.check_divisor(cx, concrete(ty))
        return ty

    def check_divisor(self, cx: Ctx, ty: str) -> None:
        """The divisor is a constant whose bits, after rounding to ``ty``, cannot be zero (§4.4)."""
        d = self.b.e if isinstance(self.b, Neg) else self.b
        if isinstance(d, NumLit) and literal_bits(ty, d.text) != 0:
            return
        if isinstance(d, Dotted) and d.is_constant(cx, ty):
            return
        raise CompileError("E_DIV", "divisor is not a constant with non-zero bits")

    def resolve(self, ty: str) -> None:
        """Arithmetic passes the type down; comparisons and logic are already concrete."""
        self.ty = ty
        if self.op in ARITH_OPS:
            self.a.resolve(ty)
            self.b.resolve(ty)

    def eval(self, ev: Ev) -> Value:
        """Evaluate left, then right, then apply."""
        a = self.a.eval(ev)
        b = self.b.eval(ev)
        if self.op == "and":
            return bool(a) and bool(b)
        if self.op == "or":
            return bool(a) or bool(b)
        if self.op in CMP_OPS:
            return compare(self.op, a, b)
        return arith(self.op, a, b)


def compare(op: str, a: Value, b: Value) -> bool:
    """Equality on any scalar; ordering on numbers."""
    if op == "==":
        return a == b
    if op == "!=":
        return a != b
    ka, kb = numeric_key(a), numeric_key(b)
    return {"<": ka < kb, "<=": ka <= kb, ">": ka > kb, ">=": ka >= kb}[op]


# ----------------------------------------------------------------------------- effects AST
@dataclass
class Action:
    """``Cmd(name: expr, …)``."""

    name: str
    args: list[tuple[str, Node]]
    person_index: int = -1
    order: list[int] = field(default_factory=list)
    dropped: bool = False

    def check(self, cx: Ctx) -> None:
        """The command is known, declared, sent to its owner, registered, and fully named."""
        command = cx.reg.commands.get(self.name)
        if command is None:
            raise CompileError("E_UNKNOWN_NAME", f"command {self.name}")
        target = cx.system.sends.get(self.name)
        if target is None:
            raise CompileError("E_UNDECLARED_COMMAND", f"{cx.system_name} sends {self.name}")
        if target != command.owner:
            raise CompileError("E_COMMAND_OWNER", f"{self.name} to {target}, owner {command.owner}")
        self.dropped = command.owner not in cx.reg.registered  # NEW-4: dropped at registration
        self.check_args(cx, command)

    def check_args(self, cx: Ctx, command: Command) -> None:
        """Every parameter exactly once, each of its declared type."""
        wanted = {"person": "ent:person", **dict(command.params)}
        given = [name for name, _ in self.args]
        if sorted(given) != sorted(wanted):
            raise CompileError("E_ARITY", f"{self.name} arguments {given}")
        for name, node in self.args:
            node.resolve(unify(node.check(cx), wanted[name]))
        by_name = {name: i for i, (name, _) in enumerate(self.args)}
        self.person_index = by_name["person"]
        self.order = [by_name[name] for name, _ in command.params]

    def eval(self, ev: Ev) -> Cmd | None:
        """Evaluate the arguments in order, then encode (§6); dropped when the owner is absent."""
        values = [node.eval(ev) for _, node in self.args]
        if self.dropped:
            ev.world.dropped_actions += 1
            return None
        person = values[self.person_index]
        assert isinstance(person, Et)
        return Cmd(self.name, person.ident, tuple(encode(values[i]) for i in self.order))


def encode(v: Value) -> int | str:
    """Encode a scalar into ``Command::args`` (``i64``); the verb's own event is ``event`` (§6)."""
    if isinstance(v, EventRef):
        return "event" if v.own else 0
    if isinstance(v, bool):
        return int(v)
    if isinstance(v, int):
        return v
    if isinstance(v, Fx):
        return v.bits
    if isinstance(v, En):
        return v.index
    assert isinstance(v, Et)
    return v.ident - (1 << 64) if v.ident > I64_MAX else v.ident


@dataclass
class Effect:
    """One effect: a conditional action or a bounded ``for`` over a relation."""

    action: Action
    when: Node | None = None
    var: str | None = None
    rel: Rel | None = None
    where: Node | None = None

    def check(self, cx: Ctx) -> None:
        """Type the guard, bind the loop variable, check the action."""
        if self.when is not None:
            self.when.resolve(unify(self.when.check(cx), "bool"))
        if self.rel is None or self.var is None:
            self.action.check(cx)
            return
        rel_ty = self.rel.check(cx)
        cx.enter(self.rel.bound)
        cx.bind(self.var, f"ent:{rel_ty[4:]}", loop=True)
        try:
            if self.where is not None:
                self.where.resolve(unify(self.where.check(cx), "bool"))
            self.action.check(cx)
        finally:
            cx.unbind(self.var)
            cx.leave(self.rel.bound)

    def eval(self, ev: Ev) -> list[Cmd]:
        """The commands this effect emits, in order."""
        out: list[Cmd] = []
        if self.rel is None or self.var is None:
            if self.when is None or self.when.eval(ev):
                emitted = self.action.eval(ev)
                out.extend([emitted] if emitted is not None else [])
            return out
        elements = self.rel.eval(ev)
        assert isinstance(elements, list)
        for element in elements:
            ev.vars[self.var] = element
            if self.where is None or self.where.eval(ev):
                emitted = self.action.eval(ev)
                out.extend([emitted] if emitted is not None else [])
        ev.vars.pop(self.var, None)
        return out


@dataclass
class Effects:
    """An effect program."""

    items: list[Effect]

    def check(self, cx: Ctx) -> None:
        """Check every effect."""
        for item in self.items:
            item.check(cx)

    def eval(self, ev: Ev) -> list[Cmd]:
        """Emit in order."""
        return [cmd for item in self.items for cmd in item.eval(ev)]


# ----------------------------------------------------------------------------- parser
class Parser:
    """A recursive-descent parser over the token list (spec §2)."""

    def __init__(self, text: str) -> None:
        """Tokenise the program."""
        self.tokens = lex(text)
        self.pos = 0
        self.depth = 0

    def peek(self, offset: int = 0) -> Token:
        """The token ``offset`` ahead."""
        index = min(self.pos + offset, len(self.tokens) - 1)
        return self.tokens[index]

    def take(self) -> Token:
        """Consume and return the next token."""
        token = self.tokens[self.pos]
        if token.kind != "eof":
            self.pos += 1
        return token

    def at(self, text: str) -> bool:
        """Whether the next token is the operator or keyword ``text``."""
        token = self.peek()
        return token.kind in ("op", "kw") and token.text == text

    def accept(self, text: str) -> bool:
        """Consume the next token if it is ``text``."""
        if self.at(text):
            self.take()
            return True
        return False

    def expect(self, text: str) -> None:
        """Consume ``text`` or fail."""
        if not self.accept(text):
            raise CompileError("E_SYNTAX", f"expected {text!r}, found {self.peek().text!r}")

    def ident(self) -> str:
        """Consume an identifier."""
        token = self.take()
        if token.kind != "ident":
            raise CompileError("E_SYNTAX", f"expected a name, found {token.text!r}")
        return token.text

    def dotted_tail(self) -> str:
        """The name after a dot: an identifier or a keyword (``death_cause.none``, minor 4)."""
        token = self.take()
        if token.kind not in ("ident", "kw"):
            raise CompileError("E_SYNTAX", f"expected a name after '.', found {token.text!r}")
        return token.text

    def end(self) -> None:
        """Fail unless every token was consumed."""
        if self.peek().kind != "eof":
            raise CompileError("E_SYNTAX", f"unexpected {self.peek().text!r}")

    # --- programs
    def program(self) -> Node:
        """An expression program."""
        node = self.expr()
        self.end()
        return node

    def candidates(self) -> Rel:
        """A relation program."""
        node = self.primary()
        self.end()
        if not isinstance(node, Rel):
            raise CompileError("E_TYPE", "candidates must be a relation")
        return node

    def effects(self) -> Effects:
        """An effect program."""
        items: list[Effect] = []
        if self.peek().kind != "eof":
            items.append(self.effect())
            while self.accept(";"):
                items.append(self.effect())
        self.end()
        return Effects(items)

    def effect(self) -> Effect:
        """One effect."""
        if self.accept("when"):
            cond = self.expr()
            self.expect(":")
            return Effect(self.action(), when=cond)
        if self.accept("for"):
            var = self.ident()
            self.expect("in")
            rel = self.relation()
            where = self.expr() if self.accept("where") else None
            self.expect(":")
            return Effect(self.action(), var=var, rel=rel, where=where)
        return Effect(self.action())

    def action(self) -> Action:
        """``Cmd(name: expr, …)``."""
        name = self.ident()
        self.expect("(")
        args: list[tuple[str, Node]] = []
        if not self.at(")"):
            args.append(self.named_arg())
            while self.accept(","):
                args.append(self.named_arg())
        self.expect(")")
        return Action(name, args)

    def named_arg(self) -> tuple[str, Node]:
        """``name: expr``."""
        name = self.ident()
        self.expect(":")
        return name, self.expr()

    def relation(self) -> Rel:
        """A relation after ``in``."""
        node = self.primary()
        if isinstance(node, Call) and node.name not in KNOWN_CALLS:
            raise CompileError("E_UNKNOWN_NAME", f"relation {node.name}")
        if not isinstance(node, Rel):
            raise CompileError("E_SYNTAX", "expected a relation")
        return node

    # --- expressions
    def expr(self) -> Node:
        """``or_expr``; nesting deeper than ``EXPRESSION_DEPTH_MAX`` is ``E_BOUND``."""
        self.depth += 1
        if self.depth > EXPRESSION_DEPTH_MAX:
            raise CompileError("E_BOUND", f"expression nesting above {EXPRESSION_DEPTH_MAX}")
        node = self.and_expr()
        while self.accept("or"):
            node = Bin("or", node, self.and_expr())
        self.depth -= 1
        return node

    def and_expr(self) -> Node:
        """``and_expr``."""
        node = self.not_expr()
        while self.accept("and"):
            node = Bin("and", node, self.not_expr())
        return node

    def not_expr(self) -> Node:
        """``not_expr``."""
        if self.accept("not"):
            return Not(self.not_expr())
        return self.cmp_expr()

    def cmp_expr(self) -> Node:
        """Non-associative comparison."""
        node = self.add_expr()
        token = self.peek()
        if token.kind == "op" and token.text in CMP_OPS:
            self.take()
            node = Bin(token.text, node, self.add_expr())
            after = self.peek()
            if after.kind == "op" and after.text in CMP_OPS:
                raise CompileError("E_SYNTAX", "chained comparison")
        return node

    def add_expr(self) -> Node:
        """``+`` and ``-``."""
        node = self.mul_expr()
        while self.peek().kind == "op" and self.peek().text in ("+", "-"):
            node = Bin(self.take().text, node, self.mul_expr())
        return node

    def mul_expr(self) -> Node:
        """``*``, ``/``, ``div`` and ``mod``."""
        node = self.unary()
        while self.at("*") or self.at("/") or self.at("div") or self.at("mod"):
            node = Bin(self.take().text, node, self.unary())
        return node

    def unary(self) -> Node:
        """Unary minus."""
        if self.accept("-"):
            return Neg(self.unary())
        return self.primary()

    def primary(self) -> Node:
        """A primary expression."""
        token = self.peek()
        if token.kind == "num":
            return NumLit(self.take().text)
        if token.kind == "kw":
            return self.keyword_primary()
        if token.kind == "ident":
            return self.name_primary()
        if self.accept("("):
            return self.paren()
        if self.accept("["):
            return self.list_key()
        raise CompileError("E_SYNTAX", f"unexpected {token.text!r}")

    def keyword_primary(self) -> Node:
        """``true``, ``false``, ``none`` and ``if``."""
        token = self.take()
        if token.text in ("true", "false"):
            return BoolLit(token.text == "true")
        if token.text == "none":
            return NoneLit()
        if token.text == "if":
            cond = self.expr()
            self.expect("then")
            a = self.expr()
            self.expect("else")
            return If(cond, a, self.expr())
        raise CompileError("E_SYNTAX", f"unexpected {token.text!r}")

    def name_primary(self) -> Node:
        """A variable, dotted name, column read, call, relation or aggregate."""
        name = self.ident()
        if self.accept("."):
            tail = self.dotted_tail()
            if self.accept("("):
                arg = self.expr()
                self.expect(")")
                return Read(name, tail, arg)
            return Dotted(name, tail)
        if not self.at("("):
            return Var(name)
        if name in AGGREGATES and self.peek(1).kind == "ident" and self.peek(2).text == "in":
            return self.aggregate(name)
        return self.call(name)

    def call(self, name: str) -> Node:
        """The argument list of a call or relation."""
        self.expect("(")
        args: list[Node] = []
        if not self.at(")"):
            args.append(self.expr())
            while self.accept(","):
                args.append(self.expr())
        self.expect(")")
        if name in RELATIONS:
            return Rel(name, args)
        if name == "exists":
            return Exists(name, args)
        return Call(name, args)

    def aggregate(self, op: str) -> Node:
        """``agg(y in R [where c] [: e] [else d])``."""
        self.expect("(")
        var = self.ident()
        self.expect("in")
        rel = self.relation()
        where = self.expr() if self.accept("where") else None
        body = self.expr() if self.accept(":") else None
        orelse = self.expr() if self.accept("else") else None
        self.expect(")")
        return Agg(op, var, rel, where, body, orelse)

    def paren(self) -> Node:
        """A parenthesised expression or a tuple."""
        first = self.expr()
        if not self.accept(","):
            self.expect(")")
            return first
        items = [first, self.expr()]
        while self.accept(","):
            items.append(self.expr())
        self.expect(")")
        return Tuple(items)

    def list_key(self) -> Node:
        """``[ key for y in R ]``."""
        body = self.expr()
        self.expect("for")
        var = self.ident()
        self.expect("in")
        rel = self.relation()
        self.expect("]")
        return ListKey(body, var, rel)


# ----------------------------------------------------------------------------- relations
def parents(world: World, ident: int, agnatic: bool) -> list[int]:
    """The parent ids from the life-course log, father first; fathers only if ``agnatic``."""
    row = world.row("life_course", ident)
    if row is None:
        return []
    out: list[int] = []
    for column in ("father",) if agnatic else ("father", "mother"):
        parent = row.get(column)
        if isinstance(parent, Et) and parent.ident:
            out.append(parent.ident)
    return out


def children_of(world: World, ident: int) -> list[int]:
    """Every person whose mother or father is ``ident`` in the life-course log, ascending."""
    return world.children(ident)


def ancestors_or_self(world: World, ident: int, depth: int, agnatic: bool) -> set[int]:
    """Ancestors-or-self within ``depth`` generations."""
    seen = {ident}
    frontier = [ident]
    for _ in range(depth):
        frontier = [p for y in frontier for p in parents(world, y, agnatic) if p not in seen]
        seen.update(frontier)
    return seen


def by_generation(world: World, ident: int, depth: int, down: bool) -> list[int]:
    """``ancestors`` or ``descendants`` within ``depth`` generations: by generation, then id."""
    seen = {ident}
    frontier = [ident]
    out: list[int] = []
    for _ in range(depth):
        nxt: set[int] = set()
        for y in frontier:
            for z in children_of(world, y) if down else parents(world, y, False):
                if z not in seen:
                    seen.add(z)
                    nxt.add(z)
        frontier = sorted(nxt)
        out.extend(frontier)
    return out


def descends(world: World, x: int, a: int) -> bool:
    """Whether ``x`` is a descendant-or-self of ``a`` within ``DESCENT_MAX`` generations."""
    return x != 0 and a != 0 and a in ancestors_or_self(world, x, DESCENT_MAX, False)


def line_key(world: World, a: int, x: int) -> tuple[tuple[int, int], ...]:
    """The ``(birth_tick, person_id)`` key of each person on ``line(a, x)``."""
    out: list[tuple[int, int]] = []
    for y in line_between(world, a, x, False):
        row = world.row("life_course", y) or {}
        out.append((scalar_int(row.get("birth_tick", 0)), y))
    return tuple(out)


def lineage(world: World, p: int, height: int) -> list[int]:
    """The lineage of ``p``: ``(up, line)``-ordered descendants of its ancestors (spec §5.3).

    Every descendant within ``DESCENT_MAX`` generations of every ancestor-or-self of ``p`` at
    height ``0..height``, ``p`` itself excluded.
    """
    members: set[int] = set()
    level = [p]
    for _ in range(height + 1):
        for a in level:
            members.update(by_generation(world, a, DESCENT_MAX, True))
        level = [q for y in level for q in parents(world, y, False)]
    members.discard(p)
    kept = prune_dead(world, members)
    if len(kept) > LINEAGE_BOUND:
        raise InvariantError("E_TRUNCATED: a candidates relation exceeded its bound")
    keyed = []
    for x in kept:
        up, common = up_and_common(world, x, p, False)
        keyed.append(((up, line_key(world, common, x)), x))
    return [x for _, x in sorted(keyed)]


def prune_dead(world: World, members: set[int]) -> set[int]:
    """Drop the dead members with no living descendant among the members (N2, spec §5.2)."""
    kept: set[int] = set()
    for x in members:
        if world.alive(x) or any(
            z in members and world.alive(z) for z in by_generation(world, x, DESCENT_MAX, True)
        ):
            kept.add(x)
    return kept


def holders_of(world: World, title: int, n: int) -> list[int]:
    """The holders of a title's last ``n`` tenure rows, most recent first, each once (B3)."""
    rows = world.tables.get("tenure", {})
    tenures = [
        row.get("holder")
        for _, row in sorted(rows.items())
        if (t := row.get("title")) is not None and isinstance(t, Et) and t.ident == title
    ]
    out: list[int] = []
    for holder in reversed(tenures[-n:]):
        if isinstance(holder, Et) and holder.ident and holder.ident not in out:
            out.append(holder.ident)
    return out


def kin_within(world: World, ident: int, degrees: int) -> list[int]:
    """Persons within ``degrees`` parent-or-child steps, excluding ``ident``, ascending."""
    seen = {ident}
    frontier = [ident]
    for _ in range(degrees):
        nxt: list[int] = []
        for y in frontier:
            for z in parents(world, y, False) + children_of(world, y):
                if z not in seen:
                    seen.add(z)
                    nxt.append(z)
        frontier = nxt
    return sorted(seen - {ident})


def line_between(world: World, a: int, x: int, agnatic: bool) -> list[int]:
    """The shortest, father-first parent chain from ``x`` up to ``a``, listed downward."""
    if a in (x, 0) or x == 0:
        return []
    queue: deque[list[int]] = deque([[x]])
    while queue:
        path = queue.popleft()
        if len(path) > DESCENT_MAX:
            return []
        for parent in parents(world, path[-1], agnatic):
            if parent == a:
                return list(reversed(path))
            if parent not in path:
                queue.append([*path, parent])
    return []


def up_and_common(world: World, x: int, p: int, agnatic: bool) -> tuple[int, int]:
    """``up(x, p)`` and ``common(x, p)``: least height and the father-first ancestor found."""
    x_anc = ancestors_or_self(world, x, DESCENT_MAX, agnatic) if x else set()
    level = [p] if p else []
    for height in range(KIN_DEGREES_MAX + 1):
        for candidate in level:
            if candidate in x_anc:
                return height, candidate
        nxt: list[int] = []
        for y in level:
            nxt.extend(q for q in parents(world, y, agnatic) if q not in nxt)
        level = nxt
    return KIN_DEGREES_MAX + 1, 0


def accessor(world: World, name: str, x: Et, p: Et) -> Value:
    """Evaluate a genealogical accessor."""
    agnatic = name.endswith("_agnatic")
    height, common = up_and_common(world, x.ident, p.ident, agnatic)
    return height if name.startswith("up") else Et("person", common)


def rows_where(world: World, kind: str, column: str, target: int) -> list[int]:
    """Ids of ``kind`` whose entity column ``column`` references ``target``, ascending."""
    return sorted(
        ident
        for ident, row in world.tables.get(kind, {}).items()
        if isinstance(ref := row.get(column), Et) and ref.ident == target
    )


def court_of(world: World, title: int) -> list[int]:
    """Holders of the title's vassal titles and offices, ``none`` excluded, ascending."""
    holders: set[int] = set()
    for vassal in rows_where(world, "title", "liege", title):
        holder = world.tables["title"][vassal].get("holder")
        if isinstance(holder, Et):
            holders.add(holder.ident)
    for office in rows_where(world, "office", "title", title):
        holder = world.tables["office"][office].get("holder")
        if isinstance(holder, Et):
            holders.add(holder.ident)
    return sorted(holders - {0})


def relation(world: World, name: str, args: list[Value]) -> list[Et]:
    """Evaluate a prelude relation in canonical order, truncated at its bound."""
    _, yields, _, bound = RELATIONS[name]
    ids = [a.ident if isinstance(a, Et) else scalar_int(a) for a in args]
    simple = {
        "children": lambda: children_of(world, ids[0]),
        "kin": lambda: kin_within(world, ids[0], ids[1]),
        "ancestors": lambda: by_generation(world, ids[0], ids[1], False),
        "descendants": lambda: by_generation(world, ids[0], ids[1], True),
        "lineage": lambda: lineage(world, ids[0], ids[1]),
        "line": lambda: line_between(world, ids[0], ids[1], False),
        "line_agnatic": lambda: line_between(world, ids[0], ids[1], True),
        "titles": lambda: rows_where(world, "title", "holder", ids[0]),
        "vassals": lambda: rows_where(world, "title", "liege", ids[0]),
        "offices": lambda: rows_where(world, "office", "holder", ids[0]),
        "court": lambda: court_of(world, ids[0]),
        "claims_on": lambda: rows_where(world, "claim", "title", ids[0]),
        "claims_of": lambda: rows_where(world, "claim", "claimant", ids[0]),
        "vows_of": lambda: rows_where(world, "vow", "vower", ids[0]),
        "holders_of": lambda: holders_of(world, ids[0], ids[1]),
    }
    found = simple[name]() if ids[0] or name.startswith("line") else []
    if len(found) > bound:
        world.truncated += 1
        found = found[:bound]
    return [Et(yields, i) for i in found]


# ----------------------------------------------------------------------------- compiling
def compile_expr(text: str, cx: Ctx, expected: str | None = None) -> Node:
    """Parse, check and resolve an expression program; the kind's result type is a context."""
    node = Parser(text).program()
    ty = no_event(node.check(cx))
    if expected is not None:
        node.resolve(unify(ty, expected))
    else:
        node.finish()
    return node


def compile_candidates(text: str, cx: Ctx) -> Rel:
    """Parse and check a candidates program (a relation of persons)."""
    node = Parser(text).candidates()
    if node.check(cx) != "rel:person":
        raise CompileError("E_TYPE", "candidates must yield persons")
    return node


def compile_effects(text: str, cx: Ctx) -> Effects:
    """Parse and check an effect program."""
    node = Parser(text).effects()
    node.check(cx)
    return node


# ----------------------------------------------------------------------------- rendering
def render_scalar(v: Value) -> str:
    """Render a scalar in the golden file's canonical text."""
    if isinstance(v, bool):
        return f"bool:{'true' if v else 'false'}"
    if isinstance(v, int):
        return f"int:{v}"
    if isinstance(v, Fx):
        return f"{v.ty}:{v.bits}"
    if isinstance(v, En):
        return f"enum:{v.enum}.{ENUM_NAMES[v.enum][v.index]}"
    if isinstance(v, Et):
        return f"{v.kind}:{v.ident if v.ident else 'none'}"
    return render_key(v)


def render_key(v: Value) -> str:
    """Render a key: tuples in parentheses, lists in brackets."""
    if isinstance(v, tuple):
        return "(" + ", ".join(render_scalar(item) for item in v) + ")"
    if isinstance(v, list):
        return "[" + ", ".join(render_scalar(item) for item in v) + "]"
    return render_scalar(v)


def render_commands(commands: list[Cmd]) -> str:
    """Render emitted commands in queue form."""
    items = [
        f"{c.name}(person={c.person}, args=[{', '.join(str(a) for a in c.args)}])" for c in commands
    ]
    return "effects:[" + "; ".join(items) + "]"


ENUM_NAMES: dict[str, list[str]] = {}


KeyForm = tuple[tuple[int, ...], ...]
LIST_END: tuple[int, ...] = (0,)


def key_form(v: Value) -> KeyForm:
    """A flat, comparable form of a key with the spec's total order (spec §3.4).

    Scalars become ``(1, n)``; a list is its elements followed by ``(0,)``, which is below every
    element, so a list that is a prefix of another sorts first. Tuples have a fixed arity within
    one program, so flattening keeps the lexicographic order.
    """
    if isinstance(v, tuple):
        return tuple(part for item in v for part in key_form(item))
    if isinstance(v, list):
        return (*(part for item in v for part in key_form(item)), LIST_END)
    return ((1, scalar_int(v)),)


def scalar_int(v: Value) -> int:
    """The integer that orders a scalar inside a key."""
    if isinstance(v, bool):
        return int(v)
    if isinstance(v, Fx):
        return v.bits
    if isinstance(v, En):
        return v.index
    if isinstance(v, Et):
        return v.ident
    assert isinstance(v, int)
    return v


# ----------------------------------------------------------------------------- cases
@dataclass
class Golden:
    """The loaded golden file."""

    reg: Registry
    worlds: dict[str, dict[str, Any]]
    cases: list[dict[str, Any]]


def load_golden(path: Path) -> Golden:
    """Load and validate the golden file's header."""
    raw = tomllib.loads(path.read_text(encoding="utf-8"))
    if raw.get("version") != LANGUAGE_VERSION:
        raise ValueError(f"golden file version {raw.get('version')} != {LANGUAGE_VERSION}")
    reg = load_registry(raw["registry"])
    ENUM_NAMES.clear()
    ENUM_NAMES.update(reg.enums)
    return Golden(reg, raw["worlds"], raw["case"])


def parse_binding(text: object) -> tuple[str, Value]:
    """A case binding ``kind:id``, an int or a bool, with its type."""
    if isinstance(text, bool):
        return "bool", text
    if isinstance(text, int):
        return "int", text
    kind, ident = str(text).split(":")
    return f"ent:{kind}", Et(kind, int(ident))


def system_for_case(reg: Registry, case: dict[str, Any]) -> str:
    """The declaring system, with optional per-case extra reads or sends (a variant system)."""
    name = str(case["system"])
    extra_reads = case.get("extra_reads")
    extra_sends = case.get("extra_sends")
    if not extra_reads and not extra_sends:
        return name
    base = reg.systems[name]
    variant = System(
        reads=base.reads | set(extra_reads or {}),
        sends={**base.sends, **{s["command"]: s["target"] for s in extra_sends or []}},
        rng=base.rng,
        params=base.params,
        neutral={**base.neutral, **dict(extra_reads or {})},  # column -> its neutral value
    )
    variant_name = f"{name}@{case['id']}"
    reg.systems[variant_name] = variant
    return variant_name


@dataclass
class Run:
    """One case's compile-time and run-time contexts."""

    cx: Ctx
    ev: Ev

    def fresh(self) -> None:
        """A new compile-time context for a repeated evaluation (the site lint is per compile)."""
        old = self.cx
        self.cx = Ctx(old.reg, old.system_name, dict(old.scope), old.allow_key)
        self.cx.outcome_payload = old.outcome_payload


def prepare(golden: Golden, case: dict[str, Any]) -> Run:
    """Build the compile-time and run-time contexts of a case."""
    reg = golden.reg
    world = load_world(reg, golden.worlds[str(case.get("world", "dynasty"))])
    system = system_for_case(reg, case)
    scope: dict[str, str] = {"now": "int"}
    values: dict[str, Value] = {"now": world.now}
    if case["kind"] == "effects":  # the verb's own event (§7.1), rewritten at the merge (N3)
        scope["event"], values["event"] = "event", EventRef(own=True)
    for name, raw in case.get("bind", {}).items():
        scope[name], values[name] = parse_binding(raw)
    payload = case.get(
        "payload", {}
    )  # the verb's event payload fields (§7.1): bound like variables
    for name, raw in payload.items():
        scope[name], values[name] = parse_binding(raw)
    cx = Ctx(reg, system, scope, allow_key=case["kind"] == "order", outcome_payload=bool(payload))
    # Parameters are read as the system they belong to, so a variant system shares them.
    base = system.split("@")[0]
    ev = Ev(reg, world, base, values)
    return Run(cx, ev)


def evaluate(node: Node | Effects, ev: Ev) -> Value | list[Cmd]:
    """One evaluation of one program for one binding: the draw ledger of §4.6 starts empty (N6)."""
    ev.world.drawn = set()
    return node.eval(ev)


def run_expr(run: Run, case: dict[str, Any]) -> str:
    """Evaluate an ``expr`` or ``order`` case."""
    wanted = case.get("result")
    node = compile_expr(str(case["text"]), run.cx, str(wanted) if wanted else None)
    value = evaluate(node, run.ev)
    assert not isinstance(value, list) or case["kind"] == "order"
    if case["kind"] == "order":
        return "key:" + render_key(value)
    return render_scalar(value)


def run_candidates(run: Run, case: dict[str, Any]) -> str:
    """Evaluate a ``candidates`` case."""
    node = compile_candidates(str(case["text"]), run.cx)
    elements = node.eval(run.ev)
    assert isinstance(elements, list)
    return "relation:[" + ", ".join(str(e.ident) for e in elements) + "]"


def run_effects(run: Run, case: dict[str, Any]) -> str:
    """Evaluate an ``effects`` case."""
    node = compile_effects(str(case["text"]), run.cx)
    commands = evaluate(node, run.ev)
    assert isinstance(commands, list)
    return render_commands(commands)


def succession_line(run: Run, programs: dict[str, str]) -> list[int]:
    """The succession protocol of spec §7.2 for one law."""
    cands = compile_candidates(programs["candidates"], run.cx)
    run.cx.bind("x", "ent:person", loop=True)
    eligible = compile_expr(programs["eligible"], run.cx, "bool")
    run.cx.allow_key = True
    order = compile_expr(programs["order"], run.cx)
    run.cx.unbind("x")
    keyed: list[tuple[KeyForm, int]] = []
    elements = cands.eval(run.ev)
    assert isinstance(elements, list)
    for x in elements:
        run.ev.vars["x"] = x
        if evaluate(eligible, run.ev):
            keyed.append((key_form(evaluate(order, run.ev)), x.ident))
    run.ev.vars.pop("x", None)
    return [ident for _, ident in sorted(keyed)]


def run_succession(run: Run, case: dict[str, Any], golden: Golden) -> str:
    """Evaluate a ``succession`` case: the law's line for the bound holder and title."""
    law = str(case["law"])
    programs = golden.reg.programs["succession_laws"][law]
    return "line:[" + ", ".join(str(i) for i in succession_line(run, programs)) + "]"


RUNNERS = {
    "expr": run_expr,
    "order": run_expr,
    "candidates": run_candidates,
    "effects": run_effects,
}


DIAGNOSTICS = ("missing_reads", "dropped_actions", "neutral_reads", "truncated")


def run_case(golden: Golden, case: dict[str, Any]) -> tuple[str, dict[str, int]]:
    """Run one case; return the rendered result (or ``error:CODE``) and the diagnostics."""
    run = prepare(golden, case)
    try:
        texts: list[str] = []
        for _ in range(int(case.get("evaluations", 1))):  # N6: repeated in one step, same result
            run.fresh()
            if case["kind"] == "succession":
                texts.append(run_succession(run, case, golden))
            else:
                texts.append(RUNNERS[str(case["kind"])](run, case))
        text = texts[0] if len(set(texts)) == 1 else "error:NOT_IDEMPOTENT"
    except CompileError as error:
        return f"error:{error.code}", {}
    except InvariantError as error:
        return f"invariant:{str(error).split(':')[0]}", {}
    return text, {name: getattr(run.ev.world, name) for name in DIAGNOSTICS}


def check_all(golden: Golden, verbose: bool, only: str | None) -> int:
    """Run every case (or one) and report; return the number of failures."""
    failures = 0
    seen: set[str] = set()
    for case in golden.cases:
        ident = str(case["id"])
        if only is not None and ident != only:
            continue
        if ident in seen:
            print(f"FAIL {ident}: duplicate case id")
            failures += 1
        seen.add(ident)
        failures += report_case(golden, case, verbose)
    return failures


def report_case(golden: Golden, case: dict[str, Any], verbose: bool) -> int:
    """Run and compare one case; print a line; return 1 on failure."""
    ident = str(case["id"])
    try:
        got, diags = run_case(golden, case)
    except Exception as error:  # a crash is a failed case, reported by name, never a traceback
        print(f"FAIL {ident}: crashed: {type(error).__name__}: {error}")
        return 1
    expected = str(case["expect"])
    ok = got == expected
    for name in DIAGNOSTICS:
        if name in case:
            ok = ok and int(case[name]) == diags.get(name, -1)
    if verbose or not ok:
        print(f"{'ok  ' if ok else 'FAIL'} {ident}: {case['text']!r}")
        print(f"      expected {expected}  got {got}  diagnostics {diags}")
    return 0 if ok else 1


def main(argv: list[str]) -> int:
    """Command line."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--check", action="store_true", help="exit 1 on any mismatch")
    parser.add_argument("--case", help="run one case by id, verbosely")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the golden file")
    parser.add_argument("-v", "--verbose", action="store_true")
    args = parser.parse_args(argv)
    golden = load_golden(args.golden)
    failures = check_all(golden, args.verbose or args.case is not None, args.case)
    total = len(golden.cases) if args.case is None else 1
    if failures:
        print(f"EFFECT LANGUAGE v{LANGUAGE_VERSION}: FAIL ({failures} of {total} cases)")
        return 1
    print(f"EFFECT LANGUAGE v{LANGUAGE_VERSION}: PASS ({total} cases reproduce {args.golden.name})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
