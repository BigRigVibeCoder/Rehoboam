"""The performance gate of the PR tier: instruction counts against a committed baseline (SPR-009).

V-PERF-04 (`docs/09`): more than 10% more instructions than the committed baseline fails the
build; a speed-up never re-baselines automatically (`docs/08` §9). Instruction counts come from
callgrind through iai-callgrind (valgrind as an outside tool), which makes them deterministic: two
runs of one build on one toolchain give the same counts, on a hosted runner or a laptop (T-02a).

Subcommands:

    measure    Run the instruction-count benches (`cargo bench -p rh-bench`) and write their counts
               in the baseline format (below) to --out. Every run starts from an empty
               iai-callgrind home, so no earlier run is compared or kept.
    check      Compare measured counts with a baseline: FAIL on any bench more than 10% above its
               baseline, on a bench missing from either side, or (exit 3) when the toolchain
               differs, which reports "toolchain changed: re-baseline required" instead of a false
               pass or fail (ruling 2). It never writes the baseline.
    gate       measure, then check.
    same       Two measured files must be identical (T-02 acceptance: two runs on one commit).
    integrity  Ruling 3: a baseline is regenerated, never typed. If the committed baseline differs
               from the base revision's (or is new), it must equal the tool's output byte for byte,
               and the pull request's title must carry [perf-baseline] and its body a line
               `perf-baseline-reason: <why>`. The title and body come from RH_PR_TITLE and
               RH_PR_BODY; outside a pull request (no RH_PR_TITLE, GITHUB_EVENT_NAME not
               pull_request) they cannot be checked and the report says so: CI checks them.
    record     Measure twice, require identical counts, and write the baseline. The only way a
               baseline file is written.

Baseline format `rehoboam/rh-bench/instructions/v1`, one record per line, `#` lines are comments:

    format rehoboam/rh-bench/instructions/v1
    toolchain <name> <version text>     rustc, llvm, host, valgrind, libc, iai-callgrind
    slot <bench> <note>                 a named, empty slot of docs/08 §9 (bench 4, ruling 1)
    bench <file>/<function>/<case> <Ir> one per measured case, in byte order of keys

Usage:  python -I bin/perf_gate.py gate --baseline crates/rh-bench/baselines/instructions_v1.txt \
            --out artifacts/perf/instructions.txt
Exit:   0 pass, 1 fail, 2 usage or a tool failed, 3 toolchain changed: re-baseline required.
Standard library only (GOV-003 §3).

REF: docs/08_performance_and_capacity.md §9; docs/09 V-PERF-04; docs/13 §4.3, §4.4;
CODEX/05_PROJECT/SPR-009_Benchmarks.md T-01, T-02, rulings 1-3.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib
from collections.abc import Iterable, Sequence
from dataclasses import dataclass, field
from pathlib import Path

FORMAT = "rehoboam/rh-bench/instructions/v1"
ROOT = Path(__file__).resolve().parents[1]
BASELINE = Path("crates/rh-bench/baselines/instructions_v1.txt")
PACKAGE = "rh-bench"

# REF: docs/08 §9 regression policy, V-PERF-04: "a slowdown of more than 10% fails the build".
LIMIT_PERCENT = 10

# The benches of docs/08 §9 that P0 owns, by number. Bench 4 is a named, empty slot by the
# director's ruling at G0 (SPR-009 ruling 1): stencil fields do not exist before P1a.
BENCH_TARGETS = ("keyframe_write_read", "plane_hashes")
SLOTS = (
    (
        "raster_8x512",
        "bench 4: no baseline until P1a, the first sprint with a spatial field (M1 or M3)",
    ),
)

PR_TAG = "[perf-baseline]"
REASON_KEY = "perf-baseline-reason:"

OK, FAIL, USAGE, REBASELINE = 0, 1, 2, 3


@dataclass
class Counts:
    """A baseline or measured file: its toolchain, slots and per-case counts."""

    toolchain: dict[str, str] = field(default_factory=dict)
    slots: dict[str, str] = field(default_factory=dict)
    benches: dict[str, int] = field(default_factory=dict)

    def render(self) -> str:
        """The canonical text: every value in a fixed order, so equal counts are equal bytes."""
        lines = [
            f"# {FORMAT}: callgrind instruction counts (Ir) of the PR-tier benches (SPR-009).",
            "# Written by `python -I bin/perf_gate.py record`, never by hand (SPR-009 ruling 3).",
            f"format {FORMAT}",
        ]
        lines += [f"toolchain {k} {v}" for k, v in self.toolchain.items()]
        lines += [f"slot {k} {v}" for k, v in sorted(self.slots.items())]
        lines += [f"bench {k} {v}" for k, v in sorted(self.benches.items())]
        return "\n".join(lines) + "\n"


class GateError(Exception):
    """A tool failed or a file is not in the baseline format (exit 2)."""


def parse(text: str, origin: str) -> Counts:
    """Parses the baseline format; anything else is a GateError naming the line."""
    counts = Counts()
    seen_format = False
    for number, raw in enumerate(text.splitlines(), 1):
        if not raw or raw.startswith("#"):
            continue
        kind, _, rest = raw.partition(" ")
        key, _, value = rest.partition(" ")
        if kind == "format" and rest == FORMAT:
            seen_format = True
        elif kind == "toolchain" and value:
            counts.toolchain[key] = value
        elif kind == "slot" and value:
            counts.slots[key] = value
        elif kind == "bench" and value.isdigit():
            counts.benches[key] = int(value)
        else:
            raise GateError(f"{origin}:{number}: not a {FORMAT} line: {raw!r}")
    if not seen_format:
        raise GateError(f"{origin}: no `format {FORMAT}` line")
    return counts


def read_counts(path: Path) -> Counts:
    """Reads and parses a baseline or measured file."""
    try:
        return parse(path.read_text(encoding="utf-8"), str(path))
    except OSError as e:
        raise GateError(f"cannot read {path}: {e}") from e


def _first_lines(cmd: Sequence[str]) -> list[str]:
    """The output lines of a version command, run in the root (rustup reads rust-toolchain.toml)."""
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=ROOT).stdout
    except (OSError, subprocess.CalledProcessError) as e:
        raise GateError(f"`{' '.join(cmd)}` failed: {e}") from e
    return out.splitlines()


def _first_line(cmd: Sequence[str]) -> str:
    lines = _first_lines(cmd)
    return lines[0].strip() if lines else ""


def locked_version(package: str) -> str:
    """A package's version in Cargo.lock.

    iai-callgrind's runner refuses to run a library of another version, so the library's locked
    version is the runner's too.
    """
    try:
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as e:
        raise GateError(f"cannot read Cargo.lock: {e}") from e
    versions = [str(p["version"]) for p in lock.get("package", []) if p.get("name") == package]
    if len(versions) != 1:
        raise GateError(f"Cargo.lock holds {len(versions)} versions of {package}, not one")
    return versions[0]


def toolchain() -> dict[str, str]:
    """The versions an instruction count depends on besides the code (ruling 2), in fixed order."""
    rustc = dict(line.split(": ", 1) for line in _first_lines(["rustc", "-vV"]) if ": " in line)
    return {
        "rustc": f"{rustc.get('release', '?')} {rustc.get('commit-hash', '?')}",
        "llvm": rustc.get("LLVM version", "?"),
        "host": rustc.get("host", "?"),
        "valgrind": _first_line(["valgrind", "--version"]),
        "libc": os.confstr("CS_GNU_LIBC_VERSION") or "?",
        "iai-callgrind": locked_version("iai-callgrind"),
    }


def instructions(record: dict[str, object]) -> tuple[str, int]:
    """The key and Ir of one iai-callgrind JSON summary line (a fresh home: only `Left`)."""
    try:
        exe = Path(str(record["benchmark_file"])).stem
        key = f"{exe}/{record['function_name']}/{record['id']}"
        profiles = record["profiles"]
        if not isinstance(profiles, list):
            raise TypeError("profiles is not a list")
        ir = profiles[0]["summaries"]["total"]["summary"]["Callgrind"]["Ir"]["metrics"]["Left"]
        return key, int(ir["Int"])
    except (KeyError, IndexError, TypeError, ValueError) as e:
        raise GateError(f"unexpected iai-callgrind summary: {e!r}") from e


def measure(features: Sequence[str]) -> Counts:
    """Runs the benches once from an empty iai-callgrind home and returns their counts."""
    cmd = ["cargo", "bench", "--locked", "--quiet", "-p", PACKAGE]
    for target in BENCH_TARGETS:
        cmd += ["--bench", target]
    if features:
        cmd += ["--features", ",".join(features)]
    cmd += ["--", "--output-format=json"]
    with tempfile.TemporaryDirectory(prefix="rh-iai-") as home:
        env = dict(os.environ, IAI_CALLGRIND_HOME=home)
        run = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, env=env, check=False)
    if run.returncode != 0:
        tail = "\n".join(run.stderr.splitlines()[-30:])
        raise GateError(f"`{' '.join(cmd)}` failed ({run.returncode}):\n{tail}")
    counts = Counts(toolchain=toolchain(), slots=dict(SLOTS))
    for line in run.stdout.splitlines():
        if line.strip():
            key, ir = instructions(json.loads(line))
            counts.benches[key] = ir
    if not counts.benches:
        raise GateError(f"`{' '.join(cmd)}` reported no benchmark")
    return counts


def write(path: Path, counts: Counts) -> None:
    """Writes counts in the canonical format, creating the directory."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(counts.render(), encoding="utf-8")


