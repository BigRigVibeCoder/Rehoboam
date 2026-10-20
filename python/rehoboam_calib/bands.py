"""Machine-readable band evaluator for Rehoboam M9 calibration (SPR-028).

Reads stylized fact specifications from harness_v0.toml / bands_v0.toml, evaluates
simulation runs against band definitions, and enforces the sealed rule (D61 item 4).

Refs:
    SPR-028 (M9 validation harness);
    docs/09_verification_and_validation.md §3.1;
    specs/validation/harness_v0.md.
"""

from __future__ import annotations

import math
import tomllib
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from rehoboam_calib.read import RunData
from rehoboam_calib.stats import clopper_pearson_ci, t_ci

_MARGINAL_REL_TOL: float = 1e-9
_MIN_SEEDS: int = 2
_MIN_LIVING_RATIO_FLOOR: float = 1e-12
_AGE_CHILD_LIMIT: float = 15.0
_AGE_ADULT_LIMIT: float = 65.0

_BUILTIN_BANDS: dict[str, dict[str, Any]] = {
    "V-SCALE-02": {
        "id": "V-SCALE-02",
        "fact": "No false statistics at small N",
        "kind": "Identity",
        "role": "Calibration",
        "level": "System",
        "status": "gate_g1a",
        "statistic": "Harness on small worlds; every band reports INSUFFICIENT or valid",
        "unit": "persons",
        "min_samples": 200,
        "test_method": "scale_check",
    }
}


@dataclass(frozen=True)
class BandDefinition:
    """Declared specification of a stylized fact band."""

    id: str
    fact: str
    kind: str
    role: str
    level: str
    status: str
    statistic: str
    unit: str
    min_samples: int
    lower_bound: float | None = None
    upper_bound: float | None = None
    target: float | None = None
    tolerance: float | None = None
    test_method: str = "student_t"
    alpha: float = 0.001


@dataclass
class BandVerdict:
    """The verdict and confidence interval for a single evaluated band."""

    band_id: str
    status: str
    verdict: str  # PASS, FAIL, MARGINAL, REPORT, INSUFFICIENT, SEALED
    stat: float | None
    ci_lo: float | None
    ci_hi: float | None
    bounds: tuple[float | None, float | None]
    unit: str
    n_samples: int
    note: str = ""


def load_bands(toml_path: str | Path | None = None) -> dict[str, dict[str, Any]]:
    """Load machine-readable band definitions from TOML catalog.

    Args:
        toml_path: Path to harness_v0.toml or bands_v0.toml. If None, resolves from repo.

    Returns:
        Dictionary mapping band_id to raw band dictionary.
    """
    path = Path(toml_path) if toml_path is not None else None
    if path is None:
        root = Path(__file__).resolve().parents[2]
        path = root / "specs" / "validation" / "harness_v0.toml"
        if not path.exists():
            path = root / "specs" / "validation" / "bands_v0.toml"

    with path.open("rb") as f:
        data = tomllib.load(f)

    bands: dict[str, dict[str, Any]] = dict(data.get("bands", {}))
    for bid, bdef in _BUILTIN_BANDS.items():
        if bid not in bands:
            bands[bid] = dict(bdef)
    return bands


def _resolve_bounds(
    band_def: dict[str, Any], sample_stat: float | None
) -> tuple[float | None, float | None]:
    """Resolve lower and upper bounds from bounds or target/tolerance."""
    lo_bound = band_def.get("lower_bound")
    hi_bound = band_def.get("upper_bound")
    tol = band_def.get("tolerance")
    if tol is not None and lo_bound is None and hi_bound is None and sample_stat is not None:
        target = band_def.get("target", 0.0)
        lo_bound = target - tol
        hi_bound = target + tol
    return lo_bound, hi_bound


def _is_marginal_bound(bound: float | None, ci_val: float) -> bool:
    """Check if confidence interval bound is within 10^-9 relative of band edge."""
    if bound is None:
        return False
    return abs(ci_val - bound) / max(abs(bound), 1.0) <= _MARGINAL_REL_TOL


