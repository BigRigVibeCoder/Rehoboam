//! `rh-chronicle`: chronicle writer and cinematic event envelope ring buffer (WAVE-004 / SPEC-004).
//!
//! Plane A, machinery. Implements bounded-memory colony event logging, ring buffer storage,
//! structured cinematic event envelopes, and zero heap allocation in simulation loop.
//!
//! REF: `specs/chronicle/envelope_v0.md`; `CODEX/05_PROJECT/SPR-020_EventEnvelope.md`;
//! `.spec/drafts/SPEC-004_chronicle_writer.md`.
#![forbid(unsafe_code)]

pub mod contracts;

pub use contracts::{
    BoundedChronicleWriter, ChronicleError, ChronicleWriter, ColonyEvent, EventPayload,
    RingBufferChronicle,
};
