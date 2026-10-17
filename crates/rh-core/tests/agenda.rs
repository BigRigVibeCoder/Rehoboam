//! T-01 and T-02 (agenda part): the agenda pops in key order with ties, cancelled timers never
//! pop, and the serialisable state restores an agenda that pops identically (SPR-006).
//!
//! Property test, deterministic: each case is a sequence of random operations (insert with many
//! ties in time, phase and entity; cancel a live or a dead id; pop; dump and restore) drawn from the
//! repository's scalar hash, checked against a model (a `BTreeMap` of live keys).
//!
//! Refs: SPR-006 T-01, T-02, rulings 2 and 7; `05.1` §4.3.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken case must fail the test loudly"
)]

use std::collections::BTreeMap;

use rh_core::Phase;
use rh_core::agenda::{Agenda, AgendaError, AgendaKey, Timer, TimerId};
use rh_core::time::SimTime;
use rh_ids::rng::{h64_words, tag};
use rh_ids::{EntityKind, EntityRef, PurposeId, SystemId};

const CASES: u64 = 256;
const OPS: u64 = 300;
const TAG: u64 = tag("rehoboam/rh-core/tests/agenda/v1");

/// A deterministic stream of words for one case.
struct Draws {
    case: u64,
    k: u64,
}

impl Draws {
    fn next(&mut self, modulus: u64) -> u64 {
        self.k += 1;
        h64_words(TAG, &[self.case, self.k]) % modulus
    }
}

fn random_insert(a: &mut Agenda, d: &mut Draws) -> (AgendaKey, Timer) {
    // Small ranges, so equal times, phases and entities are common.
    let time = SimTime::from_minutes(i64::try_from(d.next(6)).unwrap() - 2);
    let phase = Phase::ALL[usize::try_from(d.next(3)).unwrap() * 4];
    let kind = EntityKind::ALL[usize::try_from(d.next(3)).unwrap()];
    let entity = EntityRef::new(kind, d.next(3));
    let timer = Timer {
        owner: SystemId::from_raw(u32::try_from(d.next(4)).unwrap()),
        purpose: PurposeId::from_raw(d.next(1 << 20)),
        kind: u32::try_from(d.next(9)).unwrap(),
        payload: i64::try_from(d.next(1000)).unwrap(),
    };
    let id = a.insert(time, phase, entity, timer);
    let key = AgendaKey {
        time,
        phase,
        entity,
        seq: id.to_raw(),
    };
    (key, timer)
}

#[test]
fn t01_pop_order_is_key_order_with_ties_and_cancellation() {
    let mut pops = 0_u64;
    let mut ties = 0_u64;
    for case in 0..CASES {
        let mut d = Draws { case, k: 0 };
        let mut a = Agenda::new();
        let mut model: BTreeMap<AgendaKey, Timer> = BTreeMap::new();
        let mut dead: Vec<TimerId> = Vec::new();
        for _ in 0..OPS {
            match d.next(10) {
                0..=4 => {
                    let (key, timer) = random_insert(&mut a, &mut d);
                    if model
                        .keys()
                        .any(|k| (k.time, k.phase, k.entity) == (key.time, key.phase, key.entity))
                    {
                        ties += 1;
                    }
                    model.insert(key, timer);
                }
                5 | 6 => {
                    if model.is_empty() {
                        continue;
                    }
                    let n = usize::try_from(d.next(model.len() as u64)).unwrap();
                    let key = *model.keys().nth(n).unwrap();
                    assert_eq!(a.cancel(key.id()), Ok((key, model[&key])));
                    model.remove(&key);
                    dead.push(key.id());
                }
                7 => {
                    if let Some(&id) = dead.last() {
                        assert_eq!(a.cancel(id), Err(AgendaError::NotPending(id)));
                        assert!(
                            !a.is_pending(id),
                            "a cancelled or fired timer is never pending"
                        );
                    }
                }
                8 => {
                    let expected = model.pop_first();
                    let got = a.pop();
                    assert_eq!(got, expected, "case {case}: pop is the least live key");
                    if let Some((k, _)) = got {
                        dead.push(k.id());
                        pops += 1;
                    }
                }
                _ => {
                    // Dump and restore: the restored agenda is the same state and pops the same.
                    let state = a.state();
                    assert!(state.entries.windows(2).all(|w| w[0].0 < w[1].0));
                    let restored = Agenda::restore(&state).unwrap();
                    assert_eq!(restored.state(), state);
                    a = restored;
                }
            }
            assert_eq!(a.len(), model.len());
            assert_eq!(a.peek(), model.keys().next().copied());
            for kind in &EntityKind::ALL[..3] {
                for raw in 0..3 {
                    let e = EntityRef::new(*kind, raw);
                    let expected: Vec<_> = model
                        .iter()
                        .filter(|(k, _)| k.entity == e)
                        .map(|(k, t)| (*k, *t))
                        .collect();
                    assert_eq!(a.pending_for(e), expected, "case {case}: the entity index");
                }
            }
        }
        let rest: Vec<_> = std::iter::from_fn(|| a.pop()).collect();
        let expected: Vec<_> = model.into_iter().collect();
        assert_eq!(rest, expected, "case {case}: the drain is in key order");
    }
    assert!(pops > CASES * 10, "the property pops often ({pops})");
    assert!(ties > CASES * 10, "the property exercises ties ({ties})");
}

#[test]
fn t01_order_is_a_function_of_the_keys_not_of_insertion_order() {
    // The same (time, phase, entity) triples inserted in two orders: each agenda pops its own keys
    // in sorted order, and the triples come out in the same sequence (seq only breaks exact ties).
    let mut d = Draws { case: 9_999, k: 0 };
    let mut triples = Vec::new();
    for _ in 0..200 {
        let time = SimTime::from_minutes(i64::try_from(d.next(5)).unwrap());
        let phase = Phase::ALL[usize::try_from(d.next(12)).unwrap()];
        let entity = EntityRef::new(EntityKind::Person, d.next(4));
        triples.push((time, phase, entity));
    }
    let timer = Timer {
        owner: SystemId::from_raw(0),
        purpose: PurposeId::from_raw(0),
        kind: 0,
        payload: 0,
    };
    let run = |order: &[(SimTime, Phase, EntityRef)]| -> Vec<(SimTime, Phase, EntityRef)> {
        let mut a = Agenda::new();
        for &(t, p, e) in order {
            a.insert(t, p, e, timer);
        }
        let keys: Vec<AgendaKey> = std::iter::from_fn(|| a.pop()).map(|(k, _)| k).collect();
        assert!(
            keys.windows(2).all(|w| w[0] < w[1]),
            "strictly increasing keys"
        );
        keys.iter().map(|k| (k.time, k.phase, k.entity)).collect()
    };
    let forward = run(&triples);
    triples.reverse();
    let backward = run(&triples);
    assert_eq!(forward, backward);
    let mut sorted = triples.clone();
    sorted.sort();
    assert_eq!(forward, sorted);
}
