"""Run directory reader for Rehoboam M9 calibration and validation (SPR-028).

Reads simulation run directories using DuckDB over sealed Parquet segments:
- LIFE_ENTRY (`life_course_entry.parquet`)
- LIFE_EXIT (`life_course_exit.parquet`)
- STEP_DIGEST (`step_digest.parquet`)
- EVENT (`event.parquet`)
- VERB (`verb.parquet`)

Parses `--vitals` lines strictly by key.
Ruling G11: strictly channel A and vitals, NO keyframe reader.

Refs:
    SPR-028 (M9 validation harness);
    specs/validation/harness_v0.md §4;
    Ruling G11.
"""

from __future__ import annotations

import contextlib
import re
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

import duckdb

_MINUTES_PER_YEAR: float = 518400.0
_TIP_PARTS_COUNT: int = 2


@dataclass(frozen=True)
class RunMetadata:
    """Metadata read from run.toml."""

    world: str
    seed: int
    years: int
    engine_build: str
    registry_version: str
    manifest_hash: str
    engine_build_commit: str | None = None
    rung: str = "yearly"
    minutes_per_year: int = 518400


@dataclass
class VitalsData:
    """Parsed metrics from simulation --vitals output."""

    raw_lines: list[str] = field(default_factory=list)
    keys: dict[str, str] = field(default_factory=dict)
    tip: str | None = None
    pace: float | None = None
    ticks_per_sec: float | None = None
    serial_pct: float | None = None
    wall_ms: int | None = None
    state_hash: str | None = None
    observation_hash: str | None = None
    narrative_hash: str | None = None
    serial_fraction: float | None = None
    instants: int | None = None
    system_job_time_ms: dict[str, int] = field(default_factory=dict)
    dropped: dict[str, int] = field(default_factory=dict)


@dataclass
class DemographicSummary:
    """Summary demographic counts and distributions from Channel A."""

    total_entries: int = 0
    births_count: int = 0
    founders_count: int = 0
    immigrants_count: int = 0
    deaths_count: int = 0
    living_start: int = 0
    living_end: int = 0
    ages_at_death: list[float] = field(default_factory=list)
    q15_cohort_births: int = 0
    q15_cohort_deaths: int = 0


@dataclass
class RunData:
    """Complete observed run data: metadata, vitals, and demographics."""

    run_dir: Path
    metadata: RunMetadata
    vitals: VitalsData
    demographics: DemographicSummary


def parse_vitals_text(text: str) -> VitalsData:
    """Parse vitals lines strictly by key, insensitive to field order (G14, S7).

    Args:
        text: Raw vitals output text or file content.

    Returns:
        Populated VitalsData instance.
    """
    vitals = VitalsData()
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    vitals.raw_lines = lines

    for line in lines:
        content = line[len("vitals:") :].strip() if line.startswith("vitals:") else line
        _parse_vitals_line_content(content, vitals)

    return vitals


def _parse_vitals_line_content(content: str, vitals: VitalsData) -> None:
    """Parse a single vitals content string into VitalsData."""
    if "system" in content and "ms of job time" in content:
        m = re.search(r"system\s+(\S+)\s+(\d+)\s+ms of job time", content)
        if m:
            vitals.system_job_time_ms[m.group(1)] = int(m.group(2))
            vitals.keys[f"system.{m.group(1)}"] = m.group(2)
        return

    if "dropped{" in content:
        m = re.search(r"dropped\{([^}]+)\}\s*=\s*(\d+)", content)
        if m:
            vitals.dropped[m.group(1)] = int(m.group(2))
            vitals.keys[f"dropped.{m.group(1)}"] = m.group(2)
        return

    if "serial fraction" in content:
        m = re.search(r"serial fraction\s+([\d.]+)%\s+over\s+(\d+)\s+instants", content)
        if m:
            vitals.serial_fraction = float(m.group(1))
            vitals.instants = int(m.group(2))
            vitals.keys["serial_fraction"] = m.group(1)
            vitals.keys["instants"] = m.group(2)
        return

    if "|" in content:
        _parse_pipe_vitals_segments(content, vitals)
    else:
        _parse_kv_vitals(content, vitals)


