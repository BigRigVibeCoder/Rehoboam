"""Divergence record of two golden runs of one world (SPR-014 T-08; Architect ruling, option A).

A C3 change that is allowed to move a baseline only through named tables (SPR-014: the
`rh-core/engine.meta` cursor, which M2's daily `Day`/`Vital` instants advance) must show that
nothing else moved. This script compares a run from the base commit with a run from the head:

    ladder     Two plane-A hash ladders (`rehoboam-ladder v1`, as `rh-cli run --out` and
               `run.ladder` hold them). The headers must be equal line for line, and so must the
               rungs' index, tick and date and the set of tables. Every table's root at every
               rung is compared; a table outside `--allow` that differs at any rung fails. The
               rung's own hashes (`hash`, `months`) are folds of the roots and are reported as
               consequences. `--record` prints every table's two roots at every rung.
    canonical  Two channel-A canonical files (`rh-cli tail --canonical`). The header lines and
               the record count must be equal, and so must every record line but the fields
               `--allow` names as `KIND.field` (`STEP_DIGEST.state_hash`, `KEYFRAME.state_hash`):
               a record that differs anywhere else fails. A segment's `canonical_digest` is a hash
               of these lines, so it differs exactly when an allowed field does.

Usage:  python -I bin/ladder_divergence.py ladder OLD NEW [--allow TABLE ...] [--record]
        python -I bin/ladder_divergence.py canonical OLD NEW [--allow KIND.field ...]
Exit:   0 only allowed parts differ; 1 something else differs; 2 a file is missing or malformed.
Standard library only (GOV-003 §3).

Refs: SPR-014 T-08; REV-EVO-001 R1 and EVO-001 §5 as amended; specs/stream/canonical_v0.md §3.
"""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass, field
from pathlib import Path

# Field positions after the kind name and the record header (`tl seq tick`), canonical_v0.md §3.
CANONICAL_FIELDS: dict[str, tuple[str, ...]] = {
    "INPUT": (
        "journal_seq",
        "class",
        "input_kind",
        "scope",
        "scope_id",
        "producer",
        "payload_hash",
        "row_digest",
    ),
    "EVENT": (
        "event_id",
        "phase",
        "verb_id",
        "verb",
        "act_classes",
        "place_id",
        "magnitude",
        "tellability",
        "secrecy",
        "origin",
        "payload",
        "roles",
        "snapshots",
        "causes",
    ),
    "LIFE_ENTRY": ("person_id", "origin", "sex", "mother_id", "father_id", "birth_tick"),
    "LIFE_EXIT": ("person_id", "exit_kind", "death_cause", "event_id"),
    "STEP_DIGEST": (
        "living",
        "births",
        "deaths",
        "state_hash",
        "observation_hash",
        "narrative_hash",
    ),
    "KEYFRAME": ("keyframe_format", "keyframe_path", "state_hash", "journal_position"),
}
HEADER_FIELDS = ("timeline_id", "seq", "tick")
RUNG_TOKENS = 6
ROOT_TOKENS = 3
# At most this many differing records are quoted; the counts cover them all.
MAX_QUOTED = 20


class MalformedError(Exception):
    """A file this script cannot read as the format it expects."""


@dataclass
class Rung:
    """One ladder rung: its position, its two folded hashes and its tables' roots."""

    index: str
    tick: str
    date: str
    hash: str
    months: str
    roots: dict[str, str] = field(default_factory=dict)


@dataclass
class Ladder:
    """A parsed ladder: the header lines, the rungs, and a `failed` line if the run failed."""

    header: list[str]
    rungs: list[Rung]
    tail: list[str]


def read_ladder(path: Path) -> Ladder:
    """Parse a `rehoboam-ladder v1` file."""
    try:
        lines = path.read_text(encoding="ascii").splitlines()
    except (OSError, UnicodeDecodeError) as e:
        raise MalformedError(f"{path}: {e}") from e
    if not lines or lines[0] != "rehoboam-ladder v1":
        raise MalformedError(f"{path}: not a rehoboam-ladder v1 file")
    ladder = Ladder(header=[], rungs=[], tail=[])
    for n, line in enumerate(lines, 1):
        tokens = line.split(" ")
        if tokens[0] == "rung" and len(tokens) == RUNG_TOKENS:
            _, index, tick, date, hsh, months = tokens
            ladder.rungs.append(Rung(index, tick, date, hsh, months))
        elif tokens[0] == "root" and len(tokens) == ROOT_TOKENS:
            if not ladder.rungs:
                raise MalformedError(f"{path}:{n}: a root before any rung")
            ladder.rungs[-1].roots[tokens[1]] = tokens[2]
        elif ladder.rungs:
            ladder.tail.append(line)
        else:
            ladder.header.append(line)
    return ladder


def compare_frames(old: Ladder, new: Ladder) -> list[str]:
    """The problems outside the rungs' roots: headers, rung count, trailing lines."""
    problems = [
        f"header differs: `{x}` vs `{y}`"
        for x, y in zip(old.header, new.header, strict=False)
        if x != y
    ]
    if len(old.header) != len(new.header):
        problems.append("header line counts differ")
    if len(old.rungs) != len(new.rungs):
        problems.append(f"rung counts differ: {len(old.rungs)} vs {len(new.rungs)}")
    if old.tail != new.tail:
        problems.append(f"trailing lines differ: {old.tail} vs {new.tail}")
    return problems


