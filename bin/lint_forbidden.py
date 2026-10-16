"""Forbidden-API gate for plane-A crates (V-DET-19; GOV-003 §2; docs/13 §3) (SPR-001 T-04).

Scans every ``.rs`` file of every plane-A workspace crate (src/, tests/, benches/, examples/,
build.rs), after blanking comments and string and character literals, for APIs that break
determinism. A crate whose plane is missing or invalid is scanned as plane A (and bin/lint_arch.py
reports its metadata). Rules:

    hash-collections  HashMap, HashSet and variants (FxHashMap, AHashSet, ...). Iteration order is
                      unspecified, and text cannot tell iteration from lookup, so plane A forbids
                      the types; use BTreeMap/BTreeSet, an insertion-ordered map or a sorted Vec.
    thread-rng        rand::thread_rng, ThreadRng, rand::rng: thread-local, unkeyed RNG.
    std-rng           StdRng, SmallRng, OsRng, from_entropy, from_os_rng, getrandom: unkeyed or
                      entropy-seeded RNG, some with algorithms that change between releases.
    rand-random       rand::random and rand 0.9's random_range, random_bool, random_ratio and
                      random_iter: thread-local RNG (DEF-1).
    mul-add           mul_add: fused multiply-add rounds differently from multiply-then-add.
    wall-clock        Instant::now, std::time::{Instant, SystemTime, UNIX_EPOCH} (also inside use
                      groups and globs), SystemTime, Utc::now, Local::now. A bare `Instant` is not
                      flagged, so that the simulation may name its own time type; every path to
                      the std type is.
    unsafe            the unsafe keyword: a backstop to #![forbid(unsafe_code)].
    source-include    #[path = ...] (also inside cfg_attr) and include!(...): they pull in source
                      from a file the gate may never scan, such as one outside the crate (DEF-4).
    lint-allow        allow(...) or expect(...) of clippy::disallowed_methods, disallowed_types, or
                      the groups that hold them (style, all), also inside cfg_attr: it would
                      silence clippy's half of these rules (the std transcendentals, HashMap,
                      Instant, mul_add) without a trace. Escaping it with a reason is how it is
                      counted and printed (VER-SPR-003-TEST GAP-5).
    float             f32 and f64 (the types, `as f64`, `f64::MAX`), conversions named after them
                      (to_f64, from_f32, as_f64), and float literals (0.5, 1e-3, 2f32, 1.) (SPR-010
                      T-04; DET-3). Threshold quantities are fixed point (rh_core::fixed); floats
                      are allowed only for declared float fields (below).

Paths match with any whitespace or comments around `::` (`std :: time :: Instant :: now()`, as
in macro bodies), and are reported with it removed (DEF-4).

The only RNG in plane A is the keyed one (ADR-0006); see clippy.toml for the matching clippy lists.

Escape, counted and printed so review sees every one:
    // LINT-ALLOW(forbidden-api): <reason>
on the offending line, or alone on the line directly above it. An escape without a reason, or one
that covers no finding, is itself a violation.

Declared float fields (05.1 §6 item 4: floats only in fields and continuous dynamics; GOV-003 §2).
A float finding is excused only by an escape whose reason starts with `f32_field <entity.column>`,
naming a column that a system spec declares with `type = "f32_field"` in an `[[owns]]` entry of
specs/systems/*.odd.toml (or of --specs DIR). Any other reason, or an undeclared column, leaves the
float a violation:
    pub moisture: f32, // LINT-ALLOW(forbidden-api): f32_field region.soil_moisture

The sampler float zone (SPR-003 Architect ruling, 2026-10-16, item 2). In workspace mode, the
float rule does not apply to the crate named exactly `rh-sample` (FLOAT_EXEMPT_CRATES), the vendored
samplers, which compute in f64 with libm and return integers or fixed point. The exemption is by
that name, which bin/lint_arch.py pins to (A, machinery), so the crate cannot be relabelled out of
plane A; every other rule still applies to it, and no other crate shares the exemption (a crate
named `rh-sampler` does not). `--scan` never exempts anything: it scans paths, not crates. The
summary line names the exemption.

    float-api         In the float zone only (ruling item 4: floats never leave the crate): the
                      public surface of its src/ (outside tests/, benches/, examples/) may not
                      name f32, f64 or libm. Checked: every `pub` item (not `pub(crate)` and
                      the like): a fn signature up to its body, a const or static type, a type
                      alias, a `pub use`, a struct, enum, trait or mod header, and a `pub` field;
                      and every `impl` header (a trait impl is public without `pub`). No escape
                      excuses it (VER-SPR-003-TEST GAP-1).
                      Also in the zone's src/, private or public: no `type … = …` alias that
                      names f32 or f64 (an associated type in an impl body included), since a
                      private alias would carry a float through a `pub` signature unnamed
                      (VER-SPR-003-TEST GAP-11; reported as `float-api` too).
                      RESIDUAL (Architect ruling on GAP-11): the rule is textual. It does not see
                      a float in a trait method signature inside a `pub trait` body, an
                      associated type defined elsewhere and named through a projection, a type
                      passed to a macro as a token (`mk!(leak, f64)`), or an opaque `impl Trait`
                      return that wraps a float. These are deliberate evasions, not accidents;
                      review catches them (SPR-003: the reviewer reads rh-sample's public items),
                      and BCK-002 tracks moving the rule to rustdoc JSON once its format is
                      stable.

DECISION (SPR-010 T-04): the float rule does not apply to files under a crate's tests/, benches/ or
examples/ directories. They never link into the engine, so they hold no threshold position, and a
statistical test may legitimately compute in floats. Every other rule still applies there, and the
float rule applies to all of src/ (unit-test modules included) and build.rs. ALTERNATIVES
CONSIDERED: scanning tests too (every statistical test would need an f32_field escape that names no
field), or skipping #[cfg(test)] modules (text cannot find their extent reliably).

Usage:  python -I bin/lint_forbidden.py [--manifest-path Cargo.toml] [--specs DIR]
        python -I bin/lint_forbidden.py --scan PATH [PATH ...]   (scan these files or directories
                                                                  as plane A; used by fixtures)
Exit:   0 clean, 1 violations, 2 the workspace, a file or a system spec could not be read.
Standard library only (GOV-003 §3).
"""