def _parse_kv_vitals(content: str, vitals: VitalsData) -> None:
    """Parse space-separated key=value pairs into VitalsData."""
    for token in content.split():
        if "=" not in token:
            continue
        k, v = token.split("=", 1)
        vitals.keys[k] = v
        if k == "tip":
            vitals.tip = v
        elif k == "pace":
            vitals.pace = float(v)
        elif k in ("ticks_per_sec", "ticks/s"):
            vitals.ticks_per_sec = float(v)
        elif k in ("wall_ms", "wall"):
            vitals.wall_ms = int(float(v))
        elif k in ("serial_pct", "serial"):
            vitals.serial_pct = float(v)


def _parse_pipe_vitals_segments(content: str, vitals: VitalsData) -> None:
    """Parse pipe-separated vitals summary line segments."""
    segments = [seg.strip() for seg in content.split("|")]
    for seg in segments:
        _parse_single_segment(seg, vitals)


def _parse_single_segment(seg: str, vitals: VitalsData) -> None:
    """Parse an individual segment of a pipe-separated vitals line."""
    if seg.startswith("tip"):
        parts = seg.split(maxsplit=1)
        if len(parts) == _TIP_PARTS_COUNT:
            vitals.tip = parts[1]
            vitals.keys["tip"] = parts[1]
    elif "pace" in seg:
        m = re.search(r"pace\s+([\d.]+)", seg)
        if m:
            vitals.pace = float(m.group(1))
            vitals.keys["pace"] = m.group(1)
    elif "ticks/s" in seg:
        m = re.search(r"([\d.]+)\s+ticks/s", seg)
        if m:
            vitals.ticks_per_sec = float(m.group(1))
            vitals.keys["ticks_per_sec"] = m.group(1)
    elif "serial" in seg and "of" in seg:
        m = re.search(r"serial\s+([\d.]+)%\s+of\s+(\d+)\s+ms", seg)
        if m:
            vitals.serial_pct = float(m.group(1))
            vitals.wall_ms = int(m.group(2))
            vitals.keys["serial_pct"] = m.group(1)
            vitals.keys["wall_ms"] = m.group(2)
    elif "state" in seg:
        _parse_hash_segment(seg, vitals)


def _parse_hash_segment(seg: str, vitals: VitalsData) -> None:
    """Parse state, observation, and narrative plane hashes."""
    for m in re.finditer(r"(\w+)\s+([0-9a-fA-F]{16})", seg):
        k, val = m.group(1), m.group(2)
        vitals.keys[k] = val
        if k == "state":
            vitals.state_hash = val
        elif k == "observation":
            vitals.observation_hash = val
        elif k == "narrative":
            vitals.narrative_hash = val


def read_metadata(run_dir: str | Path) -> RunMetadata:
    """Read run.toml from run directory.

    Args:
        run_dir: Root path of the simulation run.

    Returns:
        Parsed RunMetadata instance.

    Raises:
        FileNotFoundError: If run.toml does not exist.
    """
    path = Path(run_dir)
    toml_path = path / "run.toml"
    if not toml_path.exists():
        raise FileNotFoundError(f"Missing run.toml in {run_dir}")

    with toml_path.open("rb") as f:
        data = tomllib.load(f)

    return RunMetadata(
        world=str(data.get("world", "unknown")),
        seed=int(data.get("seed", 0)),
        years=int(data.get("years", 0)),
        engine_build=str(data.get("engine_build", "")),
        registry_version=str(data.get("registry_version", "")),
        manifest_hash=str(data.get("manifest_hash", "")),
        engine_build_commit=data.get("engine_build_commit"),
        rung=str(data.get("rung", "yearly")),
        minutes_per_year=int(data.get("minutes_per_year", 518400)),
    )


def _query_entry_counts(
    con: duckdb.DuckDBPyConnection, entries_glob: str
) -> tuple[int, int, int, int]:
    """Query entry counts (total, births, founders, immigrants) from life_course_entry."""
    q = f"""
        SELECT
            count(*),
            count(CASE WHEN origin = 0 THEN 1 END),
            count(CASE WHEN origin = 1 THEN 1 END),
            count(CASE WHEN origin = 2 THEN 1 END)
        FROM read_parquet('{entries_glob}')
    """
    row = con.execute(q).fetchone()
    if not row:
        return 0, 0, 0, 0
    return int(row[0]), int(row[1]), int(row[2]), int(row[3])


