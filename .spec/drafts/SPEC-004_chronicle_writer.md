# SPEC-004: Chronicle Writer & Cinematic Event Envelope Integration

- **Sprint / Wave:** `WAVE-004` (P1a Bloodlines)
- **Author:** Lead Architect Agent (`gemini-architect`)
- **Status:** `DRAFT ⏳`
- **Created:** 2026-10-21
- **Branch:** `arch/spec-004-draft`
- **Related Specs:** `specs/chronicle/envelope_v0.md`, `CODEX/05_PROJECT/SPR-020_EventEnvelope.md`, `ADR-0007-chronicle-storage.md`

---

## 1. Observable Outcomes (Behavioral State Transitions)

- [ ] **Event Envelope Schema & Attribution Invariants (`ColonyEvent`):**
  Every event recorded in the colony history carries an immutable envelope containing:
  - **Structured Timestamps:** Simulation tick (`tick: u64`), scheduler phase (`phase: u8`), calendar year (`year: u32`), and day within year (`day: u16`, `0..=364`).
  - **Causality Markers:** Canonical identifier (`event_id: EventId`), optional inciting event pointer (`cause_id: Option<EventId>`), and semantic causal edge classification (`edge_kind: EdgeKind` per `envelope_v0.md` §5). The causality invariant holds strictly: whenever `cause_id` is present, `cause_id < event_id`.
  - **Agent Attribution:** Primary actor (`actor: Option<PersonId>`), target entity (`target: Option<PersonId>`), journal origin (`origin: Origin`: `Sim`, `Player`, `Oracle`, `Director`), and visibility tier (`secrecy: Secrecy`: `Public`, `Witnessed`, `Faction`, `Secret`).
  - **Payload:** Bounded fixed-size payload (`payload: [i64; 4]`) and scalar magnitude (`magnitude: i32`).
- [ ] **Ring Buffer Chronicle Writer & Bounded Memory (`BoundedChronicleWriter`):**
  Colony events are retained in an in-memory ring buffer with fixed maximum capacity $C$.
  - When $N \le C$ events have been recorded, exactly $N$ events are retained.
  - When $N > C$ events have been recorded, the oldest $N - C$ events are overwritten in circular FIFO order without memory growth, fragmentation, or memory leaks.
  - Memory consumption is strictly bounded $O(C)$ and allocated once at writer initialization.
- [ ] **Zero Heap Allocation in Simulation Loop:**
  During the hot simulation loop, calling `ChronicleWriter::record_event(&mut self, event: &ColonyEvent)` and `ChronicleWriter::query_recent(&self, limit: usize)` executes with **zero heap allocations** (`0 bytes` allocated via global allocator).
- [ ] **Mirrored Buffer Architecture & Contiguous Query Slices:**
  The internal buffer mirrors entries across a $2C$ capacity slot layout. `query_recent(limit)` returns a single contiguous slice `&[ColonyEvent]` ordered chronologically (oldest to newest) in $O(1)$ time without intermediate vector allocations or element copying.
- [ ] **Monotonic Sequencing & Causality Error Handling:**
  - Recording an event with `tick < last_tick` deterministically returns `Err(ChronicleError::InvalidTimestamp)`.
  - Recording an event with `cause_id >= event_id` deterministically returns `Err(ChronicleError::CausalityViolation)`.
  - Zero `.unwrap()` or panic behavior on invalid inputs.

---

## 2. Explicit Non-Goals (Out of Scope — Enforced by Gemini Verifier)

- **DO NOT** perform blocking disk I/O, SQLite, or DuckDB queries inside the simulation hot loop. Persistent chronicle streaming belongs to the World Stream v1 writer (`rh-stream`, `WAVE-005`).
- **DO NOT** allocate dynamic collections (`Vec`, `String`, `Box`) inside `record_event`.
- **DO NOT** use floating-point types (`f32`/`f64`) in plane A event accounting.
- **DO NOT** use `HashMap`, `HashSet`, or unkeyed system clocks (`SystemTime::now()`).
- **DO NOT** modify files outside the Allowed Write Paths (`specs/chronicle/*`, `crates/rh-chronicle/*`, `.spec/drafts/*`).

---

## 3. Frozen Interface Contract (`CON-004`)
> *"The contract is truth. The code is an attempt to match it."* (GOV-007 §3.1)

- **Contract / Trait Files:**
  - `crates/rh-chronicle/src/contracts.rs`
  - `crates/rh-chronicle/src/lib.rs`
- **Reference Spec Scripts:**
  - `specs/chronicle/envelope_v0.md`
  - `specs/chronicle/selftest_envelope.py`
  - `specs/chronicle/selftest.py`
- **Input / Output & Error Schema:**