import argparse
import bisect
import json
import re
import subprocess
import sys
import tomllib
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Literal

RULE_ID = "V-DET-19"
EXIT_OK = 0
EXIT_VIOLATIONS = 1
EXIT_UNREADABLE = 2

ESCAPE_PATTERN = re.compile(r"LINT-ALLOW\(forbidden-api\)(?P<colon>:?)(?P<reason>.*)")
IDENT_CHAR = re.compile(r"[A-Za-z0-9_]")
RAW_STRING_START = re.compile(r'(?:b|c)?r(?P<hashes>#*)"')
FLOAT_FIELD_REASON = re.compile(r"f32_field\s+(?P<column>\S+)")
FLOAT_RULE = "float"
# Crate directories that never link into the engine: the float rule skips them (DECISION above).
FLOAT_FREE_DIRS = frozenset({"tests", "benches", "examples"})
# The plane-A crates exempt from the float rule, by their pinned names (module docstring; SPR-003):
# exactly one. python/tests/test_lint_forbidden.py asserts the set, so widening it is a reviewed
# change (VER-SPR-003-TEST GAP-3).
FLOAT_EXEMPT_CRATES = frozenset({"rh-sample"})
FLOAT_API_RULE = "float-api"
# Which float rule a file gets (scan_source): the float rule, the float-api rule, or neither.
FloatScope = Literal["rule", "zone", "none"]
FLOAT_API_WHY = (
    "the float zone's public API never exposes f32, f64 or libm (SPR-003 ruling item 4): return "
    "integers or rh_core::fixed values; no escape excuses this"
)
# `pub` that is not `pub(crate)`, `pub(super)` or `pub(in ...)`.
PUBLIC = re.compile(r"\bpub\b(?!\s*\()")
PUBLIC_KIND = re.compile(
    r"\s*(?:(?:const|unsafe|async|extern(?:\s+[^\s]+)?)\s+)*"
    r"(?P<kind>fn|struct|enum|union|trait|mod|const|static|type|use)\b"
)
# Where the public part of an item ends, by kind; anything else after `pub` is a field.
PUBLIC_END = {"fn": "{;", "const": "=;", "static": "=;", "type": ";", "use": ";"}
HEADER_END = "{;("
FIELD_END = ",})"
IMPL_HEADER = re.compile(r"\bimpl\b[^{;]*")
# A type alias (or an associated type in an impl body) whose definition names a float.
FLOAT_ALIAS = re.compile(r"\btype\s+\w+\s*(?:<[^=;]*>)?\s*=[^;]*")
FLOAT_API_TOKEN = re.compile(r"\b(?:f32|f64|libm)\b")
DEFAULT_SPECS = Path(__file__).resolve().parents[1] / "specs" / "systems"