def toolchain_diff(baseline: Counts, measured: Counts) -> list[str]:
    """Every toolchain entry that differs, as `name: baseline -> measured`."""
    names = sorted(set(baseline.toolchain) | set(measured.toolchain))
    return [
        f"{n}: {baseline.toolchain.get(n, '(none)')} -> {measured.toolchain.get(n, '(none)')}"
        for n in names
        if baseline.toolchain.get(n) != measured.toolchain.get(n)
    ]


def percent(base: int, now: int) -> str:
    """The change from base to now, signed, two decimals."""
    return f"{(now - base) * 100 / base:+.2f}%" if base else "n/a"


def compare(baseline: Counts, measured: Counts) -> tuple[int, list[str]]:
    """The verdict of measured against baseline, and one report line per case."""
    changed = toolchain_diff(baseline, measured)
    if changed:
        lines = ["toolchain changed: re-baseline required (SPR-009 ruling 2)"]
        return REBASELINE, lines + [f"  {c}" for c in changed]
    verdict, lines = OK, []
    for key in sorted(set(baseline.benches) | set(measured.benches)):
        base, now = baseline.benches.get(key), measured.benches.get(key)
        if base is None or now is None:
            side = "baseline" if base is None else "this run"
            lines.append(f"MISSING {key}: not in {side}; a bench set change is a re-baseline")
            verdict = FAIL
        elif now * 100 > base * (100 + LIMIT_PERCENT):
            lines.append(
                f"REGRESSION {key}: {base} -> {now} ({percent(base, now)} > +{LIMIT_PERCENT}%)"
            )
            verdict = FAIL
        else:
            word = "faster (baseline kept)" if now < base else "ok"
            lines.append(f"{word} {key}: {base} -> {now} ({percent(base, now)})")
    if baseline.slots != measured.slots:
        lines.append(f"SLOTS differ: {baseline.slots} -> {measured.slots}")
        verdict = FAIL
    return verdict, lines


