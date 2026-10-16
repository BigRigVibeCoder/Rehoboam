"""V-WC-04 parity gate: the Rust manifest loader agrees with the Python reference (SPR-004 T-02).

Runs ``specs/manifest/manifest_v0.py validate`` (the reference) and ``rh-cli manifest validate``
(the Rust loader) on every manifest given, and compares what each says:

    verdict   VALID, INVALID, SCHEMA INVALID, or a load error (the reference crashes: exit 1);
    exit      the process exit code;
    errors    the error count (Registry errors, or Pydantic's validation-error count);
    warnings  the warning count (Registry-checked manifests only).

Any difference is a ``PARITY MISMATCH`` and the gate fails. Where the two disagree the Python
reference wins (CLAUDE.md; SPR-004), so a mismatch is a Rust bug until shown otherwise.

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
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / "specs" / "manifest" / "manifest_v0.py"
REGISTRY = ROOT / "specs" / "manifest" / "registry_v0.toml"
EXIT_OK = 0
EXIT_MISMATCH = 1
EXIT_UNRUNNABLE = 2

CHECKED = re.compile(r"^(VALID|INVALID): .* \((\d+) errors, (\d+) warnings\)", re.MULTILINE)
SCHEMA = re.compile(r"^(\d+) validation errors? for Manifest$", re.MULTILINE)


@dataclass(frozen=True)
class Result:
    """What one validator said about one manifest."""

    verdict: str
    exit_code: int
    errors: int
    warnings: int

    def render(self) -> str:
        """Return the result on one line."""
        return (
            f"{self.verdict} (exit {self.exit_code}, {self.errors} errors, "
            f"{self.warnings} warnings)"
        )


def parse(output: str, exit_code: int) -> Result:
    """Read a validator's report.

    Args:
        output: Everything the validator printed.
        exit_code: Its exit code.

    Returns:
        The verdict and counts. Output with neither a verdict line nor a schema count is a load
        error (the reference raises an uncaught exception; the Rust loader prints LOAD ERROR).
    """
    checked = CHECKED.search(output)
    if checked:
        return Result(checked[1], exit_code, int(checked[2]), int(checked[3]))
    schema = SCHEMA.search(output)
    if schema and "SCHEMA INVALID" in output:
        return Result("SCHEMA INVALID", exit_code, int(schema[1]), 0)
    return Result("LOAD ERROR", exit_code, 1, 0)


def run(command: Sequence[str]) -> Result:
    """Run one validator and parse its report.

    Raises:
        OSError: The command could not be started.
    """
    done = subprocess.run(list(command), capture_output=True, text=True, check=False, cwd=ROOT)
    return parse(done.stdout + done.stderr, done.returncode)


def main(argv: Sequence[str] | None = None) -> int:
    """Compare the two validators on every manifest given; return the exit code."""
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("manifests", nargs="+", type=Path)
    ap.add_argument("--rust-bin", type=Path, default=ROOT / "target" / "debug" / "rh-cli")
    ap.add_argument("--rust-registry", type=Path, default=REGISTRY)
    args = ap.parse_args(argv)
    if not args.rust_bin.is_file():
        print(f"cannot run the Rust loader: {args.rust_bin} not built (cargo build -p rh-cli)")
        return EXIT_UNRUNNABLE
    mismatches = 0
    for manifest in args.manifests:
        try:
            python = run([sys.executable, "-I", str(REFERENCE), "validate", str(manifest)])
            rust = run(
                [
                    str(args.rust_bin),
                    "manifest",
                    "validate",
                    str(manifest),
                    "--registry",
                    str(args.rust_registry),
                ]
            )
        except OSError as e:
            print(f"cannot run a validator on {manifest}: {e}")
            return EXIT_UNRUNNABLE
        if python == rust:
            print(f"AGREE           {manifest}: {python.render()}")
        else:
            mismatches += 1
            print(f"PARITY MISMATCH {manifest}: python {python.render()}; rust {rust.render()}")
    total = len(args.manifests)
    if mismatches:
        print(f"PARITY: {mismatches} of {total} manifests disagree (V-WC-04)")
        return EXIT_MISMATCH
    print(f"PARITY: all {total} manifests agree (V-WC-04)")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