def _rx(pattern: str) -> re.Pattern[str]:
    """Compile a rule pattern so that every ``::`` in it also matches with whitespace around it.

    Comments are already blanked to spaces, so this also covers ``std::time /* x */ ::Instant``.
    """
    return re.compile(pattern.replace("::", r"\s*::\s*"))


def _display(text: str) -> str:
    """Return matched text on one line, with whitespace around ``::`` removed."""
    return re.sub(r"\s*::\s*", "::", " ".join(text.split()))


@dataclass(frozen=True)
class Rule:
    """A forbidden API: its name, the pattern that finds it, and why it is forbidden."""

    name: str
    pattern: re.Pattern[str]
    why: str


RULES: tuple[Rule, ...] = (
    Rule(
        "hash-collections",
        _rx(r"\b\w*Hash(?:Map|Set)\b"),
        "unspecified iteration order; use BTreeMap/BTreeSet, an insertion-ordered map or a "
        "sorted Vec (GOV-003 §2)",
    ),
    Rule(
        "thread-rng",
        _rx(r"\bthread_rng\b|\bThreadRng\b|\brand::rng\b|\brand::\{[^}]*\brng\b"),
        "thread-local, unkeyed RNG; use the keyed RNG (ADR-0006)",
    ),
    Rule(
        "std-rng",
        _rx(r"\b(?:StdRng|SmallRng|OsRng|from_entropy|from_os_rng|getrandom)\b"),
        "unkeyed or entropy-seeded RNG; use the keyed RNG (ADR-0006)",
    ),
    Rule(
        "rand-random",
        _rx(
            r"\brand::random\b|\brand::\{[^}]*\brandom\b"
            r"|\brandom_(?:range|bool|ratio|iter)\b"
        ),
        "thread-local RNG; use the keyed RNG (ADR-0006)",
    ),
    Rule(
        "mul-add",
        _rx(r"\bmul_add\b"),
        "fused multiply-add rounds differently across targets (docs/13 §3)",
    ),
    Rule(
        "wall-clock",
        _rx(
            r"\bInstant::now\b|\bSystemTime\b|\bUNIX_EPOCH\b|\b(?:Utc|Local)::now\b"
            r"|\btime::(?:Instant|\*)|\btime::\{[^}]*\bInstant\b"
        ),
        "wall-clock read; simulation time comes from rh-core (GOV-003 §2)",
    ),
    Rule(
        "unsafe",
        _rx(r"\bunsafe\b"),
        "unsafe code (GOV-003 §2); backstop to #![forbid(unsafe_code)]",
    ),
    Rule(
        "source-include",
        _rx(r"#\s*!?\s*\[\s*path\b|\bcfg_attr\s*\([^)]*\bpath\s*=|\binclude\s*!\s*[(\[{]"),
        "pulls in source from a file this gate may never scan; keep plane-A code in its own "
        "crate's files, or escape with a reason (DEF-4)",
    ),
    Rule(
        "lint-allow",
        _rx(
            r"\b(?:allow|expect)\s*\([^)]*?"
            r"\bclippy::(?:disallowed_methods|disallowed_types|style|all)\b"
        ),
        "silences clippy's disallowed_methods/disallowed_types (clippy.toml: the std "
        "transcendentals, HashMap, Instant, mul_add); escape it with a reason so review sees it "
        "(VER-SPR-003-TEST GAP-5)",
    ),
    Rule(
        FLOAT_RULE,
        re.compile(
            r"\bf(?:32|64)\b"  # the types: f32, `as f64`, f64::MAX
            r"|\b\w+_f(?:32|64)\b"  # conversions named after them: to_f64, from_f32, as_f64
            r"|(?:(?<![\w.])|(?<=\.\.))\d[\d_]*"  # a number not after `.` (t.0.1), but after `..`
            r"(?:(?:\.\d[\d_]*)?(?:[eE][+-]?[\d_]+)?_?f(?:32|64)\b"  # 2f32, 1.5_f64
            r"|\.\d[\d_]*(?:[eE][+-]?[\d_]+)?"  # 0.5, 1.25e3 (not a range 0..5)
            r"|[eE][+-]?\d[\d_]*"  # 1e-3 (not hex 0x1e5)
            r"|\.(?![.\w]))"  # 1. (not 1..2, not 1.max(2))
        ),
        "float in plane A; threshold quantities are fixed point (rh_core::fixed: Q8_8, Q16_16, "
        "Money; DET-3). A declared float field needs `// LINT-ALLOW(forbidden-api): f32_field "
        "<entity.column>` naming a column a system spec declares f32_field",
    ),
)


