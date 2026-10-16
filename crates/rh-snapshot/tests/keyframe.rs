//! Keyframes (SPR-007 T-02; V-DET-05): Arrow IPC with zstd round-trips the whole plane-A state of
//! a life world (pending timers, queued inputs, epochs, held events); a loaded keyframe advances
//! ten years exactly like the unbroken run; the permanent 10-year cadence and the yearly LRU;
//! every way a keyframe can be wrong is refused, a lost pending timer included.
//!
//! REF: `docs/09` V-DET-05; `05.2` §6; SPR-007 T-02, ruling 2; Architect rulings 2026-10-16.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests"
)]

mod life;
mod support;

/// The keyframe envelope's length: 8 bytes of magic, 16 of digest.
const ENVELOPE_LEN: usize = 24;

use std::io::Cursor;
use std::sync::Arc;

use arrow_array::{Array, ArrayRef, BinaryArray, RecordBatch, StringArray};
use arrow_ipc::reader::FileReader;
use arrow_ipc::writer::FileWriter;
use life::{CALENDAR, DAY, entity, life_engine, life_registered};
use rh_core::crash::Provenance;
use rh_core::engine::{Engine, MaterialInput};
use rh_core::time::SimTime;
use rh_snapshot::guard::PlaneAState;
use rh_snapshot::journal::{JournalPosition, chain_genesis};
use rh_snapshot::keyframe::{
    History, KeyframeError, load_engine, read_keyframe, seal_keyframe, unseal_keyframe,
    write_keyframe,
};
use rh_snapshot::state::{decode_engine_tables, engine_tables};
use rh_snapshot::state_hash;
use rh_snapshot::store::{Admission, KeyframeStore};

const SEED: u64 = 13;
const YEAR: i64 = 365 * DAY;

fn history() -> History {
    History {
        provenance: Provenance {
            build: "test-build".to_owned(),
            registry_version: "0.1.0".to_owned(),
            manifest_hash: "life".to_owned(),
        },
        world_seed: SEED,
        branch_salt: 0,
    }
}

fn position(inputs: u32) -> JournalPosition {
    JournalPosition {
        timeline_id: 0,
        inputs,
        head: chain_genesis(0),
    }
}

/// The life world with three inputs, the later two still queued at day 50.
fn life() -> Engine {
    let mut e = life_engine(SEED, 48, 3, false, 1);
    e.set_provenance(history().provenance);
    for (day, step) in [(45, 5), (400, 7), (600, 3)] {
        let scope = (0..48).step_by(step).map(entity).collect();
        e.submit_input(
            SimTime::from_minutes(day * DAY + 3),
            MaterialInput { scope },
        )
        .unwrap();
    }
    e
}

fn run_to(mut e: Engine, end: i64) -> Engine {
    e.advance_until(SimTime::from_minutes(end), |_, _| {})
        .unwrap_or_else(|err| panic!("{err}"));
    e
}

/// `state_hash` of the engine at every month boundary up to `end`.
fn monthly(e: &mut Engine, end: i64) -> Vec<(i64, u64)> {
    let mut out = Vec::new();
    while let Some(t) = e.next_instant().filter(|t| t.minutes() <= end) {
        e.advance_until(t, |_, _| {})
            .unwrap_or_else(|err| panic!("{err}"));
        if CALENDAR.is_boundary(rh_core::Pulse::Month, t) {
            out.push((t.minutes(), state_hash(&e.snapshot())));
        }
    }
    out
}

fn load(bytes: &[u8]) -> Result<Engine, KeyframeError> {
    load_engine(bytes, CALENDAR, life_registered(), &history()).map(|(e, _)| e)
}

#[test]
fn a_keyframe_round_trips_the_whole_plane_a_state() {
    let engine = run_to(life(), 50 * DAY + 1);
    let state = engine.snapshot();
    assert!(!state.agenda.entries.is_empty(), "pending timers");
    assert!(!state.events.events.is_empty(), "held events");
    assert!(!state.inputs.is_empty() && !state.epochs.is_empty());
    let (bytes, header) = write_keyframe(&engine, &history(), position(1)).unwrap();
    assert_eq!(header.state_hash, state_hash(&state));
    assert_eq!(&bytes[..8], b"RHKF\0v1\n", "the envelope's magic");
    assert_eq!(
        &bytes[ENVELOPE_LEN..ENVELOPE_LEN + 6],
        b"ARROW1",
        "an Arrow IPC file"
    );
    let (read, read_header) = read_keyframe(&bytes, &life_registered()).unwrap();
    assert_eq!(read, state);
    assert_eq!(read_header, header);
    let loaded = load(&bytes).unwrap();
    assert_eq!(loaded.snapshot(), state);
    // The decoder inverts the encoder table by table.
    let tables = engine_tables(&state)
        .into_iter()
        .map(|(n, rows)| (n.to_owned(), rows))
        .collect();
    assert_eq!(
        decode_engine_tables(&tables, &life_registered(), state.tables.clone()).unwrap(),
        state
    );
}

