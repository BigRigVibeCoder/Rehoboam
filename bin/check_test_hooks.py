"""No test hook in a baseline or golden command line (SPR-026; the Architect's poison ruling).

`rh-cli run` has test hooks (`--test-poison`, `--test-stall`, `--test-pace-ms`: every flag of
`crates/rh-golden/src/cli.rs` that starts with `--test-`, read from that file so a new hook is
covered when it is added). They exist for the live-edge and run-directory tests; a baseline or a
golden ladder made with one is not a baseline. This gate refuses one:

- anywhere in `bin/regen_baseline.sh` and `bin/golden_nightly.sh`;
- in any `bin/check_all.sh` command line (backslash continuations joined) that runs `rh-cli run`
  and names a golden or baseline path or gate (`$GOLDEN`, `target/golden`, `baselines/`,
  `check_baseline`, `Golden run`).

`--plant` scans a copy of `bin/regen_baseline.sh` whose solo run line carries `--test-poison 0`:
it must FAIL (the planted control in `bin/check_all.sh`).

Usage: python -I bin/check_test_hooks.py [--plant]
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
CLI = REPO / "crates" / "rh-golden" / "src" / "cli.rs"
WHOLE_FILES = ("bin/regen_baseline.sh", "bin/golden_nightly.sh")
CHAIN = "bin/check_all.sh"
HOOK = re.compile(r'"(--test-[a-z][a-z-]*)"')
RUN = re.compile(r"rh-cli\S*\s+run\s|RH_CLI[A-Z_]*\"?\s+run\s")
GOLDEN = re.compile(r"\$GOLDEN|\$\{GOLDEN|target/golden|baselines/|check_baseline|Golden run")


def hooks() -> list[str]:
    """The `--test-*` flags rh-cli run parses, sorted."""
    return sorted(set(HOOK.findall(CLI.read_text(encoding="utf-8"))))


def logical_lines(text: str) -> list[tuple[int, str]]:
    """The lines of a shell script with backslash continuations joined, by first line number."""
    out: list[tuple[int, str]] = []
    start, buf = 0, ""
    for i, line in enumerate(text.splitlines(), 1):
        if not buf:
            start = i
        if line.endswith("\\"):
            buf += line[:-1] + " "
            continue
        out.append((start, buf + line))
        buf = ""
    if buf:
        out.append((start, buf))
    return out


def found(hook_names: list[str], line: str) -> list[str]:
    """The hooks a command line passes (a whole flag, not a prefix of a longer one)."""
    return [h for h in hook_names if re.search(re.escape(h) + r"(?![a-z-])", line)]


def scan(texts: dict[str, str], hook_names: list[str]) -> tuple[int, list[str]]:
    """The `rh-cli run` lines checked, and every hook found as `path:line: hook`."""
    checked, bad = 0, []
    for path, text in texts.items():
        for no, line in logical_lines(text):
            if line.lstrip().startswith("#"):
                continue
            if path == CHAIN and not (RUN.search(line) and GOLDEN.search(line)):
                continue
            # The two whole files: every line is scanned, the `rh-cli run` lines counted.
            checked += 1 if RUN.search(line) else 0
            bad += [f"{path}:{no}: {h}" for h in found(hook_names, line)]
    return checked, bad


def main(argv: list[str]) -> int:
    """The gate; 0 pass, 1 a hook found (or nothing checked), 2 usage."""
    if argv not in ([], ["--plant"]):
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        return 2
    names = hooks()
    if not names:
        print(f"TEST HOOKS NOT FOUND: no --test-* flag in {CLI.relative_to(REPO)}")
        return 1
    texts = {p: (REPO / p).read_text(encoding="utf-8") for p in (*WHOLE_FILES, CHAIN)}
    if argv == ["--plant"]:
        regen = texts["bin/regen_baseline.sh"]
        planted = regen.replace(
            "run solo --years 200 --threads 1 --ladder yearly",
            "run solo --years 200 --threads 1 --ladder yearly --test-poison 0",
            1,
        )
        if planted == regen:
            print("PLANT FAILED: the solo run line of bin/regen_baseline.sh was not found")
            return 2
        texts["bin/regen_baseline.sh"] = planted
    checked, bad = scan(texts, names)
    if bad:
        for b in bad:
            print(f"TEST HOOK IN A BASELINE OR GOLDEN COMMAND: {b}")
        return 1
    if checked == 0:
        print("NO BASELINE OR GOLDEN COMMAND LINE FOUND: the gate checked nothing")
        return 1
    print(
        f"test hooks: none of {', '.join(names)} in {checked} baseline and golden "
        f"`rh-cli run` command lines ({', '.join(WHOLE_FILES)} whole; {CHAIN}'s golden lines)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