@dataclass(frozen=True)
class Lexed:
    """A source file split into code (non-code blanked, offsets kept) and its line comments."""

    code: str
    line_comments: tuple[tuple[int, str], ...]  # (1-based line, comment text)


@dataclass(frozen=True)
class Finding:
    """One forbidden-API match, excused or not."""

    rule: str
    path: str
    line: int
    column: int
    text: str
    why: str


@dataclass(frozen=True)
class Escape:
    """One LINT-ALLOW comment and the line it covers."""

    path: str
    line: int
    covers: int
    reason: str


@dataclass
class Report:
    """What a scan found: violations, excused findings and the files read."""

    violations: list[str]
    escaped: list[str]
    files: int = 0


# --- Lexing -----------------------------------------------------------------------------------


def _blank(segment: str) -> str:
    """Replace every character except newlines with a space, keeping offsets and lines."""
    return "".join("\n" if ch == "\n" else " " for ch in segment)


def _block_comment_end(src: str, start: int) -> int:
    """Return the index after the (possibly nested) block comment starting at ``start``."""
    depth, i = 0, start
    while i < len(src):
        if src.startswith("/*", i):
            depth, i = depth + 1, i + 2
        elif src.startswith("*/", i):
            depth, i = depth - 1, i + 2
            if depth == 0:
                return i
        else:
            i += 1
    return len(src)


def _quoted_end(src: str, start: int, quote: str) -> int:
    """Return the index after the literal whose body starts at ``start`` and ends at ``quote``."""
    i = start
    while i < len(src):
        if src[i] == "\\":
            i += 2
        elif src[i] == quote:
            return i + 1
        else:
            i += 1
    return len(src)


def _char_literal_end(src: str, quote: int) -> int | None:
    """Return the index after a char literal opening at ``quote``, or None for a lifetime."""
    if src.startswith("\\", quote + 1):
        return _quoted_end(src, quote + 1, "'")
    if quote + 2 < len(src) and src[quote + 2] == "'":
        return quote + 3
    return None  # a lifetime or a loop label: 'a, 'outer


