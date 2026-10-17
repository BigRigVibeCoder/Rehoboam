# ruff: noqa: PLR2004, PLR0913, PLR0917
# (a fixture is literal data, and a record's fields are positional, in line order)
"""The annals fixture chronicles, derived by script from M2's and M7's rules (SPR-029; R10).

Three fixture worlds, each a directory with ``chronicle_A.txt`` (channel A in the canonical text
form, printed by ``canonical_v0.py`` so every line is valid by construction), ``world.toml`` (the
calendar), ``names.toml`` (the manifest's names) and, for the dynastic one, ``keyframe.toml`` (the
keyframe extract: annals_v0.md §1, §6):

- ``ashcombe``: two houses and two seats over forty years on a 360-day calendar, in the shape of
  ``specs/manifest/examples/manor.yaml``. The history follows the rules the specs state: a birth
  nine months after its conception and never from a pregnancy pending at genesis (M2 0.3.0 F9,
  SPR-019 D-08); a posthumous birth within nine months of the father's death; a succession or an
  interregnum in the instant a holder dies or is deposed, at the `Polity` phase (M7 0.3.4, SAGA-13);
  a claim created only when a succession passes over the legal heir or a ruler is deposed; a
  deposition by the leading pressed claimant, who holds an unrenounced claim; a title named in
  every M7 payload; T1's acts with the edges and motives T1 declares (envelope_v0.md §3).
- ``solitary``: one person on a 365-day calendar (rung 0): a death and the extinction.
- ``untold``: the same, with the death refused by the chronicle (``cause_told = 0`` on the
  extinction's edge; no ``DEATH`` record): the untold-cause case of annals_v0.md §3.

Ids are XXH3-64 of a label (``person:edric``), so they are 64-bit like real H64 ids and the id
fallback reads as it will at G1a (REV-SPR-029 F21); they are not H64 ids and nothing here was run
by the engine. Verb ids, act classes and tellability come from the Registry plus the next-M2
delta (``BIRTH``, ``MARRIAGE``), the dictionary the chronicle headers name as ``0.2.0+next``.

Run: python -I specs/annals/fixtures/gen_fixtures.py [--check]
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from types import ModuleType
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CANONICAL = ROOT / "specs" / "stream" / "canonical_v0.py"
REGISTRY = ROOT / "specs" / "manifest" / "registry_v0.toml"
DELTA = ROOT / "specs" / "systems" / "next" / "registry_delta.toml"
DICTIONARY = "0.2.0+next"  # the Registry's registry_version plus the next-M2 delta's verbs
DAY = 1440
ONE = 256
ACTOR, TARGET = 0, 1
PERSON = 1
TRIGGERED, MOTIVATED_BY = 1, 3
VITAL, POLITY, AGENDA = 4, 7, 9
STRESS = "stress"
RETALIATION_FOR, JEALOUSY_OF, AMBITION_FOR = 0, 1, 3
NONE_CAUSE, NATURAL, MURDER, ACCIDENT = 0, 1, 2, 5
MALE, FEMALE = 0, 1
BORN, FOUNDER = 0, 1
LORDSHIP, THORNE_SEAT = 1, 2


def _load(path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(path.stem, path)
    if spec is None or spec.loader is None:
        raise ImportError(str(path))
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module
    spec.loader.exec_module(module)
    return module


_canon = _load(CANONICAL)


def pid(label: str) -> int:
    """A 64-bit illustrative id: XXH3-64 of ``person:<label>`` (or ``house:``, ``title:``)."""
    return int(_canon.xxh3_64(label.encode("ascii")))


def registry_verbs() -> tuple[dict[str, dict[str, Any]], list[str], list[str]]:
    """The Registry's verbs with the next-M2 delta, their sorted names and the act classes."""
    reg = tomllib.loads(REGISTRY.read_text(encoding="utf-8"))
    verbs: dict[str, dict[str, Any]] = dict(reg["verbs"])
    verbs.update(tomllib.loads(DELTA.read_text(encoding="utf-8")).get("verbs", {}))
    attrs = sorted(reg["attributes"])
    return verbs, sorted(verbs), [*reg["act_classes"]["ids"], "|".join(attrs)]