def _check_gated_verdict(
    lo_bound: float | None, hi_bound: float | None, ci_lo: float, ci_hi: float
) -> str:
    """Compute PASS, FAIL, or MARGINAL verdict for gated band."""
    passes_lo = (lo_bound is None) or (ci_lo >= lo_bound)
    passes_hi = (hi_bound is None) or (ci_hi <= hi_bound)
    wholly_outside = ((lo_bound is not None) and (ci_hi < lo_bound)) or (
        (hi_bound is not None) and (ci_lo > hi_bound)
    )

    if passes_lo and passes_hi:
        if _is_marginal_bound(lo_bound, ci_lo) or _is_marginal_bound(hi_bound, ci_hi):
            return "MARGINAL"
        return "PASS"
    if wholly_outside:
        return "FAIL"
    return "MARGINAL"


def evaluate_band(
    band_def: dict[str, Any],
    sample_stat: float | None,
    ci: tuple[float | None, float | None] = (None, None),
    n_samples: int = 0,
    note: str = "",
) -> BandVerdict:
    """Evaluate a single band definition against sample statistics.

    Args:
        band_def: Dictionary defining the band properties.
        sample_stat: Observed point estimate.
        ci: Tuple of (ci_lower, ci_upper).
        n_samples: Sample size.
        note: Additional diagnostic or unidentifiability notes.

    Returns:
        Evaluated BandVerdict instance.
    """
    ci_lo, ci_hi = ci
    band_id = str(band_def.get("id", "unknown"))
    status = str(band_def.get("status", "report"))
    unit = str(band_def.get("unit", ""))
    min_samples = int(band_def.get("min_samples", 1))

    lo_bound, hi_bound = _resolve_bounds(band_def, sample_stat)

    # 1. Sealed rule (D61 item 4, GOV-002 law 11): ZERO numerical leak
    if status == "sealed":
        return BandVerdict(
            band_id=band_id,
            status=status,
            verdict="SEALED",
            stat=None,
            ci_lo=None,
            ci_hi=None,
            bounds=(lo_bound, hi_bound),
            unit=unit,
            n_samples=n_samples,
            note="A-04; held-out, docs/09 §3.1",
        )

    # 2. Minimum sample requirement: refuse to pass or fail below min_samples
    if n_samples < min_samples:
        return BandVerdict(
            band_id=band_id,
            status=status,
            verdict="INSUFFICIENT",
            stat=sample_stat,
            ci_lo=ci_lo,
            ci_hi=ci_hi,
            bounds=(lo_bound, hi_bound),
            unit=unit,
            n_samples=n_samples,
            note=f"samples {n_samples} < {min_samples}",
        )

    # 3. Missing statistics
    if sample_stat is None or ci_lo is None or ci_hi is None:
        return BandVerdict(
            band_id=band_id,
            status=status,
            verdict="INSUFFICIENT",
            stat=sample_stat,
            ci_lo=ci_lo,
            ci_hi=ci_hi,
            bounds=(lo_bound, hi_bound),
            unit=unit,
            n_samples=n_samples,
            note=note,
        )

    # 4. Report status vs gated status
    if status == "report":
        verdict = "REPORT"
    else:
        verdict = _check_gated_verdict(lo_bound, hi_bound, ci_lo, ci_hi)

    return BandVerdict(
        band_id=band_id,
        status=status,
        verdict=verdict,
        stat=sample_stat,
        ci_lo=ci_lo,
        ci_hi=ci_hi,
        bounds=(lo_bound, hi_bound),
        unit=unit,
        n_samples=n_samples,
        note=note,
    )


def _eval_vsf01_r(runs: list[RunData], band_def: dict[str, Any]) -> BandVerdict:
    """Evaluate V-SF-01.r growth rate band across runs."""
    n_seeds = len(runs)
    if n_seeds < int(band_def.get("min_samples", 32)):
        return evaluate_band(band_def, None, (None, None), n_seeds, note="insufficient seeds")

    r_rates: list[float] = []
    for r in runs:
        if r.demographics.living_start > 0 and r.metadata.years > 0:
            ratio = max(
                r.demographics.living_end / r.demographics.living_start,
                _MIN_LIVING_RATIO_FLOOR,
            )
            r_rates.append(math.log(ratio) / r.metadata.years)

    if len(r_rates) < _MIN_SEEDS:
        return evaluate_band(band_def, None, (None, None), n_seeds, note="insufficient seeds")

    mean_r, _, _, _, ci_lo, ci_hi = t_ci(r_rates, alpha=0.001)
    return evaluate_band(band_def, mean_r, (ci_lo, ci_hi), n_seeds)


