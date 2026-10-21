//! Integration test suite for `rh-chronicle` contracts and ring buffer invariants (SPEC-004).

use rh_chronicle::{
    BoundedChronicleWriter, ChronicleError, ChronicleWriter, ColonyEvent, RingBufferChronicle,
};
use rh_events::{EdgeKind, Origin, Secrecy};
use rh_ids::{EventId, PersonId, PlaceId};

#[test]
fn test_colony_event_envelope_construction() {
    let eid = EventId::from_raw(100);
    let actor = PersonId::from_raw(42);
    let target = PersonId::from_raw(84);
    let place = PlaceId::from_raw(1);
    let cause = EventId::from_raw(50);

    let event = ColonyEvent::new(eid, 1200, 1, 10, 15, "MENTAL_BREAK")
        .with_cause(cause, EdgeKind::MotivatedBy)
        .with_actor(actor)
        .with_target(target)
        .with_place(place)
        .with_attribution(Origin::Sim, Secrecy::Public)
        .with_payload(50, [1, 2, 3, 4]);

    assert_eq!(event.event_id, eid);
    assert_eq!(event.tick, 1200);
    assert_eq!(event.phase, 1);
    assert_eq!(event.year, 10);
    assert_eq!(event.day, 15);
    assert_eq!(event.kind, "MENTAL_BREAK");
    assert_eq!(event.cause_id, Some(cause));
    assert_eq!(event.edge_kind, EdgeKind::MotivatedBy);
    assert_eq!(event.actor, Some(actor));
    assert_eq!(event.target, Some(target));
    assert_eq!(event.place, Some(place));
    assert_eq!(event.origin, Origin::Sim);
    assert_eq!(event.secrecy, Secrecy::Public);
    assert_eq!(event.magnitude, 50);
    assert_eq!(event.payload, [1, 2, 3, 4]);

    // Verify Copy semantics
    let copy = event;
    assert_eq!(copy, event);
}

#[test]
fn test_bounded_chronicle_zero_alloc_record_and_query() {
    let mut chronicle = BoundedChronicleWriter::new(10).expect("failed to init chronicle");
    assert_eq!(chronicle.capacity(), 10);
    assert_eq!(chronicle.len(), 0);
    assert!(chronicle.is_empty());
    assert_eq!(chronicle.total_recorded(), 0);

    // Initial empty query
    assert_eq!(chronicle.query_recent(5).len(), 0);

    for i in 1..=5 {
        let day = u16::try_from(i).expect("fit in u16");
        let event = ColonyEvent::new(EventId::from_raw(i), i * 100, 0, 1, day, "EVENT");
        chronicle.record_event(&event).expect("record event failed");
    }

    assert_eq!(chronicle.len(), 5);
    assert_eq!(chronicle.total_recorded(), 5);

    // Query 3 recent
    let recent_3 = chronicle.query_recent(3);
    assert_eq!(recent_3.len(), 3);
    assert_eq!(recent_3[0].event_id, EventId::from_raw(3));
    assert_eq!(recent_3[1].event_id, EventId::from_raw(4));
    assert_eq!(recent_3[2].event_id, EventId::from_raw(5));

    // Query more than recorded (limit 10)
    let recent_all = chronicle.query_recent(10);
    assert_eq!(recent_all.len(), 5);
    for (idx, ev) in recent_all.iter().enumerate() {
        assert_eq!(ev.event_id, EventId::from_raw((idx + 1) as u64));
    }
}

#[test]
fn test_bounded_chronicle_ring_buffer_eviction() {
    const CAP: usize = 4;
    let mut chronicle = RingBufferChronicle::new(CAP).expect("failed to init ring buffer");

    for i in 1..=10 {
        let event = ColonyEvent::new(EventId::from_raw(i), i * 10, 0, 1, 1, "TICK_EVENT");
        chronicle.record_event(&event).expect("record failed");
    }

    assert_eq!(chronicle.capacity(), CAP);
    assert_eq!(chronicle.len(), CAP);
    assert_eq!(chronicle.total_recorded(), 10);

    // Query 4 should return the 4 most recent events: 7, 8, 9, 10
    let recent = chronicle.query_recent(4);
    assert_eq!(recent.len(), 4);
    assert_eq!(recent[0].event_id, EventId::from_raw(7));
    assert_eq!(recent[1].event_id, EventId::from_raw(8));
    assert_eq!(recent[2].event_id, EventId::from_raw(9));
    assert_eq!(recent[3].event_id, EventId::from_raw(10));

    // Query 2 should return 9, 10
    let recent_2 = chronicle.query_recent(2);
    assert_eq!(recent_2.len(), 2);
    assert_eq!(recent_2[0].event_id, EventId::from_raw(9));
    assert_eq!(recent_2[1].event_id, EventId::from_raw(10));
}

#[test]
fn test_chronicle_causality_invariant() {
    let mut chronicle = BoundedChronicleWriter::new(5).expect("init failed");

    // Valid: cause_id (10) < event_id (20)
    let valid_event = ColonyEvent::new(EventId::from_raw(20), 100, 0, 1, 1, "VALID")
        .with_cause(EventId::from_raw(10), EdgeKind::Triggered);
    assert!(chronicle.record_event(&valid_event).is_ok());

    // Invalid: cause_id (30) > event_id (25)
    let invalid_future_cause = ColonyEvent::new(EventId::from_raw(25), 110, 0, 1, 1, "INVALID")
        .with_cause(EventId::from_raw(30), EdgeKind::Triggered);
    match chronicle.record_event(&invalid_future_cause) {
        Err(ChronicleError::CausalityViolation { event_id, cause_id }) => {
            assert_eq!(event_id, EventId::from_raw(25));
            assert_eq!(cause_id, EventId::from_raw(30));
        }
        other => panic!("expected CausalityViolation, got {other:?}"),
    }

    // Invalid: cause_id (25) == event_id (25)
    let self_cause = ColonyEvent::new(EventId::from_raw(25), 110, 0, 1, 1, "SELF_CAUSE")
        .with_cause(EventId::from_raw(25), EdgeKind::Triggered);
    assert!(matches!(
        chronicle.record_event(&self_cause),
        Err(ChronicleError::CausalityViolation { .. })
    ));
}

#[test]
fn test_chronicle_monotonic_timestamps() {
    let mut chronicle = BoundedChronicleWriter::new(5).expect("init failed");

    let e1 = ColonyEvent::new(EventId::from_raw(1), 100, 0, 1, 1, "E1");
    assert!(chronicle.record_event(&e1).is_ok());

    let e2 = ColonyEvent::new(EventId::from_raw(2), 100, 1, 1, 1, "E2_SAME_TICK");
    assert!(chronicle.record_event(&e2).is_ok());

    let e3_backdated = ColonyEvent::new(EventId::from_raw(3), 99, 0, 1, 1, "BACKDATED");
    match chronicle.record_event(&e3_backdated) {
        Err(ChronicleError::InvalidTimestamp { event_tick, last_tick }) => {
            assert_eq!(event_tick, 99);
            assert_eq!(last_tick, 100);
        }
        other => panic!("expected InvalidTimestamp, got {other:?}"),
    }
}

#[test]
fn test_chronicle_zero_capacity_error() {
    assert_eq!(
        BoundedChronicleWriter::new(0).err(),
        Some(ChronicleError::CapacityZero)
    );
}
