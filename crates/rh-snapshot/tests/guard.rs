//! The unsynced guard (SPR-007 T-04; V-DET-10): read-only calls pass; a write injected through a
//! read-only path trips the guard (the positive control); release builds compile it out.
//!
//! The engine-level fuzz of the read-only APIs is `tests/unsynced_guard.rs`.
//!
//! REF: `docs/09` V-DET-10; `05.1` §6 item 7; SPR-007 ruling 5.
#![allow(clippy::unwrap_used, reason = "tests")]

use std::sync::Mutex;

use rh_core::world::WorldTables;
use rh_snapshot::guard::{GUARD_ENABLED, PlaneAState, read_only};
use rh_snapshot::hash::{MerkleCache, Plane, PlaneDigest, state_hash};

fn world() -> WorldTables {
    let mut w = WorldTables::new();
    w.insert("pop.count", (0..5000).collect());
    w.insert("ledger.wealth", vec![7; 300]);
    w
}

/// Plane-A state behind interior mutability: what a cache or a foreign-function binding would
/// give a read-only API that the borrow checker cannot stop.
struct Leaky {
    tables: Mutex<WorldTables>,
}

impl PlaneAState for Leaky {
    fn plane_a_hash(&self) -> u64 {
        state_hash(&self.tables.lock().unwrap())
    }
}

/// A "read-only" frame builder with an injected write: it bumps one row while reading.
fn leaky_frame(state: &Leaky) -> usize {
    let mut t = state.tables.lock().unwrap();
    let mut rows = t.get("pop.count").unwrap().clone();
    rows[4321] += 1;
    t.insert("pop.count", rows);
    t.names().count()
}

/// The same frame builder without the write.
fn honest_frame(state: &Leaky) -> usize {
    state.tables.lock().unwrap().names().count()
}

#[test]
fn the_guard_is_on_exactly_in_debug_builds() {
    assert_eq!(GUARD_ENABLED, cfg!(debug_assertions));
}

#[test]
fn read_only_calls_pass_the_guard() {
    let w = world();
    let h = state_hash(&w);
    assert_eq!(read_only("state_hash", &w, state_hash), Ok(h));
    let digest = read_only("digest", &w, PlaneDigest::of_world).unwrap();
    assert_eq!(digest.hash, h);
    let mut cache = MerkleCache::new(Plane::A);
    assert_eq!(read_only("merkle", &w, |w| cache.update_world(w)), Ok(h));
    assert_eq!(
        read_only("column", &w, |w| w.get("pop.count").map(Vec::len)),
        Ok(Some(5000))
    );
    let leaky = Leaky {
        tables: Mutex::new(world()),
    };
    assert_eq!(read_only("honest_frame", &leaky, honest_frame), Ok(2));
}

#[cfg(debug_assertions)]
#[test]
fn an_injected_write_trips_the_guard() {
    let leaky = Leaky {
        tables: Mutex::new(world()),
    };
    let before = leaky.plane_a_hash();
    let err = read_only("leaky_frame", &leaky, leaky_frame).unwrap_err();
    assert_eq!(err.api, "leaky_frame");
    assert_eq!(err.before, before);
    assert_eq!(err.after, leaky.plane_a_hash());
    assert_ne!(err.before, err.after);
    assert!(err.to_string().contains("leaky_frame"));
    // Writing back the same value is not a change of state, and passes.
    let same = |s: &Leaky| {
        let mut t = s.tables.lock().unwrap();
        let rows = t.get("ledger.wealth").unwrap().clone();
        t.insert("ledger.wealth", rows);
    };
    assert_eq!(read_only("rewrite_same", &leaky, same), Ok(()));
}

#[cfg(not(debug_assertions))]
#[test]
fn release_builds_compile_the_guard_out() {
    let leaky = Leaky {
        tables: Mutex::new(world()),
    };
    let result: Result<usize, rh_snapshot::guard::UnsyncedWrite> =
        read_only("leaky_frame", &leaky, leaky_frame);
    assert_eq!(result, Ok(2));
}