/// V-DET-05: write a keyframe, load it, advance ten years; equal to the continuous run, month by
/// month. Two cut points: day 50 (timers pending, events held, inputs queued) and year 1.
#[test]
fn v_det_05_a_loaded_keyframe_advances_ten_years_like_the_continuous_run() {
    for cut in [50 * DAY + 1, YEAR - 1] {
        let end = cut + 10 * YEAR;
        let mut continuous = run_to(life(), cut);
        let (bytes, _) = write_keyframe(&continuous, &history(), position(1)).unwrap();
        let expected = monthly(&mut continuous, end);
        let mut loaded = load(&bytes).unwrap();
        let got = monthly(&mut loaded, end);
        assert_eq!(got.len(), 120, "ten years of months");
        assert_eq!(got, expected, "cut at minute {cut}");
        assert_eq!(loaded.snapshot(), continuous.snapshot());
    }
    println!("V-DET-05: two keyframes loaded and advanced 10 years, 120 monthly hashes equal");
}

/// The permanent 10-year cadence and the yearly LRU (V-DET-05's second half): years 0, 10, 20
/// are kept for good; the cache keeps the most recently used years; a seek to any year replays
/// from the nearest held keyframe and reaches the continuous run's hash.
#[test]
fn v_det_05_the_permanent_cadence_and_the_yearly_lru() {
    let mut store = KeyframeStore::new(10, 3);
    assert_eq!(KeyframeStore::default(), KeyframeStore::new(10, 8));
    let mut evicted = Vec::new();
    let mut frames = std::collections::BTreeMap::new();
    let mut e = life();
    let mut year_hashes = Vec::new();
    for year in 0..=24_i64 {
        e = run_to(e, year * YEAR - 1);
        year_hashes.push(state_hash(&e.snapshot()));
        match store.admit(year) {
            Admission::Permanent => assert_eq!(year % 10, 0),
            Admission::Cached { evicted: gone } => evicted.extend(gone),
        }
        let (bytes, _) = write_keyframe(&e, &history(), position(1)).unwrap();
        frames.insert(year, bytes);
        for gone in &evicted {
            frames.remove(gone);
        }
    }
    assert_eq!(store.permanent().collect::<Vec<_>>(), [0, 10, 20]);
    assert_eq!(store.cached(), [22, 23, 24]);
    assert_eq!(evicted.len(), 25 - 3 - 3);
    assert!(
        evicted
            .iter()
            .all(|y| !store.holds(*y) && !frames.contains_key(y))
    );
    // Loading year 22 makes it the most recent; year 25's admission then evicts 23.
    assert!(store.touch(22));
    assert_eq!(store.admit(25), Admission::Cached { evicted: Some(23) });
    assert_eq!(store.nearest_at_or_before(19), Some(10));
    assert_eq!(store.nearest_at_or_before(23), Some(22));
    assert_eq!(store.nearest_at_or_before(-1), None);
    // Seek to year 17: replay from year 10's keyframe reaches the continuous hash.
    let from = store.nearest_at_or_before(17).unwrap();
    let seeked = run_to(load(&frames[&from]).unwrap(), 17 * YEAR - 1);
    assert_eq!(state_hash(&seeked.snapshot()), year_hashes[17]);
}

/// Rewrites a keyframe with `edit` applied to its batches' (table, row) pairs.
fn rewrite(bytes: &[u8], edit: impl Fn(&mut Vec<(String, Vec<u8>)>)) -> Vec<u8> {
    let reader = FileReader::try_new(Cursor::new(unseal_keyframe(bytes).unwrap()), None).unwrap();
    let schema = reader.schema();
    let mut pairs = Vec::new();
    for batch in reader {
        let batch = batch.unwrap();
        let names = batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let rows = batch
            .column(1)
            .as_any()
            .downcast_ref::<BinaryArray>()
            .unwrap();
        for k in 0..batch.num_rows() {
            pairs.push((names.value(k).to_owned(), rows.value(k).to_vec()));
        }
    }
    edit(&mut pairs);
    let mut w = FileWriter::try_new(Vec::new(), &schema).unwrap();
    let names: ArrayRef = Arc::new(StringArray::from_iter_values(
        pairs.iter().map(|p| p.0.as_str()),
    ));
    let rows: ArrayRef = Arc::new(BinaryArray::from_iter_values(
        pairs.iter().map(|p| p.1.as_slice()),
    ));
    w.write(&RecordBatch::try_new(schema.clone(), vec![names, rows]).unwrap())
        .unwrap();
    w.finish().unwrap();
    seal_keyframe(&w.into_inner().unwrap())
}