def _comment_end(src: str, i: int) -> int | None:
    """Return the end of the comment starting at ``i``, or None if there is none."""
    if src.startswith("//", i):
        newline = src.find("\n", i)
        return len(src) if newline < 0 else newline
    if src.startswith("/*", i):
        return _block_comment_end(src, i)
    return None


def _prefixed_literal_end(src: str, i: int) -> int | None:
    """Return the end of a b"", b'', c"", r"", br"" or cr"" literal starting at ``i``, or None."""
    if i > 0 and IDENT_CHAR.match(src[i - 1]):
        return None  # a prefix letter inside an identifier (`sub`, `bar`) starts no literal
    raw = RAW_STRING_START.match(src, i)
    if raw:
        closing = '"' + raw.group("hashes")
        end = src.find(closing, raw.end())
        return len(src) if end < 0 else end + len(closing)
    if src.startswith(('b"', 'c"'), i):
        return _quoted_end(src, i + 2, '"')
    if src.startswith("b'", i):
        return _char_literal_end(src, i + 1)
    return None


def _literal_end(src: str, i: int) -> int | None:
    """Return the end of the string or char literal starting at ``i``, or None if there is none.

    A bare quote opens a literal wherever it stands, even right after an identifier character
    (``return'"'``): whether ``'`` starts a char literal or a lifetime is decided by what follows
    it, never by what precedes it (DEF-3).
    """
    if src.startswith('"', i):
        return _quoted_end(src, i + 1, '"')
    if src.startswith("'", i):
        return _char_literal_end(src, i)
    return _prefixed_literal_end(src, i)


def _token_end(src: str, i: int) -> int | None:
    """Return the end of the comment or literal starting at ``i``, or None if ``i`` is code."""
    end = _comment_end(src, i)
    return end if end is not None else _literal_end(src, i)


def lex(source: str) -> Lexed:
    """Blank the comments and literals of Rust source and collect its ``//`` comments.

    Args:
        source: The text of a ``.rs`` file.

    Returns:
        The code with comments and literals replaced by spaces (newlines kept, so offsets and line
        numbers are unchanged), and every line comment with its line number.
    """
    parts: list[str] = []
    comments: list[tuple[int, str]] = []
    line, i = 1, 0
    while i < len(source):
        end = _token_end(source, i)
        if end is None:
            parts.append(source[i])
            line += source[i] == "\n"
            i += 1
            continue
        segment = source[i:end]
        if segment.startswith("//"):
            comments.append((line, segment))
        parts.append(_blank(segment))
        line += segment.count("\n")
        i = end
    return Lexed("".join(parts), tuple(comments))


# --- Scanning ---------------------------------------------------------------------------------


def find_forbidden(code: str, path: str) -> list[Finding]:
    """Return every forbidden-API match in blanked code, ordered by position."""
    line_starts = [0] + [m.end() for m in re.finditer("\n", code)]
    found: list[tuple[int, Finding]] = []
    for rule in RULES:
        for match in rule.pattern.finditer(code):
            line_index = bisect.bisect_right(line_starts, match.start()) - 1
            column = match.start() - line_starts[line_index] + 1
            text = _display(match.group(0))
            found.append(
                (match.start(), Finding(rule.name, path, line_index + 1, column, text, rule.why))
            )
    return [finding for _, finding in sorted(found, key=lambda pair: (pair[0], pair[1].rule))]


def find_escapes(lexed: Lexed, path: str) -> list[Escape]:
    """Return the LINT-ALLOW escapes in a file and the line each one covers."""
    code_lines = lexed.code.split("\n")
    escapes: list[Escape] = []
    for line, comment in lexed.line_comments:
        match = ESCAPE_PATTERN.search(comment)
        if match is None:
            continue
        reason = match.group("reason").strip() if match.group("colon") else ""
        alone = not code_lines[line - 1].strip()
        escapes.append(Escape(path, line, line + 1 if alone else line, reason))
    return escapes