def _eval_vsf01_pop(runs: list[RunData], band_def: dict[str, Any]) -> BandVerdict:
    """Evaluate V-SF-01.pop population range band across runs."""
    n_seeds = len(runs)
    if n_seeds < int(band_def.get("min_samples", 32)):
        return evaluate_band(band_def, None, (None, None), n_seeds, note="insufficient seeds")

    pop_ratios: list[float] = []
    for r in runs:
        if r.demographics.living_start > 0:
            pop_ratios.append(r.demographics.living_end / r.demographics.living_start)

    if not pop_ratios:
        return evaluate_band(band_def, None, (None, None), n_seeds, note="insufficient seeds")

    mean_ratio = sum(pop_ratios) / len(pop_ratios)
    min_ratio = min(pop_ratios)
    max_ratio = max(pop_ratios)
    return evaluate_band(band_def, mean_ratio, (min_ratio, max_ratio), n_seeds)


def _eval_vsf02(runs: list[RunData], band_def: dict[str, Any]) -> BandVerdict:
    """Evaluate V-SF-02 child mortality q(15) band across runs."""
    min_samples = int(band_def.get("min_samples", 400))
    total_persons = sum(len(r.demographics.ages_at_death) + r.demographics.living_end for r in runs)
    if total_persons < min_samples:
        return evaluate_band(
            band_def,
            None,
            (None, None),
            total_persons,
            note=f"samples {total_persons} < {min_samples}",
        )

    if len(runs) > 1:
        rates: list[float] = []
        for r in runs:
            n_r = len(r.demographics.ages_at_death) + r.demographics.living_end
            if n_r > 0:
                d_r = sum(1 for a in r.demographics.ages_at_death if a < _AGE_CHILD_LIMIT)
                rates.append(d_r / n_r)
        if len(rates) >= _MIN_SEEDS:
            mean_q, _, _, _, ci_lo, ci_hi = t_ci(rates, alpha=0.001)
            return evaluate_band(band_def, mean_q, (ci_lo, ci_hi), total_persons)

    d_all = sum(1 for r in runs for a in r.demographics.ages_at_death if a < _AGE_CHILD_LIMIT)
    p = d_all / total_persons
    ci_lo, ci_hi = clopper_pearson_ci(d_all, total_persons, alpha=0.001)
    return evaluate_band(band_def, p, (ci_lo, ci_hi), total_persons)


def _eval_vsf02b(runs: list[RunData], band_def: dict[str, Any]) -> BandVerdict:
    """Evaluate V-SF-02b adult survival ratio l(65)/l(15) across runs."""
    min_samples = int(band_def.get("min_samples", 400))
    total_persons = sum(len(r.demographics.ages_at_death) + r.demographics.living_end for r in runs)
    if total_persons < min_samples:
        return evaluate_band(
            band_def,
            None,
            (None, None),
            total_persons,
            note=f"samples {total_persons} < {min_samples}",
        )

    if len(runs) > 1:
        ratios: list[float] = []
        for r in runs:
            n_15 = sum(1 for a in r.demographics.ages_at_death if a >= _AGE_CHILD_LIMIT)
            n_65 = sum(1 for a in r.demographics.ages_at_death if a >= _AGE_ADULT_LIMIT)
            if n_15 > 0:
                ratios.append(n_65 / n_15)
        if len(ratios) >= _MIN_SEEDS:
            mean_r, _, _, _, ci_lo, ci_hi = t_ci(ratios, alpha=0.001)
            return evaluate_band(band_def, mean_r, (ci_lo, ci_hi), total_persons)

    n_15_all = sum(1 for r in runs for a in r.demographics.ages_at_death if a >= _AGE_CHILD_LIMIT)
    n_65_all = sum(1 for r in runs for a in r.demographics.ages_at_death if a >= _AGE_ADULT_LIMIT)
    if n_15_all == 0:
        return evaluate_band(
            band_def, None, (None, None), total_persons, note="no persons reached age 15"
        )
    ratio = n_65_all / n_15_all
    ci_lo, ci_hi = clopper_pearson_ci(n_65_all, n_15_all, alpha=0.001)
    return evaluate_band(band_def, ratio, (ci_lo, ci_hi), total_persons)