def cmd_check(baseline: Path, measured: Path) -> int:
    """Prints the comparison and returns its verdict."""
    verdict, lines = compare(read_counts(baseline), read_counts(measured))
    print("\n".join(lines))
    word = {OK: "PASS", FAIL: "FAIL", REBASELINE: "RE-BASELINE REQUIRED"}[verdict]
    print(f"V-PERF-04 {word}: {len(read_counts(measured).benches)} cases against {baseline}")
    return verdict


def cmd_same(first: Path, second: Path) -> int:
    """Two runs on one commit must give identical counts (T-02 acceptance)."""
    a, b = read_counts(first), read_counts(second)
    diffs = [
        f"{k}: {a.benches.get(k)} != {b.benches.get(k)}"
        for k in sorted(set(a.benches) | set(b.benches))
        if a.benches.get(k) != b.benches.get(k)
    ]
    if diffs or a.toolchain != b.toolchain:
        print("NOT REPRODUCIBLE: two runs on one commit differ\n" + "\n".join(diffs))
        return FAIL
    print(f"identical counts in two runs: {len(a.benches)} cases")
    return OK


def git(*args: str) -> subprocess.CompletedProcess[str]:
    """Runs git in the repository root."""
    return subprocess.run(["git", *args], capture_output=True, text=True, cwd=ROOT, check=False)


def base_text(base_ref: str, path: Path) -> str | None:
    """The baseline at the merge base of HEAD and base_ref, or None if it did not exist there."""
    merge_base = git("merge-base", "HEAD", base_ref)
    if merge_base.returncode != 0:
        raise GateError(f"no merge base of HEAD and {base_ref}: {merge_base.stderr.strip()}")
    shown = git("show", f"{merge_base.stdout.strip()}:{path.as_posix()}")
    return shown.stdout if shown.returncode == 0 else None


def pr_context() -> tuple[str, str] | None:
    """The pull request's title and body, or None outside a pull request."""
    # A workflow passes an empty title outside pull_request events: treat it as no context.
    title = os.environ.get("RH_PR_TITLE") or None
    if title is None and os.environ.get("GITHUB_EVENT_NAME") == "pull_request":
        raise GateError("a pull_request run without RH_PR_TITLE: the workflow must pass it")
    return None if title is None else (title, os.environ.get("RH_PR_BODY", ""))