def _float_escape_problem(escape: Escape, float_fields: frozenset[str]) -> str | None:
    """Return why an escape cannot excuse a float, or None if it names a declared float field."""
    match = FLOAT_FIELD_REASON.match(escape.reason)
    if match is None:
        return (
            f"only a declared float field may hold a float: the escape must read `f32_field "
            f"<entity.column>`, not `{escape.reason}`"
        )
    if match.group("column") not in float_fields:
        return (
            f"`{match.group('column')}` is not a column any system spec declares "
            '`type = "f32_field"` (specs/systems/*.odd.toml)'
        )
    return None


def _span_end(code: str, start: int, stops: str) -> int:
    """Return the index of the first character of ``stops`` at or after ``start``, or the end."""
    ends = [i for i in (code.find(ch, start) for ch in stops) if i >= 0]
    return min(ends, default=len(code))


def public_spans(code: str) -> list[tuple[int, int]]:
    """Return the (start, end) spans of blanked code that form a crate's public surface.

    A `pub` item's signature or header (its body excluded), a `pub` field, every `impl` header,
    and every type alias definition, private ones included (GAP-11). Spans are found textually;
    they over-approximate (a `pub` item in a private module counts), which is the safe side for a
    no-floats rule.
    """
    spans: list[tuple[int, int]] = []
    for match in PUBLIC.finditer(code):
        kind = PUBLIC_KIND.match(code, match.end())
        stops = FIELD_END if kind is None else PUBLIC_END.get(kind.group("kind"), HEADER_END)
        spans.append((match.start(), _span_end(code, match.end(), stops)))
    spans += [(m.start(), m.end()) for m in IMPL_HEADER.finditer(code)]
    spans += [(m.start(), m.end()) for m in FLOAT_ALIAS.finditer(code)]
    return spans


def find_float_api(code: str, path: str) -> list[Finding]:
    """Return each f32, f64 or libm in the public surface of blanked code (rule float-api)."""
    line_starts = [0] + [m.end() for m in re.finditer("\n", code)]
    seen: set[int] = set()
    found: list[Finding] = []
    for start, end in public_spans(code):
        for token in FLOAT_API_TOKEN.finditer(code, start, end):
            if token.start() in seen:
                continue
            seen.add(token.start())
            line_index = bisect.bisect_right(line_starts, token.start()) - 1
            column = token.start() - line_starts[line_index] + 1
            found.append(
                Finding(FLOAT_API_RULE, path, line_index + 1, column, token.group(0), FLOAT_API_WHY)
            )
    return sorted(found, key=lambda f: (f.line, f.column))


def scan_source(
    source: str,
    path: str,
    report: Report,
    float_fields: frozenset[str] = frozenset(),
    *,
    floats: FloatScope = "rule",
) -> None:
    """Scan one file's text and add its violations and excused findings to ``report``.

    Args:
        source: The text of a ``.rs`` file.
        path: The path reported for its findings.
        report: Where violations and excused findings are added.
        float_fields: The columns the system specs declare ``f32_field``; only an escape naming
            one of them excuses a float.
        floats: Which float rule applies to this file: ``"rule"`` the float rule (plane-A code);
            ``"zone"`` the float-api rule instead (the float zone's src/); ``"none"`` neither
            (tests/, benches/ and examples/; see the module DECISION).
    """
    lexed = lex(source)
    findings = [
        f for f in find_forbidden(lexed.code, path) if floats == "rule" or f.rule != FLOAT_RULE
    ]
    if floats == "zone":
        findings += find_float_api(lexed.code, path)
    escapes = find_escapes(lexed, path)
    valid = {e.covers: e for e in escapes if e.reason}
    covered_lines = {f.line for f in findings}
    for finding in findings:
        where = f"{finding.path}:{finding.line}:{finding.column}"
        escape = valid.get(finding.line)
        problem = None
        if escape is not None and finding.rule == FLOAT_RULE:
            problem = _float_escape_problem(escape, float_fields)
        elif escape is not None and finding.rule == FLOAT_API_RULE:
            problem = FLOAT_API_WHY
        if escape is not None and problem is None:
            report.escaped.append(
                f"ESCAPE [{RULE_ID}/{finding.rule}] `{finding.text}` at {where}; "
                f"reason: {escape.reason}"
            )
        else:
            why = finding.why if problem is None else problem
            report.violations.append(
                f"VIOLATION [{RULE_ID}/{finding.rule}] `{finding.text}` at {where}; {why}"
            )
    for escape in escapes:
        where = f"{escape.path}:{escape.line}"
        if not escape.reason:
            report.violations.append(
                f"VIOLATION [{RULE_ID}/escape] {where}: LINT-ALLOW(forbidden-api) needs a reason "
                "after the colon"
            )
        elif escape.covers not in covered_lines:
            report.violations.append(
                f"VIOLATION [{RULE_ID}/escape] {where}: LINT-ALLOW(forbidden-api) covers no "
                f"forbidden API on line {escape.covers}; remove it"
            )
    report.files += 1


