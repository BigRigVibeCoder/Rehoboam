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

The only RNG in plane A is the keyed one (ADR-0006); see clippy.toml for the matching clippy lists.

Escape, counted and printed so review sees every one:
    // LINT-ALLOW(forbidden-api): <reason>
on the offending line, or alone on the line directly above it. An escape without a reason, or one
that covers no finding, is itself a violation.

TODO(SPR-003/SPR-005, fixed-point types): V-DET-19 also forbids float types in threshold positions
(DET-3). Text cannot tell a threshold from a declared float field, so this gate does not pretend to;
the check arrives with the fixed-point types in rh-core as a type-level rule. Until then it is an
audit item.

Usage:  python -I bin/lint_forbidden.py [--manifest-path Cargo.toml]
        python -I bin/lint_forbidden.py --scan PATH [PATH ...]   (scan these files or directories
                                                                  as plane A; used by fixtures)
Exit:   0 clean, 1 violations, 2 the workspace or a file could not be read.
Standard library only (GOV-003 §3).
"""

import argparse
import bisect
import json
import re
import subprocess
import sys
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path

RULE_ID = "V-DET-19"
EXIT_OK = 0
EXIT_VIOLATIONS = 1
EXIT_UNREADABLE = 2

ESCAPE_PATTERN = re.compile(r"LINT-ALLOW\(forbidden-api\)(?P<colon>:?)(?P<reason>.*)")
IDENT_CHAR = re.compile(r"[A-Za-z0-9_]")
RAW_STRING_START = re.compile(r'(?:b|c)?r(?P<hashes>#*)"')


@dataclass(frozen=True)
class Rule:
    """A forbidden API: its name, the pattern that finds it, and why it is forbidden."""

    name: str
    pattern: re.Pattern[str]
    why: str


RULES: tuple[Rule, ...] = (
    Rule(
        "hash-collections",
        re.compile(r"\b\w*Hash(?:Map|Set)\b"),
        "unspecified iteration order; use BTreeMap/BTreeSet, an insertion-ordered map or a "
        "sorted Vec (GOV-003 §2)",
    ),
    Rule(
        "thread-rng",
        re.compile(r"\bthread_rng\b|\bThreadRng\b|\brand::rng\b|\brand::\{[^}]*\brng\b"),
        "thread-local, unkeyed RNG; use the keyed RNG (ADR-0006)",
    ),
    Rule(
        "std-rng",
        re.compile(r"\b(?:StdRng|SmallRng|OsRng|from_entropy|from_os_rng|getrandom)\b"),
        "unkeyed or entropy-seeded RNG; use the keyed RNG (ADR-0006)",
    ),
    Rule(
        "rand-random",
        re.compile(
            r"\brand::random\b|\brand::\{[^}]*\brandom\b"
            r"|\brandom_(?:range|bool|ratio|iter)\b"
        ),
        "thread-local RNG; use the keyed RNG (ADR-0006)",
    ),
    Rule(
        "mul-add",
        re.compile(r"\bmul_add\b"),
        "fused multiply-add rounds differently across targets (docs/13 §3)",
    ),
    Rule(
        "wall-clock",
        re.compile(
            r"\bInstant::now\b|\bSystemTime\b|\bUNIX_EPOCH\b|\b(?:Utc|Local)::now\b"
            r"|\btime::(?:Instant|\*)|\btime::\{[^}]*\bInstant\b"
        ),
        "wall-clock read; simulation time comes from rh-core (GOV-003 §2)",
    ),
    Rule(
        "unsafe",
        re.compile(r"\bunsafe\b"),
        "unsafe code (GOV-003 §2); backstop to #![forbid(unsafe_code)]",
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


def _literal_end(src: str, i: int) -> int | None:
    """Return the end of the string or char literal starting at ``i``, or None if there is none."""
    if i > 0 and IDENT_CHAR.match(src[i - 1]):
        return None  # inside an identifier: not a b"", r"" or c"" prefix
    raw = RAW_STRING_START.match(src, i)
    if raw:
        closing = '"' + raw.group("hashes")
        end = src.find(closing, raw.end())
        return len(src) if end < 0 else end + len(closing)
    prefix = 1 if src.startswith(('b"', 'c"', "b'"), i) else 0
    if src.startswith('"', i + prefix):
        return _quoted_end(src, i + prefix + 1, '"')
    if src.startswith("'", i + prefix):
        return _char_literal_end(src, i + prefix)
    return None


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
            text = " ".join(match.group(0).split())
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


def scan_source(source: str, path: str, report: Report) -> None:
    """Scan one file's text and add its violations and excused findings to ``report``."""
    lexed = lex(source)
    findings = find_forbidden(lexed.code, path)
    escapes = find_escapes(lexed, path)
    valid = {e.covers: e for e in escapes if e.reason}
    covered_lines = {f.line for f in findings}
    for finding in findings:
        where = f"{finding.path}:{finding.line}:{finding.column}"
        escape = valid.get(finding.line)
        if escape is not None:
            report.escaped.append(
                f"ESCAPE [{RULE_ID}/{finding.rule}] `{finding.text}` at {where}; "
                f"reason: {escape.reason}"
            )
        else:
            report.violations.append(
                f"VIOLATION [{RULE_ID}/{finding.rule}] `{finding.text}` at {where}; {finding.why}"
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


def scan_paths(paths: Iterable[Path]) -> Report:
    """Scan every ``.rs`` file under ``paths`` as plane-A code.

    Raises:
        OSError: A file cannot be read.
        UnicodeDecodeError: A file is not UTF-8.
    """
    report = Report(violations=[], escaped=[])
    for file in rust_files(paths):
        scan_source(file.read_text(encoding="utf-8"), display(file), report)
    return report


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
    args = parser.parse_args(argv)
    try:
        if args.scan:
            scope = f"{len(args.scan)} path(s) given with --scan"
            report = scan_paths(args.scan)
        else:
            crates = plane_a_crate_dirs(args.manifest_path)
            names = ", ".join(name for name, _ in crates)
            scope = f"{len(crates)} crates scanned as plane A ({names})"
            report = scan_paths(directory for _, directory in crates)
    except (WorkspaceError, OSError, UnicodeDecodeError) as err:
        print(f"lint_forbidden: cannot read the code to scan: {err}", file=sys.stderr)
        return EXIT_UNREADABLE
    for line in [*report.escaped, *report.violations]:
        print(line)
    print(
        f"lint_forbidden: {scope}; {report.files} files scanned; "
        f"{len(report.violations)} violations; {len(report.escaped)} escapes"
    )
    return EXIT_VIOLATIONS if report.violations else EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
