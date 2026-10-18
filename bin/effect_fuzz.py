"""Differential fuzz of the effect language: effect_ref.py against Rust (SPR-017 T-08).

For each seed (SplitMix64, given on the command line) the driver writes one golden-format file
under the output directory: the golden file's `[registry]` slice, generated worlds and generated
cases. It runs `effect_ref.py --emit` and `rh-cli effect eval` on that file and compares, case by
case, the result text and the four diagnostics (`missing_reads`, `dropped_actions`,
`neutral_reads`, `truncated`). The code of a rejection is compared exactly on well-typed programs
and on single-fault mutants of them, and only as accept or reject on random character mutants
(multi-fault: the first fault the two front ends meet may differ, effect_language_v0.md §10).

Worlds: life-course logs whose row ids differ from the person ids, unknown parents, twins, the
dead and compacted, half-siblings, cousin and uncle-niece unions, a line of eight or more
generations, tenure logs of 1 to 20 rows per title (rotas included), titles, offices, claims, vows
and artifacts; and a regular fan tree of depth 3 or 4 with fanout 4 to 9, large on every even
seed (over the lineage bound after pruning, or before pruning only). Draws come from each world's
`draw_seed`.

Programs: type-directed over the golden `[registry]` slice, for every case kind (expr, order,
candidates, effects, succession with generated laws), depth 1 to 8: draws at fresh sites keyed on
their loop variable, products on rounding ties, reads of absent owners through a variant system's
neutral, repeated evaluations in one step; plus single-fault mutants aimed at each of the 16
codes, random character mutants, and three probes per seed (the lineage bound, a relation over
its bound, a run-time draw reuse).

On a mismatch the driver prints the seed, the case, the program and both results, and writes a
reduced repro (the one case, in the seed's slice and worlds) under the output directory. Coverage
counters (every code, both run-time invariants, every relation, builtin and aggregate, every case
kind, `truncated_relations > 0`) are printed, and asserted non-zero with `--assert-coverage`.

Usage:
  python -I bin/effect_fuzz.py --seeds 1-16 --rust-bin target/release/rh-cli
      [--cases N] [--out target/fuzz] [--ref specs/registry/effect_ref.py] [--rust-mutant NAME]
      [--assert-coverage] [--write-only]
Exit 0 when every case agrees (and coverage holds, if asserted), 1 on a mismatch or a crash, 2 on
bad arguments. Standard library only (AGT-002-TOOLS).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
import tomllib
from collections.abc import Callable, Sequence
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any, TypeVar

ROOT = Path(__file__).resolve().parents[1]
GOLDEN = ROOT / "specs" / "registry" / "effect_language_golden_v0.toml"
REFERENCE = ROOT / "specs" / "registry" / "effect_ref.py"
MASK = (1 << 64) - 1
I63 = (1 << 63) - 1
I64_MAX = (1 << 63) - 1
YEAR = 360 * 1440
DIAGNOSTICS = ("missing_reads", "dropped_actions", "neutral_reads", "truncated")
CODES = (
    "E_SYNTAX",
    "E_FLOAT_LITERAL",
    "E_LITERAL_RANGE",
    "E_UNKNOWN_NAME",
    "E_TYPE",
    "E_ARITY",
    "E_CONST",
    "E_DIV",
    "E_BOUND",
    "E_PLANE",
    "E_UNDECLARED_READ",
    "E_NO_NEUTRAL",
    "E_UNDECLARED_COMMAND",
    "E_COMMAND_OWNER",
    "E_RNG_PURPOSE",
    "E_RNG_REUSE",
)
INVARIANTS = ("E_TRUNCATED", "E_RNG_REUSE")
KINDS = ("expr", "order", "candidates", "effects", "succession")
# name: (argument types, element kind); `degN` is a degree constant in 1..N.
RELATIONS = {
    "children": (("ent:person",), "person"),
    "kin": (("ent:person", "deg6"), "person"),
    "ancestors": (("ent:person", "deg6"), "person"),
    "descendants": (("ent:person", "deg6"), "person"),
    "lineage": (("ent:person", "deg3"), "person"),
    "line": (("ent:person", "ent:person"), "person"),
    "line_agnatic": (("ent:person", "ent:person"), "person"),
    "court": (("ent:title",), "person"),
    "holders_of": (("ent:title", "deg16"), "person"),
    "titles": (("ent:person",), "title"),
    "vassals": (("ent:title",), "title"),
    "offices": (("ent:person",), "office"),
    "claims_on": (("ent:title",), "claim"),
    "claims_of": (("ent:person",), "claim"),
    "vows_of": (("ent:person",), "vow"),
}
GENEALOGY = ("life_course.person_id", "life_course.mother", "life_course.father")
AGNATIC = ("life_course.person_id", "life_course.father")
# The columns each relation or kinship builtin reads (§5.3): offered to a system only when it may
# read them all, so most programs pass E_UNDECLARED_READ.
NEEDS = {
    "children": GENEALOGY,
    "kin": GENEALOGY,
    "ancestors": GENEALOGY,
    "descendants": GENEALOGY,
    "line": GENEALOGY,
    "line_agnatic": AGNATIC,
    "lineage": (*GENEALOGY, "life_course.birth_tick", "person.alive"),
    "titles": ("title.holder",),
    "vassals": ("title.liege",),
    "offices": ("office.holder",),
    "court": ("title.liege", "title.holder", "office.title", "office.holder"),
    "claims_on": ("claim.title",),
    "claims_of": ("claim.claimant",),
    "vows_of": ("vow.vower",),
    "holders_of": ("tenure.title", "tenure.holder"),
    "up": GENEALOGY,
    "common": GENEALOGY,
    "descends": GENEALOGY,
    "related": GENEALOGY,
    "up_agnatic": AGNATIC,
    "common_agnatic": AGNATIC,
}
BUILTINS = (
    "abs",
    "min",
    "max",
    "clamp",
    "safe_div",
    "widen",
    "narrow",
    "q8",
    "q16",
    "floor",
    "round",
    "ordinal",
    "exists",
    "draw",
    "chance",
    "related",
    "up",
    "common",
    "up_agnatic",
    "common_agnatic",
    "descends",
)
AGGREGATES = ("count", "any", "all", "sum", "min", "max")
INT_LITERALS = ("0", "1", "2", "3", "7", "-5", "100", "9223372036854775807", "4611686018427387904")
DECIMALS = ("0.5", "1.25", "-2.5", "0.00390625", "0.001953125", "127.99609375", "-0.75", "3.0")
# Odd multiples of a type's last bit: times 0.5 (or 1.5) their product is exactly a rounding tie.
ODD_LAST_BITS = {
    "q8_8": ("0.00390625", "-0.01171875", "0.98828125"),
    "q16_16": ("0.0000152587890625", "-0.0000457763671875", "3.0000152587890625"),
}
FIXED = ("q8_8", "q16_16")
NUMERIC = ("int", *FIXED)
CHAR_ALPHABET = (
    "()+-*/<>=,.:;0123456789abcxyz " + chr(0xA0) + chr(0xE9)
)  # NBSP: not whitespace (F-1)
CASES_IN_ROTATION = 64  # the first cases cycle through every kind
DEPTH_MAX = 8
FAN_DEEP = 3  # the fan depth whose lineage reaches the root at height 3
DEEP_LINE, DEEP_LINE_WORLD = 9, 24  # a line of nine fathers in every world of 24 or more persons
FAULTS, CHARS, NEUTRAL = 3, 5, 9  # expression rolls in 0..9: < 3 a fault, < 5 characters, 9 neutral
T = TypeVar("T")
Result = tuple[str, dict[str, int]]
Meta = dict[str, dict[str, Any]]


def splitmix64(z: int) -> int:
    """SplitMix64's step and finaliser (the Rust side's `effect_cases::splitmix64`)."""
    z = (z + 0x9E3779B97F4A7C15) & MASK
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
    return z ^ (z >> 31)


class Rng:
    """A SplitMix64 stream."""

    def __init__(self, seed: int) -> None:
        """A stream from `seed`."""
        self.state = seed & MASK

    def next(self) -> int:
        """The next 64-bit value."""
        self.state = splitmix64(self.state)
        return self.state

    def below(self, n: int) -> int:
        """A value in 0..n-1 (n > 0)."""
        return self.next() % n

    def pick(self, xs: Sequence[T]) -> T:
        """One element."""
        return xs[self.below(len(xs))]

    def chance(self, num: int, den: int) -> bool:
        """True with probability num/den."""
        return self.below(den) < num

    def shuffled(self, xs: list[T]) -> list[T]:
        """A Fisher-Yates shuffle of a copy."""
        out = list(xs)
        for i in range(len(out) - 1, 0, -1):
            j = self.below(i + 1)
            out[i], out[j] = out[j], out[i]
        return out


# ------------------------------------------------------------------------------------- the slice
class Slice:
    """The golden file's `[registry]` slice: its text and its tables."""

    def __init__(self, golden_text: str) -> None:
        """The slice of a golden file's text."""
        lines = golden_text.splitlines(keepends=True)
        start = next(i for i, ln in enumerate(lines) if ln.strip() == "[registry]")
        end = next(i for i, ln in enumerate(lines) if ln.startswith(("[worlds.", "[[worlds.")))
        self.text = "".join(lines[start:end])
        data = tomllib.loads(golden_text)
        self.reg: dict[str, Any] = data["registry"]
        self.header = (
            f'language = "{data["language"]}"\nversion = "{data["version"]}"\n'
            f'spec = "{data["spec"]}"\n\n'
        )
        self.dynasty_params: dict[str, dict[str, Any]] = data["worlds"]["dynasty"]["params"]
        self.enums: dict[str, list[str]] = {n: e["values"] for n, e in self.reg["enums"].items()}
        self.columns: dict[str, dict[str, Any]] = self.reg["columns"]
        self.systems: dict[str, dict[str, Any]] = self.reg["systems"]
        self.commands: dict[str, dict[str, Any]] = self.reg["commands"]

    def column_type(self, name: str) -> str:
        """A column's language type."""
        c = self.columns[name]
        if c["type"] == "ref":
            return f"ent:{c['ref']}"
        if c["type"] == "enum":
            return f"enum:{c['enum']}"
        return str(c["type"])

    def readable(self, system: str) -> list[str]:
        """The plane-A columns `system` may read: its own and its declared reads."""
        own = {n for n, c in self.columns.items() if c["owner"] == system}
        cols = own | set(self.systems[system].get("reads", []))
        return sorted(c for c in cols if c in self.columns and self.columns[c]["plane"] == "A")

    def params(self, system: str) -> dict[str, str]:
        """`system`'s parameters and their types (int when both bounds are integers)."""
        return {
            name: "int" if isinstance(lo, int) and isinstance(hi, int) else "q16_16"
            for name, (lo, hi) in self.systems[system].get("params", {}).items()
        }