def rust_files(roots: Iterable[Path]) -> list[Path]:
    """Return the ``.rs`` files under the given files or directories.

    A directory's own top-level ``target/`` (build output) is skipped; a source module that happens
    to be named ``target`` deeper down is not.
    """
    files: set[Path] = set()
    for root in roots:
        if root.is_file():
            files.add(root)
            continue
        files.update(p for p in root.rglob("*.rs") if p.relative_to(root).parts[0] != "target")
    return sorted(p for p in files if p.suffix == ".rs")


def display(path: Path) -> str:
    """Return ``path`` relative to the working directory when it lies below it."""
    try:
        return str(path.resolve().relative_to(Path.cwd().resolve()))
    except ValueError:
        return str(path)


def float_rule_applies(file: Path, root: Path) -> bool:
    """Return whether the float rule applies to ``file`` found under ``root``.

    A root given as a file is always checked; under a directory, the float rule skips the
    top-level tests/, benches/ and examples/ (the module DECISION).
    """
    if root.is_file():
        return True
    return file.relative_to(root).parts[0] not in FLOAT_FREE_DIRS


def scan_paths(
    paths: Iterable[Path],
    float_fields: frozenset[str] = frozenset(),
    float_exempt: frozenset[Path] = frozenset(),
) -> Report:
    """Scan every ``.rs`` file under ``paths`` as plane-A code.

    Args:
        paths: Files or directories to scan.
        float_fields: The columns the system specs declare ``f32_field``.
        float_exempt: Roots among ``paths`` whose files the float rule skips entirely (the float
            zone crate's directory); every other rule still applies to them.

    Raises:
        OSError: A file cannot be read.
        UnicodeDecodeError: A file is not UTF-8.
    """
    report = Report(violations=[], escaped=[])
    scopes: dict[Path, FloatScope] = {}
    for root in paths:
        for file in rust_files([root]):
            if not float_rule_applies(file, root):
                scopes.setdefault(file, "none")
            else:
                scopes.setdefault(file, "zone" if root in float_exempt else "rule")
    for file in sorted(scopes):
        source = file.read_text(encoding="utf-8")
        scan_source(source, display(file), report, float_fields, floats=scopes[file])
    return report


class SpecError(Exception):
    """A system spec could not be read or parsed."""


