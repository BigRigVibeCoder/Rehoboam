"""V-WC-04 parity gate: the Rust manifest loader agrees with the Python reference (SPR-004 T-02).

Runs ``specs/manifest/manifest_v0.py validate`` (the reference) and ``rh-cli manifest validate``
(the Rust loader) on every manifest given, and compares what each says:

    verdict   VALID, INVALID, SCHEMA INVALID, or a load error (the reference crashes: exit 1);
    exit      the process exit code;
    errors    the error count (Registry errors, or Pydantic's validation-error count);
    warnings  the warning count (Registry-checked manifests only);
    lines     for exits 0 and 1, every ``[ERROR]``/``[WARNING]`` line (path and message) in
              order; for exit 2, every ``(loc, type)`` pair in order (SPR-004 GAP-3 ruling).

Any difference is a ``PARITY MISMATCH`` and the gate fails. Where the two disagree the Python
reference wins (CLAUDE.md; SPR-004), so a mismatch is a Rust bug until shown otherwise.

A ruled divergence (an out-of-contract form ruled so, 05.5 §3) is declared in one of a fixture's
first five lines: ``# parity-divergence: PYTHON-VERDICT / RUST-VERDICT "rust diagnostic"``. It
passes only if both verdicts are exactly those and the Rust output contains the diagnostic, so
a different Rust refusal cannot pass. The header is honoured only for fixtures under
``tests/manifest_parity/``, and two equal verdicts are not a divergence (SPR-004 DEF-15).

A manifest ``NAME.yaml`` with a ``NAME.registry.toml`` beside it is validated against that
Registry on both sides (a corpus case that needs a Registry the real one cannot provide).

Usage:  python -I bin/manifest_parity.py [--rust-bin target/debug/rh-cli]
                                          [--rust-registry PATH] MANIFEST [MANIFEST ...]
        --rust-registry gives the Rust side a different Registry; the positive control uses it to
        plant a disagreement.
Exit:   0 all agree, 1 a mismatch, 2 a tool could not be run.
Standard library only (GOV-003 §3).
"""

import argparse
import re
import subprocess
import sys
from collections.abc import Sequence
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / "specs" / "manifest" / "manifest_v0.py"
REGISTRY = ROOT / "specs" / "manifest" / "registry_v0.toml"
EXIT_OK = 0
EXIT_MISMATCH = 1
EXIT_UNRUNNABLE = 2

CHECKED = re.compile(r"^(VALID|INVALID): .* \((\d+) errors, (\d+) warnings\)", re.MULTILINE)
SCHEMA = re.compile(r"^(\d+) validation errors? for Manifest$", re.MULTILINE)
ISSUE = re.compile(r"^\[(?:ERROR|WARNING)\] .*$", re.MULTILINE)
ERROR_TYPE = re.compile(r"\[type=([a-z_]+)")
CORPUS = ROOT / "tests" / "manifest_parity"
VERDICTS = ("VALID", "INVALID", "SCHEMA INVALID", "LOAD ERROR")
DIVERGENCE = re.compile(r'^# parity-divergence: (.+?) / (.+?) "(.+)"\s*$')
HEADER_LINES = 5
RULED: list[Path] = []  # accepted divergences seen in this run


@dataclass(frozen=True)
class Divergence:
    """A ruled divergence declared by a corpus fixture."""

    python: str
    rust: str
    diagnostic: str


def declared_divergence(manifest: Path, text: str) -> Divergence | str | None:
    """Read a fixture's ``# parity-divergence:`` header (SPR-004 DEF-15).

    Returns:
        None without a header (or outside ``tests/manifest_parity/``, where a header is not
        honoured); the Divergence; or an error message for a malformed header.
    """
    if not manifest.resolve().is_relative_to(CORPUS.resolve()):
        return None
    for line in text.splitlines()[:HEADER_LINES]:
        if not line.startswith("# parity-divergence:"):
            continue
        m = DIVERGENCE.match(line)
        if not m:
            return 'malformed header: need PYTHON-VERDICT / RUST-VERDICT "rust diagnostic"'
        if m[1] not in VERDICTS or m[2] not in VERDICTS:
            return f"unknown verdict in header (one of {', '.join(VERDICTS)})"
        if m[1] == m[2]:
            return "equal verdicts are not a divergence"
        return Divergence(m[1], m[2], m[3])
    return None


@dataclass(frozen=True)
class Result:
    """What one validator said about one manifest."""

    verdict: str
    exit_code: int
    errors: int
    warnings: int
    lines: tuple[str, ...] = ()
    output: str = field(default="", compare=False)

    def render(self) -> str:
        """Return the result on one line."""
        return (
            f"{self.verdict} (exit {self.exit_code}, {self.errors} errors, "
            f"{self.warnings} warnings)"
        )


def schema_pairs(output: str) -> tuple[str, ...]:
    """Return Pydantic's ``loc | type`` pairs, in order, from a SCHEMA INVALID report.

    A location line has no indent; the message under it is indented two spaces and ends with
    ``[type=...]``; a root error has a message and no location line.
    """
    pairs: list[str] = []
    loc = ""
    for line in output.splitlines()[2:]:
        if not line.startswith(" "):
            loc = line
        elif line.startswith("  ") and not line.startswith("   "):
            kind = ERROR_TYPE.search(line)
            pairs.append(f"{loc} | {kind[1] if kind else '?'}")
            loc = ""
    return tuple(pairs)