@dataclass
class Segment:
    """One channel-A canonical file under construction."""

    days_per_year: int
    verbs: dict[str, dict[str, Any]]
    names: list[str]
    acts: list[str]
    stress_attr: int
    timeline: int = 0
    seq: int = 0
    next_event: int = 0
    lines: list[str] = field(default_factory=list)

    def tick(self, year: int, month: int, day: int) -> int:
        """A day start: year index from 0 (negative before the epoch), month and day from 1."""
        months = (
            [30] * 12
            if self.days_per_year == 360
            else [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        )
        return (year * self.days_per_year + sum(months[: month - 1]) + day - 1) * DAY

    def add(self, kind: str, tick: int, *fields: object) -> None:
        """Append one record line through the canonical printer."""
        rec = _canon.Record(kind, (self.timeline, self.seq, tick, *fields))
        self.lines.append(_canon.render(rec))
        self.seq += 1

    def event(
        self,
        tick: int,
        phase: int,
        verb: str,
        payload: tuple[int, ...] = (),
        roles: tuple[tuple[int, int], ...] = (),
        snaps: tuple[tuple[int, int], ...] = (),
        causes: tuple[tuple[int, int, int | None, int, int, int], ...] = (),
    ) -> int:
        """An EVENT record; returns its id. Roles are (role code, person id)."""
        eid = self.next_event
        self.next_event += 1
        v = self.verbs[verb]
        bits = sum(1 << self.acts.index(str(a)) for a in v["act_classes"])
        cast = tuple(sorted(((PERSON, p, r) for r, p in roles), key=lambda x: (x[2], x[0], x[1])))
        snapshots = tuple(
            sorted((self.stress_attr, PERSON, p, None, None, val) for p, val in snaps)
        )
        edges = tuple(sorted(causes))
        self.add(
            "EVENT",
            tick,
            eid,
            phase,
            self.names.index(verb),
            verb,
            bits,
            None,
            0,
            int(v["tellability"]),
            0,
            0,
            payload,
            cast,
            snapshots,
            edges,
        )
        return eid

    def entry(
        self, tick: int, person: int, origin: int, sex: int, mother: int, father: int, birth: int
    ) -> None:
        """A LIFE_ENTRY record."""
        self.add("LIFE_ENTRY", tick, person, origin, sex, mother, father, birth)

    def exit(self, tick: int, person: int, cause: int, event: int | None) -> None:
        """A LIFE_EXIT record (a death)."""
        self.add("LIFE_EXIT", tick, person, 0, cause, event)

    def text(self) -> str:
        """The canonical file."""
        head = _canon.header_line(self.timeline, "A", DICTIONARY, "rehoboam/stream/channel-a/v0")
        return "\n".join([head, *self.lines]) + "\n"


def told(cause: int, kind: int = TRIGGERED) -> tuple[int, int, int | None, int, int, int]:
    """A told edge of a deterministic kind."""
    return (cause, kind, None, 0, ONE, 1)


def motive(
    cause: int, motive_id: int | None, weight: int, evidence: int
) -> tuple[int, int, int | None, int, int, int]:
    """A told ``motivated_by`` edge (q8_8 raw weight and evidence)."""
    return (cause, MOTIVATED_BY, motive_id, weight, evidence, 1)


# --- the dynastic fixture ----------------------------------------------------------------------

P = {
    name: pid(f"person:{name}")
    for name in (
        "ash_root",
        "ash_root_wife",
        "edric",
        "maud",
        "osric",
        "aldwin",
        "edith",
        "thorne_root",
        "thorne_root_wife",
        "hugh",
        "alys",
        "giles",
        "ysolde",
        "cedric",
        "wulf",
        "wynn",
        "orm",
        "xan",
        "tilda",
        "bram",
    )
}
HOUSE = {"ashcombe": pid("house:ashcombe"), "thorne": pid("house:thorne")}
# birth years (index; negative before the epoch) of the genesis persons, and the dead's death years
Date = tuple[int, int, int]
# The order is the founder order: founder k is the k-th entry, the manifest's cast first (as
# rh-worldgen issues founder(seed, k), SPR-019 D-02; REV-SPR-029 R26), then the back-fill.
GENESIS: dict[str, tuple[int, str | int, str | int, Date, Date | None, str | None]] = {
    # name: (sex, mother, father, birth (y, m, d), death (y, m, d) or None, house or None)
    "edric": (MALE, "ash_root_wife", "ash_root", (-38, 3, 15), None, "ashcombe"),
    "maud": (FEMALE, 0, 0, (-36, 3, 15), None, None),
    "hugh": (MALE, "thorne_root_wife", "thorne_root", (-40, 3, 15), None, "thorne"),
    "alys": (FEMALE, 0, 0, (-37, 3, 15), None, None),
    "orm": (MALE, 0, 0, (-22, 4, 4), None, None),
    "ash_root": (MALE, 0, 0, (-60, 1, 1), (-12, 1, 1), "ashcombe"),
    "ash_root_wife": (FEMALE, 0, 0, (-58, 1, 1), (-15, 1, 1), None),
    "osric": (MALE, "maud", "edric", (-17, 3, 15), None, "ashcombe"),
    "aldwin": (MALE, "maud", "edric", (-14, 5, 2), None, "ashcombe"),
    "wynn": (FEMALE, "maud", "edric", (-19, 2, 2), None, "ashcombe"),
    "xan": (MALE, "wynn", "orm", (-1, 6, 6), None, None),
    "thorne_root": (MALE, 0, 0, (-65, 1, 1), (-8, 1, 1), "thorne"),
    "thorne_root_wife": (FEMALE, 0, 0, (-62, 1, 1), (-3, 1, 1), None),
    "giles": (MALE, "alys", "hugh", (-20, 7, 7), None, "thorne"),
    "ysolde": (FEMALE, "alys", "hugh", (-16, 3, 15), None, "thorne"),
    "tilda": (FEMALE, "alys", "hugh", (-5, 9, 9), None, "thorne"),
}
# Wynn, Orm, Xan and Tilda are the house-walk probe (REV-SPR-029 R18, annals_v0.md §4 rule 4):
# Xan, a genesis grandson of Edric through a daughter, has no house_member row, so no house (no
# walk into a genesis person's ancestry); his in-run son by Tilda walks to his father (no house)
# and then to his mother, House Thorne. Orm is a cast member who shares Edric's name (the R33
# golden case): both bearers and every child designated through either father carry a label.
TENURES = (  # (title, holder, start (y, m, d), end (y, m, d) or None)
    (LORDSHIP, "ash_root", (-40, 1, 1), (-12, 1, 1)),
    (LORDSHIP, "edric", (-12, 1, 1), None),
    (THORNE_SEAT, "thorne_root", (-38, 1, 1), (-8, 1, 1)),
    (THORNE_SEAT, "hugh", (-8, 1, 1), None),
)


def _ref(x: str | int) -> int:
    return P[x] if isinstance(x, str) else x


def ashcombe(seg: Segment) -> None:  # noqa: PLR0915  (one scripted history, in order)
    """House Ashcombe and House Thorne over forty years."""
    t = seg.tick
    living = sorted(n for n, g in GENESIS.items() if g[4] is None)
    for name in sorted(living, key=lambda n: P[n]):
        sex, mother, father, birth, _, _ = GENESIS[name]
        seg.entry(0, P[name], FOUNDER, sex, _ref(mother), _ref(father), t(*birth))
    # y1: Osric and Ysolde marry; their first son is born nine months after the wedding.
    tk = t(1, 2, 5)
    seg.event(tk, VITAL, "MARRIAGE", roles=((ACTOR, P["osric"]), (TARGET, P["ysolde"])))
    tk = t(1, 11, 20)
    seg.event(tk, VITAL, "BIRTH", (MALE, 1), ((ACTOR, P["ysolde"]), (TARGET, P["cedric"])))
    seg.entry(tk, P["cedric"], BORN, MALE, P["ysolde"], P["osric"], tk)
    seg.add("STEP_DIGEST", t(1, 12, 30), 11, 1, 0, 0x0123456789ABCDEF, 0, 0x1111111111111111)
    # y3: Maud bears Edric a daughter, her third child.
    tk = t(3, 4, 9)
    seg.event(tk, VITAL, "BIRTH", (FEMALE, 3), ((ACTOR, P["maud"]), (TARGET, P["edith"])))
    seg.entry(tk, P["edith"], BORN, FEMALE, P["maud"], P["edric"], tk)
    # y7: Edric dies; Osric, the first son, succeeds in the same instant (no heir is passed over).
    tk = t(7, 2, 20)
    e_edric = seg.event(tk, VITAL, "DEATH", (NATURAL,), ((TARGET, P["edric"]),))
    e_osric_succ = seg.event(
        tk,
        POLITY,
        "SUCCEED",
        (LORDSHIP,),
        ((ACTOR, P["osric"]), (TARGET, P["edric"])),
        causes=(told(e_edric),),
    )
    seg.exit(tk, P["edric"], NATURAL, e_edric)
    # y8: Hugh declares a feud against Osric out of jealousy of his succession (a named motive).
    tk = t(8, 5, 7)
    seg.event(
        tk,
        AGENDA,
        "FEUD_DECLARED",
        roles=((ACTOR, P["hugh"]), (TARGET, P["osric"])),
        snaps=((P["hugh"], 63 * ONE),),
        causes=(motive(e_osric_succ, JEALOUSY_OF, 128, 64),),
    )
    # y9: the feud attack: a murder attempt moved by ambition for the lordship, then the death it
    # causes, the succession of Osric's child, a regency, and the feud's end on the death path.
    tk = t(9, 1, 12)
    e_attempt = seg.event(
        tk,
        AGENDA,
        "MURDER_ATTEMPT",
        roles=((ACTOR, P["hugh"]), (TARGET, P["osric"])),
        snaps=((P["hugh"], 70 * ONE),),
        causes=(motive(e_osric_succ, AMBITION_FOR, 200, 100),),
    )
    tk = t(9, 1, 13)
    e_osric = seg.event(
        tk,
        VITAL,
        "DEATH",
        (MURDER,),
        ((ACTOR, P["hugh"]), (TARGET, P["osric"])),
        causes=(told(e_attempt),),
    )
    e_cedric_succ = seg.event(
        tk,
        POLITY,
        "SUCCEED",
        (LORDSHIP,),
        ((ACTOR, P["cedric"]), (TARGET, P["osric"])),
        causes=(told(e_osric),),
    )
    seg.event(
        tk,
        POLITY,
        "REGENCY_BEGUN",
        (LORDSHIP,),
        ((ACTOR, P["ysolde"]), (TARGET, P["cedric"])),
        causes=(told(e_cedric_succ),),
    )
    seg.event(
        tk,
        AGENDA,
        "FEUD_ENDED",
        roles=((ACTOR, P["osric"]), (TARGET, P["hugh"])),
        snaps=((P["osric"], 10 * ONE),),
        causes=(told(e_osric),),
    )
    seg.exit(tk, P["osric"], MURDER, e_osric)
    # y9: Osric's second son is born eight months after his death (conceived before it).
    tk = t(9, 9, 1)
    seg.event(tk, VITAL, "BIRTH", (MALE, 2), ((ACTOR, P["ysolde"]), (TARGET, P["wulf"])))
    seg.entry(tk, P["wulf"], BORN, MALE, P["ysolde"], P["osric"], tk)
    # y12-13: Aldwin's feud against Hugh, moved by an unnamed motive (an opinion term), reconciled.
    tk = t(12, 7, 9)
    seg.event(
        tk,
        AGENDA,
        "FEUD_DECLARED",
        roles=((ACTOR, P["aldwin"]), (TARGET, P["hugh"])),
        snaps=((P["aldwin"], 55 * ONE),),
        causes=(motive(e_osric, None, 90, 40),),
    )
    tk = t(13, 2, 2)
    e_recon = seg.event(
        tk,
        AGENDA,
        "FEUD_ENDED",
        roles=((ACTOR, P["aldwin"]), (TARGET, P["hugh"])),
        snaps=((P["aldwin"], 20 * ONE),),
    )
    # y14: Giles declares a feud against Aldwin; its only told edge, to the reconciliation, has a
    # weight at or below zero (it inhibited the act), so no motive is chosen and the plain
    # variant renders (REV-SPR-029 R31).
    tk = t(14, 6, 6)
    seg.event(
        tk,
        AGENDA,
        "FEUD_DECLARED",
        roles=((ACTOR, P["giles"]), (TARGET, P["aldwin"])),
        snaps=((P["giles"], 58 * ONE),),
        causes=(motive(e_recon, None, -40, 30),),
    )
    # y16: Hugh dies; Giles succeeds to the seat of Thorne in the same instant.
    tk = t(16, 11, 30)
    e_hugh = seg.event(tk, VITAL, "DEATH", (NATURAL,), ((TARGET, P["hugh"]),))
    seg.event(
        tk,
        POLITY,
        "SUCCEED",
        (THORNE_SEAT,),
        ((ACTOR, P["giles"]), (TARGET, P["hugh"])),
        causes=(told(e_hugh),),
    )
    seg.exit(tk, P["hugh"], NATURAL, e_hugh)
    # y17: Cedric comes of age (born y1 m11; majority 16); y18: Alys dies by accident.
    tk = t(17, 11, 20)
    seg.event(
        tk, POLITY, "REGENCY_ENDED", (LORDSHIP,), ((ACTOR, P["ysolde"]), (TARGET, P["cedric"]))
    )
    tk = t(18, 8, 8)
    e_alys = seg.event(tk, VITAL, "DEATH", (ACCIDENT,), ((TARGET, P["alys"]),))
    seg.exit(tk, P["alys"], ACCIDENT, e_alys)
    # y20-21: the house-walk probe (R18): Xan, houseless, marries Tilda of Thorne; their son is
    # born the next year and walks to House Thorne through his mother. He dies in infancy (y24)
    # and Tilda in y31, so House Thorne is still extinguished with Giles in y33.
    tk = t(20, 4, 4)
    seg.event(tk, VITAL, "MARRIAGE", roles=((ACTOR, P["xan"]), (TARGET, P["tilda"])))
    tk = t(21, 2, 2)
    seg.event(tk, VITAL, "BIRTH", (MALE, 1), ((ACTOR, P["tilda"]), (TARGET, P["bram"])))
    seg.entry(tk, P["bram"], BORN, MALE, P["tilda"], P["xan"], tk)
    # y22: a deposition timer fires with no pressed claimant: Cedric is deposed and gains a claim;
    # his brother Wulf (13) succeeds under a regency. The timer fires on day 2 or later (M7:
    # successions and depositions are never on a year's first day; R26).
    tk = t(22, 3, 2)
    e_dep1 = seg.event(tk, POLITY, "DEPOSED", (LORDSHIP,), ((TARGET, P["cedric"]),))
    e_wulf_succ = seg.event(
        tk,
        POLITY,
        "SUCCEED",
        (LORDSHIP,),
        ((ACTOR, P["wulf"]), (TARGET, P["cedric"])),
        causes=(told(e_dep1),),
    )
    seg.event(
        tk,
        POLITY,
        "CLAIM_CREATED",
        (LORDSHIP,),
        ((ACTOR, P["cedric"]), (TARGET, P["wulf"])),
        causes=(told(e_dep1),),
    )
    seg.event(
        tk,
        POLITY,
        "REGENCY_BEGUN",
        (LORDSHIP,),
        ((ACTOR, P["ysolde"]), (TARGET, P["wulf"])),
        causes=(told(e_wulf_succ),),
    )
    # y23: Cedric presses his claim; y25: Wulf comes of age (born y9 m9); y26: Cedric, the leading
    # pressed claimant, deposes Wulf and succeeds; Wulf gains a claim and renounces it in y27.
    tk = t(23, 1, 9)
    seg.event(tk, POLITY, "CLAIM_PRESSED", (LORDSHIP,), ((ACTOR, P["cedric"]), (TARGET, P["wulf"])))
    tk = t(24, 3, 3)
    e_bram = seg.event(tk, VITAL, "DEATH", (NATURAL,), ((TARGET, P["bram"]),))
    seg.exit(tk, P["bram"], NATURAL, e_bram)
    tk = t(25, 9, 1)
    seg.event(tk, POLITY, "REGENCY_ENDED", (LORDSHIP,), ((ACTOR, P["ysolde"]), (TARGET, P["wulf"])))
    tk = t(26, 5, 5)
    e_dep2 = seg.event(
        tk, POLITY, "DEPOSED", (LORDSHIP,), ((ACTOR, P["cedric"]), (TARGET, P["wulf"]))
    )
    seg.event(
        tk,
        POLITY,
        "SUCCEED",
        (LORDSHIP,),
        ((ACTOR, P["cedric"]), (TARGET, P["wulf"])),
        causes=(told(e_dep2),),
    )
    seg.event(
        tk,
        POLITY,
        "CLAIM_CREATED",
        (LORDSHIP,),
        ((ACTOR, P["wulf"]), (TARGET, P["cedric"])),
        causes=(told(e_dep2),),
    )
    tk = t(27, 3, 3)
    seg.event(tk, POLITY, "CLAIM_RENOUNCED", (LORDSHIP,), ((ACTOR, P["wulf"]),))
    # y30: Ysolde dies; y31: Tilda. y33: Giles dies childless: the seat of Thorne finds no clear
    # heir (an interregnum, castless) and House Thorne is extinguished with him. No claim arises:
    # M7 creates claims only for a passed-over heir or at a deposition (R26). y34: the
    # interregnum resolves in Cedric's favour.
    tk = t(30, 6, 6)
    e_ysolde = seg.event(tk, VITAL, "DEATH", (NATURAL,), ((TARGET, P["ysolde"]),))
    seg.exit(tk, P["ysolde"], NATURAL, e_ysolde)
    tk = t(31, 8, 8)
    e_tilda = seg.event(tk, VITAL, "DEATH", (NATURAL,), ((TARGET, P["tilda"]),))
    seg.exit(tk, P["tilda"], NATURAL, e_tilda)
    tk = t(33, 6, 6)
    e_giles = seg.event(tk, VITAL, "DEATH", (NATURAL,), ((TARGET, P["giles"]),))
    e_inter = seg.event(tk, POLITY, "INTERREGNUM_CRISIS", (THORNE_SEAT,), causes=(told(e_giles),))
    seg.event(
        tk,
        POLITY,
        "HOUSE_EXTINCT",
        (HOUSE["thorne"],),
        ((TARGET, P["giles"]),),
        causes=(told(e_giles),),
    )
    seg.exit(tk, P["giles"], NATURAL, e_giles)
    tk = t(34, 1, 1)
    seg.event(
        tk,
        POLITY,
        "SUCCEED",
        (THORNE_SEAT,),
        ((ACTOR, P["cedric"]), (TARGET, P["giles"])),
        causes=(told(e_inter),),
    )
    # y35: Cedric grants the vacant office of the seat to his brother.
    tk = t(35, 2, 2)
    seg.event(
        tk, POLITY, "SEAT_GRANTED", (THORNE_SEAT,), ((ACTOR, P["cedric"]), (TARGET, P["wulf"]))
    )


def ashcombe_keyframe(seg: Segment) -> str:
    """The keyframe extract of the dynastic fixture (annals_v0.md §1, §6)."""
    t = seg.tick
    out = [
        "# The keyframe extract of the ashcombe fixture (annals_v0.md §1, §6): the rows of the",
        "# genesis",
        "# keyframe a house prologue reads, because the back-filled dead have no channel-A record",
        "# (REV-SPR-019 R1). `founder` is the founder index k of founder(seed, k) (SPR-019 D-02;",
        "# the cast first), shown as `Founder #k+1` (annals_v0.md §4). GENERATED; do not edit.",
        "[extract]",
        'format = "rehoboam/annals/keyframe-extract/v0"',
        'keyframe = "kf/y000000/t000000000.arrow"   # the genesis keyframe the rows came from',
        'state_hash = "0a5c3e1d9b7f2468"                 # its plane-A state_hash (illustrative)',
        "",
    ]
    for key in ("ashcombe", "thorne"):
        out += ["[[house]]", f"id = {HOUSE[key]}   # {key}"]
    for title in (LORDSHIP, THORNE_SEAT):
        out += ["[[title]]", f"id = {title}"]
    founders = {name: k for k, name in enumerate(GENESIS)}  # founder k: the cast first (R26)
    for name, (sex, mother, father, birth, death, _) in sorted(
        GENESIS.items(), key=lambda kv: P[kv[0]]
    ):
        out += [
            "[[life_course]]",
            f"person_id = {P[name]}   # {name}",
            f"founder = {founders[name]}",
            f"sex = {sex}",
            f"mother_id = {_ref(mother)}",
            f"father_id = {_ref(father)}",
            f"birth_tick = {t(*birth)}",
        ]
        if death is not None:
            out.append(f"exit_tick = {t(*death)}")
    for name, (_, _, _, _, _, house) in sorted(GENESIS.items(), key=lambda kv: P[kv[0]]):
        if house is not None:
            out += ["[[house_member]]", f"person = {P[name]}   # {name}", f"house = {HOUSE[house]}"]
    for title, holder, start, end in TENURES:
        out += [
            "[[tenure]]",
            f"title = {title}",
            f"holder = {P[holder]}   # {holder}",
            f"start = {t(*start)}",
        ]
        if end is not None:
            out.append(f"end = {t(*end)}")
    return "\n".join(out) + "\n"


ASHCOMBE_NAMES = """# The plane-C names file of the ashcombe fixture (annals_v0.md §4): what the
# chronicle does not carry. Persons: the living genesis dynasts, as a manifest cast would name
# them (cast member k is founder k in rh-worldgen's order); the genesis children and every in-run
# birth are unnamed, so the kinship and id fallbacks show. Houses: the manifest's dynasties
# (house k is the k-th dynasty, SPR-019 B1). Titles: the polity's labels. Keys are the ids'
# decimal text. GENERATED by gen_fixtures.py; do not edit.
[world]
manifest = "specs/manifest/examples/manor.yaml"   # the shape; ids and the hash are illustrative
manifest_hash = "sha256:afce94fcb48fdfff99f60be5bc6104222a04953e1da364f6cb06c84738d1a2e0"

[persons]
"{edric}" = "Edric Ashcombe"
"{maud}" = "Maud"
"{hugh}" = "Hugh Thorne"
"{alys}" = "Alys"
"{orm}" = "Edric Ashcombe"   # a shared name (annals_v0.md §4; REV-SPR-029 R33)

[houses]
"{ashcombe}" = "House Ashcombe"
"{thorne}" = "House Thorne"

[titles]
"1" = "the lordship of the Manor of Ash"
"2" = "the seat of Thorne"
"""

ASHCOMBE_WORLD = """# Fixture world for the family annals reference (SPR-029): a scripted dynastic
# chronicle in the shape of specs/manifest/examples/manor.yaml (two houses, two seats). Nothing
# here was run by the engine: gen_fixtures.py derives the records from M2's and M7's rules, and
# the self-test checks that the committed files equal its output. GENERATED; do not edit.
[world]
id = "ashcombe"
days_per_year = 360          # the manifest calendar (05.1 §4.1): 12 months of 30 days
epoch_label = "Year of Ash"  # world.calendar.epoch_label
"""

SOLITARY_WORLD = """# Fixture world for the family annals reference (SPR-029): one person on a
# 365-day
# calendar, in the shape of specs/manifest/examples/solo.yaml (rung 0). No keyframe extract: no
# house, no prologue.
# GENERATED by gen_fixtures.py; do not edit.
[world]
id = "{id}"
days_per_year = 365
epoch_label = "Test era"
"""

SOLITARY_NAMES = """# The cast of solo.yaml: Ada is cast member 0, so founder 0 (an illustrative
# id).
# GENERATED.
[world]
manifest = "specs/manifest/examples/solo.yaml"
manifest_hash = "sha256:1f0c52a43606ad0befdcc0adfe23627d83400684af525f12832a77f00abcf8c6"

[persons]
"{ada}" = "Ada"
"""


def solitary(seg: Segment, *, untold: bool) -> None:
    """Ada's death and the extinction; with ``untold``, the death is refused by the chronicle."""
    ada = pid("person:ada")
    seg.entry(0, ada, FOUNDER, FEMALE, 0, 0, seg.tick(-25, 7, 4))
    tk = seg.tick(3, 2, 10)
    if untold:
        seg.next_event = 1  # event 0, the DEATH, got its id and was refused (05.2 §4.0)
        seg.event(tk, VITAL, "EXTINCTION", causes=((0, TRIGGERED, None, 0, ONE, 0),))
        seg.exit(tk, ada, NATURAL, None)
        return
    e_death = seg.event(tk, VITAL, "DEATH", (NATURAL,), ((TARGET, ada),))
    seg.event(tk, VITAL, "EXTINCTION", causes=(told(e_death),))
    seg.exit(tk, ada, NATURAL, e_death)


def build() -> dict[Path, str]:
    """Every fixture file, by path."""
    verbs, names, acts = registry_verbs()
    stress_attr = acts.pop().split("|").index(STRESS)
    out: dict[Path, str] = {}
    seg = Segment(360, verbs, names, acts, stress_attr)
    ashcombe(seg)
    d = HERE / "ashcombe"
    out[d / "chronicle_A.txt"] = seg.text()
    out[d / "keyframe.toml"] = ashcombe_keyframe(seg)
    out[d / "names.toml"] = ASHCOMBE_NAMES.format(
        **{k: P[k] for k in ("edric", "maud", "hugh", "alys", "orm")}, **HOUSE
    )
    out[d / "world.toml"] = ASHCOMBE_WORLD
    for name, untold in (("solitary", False), ("untold", True)):
        seg = Segment(365, verbs, names, acts, stress_attr)
        solitary(seg, untold=untold)
        d = HERE / name
        out[d / "chronicle_A.txt"] = seg.text()
        out[d / "world.toml"] = SOLITARY_WORLD.format(id=name)
        out[d / "names.toml"] = SOLITARY_NAMES.format(ada=pid("person:ada"))
    return out


def main(argv: list[str] | None = None) -> int:
    """Write the fixtures; with ``--check``, compare the committed files with the output."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    files = build()
    for path, text in files.items():
        if args.check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"STALE: {path} differs from gen_fixtures.py's output")
                return 1
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"wrote {path}")
    for path in files:
        if path.name == "chronicle_A.txt":
            _canon.digest_file(path)
    print(f"{'checked' if args.check else 'wrote'} {len(files)} fixture files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