def _eval_vsf03(runs: list[RunData], band_def: dict[str, Any]) -> BandVerdict:
    """Evaluate V-SF-03 famine mortality crises across runs."""
    min_samples = int(band_def.get("min_samples", 10))
    total_centuries = sum(r.metadata.years / 100.0 for r in runs)
    n_centuries = int(total_centuries)
    if n_centuries < min_samples:
        return evaluate_band(
            band_def,
            None,
            (None, None),
            n_centuries,
            note=f"samples {n_centuries} < {min_samples}",
        )
    return evaluate_band(
        band_def, None, (None, None), n_centuries, note="no crises data in channel A"
    )


def _eval_vscale02(runs: list[RunData], band_def: dict[str, Any]) -> BandVerdict:
    """Evaluate V-SCALE-02 small-N scale check across runs."""
    min_samples = int(band_def.get("min_samples", 200))
    total_pop = max((r.demographics.total_entries for r in runs), default=0)
    if total_pop < min_samples:
        return evaluate_band(
            band_def,
            None,
            (None, None),
            total_pop,
            note=f"samples {total_pop} < {min_samples}",
        )
    return evaluate_band(band_def, 1.0, (1.0, 1.0), total_pop)


def expand_band_filter(target_bands: Sequence[str], catalog: dict[str, Any]) -> list[str]:
    """Expand requested band IDs against catalog with exact and prefix matching."""
    matched: list[str] = []
    for requested in target_bands:
        req = requested.strip()
        if not req:
            continue
        if req in catalog:
            if req not in matched:
                matched.append(req)
            continue
        prefix_matches = [
            bid
            for bid in catalog
            if bid.startswith(f"{req}.")
            or (len(bid) > len(req) and bid.startswith(req) and bid[len(req)].isalpha())
        ]
        if prefix_matches:
            for m in prefix_matches:
                if m not in matched:
                    matched.append(m)
        elif req not in matched:
            matched.append(req)
    return matched


def _dispatch_band_eval(bid: str, bdef: dict[str, Any], runs: list[RunData]) -> BandVerdict:
    """Dispatch evaluation of single band across runs."""
    dispatch_map = {
        "V-SF-01.r": _eval_vsf01_r,
        "V-SF-01.pop": _eval_vsf01_pop,
        "V-SF-02": _eval_vsf02,
        "V-SF-02b": _eval_vsf02b,
        "V-SF-03": _eval_vsf03,
        "V-SCALE-02": _eval_vscale02,
    }
    handler = dispatch_map.get(bid)
    if handler is not None:
        return handler(runs, bdef)
    n_samples = len(runs)
    return evaluate_band(bdef, None, (None, None), n_samples)


def evaluate_runs(
    runs: list[RunData],
    bands_catalog: dict[str, dict[str, Any]] | None = None,
    filter_band_ids: Sequence[str] | None = None,
) -> list[BandVerdict]:
    """Evaluate all declared bands against observed simulation runs.

    Args:
        runs: List of observed RunData instances.
        bands_catalog: Optional bands dictionary; loads catalog if None.
        filter_band_ids: Optional list of band IDs to evaluate; evaluates all if None.

    Returns:
        List of BandVerdict instances.
    """
    if bands_catalog is None:
        bands_catalog = load_bands()

    if filter_band_ids is not None:
        target_ids = expand_band_filter(filter_band_ids, bands_catalog)
    else:
        target_ids = list(bands_catalog.keys())

    verdicts: list[BandVerdict] = []
    for bid in target_ids:
        if bid not in bands_catalog:
            continue
        bdef = bands_catalog[bid]
        verdicts.append(_dispatch_band_eval(bid, bdef, runs))

    return verdicts