# ---------------------------------------------------------------------------------------- worlds
def toml_value(v: object) -> str:
    """A TOML literal: a boolean, an integer, or a string."""
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    return json.dumps(str(v))


def table_rows(name: str, kind: str, rows: list[dict[str, Any]]) -> str:
    """`[[worlds.<name>.<kind>]]` rows."""
    out = []
    for row in rows:
        out.append(f"[[worlds.{name}.{kind}]]\n")
        out.extend(f"{k} = {toml_value(v)}\n" for k, v in row.items())
    return "".join(out)


def world_header(sl: Slice, name: str, seed: int) -> str:
    """A world's scalars, parameters and draw seed."""
    out = [f"[worlds.{name}]\nnow = 0\ndays_per_year = 360\ndraw_seed = {seed & I63}\n"]
    for system, values in sl.dynasty_params.items():
        out.append(f'[worlds.{name}.params."{system}"]\n')
        out.extend(f"{k} = {toml_value(v)}\n" for k, v in values.items())
    return "".join(out)


def gen_people(rng: Rng, n: int) -> list[dict[str, Any]]:
    """Persons with genealogy.

    Founders, a line of fathers at least eight generations deep, then children of earlier persons
    (unknown parents, twins, half-siblings), and an uncle-niece union.
    """
    people: list[dict[str, Any]] = []
    pids = rng.shuffled([2 + 3 * i for i in range(n)])

    def add(mother: int, father: int, sex: str, gen: int, twin: dict[str, Any] | None) -> None:
        birth = twin["birth_tick"] if twin else -(12 - gen) * 25 * YEAR + rng.below(400) * 1440
        people.append(
            {
                "person_id": pids[len(people)],
                "mother": mother,
                "father": father,
                "sex": sex,
                "birth_tick": birth,
                "gen": gen,
            }
        )

    for i in range(max(2, n // 6)):
        add(0, 0, ("male", "female")[i % 2], 0, None)
    for _ in range(min(DEEP_LINE, n - len(people)) if n >= DEEP_LINE_WORLD else 0):
        prev = people[-1] if people[-1]["sex"] == "male" else people[0]
        add(0, prev["person_id"], "male", prev["gen"] + 1, None)
    while len(people) < n:
        if rng.chance(1, 10):  # a twin of the last-born
            last = people[-1]
            add(last["mother"], last["father"], rng.pick(("male", "female")), last["gen"], last)
            continue
        males = [p for p in people if p["sex"] == "male"]
        females = [p for p in people if p["sex"] == "female"]
        father = rng.pick(males) if males and not rng.chance(1, 7) else None
        mother = rng.pick(females) if females and not rng.chance(1, 7) else None
        gen = 1 + max([p["gen"] for p in (father, mother) if p] or [0])
        add(
            mother["person_id"] if mother else 0,
            father["person_id"] if father else 0,
            rng.pick(("male", "female")),
            min(gen, 11),
            None,
        )
    uncle_niece(people)
    return people


def uncle_niece(people: list[dict[str, Any]]) -> None:
    """Makes the last-born the child of a niece and her father's brother."""
    by_id = {p["person_id"]: p for p in people}
    for niece in people:
        parent = by_id.get(niece["father"])
        if niece["sex"] != "female" or parent is None or not parent["father"]:
            continue
        uncles = [
            p
            for p in people
            if p["sex"] == "male" and p is not parent and p["father"] == parent["father"]
        ]
        if uncles and people[-1] is not niece and people[-1] is not uncles[0]:
            child = people[-1]
            child["mother"], child["father"] = niece["person_id"], uncles[0]["person_id"]
            child["birth_tick"] = max(niece["birth_tick"], uncles[0]["birth_tick"]) + 20 * YEAR
            return


def gen_ledger(
    sl: Slice, rng: Rng, people: list[dict[str, Any]]
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """The person ledger and the life-course log.

    The ledger holds the living and some dead of the instant; the log's row ids differ from the
    person ids; a fifth of the persons are dead and compacted (in the log only).
    """
    lc_ids = rng.shuffled([5000 + 11 * i for i in range(len(people))])
    ledger, log = [], []
    for row_id, p in zip(lc_ids, people, strict=True):
        compacted = rng.chance(1, 5)
        cause = rng.pick(sl.enums["death_cause"]) if compacted else "none"
        log.append(
            {
                "id": row_id,
                "person_id": p["person_id"],
                "mother": p["mother"],
                "father": p["father"],
                "birth_tick": p["birth_tick"],
                "sex": p["sex"],
                "death_cause": cause,
            }
        )
        if compacted:
            continue
        ledger.append(
            {
                "id": p["person_id"],
                "alive": not rng.chance(1, 15),
                "sex": p["sex"],
                "birth_tick": p["birth_tick"],
                "death_tick": 0,
                "death_cause": "none",
                "mother": p["mother"],
                "father": p["father"],
                "spouse": 0,
                "house": 0,
                "vocation": rng.pick(sl.enums["vocations"]),
                "named": rng.chance(1, 2),
                "residence": rng.pick((0, 1)),
            }
        )
    living = [r["id"] for r in ledger]
    for row in ledger:  # spouses, cousins and kin among them
        if living and rng.chance(1, 3):
            row["spouse"] = rng.pick(living)
    return ledger, log


def gen_titles(
    sl: Slice, rng: Rng, n: int, anyone: Callable[[], int], all_ids: list[int]
) -> tuple[list[int], dict[str, list[dict[str, Any]]]]:
    """Titles (a liege forest), their tenure logs of 1 to 20 rows (rotas included) and offices."""
    title_ids = [100 + 7 * i for i in range(max(3, n // 8))]
    titles = [
        {
            "id": t,
            "tier": rng.pick(sl.enums["title_tiers"]),
            "liege": rng.pick(title_ids[:i]) if i and rng.chance(2, 3) else 0,
            "polity": rng.pick((0, 1)),
            "law": rng.pick(sl.enums["succession_laws"]),
            "holder": anyone(),
            "state": rng.pick(sl.enums["title_state"]),
            "held_since": -rng.below(40) * YEAR,
            "regent": anyone() if rng.chance(1, 5) else 0,
        }
        for i, t in enumerate(title_ids)
    ]
    tenure: list[dict[str, Any]] = []
    for t in title_ids:
        rows = 1 + rng.below(20)
        rota = (rng.pick(all_ids), rng.pick(all_ids)) if rng.chance(1, 4) else None
        for r in range(rows):
            end = -(rows - r - 1) * YEAR if r + 1 < rows else I64_MAX
            tenure.append(
                {
                    "id": 600 + len(tenure),
                    "title": t,
                    "holder": rota[r % 2] if rota else anyone(),
                    "start": -(rows - r) * YEAR,
                    "end": end,
                }
            )
    offices: list[dict[str, Any]] = []
    for t in title_ids:
        for _ in range(rng.below(4)):
            kind = rng.pick(sl.enums["office_kinds"])
            offices.append({"id": 200 + len(offices), "title": t, "kind": kind, "holder": anyone()})
    return title_ids, {"title": titles, "tenure": tenure, "office": offices}


def gen_world(sl: Slice, rng: Rng, name: str, n: int) -> tuple[str, dict[str, list[int]]]:
    """A listed world of `n` persons, and its ids by kind (for bindings)."""
    people = gen_people(rng, n)
    ledger, log = gen_ledger(sl, rng, people)
    all_ids = [p["person_id"] for p in people]
    living = [r["id"] for r in ledger] or all_ids

    def anyone() -> int:
        return rng.pick(all_ids) if not rng.chance(1, 8) else 0

    title_ids, tables = gen_titles(sl, rng, n, anyone, all_ids)
    claims = [
        {
            "id": 300 + i,
            "claimant": anyone(),
            "title": rng.pick(title_ids),
            "status": rng.pick(sl.enums["claim_status"]),
            "pressed": rng.chance(1, 2),
        }
        for i in range(max(1, n // 6))
    ]
    vows = [
        {
            "id": 400 + i,
            "vower": anyone(),
            "kind": rng.pick(sl.enums["vow_kinds"]),
            "target": rng.below(30),
            "status": rng.pick(sl.enums["vow_status"]),
            "priority": rng.pick(("0.5", "0.25", "0.75")),
            "conflict": rng.pick(("0.0", "0.5", "1.0")),
        }
        for i in range(max(1, n // 5))
    ]
    artifacts = [
        {
            "id": 500 + i,
            "bearer": anyone(),
            "state": rng.pick(sl.enums["artifact_state"]),
            "location": 1,
            "exposure": rng.pick(("0.25", "1.5")),
            "charge": rng.pick(("0.5", "-0.5")),
        }
        for i in range(2)
    ]
    minds = [
        {
            "id": p,
            "stress": rng.pick(("10.0", "80.5")),
            "conscientiousness": rng.pick(("0.5", "-0.25")),
            "honesty_humility": rng.pick(("0.0", "0.75")),
        }
        for p in living[: max(1, len(living) // 3)]
    ]
    region = {"id": 1, "conditions": rng.pick(sl.enums["region_conditions"])}
    text = world_header(sl, name, rng.next())
    for kind, rows in (
        ("person", ledger),
        ("life_course", log),
        ("title", tables["title"]),
        ("tenure", tables["tenure"]),
        ("office", tables["office"]),
        ("claim", claims),
        ("vow", vows),
        ("artifact", artifacts),
        ("mind", minds),
        ("polity", [{"id": 1, "exec_constraint": "0.25"}]),
        ("region", [region]),
    ):
        text += table_rows(name, kind, rows)
    ids = {
        "person": all_ids or [0],
        "title": title_ids,
        "office": [o["id"] for o in tables["office"]] or [0],
        "claim": [c["id"] for c in claims],
        "vow": [v["id"] for v in vows],
        "artifact": [a["id"] for a in artifacts],
    }
    return text, ids


def gen_fan(sl: Slice, rng: Rng, name: str, seed: int) -> tuple[str, dict[str, list[int]]]:
    """A regular fan tree (the golden `fan` generator): depth 3 or 4, fanout 4 to 9.

    An even seed's is large (depth 4, fanout 8 or 9): with its leaves alive when the seed is a
    multiple of 4, so a lineage from depth 3 is over 4096 after pruning (E_TRUNCATED); with dead
    leaves otherwise, so it is over 4096 before pruning only, and the bound must apply after it
    (§5.2).
    """
    depth, fanout, dead = rng.pick((3, 4)), 4 + rng.below(6), rng.chance(1, 2)
    if seed % 2 == 0:
        depth, fanout, dead = 4, 8 + rng.below(2), seed % 4 != 0
    total = sum(fanout**g for g in range(depth + 1))
    firsts = [1 + sum(fanout**g for g in range(d)) for d in range(depth + 1)]
    text = world_header(sl, name, rng.next())
    text += f"[worlds.{name}.fan]\nfanout = {fanout}\ndepth = {depth}\n"
    text += f"dead_leaves = {toml_value(dead)}\n"
    ids = {
        "person": [*firsts, firsts[-1] - 1, total],
        "deep": [firsts[min(FAN_DEEP, depth)]],
        "title": [0],
        "office": [0],
        "claim": [0],
        "vow": [0],
        "artifact": [0],
    }
    return text, ids


# -------------------------------------------------------------------------------------- programs
class Gen:
    """A type-directed program generator over the slice, for one system and scope."""

    def __init__(self, rng: Rng, sl: Slice, system: str, scope: dict[str, str]) -> None:
        """A generator for `system` with the bound variables of `scope` (name to type)."""
        self.rng, self.sl, self.system = rng, sl, system
        self.scope = dict(scope)
        self.readable = sl.readable(system)
        self.params = sl.params(system)
        self.purposes = list(sl.systems[system].get("rng", []))
        self.fresh = 0
        self.loops: list[str] = []  # the loop variables in scope, innermost last
        self.sites: set[str] = set()
        self.k = 0
        self.verb = False  # an effects program: its draws key on actor or target (r4-2)
        visible = set(self.readable)
        self.can = {n for n, need in NEEDS.items() if all(c in visible for c in need)}

    def ok(self, name: str) -> bool:
        """Whether the system may use a relation or kinship builtin."""
        return name in self.can

    def var(self, ty: str) -> str | None:
        """A bound variable of type `ty`."""
        names = [n for n, t in self.scope.items() if t == ty]
        return self.rng.pick(names) if names else None

    def columns_of(self, ty: str, entity: str | None = None) -> list[str]:
        """The readable columns of type `ty` (of `entity`)."""
        return [
            c
            for c in self.readable
            if self.sl.column_type(c) == ty
            and (entity is None or self.sl.columns[c]["entity"] == entity)
        ]

    def draw_call(self, name: str) -> str | None:
        """A `draw` or `chance` at a new static site, or none.

        In a loop it is keyed on the loop variable (§4.6, minor 3), in a verb program on actor or
        target (r4-2); under two loops there is none, since one key cannot name both.
        """
        entities = (
            ["actor", "target"]
            if self.verb
            else [n for n, t in self.scope.items() if t == "ent:person"]
        )
        if not self.purposes or len(self.loops) > 1 or not entities:
            return None
        for _ in range(8):
            purpose, entity = self.rng.pick(self.purposes), self.rng.pick(entities)
            if self.loops:
                k = self.loops[-1]
            else:
                self.k += 1
                k = str(self.k)
            site = f"{purpose}/{entity}/{k}"
            if site in self.sites:
                continue
            self.sites.add(site)
            if name == "draw":
                return f"(draw({purpose}, {entity}, {k}) mod 1000)"
            return f"chance({purpose}, {entity}, {k}, {self.leaf('q16_16')})"
        return None

    def entity_arg(self, kind: str, depth: int, typed: bool = False) -> str:
        """An entity of `kind`; `none` only where the context gives it its kind (not `typed`)."""
        r = self.rng
        v = self.var(f"ent:{kind}")
        if v is not None and (depth <= 0 or r.chance(3, 4)):
            return v
        if depth > 0 and r.chance(1, 2):
            cols = [
                c
                for c in self.columns_of(f"ent:{kind}")
                if self.var(f"ent:{self.sl.columns[c]['entity']}")
            ]
            if cols:
                col = r.pick(cols)
                return f"{col}({self.entity_arg(self.sl.columns[col]['entity'], depth - 1)})"
        kin = [f for f in ("common", "common_agnatic") if self.ok(f)]
        if kind == "person" and depth > 0 and kin and r.chance(1, 3):
            a, b = self.entity_arg("person", depth - 1), self.entity_arg("person", depth - 1)
            return f"{r.pick(kin)}({a}, {b})"
        if v is not None:
            return v
        person = self.var("ent:person")
        cols = self.columns_of(f"ent:{kind}", "person")
        if typed and person and cols:
            return f"{cols[0]}({person})"
        return "none"

    def read(self, ty: str, depth: int) -> str | None:
        """A declared read of a column of type `ty`."""
        kinds = {t.split(":")[1] for t in self.scope.values() if t.startswith("ent:")} | {"person"}
        cols = [c for c in self.columns_of(ty) if self.sl.columns[c]["entity"] in kinds]
        if not cols:
            return None
        col = self.rng.pick(cols)
        key = self.sl.columns[col].get("key", self.sl.columns[col]["entity"])
        return f"{col}({self.entity_arg(key, depth - 1)})"

    def leaf(self, ty: str) -> str:
        """A literal, a variable, a parameter or a calendar constant of type `ty`."""
        r = self.rng
        if ty == "bool":
            return self.var("bool") or r.pick(("true", "false"))
        if ty == "int":
            choices = [r.pick(INT_LITERALS), "now", "calendar.days_per_year"]
            choices += ["calendar.minutes_per_day"]
            choices += [f"params.{p}" for p, t in self.params.items() if t == "int"]
            v = self.var("int")
            return v if v and r.chance(1, 3) else r.pick(choices)
        if ty in FIXED:
            conv = "q8" if ty == "q8_8" else "q16"
            choices = [r.pick(DECIMALS), f"{conv}({r.pick(('0', '1', '-3', '200'))})"]
            if ty == "q16_16":
                choices += [f"params.{p}" for p, t in self.params.items() if t == "q16_16"]
            return r.pick(choices)
        if ty.startswith("enum:"):
            name = ty.split(":")[1]
            return f"{name}.{r.pick(self.sl.enums[name])}"
        return self.entity_arg(ty.split(":")[1], 0)

    def relation(self, kind: str, depth: int) -> str:
        """A prelude relation of elements of `kind`."""
        names = [n for n, (_, k) in RELATIONS.items() if k == kind and self.ok(n)]
        name = self.rng.pick(names or [n for n, (_, k) in RELATIONS.items() if k == kind])
        args = [
            str(1 + self.rng.below(int(a[3:])))
            if a.startswith("deg")
            else self.entity_arg(a.split(":")[1], depth - 1)
            for a in RELATIONS[name][0]
        ]
        return f"{name}({', '.join(args)})"

    def loop(self, kind: str) -> str:
        """Binds a fresh loop variable of `kind` (`unloop` releases it)."""
        self.fresh += 1
        name = f"y{self.fresh}"
        self.scope[name] = f"ent:{kind}"
        self.loops.append(name)
        return name

    def unloop(self, name: str) -> None:
        """Releases the innermost loop variable."""
        del self.scope[name]
        self.loops.pop()

    def aggregate(self, ty: str, depth: int) -> str:
        """An aggregate of type `ty` over a relation (§5.4)."""
        r = self.rng
        kind = r.pick(("person", "person", "title", "office", "claim", "vow"))
        rel = self.relation(kind, depth)
        y = self.loop(kind)
        where = f" where {self.expr('bool', depth - 1)}" if r.chance(1, 2) else ""
        if ty == "bool":
            f = r.pick(("any", "all"))
            out = (
                f"{f}({y} in {rel}{where})"
                if where
                else f"{f}({y} in {rel}: {self.expr('bool', depth - 1)})"
            )
        elif ty == "int" and r.chance(1, 2):
            out = f"count({y} in {rel}{where})"
        else:
            f = r.pick(("sum", "min", "max"))
            tail = f" else {self.leaf(ty)}" if f != "sum" else ""
            out = f"{f}({y} in {rel}{where}: {self.expr(ty, depth - 1)}{tail})"
        self.unloop(y)
        return out

    # The productions of each type, tried in random order until one applies.
    def bool_productions(self, d: int) -> list[Callable[[], str | None]]:
        """The `bool` productions at depth `d`."""
        r = self.rng

        def compare() -> str:
            t = r.pick(NUMERIC)
            return f"({self.expr(t, d)} {r.pick(('<', '<=', '>', '>='))} {self.expr(t, d)})"

        def equal() -> str:
            t = r.pick(("int", "bool", "ent:person", "enum:sex", "enum:claim_status"))
            left = self.entity_arg("person", d, True) if t == "ent:person" else self.expr(t, d)
            return f"({left} {r.pick(('==', '!='))} {self.expr(t, d)})"

        def exists() -> str:
            kinds = [
                k for k in ("person", "title", "office", "claim", "vow") if self.var(f"ent:{k}")
            ]
            return f"exists({self.entity_arg(r.pick(kinds or ['person']), d, True)})"

        def kinship() -> str | None:
            f = r.pick(("descends", "related"))
            if not self.ok(f):
                return None
            a, b = self.entity_arg("person", d), self.entity_arg("person", d)
            return f"{f}({a}, {b})" if f == "descends" else f"{f}({a}, {b}, {1 + r.below(6)})"

        return [
            compare,
            equal,
            lambda: f"not ({self.expr('bool', d)})",
            lambda: f"({self.expr('bool', d)} {r.pick(('and', 'or'))} {self.expr('bool', d)})",
            lambda: self.aggregate("bool", d + 1),
            lambda: self.aggregate("bool", d + 1),
            exists,
            kinship,
            lambda: self.draw_call("chance"),
            lambda: self.conditional("bool", d),
            lambda: self.read("bool", d + 1),
        ]

    def numeric_productions(self, ty: str, d: int) -> list[Callable[[], str | None]]:
        """The productions of a numeric type at depth `d`."""
        r = self.rng

        def arith() -> str:
            if ty in FIXED and r.chance(1, 2):  # a product on a tie: half to even decides it
                return f"({r.pick(ODD_LAST_BITS[ty])} * {r.pick(('0.5', '1.5', '-0.5'))})"
            return f"({self.expr(ty, d)} {r.pick(('+', '-', '*'))} {self.expr(ty, d)})"

        def divide() -> str:
            c = r.pick(("2", "3", "-4")) if ty == "int" else r.pick(("0.5", "2.0", "-1.5"))
            return f"({self.expr(ty, d)} / {c})"

        def call() -> str:
            f = r.pick(("abs", "min", "max", "clamp", "safe_div"))
            n = {"abs": 1, "min": 2, "max": 2}.get(f, 3)
            return f"{f}({', '.join(self.expr(ty, d) for _ in range(n))})"

        out: list[Callable[[], str | None]] = [
            arith,
            arith,
            divide,
            lambda: f"-{self.expr(ty, d)}",
            call,
            lambda: self.aggregate(ty, d + 1),
            lambda: self.conditional(ty, d),
        ]
        if ty == "int":
            out += self.int_productions(d)
        elif ty == "q16_16":
            out.append(lambda: f"widen({self.expr('q8_8', d)})")
        else:
            out.append(lambda: f"narrow({self.expr('q16_16', d)})")
        return out

    def int_productions(self, d: int) -> list[Callable[[], str | None]]:
        """The productions only `int` has."""
        r = self.rng

        def kinship() -> str | None:
            ups = [f for f in ("up", "up_agnatic") if self.ok(f)]
            a, b = self.entity_arg("person", d), self.entity_arg("person", d)
            return f"{r.pick(ups)}({a}, {b})" if ups else None

        def draw() -> str | None:
            plain = not self.loops and not self.verb and "holder" in self.scope
            if plain and self.purposes and r.chance(1, 4):
                # One site text each, two keys that meet when x = holder at run time.
                p = r.pick(self.purposes)
                self.k += 1
                return f"(draw({p}, x, {self.k}) + draw({p}, holder, {self.k}))"
            return self.draw_call("draw")

        enum = r.pick(("sex", "death_cause", "title_tiers", "claim_status"))
        return [
            lambda: f"({self.expr('int', d)} {r.pick(('div', 'mod'))} {r.pick(('2', '3', '-7'))})",
            lambda: f"{r.pick(('floor', 'round'))}({self.expr(r.pick(FIXED), d)})",
            lambda: f"ordinal({self.expr('enum:' + enum, d)})",
            kinship,
            draw,
            lambda: f"round({r.pick(('q8', 'q16'))}({self.expr('int', d)}) * {r.pick(DECIMALS)})",
        ]

    def conditional(self, ty: str, d: int) -> str:
        """`if c then a else b` of type `ty` (only the taken branch is evaluated)."""
        return f"(if {self.expr('bool', d)} then {self.expr(ty, d)} else {self.expr(ty, d)})"

    def expr(self, ty: str, depth: int) -> str:
        """An expression of type `ty`, nesting at most `depth` productions."""
        r = self.rng
        if depth <= 0 or r.chance(1, 6):
            read = self.read(ty, depth) if r.chance(1, 3) else None
            return read or self.leaf(ty)
        d = depth - 1
        if ty == "bool":
            productions = self.bool_productions(d)
        elif ty in NUMERIC:
            productions = self.numeric_productions(ty, d)
        elif r.chance(1, 3):
            return self.conditional(ty, d)
        elif ty.startswith("ent:"):
            return self.entity_arg(ty.split(":")[1], depth)
        else:
            return self.read(ty, depth) or self.leaf(ty)
        for production in r.shuffled(productions):
            if (out := production()) is not None:
                return out
        return self.leaf(ty)

    def key(self, depth: int) -> str:
        """An order key: a scalar, a tuple, or a tuple with a list comprehension (§3.4)."""
        parts = []
        for _ in range(1 + self.rng.below(3)):
            if self.rng.chance(1, 4):
                rel = self.relation("person", depth)
                y = self.loop("person")
                inner = f"({self.expr('bool', depth - 1)}, {self.expr('int', depth - 1)})"
                parts.append(f"[{inner} for {y} in {rel}]")
                self.unloop(y)
            else:
                parts.append(self.expr(self.rng.pick(("int", "bool", "q16_16")), depth))
        return parts[0] if len(parts) == 1 else f"({', '.join(parts)})"

    def action(self, depth: int) -> str:
        """One action of an effects program: a declared send with typed arguments (§6)."""
        sends = [
            s["command"]
            for s in self.sl.systems[self.system].get("sends", [])
            if s["command"] in self.sl.commands
        ]
        cmd = self.rng.pick(sends)
        args = [f"person: {self.entity_arg('person', depth, True)}"]
        for p in self.sl.commands[cmd]["params"]:
            t = {"ref": f"ent:{p.get('ref')}", "enum": f"enum:{p.get('enum')}"}.get(
                p["type"], p["type"]
            )
            v = self.rng.pick(("event", "none")) if t == "event" else self.expr(t, depth)
            args.append(f"{p['name']}: {v}")
        return f"{cmd}({', '.join(args)})"

    def effects(self, depth: int) -> str:
        """An effects program: actions, guarded actions and loops of actions (§6)."""
        items = []
        for _ in range(1 + self.rng.below(3)):
            form = self.rng.below(3)
            if form == 0:
                items.append(self.action(depth - 1))
            elif form == 1:
                items.append(f"when {self.expr('bool', depth - 1)}: {self.action(depth - 1)}")
            else:
                kind = self.rng.pick(("person", "office", "claim"))
                rel = self.relation(kind, depth - 1)
                y = self.loop(kind)
                where = f" where {self.expr('bool', depth - 1)}" if self.rng.chance(1, 2) else ""
                items.append(f"for {y} in {rel}{where}: {self.action(depth - 1)}")
                self.unloop(y)
        return "; ".join(items)


# ---------------------------------------------------------------------------------------- faults
def fault(rng: Rng, text: str) -> tuple[str, str, str]:
    """A single-fault mutant of a well-typed M7 program aimed at one code: (code, system, text).

    The fault is a conjunct of an always-defined comparison, the only fault of the program.
    """
    code = rng.pick(CODES)
    guard = f"({text}) == ({text}) and"
    bad = {
        "E_SYNTAX": ("M7.dynasty", f"({text}) +"),
        "E_FLOAT_LITERAL": ("M7.dynasty", f"{guard} 1e{1 + rng.below(9)} > 0"),
        "E_LITERAL_RANGE": ("M7.dynasty", f"{guard} 9223372036854775808 > 0"),
        "E_UNKNOWN_NAME": ("M7.dynasty", f"{guard} frobnicate(1) > 0"),
        "E_TYPE": ("M7.dynasty", f"{guard} true + 1 > 0"),
        "E_ARITY": ("M7.dynasty", f"{guard} abs(1, 2) > 0"),
        "E_CONST": ("M7.dynasty", f"{guard} count(z in kin(holder, now)) > 0"),
        "E_DIV": ("M7.dynasty", f"{guard} 1 / 0 > 0"),
        "E_BOUND": ("M7.dynasty", f"{guard} count(z in kin(holder, 7)) > 0"),
        "E_PLANE": ("X.fixture", "pin.flag(x)"),
        "E_UNDECLARED_READ": ("M7.dynasty", f"{guard} mind.stress(none) > 0.0"),
        "E_NO_NEUTRAL": ("X.fixture", "title.holder(none) == none"),
        "E_UNDECLARED_COMMAND": ("S7.vows", "Resign(person: vower, office: none)"),
        "E_COMMAND_OWNER": ("X.fixture", "PressClaim(person: x, claim: none)"),
        "E_RNG_PURPOSE": ("M7.dynasty", f"{guard} draw(nosuch, x, 0) > 0"),
        "E_RNG_REUSE": ("M7.dynasty", f"{guard} draw(sortition, x, 3) + draw(sortition, x, 3) > 0"),
    }[code]
    return code, bad[0], bad[1]


def char_mutant(rng: Rng, text: str) -> str:
    """A random character mutant: delete, duplicate or replace one character (multi-fault)."""
    if not text:
        return "("
    i = rng.below(len(text))
    op = rng.below(3)
    if op == 0:
        return text[:i] + text[i + 1 :]
    if op == 1:
        return text[:i] + text[i] + text[i:]
    return text[:i] + rng.pick(CHAR_ALPHABET) + text[i + 1 :]


# -------------------------------------------------------------------------------------- one seed
def case_toml(case: dict[str, Any]) -> str:
    """A `[[case]]` table."""
    out = ["[[case]]\n"]
    for k, v in case.items():
        if isinstance(v, dict):
            inner = ", ".join(f"{json.dumps(a)} = {toml_value(b)}" for a, b in v.items())
            out.append(f"{k} = {{ {inner} }}\n")
        else:
            out.append(f"{k} = {toml_value(v)}\n")
    return "".join(out)


class Seed:
    """One seed's file: the slice, its laws, its worlds and its cases."""

    def __init__(self, sl: Slice, seed: int) -> None:
        """The worlds of `seed`: three listed worlds of growing size and a fan tree."""
        self.sl, self.seed, self.rng = sl, seed, Rng(seed)
        self.worlds: list[tuple[str, dict[str, list[int]]]] = []
        self.world_text: list[str] = []
        for i, n in enumerate(
            [8 + self.rng.below(8), 24 + self.rng.below(24), 60 + self.rng.below(60)]
        ):
            text, ids = gen_world(sl, self.rng, f"w{i}", n)
            self.worlds.append((f"w{i}", ids))
            self.world_text.append(text)
        text, ids = gen_fan(sl, self.rng, "fan", seed)
        self.worlds.append(("fan", ids))
        self.world_text.append(text)
        self.laws: list[str] = []
        self.cases: list[dict[str, Any]] = []
        self.meta: Meta = {}

    def add(self, case: dict[str, Any], cls: str) -> None:
        """Records a case with its class (`typed`, `fault:<code>`, `chars`, `neutral`)."""
        case["expect"] = "unchecked"
        self.cases.append(case)
        self.meta[case["id"]] = {
            "class": cls,
            "kind": case["kind"],
            "text": case["text"],
            "world": case["world"],
        }

    def probes(self) -> None:
        """The bound probes on the fan tree and a run-time draw reuse."""
        deep = f"person:{self.worlds[-1][1]['deep'][0]}"
        for i, text in enumerate(
            ["count(y in lineage(holder, 3))", f"count(y in kin(holder, {4 + self.rng.below(3)}))"]
        ):
            self.add(
                {
                    "id": f"s{self.seed}_p{i}",
                    "world": "fan",
                    "kind": "expr",
                    "system": "M7.dynasty",
                    "bind": {"holder": deep},
                    "text": text,
                },
                "typed",
            )
        world, ids = self.worlds[0]
        who = f"person:{self.rng.pick(ids['person'])}"
        self.add(
            {
                "id": f"s{self.seed}_p2",
                "world": world,
                "kind": "expr",
                "system": "M7.dynasty",
                "bind": {"holder": who, "x": who},
                "text": "draw(sortition, x, 5) + draw(sortition, holder, 5)",
            },
            "typed",
        )

    def generated(self, c: int) -> None:
        """Generated case `c`: the kinds in rotation first, then mostly expressions."""
        rng = self.rng
        world, ids = rng.pick(self.worlds)
        kinds = (*KINDS, "expr", "expr")
        kind = KINDS[c % len(KINDS)] if c < CASES_IN_ROTATION else rng.pick(kinds)
        depth = 1 + rng.below(DEPTH_MAX)
        holder = rng.pick(ids["person"])
        x = holder if rng.chance(1, 6) else rng.pick(ids["person"])  # x = holder: a draw reuse
        bind = {
            "holder": f"person:{holder}",
            "x": f"person:{x}",
            "title": f"title:{rng.pick(ids['title'])}",
        }
        case: dict[str, Any] = {"id": f"s{self.seed}_c{c}", "world": world, "kind": kind}
        cls = "typed"
        if kind == "effects":
            case.update(self.effects_case(ids, depth))
        elif kind == "succession":
            case.update(self.law_case(bind, depth))
        elif kind == "candidates":
            del bind["x"]
            g = Gen(rng, self.sl, "M7.dynasty", scope_of(bind))
            case.update(system="M7.dynasty", bind=bind, text=g.relation("person", depth))
        elif kind == "order":
            g = Gen(rng, self.sl, "M7.dynasty", scope_of(bind))
            case.update(system="M7.dynasty", bind=bind, text=g.key(depth))
        else:
            cls = self.expr_case(case, bind, depth)
        if case["kind"] in ("expr", "order", "succession") and rng.chance(1, 5):
            case["evaluations"] = 2  # one step's repeated evaluation: a ledger each (N6)
        self.add(case, cls)

    def effects_case(self, ids: dict[str, list[int]], depth: int) -> dict[str, Any]:
        """An effects case of T1."""
        rng = self.rng
        bind = {
            "actor": f"person:{rng.pick(ids['person'])}",
            "target": f"person:{rng.pick(ids['person'])}",
            "instrument": f"artifact:{rng.pick(ids['artifact'])}",
        }
        scope = {"actor": "ent:person", "target": "ent:person", "instrument": "ent:artifact"}
        g = Gen(rng, self.sl, "T1.minds", {**scope, "now": "int", "event": "event"})
        g.verb = True
        return {"system": "T1.minds", "bind": bind, "text": g.effects(depth)}

    def law_case(self, bind: dict[str, str], depth: int) -> dict[str, Any]:
        """A succession case of a generated law, added to the slice's programs."""
        g = Gen(self.rng, self.sl, "M7.dynasty", {"holder": "ent:person", "title": "ent:title"})
        g.scope["now"] = "int"
        candidates = g.relation("person", depth)
        g.scope["x"] = "ent:person"
        g.loops = ["x"]  # a law's x is its loop variable (§4.6)
        law = f"fz{self.seed}_{len(self.laws)}"
        self.laws.append(
            f"[registry.programs.succession_laws.{law}]\ncandidates = {json.dumps(candidates)}\n"
            f"eligible = {json.dumps(g.expr('bool', depth))}\norder = {json.dumps(g.key(depth))}\n"
        )
        del bind["x"]
        return {"system": "M7.dynasty", "law": law, "bind": bind, "text": f"law {law}"}

    def expr_case(self, case: dict[str, Any], bind: dict[str, str], depth: int) -> str:
        """An expression case, and its class.

        Well-typed, a single fault, a character mutant or an absent owner's read.
        """
        rng = self.rng
        system = rng.pick(("M7.dynasty", "M7.dynasty", "T1.minds", "S7.vows"))
        text = Gen(rng, self.sl, system, scope_of(bind)).expr(
            rng.pick(("bool", "int", "q8_8", "q16_16", "ent:person")), depth
        )
        roll, cls = rng.below(10), "typed"
        if roll < FAULTS:
            text = Gen(rng, self.sl, "M7.dynasty", scope_of(bind)).expr(
                rng.pick(("bool", "int", "q16_16")), depth
            )
            code, system, text = fault(rng, text)
            cls = f"fault:{code}"
            if code in ("E_UNDECLARED_COMMAND", "E_COMMAND_OWNER"):
                case["kind"] = "effects"
                bind = {"vower": bind["x"], "x": bind["x"]}
        elif roll < CHARS:
            text, cls = char_mutant(rng, text), "chars"
        elif roll == NEUTRAL:
            # An absent owner's read (NEW-4): the case's variant system declares the neutral.
            column, raw, var = rng.pick(
                [
                    ("polity.exec_constraint", rng.pick(("0.5", "-1.25", "0.0039")), "polity:1"),
                    ("region.conditions", rng.pick(self.sl.enums["region_conditions"]), "region:1"),
                ]
            )
            system, bind, cls = "S6.artifacts", {"p": var}, "neutral"
            case["extra_reads"] = {column: raw}
            text = f"{column}(p)" if rng.chance(1, 2) else f"({column}(p) == {column}(p))"
        case.update(system=system, bind=bind, text=text)
        return cls

    def text(self) -> str:
        """The golden-format file."""
        return (
            self.sl.header
            + self.sl.text
            + "".join(self.laws)
            + "\n"
            + "".join(self.world_text)
            + "\n"
            + "".join(case_toml(c) for c in self.cases)
        )


def scope_of(bind: dict[str, str]) -> dict[str, str]:
    """The bound variables' types of a binding table, plus `now`."""
    return {name: f"ent:{v.split(':')[0]}" for name, v in bind.items()} | {"now": "int"}


def build(sl: Slice, seed: int, cases: int) -> tuple[str, Meta]:
    """One seed's golden-format text, and each case's metadata (class, kind, program, world)."""
    s = Seed(sl, seed)
    s.probes()
    for c in range(cases):
        s.generated(c)
    return s.text(), s.meta


# --------------------------------------------------------------------------------------- compare
LINE = re.compile(r"^(?P<id>\S+) (?P<result>\S.*?)(?P<diags>(?: [a-z_]+=-?\d+)*)$")


def parse_emit(text: str) -> dict[str, Result]:
    """`id result [name=value ...]` lines."""
    out = {}
    for line in text.splitlines():
        m = LINE.match(line)
        if m is not None:
            pairs = (kv.split("=") for kv in m.group("diags").split())
            out[m.group("id")] = (m.group("result"), {k: int(v) for k, v in pairs})
    return out


def agree(cls: str, a: Result, b: Result) -> bool:
    """Whether the two sides agree.

    Exactly, or only as accept or reject on a multi-fault (character) mutant.
    """
    if cls == "chars":
        rejected = a[0].startswith("error:")
        return rejected == b[0].startswith("error:") and (rejected or a == b)
    return a == b


def bump(cov: dict[str, int], key: str) -> None:
    """Counts one more of `key`."""
    cov[key] = cov.get(key, 0) + 1


def coverage_of(meta: Meta, results: dict[str, Result], cov: dict[str, int]) -> None:
    """Counts what the run reached, on the cases both sides agreed on."""
    for cid, (result, diags) in results.items():
        m = meta[cid]
        bump(cov, f"kind:{m['kind']}")
        if result.startswith(("error:", "invariant:")):
            head, _, code = result.partition(":")
            bump(cov, f"{'code' if head == 'error' else 'invariant'}:{code}")
        if diags.get("truncated", 0) > 0:
            bump(cov, "truncated_relations>0")
        for name in (*RELATIONS, *BUILTINS):
            if re.search(rf"\b{name}\(", m["text"]):
                bump(cov, f"call:{name}")
        for name in AGGREGATES:
            if re.search(rf"\b{name}\(\w+ in ", m["text"]):
                bump(cov, f"aggregate:{name}")


def required_coverage() -> list[str]:
    """Every counter the PR tier asserts non-zero."""
    return (
        [f"code:{c}" for c in CODES]
        + [f"invariant:{i}" for i in INVARIANTS]
        + [f"call:{n}" for n in (*RELATIONS, *BUILTINS)]
        + [f"aggregate:{a}" for a in AGGREGATES]
        + [f"kind:{k}" for k in KINDS]
        + ["truncated_relations>0"]
    )


def seeds_of(spec: str) -> list[int]:
    """`1-16`, `3,5,9` or a mix."""
    out: list[int] = []
    for part in spec.split(","):
        a, _, b = part.partition("-")
        out.extend(range(int(a), int(b or a) + 1))
    return out


def run(cmd: list[str]) -> tuple[int, str, str]:
    """Runs a command and returns its exit code, stdout and stderr."""
    done = subprocess.run(cmd, capture_output=True, text=True, check=False)
    return done.returncode, done.stdout, done.stderr


def write_repro(out: Path, cid: str, body: str) -> None:
    """The failing case alone, in the seed's slice and worlds: a reduced repro."""
    head, _, cases = body.partition("[[case]]\n")
    keep = next(
        (f"[[case]]\n{c}" for c in cases.split("[[case]]\n") if c.startswith(f'id = "{cid}"')),
        "",
    )
    (out / f"repro_{cid}.toml").write_text(f"{head}{keep}", encoding="utf-8")


Outputs = tuple[tuple[int, str, str], tuple[int, str, str]]


def both_sides(args: argparse.Namespace, seed: int) -> Outputs:
    """The reference's and the Rust evaluator's runs on one seed's file."""
    path = args.out / f"seed_{seed}.toml"
    rust_cmd = [str(args.rust_bin), "effect", "eval", str(path)]
    if args.rust_mutant:
        rust_cmd += ["--mutant", args.rust_mutant]
    ref = run([sys.executable, "-I", str(args.ref), "--golden", str(path), "--emit"])
    return ref, run(rust_cmd)


def compare_seed(
    args: argparse.Namespace,
    seed: int,
    outputs: Outputs,
    built: tuple[str, Meta],
    cov: dict[str, int],
) -> tuple[int, int]:
    """Compares both sides on one seed's file; returns (mismatches, crashes), counts coverage."""
    (rc_ref, out_ref, err_ref), (rc_rs, out_rs, err_rs) = outputs
    body, meta = built
    if rc_ref != 0 or rc_rs != 0:
        print(f"CRASH seed {seed}: reference exit {rc_ref}: {err_ref.strip()[-400:]}")
        print(f"  rust exit {rc_rs}: {err_rs.strip()[-400:]}")
        return 0, 1
    ref, rs = parse_emit(out_ref), parse_emit(out_rs)
    mismatches = crashes = 0
    agreed = {}
    for cid, m in meta.items():
        a, b = ref.get(cid), rs.get(cid)
        what = f"seed {seed} case {cid} ({m['kind']}, {m['class']}, world {m['world']})"
        if a is None or b is None or a[0].startswith("crash:") or b[0].startswith("crash:"):
            crashes += 1
            print(f"CRASH {what}: reference {a}, rust {b}\n  program: {m['text']}")
        elif not agree(m["class"], a, b):
            mismatches += 1
            print(f"MISMATCH {what}\n  program:   {m['text']}\n  reference: {a}\n  rust:      {b}")
            write_repro(args.out, cid, body)
        else:
            agreed[cid] = a
    coverage_of(meta, agreed, cov)
    return mismatches, crashes


def main(argv: list[str]) -> int:
    """Command line; see the module docstring."""
    ap = argparse.ArgumentParser(description=(__doc__ or "").split("\n", maxsplit=1)[0])
    ap.add_argument("--seeds", required=True, help="e.g. 1-16 or 3,5,9")
    ap.add_argument("--rust-bin", type=Path)
    ap.add_argument("--ref", type=Path, default=REFERENCE)
    ap.add_argument("--golden", type=Path, default=GOLDEN)
    ap.add_argument("--cases", type=int, default=160)
    ap.add_argument("--out", type=Path, default=ROOT / "target" / "fuzz")
    ap.add_argument("--rust-mutant", default=None)
    ap.add_argument("--assert-coverage", action="store_true")
    ap.add_argument("--write-only", action="store_true", help="write the files, run nothing")
    args = ap.parse_args(argv)
    if args.rust_bin is None and not args.write_only:
        ap.error("--rust-bin is required unless --write-only")
    sl = Slice(args.golden.read_text(encoding="utf-8"))
    args.out.mkdir(parents=True, exist_ok=True)
    seeds = seeds_of(args.seeds)
    mismatches = crashes = total = 0
    cov: dict[str, int] = {}
    start = time.monotonic()
    built = {}
    for seed in seeds:
        body, meta = build(sl, seed, args.cases)
        (args.out / f"seed_{seed}.toml").write_text(body, encoding="utf-8")
        total += len(meta)
        built[seed] = (body, meta)
    if not args.write_only:
        # The seeds run concurrently (each side is a subprocess); results are judged in seed order.
        with ThreadPoolExecutor(max_workers=max(1, min(8, os.cpu_count() or 1))) as pool:
            outputs = list(pool.map(lambda seed: both_sides(args, seed), seeds))
        for seed, out in zip(seeds, outputs, strict=True):
            m, c = compare_seed(args, seed, out, built[seed], cov)
            mismatches, crashes = mismatches + m, crashes + c
    if args.write_only:
        print(f"WROTE {len(seeds)} files ({total} cases) under {args.out}")
        return 0
    missing = [k for k in required_coverage() if cov.get(k, 0) == 0]
    print("COVERAGE " + " ".join(f"{k}={cov.get(k, 0)}" for k in required_coverage()))
    if missing:
        print(f"COVERAGE MISSING: {' '.join(missing)}")
    verdict = mismatches == 0 and crashes == 0 and not (args.assert_coverage and missing)
    print(
        f"EFFECT FUZZ: {'PASS' if verdict else 'FAIL'} ({total} cases, {len(seeds)} seeds, "
        f"{mismatches} mismatches, {crashes} crashes, {time.monotonic() - start:.1f} s)"
    )
    return 0 if verdict else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