def has_reason(body: str) -> bool:
    """Whether the body has a `perf-baseline-reason:` line with text after the key."""
    return any(
        line.strip().lower().startswith(REASON_KEY) and line.strip()[len(REASON_KEY) :].strip()
        for line in body.splitlines()
    )


def integrity_problems(committed: str, measured: str, pr: tuple[str, str] | None) -> list[str]:
    """Why a changed baseline is not acceptable (ruling 3); empty if it is."""
    problems = []
    if committed != measured:
        problems.append(
            "the changed baseline does not equal the tool's output: a baseline is regenerated "
            "with `python -I bin/perf_gate.py record`, never typed (SPR-009 ruling 3)"
        )
    if pr is None:
        print("PR title and body not checked: no pull-request context here; CI checks them")
        return problems
    title, body = pr
    if PR_TAG not in title:
        problems.append(f"a changed baseline needs {PR_TAG} in the pull request's title: {title!r}")
    if not has_reason(body):
        problems.append(f"a changed baseline needs a `{REASON_KEY} <why>` line in the PR body")
    return problems


def cmd_integrity(
    baseline: Path, measured: Path, base_ref: str | None, base_file: Path | None
) -> int:
    """Ruling 3 for one baseline file."""
    committed = baseline.read_text(encoding="utf-8")
    if base_file is not None:
        before: str | None = base_file.read_text(encoding="utf-8")
    else:
        before = base_text(base_ref or "origin/main", baseline.resolve().relative_to(ROOT))
    if before == committed:
        print(f"{baseline}: unchanged from the base; the regression check governs it")
        return OK
    state = "new" if before is None else "changed from the base"
    problems = integrity_problems(committed, measured.read_text(encoding="utf-8"), pr_context())
    for p in problems:
        print(f"INTEGRITY {baseline} ({state}): {p}")
    if problems:
        return FAIL
    print(
        f"{baseline} ({state}): equals the tool's output, in a {PR_TAG} pull request with a reason"
    )
    return OK


def cmd_record(baseline: Path, features: Sequence[str]) -> int:
    """Measures twice, requires equal counts, and writes the baseline."""
    first, second = measure(features), measure(features)
    if first.render() != second.render():
        print("NOT REPRODUCIBLE: two runs differ; nothing written")
        return FAIL
    write(baseline, first)
    print(f"wrote {baseline}: {len(first.benches)} cases; commit it in a {PR_TAG} pull request")
    return OK


def build_parser() -> argparse.ArgumentParser:
    """The command-line interface."""
    p = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    sub = p.add_subparsers(dest="command", required=True)
    for name in ("measure", "gate", "record"):
        s = sub.add_parser(name)
        s.add_argument("--features", action="append", default=[])
        if name != "record":
            s.add_argument("--out", type=Path, required=True)
        if name != "measure":
            s.add_argument("--baseline", type=Path, default=BASELINE)
    s = sub.add_parser("check")
    s.add_argument("--baseline", type=Path, default=BASELINE)
    s.add_argument("--measured", type=Path, required=True)
    s = sub.add_parser("same")
    s.add_argument("first", type=Path)
    s.add_argument("second", type=Path)
    s = sub.add_parser("integrity")
    s.add_argument("--baseline", type=Path, default=BASELINE)
    s.add_argument("--measured", type=Path, required=True)
    s.add_argument("--base-ref", default=None, help="default origin/main")
    s.add_argument("--base-file", type=Path, default=None, help="the base revision's baseline")
    return p


def dispatch(a: argparse.Namespace) -> int:
    """Runs one subcommand."""
    if a.command == "measure":
        write(a.out, measure(a.features))
        print(f"wrote {a.out}")
        return OK
    if a.command == "gate":
        write(a.out, measure(a.features))
        return cmd_check(a.baseline, a.out)
    if a.command == "check":
        return cmd_check(a.baseline, a.measured)
    if a.command == "same":
        return cmd_same(a.first, a.second)
    if a.command == "integrity":
        return cmd_integrity(a.baseline, a.measured, a.base_ref, a.base_file)
    return cmd_record(a.baseline, a.features)


def main(argv: Iterable[str] | None = None) -> int:
    """Entry point."""
    args = build_parser().parse_args(None if argv is None else list(argv))
    for tool in ("cargo", "valgrind", "iai-callgrind-runner"):
        if args.command in ("measure", "gate", "record") and shutil.which(tool) is None:
            print(f"perf_gate: {tool} not found (RUN-001 §3)", file=sys.stderr)
            return USAGE
    try:
        return dispatch(args)
    except GateError as e:
        print(f"perf_gate: {e}", file=sys.stderr)
        return USAGE


if __name__ == "__main__":
    sys.exit(main())
