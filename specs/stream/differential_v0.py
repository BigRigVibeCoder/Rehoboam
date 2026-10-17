"""rh-stream's canonical parser against the reference, line for line and file for file.

REV-SPR-013 MAJ-1; canonical_v0.md §3: rh-stream's parser and ``canonical_v0.py`` must refuse the
same lines. This tool, independently of rh-stream, builds a corpus:

  - lines: the reference's generated records of every kind over fixed seeds, the golden file's
    record lines and the self-test's planted refusals, each with mutants (byte replacements,
    insertions, deletions and token swaps, ASCII and two non-ASCII bytes);
  - files: the self-test's valid and planted files, and mutants of the valid file's header line;

then runs ``rh-cli canonical parse`` and ``rh-cli canonical files`` on it and requires the same
verdict from both parsers for every item: refused by both, or accepted by both with the same
rendered line (lines) or the same digest (files).

``--control relax-header`` drops the reference's header domain check (a u32 timeline, canonical
string tokens): the planted out-of-domain headers then pass the reference and are refused by
rh-stream, so the tool must FAIL with ``DIFFERENTIAL DISAGREES`` (the positive control).

Usage: python -I specs/stream/differential_v0.py --rust-bin target/release/rh-cli
           [--control relax-header]
Exit:  0 no disagreement, 1 a disagreement, 2 the corpus could not be run.
"""

from __future__ import annotations

import argparse
import importlib.util
import random
import subprocess
import sys
import tempfile
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve().parent
CANONICAL = HERE / "canonical_v0.py"
EXIT_OK, EXIT_DIFFER, EXIT_TOOL = 0, 1, 2
SEEDS = 12
RECORDS_PER_KIND = 40
MUTANTS_PER_LINE = 12
HEADER_MUTANTS = 3000
ALPHABET = b" 0123456789abcdefABCDEF-~%[],/_:.e\t\rZ#=" + "é".encode()


