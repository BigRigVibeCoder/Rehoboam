"""`python -I -m rehoboam_tools`: print the distribution's version.

The entry point every later package uses (`python -I -m rehoboam_<name> ...`), exercised from the
repository root by python/tests/test_packaging.py.

Refs: PY-PKG (REV-WAVE8 S6); python/README.md.
"""

import argparse
import sys
from collections.abc import Sequence

from rehoboam_tools import __version__


def main(argv: Sequence[str] | None = None) -> int:
    """Print ``rehoboam_tools <version>``; an argument other than ``--help`` exits with status 2."""
    parser = argparse.ArgumentParser(
        prog="python -m rehoboam_tools",
        description="Print the version of the rehoboam-tools distribution.",
    )
    parser.parse_args(argv)
    print(f"rehoboam_tools {__version__}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