def parse(output: str, exit_code: int) -> Result:
    """Read a validator's report.

    Args:
        output: Everything the validator printed.
        exit_code: Its exit code.

    Returns:
        The verdict, counts and comparable lines. Output with neither a verdict line nor a
        schema count is a load error (the reference raises an uncaught exception; the Rust
        loader prints LOAD ERROR).
    """
    checked = CHECKED.search(output)
    if checked:
        issues = tuple(ISSUE.findall(output))
        return Result(checked[1], exit_code, int(checked[2]), int(checked[3]), issues, output)
    schema = SCHEMA.search(output)
    if schema and "SCHEMA INVALID" in output:
        start = output.index("SCHEMA INVALID")
        pairs = schema_pairs(output[start:])
        return Result("SCHEMA INVALID", exit_code, int(schema[1]), 0, pairs, output)
    return Result("LOAD ERROR", exit_code, 1, 0, (), output)


def divergence_holds(declared: Divergence, python: Result, rust: Result) -> bool:
    """Whether both verdicts are the declared ones and Rust said the declared diagnostic."""
    verdicts = (python.verdict, rust.verdict) == (declared.python, declared.rust)
    return verdicts and declared.diagnostic in rust.output


def run(command: Sequence[str]) -> Result:
    """Run one validator and parse its report.

    Raises:
        OSError: The command could not be started.
    """
    done = subprocess.run(list(command), capture_output=True, text=True, check=False, cwd=ROOT)
    return parse(done.stdout + done.stderr, done.returncode)


def first_difference(python: Result, rust: Result) -> str:
    """Name the first line on which the two reports differ."""
    for i, (p, r) in enumerate(zip(python.lines, rust.lines, strict=False)):
        if p != r:
            return f"line {i + 1}: python {p!r}; rust {r!r}"
    if len(python.lines) != len(rust.lines):
        return f"python has {len(python.lines)} lines, rust {len(rust.lines)}"
    return "verdict or counts"


def compare(manifest: Path, rust_bin: Path, rust_registry: Path | None) -> bool:
    """Validate one manifest on both sides, print the outcome, and return whether they agree."""
    beside = manifest.with_name(manifest.name.removesuffix(".yaml") + ".registry.toml")
    registry = beside if beside.is_file() else REGISTRY
    python = run(
        [
            sys.executable,
            "-I",
            str(REFERENCE),
            "validate",
            str(manifest),
            "--registry",
            str(registry),
        ]
    )
    rust_reg = rust_registry or registry
    rust = run([str(rust_bin), "manifest", "validate", str(manifest), "--registry", str(rust_reg)])
    declared = declared_divergence(manifest, manifest.read_text(encoding="utf-8"))
    if isinstance(declared, str):
        print(f"PARITY MISMATCH {manifest}: parity-divergence header: {declared}")
        return False
    if declared is not None:
        if divergence_holds(declared, python, rust):
            print(f"DIVERGE (ruled) {manifest}: python {python.verdict}; rust {rust.verdict}")
            RULED.append(manifest)
            return True
        print(
            f"PARITY MISMATCH {manifest}: declared divergence {declared.python} / "
            f"{declared.rust} {declared.diagnostic!r}; got python {python.verdict}; "
            f"rust {rust.verdict} {rust.output.strip()[-160:]!r}"
        )
        return False
    if python == rust:
        print(f"AGREE           {manifest}: {python.render()}")
        return True
    print(
        f"PARITY MISMATCH {manifest}: python {python.render()}; rust {rust.render()}; "
        f"{first_difference(python, rust)}"
    )
    return False


def main(argv: Sequence[str] | None = None) -> int:
    """Compare the two validators on every manifest given; return the exit code."""
    ap = argparse.ArgumentParser(description="V-WC-04 parity gate (see the module docstring).")
    ap.add_argument("manifests", nargs="+", type=Path)
    ap.add_argument("--rust-bin", type=Path, default=ROOT / "target" / "debug" / "rh-cli")
    ap.add_argument("--rust-registry", type=Path, default=None)
    args = ap.parse_args(argv)
    if not args.rust_bin.is_file():
        print(f"cannot run the Rust loader: {args.rust_bin} not built (cargo build -p rh-cli)")
        return EXIT_UNRUNNABLE
    mismatches = 0
    for manifest in args.manifests:
        try:
            agree = compare(manifest, args.rust_bin, args.rust_registry)
        except OSError as e:
            print(f"cannot run a validator on {manifest}: {e}")
            return EXIT_UNRUNNABLE
        mismatches += 0 if agree else 1
    total = len(args.manifests)
    if mismatches:
        print(f"PARITY: {mismatches} of {total} manifests disagree (V-WC-04)")
        return EXIT_MISMATCH
    ruled = f"; {len(RULED)} of them a ruled divergence" if RULED else ""
    print(f"PARITY: all {total} manifests agree, line for line (V-WC-04){ruled}")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
