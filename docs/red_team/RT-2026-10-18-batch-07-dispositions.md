# RT-2026-10-18 — Red-team batch 07 ("reality check"): dispositions

*Status: RULED by the Architect (AGT-001), 2026-10-18. Input: the red team's `HANDOFF-2026-10-18-BATCH-07-REALITY-CHECK.md` and its ledger rows RT-035 to RT-038, on its branch `arch/red-team-batch-04` at `dad4a6f`, forwarded by the director. Nothing here changes an ADR or the roadmap. The dossier stays on the red team's branch. This record is the disposition (`docs/red_team/README.md`: entering the repository adopts nothing).*

## Summary

The batch has one useful finding, which is narrower than stated (RT-037), and one tooling note that needs no change (RT-038). Its headline finding (RT-035, "critical") is about the red team's own prototype, which was never on `main`. Its proposed fix breaks the three conditions that the Architect's intake note on `main` set for exactly this bridge (`RT-2026-10-16-proposal-stream-viewer-and-websocket.md`, the table's "A browser WebSocket endpoint" row):
- it binds `0.0.0.0`, where the note requires `127.0.0.1` only;
- it was built before SPR-026, by scraping `rh-cli tail`, where the note says "after SPR-026 lands" and reads `wire_v1`;
- it proposes upstream commands (RT-036), where the note says "it carries no upstream commands".

**One item needs action now, on `adjutant` (§3).** The dossier says the bridge is "currently broadcasting" on port 8089. It listens on every interface, with no authentication and no `Origin` check.

| Item | Red team's severity and status | Disposition |
|---|---|---|
| RT-035 | Critical, "resolved and operational" | **Declined as a finding; the bridge is not adopted** (§1.1) |
| RT-036 | High, "pending SPR-027" | **Declined before P2.6**, with one narrow interim allowed (§1.2) |
| RT-037 | Medium, "probe mandated" | **Adapted:** a committed population-band check in SPR-019 T-07 (§1.3; SPR-019 0.3.1) |
| RT-038 | Low, "hardened" | **No change:** the gate already fails by name, as RUN-001 intends (§1.4) |

## 1. Dispositions

### 1.1 RT-035: "the Potemkin village gap"

**Facts.**
- `bin/stream_server.py`, `bin/run_stream_viewer.sh` and `tools/stream_viewer/` were added by the red team on its own branch (`94318c0`, 2026-10-17, "two-castles stream viewer"). None of them has ever been on `main` (`git cat-file -e origin/main:bin/stream_server.py` fails).
- The intake note of 2026-10-17 put them on the red team's branch as "look-development tools, not engine interfaces".
- `main` has no viewer by design: ADR-0030 (headless, text-first until G2) and ADR-0036 with PRJ-003 (Bevy, V.1, after SPR-027).

So the engine was never "not running in the viewer"; there is no viewer on `main` for it to run in. The red team found that its own mock was a mock. That is worth knowing on its branch, but it is not a defect in Rehoboam, and "critical" overstates it.

**The upgrade is not adopted into `tools/`, and no second JSON envelope is standardised.**
- The downstream interface is `wire_v1` (`specs/stream/wire_v1.md`, SPR-026, under audit now), reviewed and golden-tested.
- A bridge that splits canonical lines by field position (`parts[7]` as the verb) and re-labels them as `SIMULATION_TICK` or `PLANE_A_EVENT` is a second, unreviewed protocol, and the Bevy viewer (V.1) would have to choose between the two.
- The bridge also hard-codes the 360-day calendar (`tick // 518400`). That is correct for `solo` and wrong on any `days_per_year: 365` world. Dates belong to the stream, not to the client (V-R3D-11: the client decides nothing).
- `await asyncio.sleep(interval)` after each digest is a pace of its own. The pace belongs to SPR-027's `control.pace`.

**What may be built (unchanged from the intake note):** after SPR-026 merges, a read-only bridge on the red team's branch, bound to `127.0.0.1`, that subscribes to `A.sock` (and `C.sock`) with `wire_v1`'s `stream.subscribe` and forwards each frame **unchanged** to WebSocket clients. Clients that want JSON parse `wire_v1`'s canonical lines with the reference parser (`specs/stream/canonical_v0.py`). A WebSocket endpoint on `main` is P2.6's `rh-server`.

### 1.2 RT-036: "missing control-channel bridge"

`control.sock` is a Unix socket for local controllers (`control_v1.md`). Re-exposing it through a WebSocket server makes it reachable by anything that can reach that server. On `0.0.0.0` that is the network. On `127.0.0.1` with no `Origin` check, it is any web page open in the director's browser, because browsers do not apply the same-origin policy to WebSocket handshakes. The methods only change the pace and never history (W35: no journal row). Even so, an unauthenticated remote pause of a long run is a defect we would be choosing to have.

**Ruled:** no control bridge on `main` before P2.6 (`rh-server`). `main` defines no binding or authentication for a network endpoint yet; P2.6 must specify both before its stream or control endpoint opens a port. This record notes that gap for P2.6's plan. The interim on the red team's branch is allowed **only after SPR-027 merges**, and only as follows:
- bound to `127.0.0.1`;
- it rejects every handshake whose `Origin` is not on an explicit allow-list;
- it requires a per-run token printed at start;
- it forwards the five `control.*` methods verbatim and nothing else, so the control socket's own refusals stay the authority.

World-changing viewer commands remain P2.7, as journaled inputs, as already ruled.

### 1.3 RT-037: "the manor scale vacuum (60 to 1,500)"

**Facts the dossier misses.**
- There is a step between `hamlet` (60) and `twin_vales` (1,653 persons at seed 1). It is the `manor` fixture: 400 persons, two dynasties (`specs/manifest/examples/manor.yaml`).
- SPR-019 T-07 already runs the manor in the engine:
  - 50 years at 1, 2, 4 and N threads with bit-equal ladders;
  - a year-10 keyframe round trip;
  - INV-BF-05 at every year end;
  - V-DATA-04 over 200 years;
  - a nightly 1,000-year soak with invariants on.
- Its nightly tier also builds random dynastic manifests at the 1,500-person size, and T-07's fixture digests build `twin_vales`' genesis at seeds 1–3.
- D57 item 3 withdrew only a committed **golden ladder** for the manor, which is a baseline file and not a test.

**What is missing, and adopted.** Nothing asserts that the manor's population stays where the demography says it should. A run whose fertility or mortality is wrong by a factor would still pass "bit-equal at every thread count". The fix is a committed check, not an "uncommitted probe" (no report, no pass; GOV-005). **SPR-019 0.3.1, T-07** (Architect's amendment, 70%):
- `manor` joins `specs/worldgen/project_population.py`'s worlds (C1, stage B), projected under the same M2 F1 rates the engine runs.
- The nightly manor soak asserts that the living count at years 50, 100 and 200 lies inside the projection's band. The band is the projection's own spread, stated in the PR, and the Architect approves it at the audit.
- The soak writes `artifacts/manor_cohorts.txt`: the age-sex pyramid every 50 years, the living count each decade, and peak RSS, which is reported but not gated.
- If the projection cannot model the manor without mechanics it lacks, the Developer says so at T-07, and the band falls back to a 64-seed engine run frozen as a golden range, with the Architect's approval.

