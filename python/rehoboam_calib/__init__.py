"""Rehoboam Calibration and Statistical Validation Package (SPR-028).

Production implementation of M9 validation harness components:
- `rehoboam_calib.stats`: exact statistical primitives and estimators.
- `rehoboam_calib.read`: DuckDB sealed stream segment reader and vitals parser.
- `rehoboam_calib.bands`: machine-readable stylized fact band evaluator.
- `rehoboam_calib.report`: canonical report formatter and markdown generator.

Refs:
    SPR-028 (M9 validation harness);
    docs/09_verification_and_validation.md;
    specs/validation/harness_v0.md.
"""

from typing import Final

__version__: Final = "0.0.0"
