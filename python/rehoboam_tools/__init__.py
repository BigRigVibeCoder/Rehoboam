"""The rehoboam-tools distribution's own package: its version, and nothing else.

It exists so that the uv project in python/ is a package with at least one module, and so that
`python -I -m rehoboam_tools` proves the layout every later package uses (PY-PKG). Code goes in a
package named for its purpose (rehoboam_calib, rehoboam_narrative, rehoboam_tui; 05.0 §6), never
here.

Refs: PY-PKG (REV-WAVE8 S6); python/README.md; docs/05_architecture/05.0_overview.md §6.
"""

from typing import Final

# REF: python/pyproject.toml [project] version; python/tests/test_packaging.py pins the two equal.
__version__: Final = "0.0.0"