def compare_ladders(old: Ladder, new: Ladder, allow: set[str], record: bool) -> list[str]:
    """Print the comparison; return the problems (empty: only allowed tables differ)."""
    problems = compare_frames(old, new)
    tables = sorted({t for r in old.rungs + new.rungs for t in r.roots})
    differing = dict.fromkeys(tables, 0)
    folded = 0
    for ra, rb in zip(old.rungs, new.rungs, strict=False):
        where = f"rung {ra.index}"
        if (ra.index, ra.tick, ra.date) != (rb.index, rb.tick, rb.date):
            problems.append(f"{where}: position differs: {ra.date} vs {rb.date}")
        if set(ra.roots) != set(rb.roots):
            problems.append(f"{where}: table sets differ: {sorted(set(ra.roots) ^ set(rb.roots))}")
        folded += (ra.hash, ra.months) != (rb.hash, rb.months)
        for t in tables:
            x, y = ra.roots.get(t, "-"), rb.roots.get(t, "-")
            differing[t] += x != y
            if record:
                print(f"rung {ra.index} {ra.date} {t} {x} {y} {'=' if x == y else 'DIFF'}")
    n = len(old.rungs)
    for t in tables:
        if not differing[t]:
            print(f"table {t}: equal at all {n} rungs")
            continue
        state = "allowed" if t in allow else "NOT ALLOWED"
        print(f"table {t}: differs at {differing[t]} of {n} rungs ({state})")
        if t not in allow:
            problems.append(f"table {t} differs at {differing[t]} rungs and is not allowed")
    print(f"rung hashes (folds of the roots): differ at {folded} of {n} rungs")
    return problems


def read_canonical(path: Path) -> list[str]:
    """The lines of a canonical file, comments dropped (they are not records)."""
    try:
        lines = path.read_text(encoding="ascii").splitlines()
    except (OSError, UnicodeDecodeError) as e:
        raise MalformedError(f"{path}: {e}") from e
    if not lines or not lines[0].startswith("#rehoboam/stream/canonical/v0 "):
        raise MalformedError(f"{path}: no canonical v0 header")
    return [lines[0]] + [ln for ln in lines[1:] if not ln.startswith("#")]


def field_name(kind: str, position: int) -> str:
    """The `KIND.field` name of a token position (0 is the kind name)."""
    names = ("kind", *HEADER_FIELDS, *CANONICAL_FIELDS.get(kind, ()))
    return f"{kind}.{names[position]}" if position < len(names) else f"{kind}.#{position}"


def compare_canonical(old: list[str], new: list[str], allow: set[str]) -> list[str]:
    """Print the comparison; return the problems (empty: only allowed fields differ)."""
    problems: list[str] = []
    if old[0] != new[0]:
        problems.append(f"headers differ: `{old[0]}` vs `{new[0]}`")
    if len(old) != len(new):
        problems.append(f"record counts differ: {len(old) - 1} vs {len(new) - 1}")
    counts: dict[str, int] = {}
    kinds: dict[str, int] = {}
    equal = 0
    for a, b in zip(old[1:], new[1:], strict=False):
        ta, tb = a.split(" "), b.split(" ")
        kinds[ta[0]] = kinds.get(ta[0], 0) + 1
        if a == b:
            equal += 1
            continue
        if ta[0] != tb[0] or len(ta) != len(tb):
            problems.append(f"record shapes differ: `{a}` vs `{b}`")
            continue
        for i, (x, y) in enumerate(zip(ta, tb, strict=True)):
            if x != y:
                name = field_name(ta[0], i)
                counts[name] = counts.get(name, 0) + 1
                if name not in allow and len(problems) < MAX_QUOTED:
                    problems.append(f"{name} differs: `{a}` vs `{b}`")
    total = len(old) - 1
    print(f"records: {total} ({', '.join(f'{k} {v}' for k, v in sorted(kinds.items()))})")
    print(f"records byte-identical: {equal} of {total}")
    for name, c in sorted(counts.items()):
        state = "allowed" if name in allow else "NOT ALLOWED"
        print(f"field {name}: differs in {c} records ({state})")
    return problems


def main(argv: list[str] | None = None) -> int:
    """Run the comparison the arguments name."""
    p = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    p.add_argument("mode", choices=("ladder", "canonical"))
    p.add_argument("old", type=Path)
    p.add_argument("new", type=Path)
    p.add_argument("--allow", action="append", default=[])
    p.add_argument("--record", action="store_true", help="ladder: every table's roots per rung")
    a = p.parse_args(argv)
    allow = set(a.allow)
    try:
        if a.mode == "ladder":
            problems = compare_ladders(read_ladder(a.old), read_ladder(a.new), allow, a.record)
        else:
            problems = compare_canonical(read_canonical(a.old), read_canonical(a.new), allow)
    except MalformedError as e:
        print(f"ladder_divergence: {e}", file=sys.stderr)
        return 2
    for problem in problems:
        print(f"DIVERGENCE: {problem}")
    if problems:
        print("DIVERGENCE: FAIL")
        return 1
    print(f"DIVERGENCE: PASS (only {', '.join(sorted(allow)) or 'nothing'} may differ)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