| Field / Method | Input Type & Constraints | Output Type | Deterministic Error Behavior (GOV-004) |
|:--|:--|:--|:--|
| `BoundedChronicleWriter::new` | `capacity: usize > 0` | `Result<Self, ChronicleError>` | Returns `ChronicleError::CapacityZero` if `capacity == 0` |
| `ChronicleWriter::record_event` | `event: &ColonyEvent` | `Result<(), ChronicleError>` | Returns `InvalidTimestamp` if backdated; `CausalityViolation` if `cause >= event_id` |
| `ChronicleWriter::query_recent` | `limit: usize` | `&[ColonyEvent]` | Returns contiguous slice of length `min(limit, count)`; zero allocations |
| `BoundedChronicleWriter::len` | `&self` | `usize` | Returns active count `0..=capacity` |
| `BoundedChronicleWriter::total_recorded` | `&self` | `u64` | Returns cumulative events recorded |

- **Frozen Types & Signatures:**
```rust
// crates/rh-chronicle/src/contracts.rs
use rh_events::{EdgeKind, Origin, Secrecy};
use rh_ids::{EventId, PersonId, PlaceId};

pub type EventPayload = [i64; 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColonyEvent {
    pub event_id: EventId,
    pub tick: u64,
    pub phase: u8,
    pub year: u32,
    pub day: u16,
    pub cause_id: Option<EventId>,
    pub edge_kind: EdgeKind,
    pub actor: Option<PersonId>,
    pub target: Option<PersonId>,
    pub place: Option<PlaceId>,
    pub origin: Origin,
    pub secrecy: Secrecy,
    pub kind: &'static str,
    pub magnitude: i32,
    pub payload: EventPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChronicleError {
    InvalidLimit { requested: usize, capacity: usize },
    InvalidTimestamp { event_tick: u64, last_tick: u64 },
    CausalityViolation { event_id: EventId, cause_id: EventId },
    CapacityZero,
    BufferOverflow,
}

pub trait ChronicleWriter {
    fn record_event(&mut self, event: &ColonyEvent) -> Result<(), ChronicleError>;
    fn query_recent(&self, limit: usize) -> &[ColonyEvent];
}

#[derive(Debug, Clone)]
pub struct BoundedChronicleWriter {
    buffer: Vec<ColonyEvent>,
    capacity: usize,
    write_pos: usize,
    count: usize,
    total_recorded: u64,
    last_tick: u64,
}

pub type RingBufferChronicle = BoundedChronicleWriter;
```

---

## 4. Mandatory Governance Compliance Table (`GOV-001..008`)

| Governance Standard | Mandatory Requirement for All Leaf Tasks in This Wave |
|:--|:--|
| **GOV-001 (Docs)** | Structured BLUF architecture, machine-verifiable contract definitions, markdown lint compliance. |
| **GOV-002 (Testing)** | 1:1 test coverage: every public trait method covered by unit and property tests. No `#[ignore]`. |
| **GOV-003 (Determinism)** | `[lints] workspace = true`. Pure deterministic order; zero unkeyed randomness; zero float arithmetic in plane A. |
| **GOV-004 (Errors)** | Explicit `Result<T, ChronicleError>` error types. Zero runtime `.unwrap()` or panics. |
| **GOV-005 (Commits)** | Forensic commit messages with `Why:`, `What:`, `Files:`, `Agent:`, `Refs:`. |
| **GOV-006 (Logging)** | Structured logging with `event_id`, `tick`, and `actor` correlation parameters. |
| **GOV-007 (Discrepancy)** | Workers encountering ambiguity MUST report upstream via `cerberus_orchestrator.py report-upstream --type EVO`. |
| **GOV-008 (Hygiene)** | Zero secrets, `.env` files, build caches, or temporary artifacts committed. |

---

## 5. Deterministic Verification Criteria

Every worker must pass these exact commands with exit code `0`:
```bash
cargo test -p rh-chronicle
cargo clippy -p rh-chronicle --all-targets -- -D warnings
python3 -I specs/chronicle/selftest_envelope.py
python3 -I bin/lint_arch.py
```

---

## 6. Decomposed Leaf Work Orders (Forgejo Issues)

- [ ] **Issue #16 (Work Order 1):** WAVE-004 Core `rh-chronicle` ring buffer & zero-allocation in-memory event storage.
  - *Allowed Write Paths:* `crates/rh-chronicle/src/contracts.rs`, `crates/rh-chronicle/src/lib.rs`, `crates/rh-chronicle/Cargo.toml`
  - *Worker:* `worker-or-01`
- [ ] **Issue #17 (Work Order 2):** WAVE-004 Event envelope bridge (`EventDraft` to `ColonyEvent`) and provenance edge ingestion.
  - *Allowed Write Paths:* `crates/rh-chronicle/src/*`, `crates/rh-events/src/*`
  - *Worker:* `worker-or-02`
- [ ] **Issue #18 (Work Order 3):** WAVE-004 Golden chronicle verification & envelope self-test integration.
  - *Allowed Write Paths:* `specs/chronicle/*`, `tests/contracts_test.rs`
  - *Worker:* `worker-or-01`