def _query_exit_counts(con: duckdb.DuckDBPyConnection, exits_glob: str) -> int:
    """Query death exits count from life_course_exit."""
    q = f"""
        SELECT count(CASE WHEN exit_kind = 0 THEN 1 END)
        FROM read_parquet('{exits_glob}')
    """
    row = con.execute(q).fetchone()
    return int(row[0]) if row else 0


def _query_ages_at_death(
    con: duckdb.DuckDBPyConnection, entries_glob: str, exits_glob: str, minutes_per_year: float
) -> list[float]:
    """Query ages at death in simulation years for all individuals who died."""
    q = f"""
        SELECT (x.tick - e.birth_tick) / {minutes_per_year} as age
        FROM read_parquet('{entries_glob}') e
        JOIN read_parquet('{exits_glob}') x ON e.person_id = x.person_id
        WHERE x.exit_kind = 0
        ORDER BY age
    """
    rows = con.execute(q).fetchall()
    return [float(r[0]) for r in rows]


def _query_living_counts(con: duckdb.DuckDBPyConnection, digests_glob: str) -> tuple[int, int]:
    """Query initial and final living population counts from step_digest."""
    q_first = f"SELECT living FROM read_parquet('{digests_glob}') ORDER BY tick ASC LIMIT 1"
    q_last = f"SELECT living FROM read_parquet('{digests_glob}') ORDER BY tick DESC LIMIT 1"
    r_first = con.execute(q_first).fetchone()
    r_last = con.execute(q_last).fetchone()
    first_val = int(r_first[0]) if r_first else 0
    last_val = int(r_last[0]) if r_last else 0
    return first_val, last_val


def read_demographics(run_dir: str | Path, minutes_per_year: int = 518400) -> DemographicSummary:
    """Read demographic records from Channel A sealed Parquet segments via DuckDB.

    Ruling G11: strictly channel A, NO keyframe reader.

    Args:
        run_dir: Path to the run directory.
        minutes_per_year: Simulation ticks per year.

    Returns:
        DemographicSummary populated from parquet files.
    """
    path = Path(run_dir)
    summary = DemographicSummary()
    stream_dir = path / "stream"
    if not stream_dir.exists():
        return summary

    entries_glob = str(stream_dir / "t*" / "A" / "y*" / "life_course_entry.parquet")
    exits_glob = str(stream_dir / "t*" / "A" / "y*" / "life_course_exit.parquet")
    digests_glob = str(stream_dir / "t*" / "A" / "y*" / "step_digest.parquet")

    con = duckdb.connect()
    with contextlib.suppress(Exception):
        total_e, births, founders, immigrants = _query_entry_counts(con, entries_glob)
        summary.total_entries = total_e
        summary.births_count = births
        summary.founders_count = founders
        summary.immigrants_count = immigrants

    with contextlib.suppress(Exception):
        summary.deaths_count = _query_exit_counts(con, exits_glob)

    with contextlib.suppress(Exception):
        summary.ages_at_death = _query_ages_at_death(
            con, entries_glob, exits_glob, float(minutes_per_year)
        )

    with contextlib.suppress(Exception):
        summary.living_start, summary.living_end = _query_living_counts(con, digests_glob)

    return summary


def read_run_dir(run_dir: str | Path, vitals_text: str | None = None) -> RunData:
    """Read full run data from a simulation run directory.

    Args:
        run_dir: Directory containing run.toml, stream/, etc.
        vitals_text: Optional explicit vitals text; if None, checks vitals.txt in run_dir.

    Returns:
        RunData containing metadata, parsed vitals, and demographic summaries.
    """
    path = Path(run_dir)
    metadata = read_metadata(path)

    if vitals_text is None:
        vitals_file = path / "vitals.txt"
        vitals_text = vitals_file.read_text(encoding="utf-8") if vitals_file.exists() else ""

    vitals = parse_vitals_text(vitals_text)
    demographics = read_demographics(path, metadata.minutes_per_year)

    return RunData(
        run_dir=path,
        metadata=metadata,
        vitals=vitals,
        demographics=demographics,
    )