def _load(path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(path.stem, path)
    if spec is None or spec.loader is None:
        raise ImportError(str(path))
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module
    spec.loader.exec_module(module)
    return module


CANON = _load(CANONICAL)


def mutate(rng: random.Random, data: bytes) -> bytes:
    """One to three random edits of ``data``."""
    b = bytearray(data)
    for _ in range(rng.randint(1, 3)):
        at = rng.randint(0, len(b))
        op = rng.randrange(4)
        if op == 0 and at < len(b):
            b[at] = rng.choice(ALPHABET)
        elif op == 1:
            b.insert(at, rng.choice(ALPHABET))
        elif op == 2 and at < len(b):  # noqa: PLR2004
            del b[at]
        else:
            words = bytes(b).split(b" ")
            if len(words) > 2:  # noqa: PLR2004
                i, j = rng.randrange(len(words)), rng.randrange(len(words))
                words[i], words[j] = words[j], words[i]
                b = bytearray(b" ".join(words))
    return bytes(b)


def line_corpus() -> list[bytes]:
    """The lines: generated, golden and planted records, each with its mutants (no LF inside)."""
    base: list[bytes] = []
    for seed in range(SEEDS):
        rng = random.Random(seed)
        for kind in CANON.KINDS:
            for _ in range(RECORDS_PER_KIND):
                base.append(CANON.render(CANON.generate(rng, kind)).encode())
    golden = CANON.GOLDEN.read_text(encoding="ascii").splitlines()
    base += [ln.encode() for ln in golden if ln.split(" ", 1)[0] in CANON.KINDS]
    base += [ln.encode("utf-8", "surrogateescape") for ln, _ in CANON.REFUSED]
    rng = random.Random(0xD1FF)
    lines = list(base)
    for ln in base:
        lines += [mutate(rng, ln) for _ in range(MUTANTS_PER_LINE)]
    return [ln for ln in lines if b"\n" not in ln]


def file_corpus() -> list[bytes]:
    """The files: valid, planted, and mutants of the valid file's header line."""
    valid = CANON.VALID_FILE.encode()
    header, rest = valid.split(b"\n", 1)
    rng = random.Random(0xF11E)
    files = [valid] + [data for data, _ in CANON.REFUSED_FILES]
    files += [mutate(rng, header) + b"\n" + rest for _ in range(HEADER_MUTANTS)]
    return files


def reference_line(raw: bytes) -> str:
    """The reference's verdict on one line."""
    try:
        return "ok " + str(CANON.render(CANON.parse(raw.decode("utf-8"))))
    except (CanonicalErrorType, UnicodeDecodeError):
        return "err"


def reference_file(path: Path) -> str:
    """The reference's verdict on one file."""
    try:
        return "ok " + bytes(CANON.digest_file(path)).hex()
    except (CanonicalErrorType, UnicodeDecodeError):
        return "err"


CanonicalErrorType = CANON.CanonicalError


def compare_lines(rust_bin: Path) -> tuple[list[str], int, int] | None:
    """Line disagreements, corpus size and rh-stream's accepted count; None on a tool error."""
    lines = line_corpus()
    run = subprocess.run(
        [str(rust_bin), "canonical", "parse"],
        input=b"".join(ln + b"\n" for ln in lines),
        capture_output=True,
        check=False,
    )
    rust = run.stdout.decode("ascii", "replace").split("\n")[:-1]
    if run.returncode != 0 or len(rust) != len(lines):
        print(f"differential: rh-cli canonical parse failed ({run.returncode}): {run.stderr!r}")
        return None
    problems = [
        f"LINE {raw!r}: reference {ref[:60]!r}, rh-stream {got[:60]!r}"
        for raw, got in zip(lines, rust, strict=True)
        if (ref := reference_line(raw)) != got
    ]
    return problems, len(lines), sum(1 for v in rust if v.startswith("ok "))


def compare_files(rust_bin: Path) -> tuple[list[str], int] | None:
    """The file disagreements and the corpus size; None on a tool error."""
    files = file_corpus()
    with tempfile.TemporaryDirectory(prefix="rh-differential-") as tmp:
        paths = []
        for i, data in enumerate(files):
            path = Path(tmp) / f"f{i:05d}.txt"
            path.write_bytes(data)
            paths.append(path)
        run = subprocess.run(
            [str(rust_bin), "canonical", "files", *map(str, paths)],
            capture_output=True,
            check=False,
        )
        rust = run.stdout.decode("ascii", "replace").split("\n")[:-1]
        if run.returncode != 0 or len(rust) != len(files):
            print(f"differential: rh-cli canonical files failed ({run.returncode})")
            return None
        problems = [
            f"FILE header {data.split(b'\n', 1)[0]!r}: reference {ref!r}, rh-stream {got!r}"
            for path, data, got in zip(paths, files, rust, strict=True)
            if (ref := reference_file(path)) != got
        ]
    return problems, len(files)


def main(argv: list[str] | None = None) -> int:
    """Run both parsers on the corpus and compare their verdicts."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("--rust-bin", type=Path, required=True)
    parser.add_argument("--control", choices=("relax-header",))
    args = parser.parse_args(argv)
    if args.control == "relax-header":
        setattr(CANON, "_header_values_ok", lambda _header: True)  # noqa: B010
    lines = compare_lines(args.rust_bin)
    files = compare_files(args.rust_bin)
    if lines is None or files is None:
        return EXIT_TOOL
    problems = lines[0] + files[0]
    if problems:
        for p in problems[:20]:
            print(p)
        print(
            f"DIFFERENTIAL DISAGREES: {len(problems)} of {lines[1]} lines and {files[1]} files "
            "get different verdicts from rh-stream and the reference"
        )
        return EXIT_DIFFER
    print(
        f"differential: rh-stream and the reference agree on {lines[1]} lines ({lines[2]} "
        f"accepted) and {files[1]} files (canonical_v0.md §3; REV-SPR-013 MAJ-1)"
    )
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