#[test]
fn every_wrong_keyframe_is_refused() {
    let engine = run_to(life(), 50 * DAY + 1);
    let (bytes, _) = write_keyframe(&engine, &history(), position(1)).unwrap();
    // The faithful rewrite still loads: the edits below are what the checks catch.
    assert!(load(&rewrite(&bytes, |_| {})).is_ok());
    // A lost pending timer (and the header's count lowered to match) no longer hashes equal.
    let lost = rewrite(&bytes, |pairs| {
        let k = pairs
            .iter()
            .position(|p| p.0 == "rh-core/agenda.timers")
            .unwrap();
        pairs.remove(k);
        for p in pairs.iter_mut().filter(|p| p.0 == "rehoboam:keyframe") {
            let line = String::from_utf8(p.1.clone()).unwrap();
            if let Some(n) = line.strip_prefix("table\trh-core/agenda.timers\t") {
                let n: usize = n.parse().unwrap();
                p.1 = format!("table\trh-core/agenda.timers\t{}", n - 1).into_bytes();
            }
        }
    });
    assert!(matches!(
        load(&lost),
        Err(KeyframeError::HashMismatch { .. })
    ));
    // An altered world row.
    let altered = rewrite(&bytes, |pairs| {
        let p = pairs.iter_mut().find(|p| p.0 == "vital.alive").unwrap();
        p.1[0] ^= 1;
    });
    assert!(matches!(
        load(&altered),
        Err(KeyframeError::HashMismatch { .. })
    ));
    // A row count that disagrees with the header.
    let short = rewrite(&bytes, |pairs| {
        let k = pairs.iter().position(|p| p.0 == "vital.alive").unwrap();
        pairs.remove(k);
    });
    assert!(matches!(load(&short), Err(KeyframeError::Format(_))));
    // Corrupt bytes.
    let mut corrupt = bytes.clone();
    let mid = corrupt.len() / 2;
    corrupt[mid] ^= 0xFF;
    assert!(matches!(load(&corrupt), Err(KeyframeError::Format(_))));
    assert!(matches!(
        load(&bytes[..bytes.len() / 2]),
        Err(KeyframeError::Format(_))
    ));
    assert!(matches!(
        load(&bytes[ENVELOPE_LEN..]),
        Err(KeyframeError::Format(_))
    ));
    // Every single flipped byte, envelope and body alike, is refused without a panic.
    for k in (0..bytes.len()).step_by(bytes.len() / 64 + 1) {
        let mut one = bytes.clone();
        one[k] ^= 0x01;
        assert!(load(&one).is_err(), "byte {k} flipped");
    }
    // Another history.
    let mut other = history();
    other.world_seed += 1;
    assert!(matches!(
        load_engine(&bytes, CALENDAR, life_registered(), &other),
        Err(KeyframeError::History("world_seed"))
    ));
    // Another timeline's journal position.
    let mut wrong = position(1);
    wrong.timeline_id = 4;
    assert!(matches!(
        write_keyframe(&engine, &history(), wrong),
        Err(KeyframeError::Format(_))
    ));
}

#[test]
fn a_poisoned_engine_has_no_keyframe() {
    use rh_core::system::SimError;
    let mut stubs = vec![
        support::Stub::new("X1.bad")
            .writes("bad.n")
            .at(rh_core::Pulse::Day, rh_core::Phase::Vital)
            .stepping(|_, _, _, _| {
                Err(SimError::System {
                    system: "X1.bad",
                    message: "always".to_owned(),
                })
            }),
    ];
    let specs = support::agreeing_specs(&stubs);
    let registered =
        rh_core::register::register(stubs.drain(..).map(support::Stub::boxed).collect(), &specs)
            .unwrap();
    let mut e = Engine::new(
        CALENDAR,
        1,
        rh_ids::BranchSalt::from_raw(0),
        registered,
        rh_core::world::WorldTables::new(),
        SimTime::EPOCH,
    );
    assert!(
        e.advance_until(SimTime::from_minutes(DAY), |_, _| {})
            .is_err()
    );
    assert!(matches!(
        write_keyframe(&e, &history(), position(0)),
        Err(KeyframeError::Poisoned(_))
    ));
}

/// VER-SPR-007-TEST GAP-4: the guard's hash of an engine covers its engine-held state: two engines
/// with equal tables and different agendas hash differently.
#[test]
fn the_guard_sees_engine_held_state() {
    let engine = run_to(life(), 50 * DAY + 1);
    let (bytes, _) = write_keyframe(&engine, &history(), position(1)).unwrap();
    let mut state = read_keyframe(&bytes, &life_registered()).unwrap().0;
    let tables_only = rh_snapshot::tables_hash(&state.tables);
    state.agenda.entries.pop();
    state
        .agenda
        .tombstones
        .retain(|t| state.agenda.entries.iter().any(|(k, _)| k.id() == *t));
    let fewer = Engine::restore(
        CALENDAR,
        SEED,
        rh_ids::BranchSalt::from_raw(0),
        life_registered(),
        history().provenance,
        state,
    )
    .unwrap();
    assert_eq!(rh_snapshot::tables_hash(fewer.tables()), tables_only);
    assert_ne!(fewer.plane_a_hash(), engine.plane_a_hash());
}