def declared_float_fields(specs: Path) -> frozenset[str]:
    """Return every column declared ``type = "f32_field"`` in the system specs under ``specs``.

    Reads each ``*.odd.toml`` except the template (``_*``) and collects its ``[[owns]]`` entries.

    Raises:
        SpecError: The directory is missing, or a spec cannot be read or parsed.
    """
    if not specs.is_dir():
        raise SpecError(f"no system-spec directory at {specs}")
    columns: set[str] = set()
    for spec in sorted(specs.glob("*.odd.toml")):
        if spec.name.startswith("_"):
            continue
        try:
            document = tomllib.loads(spec.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as err:
            raise SpecError(f"{spec}: {err}") from err
        owns = document.get("owns", [])
        for entry in owns if isinstance(owns, list) else []:
            if isinstance(entry, dict) and entry.get("type") == "f32_field":
                columns.add(str(entry.get("column")))
    return frozenset(columns)


# --- Workspace --------------------------------------------------------------------------------


class WorkspaceError(Exception):
    """The workspace metadata could not be read or understood."""


def plane_a_crate_dirs(manifest_path: Path) -> list[tuple[str, Path]]:
    """Return (name, directory) for each workspace crate to scan: plane A or undeclared.

    Raises:
        WorkspaceError: cargo is missing or fails, or its output has an unexpected shape.
    """
    command = ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline"]
    try:
        result = subprocess.run(
            [*command, "--manifest-path", str(manifest_path)],
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as err:
        raise WorkspaceError("cargo is not on PATH (RUN-001 §2)") from err
    if result.returncode != 0:
        raise WorkspaceError(f"cargo metadata failed:\n{result.stderr.strip()}")
    try:
        document = json.loads(result.stdout)
        members = set(document["workspace_members"])
        packages = [p for p in document["packages"] if p["id"] in members]
        crates = [
            (str(p["name"]), Path(p["manifest_path"]).parent)
            for p in packages
            if _plane(p.get("metadata")) in (None, "A")
        ]
    except (json.JSONDecodeError, KeyError, TypeError) as err:
        raise WorkspaceError(f"unexpected cargo metadata output: {err!r}") from err
    return sorted(crates)


def _plane(metadata: object) -> str | None:
    """Return the declared plane from a package's metadata table; None if absent or invalid."""
    section = metadata.get("rehoboam") if isinstance(metadata, dict) else None
    plane = section.get("plane") if isinstance(section, dict) else None
    return plane if plane in ("A", "B", "C", "tool") else None


def main(argv: Sequence[str] | None = None) -> int:
    """Run the gate and return the process exit status."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument(
        "--manifest-path",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "Cargo.toml",
        help="workspace Cargo.toml whose plane-A crates are scanned (default: the repository's)",
    )
    parser.add_argument("--scan", type=Path, nargs="+", help="scan these paths as plane A instead")
    parser.add_argument(
        "--specs",
        type=Path,
        default=DEFAULT_SPECS,
        help="directory of *.odd.toml system specs that declare f32_field columns "
        "(default: the repository's specs/systems)",
    )
    args = parser.parse_args(argv)
    try:
        float_fields = declared_float_fields(args.specs)
        if args.scan:
            scope = f"{len(args.scan)} path(s) given with --scan"
            report = scan_paths(args.scan, float_fields)
        else:
            crates = plane_a_crate_dirs(args.manifest_path)
            names = ", ".join(name for name, _ in crates)
            scope = f"{len(crates)} crates scanned as plane A ({names})"
            zone = frozenset(d for name, d in crates if name in FLOAT_EXEMPT_CRATES)
            if zone:
                exempt = ", ".join(sorted(n for n, d in crates if d in zone))
                scope += f"; float rule exempts {exempt} only (SPR-003 float zone)"
            report = scan_paths((directory for _, directory in crates), float_fields, zone)
    except (WorkspaceError, SpecError, OSError, UnicodeDecodeError) as err:
        print(f"lint_forbidden: cannot read the code to scan: {err}", file=sys.stderr)
        return EXIT_UNREADABLE
    for line in [*report.escaped, *report.violations]:
        print(line)
    print(
        f"lint_forbidden: {scope}; {report.files} files scanned; "
        f"{len(report.violations)} violations; {len(report.escaped)} escapes; "
        f"{len(float_fields)} declared f32_field columns"
    )
    return EXIT_VIOLATIONS if report.violations else EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