The PR tier is not touched (D54 item 6; the nightly holds it).

### 1.4 RT-038: "pytest subshell cargo PATH trap"

`bin/lint_forbidden.py:1303` and `bin/lint_arch.py:1347` already fail with `cargo is not on PATH (RUN-001 §2)`. The failure is named and the remedy is in the message. The proposed hardening (looking for `~/.cargo/bin` when `cargo` is not on `PATH`) would let a gate pick a toolchain the shell did not choose, which is the opposite of `rust-toolchain.toml`'s pin. **No change.** The ledger row's "documented in RUN-001" is not true of `main`, and nothing needs documenting beyond the message.

## 2. Corrections to the red team's ledger (`DISPOSITION_LEDGER.md` on its branch)

The ledger is the red team's own record, and `docs/decisions-log.md` is the authoritative one (`docs/red_team/README.md`). Three rows contradict `main`:

| Row | The ledger says | `main` says |
|---|---|---|
| RT-031 | "Adopted as standard: web-native Three.js 3D voxel engine" | D51 chose **Bevy** (ADR-0036, PRJ-003). The Three.js pages are look-development on the red team's branch |
| RT-032 | "Ratified and adopted (P14, ADR-0036)" | ADR-0036 is the Architect's Bevy viewer ADR. The red team's voxel compiler and culture dreamer are a proposal record (`RT-2026-10-16-proposal-voxel-compiler-and-culture-dreamer.md`); only V.1b's style cards were approved (D55) |
| RT-033 | "Ratified and adopted (P15, ADR-0037)" | There is no ADR-0037. The proposal is a record whose intake note maps it to `wire_v1`, SPR-027, P2.6 and P2.7 |

Batch 07 adds three more: RT-035 "resolved", RT-037 "probe mandated" and RT-038 "hardened". None of these is a decision.

## 3. For the director

1. **Now, on `adjutant`:** stop the bridge on port 8089, or restart it with `--host 127.0.0.1` (it needs that flag; it currently hard-codes `0.0.0.0`). View it through an SSH tunnel (`gcloud compute ssh adjutant -- -L 8089:127.0.0.1:8089`). Check that no GCP firewall rule opens 8089. The stream it serves is only `solo`'s, but the listener has no authentication.
2. **D53's follow-up is still open on the red team's branch.** `docs/red_team/prototypes/asset_showcase.html` and `tools/asset_ripper/extract_{aoe2,caesar3,sc2k,stronghold}.py` are still there after the merge of `main` (`fcd34b8`). D53 asked for the showcase's deletion in a normal commit. The extractors may stay on that branch but never reach `main`.
3. The red team should label its findings by what it measured, not by what it would like decided ("mandated", "ratified", "resolved"). The director and the Architect decide.
