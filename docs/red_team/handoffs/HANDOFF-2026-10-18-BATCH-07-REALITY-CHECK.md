# RED TEAM OPERATIONAL HANDOFF // BATCH 07: PHASE 1A REALITY CHECK
## Real Plane A Stream Ingestion, Potemkin Village Demolition & Governance Audit

> **Document ID:** `HANDOFF-2026-10-18-BATCH-07`  
> **From:** Red Team Leader (Google Cloud VM `adjutant` Session)  
> **To:** The Director & The Architect  
> **Date:** 2026-10-18  
> **Status:** Active Execution Dossier · Batch 07 Findings  
> **Branch Tracking:** `arch/red-team-batch-04` (Fast-Forward Merged with `origin/main` at `fcd34b8`)  

---

## 1. Executive BLUF: The "Potemkin Village" Reality Check

The Red Team has completed a deep-dive probe into the newly entered Phase 1a runtime. While the automated test matrix on `origin/main` reports green across all 681 tests, our adversarial inspection revealed a critical architectural disconnect:

**The G0/Phase 1a person simulation (Ada in `worlds/solo`) was NOT running in the viewer.**

1. **The Discovery:** [`bin/stream_server.py`](../../bin/stream_server.py) was found to be an ungrounded, synthetic mock script. It generated fake "Val d'Or" village data and simulated trebuchet combat using Python's standard `random.random()` and `random.randint()`, with zero connections to `worlds/solo`, `rh-stream`, `journal.sqlite`, or Parquet files.
2. **The Fix (Delivered Tonight):** The Red Team has upgraded [`bin/stream_server.py`](../../bin/stream_server.py) with `--run-dir <DIR>` support. It now spawns `rh-cli tail <run-dir> --canonical --wait` and broadcasts real Plane A `STEP_DIGEST`, `EVENT` (`DEATH`, `EXTINCTION`), `LIFE_EXIT`, and Plane C `SIFT_MATCH` packets over port 8089.
3. **Verified Live:** Port 8089 is currently broadcasting the real, bitwise-deterministic life and natural death of Ada (Month 693, tick 29,975,040) directly from `/tmp/solo_run/stream`.

---

## 2. Batch 07 Findings & Action Items

| ID | Title | Severity | Architect Action Required |
|---|---|---|---|
| **RT-035** | **The Potemkin Village Gap & Live Stream Bridge** | **CRITICAL (RESOLVED)** | Officially adopt the `--run-dir` stream server bridge into `tools/` and standardize the JSON envelope for downstream 3D/Bevy viewers. |
| **RT-036** | **Missing IF-0 Control Channel Bridge (`control.sock`)** | **HIGH** | PR #80 standardized `specs/stream/control_v1.md`. When SPR-027 builds `control.sock`, connect `stream_server.py` to forward viewer UI commands (Pause, Resume, Pace) to the Unix socket. |
| **RT-037** | **The Manor Demographic Scale Vacuum (60 to 1,500)** | **MEDIUM** | D57 item 3 omitted `manor` as a committed golden ladder to save CI time. Mandate an uncommitted demographic sanity probe to ensure kinship trees and mortality curves do not explode between $N=60$ (`hamlet`) and $N=1,500$ (`twin_vales`). |
| **RT-038** | **Pytest Subshell Cargo PATH Trap** | **LOW (TOOLING)** | `python/tests/test_lint_forbidden.py` fails with `cargo is not on PATH` if the subshell does not source `~/.cargo/bin`. Harden Python linter discovery with `shutil.which("cargo")`. |

---

## 3. Deep-Dive Findings

### Finding RT-035: Demolishing the Python Mock
Before tonight, running `bin/run_stream_viewer.sh` showed an impressive animated diorama, but it was purely choreographic eye candy. The real engine was simulating Ada:
* Born tick: `-13,236,200` (Age 25.5 at genesis)
* Scoped material input: tick `27,786,607` (Year 53, Day 216)
* Natural death: tick `29,975,040` (Year 57, Month 693)
* Extinction event: tick `29,975,040` (Empty world onward)

With our `--run-dir` upgrade to [`bin/stream_server.py`](../../bin/stream_server.py), the stream server broadcasts:
```json
{
  "type": "SIMULATION_TICK",
  "mode": "REAL_PLANE_A_STREAM",
  "tick": 29975040,
  "calendar": {"year": 57, "month": "Oct", "season": "Autumn"},
  "village": {"name": "solo_run", "population": 0, "births": 0, "deaths": 1, "state_hash": "3f2ace1e4ec6eca0"}
}
```
followed by:
```json
{
  "type": "PLANE_A_EVENT",
  "verb": "DEATH",
  "tick": 29975040,
  "message": "[Plane A Event] DEATH at tick 29975040"
}
```
and Plane C causal story sifter matches:
```json
{
  "type": "SIFT_MATCH",
  "pattern": "solitary_life/life_ends A=Ada (match 7f9a50b9a37686da)",
  "message": "[Plane C Story Sifter] Match: solitary_life/life_ends A=Ada (match 7f9a50b9a37686da)"
}
```

### Finding RT-037: The Manor Scale Vacuum
Under D57 (Item 3), the Architect recommended against committing `manor` to CI to avoid adding 1.5 minutes to golden runs. The Red Team supported this for CI budget reasons, but we warn the Architect:
* Jumping from `hamlet` (60 people, 1 settlement) directly to `twin_vales` (1,500 people, 2 valleys, 5 great houses, tanistry and primogeniture) is a **25x complexity leap**.
* We demand that SPR-019 include a headless stress test script `tests/probes/probe_manor_scale.py` that runs without gating CI but produces an artifact report on memory usage and cohort stability.

---

## 4. Director Rulings Ready for Handoff

The Director can formally unblock Wave 6 and ratify all pending decisions with this single statement:

```text
Rulings on pending decisions:
- D52: Approved as one sprint with checkpoint after T-05.
- D53: Approved (no commercial ripped assets on main; procedural voxels only).
- D54: Approved (wave-5 plans approved; 3rd CI job approved).
- D55: Approved (V.1 Bevy viewer and V.1b Culture Dreamer added to roadmap).
- D56: Approved (Agnatic membership, later-children vocations, couple elite fertility).
- D57: Approved (PR #85 approved: SPR-018 single with checkpoint, SPR-019 split, kind-7 deferred to P1b, no manor golden, 3rd CI job, SPR-019b waits for D56).
```
